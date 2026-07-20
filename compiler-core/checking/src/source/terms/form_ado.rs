use std::sync::Arc;

use building_types::QueryResult;
use itertools::Itertools;

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::{TypeId, unification};
use crate::error::{ErrorCrumb, ErrorKind};
use crate::semantic::{
    CheckedAdoExpression, CheckedAdoStep, CheckedApplication, CheckedBinderId, CheckedBinderKind,
    CheckedBlockStatement, CheckedErrorStatement, CheckedExpressionId, CheckedExpressionKind,
    CheckedUnaryApplication,
};
use crate::source::binder;
use crate::source::terms::{application, form_do, form_let};
use crate::state::CheckState;

#[derive(Clone, Copy)]
enum AdoBinder {
    Bind(Option<lowering::BinderId>),
    Discard,
}

enum AdoStep<'a> {
    Error {
        statement: lowering::DoStatementId,
    },
    Action {
        statement: lowering::DoStatementId,
        binder: AdoBinder,
        binder_type: TypeId,
        expression: Option<lowering::ExpressionId>,
    },
    Let {
        statement: lowering::DoStatementId,
        statements: &'a [lowering::LetBindingChunk],
    },
}

enum AdoApplicationKind {
    Map,
    Apply,
}

pub struct AdoFunctions {
    pub map: Option<lowering::TermVariableResolution>,
    pub apply: Option<lowering::TermVariableResolution>,
    pub pure: Option<lowering::TermVariableResolution>,
}

struct InferredUnaryApplication {
    type_id: TypeId,
    application: Option<CheckedApplication>,
}

