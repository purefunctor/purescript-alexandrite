use std::sync::Arc;

use building_types::QueryResult;

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::{TypeId, exhaustive, toolkit, unification};
use crate::semantic::{
    CheckedBinderKind, CheckedCaseAlternative, CheckedExpressionKind, CheckedGuardedExpression,
    CheckedLiteral, CheckedPatternGuard,
};
use crate::source::terms::{form_let, guarded};
use crate::source::{binder, terms};
use crate::state::CheckState;

#[derive(Copy, Clone, Debug)]
enum IfThenElseMode {
    Infer,
    Check { expected: TypeId },
}

#[derive(Copy, Clone, Debug)]
enum CaseOfMode {
    Infer,
    Check { expected: TypeId },
}

pub fn infer_if_then_else<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
    if_: Option<lowering::ExpressionId>,
    then: Option<lowering::ExpressionId>,
    else_: Option<lowering::ExpressionId>,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    if_then_else_core(state, context, expression, if_, then, else_, IfThenElseMode::Infer)
}

pub fn check_if_then_else<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
    if_: Option<lowering::ExpressionId>,
    then: Option<lowering::ExpressionId>,
    else_: Option<lowering::ExpressionId>,
    expected: TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    if_then_else_core(
        state,
        context,
        expression,
        if_,
        then,
        else_,
        IfThenElseMode::Check { expected },
    )
}

fn if_then_else_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
    if_: Option<lowering::ExpressionId>,
    then: Option<lowering::ExpressionId>,
    else_: Option<lowering::ExpressionId>,
    mode: IfThenElseMode,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    if let Some(if_) = if_ {
        super::check_expression(state, context, if_, context.prim.boolean)?;
    }

    let result_type = match mode {
        IfThenElseMode::Infer => state.fresh_unification(context.queries, context.prim.t),
        IfThenElseMode::Check { expected } => expected,
    };

    if let Some(then) = then {
        super::check_expression(state, context, then, result_type)?;
    }

    if let Some(else_) = else_ {
        super::check_expression(state, context, else_, result_type)?;
    }

    record_if_then_else(state, context.prim.boolean, expression, if_, then, else_, result_type);

    Ok(result_type)
}

fn record_if_then_else(
    state: &mut CheckState,
    boolean_type: TypeId,
    expression: lowering::ExpressionId,
    if_: Option<lowering::ExpressionId>,
    then: Option<lowering::ExpressionId>,
    else_: Option<lowering::ExpressionId>,
    result_type: TypeId,
) {
    let Some(if_) = if_.and_then(|if_| state.checked.core.lookup_expression(if_)) else {
        return;
    };
    let Some(then) = then.and_then(|then| state.checked.core.lookup_expression(then)) else {
        return;
    };
    let Some(else_) = else_.and_then(|else_| state.checked.core.lookup_expression(else_)) else {
        return;
    };

    let true_binder = state.checked.core.allocate_synthesized_binder(
        boolean_type,
        CheckedBinderKind::Literal(CheckedLiteral::Boolean(true)),
    );
    let false_binder = state.checked.core.allocate_synthesized_binder(
        boolean_type,
        CheckedBinderKind::Literal(CheckedLiteral::Boolean(false)),
    );
    let then_result = CheckedGuardedExpression { guards: Arc::from([]), expression: then };
    let else_result = CheckedGuardedExpression { guards: Arc::from([]), expression: else_ };
    let alternatives = [
        CheckedCaseAlternative {
            binders: Arc::from([true_binder]),
            results: Arc::from([then_result]),
        },
        CheckedCaseAlternative {
            binders: Arc::from([false_binder]),
            results: Arc::from([else_result]),
        },
    ];
    let kind = CheckedExpressionKind::Case {
        scrutinees: Arc::from([if_]),
        alternatives: Arc::from(alternatives),
    };
    let checked_expression = state.checked.core.allocate_expression(result_type, kind);
    state.checked.core.record_expression(expression, checked_expression);
}

pub fn infer_lambda<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    lambda: lowering::ExpressionId,
    binders: &[lowering::BinderId],
    expression: Option<lowering::ExpressionId>,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let mut argument_types = vec![];

    for &binder_id in binders.iter() {
        let argument_type = state.fresh_unification(context.queries, context.prim.t);
        binder::check_binder(state, context, binder_id, argument_type)?;
        argument_types.push(argument_type);
    }

    let result_type = if let Some(body) = expression {
        let body_type = super::infer_expression(state, context, body)?;
        toolkit::instantiate_constrained(state, context, body_type)?
    } else {
        state.fresh_unification(context.queries, context.prim.t)
    };

    let function_type = context.intern_function_list(&argument_types, result_type);

    let exhaustiveness =
        exhaustive::check_lambda_patterns(state, context, &argument_types, binders)?;

    let has_missing = exhaustiveness.missing.is_some();
    state.report_exhaustiveness(exhaustiveness);

    let lambda_type = if has_missing {
        context.intern_constrained(context.prim.partial, function_type)
    } else {
        function_type
    };

    record_lambda(state, lambda, binders, expression, lambda_type);
    Ok(lambda_type)
}

