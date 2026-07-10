use building_types::QueryResult;

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::{Type, TypeId, constraint};
use crate::error::ErrorCrumb;
use crate::evidence::{EvidenceAbstractionSite, EvidenceBinderId, WantedCollector};
use crate::state::CheckState;

use super::{DeriveHeadResult, DeriveStrategy, field, tools, variance};

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
            let givens = state
                .capture_binders(EvidenceAbstractionSite::Term(result.item_id), |state| {
                    allocate_derive_givens(state, context, &result.constraints)
                })?;
            state.with_implication(|state| {
                for &(constraint, evidence) in &givens {
                    state.push_given_with_evidence(constraint, evidence);
                }
                let mut collector = WantedCollector::derived_requirement(result.derive_id);
                state.capture_binders(EvidenceAbstractionSite::Derived(result.derive_id), |state| {
                    check_derive_member(state, context, &mut collector, result)
                })
            })
        })?;
    }
    Ok(())
}

fn allocate_derive_givens<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    constraints: &[TypeId],
) -> QueryResult<Vec<(TypeId, EvidenceBinderId)>>
where
    Q: ExternalQueries,
{
    let mut givens = Vec::with_capacity(constraints.len());
    for &constraint in constraints {
        if constraint::is_type_error(state, context, constraint)? {
            continue;
        }
        let evidence = state.fresh_evidence_binder();
        givens.push((constraint, evidence));
    }
    Ok(givens)
}

fn check_derive_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    collector: &mut WantedCollector,
    result: &DeriveHeadResult,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    match result.strategy {
        DeriveStrategy::FieldConstraints { data_file, data_id, derived_type, class } => {
            tools::emit_superclass_constraints(
                state,
                context,
                collector,
                result.class_file,
                result.class_id,
                &result.arguments,
            )?;
            field::generate_field_constraints(
                state,
                context,
                collector,
                data_file,
                data_id,
                derived_type,
                class,
            )?;
            tools::solve_and_report_constraints(state, context)?;
        }
        DeriveStrategy::DelegateConstraint { derived_type, class } => {
            tools::emit_superclass_constraints(
                state,
                context,
                collector,
                result.class_file,
                result.class_id,
                &result.arguments,
            )?;
            generate_delegate_constraint(state, context, collector, derived_type, class);
            tools::solve_and_report_constraints(state, context)?;
        }
        DeriveStrategy::NewtypeDeriveConstraint { delegate_constraint } => {
            collector.collect(state, delegate_constraint);
            tools::solve_and_report_constraints(state, context)?;
        }
        DeriveStrategy::HeadOnly => {
            tools::emit_superclass_constraints(
                state,
                context,
                collector,
                result.class_file,
                result.class_id,
                &result.arguments,
            )?;
            tools::solve_and_report_constraints(state, context)?;
        }
        DeriveStrategy::VarianceConstraints { data_file, data_id, derived_type, config } => {
            tools::emit_superclass_constraints(
                state,
                context,
                collector,
                result.class_file,
                result.class_id,
                &result.arguments,
            )?;
            variance::generate_variance_constraints(
                state,
                context,
                collector,
                data_file,
                data_id,
                derived_type,
                config,
            )?;
            tools::solve_and_report_constraints(state, context)?;
        }
    }
    Ok(())
}

fn generate_delegate_constraint<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    collector: &mut WantedCollector,
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
    collector.collect(state, wanted_constraint);
}
