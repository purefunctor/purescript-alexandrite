use std::mem;
use std::ops::ControlFlow;
use std::sync::Arc;

use building_types::QueryResult;

use crate::context::CheckContext;
use crate::core::substitute::SubstituteName;
use crate::core::{ForallBinder, Type, TypeId, normalise, unification};
use crate::error::ErrorKind;
use crate::evidence::EvidenceVarId;
use crate::semantic::{
    CheckedApplication, CheckedBinaryApplication, CheckedExpressionId, CheckedExpressionKind,
};
use crate::source::types;
use crate::state::CheckState;
use crate::{ExternalQueries, safe_loop};

pub struct ApplicationAnalysis {
    pub constraints: Vec<TypeId>,
    pub argument: TypeId,
    pub result: TypeId,
}

pub(super) enum BinaryApplicationOutcome {
    Complete { first: CheckedApplication, second: CheckedApplication },
    Partial { first: CheckedApplication },
    Error,
}

pub(super) struct InferredBinaryApplication {
    pub type_id: TypeId,
    pub outcome: BinaryApplicationOutcome,
}

struct CheckedTypeApplication {
    argument: Option<TypeId>,
    result: TypeId,
}

fn analyse_function_application_step<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function: TypeId,
    constraints: &mut Vec<TypeId>,
) -> QueryResult<ControlFlow<Option<ApplicationAnalysis>, TypeId>>
where
    Q: ExternalQueries,
{
    match context.lookup_type(function) {
        Type::Function(argument, result) => {
            let analysis =
                ApplicationAnalysis { constraints: mem::take(constraints), argument, result };
            Ok(ControlFlow::Break(Some(analysis)))
        }

        Type::Unification(unification_id) => {
            let argument = state.fresh_unification(context.queries, context.prim.t);
            let result = state.fresh_unification(context.queries, context.prim.t);
            let function = context.intern_function(argument, result);

            unification::solve(state, context, function, unification_id, function)?;

            let analysis =
                ApplicationAnalysis { constraints: mem::take(constraints), argument, result };

            Ok(ControlFlow::Break(Some(analysis)))
        }

        Type::Forall(binder_id, inner) => {
            let binder = context.lookup_forall_binder(binder_id);
            let binder_kind = normalise::expand(state, context, binder.kind)?;

            let replacement = state.fresh_unification(context.queries, binder_kind);
            let function = SubstituteName::one(state, context, binder.name, replacement, inner)?;
            Ok(ControlFlow::Continue(function))
        }

        Type::Constrained(constraint, constrained) => {
            constraints.push(constraint);
            Ok(ControlFlow::Continue(constrained))
        }

        Type::Application(function_argument, result) => {
            let function_argument = normalise::expand(state, context, function_argument)?;

            let Type::Application(constructor, argument) = context.lookup_type(function_argument)
            else {
                return Ok(ControlFlow::Break(None));
            };

            let constructor = normalise::expand(state, context, constructor)?;
            if constructor == context.prim.function {
                let analysis =
                    ApplicationAnalysis { constraints: mem::take(constraints), argument, result };
                return Ok(ControlFlow::Break(Some(analysis)));
            }

            if let Type::Unification(unification_id) = context.lookup_type(constructor) {
                unification::solve(
                    state,
                    context,
                    constructor,
                    unification_id,
                    context.prim.function,
                )?;

                let analysis =
                    ApplicationAnalysis { constraints: mem::take(constraints), argument, result };

                return Ok(ControlFlow::Break(Some(analysis)));
            }

            Ok(ControlFlow::Break(None))
        }

        _ => Ok(ControlFlow::Break(None)),
    }
}

pub fn analyse_function_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut function: TypeId,
) -> QueryResult<Option<ApplicationAnalysis>>
where
    Q: ExternalQueries,
{
    let mut constraints = vec![];
    safe_loop! {
        function = normalise::expand(state, context, function)?;
        match analyse_function_application_step(state, context, function, &mut constraints)? {
            ControlFlow::Continue(next) => function = next,
            ControlFlow::Break(analysis) => return Ok(analysis),
        };
    }
}

