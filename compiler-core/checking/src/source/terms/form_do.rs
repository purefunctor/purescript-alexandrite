use std::iter;
use std::sync::Arc;

use building_types::QueryResult;
use itertools::{Itertools, Position};

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::{TypeId, toolkit, unification};
use crate::error::{ErrorCrumb, ErrorKind};
use crate::semantic::{
    CheckedBinderKind, CheckedBlockStatement, CheckedDoExpression, CheckedDoStep,
    CheckedErrorStatement, CheckedExpressionId, CheckedExpressionKind,
};
use crate::source::binder;
use crate::source::terms::{application, form_let};
use crate::state::CheckState;

enum DoStep<'a> {
    Error {
        statement: lowering::DoStatementId,
    },
    Bind {
        statement: lowering::DoStatementId,
        binder: Option<lowering::BinderId>,
        binder_type: TypeId,
        expression: Option<lowering::ExpressionId>,
    },
    Discard {
        statement: lowering::DoStatementId,
        expression: Option<lowering::ExpressionId>,
    },
    Let {
        statement: lowering::DoStatementId,
        statements: &'a [lowering::LetBindingChunk],
    },
}

impl DoStep<'_> {
    fn is_action(&self) -> bool {
        matches!(self, Self::Bind { .. } | Self::Discard { .. })
    }
}

enum DoBlockFinalStep {
    Empty,
    InvalidBind {
        statement: lowering::DoStatementId,
        binder: Option<lowering::BinderId>,
        binder_type: TypeId,
        expression: Option<lowering::ExpressionId>,
    },
    Discard {
        statement: lowering::DoStatementId,
        expression: Option<lowering::ExpressionId>,
    },
    InvalidLet {
        statement: lowering::DoStatementId,
    },
}

impl DoBlockFinalStep {
    fn is_invalid_let(&self) -> bool {
        matches!(self, Self::InvalidLet { .. })
    }
}

/// Lookup `bind` from resolution, or synthesize `?m ?a -> (?a -> ?m ?b) -> ?m ?b`.
pub fn lookup_or_synthesise_bind<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    resolution: Option<lowering::TermVariableResolution>,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    if let Some(resolution) = resolution {
        toolkit::lookup_term_variable(state, context, resolution)
    } else {
        let m = state.fresh_unification(context.queries, context.prim.type_to_type);
        let a = state.fresh_unification(context.queries, context.prim.t);
        let b = state.fresh_unification(context.queries, context.prim.t);
        let m_a = context.intern_application(m, a);
        let m_b = context.intern_application(m, b);
        let a_to_m_b = context.intern_function(a, m_b);
        Ok(context.intern_function_list(&[m_a, a_to_m_b], m_b))
    }
}

/// Lookup `discard` from resolution, or synthesize `?m ?a -> (?a -> ?m ?b) -> ?m ?b`.
pub fn lookup_or_synthesise_discard<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    resolution: Option<lowering::TermVariableResolution>,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    // Same shape as bind
    lookup_or_synthesise_bind(state, context, resolution)
}

/// Lookup `map` from resolution, or synthesize `(?a -> ?b) -> ?f ?a -> ?f ?b`.
pub fn lookup_or_synthesise_map<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    resolution: Option<lowering::TermVariableResolution>,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    if let Some(resolution) = resolution {
        toolkit::lookup_term_variable(state, context, resolution)
    } else {
        let f = state.fresh_unification(context.queries, context.prim.type_to_type);
        let a = state.fresh_unification(context.queries, context.prim.t);
        let b = state.fresh_unification(context.queries, context.prim.t);
        let f_a = context.intern_application(f, a);
        let f_b = context.intern_application(f, b);
        let a_to_b = context.intern_function(a, b);
        Ok(context.intern_function_list(&[a_to_b, f_a], f_b))
    }
}

/// Lookup `apply` from resolution, or synthesize `?f (?a -> ?b) -> ?f ?a -> ?f ?b`.
pub fn lookup_or_synthesise_apply<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    resolution: Option<lowering::TermVariableResolution>,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    if let Some(resolution) = resolution {
        toolkit::lookup_term_variable(state, context, resolution)
    } else {
        let f = state.fresh_unification(context.queries, context.prim.type_to_type);
        let a = state.fresh_unification(context.queries, context.prim.t);
        let b = state.fresh_unification(context.queries, context.prim.t);
        let a_to_b = context.intern_function(a, b);
        let f_a_to_b = context.intern_application(f, a_to_b);
        let f_a = context.intern_application(f, a);
        let f_b = context.intern_application(f, b);
        Ok(context.intern_function_list(&[f_a_to_b, f_a], f_b))
    }
}

