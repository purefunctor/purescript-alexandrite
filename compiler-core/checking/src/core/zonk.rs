use std::mem;
use std::sync::Arc;

use building_types::QueryResult;
use smol_str::SmolStr;

use crate::context::CheckContext;
use crate::core::fold::{FoldAction, TypeFold, fold_type};
use crate::core::{Type, TypeId};
use crate::error::{CheckingError, ErrorKind};
use crate::holes::{HoleBinding, TermHole, TypeHole};
use crate::semantic::{
    CheckedAdoExpression, CheckedAdoStep, CheckedApplication, CheckedBinaryApplication,
    CheckedBinderId, CheckedBlockStatement, CheckedDoExpression, CheckedDoStep,
    CheckedExpressionId, CheckedExpressionKind, CheckedLetBinding,
};
use crate::state::CheckState;
use crate::{ExternalQueries, OperatorBranchTypes, holes};

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

pub fn zonk_core<Q>(state: &mut CheckState, context: &CheckContext<Q>) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let expressions = state.checked.core.expressions.iter().map(|(id, _)| id);
    let expressions = expressions.collect::<Vec<_>>();
    for expression_id in expressions {
        zonk_checked_expression(state, context, expression_id)?;
    }

    let binders = state.checked.core.binders.iter().map(|(id, _)| id);
    let binders = binders.collect::<Vec<_>>();
    for binder_id in binders {
        zonk_checked_binder(state, context, binder_id)?;
    }

    Ok(())
}

fn zonk_checked_expression<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression_id: CheckedExpressionId,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let mut expression = state.checked.core.expressions[expression_id].clone();
    expression.type_id = zonk(state, context, expression.type_id)?;

    match &mut expression.kind {
        CheckedExpressionKind::Do { expression } => {
            zonk_do_expression(state, context, expression)?;
        }
        CheckedExpressionKind::Ado { expression } => {
            zonk_ado_expression(state, context, expression)?;
        }
        CheckedExpressionKind::TypeApplication { argument, .. } => {
            *argument = zonk(state, context, *argument)?;
        }
        _ => (),
    }

    state.checked.core.expressions[expression_id] = expression;
    Ok(())
}

fn zonk_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    application: &mut CheckedApplication,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    application.argument = zonk(state, context, application.argument)?;
    application.result = zonk(state, context, application.result)?;
    Ok(())
}

fn zonk_binary_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    application: &mut CheckedBinaryApplication,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    match application {
        CheckedBinaryApplication::Complete { first, second, .. } => {
            zonk_application(state, context, first)?;
            zonk_application(state, context, second)?;
        }
        CheckedBinaryApplication::Partial { first, .. } => {
            zonk_application(state, context, first)?;
        }
        CheckedBinaryApplication::Error { .. } => (),
    }
    Ok(())
}

fn zonk_do_expression<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: &mut CheckedDoExpression,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for step in Arc::make_mut(&mut expression.steps) {
        match step {
            CheckedDoStep::Bind { continuation_type, application, .. }
            | CheckedDoStep::Discard { continuation_type, application, .. } => {
                *continuation_type = zonk(state, context, *continuation_type)?;
                zonk_binary_application(state, context, application)?;
            }
            CheckedDoStep::Statement(statement) => {
                zonk_block_statement(state, context, statement)?;
            }
        }
    }
    Ok(())
}

fn zonk_ado_expression<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: &mut CheckedAdoExpression,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    match expression {
        CheckedAdoExpression::Pure { statements, application, .. } => {
            if let crate::semantic::CheckedUnaryApplication::Complete { application, .. } =
                application
            {
                zonk_application(state, context, application)?;
            }
            for statement in Arc::make_mut(statements) {
                zonk_block_statement(state, context, statement)?;
            }
        }
        CheckedAdoExpression::Error { statements, .. } => {
            for statement in Arc::make_mut(statements) {
                zonk_block_statement(state, context, statement)?;
            }
        }
        CheckedAdoExpression::Actions { steps, lambda_type, .. } => {
            *lambda_type = zonk(state, context, *lambda_type)?;
            for step in Arc::make_mut(steps) {
                zonk_ado_step(state, context, step)?;
            }
        }
    }
    Ok(())
}