pub fn check_generic_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function: TypeId,
) -> QueryResult<Option<CheckedApplication>>
where
    Q: ExternalQueries,
{
    let Some(ApplicationAnalysis { constraints, argument, result }) =
        analyse_function_application(state, context, function)?
    else {
        return Ok(None);
    };

    let evidence = constraints.into_iter().map(|constraint| state.push_wanted(constraint));
    let evidence = evidence.collect();

    Ok(Some(CheckedApplication { evidence, argument, result }))
}

pub(super) fn check_binary_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function_type: TypeId,
    first_argument_type: TypeId,
    second_argument_type: TypeId,
) -> QueryResult<InferredBinaryApplication>
where
    Q: ExternalQueries,
{
    let Some(first @ CheckedApplication { argument, result, .. }) =
        check_generic_application(state, context, function_type)?
    else {
        let type_id = context.unknown("invalid function application");
        let outcome = BinaryApplicationOutcome::Error;
        return Ok(InferredBinaryApplication { type_id, outcome });
    };
    unification::subtype(state, context, first_argument_type, argument)?;

    let Some(second @ CheckedApplication { argument, result, .. }) =
        check_generic_application(state, context, result)?
    else {
        let type_id = context.unknown("invalid function application");
        let outcome = BinaryApplicationOutcome::Partial { first };
        return Ok(InferredBinaryApplication { type_id, outcome });
    };
    unification::subtype(state, context, second_argument_type, argument)?;

    let outcome = BinaryApplicationOutcome::Complete { first, second };
    Ok(InferredBinaryApplication { type_id: result, outcome })
}

pub(super) fn record_binary_application(
    state: &mut CheckState,
    function: Option<lowering::TermVariableResolution>,
    function_type: TypeId,
    outcome: BinaryApplicationOutcome,
) -> CheckedBinaryApplication {
    let kind = function.map_or(CheckedExpressionKind::Error, |resolution| {
        CheckedExpressionKind::Variable { resolution }
    });
    let function = state.checked.core.allocate_expression(function_type, kind);
    match outcome {
        BinaryApplicationOutcome::Complete { first, second } => {
            CheckedBinaryApplication::Complete { function, first, second }
        }
        BinaryApplicationOutcome::Partial { first } => {
            CheckedBinaryApplication::Partial { function, first }
        }
        BinaryApplicationOutcome::Error => CheckedBinaryApplication::Error { function },
    }
}

pub fn check_function_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function_type: TypeId,
    argument: &lowering::ExpressionArgument,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    match argument {
        lowering::ExpressionArgument::Type(type_argument) => {
            let Some(type_argument) = type_argument else {
                return Ok(context.unknown("missing type argument"));
            };
            let result =
                check_function_type_application(state, context, function_type, *type_argument)?;
            Ok(result)
        }
        lowering::ExpressionArgument::Term(term_argument) => {
            let Some(term_argument) = term_argument else {
                return Ok(context.unknown("missing term argument"));
            };
            let result =
                check_function_term_application(state, context, function_type, *term_argument)?;
            Ok(result)
        }
    }
}

pub fn check_core_function_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function_type: TypeId,
    function_expression: Option<CheckedExpressionId>,
    argument: &lowering::ExpressionArgument,
) -> QueryResult<(TypeId, Option<CheckedExpressionId>)>
where
    Q: ExternalQueries,
{
    match argument {
        lowering::ExpressionArgument::Type(type_argument) => {
            let Some(type_argument) = type_argument else {
                let type_id = context.unknown("missing type argument");
                return Ok((type_id, None));
            };
            let application = check_core_function_type_application(
                state,
                context,
                function_type,
                *type_argument,
            )?;
            let expression = function_expression.zip(application.argument);
            let expression = expression.map(|(function, argument)| {
                let kind = CheckedExpressionKind::TypeApplication { function, argument };
                state.checked.core.allocate_expression(application.result, kind)
            });
            Ok((application.result, expression))
        }
        lowering::ExpressionArgument::Term(term_argument) => {
            let Some(term_argument) = term_argument else {
                let type_id = context.unknown("missing term argument");
                return Ok((type_id, None));
            };
            let application = check_core_function_term_application(
                state,
                context,
                function_type,
                function_expression,
                *term_argument,
            )?;
            Ok(application)
        }
    }
}

