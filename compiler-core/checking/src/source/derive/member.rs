use building_types::QueryResult;

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::Type;
use crate::error::ErrorCrumb;
use crate::state::CheckState;

use super::{
    DeriveDispatch, DeriveHeadResult, DeriveStrategy, derive_dispatch, field, generate, tools,
    variance,
};

pub fn check_derive_members<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    derives: &[DeriveHeadResult],
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for result in derives {
        state.with_error_crumb(ErrorCrumb::TermDeclaration(result.item_id), |state| {
            state.with_implication(|state| check_derive_member(state, context, result))
        })?;
    }
    Ok(())
}

fn check_derive_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for &constraint in &result.constraints {
        state.push_given(constraint);
    }

    let mut variance_recipe = None;
    match result.strategy {
        DeriveStrategy::FieldConstraints { data_file, data_id, derived_type, class } => {
            tools::emit_superclass_constraints(
                state,
                context,
                result.class_file,
                result.class_id,
                &result.arguments,
            )?;
            field::generate_field_constraints(
                state,
                context,
                data_file,
                data_id,
                derived_type,
                class,
            )?;
        }
        DeriveStrategy::DelegateConstraint { derived_type, class } => {
            tools::emit_superclass_constraints(
                state,
                context,
                result.class_file,
                result.class_id,
                &result.arguments,
            )?;
            generate_delegate_constraint(state, context, derived_type, class);
        }
        DeriveStrategy::NewtypeDeriveConstraint { delegate_constraint } => {
            state.push_wanted(delegate_constraint);
        }
        DeriveStrategy::HeadOnly => {
            tools::emit_superclass_constraints(
                state,
                context,
                result.class_file,
                result.class_id,
                &result.arguments,
            )?;
        }
        DeriveStrategy::VarianceConstraints { data_file, data_id, derived_type, config } => {
            tools::emit_superclass_constraints(
                state,
                context,
                result.class_file,
                result.class_id,
                &result.arguments,
            )?;
            let recipe = variance::generate_variance_constraints(
                state,
                context,
                data_file,
                data_id,
                derived_type,
                config,
            )?;
            if matches!(
                derive_dispatch(context, result.class_file, result.class_id),
                DeriveDispatch::Functor | DeriveDispatch::Bifunctor
            ) && recipe.valid
            {
                variance_recipe = Some(recipe);
            }
        }
    }

    if let tools::ConstraintReport::Solved = tools::solve_and_report_constraints(state, context)? {
        let declaration =
            generate::generate_instance(state, context, result, variance_recipe.as_ref())?;
        if let tools::ConstraintReport::Solved =
            tools::solve_and_report_constraints(state, context)?
            && let Some(declaration) = declaration
        {
            state.checked.tree.insert_term(result.item_id, declaration);
        }
    }

    Ok(())
}

fn generate_delegate_constraint<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    derived_type: crate::core::TypeId,
    class: (files::FileId, indexing::TypeItemId),
) where
    Q: ExternalQueries,
{
    let skolem_type = state.fresh_rigid(context.queries, context.prim.t);
    let applied_type = context.intern_application(derived_type, skolem_type);

    let class_type = context.queries.intern_type(Type::Constructor(class.0, class.1));
    let given_constraint = context.intern_application(class_type, skolem_type);
    let wanted_constraint = context.intern_application(class_type, applied_type);

    state.push_given(given_constraint);
    state.push_wanted(wanted_constraint);
}
