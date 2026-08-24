use std::sync::Arc;

use building_types::QueryResult;
use itertools::Itertools;
use lowering::TypeVariableBinding;

use crate::context::CheckContext;
use crate::core::substitute::RigidRenaming;
use crate::core::{ForallBinderId, Type, TypeId, normalise, toolkit, unification};
use crate::error::ErrorKind;
use crate::state::CheckState;
use crate::{ExternalQueries, safe_loop};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecomposedAbstraction {
    Type { binder: ForallBinderId },
    Constraint { constraint: TypeId },
}

pub struct DecomposedSignature {
    pub abstractions: Vec<DecomposedAbstraction>,
    pub arguments: Vec<TypeId>,
    pub result: TypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkolemisedAbstraction {
    Type { binder: ForallBinderId, rigid: TypeId },
    Constraint { constraint: TypeId },
}

pub struct SkolemisedSignature {
    pub renaming: Arc<RigidRenaming>,
    pub abstractions: Vec<SkolemisedAbstraction>,
    pub arguments: Vec<TypeId>,
    pub result: TypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecomposeSignatureMode {
    Full,
    Patterns { required: usize },
}

pub fn decompose_signature<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut current: TypeId,
    mode: DecomposeSignatureMode,
) -> QueryResult<DecomposedSignature>
where
    Q: ExternalQueries,
{
    let mut abstractions = vec![];
    let mut arguments = vec![];

    safe_loop! {
        current = normalise::expand(state, context, current)?;

        match context.lookup_type(current) {
            Type::Forall(binder_id, inner) => {
                abstractions.push(DecomposedAbstraction::Type { binder: binder_id });
                current = inner;
            }

            Type::Constrained(constraint, constrained) => {
                abstractions.push(DecomposedAbstraction::Constraint { constraint });
                current = constrained;
            }

            Type::Function(argument, result) => {
                if let DecomposeSignatureMode::Patterns { required } = mode
                    && arguments.len() >= required
                {
                    return Ok(DecomposedSignature { abstractions, arguments, result: current });
                }

                arguments.push(argument);
                current = result;
            }

            Type::Application(function_argument, result) => {
                if let DecomposeSignatureMode::Patterns { required } = mode
                    && arguments.len() >= required
                {
                    return Ok(DecomposedSignature { abstractions, arguments, result: current });
                }

                let function_argument =
                    normalise::expand(state, context, function_argument)?;

                let Type::Application(function, argument) = context.lookup_type(function_argument)
                else {
                    return Ok(DecomposedSignature { abstractions, arguments, result: current });
                };

                let function = normalise::expand(state, context, function)?;
                if function == context.prim.function {
                    arguments.push(argument);
                    current = result;
                } else {
                    return Ok(DecomposedSignature { abstractions, arguments, result: current });
                }
            }

            _ => return Ok(DecomposedSignature { abstractions, arguments, result: current }),
        }
    }
}

pub fn expect_type_signature<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    (signature_id, signature_type): (lowering::TypeId, TypeId),
    bindings: &[TypeVariableBinding],
) -> QueryResult<DecomposedSignature>
where
    Q: ExternalQueries,
{
    let signature =
        decompose_signature(state, context, signature_type, DecomposeSignatureMode::Full)?;

    let actual = bindings.len() as u32;
    let expected = signature.arguments.len() as u32;

    if actual > expected {
        state.insert_error(ErrorKind::TypeSignatureVariableMismatch {
            id: signature_id,
            expected,
            actual,
        });
    }

    let mut remaining = signature.arguments.into_iter();
    let arguments = remaining.by_ref().take(actual as usize).collect();
    let result = context.intern_function_iter(remaining, signature.result);

    Ok(DecomposedSignature { abstractions: signature.abstractions, arguments, result })
}

pub fn expect_term_signature<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    signature_type: TypeId,
    required: usize,
) -> QueryResult<SkolemisedSignature>
where
    Q: ExternalQueries,
{
    let signature =
        decompose_signature(state, context, signature_type, DecomposeSignatureMode::Full)?;

    let SkolemisedSignature { renaming, abstractions, arguments, result } =
        skolemise_decomposed_signature(state, context, signature)?;

    let mut remaining = arguments.into_iter();
    let mut arguments = remaining.by_ref().take(required).collect_vec();

    let mut result = context.intern_function_iter(remaining, result);
    synthesise_functions(state, context, &mut arguments, &mut result, required)?;

    Ok(SkolemisedSignature { renaming, abstractions, arguments, result })
}

fn synthesise_functions<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    arguments: &mut Vec<TypeId>,
    result_type: &mut TypeId,
    required: usize,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    while arguments.len() < required {
        let current = normalise::expand(state, context, *result_type)?;

        let Type::Unification(unification_id) = context.lookup_type(current) else {
            break;
        };

        let argument = state.fresh_unification(context.queries, context.prim.t);
        let result = state.fresh_unification(context.queries, context.prim.t);
        let function = context.intern_function(argument, result);

        if !unification::solve(state, context, current, unification_id, function)? {
            break;
        }

        arguments.push(argument);
        *result_type = result;
    }

    Ok(())
}

fn skolemise_decomposed_signature<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    signature: DecomposedSignature,
) -> QueryResult<SkolemisedSignature>
where
    Q: ExternalQueries,
{
    let mut renaming = RigidRenaming::default();
    let mut abstractions = Vec::with_capacity(signature.abstractions.len());

    for abstraction in signature.abstractions {
        match abstraction {
            DecomposedAbstraction::Type { binder } => {
                let forall_binder = context.lookup_forall_binder(binder);
                let kind = renaming.substitute(state, context, forall_binder.kind)?;
                let text = toolkit::lookup_name(state, context, forall_binder.name)?;
                let rigid = state.fresh_rigid_named(context.queries, kind, text);
                renaming.insert(context, forall_binder.name, rigid);
                abstractions.push(SkolemisedAbstraction::Type { binder, rigid });
            }
            DecomposedAbstraction::Constraint { constraint } => {
                let constraint = renaming.substitute(state, context, constraint)?;
                abstractions.push(SkolemisedAbstraction::Constraint { constraint });
            }
        }
    }

    let arguments = signature
        .arguments
        .iter()
        .map(|&argument| renaming.substitute(state, context, argument))
        .collect::<QueryResult<Vec<_>>>()?;

    let result = renaming.substitute(state, context, signature.result)?;
    let renaming = Arc::new(renaming);

    Ok(SkolemisedSignature { renaming, abstractions, arguments, result })
}