pub fn infer_ado<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    source_expression: lowering::ExpressionId,
    functions: AdoFunctions,
    statement_ids: &[lowering::DoStatementId],
    expression: Option<lowering::ExpressionId>,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let AdoFunctions { map, apply, pure } = functions;

    // First, perform a forward pass where variable bindings are bound
    // to unification variables. Let bindings are not checked here to
    // avoid premature solving of unification variables. Instead, they
    // are checked inline during the statement checking loop.
    let mut steps = vec![];
    for &statement_id in statement_ids.iter() {
        let Some(statement) = context.lowered.info.get_do_statement(statement_id) else {
            steps.push(AdoStep::Error { statement: statement_id });
            continue;
        };
        match statement {
            lowering::DoStatement::Bind { binder, expression } => {
                let binder_type = if let Some(binder) = binder {
                    binder::infer_binder(state, context, *binder)?
                } else {
                    state.fresh_unification(context.queries, context.prim.t)
                };
                let binder = AdoBinder::Bind(*binder);
                steps.push(AdoStep::Action {
                    statement: statement_id,
                    binder,
                    binder_type,
                    expression: *expression,
                });
            }
            lowering::DoStatement::Let { statements } => {
                steps.push(AdoStep::Let { statement: statement_id, statements });
            }
            lowering::DoStatement::Discard { expression } => {
                let binder_type = state.fresh_unification(context.queries, context.prim.t);
                steps.push(AdoStep::Action {
                    statement: statement_id,
                    binder: AdoBinder::Discard,
                    binder_type,
                    expression: *expression,
                });
            }
        }
    }

    let binder_types = steps.iter().filter_map(|step| match step {
        AdoStep::Action { binder_type, expression: Some(_), .. } => Some(*binder_type),
        AdoStep::Error { .. } | AdoStep::Action { expression: None, .. } | AdoStep::Let { .. } => {
            None
        }
    });
    let binder_types = binder_types.collect_vec();
    let has_source_actions = steps.iter().any(|step| matches!(step, AdoStep::Action { .. }));

    // For ado blocks with no bindings, we check let statements and then
    // apply pure to the expression.
    //
    //   pure_type  := a -> f a
    //   expression := t
    if binder_types.is_empty() {
        for step in &steps {
            if let AdoStep::Let { statement, statements } = step {
                state.with_error_crumb(ErrorCrumb::CheckingAdoLet(*statement), |state| {
                    form_let::check_let_chunks(state, context, statements)
                })?;
            }
        }
        let missing_expression_type = context.unknown("missing ado action");
        let statements =
            checked_ado_recovery_statements(state, context, &steps, missing_expression_type);
        if has_source_actions {
            let result_type = context.unknown("malformed ado actions");
            let expression = if let Some(expression) = expression {
                super::infer_expression(state, context, expression)?;
                match state.checked.core.lookup_expression(expression) {
                    Some(expression) => expression,
                    None => form_do::record_malformed_expression(state, expression, result_type),
                }
            } else {
                form_do::allocate_missing_expression(state, result_type)
            };
            record_ado_error_expression(
                state,
                source_expression,
                statements,
                expression,
                result_type,
            );
            return Ok(result_type);
        }
        return if let Some(expression) = expression {
            let pure_type = form_do::lookup_or_synthesise_pure(state, context, pure)?;
            let inferred = infer_ado_pure_core(state, context, pure_type, expression)?;
            let result_type = inferred.type_id;
            let expression = match state.checked.core.lookup_expression(expression) {
                Some(expression) => expression,
                None => form_do::record_malformed_expression(state, expression, result_type),
            };
            record_ado_pure_expression(
                state,
                source_expression,
                statements,
                pure,
                pure_type,
                expression,
                inferred,
            );
            Ok(result_type)
        } else {
            state.insert_error(ErrorKind::EmptyAdoBlock);
            let result_type = context.unknown("empty ado block");
            let inferred = InferredUnaryApplication { type_id: result_type, application: None };
            let expression = form_do::allocate_missing_expression(state, result_type);
            record_ado_pure_expression(
                state,
                source_expression,
                statements,
                None,
                context.unknown("missing pure application"),
                expression,
                inferred,
            );
            Ok(result_type)
        };
    }

    // Create a fresh unification variable for the in_expression.
    // Inferring expression directly may solve the unification variables
    // introduced in the first pass. This is undesirable, because the
    // errors would be attributed incorrectly to the ado statements
    // rather than the in-expression itself.
    //
    //   ado
    //     a <- pure "Hello!"
    //     _ <- pure 42
    //     in Message a
    //
    //   in_expression      :: Effect Message
    //   in_expression_type := ?in_expression
    //   lambda_type        := ?a -> ?b -> ?in_expression
    let in_expression_type = state.fresh_unification(context.queries, context.prim.t);
    let lambda_type = context.intern_function_list(&binder_types, in_expression_type);

    // The desugared form of an ado-expression is a forward applicative
    // pipeline, unlike do-notation which works inside-out. The example
    // above desugars to the following expression:
    //
    //   (\a _ -> Message a) <$> (pure "Hello!") <*> (pure 42)
    //
    // To emulate this, we process steps in source order. Let bindings
    // are checked inline between map/apply operations. The first action
    // uses infer_ado_map, and subsequent actions use infer_ado_apply.
    //
    //   map_type        :: (a -> b) -> f a -> f b
    //   lambda_type     := ?a -> ?b -> ?in_expression
    //
    //   expression_type         := Effect String
    //   map(lambda, expression) := Effect (?b -> ?in_expression)
    //                           >>
    //                           >> ?a := String
    //
    //   continuation_type := Effect (?b -> ?in_expression)

    // Lazily compute map_type and apply_type only when needed.
    // - 1 action: only map is needed
    // - 2+ actions: map and apply are needed
    let action_count = binder_types.len();

    let map_type = form_do::lookup_or_synthesise_map(state, context, map)?;

    let apply_type = if action_count > 1 {
        form_do::lookup_or_synthesise_apply(state, context, apply)?
    } else {
        context.unknown("unused apply")
    };

    let mut continuation_type = None;
    let mut checked_steps = vec![];
    let missing_expression_type = context.unknown("missing ado action");

    for step in &steps {
        match step {
            AdoStep::Error { statement } => {
                let error =
                    CheckedErrorStatement { source: *statement, binder: None, expression: None };
                let statement = CheckedBlockStatement::Error(error);
                checked_steps.push(CheckedAdoStep::Statement(statement));
            }
            AdoStep::Let { statement, statements } => {
                state.with_error_crumb(ErrorCrumb::CheckingAdoLet(*statement), |state| {
                    form_let::check_let_chunks(state, context, statements)
                })?;
                let statement =
                    form_let::checked_let_statement(state, context, *statement, statements);
                let statement = CheckedBlockStatement::Let(statement);
                checked_steps.push(CheckedAdoStep::Statement(statement));
            }
            AdoStep::Action { statement, binder, binder_type, expression } => {
                let Some(source_expression) = *expression else {
                    let binder = checked_ado_error_binder(state, binder, *binder_type);
                    let expression =
                        form_do::allocate_missing_expression(state, missing_expression_type);
                    let error = CheckedErrorStatement {
                        source: *statement,
                        binder,
                        expression: Some(expression),
                    };
                    let statement = CheckedBlockStatement::Error(error);
                    checked_steps.push(CheckedAdoStep::Statement(statement));
                    continue;
                };
                let (inferred, kind, function, function_type) =
                    if let Some(continuation_type) = continuation_type {
                        // Then, the infer_ado_apply rule applies `apply` to the inferred
                        // expression type and the continuation type that is a function
                        // contained within some container, like Effect.
                        //
                        //   apply_type        := f (x -> y) -> f x -> f y
                        //   continuation_type := Effect (?b -> ?in_expression)
                        //
                        //   expression_type                 := Effect Int
                        //   apply(continuation, expression) := Effect ?in_expression
                        //                                   >>
                        //                                   >> ?b := Int
                        //
                        //   continuation_type := Effect ?in_expression
                        let inferred = state.with_error_crumb(
                            ErrorCrumb::InferringAdoApply(*statement),
                            |state| {
                                infer_ado_apply_core(
                                    state,
                                    context,
                                    apply_type,
                                    continuation_type,
                                    source_expression,
                                )
                            },
                        )?;
                        (inferred, AdoApplicationKind::Apply, apply, apply_type)
                    } else {
                        let inferred = state.with_error_crumb(
                            ErrorCrumb::InferringAdoMap(*statement),
                            |state| {
                                infer_ado_map_core(
                                    state,
                                    context,
                                    map_type,
                                    lambda_type,
                                    source_expression,
                                )
                            },
                        )?;
                        (inferred, AdoApplicationKind::Map, map, map_type)
                    };
                continuation_type = Some(inferred.type_id);
                let binder = match binder {
                    AdoBinder::Bind(Some(source)) => {
                        match state.checked.core.lookup_binder(*source) {
                            Some(binder) => binder,
                            None => form_do::record_malformed_binder(state, *source, *binder_type),
                        }
                    }
                    AdoBinder::Bind(None) => form_do::allocate_missing_binder(state, *binder_type),
                    AdoBinder::Discard => state
                        .checked
                        .core
                        .allocate_synthesized_binder(*binder_type, CheckedBinderKind::Wildcard),
                };
                let expression = match state.checked.core.lookup_expression(source_expression) {
                    Some(expression) => expression,
                    None => form_do::record_malformed_expression(
                        state,
                        source_expression,
                        missing_expression_type,
                    ),
                };
                let application = application::record_binary_application(
                    state,
                    function,
                    function_type,
                    inferred.outcome,
                );
                let step = match kind {
                    AdoApplicationKind::Map => {
                        CheckedAdoStep::Map { binder, expression, application }
                    }
                    AdoApplicationKind::Apply => {
                        CheckedAdoStep::Apply { binder, expression, application }
                    }
                };
                checked_steps.push(step);
            }
        }
    }

    // Finally, check the in-expression against in_expression.
    // At this point the binder unification variables have been solved
    // to concrete types, so errors are attributed to the in-expression.
    //
    //   in_expression      :: Effect Message
    //   in_expression_type := Effect ?in_expression
    //                      >>
    //                      >> ?in_expression := Message
    if let Some(expression) = expression {
        super::check_expression(state, context, expression, in_expression_type)?;
    }

    let Some(continuation_type) = continuation_type else {
        unreachable!("invariant violated: impossible empty steps");
    };

    let expression = match expression {
        Some(source) => match state.checked.core.lookup_expression(source) {
            Some(expression) => expression,
            None => form_do::record_malformed_expression(state, source, in_expression_type),
        },
        None => form_do::allocate_missing_expression(state, in_expression_type),
    };

    record_ado_actions_expression(
        state,
        source_expression,
        checked_steps,
        expression,
        lambda_type,
        continuation_type,
    );

    Ok(continuation_type)
}

