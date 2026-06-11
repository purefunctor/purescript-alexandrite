use std::mem;
use std::sync::Arc;

use building_types::QueryResult;

use crate::context::CheckContext;
use crate::core::fold::{FoldAction, TypeFold, fold_type};
use crate::core::{Type, TypeId};
use crate::error::{CheckingError, ErrorKind};
use crate::state::CheckState;
use crate::{ExternalQueries, OperatorBranchTypes};

struct Zonk;

impl TypeFold for Zonk {
    fn transform<Q>(
        &mut self,
        _state: &mut CheckState,
        _context: &CheckContext<Q>,
        _id: TypeId,
        _t: &Type,
    ) -> QueryResult<FoldAction>
    where
        Q: ExternalQueries,
    {
        Ok(FoldAction::Continue)
    }
}

pub fn zonk<Q>(state: &mut CheckState, context: &CheckContext<Q>, id: TypeId) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    fold_type(state, context, id, &mut Zonk)
}

pub fn zonk_nodes<Q>(state: &mut CheckState, context: &CheckContext<Q>) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    macro_rules! zonk_type_map {
        ($field:ident) => {
            for (node_id, type_id) in mem::take(&mut state.checked.nodes.$field) {
                let type_id = zonk(state, context, type_id)?;
                state.checked.nodes.$field.insert(node_id, type_id);
            }
        };
    }

    macro_rules! zonk_operator_map {
        ($field:ident) => {
            for (node_id, branch_types) in mem::take(&mut state.checked.nodes.$field) {
                let branch_types = zonk_operator_branch(state, context, branch_types)?;
                state.checked.nodes.$field.insert(node_id, branch_types);
            }
        };
    }

    zonk_type_map!(types);
    zonk_type_map!(expressions);
    zonk_type_map!(binders);
    zonk_type_map!(lets);
    zonk_type_map!(puns);
    zonk_type_map!(sections);
    zonk_type_map!(forall_bindings);
    zonk_type_map!(implicit_bindings);
    zonk_operator_map!(term_operator);
    zonk_operator_map!(type_operator);

    Ok(())
}

pub fn zonk_errors<Q>(state: &mut CheckState, context: &CheckContext<Q>) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for CheckingError { kind, crumbs } in mem::take(&mut state.checked.errors) {
        let kind = zonk_error_kind(state, context, kind)?;
        state.checked.errors.push(CheckingError { kind, crumbs });
    }

    Ok(())
}

fn zonk_error_kind<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    kind: ErrorKind,
) -> QueryResult<ErrorKind>
where
    Q: ExternalQueries,
{
    Ok(match kind {
        ErrorKind::AmbiguousConstraint { constraint } => {
            let constraint = zonk(state, context, constraint)?;
            ErrorKind::AmbiguousConstraint { constraint }
        }
        ErrorKind::CannotDeriveForType { type_id } => {
            let type_id = zonk(state, context, type_id)?;
            ErrorKind::CannotDeriveForType { type_id }
        }
        ErrorKind::ContravariantOccurrence { type_id } => {
            let type_id = zonk(state, context, type_id)?;
            ErrorKind::ContravariantOccurrence { type_id }
        }
        ErrorKind::CovariantOccurrence { type_id } => {
            let type_id = zonk(state, context, type_id)?;
            ErrorKind::CovariantOccurrence { type_id }
        }
        ErrorKind::CannotUnify { t1, t2 } => {
            let t1 = zonk(state, context, t1)?;
            let t2 = zonk(state, context, t2)?;
            ErrorKind::CannotUnify { t1, t2 }
        }
        ErrorKind::InstanceHeadLabeledRow { class_file, class_item, position, type_id } => {
            let type_id = zonk(state, context, type_id)?;
            ErrorKind::InstanceHeadLabeledRow { class_file, class_item, position, type_id }
        }
        ErrorKind::InstanceMemberTypeMismatch { expected, actual } => {
            let expected = zonk(state, context, expected)?;
            let actual = zonk(state, context, actual)?;
            ErrorKind::InstanceMemberTypeMismatch { expected, actual }
        }
        ErrorKind::InvalidTypeApplication { function_type, function_kind, argument_type } => {
            let function_type = zonk(state, context, function_type)?;
            let function_kind = zonk(state, context, function_kind)?;
            let argument_type = zonk(state, context, argument_type)?;
            ErrorKind::InvalidTypeApplication { function_type, function_kind, argument_type }
        }
        ErrorKind::ExpectedNewtype { type_id } => {
            let type_id = zonk(state, context, type_id)?;
            ErrorKind::ExpectedNewtype { type_id }
        }
        ErrorKind::NonLocalNewtype { type_id } => {
            let type_id = zonk(state, context, type_id)?;
            ErrorKind::NonLocalNewtype { type_id }
        }
        ErrorKind::NoInstanceFound { given, constraint } => {
            let given = given
                .iter()
                .map(|&given| zonk(state, context, given))
                .collect::<QueryResult<Arc<[_]>>>()?;
            let constraint = zonk(state, context, constraint)?;
            ErrorKind::NoInstanceFound { given, constraint }
        }
        ErrorKind::NoVisibleTypeVariable { function_type } => {
            let function_type = zonk(state, context, function_type)?;
            ErrorKind::NoVisibleTypeVariable { function_type }
        }
        kind @ (ErrorKind::CannotDeriveClass { .. }
        | ErrorKind::DeriveInvalidArity { .. }
        | ErrorKind::DeriveNotSupportedYet { .. }
        | ErrorKind::DeriveMissingFunctor
        | ErrorKind::EmptyAdoBlock
        | ErrorKind::EmptyDoBlock
        | ErrorKind::InvalidFinalBind
        | ErrorKind::InvalidFinalLet
        | ErrorKind::InstanceHeadMismatch { .. }
        | ErrorKind::InvalidNewtypeDeriveSkolemArguments
        | ErrorKind::PartialSynonymApplication { .. }
        | ErrorKind::RecursiveSynonymExpansion { .. }
        | ErrorKind::TooManyBinders { .. }
        | ErrorKind::TypeSignatureVariableMismatch { .. }
        | ErrorKind::InvalidRoleDeclaration { .. }
        | ErrorKind::CoercibleConstructorNotInScope { .. }
        | ErrorKind::CustomWarning { .. }
        | ErrorKind::RedundantPatterns { .. }
        | ErrorKind::MissingPatterns { .. }
        | ErrorKind::CustomFailure { .. }
        | ErrorKind::PropertyIsMissing { .. }
        | ErrorKind::AdditionalProperty { .. }) => kind,
    })
}

fn zonk_operator_branch<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    branch_types: OperatorBranchTypes,
) -> QueryResult<OperatorBranchTypes>
where
    Q: ExternalQueries,
{
    Ok(OperatorBranchTypes {
        left: zonk(state, context, branch_types.left)?,
        right: zonk(state, context, branch_types.right)?,
        result: zonk(state, context, branch_types.result)?,
    })
}