/// Lookup `pure` from resolution, or synthesize `?a -> ?f ?a`.
pub fn lookup_or_synthesise_pure<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    resolution: Option<lowering::TermVariableResolution>,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    if let Some(resolution) = resolution {
        toolkit::lookup_term_variable(state, context, resolution)
    } else {
        let f = state.fresh_unification(context.queries, context.prim.type_to_type);
        let a = state.fresh_unification(context.queries, context.prim.t);
        let f_a = context.intern_application(f, a);
        Ok(context.intern_function(a, f_a))
    }
}

pub fn infer_do<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    source_expression: lowering::ExpressionId,
    bind: Option<lowering::TermVariableResolution>,
    discard: Option<lowering::TermVariableResolution>,
    statement_id: &[lowering::DoStatementId],
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    // First, perform a forward pass where variable bindings are bound
    // to unification variables. Let bindings are not checked here to
    // avoid premature solving of unification variables. Instead, they
    // are checked inline during the statement checking loop.
    let mut steps = vec![];
    for &statement_id in statement_id.iter() {
        let Some(statement) = context.lowered.info.get_do_statement(statement_id) else {
            steps.push(DoStep::Error { statement: statement_id });
            continue;
        };
        match statement {
            lowering::DoStatement::Bind { binder, expression } => {
                let binder_type = if let Some(binder) = binder {
                    binder::infer_binder(state, context, *binder)?
                } else {
                    state.fresh_unification(context.queries, context.prim.t)
                };
                steps.push(DoStep::Bind {
                    statement: statement_id,
                    binder: *binder,
                    binder_type,
                    expression: *expression,
                });
            }
            lowering::DoStatement::Let { statements } => {
                steps.push(DoStep::Let { statement: statement_id, statements });
            }
            lowering::DoStatement::Discard { expression } => {
                steps.push(DoStep::Discard { statement: statement_id, expression: *expression });
            }
        }
    }

    let final_step = match steps.iter().rev().find(|step| !matches!(step, DoStep::Error { .. })) {
        Some(DoStep::Bind { statement, binder, binder_type, expression }) => {
            DoBlockFinalStep::InvalidBind {
                statement: *statement,
                binder: *binder,
                binder_type: *binder_type,
                expression: *expression,
            }
        }
        Some(DoStep::Discard { statement, expression }) => {
            DoBlockFinalStep::Discard { statement: *statement, expression: *expression }
        }
        Some(DoStep::Let { statement, .. }) => {
            DoBlockFinalStep::InvalidLet { statement: *statement }
        }
        Some(DoStep::Error { .. }) => unreachable!("error steps were filtered out"),
        None => DoBlockFinalStep::Empty,
    };

    let (has_bind_step, has_discard_step) = {
        let mut has_bind = false;
        let mut has_discard = false;
        let checked_steps = steps.iter().filter(|step| !matches!(step, DoStep::Error { .. }));
        for (position, statement) in checked_steps.with_position() {
            let is_final = matches!(position, Position::Last | Position::Only);
            match statement {
                DoStep::Bind { .. } => has_bind = true,
                DoStep::Discard { .. } if !is_final => has_discard = true,
                DoStep::Error { .. } | DoStep::Discard { .. } | DoStep::Let { .. } => (),
            }
        }
        (has_bind, has_discard)
    };

    let bind_type = if has_bind_step {
        lookup_or_synthesise_bind(state, context, bind)?
    } else {
        context.unknown("unused bind")
    };

    let discard_type = if has_discard_step {
        lookup_or_synthesise_discard(state, context, discard)?
    } else {
        context.unknown("unused discard")
    };

    let final_expression = match final_step {
        DoBlockFinalStep::Empty => {
            state.insert_error(ErrorKind::EmptyDoBlock);
            let result_type = context.unknown("empty do block");
            let checked_steps = steps.iter().filter_map(|step| match step {
                DoStep::Error { statement } => {
                    let error = CheckedErrorStatement {
                        source: *statement,
                        binder: None,
                        expression: None,
                    };
                    let statement = CheckedBlockStatement::Error(error);
                    Some(CheckedDoStep::Statement(statement))
                }
                DoStep::Bind { .. } | DoStep::Discard { .. } | DoStep::Let { .. } => None,
            });
            let checked_steps = checked_steps.collect_vec();
            let final_expression = allocate_missing_expression(state, result_type);
            record_do_expression(
                state,
                source_expression,
                checked_steps,
                final_expression,
                result_type,
            );
            return Ok(result_type);
        }
        // Technically valid, syntactically disallowed. This allows
        // partially-written do expressions to infer, with a friendly
        // warning to nudge the user that `bind` is prohibited.
        DoBlockFinalStep::InvalidBind { statement, expression, .. } => {
            state.with_error_crumb(ErrorCrumb::InferringDoBind(statement), |state| {
                state.insert_error(ErrorKind::InvalidFinalBind);
            });
            if expression.is_none() {
                state.insert_error(ErrorKind::EmptyDoBlock);
            }
            expression
        }
        DoBlockFinalStep::Discard { expression, .. } => {
            if expression.is_none() {
                state.insert_error(ErrorKind::EmptyDoBlock);
            }
            expression
        }
        DoBlockFinalStep::InvalidLet { statement } => {
            state.with_error_crumb(ErrorCrumb::CheckingDoLet(statement), |state| {
                state.insert_error(ErrorKind::InvalidFinalLet);
            });
            None
        }
    };

    // Create unification variables that each statement in the do expression
    // will unify against. The next section will get into more detail how
    // these are used. These unification variables are used to enable GHC-like
    // left-to-right checking of do expressions while maintaining the same
    // semantics as rebindable `do` in PureScript. When there is an invalid
    // final let, we synthesise a placeholder continuation for the missing
    // final expression, such that checking and inference proceeds as normal.
    let mut continuation_count = steps.iter().filter(|step| step.is_action()).count();
    continuation_count += usize::from(final_step.is_invalid_let());

    let continuation_types =
        iter::repeat_with(|| state.fresh_unification(context.queries, context.prim.t))
            .take(continuation_count)
            .collect_vec();

    // Let's trace over the following example:
    //
    //   main = do
    //     a <- effect
    //     b <- aff
    //     pure { a, b }
    //
    // For the first statement, we know the following information. We
    // instantiate the `bind` function to prepare it for application.
    // The first argument is easy, it's just the expression_type; the
    // second argument involves synthesising a function type using the
    // `binder_type` and the `next` continuation. The application of
    // these arguments creates important unifications, listed below.
    // Additionally, we also create a unification to unify the `now`
    // type with the result of the `bind` application.
    //
    //   expression_type := Effect Int
    //   binder_type     := ?a
    //   now             := ?0
    //   next            := ?1
    //   lambda_type     := ?a -> ?1
    //
    //   bind_type       := m a -> (a -> m b) -> m b
    //                   |
    //                   := apply(expression_type)
    //                   := (Int -> Effect ?r1) -> Effect ?r1
    //                   |
    //                   := apply(lambda_type)
    //                   := Effect ?r1
    //                   |
    //                   >> ?a := Int
    //                   >> ?1 := Effect ?r1
    //                   >> ?0 := Effect ?r1
    //
    // For the second statement, we know the following information.
    // The `now` type was already solved by the previous statement,
    // and an error should surface once we check the inferred type
    // of the statement against it.
    //
    //   expression_type := Aff Int
    //   binder_type     := ?b
    //   now             := ?1 := Effect ?r1
    //   next            := ?2
    //   lambda_type     := ?b -> ?2
    //
    //   bind_type       := m a -> (a -> m b) -> m b
    //                   |
    //                   := apply(expression_type)
    //                   := (Int -> Aff ?r2) -> Aff ?r2
    //                   |
    //                   := apply(lambda_type)
    //                   := Aff ?r2
    //                   |
    //                   >> ?b := Int
    //                   >> ?2 := Aff ?r2
    //                   |
    //                   >> ?1         ~ Aff ?r2
    //                   >> Effect ?r1 ~ Aff ?r2
    //                   |
    //                   >> Oh no!
    //
    // This unification error is expected, but this left-to-right checking
    // approach significantly improves the reported error positions compared
    // to the previous approach that emulated desugared checking.

    let mut continuations = continuation_types.iter().tuple_windows::<(_, _)>();
    let mut checked_steps = vec![];
    let missing_expression_type = context.unknown("missing do expression");

    for step in &steps {
        match step {
            DoStep::Error { statement } => {
                let error =
                    CheckedErrorStatement { source: *statement, binder: None, expression: None };
                let statement = CheckedBlockStatement::Error(error);
                checked_steps.push(CheckedDoStep::Statement(statement));
            }
            DoStep::Let { statement, statements } => {
                state.with_error_crumb(ErrorCrumb::CheckingDoLet(*statement), |state| {
                    form_let::check_let_chunks(state, context, statements)
                })?;
                let statement =
                    form_let::checked_let_statement(state, context, *statement, statements);
                let statement = CheckedBlockStatement::Let(statement);
                checked_steps.push(CheckedDoStep::Statement(statement));
            }
            DoStep::Bind { statement, binder, binder_type, expression } => {
                let Some((&now_type, &next_type)) = continuations.next() else {
                    continue;
                };
                let Some(source_expression) = *expression else {
                    let binder = match binder {
                        Some(source) => match state.checked.core.lookup_binder(*source) {
                            Some(binder) => binder,
                            None => record_malformed_binder(state, *source, *binder_type),
                        },
                        None => allocate_missing_binder(state, *binder_type),
                    };
                    let expression = allocate_missing_expression(state, missing_expression_type);
                    let error = CheckedErrorStatement {
                        source: *statement,
                        binder: Some(binder),
                        expression: Some(expression),
                    };
                    let statement = CheckedBlockStatement::Error(error);
                    checked_steps.push(CheckedDoStep::Statement(statement));
                    continue;
                };
                let inferred =
                    state.with_error_crumb(ErrorCrumb::InferringDoBind(*statement), |state| {
                        let inferred = infer_do_bind_core(
                            state,
                            context,
                            bind_type,
                            next_type,
                            source_expression,
                            *binder_type,
                        )?;
                        unification::subtype(state, context, inferred.type_id, now_type)?;
                        Ok(inferred)
                    })?;
                let binder = match binder {
                    Some(source) => match state.checked.core.lookup_binder(*source) {
                        Some(binder) => binder,
                        None => record_malformed_binder(state, *source, *binder_type),
                    },
                    None => allocate_missing_binder(state, *binder_type),
                };
                let expression = match state.checked.core.lookup_expression(source_expression) {
                    Some(expression) => expression,
                    None => record_malformed_expression(state, source_expression, now_type),
                };
                let application = application::record_binary_application(
                    state,
                    bind,
                    bind_type,
                    inferred.outcome,
                );
                checked_steps.push(CheckedDoStep::Bind {
                    binder,
                    expression,
                    continuation_type: next_type,
                    application,
                });
            }
            DoStep::Discard { statement, expression } => {
                let Some((&now_type, &next_type)) = continuations.next() else {
                    continue;
                };
                let Some(source_expression) = *expression else {
                    let expression = allocate_missing_expression(state, missing_expression_type);
                    let error = CheckedErrorStatement {
                        source: *statement,
                        binder: None,
                        expression: Some(expression),
                    };
                    let statement = CheckedBlockStatement::Error(error);
                    checked_steps.push(CheckedDoStep::Statement(statement));
                    continue;
                };
                let (binder_type, inferred) = state.with_error_crumb(
                    ErrorCrumb::InferringDoDiscard(*statement),
                    |state| {
                        let (binder_type, inferred) = infer_do_discard_core(
                            state,
                            context,
                            discard_type,
                            next_type,
                            source_expression,
                        )?;
                        unification::subtype(state, context, inferred.type_id, now_type)?;
                        Ok((binder_type, inferred))
                    },
                )?;
                let binder = state
                    .checked
                    .core
                    .allocate_synthesized_binder(binder_type, CheckedBinderKind::Wildcard);
                let expression = match state.checked.core.lookup_expression(source_expression) {
                    Some(expression) => expression,
                    None => record_malformed_expression(state, source_expression, now_type),
                };
                let application = application::record_binary_application(
                    state,
                    discard,
                    discard_type,
                    inferred.outcome,
                );
                checked_steps.push(CheckedDoStep::Discard {
                    binder,
                    expression,
                    continuation_type: next_type,
                    application,
                });
            }
        }
    }

    // The `first_continuation` is the overall type of the do expression,
    // built iteratively and through solving unification variables. On
    // the other hand, the `final_continuation` is the expected type for
    // the final statement in the do expression. If there is only a single
    // statement in the do expression, then these two bindings are equivalent.
    let first_continuation =
        *continuation_types.first().expect("invariant violated: empty continuation_types");
    let final_continuation =
        *continuation_types.last().expect("invariant violated: empty continuation_types");

    if let Some(final_expression) = final_expression {
        super::check_expression(state, context, final_expression, final_continuation)?;
    }

    let final_expression = match final_step {
        DoBlockFinalStep::InvalidBind { statement, binder, binder_type, expression } => {
            let binder = match binder {
                Some(source) => match state.checked.core.lookup_binder(source) {
                    Some(binder) => binder,
                    None => record_malformed_binder(state, source, binder_type),
                },
                None => allocate_missing_binder(state, binder_type),
            };
            let expression = match expression {
                Some(source) => match state.checked.core.lookup_expression(source) {
                    Some(expression) => expression,
                    None => record_malformed_expression(
                        state,
                        source,
                        context.unknown("malformed final do expression"),
                    ),
                },
                None => allocate_missing_expression(
                    state,
                    context.unknown("missing final do expression"),
                ),
            };
            let error = CheckedErrorStatement {
                source: statement,
                binder: Some(binder),
                expression: Some(expression),
            };
            let statement = CheckedBlockStatement::Error(error);
            checked_steps.push(CheckedDoStep::Statement(statement));
            None
        }
        DoBlockFinalStep::Discard { statement: _, expression: Some(expression) } => {
            Some(expression)
        }
        DoBlockFinalStep::Discard { statement, expression: None } => {
            let expression =
                allocate_missing_expression(state, context.unknown("missing final do expression"));
            let error = CheckedErrorStatement {
                source: statement,
                binder: None,
                expression: Some(expression),
            };
            let statement = CheckedBlockStatement::Error(error);
            checked_steps.push(CheckedDoStep::Statement(statement));
            None
        }
        DoBlockFinalStep::InvalidLet { .. } => None,
        DoBlockFinalStep::Empty => unreachable!("empty do blocks return before checking steps"),
    };
    let final_expression = match final_expression {
        Some(source) => match state.checked.core.lookup_expression(source) {
            Some(expression) => expression,
            None => record_malformed_expression(state, source, final_continuation),
        },
        None => allocate_missing_expression(state, final_continuation),
    };
    record_do_expression(
        state,
        source_expression,
        checked_steps,
        final_expression,
        first_continuation,
    );

    Ok(first_continuation)
}