pub fn check_lambda<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    lambda: lowering::ExpressionId,
    binders: &[lowering::BinderId],
    expression: Option<lowering::ExpressionId>,
    expected: TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let mut arguments = vec![];
    let mut remaining = expected;

    for &binder_id in binders.iter() {
        let decomposed = toolkit::decompose_function(state, context, remaining)?;
        if let Some((argument, result)) = decomposed {
            let argument = if binder::requires_instantiation(context, binder_id) {
                toolkit::instantiate_unifications(state, context, argument)?
            } else {
                argument
            };
            binder::check_binder(state, context, binder_id, argument)?;
            arguments.push(argument);
            remaining = result;
        } else {
            let argument_type = state.fresh_unification(context.queries, context.prim.t);
            binder::check_binder(state, context, binder_id, argument_type)?;
            arguments.push(argument_type);
        }
    }

    let result_type = if let Some(body) = expression {
        super::check_expression(state, context, body, remaining)?
    } else {
        state.fresh_unification(context.queries, context.prim.t)
    };

    let function_type = context.intern_function_list(&arguments, result_type);

    let exhaustiveness = exhaustive::check_lambda_patterns(state, context, &arguments, binders)?;

    let has_missing = exhaustiveness.missing.is_some();
    state.report_exhaustiveness(exhaustiveness);

    if has_missing {
        state.push_wanted(context.prim.partial);
    }

    record_lambda(state, lambda, binders, expression, function_type);
    Ok(function_type)
}

fn record_lambda(
    state: &mut CheckState,
    lambda: lowering::ExpressionId,
    binders: &[lowering::BinderId],
    expression: Option<lowering::ExpressionId>,
    type_id: TypeId,
) {
    let Some(expression) =
        expression.and_then(|expression| state.checked.core.lookup_expression(expression))
    else {
        return;
    };

    let checked_binders = binders.iter().map(|binder| state.checked.core.lookup_binder(*binder));
    let checked_binders = checked_binders.collect::<Option<Vec<_>>>();
    let Some(checked_binders) = checked_binders else { return };

    if checked_binders.is_empty() {
        state.checked.core.record_expression(lambda, expression);
        return;
    }

    let kind = CheckedExpressionKind::Lambda { binders: Arc::from(checked_binders), expression };
    let checked_lambda = state.checked.core.allocate_expression(type_id, kind);
    state.checked.core.record_expression(lambda, checked_lambda);
}

pub fn instantiate_trunk_types<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    trunk_types: &mut [TypeId],
    branches: &[lowering::CaseBranch],
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    binder::instantiate_pattern_column_types(
        state,
        context,
        trunk_types,
        branches.iter().flat_map(|branch| branch.binders.iter().copied().enumerate()),
    )
}

pub fn infer_case_of<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    case: lowering::ExpressionId,
    trunk: &[lowering::ExpressionId],
    branches: &[lowering::CaseBranch],
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    case_of_core(state, context, case, trunk, branches, CaseOfMode::Infer)
}

pub fn check_case_of<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    case: lowering::ExpressionId,
    trunk: &[lowering::ExpressionId],
    branches: &[lowering::CaseBranch],
    expected: TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    case_of_core(state, context, case, trunk, branches, CaseOfMode::Check { expected })
}

fn case_of_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    case: lowering::ExpressionId,
    trunk: &[lowering::ExpressionId],
    branches: &[lowering::CaseBranch],
    mode: CaseOfMode,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let expected = match mode {
        CaseOfMode::Infer => state.fresh_unification(context.queries, context.prim.t),
        CaseOfMode::Check { expected } => expected,
    };

    let mut trunk_types = vec![];
    for trunk in trunk.iter() {
        let trunk_type = super::infer_expression(state, context, *trunk)?;
        let trunk_type = toolkit::instantiate_constrained(state, context, trunk_type)?;
        trunk_types.push(trunk_type);
    }

    instantiate_trunk_types(state, context, &mut trunk_types, branches)?;

    for branch in branches.iter() {
        for (binder, trunk) in branch.binders.iter().zip(&trunk_types) {
            binder::check_binder(state, context, *binder, *trunk)?;
        }
        if let Some(guarded) = &branch.guarded_expression {
            match mode {
                CaseOfMode::Infer => {
                    let guarded_type = guarded::infer_guarded_expression(state, context, guarded)?;
                    unification::subtype(state, context, guarded_type, expected)?;
                }
                CaseOfMode::Check { .. } => {
                    guarded::check_guarded_expression(state, context, guarded, expected)?;
                }
            }
        }
    }

    let exhaustiveness = exhaustive::check_case_patterns(state, context, &trunk_types, branches)?;

    let has_missing = exhaustiveness.missing.is_some();
    state.report_exhaustiveness(exhaustiveness);

    let result_type = if has_missing {
        match mode {
            CaseOfMode::Infer => context.intern_constrained(context.prim.partial, expected),
            CaseOfMode::Check { .. } => {
                state.push_wanted(context.prim.partial);
                expected
            }
        }
    } else {
        expected
    };

    record_case(state, case, trunk, branches, result_type);

    Ok(result_type)
}

