use building_types::QueryResult;

use crate::context::CheckContext;
use crate::core::substitute::SubstituteName;
use crate::core::{ForallBinder, Type, TypeId, normalise, unification};
use crate::error::ErrorKind;
use crate::source::types;
use crate::state::CheckState;
use crate::{ExternalQueries, safe_loop, tree};

use super::ElaboratedExpression;

pub struct GenericApplication {
    pub argument: TypeId,
    pub result: TypeId,
}

enum ApplicationStep {
    Applied(ElaboratedExpression),
    Error(TypeId),
}

pub enum CallableAnalysis {
    Forall { binder: ForallBinder, body: TypeId },
    Constraint { constraint: TypeId, result: TypeId },
    Function { argument: TypeId, result: TypeId },
    NotCallable,
}

pub fn analyse_callable_head<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function: TypeId,
) -> QueryResult<CallableAnalysis>
where
    Q: ExternalQueries,
{
    let function = normalise::expand(state, context, function)?;

    match context.lookup_type(function) {
        Type::Function(argument, result) => Ok(CallableAnalysis::Function { argument, result }),

        Type::Unification(unification_id) => {
            let argument = state.fresh_unification(context.queries, context.prim.t);
            let result = state.fresh_unification(context.queries, context.prim.t);
            let function = context.intern_function(argument, result);

            unification::solve(state, context, function, unification_id, function)?;

            Ok(CallableAnalysis::Function { argument, result })
        }

        Type::Forall(binder_id, inner) => {
            let binder = context.lookup_forall_binder(binder_id);
            Ok(CallableAnalysis::Forall { binder, body: inner })
        }

        Type::Constrained(constraint, result) => {
            Ok(CallableAnalysis::Constraint { constraint, result })
        }

        Type::Application(function_argument, result) => {
            let function_argument = normalise::expand(state, context, function_argument)?;

            let Type::Application(constructor, argument) = context.lookup_type(function_argument)
            else {
                return Ok(CallableAnalysis::NotCallable);
            };

            let constructor = normalise::expand(state, context, constructor)?;
            if constructor == context.prim.function {
                return Ok(CallableAnalysis::Function { argument, result });
            }

            if let Type::Unification(unification_id) = context.lookup_type(constructor) {
                unification::solve(
                    state,
                    context,
                    constructor,
                    unification_id,
                    context.prim.function,
                )?;

                return Ok(CallableAnalysis::Function { argument, result });
            }

            Ok(CallableAnalysis::NotCallable)
        }

        _ => Ok(CallableAnalysis::NotCallable),
    }
}

pub fn instantiate_callable_forall<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder: ForallBinder,
    body: TypeId,
) -> QueryResult<(TypeId, TypeId)>
where
    Q: ExternalQueries,
{
    let binder_kind = normalise::expand(state, context, binder.kind)?;
    let argument = state.fresh_unification(context.queries, binder_kind);
    let result = SubstituteName::one(state, context, binder.name, argument, body)?;
    Ok((argument, result))
}

pub fn instantiate_expression<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut expression: ElaboratedExpression,
) -> QueryResult<ElaboratedExpression>
where
    Q: ExternalQueries,
{
    safe_loop! {
        let type_id = normalise::expand(state, context, expression.type_id)?;
        match context.lookup_type(type_id) {
            Type::Forall(binder_id, body) => {
                let binder = context.lookup_forall_binder(binder_id);
                let (argument, result) =
                    instantiate_callable_forall(state, context, binder, body)?;
                let kind = tree::ExpressionKind::TypeApplication {
                    function: expression.expression,
                    argument,
                };
                expression = super::allocate_expression(state, result, kind);
            }
            Type::Constrained(constraint, result) => {
                let evidence = state.push_wanted(constraint);
                let kind = tree::ExpressionKind::EvidenceApplication {
                    function: expression.expression,
                    evidence,
                };
                expression = super::allocate_expression(state, result, kind);
            }
            _ => {
                break Ok(expression);
            }
        }
    }
}

pub fn collect_expression_wanteds<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut expression: ElaboratedExpression,
) -> QueryResult<ElaboratedExpression>
where
    Q: ExternalQueries,
{
    safe_loop! {
        let type_id = normalise::expand(state, context, expression.type_id)?;
        let Type::Constrained(constraint, result) = context.lookup_type(type_id) else {
            break Ok(expression);
        };
        let evidence = state.push_wanted(constraint);
        let kind = tree::ExpressionKind::EvidenceApplication {
            function: expression.expression,
            evidence,
        };
        expression = super::allocate_expression(state, result, kind);
    }
}

pub fn check_generic_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    function: TypeId,
) -> QueryResult<Option<GenericApplication>>
where
    Q: ExternalQueries,
{
    let mut function = function;
    let mut constraints = vec![];
    safe_loop! {
        match analyse_callable_head(state, context, function)? {
            CallableAnalysis::Forall { binder, body } => {
                let (_, result) = instantiate_callable_forall(state, context, binder, body)?;
                function = result;
            }
            CallableAnalysis::Constraint { constraint, result } => {
                constraints.push(constraint);
                function = result;
            }
            CallableAnalysis::Function { argument, result } => {
                for constraint in constraints {
                    state.push_wanted(constraint);
                }
                break Ok(Some(GenericApplication { argument, result }));
            }
            CallableAnalysis::NotCallable => break Ok(None),
        }
    }
}