fn zonk_ado_step<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    step: &mut CheckedAdoStep,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    match step {
        CheckedAdoStep::Map { application, .. } | CheckedAdoStep::Apply { application, .. } => {
            zonk_binary_application(state, context, application)?;
        }
        CheckedAdoStep::Statement(statement) => {
            zonk_block_statement(state, context, statement)?;
        }
    }
    Ok(())
}

fn zonk_block_statement<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    statement: &mut CheckedBlockStatement,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let CheckedBlockStatement::Let(statement) = statement else {
        return Ok(());
    };
    for binding in Arc::make_mut(&mut statement.bindings) {
        if let CheckedLetBinding::Name { type_id, .. } = binding {
            *type_id = zonk(state, context, *type_id)?;
        }
    }
    Ok(())
}

fn zonk_checked_binder<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder_id: CheckedBinderId,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let type_id = state.checked.core.binders[binder_id].type_id;
    state.checked.core.binders[binder_id].type_id = zonk(state, context, type_id)?;
    Ok(())
}

pub fn zonk_evidence<Q>(state: &mut CheckState, context: &CheckContext<Q>) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let binders = state.checked.evidence.binders().map(|(id, binder)| (id, binder.constraint));
    let binders = binders.collect::<Vec<_>>();
    for (id, constraint) in binders {
        let constraint = zonk(state, context, constraint)?;
        state.checked.evidence.bind_binder(id, constraint);
    }

    Ok(())
}

pub fn zonk_holes<Q>(state: &mut CheckState, context: &CheckContext<Q>) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for (source_term, hole) in mem::take(&mut state.checked.holes.terms) {
        let type_id = zonk(state, context, hole.type_id)?;
        let bindings = zonk_hole_bindings(state, context, &hole.bindings)?;
        let bindings = holes::refine_bindings(state, context, type_id, bindings)?;

        let hole = TermHole { type_id, bindings };
        state.checked.holes.terms.insert(source_term, hole);
    }

    for (source_type, hole) in mem::take(&mut state.checked.holes.types) {
        let type_id = zonk(state, context, hole.type_id)?;
        let kind_id = zonk(state, context, hole.kind_id)?;
        let bindings = zonk_hole_bindings(state, context, &hole.bindings)?;
        let bindings = holes::refine_bindings(state, context, kind_id, bindings)?;

        let hole = TypeHole { type_id, kind_id, bindings };
        state.checked.holes.types.insert(source_type, hole);
    }

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
        ErrorKind::CannotGeneraliseRecursiveFunction { type_id } => {
            let type_id = zonk(state, context, type_id)?;
            ErrorKind::CannotGeneraliseRecursiveFunction { type_id }
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
        ErrorKind::OverlappingInstances { constraint, instances } => {
            let constraint = zonk(state, context, constraint)?;
            let instances = instances
                .iter()
                .map(|&instance| zonk(state, context, instance))
                .collect::<QueryResult<Arc<[_]>>>()?;
            ErrorKind::OverlappingInstances { constraint, instances }
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
        | ErrorKind::TermHole { .. }
        | ErrorKind::TypeHole { .. }
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

fn zonk_hole_bindings<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    bindings: &[HoleBinding],
) -> QueryResult<Vec<HoleBinding>>
where
    Q: ExternalQueries,
{
    let bindings = bindings.iter().map(|binding| {
        let name = SmolStr::clone(&binding.name);
        let type_id = zonk(state, context, binding.type_id)?;
        Ok(HoleBinding { name, type_id })
    });
    bindings.collect()
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