fn record_case(
    state: &mut CheckState,
    case: lowering::ExpressionId,
    trunk: &[lowering::ExpressionId],
    branches: &[lowering::CaseBranch],
    case_type: TypeId,
) {
    if trunk.is_empty() || branches.is_empty() {
        return;
    }

    let scrutinees =
        trunk.iter().map(|expression| state.checked.core.lookup_expression(*expression));
    let Some(scrutinees) = scrutinees.collect::<Option<Vec<_>>>() else { return };

    let mut alternatives = Vec::with_capacity(branches.len());
    for branch in branches {
        if branch.binders.len() != trunk.len() {
            return;
        }

        let binders = branch.binders.iter().map(|binder| state.checked.core.lookup_binder(*binder));
        let Some(binders) = binders.collect::<Option<Vec<_>>>() else { return };
        let Some(results) = checked_guarded_expressions(state, &branch.guarded_expression) else {
            return;
        };

        alternatives.push(CheckedCaseAlternative {
            binders: Arc::from(binders),
            results: Arc::from(results),
        });
    }

    let kind = CheckedExpressionKind::Case {
        scrutinees: Arc::from(scrutinees),
        alternatives: Arc::from(alternatives),
    };
    let checked_case = state.checked.core.allocate_expression(case_type, kind);
    state.checked.core.record_expression(case, checked_case);
}

fn checked_guarded_expressions(
    state: &CheckState,
    guarded: &Option<lowering::GuardedExpression>,
) -> Option<Vec<CheckedGuardedExpression>> {
    match guarded.as_ref()? {
        lowering::GuardedExpression::Unconditional { where_expression } => {
            let expression = checked_where_expression(state, where_expression.as_ref()?)?;
            let guarded = CheckedGuardedExpression { guards: Arc::from([]), expression };
            Some(vec![guarded])
        }
        lowering::GuardedExpression::Conditionals { pattern_guarded } => {
            checked_conditional_guarded_expressions(state, pattern_guarded)
        }
    }
}

fn checked_conditional_guarded_expressions(
    state: &CheckState,
    guarded: &[lowering::PatternGuarded],
) -> Option<Vec<CheckedGuardedExpression>> {
    if guarded.is_empty() {
        return None;
    }

    let mut results = Vec::with_capacity(guarded.len());
    for guarded in guarded {
        results.push(checked_conditional_guarded_expression(state, guarded)?);
    }
    Some(results)
}

fn checked_conditional_guarded_expression(
    state: &CheckState,
    guarded: &lowering::PatternGuarded,
) -> Option<CheckedGuardedExpression> {
    if guarded.pattern_guards.is_empty() {
        return None;
    }

    let guards = guarded.pattern_guards.iter().map(|guard| checked_pattern_guard(state, guard));
    let guards = guards.collect::<Option<Vec<_>>>()?;

    let where_expression = guarded.where_expression.as_ref()?;
    let expression = checked_where_expression(state, where_expression)?;
    Some(CheckedGuardedExpression { guards: Arc::from(guards), expression })
}

fn checked_pattern_guard(
    state: &CheckState,
    guard: &lowering::PatternGuard,
) -> Option<CheckedPatternGuard> {
    let source_expression = guard.expression?;
    let expression = state.checked.core.lookup_expression(source_expression)?;
    match guard.binder {
        Some(source_binder) => {
            let binder = state.checked.core.lookup_binder(source_binder)?;
            Some(CheckedPatternGuard::Pattern { binder, expression })
        }
        None => Some(CheckedPatternGuard::Boolean { expression }),
    }
}

fn checked_where_expression(
    state: &CheckState,
    where_expression: &lowering::WhereExpression,
) -> Option<crate::semantic::CheckedExpressionId> {
    if !where_expression.bindings.is_empty() {
        return None;
    }

    let expression = where_expression.expression?;
    state.checked.core.lookup_expression(expression)
}

pub fn check_let_in<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    bindings: &[lowering::LetBindingChunk],
    expression: Option<lowering::ExpressionId>,
    expected: TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    form_let::check_let_chunks(state, context, bindings)?;

    let Some(expression) = expression else {
        return Ok(context.unknown("missing let expression"));
    };

    terms::check_expression(state, context, expression, expected)
}