pub fn check_expression_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut function: ElaboratedExpression,
    arguments: &[lowering::ExpressionArgument],
) -> QueryResult<ElaboratedExpression>
where
    Q: ExternalQueries,
{
    for argument in arguments {
        let step = match argument {
            lowering::ExpressionArgument::Type(Some(argument)) => {
                check_expression_type_application(state, context, function, *argument)?
            }
            lowering::ExpressionArgument::Type(None) => {
                ApplicationStep::Error(context.unknown("missing type argument"))
            }
            lowering::ExpressionArgument::Term(Some(argument)) => {
                check_expression_term_application(state, context, function, *argument)?
            }
            lowering::ExpressionArgument::Term(None) => {
                ApplicationStep::Error(context.unknown("missing term argument"))
            }
        };

        match step {
            ApplicationStep::Applied(expression) => {
                function = expression;
            }
            ApplicationStep::Error(type_id) => {
                return Ok(super::allocate_error_expression(state, type_id));
            }
        }
    }

    Ok(function)
}

fn check_expression_term_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut function: ElaboratedExpression,
    expression_id: lowering::ExpressionId,
) -> QueryResult<ApplicationStep>
where
    Q: ExternalQueries,
{
    safe_loop! {
        match analyse_callable_head(state, context, function.type_id)? {
            CallableAnalysis::Forall { binder, body } => {
                let (argument, result) =
                    instantiate_callable_forall(state, context, binder, body)?;
                let kind = tree::ExpressionKind::TypeApplication {
                    function: function.expression,
                    argument,
                };
                function = super::allocate_expression(state, result, kind);
            }
            CallableAnalysis::Constraint { constraint, result } => {
                let evidence = state.push_wanted(constraint);
                let kind = tree::ExpressionKind::EvidenceApplication {
                    function: function.expression,
                    evidence,
                };
                function = super::allocate_expression(state, result, kind);
            }
            CallableAnalysis::Function { argument, result } => {
                let argument = super::check_expression(state, context, expression_id, argument)?;
                let kind = tree::ExpressionKind::TermApplication {
                    function: function.expression,
                    argument: argument.expression,
                };
                let application = super::allocate_expression(state, result, kind);
                break Ok(ApplicationStep::Applied(application));
            }
            CallableAnalysis::NotCallable => {
                let type_id = context.unknown("invalid function application");
                break Ok(ApplicationStep::Error(type_id));
            }
        }
    }
}

fn check_expression_type_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut function: ElaboratedExpression,
    argument: lowering::TypeId,
) -> QueryResult<ApplicationStep>
where
    Q: ExternalQueries,
{
    let function_type = function.type_id;

    safe_loop! {
        let type_id = normalise::expand(state, context, function.type_id)?;
        match context.lookup_type(type_id) {
            Type::Forall(binder_id, body) => {
                let binder = context.lookup_forall_binder(binder_id);
                if binder.visible {
                    let binder_kind = normalise::expand(state, context, binder.kind)?;
                    let (argument, _) = types::check_kind(state, context, argument, binder_kind)?;
                    let result =
                        SubstituteName::one(state, context, binder.name, argument, body)?;
                    let kind = tree::ExpressionKind::TypeApplication {
                        function: function.expression,
                        argument,
                    };
                    let application = super::allocate_expression(state, result, kind);
                    break Ok(ApplicationStep::Applied(application));
                }

                let (argument, result) =
                    instantiate_callable_forall(state, context, binder, body)?;
                let kind = tree::ExpressionKind::TypeApplication {
                    function: function.expression,
                    argument,
                };
                function = super::allocate_expression(state, result, kind);
            }
            Type::Constrained(constraint, result) => {
                let evidence = state.push_wanted(constraint);
                let kind = tree::ExpressionKind::EvidenceApplication {
                    function: function.expression,
                    evidence,
                };
                function = super::allocate_expression(state, result, kind);
            }
            _ => {
                state.insert_error(ErrorKind::NoVisibleTypeVariable { function_type });
                let type_id = context.unknown("invalid visible type application");
                break Ok(ApplicationStep::Error(type_id));
            }
        }
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
    let Some(GenericApplication { argument, result }) =
        check_generic_application(state, context, function)?
    else {
        return Ok(context.unknown("invalid function application"));
    };
    super::check_expression(state, context, expression_id, argument)?;
    Ok(result)
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
    let mut infix_type = super::infer_expression(state, context, head)?.type_id;

    for lowering::InfixPair { tick, element } in tail.iter() {
        let Some(tick) = tick else { return Ok(context.unknown("missing infix tick")) };
        let Some(element) = element else { return Ok(context.unknown("missing infix element")) };

        let tick_type = super::infer_expression(state, context, *tick)?.type_id;
        let Some(GenericApplication { argument, result }) =
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