fn record_do_expression(
    state: &mut CheckState,
    source_expression: lowering::ExpressionId,
    steps: Vec<CheckedDoStep>,
    final_expression: CheckedExpressionId,
    result_type: TypeId,
) {
    let expression = CheckedDoExpression { steps: Arc::from(steps), final_expression };
    let kind = CheckedExpressionKind::Do { expression };
    let expression = state.checked.core.allocate_expression(result_type, kind);
    state.checked.core.record_expression(source_expression, expression);
}

pub(super) fn allocate_missing_expression(
    state: &mut CheckState,
    type_id: TypeId,
) -> CheckedExpressionId {
    state.checked.core.allocate_expression(type_id, CheckedExpressionKind::Error)
}

pub(super) fn record_malformed_expression(
    state: &mut CheckState,
    source: lowering::ExpressionId,
    fallback_type: TypeId,
) -> CheckedExpressionId {
    let type_id = state.checked.nodes.lookup_expression(source).unwrap_or(fallback_type);
    let expression = state.checked.core.allocate_expression(type_id, CheckedExpressionKind::Error);
    state.checked.core.record_expression(source, expression);
    expression
}

pub(super) fn allocate_missing_binder(
    state: &mut CheckState,
    type_id: TypeId,
) -> crate::semantic::CheckedBinderId {
    state.checked.core.allocate_synthesized_binder(type_id, CheckedBinderKind::Error)
}