fn infer_ado_pure_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    pure_type: TypeId,
    expression: lowering::ExpressionId,
) -> QueryResult<InferredUnaryApplication>
where
    Q: ExternalQueries,
{
    let expression_type = super::infer_expression(state, context, expression)?;
    let Some(application) = application::check_generic_application(state, context, pure_type)?
    else {
        let type_id = context.unknown("invalid function application");
        return Ok(InferredUnaryApplication { type_id, application: None });
    };
    unification::subtype(state, context, expression_type, application.argument)?;
    Ok(InferredUnaryApplication { type_id: application.result, application: Some(application) })
}

fn record_ado_pure_expression(
    state: &mut CheckState,
    source_expression: lowering::ExpressionId,
    statements: Vec<CheckedBlockStatement>,
    function: Option<lowering::TermVariableResolution>,
    function_type: TypeId,
    expression: CheckedExpressionId,
    inferred: InferredUnaryApplication,
) {
    let kind = function.map_or(CheckedExpressionKind::Error, |resolution| {
        CheckedExpressionKind::Variable { resolution }
    });
    let function = state.checked.core.allocate_expression(function_type, kind);
    let application = match inferred.application {
        Some(application) => CheckedUnaryApplication::Complete { function, application },
        None => CheckedUnaryApplication::Error { function },
    };

    let statements = Arc::from(statements);

    let expression = CheckedAdoExpression::Pure { statements, expression, application };
    let kind = CheckedExpressionKind::Ado { expression };

    let expression = state.checked.core.allocate_expression(inferred.type_id, kind);
    state.checked.core.record_expression(source_expression, expression);
}

