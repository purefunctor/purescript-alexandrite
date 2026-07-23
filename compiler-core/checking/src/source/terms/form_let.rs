use building_types::QueryResult;

use crate::context::CheckContext;
use crate::core::{Type, exhaustive, normalise, unification};
use crate::error::ErrorCrumb;
use crate::source::terms::{ElaboratedExpression, application, equations, guarded};
use crate::source::{binder, types};
use crate::state::CheckState;
use crate::{ExternalQueries, tree};

pub fn check_let_chunks<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    chunks: &[lowering::LetBindingChunk],
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for chunk in chunks {
        match chunk {
            lowering::LetBindingChunk::Pattern { binder, where_expression, .. } => {
                check_pattern_let_binding(state, context, binder, where_expression)?;
            }
            lowering::LetBindingChunk::Names { bindings, scc } => {
                check_names_chunk(state, context, bindings, scc)?;
            }
        }
    }
    Ok(())
}

pub fn check_pattern_let_binding<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder: &Option<lowering::BinderId>,
    where_expression: &Option<lowering::WhereExpression>,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let Some(where_expression) = where_expression else {
        return Ok(());
    };

    let where_expression = guarded::infer_where_expression(state, context, where_expression)?;

    let Some(binder_id) = *binder else {
        return Ok(());
    };

    let expression = ElaboratedExpression {
        type_id: where_expression.type_id,
        expression: where_expression.where_expression.expression,
    };
    let expression = if binder::requires_instantiation(context, binder_id) {
        application::instantiate_expression(state, context, expression)?
    } else {
        application::collect_expression_wanteds(state, context, expression)?
    };

    let binder = binder::check_binder(state, context, binder_id, expression.type_id)?;

    let exhaustiveness =
        exhaustive::check_lambda_patterns(state, context, &[binder.type_id], &[binder_id])?;

    let has_missing = exhaustiveness.missing.is_some();
    state.report_exhaustiveness(exhaustiveness);

    if has_missing {
        state.push_wanted(context.prim.partial);
    }

    Ok(())
}

pub fn check_names_chunk<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    bindings: &[lowering::LetBindingNameGroupId],
    scc: &[lowering::Scc<lowering::LetBindingNameGroupId>],
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for &id in bindings {
        let Some(name) = context.lowered.info.get_let_binding(id) else {
            continue;
        };
        if let Some(signature_id) = name.signature {
            let (name_type, _) = types::check_kind(state, context, signature_id, context.prim.t)?;
            state.checked.nodes.lets.insert(id, name_type);
        } else {
            let name_type = state.fresh_unification(context.queries, context.prim.t);
            state.checked.nodes.lets.insert(id, name_type);
        }
    }

    for item in scc {
        match item {
            lowering::Scc::Base(id) | lowering::Scc::Recursive(id) => {
                let declaration = check_let_name_binding(state, context, *id)?;
                state.checked.tree.insert_let(declaration);
            }
            lowering::Scc::Mutual(mutual) => {
                for &id in mutual {
                    let declaration = check_let_name_binding(state, context, id)?;
                    state.checked.tree.insert_let(declaration);
                }
            }
        }
    }

    Ok(())
}

pub fn check_let_name_binding<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    id: lowering::LetBindingNameGroupId,
) -> QueryResult<tree::LocalDeclaration>
where
    Q: ExternalQueries,
{
    state.with_implication(|state| {
        state.with_error_crumb(ErrorCrumb::CheckingLetName(id), |state| {
            check_let_name_binding_core(state, context, id)
        })
    })
}

pub fn check_let_name_binding_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    id: lowering::LetBindingNameGroupId,
) -> QueryResult<tree::LocalDeclaration>
where
    Q: ExternalQueries,
{
    let group = context.lowered.info.get_let_binding_group(id);
    let name = context
        .lowered
        .info
        .get_let_binding(id)
        .expect("invariant violated: let binding group has no lowered binding");
    let name_type = state
        .checked
        .nodes
        .lookup_let(id)
        .expect("invariant violated: let binding has no preallocated type");

    let checked_equations = if let Some(signature_id) = name.signature {
        equations::check_value_equations(
            state,
            context,
            equations::EquationTypeOrigin::Explicit(signature_id),
            name_type,
            &name.equations,
        )?
    } else {
        if let [equation] = name.equations.as_ref()
            && let [equation_source] = group.equations.as_ref()
            && equation.binders.is_empty()
            && let Some(guarded) = &equation.guarded
        {
            let inferred = guarded::infer_guarded_expression(state, context, guarded)?;
            let inferred_type = inferred.type_id;
            // Keep simple let bindings e.g. `appendLocal = append` polymorphic.
            let expanded_name_type = normalise::expand(state, context, name_type)?;
            if let Type::Unification(unification_id) = context.lookup_type(expanded_name_type) {
                unification::solve(
                    state,
                    context,
                    expanded_name_type,
                    unification_id,
                    inferred_type,
                )?;
            } else {
                unification::subtype(state, context, inferred_type, expanded_name_type)?;
            }

            let declaration = tree::LocalDeclaration::nullary(
                id,
                name_type,
                *equation_source,
                inferred.guarded_expression,
            );
            return Ok(declaration);
        } else {
            equations::infer_value_equations(state, context, name_type, &name.equations)?
        }
    };

    let exhaustiveness = exhaustive::check_equation_patterns(
        state,
        context,
        &checked_equations.patterns,
        &name.equations,
    )?;
    state.report_exhaustiveness(exhaustiveness);

    assert_eq!(
        group.equations.len(),
        checked_equations.equations.len(),
        "invariant violated: checked local equation count does not match lowering",
    );

    let equations = std::iter::zip(group.equations.iter().copied(), checked_equations.equations)
        .map(|(source, equation)| equation.into_local_tree(source));

    let equations = equations.collect();
    let evidences = checked_equations.evidences.into();

    Ok(tree::LocalDeclaration::new(id, name_type, evidences, equations))
}