pub(super) fn record_malformed_binder(
    state: &mut CheckState,
    source: lowering::BinderId,
    fallback_type: TypeId,
) -> crate::semantic::CheckedBinderId {
    let type_id = state.checked.nodes.lookup_binder(source).unwrap_or(fallback_type);
    state.checked.core.allocate_source_binder(source, type_id, CheckedBinderKind::Error)
}

pub(super) fn infer_do_bind_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    bind_type: TypeId,
    continuation_type: TypeId,
    expression: lowering::ExpressionId,
    binder_type: TypeId,
) -> QueryResult<application::InferredBinaryApplication>
where
    Q: ExternalQueries,
{
    let expression_type = super::infer_expression(state, context, expression)?;
    let lambda_type = context.intern_function(binder_type, continuation_type);
    application::check_binary_application(state, context, bind_type, expression_type, lambda_type)
}

fn infer_do_discard_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    discard_type: TypeId,
    continuation_type: TypeId,
    expression: lowering::ExpressionId,
) -> QueryResult<(TypeId, application::InferredBinaryApplication)>
where
    Q: ExternalQueries,
{
    let binder_type = state.fresh_unification(context.queries, context.prim.t);
    let application = infer_do_bind_core(
        state,
        context,
        discard_type,
        continuation_type,
        expression,
        binder_type,
    )?;
    Ok((binder_type, application))
}