fn check_core_function_term_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function_type: TypeId,
    function_expression: Option<CheckedExpressionId>,
    argument_expression: lowering::ExpressionId,
) -> QueryResult<(TypeId, Option<CheckedExpressionId>)>
where
    Q: ExternalQueries,
{
    let Some(CheckedApplication { evidence, argument, result }) =
        check_generic_application(state, context, function_type)?
    else {
        let type_id = context.unknown("invalid function application");
        return Ok((type_id, None));
    };

    let function_expression = function_expression
        .map(|function| apply_evidence(state, context, function, argument, result, evidence));

    super::check_expression(state, context, argument_expression, argument)?;
    let argument_expression = state.checked.core.lookup_expression(argument_expression);
    let expression = function_expression.zip(argument_expression);
    let expression = expression.map(|(function, argument)| {
        let kind = CheckedExpressionKind::TermApplication { function, argument };
        state.checked.core.allocate_expression(result, kind)
    });

    Ok((result, expression))
}

pub(crate) fn apply_evidence<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function: CheckedExpressionId,
    argument_type: TypeId,
    result_type: TypeId,
    evidence: Arc<[EvidenceVarId]>,
) -> CheckedExpressionId
where
    Q: ExternalQueries,
{
    if evidence.is_empty() {
        function
    } else {
        let function_type = context.intern_function(argument_type, result_type);
        state.checked.core.allocate_expression(
            function_type,
            CheckedExpressionKind::EvidenceApplication { expression: function, evidence },
        )
    }
}

pub fn check_function_term_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function: TypeId,
    expression_id: lowering::ExpressionId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let Some(CheckedApplication { argument, result, .. }) =
        check_generic_application(state, context, function)?
    else {
        return Ok(context.unknown("invalid function application"));
    };
    super::check_expression(state, context, expression_id, argument)?;
    Ok(result)
}

pub fn check_function_type_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function: TypeId,
    argument: lowering::TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let application = check_core_function_type_application(state, context, function, argument)?;
    Ok(application.result)
}

fn check_core_function_type_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut function: TypeId,
    argument: lowering::TypeId,
) -> QueryResult<CheckedTypeApplication>
where
    Q: ExternalQueries,
{
    let function_type = function;

    safe_loop! {
        function = normalise::expand(state, context, function)?;
        let Type::Forall(binder_id, inner) = context.lookup_type(function) else {
            state.insert_error(ErrorKind::NoVisibleTypeVariable { function_type });
            let result = context.unknown("invalid visible type application");
            return Ok(CheckedTypeApplication { argument: None, result });
        };

        let ForallBinder { visible, name, kind } = context.lookup_forall_binder(binder_id);
        let kind = normalise::expand(state, context, kind)?;

        if visible {
            let (argument_type, _) = types::check_kind(state, context, argument, kind)?;
            let result = SubstituteName::one(state, context, name, argument_type, inner)?;
            return Ok(CheckedTypeApplication { argument: Some(argument_type), result });
        }

        let replacement = state.fresh_unification(context.queries, kind);
        function = SubstituteName::one(state, context, name, replacement, inner)?;
    }
}

pub fn infer_infix_chain<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    head: lowering::ExpressionId,
    tail: &[lowering::InfixPair<lowering::ExpressionId>],
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let mut infix_type = super::infer_expression(state, context, head)?;

    for lowering::InfixPair { tick, element } in tail.iter() {
        let Some(tick) = tick else { return Ok(context.unknown("missing infix tick")) };
        let Some(element) = element else { return Ok(context.unknown("missing infix element")) };

        let tick_type = super::infer_expression(state, context, *tick)?;
        let Some(CheckedApplication { argument, result, .. }) =
            check_generic_application(state, context, tick_type)?
        else {
            return Ok(context.unknown("invalid function application"));
        };
        unification::subtype(state, context, infix_type, argument)?;
        let applied_tick = result;

        infix_type = check_function_term_application(state, context, applied_tick, *element)?;
    }

    Ok(infix_type)
}