fn record_ado_actions_expression(
    state: &mut CheckState,
    source_expression: lowering::ExpressionId,
    steps: Vec<CheckedAdoStep>,
    expression: CheckedExpressionId,
    lambda_type: TypeId,
    result_type: TypeId,
) {
    let expression =
        CheckedAdoExpression::Actions { steps: Arc::from(steps), expression, lambda_type };
    let kind = CheckedExpressionKind::Ado { expression };
    let expression = state.checked.core.allocate_expression(result_type, kind);
    state.checked.core.record_expression(source_expression, expression);
}

fn record_ado_error_expression(
    state: &mut CheckState,
    source_expression: lowering::ExpressionId,
    statements: Vec<CheckedBlockStatement>,
    expression: CheckedExpressionId,
    result_type: TypeId,
) {
    let expression = CheckedAdoExpression::Error { statements: statements.into(), expression };
    let kind = CheckedExpressionKind::Ado { expression };
    let expression = state.checked.core.allocate_expression(result_type, kind);
    state.checked.core.record_expression(source_expression, expression);
}

fn checked_ado_error_binder(
    state: &mut CheckState,
    binder: &AdoBinder,
    binder_type: TypeId,
) -> Option<CheckedBinderId> {
    match binder {
        AdoBinder::Bind(Some(source)) => Some(match state.checked.core.lookup_binder(*source) {
            Some(binder) => binder,
            None => form_do::record_malformed_binder(state, *source, binder_type),
        }),
        AdoBinder::Bind(None) => Some(form_do::allocate_missing_binder(state, binder_type)),
        AdoBinder::Discard => None,
    }
}

fn checked_ado_recovery_statements<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    steps: &[AdoStep<'_>],
    missing_expression_type: TypeId,
) -> Vec<CheckedBlockStatement>
where
    Q: ExternalQueries,
{
    let checked_steps = steps.iter().filter_map(|step| {
        let statement = match step {
            AdoStep::Error { statement } => {
                let error =
                    CheckedErrorStatement { source: *statement, binder: None, expression: None };
                CheckedBlockStatement::Error(error)
            }
            AdoStep::Let { statement, statements } => {
                let statement =
                    form_let::checked_let_statement(state, context, *statement, statements);
                CheckedBlockStatement::Let(statement)
            }
            AdoStep::Action { statement, binder, binder_type, expression: None } => {
                let binder = checked_ado_error_binder(state, binder, *binder_type);
                let expression =
                    form_do::allocate_missing_expression(state, missing_expression_type);
                let error = CheckedErrorStatement {
                    source: *statement,
                    binder,
                    expression: Some(expression),
                };
                CheckedBlockStatement::Error(error)
            }
            AdoStep::Action { expression: Some(_), .. } => return None,
        };
        Some(statement)
    });
    checked_steps.collect_vec()
}

fn infer_ado_map_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    map_type: TypeId,
    lambda_type: TypeId,
    expression: lowering::ExpressionId,
) -> QueryResult<application::InferredBinaryApplication>
where
    Q: ExternalQueries,
{
    let expression_type = super::infer_expression(state, context, expression)?;
    application::check_binary_application(state, context, map_type, lambda_type, expression_type)
}

fn infer_ado_apply_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    apply_type: TypeId,
    continuation_type: TypeId,
    expression: lowering::ExpressionId,
) -> QueryResult<application::InferredBinaryApplication>
where
    Q: ExternalQueries,
{
    let expression_type = super::infer_expression(state, context, expression)?;
    application::check_binary_application(
        state,
        context,
        apply_type,
        continuation_type,
        expression_type,
    )
}
