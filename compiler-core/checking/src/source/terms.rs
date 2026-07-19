pub mod application;
pub mod collections;
pub mod equations;
pub mod form_ado;
pub mod form_do;
pub mod form_let;
pub mod forms;
pub mod guarded;

use std::sync::Arc;

use building_types::QueryResult;
use files::FileId;
use indexing::{ImportKind, TermItemId};
use itertools::Itertools;
use lowering::GraphNode;
use rustc_hash::FxHashSet;
use smol_str::SmolStr;

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::{TypeId, normalise, toolkit, unification};
use crate::error::{ErrorCrumb, ErrorKind};
use crate::holes::{HoleBinding, TermHole};
use crate::semantic::{CheckedExpressionKind, CheckedLiteral};
use crate::source::{operator, types};
use crate::state::CheckState;

/// Checks the type of an expression.
pub fn check_expression<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
    expected: TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    state.with_error_crumb(ErrorCrumb::CheckingExpression(expression), |state| {
        let expected = check_expression_quiet(state, context, expression, expected)?;
        state.checked.nodes.expressions.insert(expression, expected);
        Ok(expected)
    })
}

fn check_expression_quiet<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
    expected: TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let expected = normalise::normalise(state, context, expected)?;
    let expected = toolkit::skolemise_forall(state, context, expected)?;
    let expected = toolkit::collect_givens(state, context, expected)?;

    if let Some(section_result) = context.sectioned.expressions.get(&expression) {
        check_sectioned_expression(state, context, expression, section_result, expected)
    } else {
        check_expression_core(state, context, expression, expected)
    }
}

fn check_sectioned_expression<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
    section_result: &sugar::SectionResult,
    expected: TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let mut current = expected;
    let mut parameters = vec![];

    for &section_id in section_result.iter() {
        let decomposed = toolkit::decompose_function(state, context, current)?;
        if let Some((argument_type, result_type)) = decomposed {
            state.checked.nodes.sections.insert(section_id, argument_type);
            parameters.push(argument_type);
            current = result_type;
        } else {
            let parameter = state.fresh_unification(context.queries, context.prim.t);
            let result = state.fresh_unification(context.queries, context.prim.t);

            let function = context.intern_function(parameter, result);
            unification::subtype(state, context, function, current)?;

            parameters.push(parameter);
            current = result;

            state.checked.nodes.sections.insert(section_id, parameter);
        }
    }

    let result_type = infer_expression_core(state, context, expression)?;
    let result_type = toolkit::instantiate_constrained(state, context, result_type)?;

    unification::subtype(state, context, result_type, current)?;

    let function_type = context.intern_function_list(&parameters, result_type);
    Ok(function_type)
}

fn check_expression_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
    expected: TypeId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let unknown = context.unknown("missing expression");

    let Some(kind) = context.lowered.info.get_expression_kind(expression) else {
        return Ok(unknown);
    };

    match kind {
        lowering::ExpressionKind::Lambda { binders, expression: body } => {
            forms::check_lambda(state, context, expression, binders, *body, expected)
        }
        lowering::ExpressionKind::IfThenElse { if_, then, else_ } => {
            forms::check_if_then_else(state, context, *if_, *then, *else_, expected)
        }
        lowering::ExpressionKind::CaseOf { trunk, branches } => {
            forms::check_case_of(state, context, trunk, branches, expected)
        }
        lowering::ExpressionKind::OperatorChain { .. } => {
            let (_, checked_type) =
                operator::check_operator_chain(state, context, expression, expected)?;
            Ok(checked_type)
        }
        lowering::ExpressionKind::LetIn { bindings, expression } => {
            forms::check_let_in(state, context, bindings, *expression, expected)
        }
        lowering::ExpressionKind::Parenthesized { parenthesized } => {
            let Some(parenthesized) = parenthesized else { return Ok(unknown) };
            let type_id = check_expression(state, context, *parenthesized, expected)?;
            if let Some(checked_expression) = state.checked.core.lookup_expression(*parenthesized) {
                state.checked.core.record_expression(expression, checked_expression);
            }
            Ok(type_id)
        }
        lowering::ExpressionKind::Array { array } => {
            collections::check_array(state, context, array, expected)
        }
        lowering::ExpressionKind::Record { record } => {
            collections::check_record(state, context, record, expected)
        }
        _ => {
            let inferred = infer_expression_quiet(state, context, expression)?;
            let inferred = toolkit::instantiate_constrained(state, context, inferred)?;
            unification::subtype(state, context, inferred, expected)?;
            Ok(inferred)
        }
    }
}

/// Infers the type of an expression.
pub fn infer_expression<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    state.with_error_crumb(ErrorCrumb::InferringExpression(expression), |state| {
        let inferred = infer_expression_quiet(state, context, expression)?;
        state.checked.nodes.expressions.insert(expression, inferred);
        Ok(inferred)
    })
}

fn infer_expression_quiet<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    if let Some(section_result) = context.sectioned.expressions.get(&expression) {
        infer_sectioned_expression(state, context, expression, section_result)
    } else {
        infer_expression_core(state, context, expression)
    }
}

fn infer_sectioned_expression<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
    section_result: &sugar::SectionResult,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let parameter_types = section_result.iter().map(|&section_id| {
        let parameter_type = state.fresh_unification(context.queries, context.prim.t);
        state.checked.nodes.sections.insert(section_id, parameter_type);
        parameter_type
    });

    let parameter_types = parameter_types.collect_vec();

    let result_type = infer_expression_core(state, context, expression)?;
    let result_type = toolkit::instantiate_constrained(state, context, result_type)?;

    Ok(context.intern_function_list(&parameter_types, result_type))
}

fn infer_expression_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let unknown = context.unknown("missing expression");

    let Some(kind) = context.lowered.info.get_expression_kind(expression) else {
        return Ok(unknown);
    };

    match kind {
        lowering::ExpressionKind::Typed { expression: annotated, type_ } => {
            let Some(annotated) = annotated else { return Ok(unknown) };
            let Some(t) = type_ else { return Ok(unknown) };

            let (t, _) = types::infer_kind(state, context, *t)?;
            check_expression(state, context, *annotated, t)?;
            if let Some(checked_expression) = state.checked.core.lookup_expression(*annotated) {
                state.checked.core.record_expression(expression, checked_expression);
            }

            Ok(t)
        }

        lowering::ExpressionKind::OperatorChain { .. } => {
            let (_, inferred_type) = operator::infer_operator_chain(state, context, expression)?;
            Ok(inferred_type)
        }

        lowering::ExpressionKind::InfixChain { head, tail } => {
            let Some(head) = *head else { return Ok(unknown) };
            application::infer_infix_chain(state, context, head, tail)
        }

        lowering::ExpressionKind::Negate { negate, expression } => {
            let Some(negate) = negate else { return Ok(unknown) };
            let Some(expression) = expression else { return Ok(unknown) };

            let negate_type = toolkit::lookup_term_variable(state, context, *negate)?;
            application::check_function_term_application(state, context, negate_type, *expression)
        }

        lowering::ExpressionKind::Application { function, arguments } => {
            let Some(function) = function else { return Ok(unknown) };

            let function_type = infer_expression(state, context, *function)?;
            let function_expression = state.checked.core.lookup_expression(*function);
            let mut application = application::CheckedApplication {
                type_id: function_type,
                expression: function_expression,
            };

            for argument in arguments.iter() {
                application = application::check_core_function_application(
                    state,
                    context,
                    application.type_id,
                    application.expression,
                    argument,
                )?;
            }

            if let Some(checked_expression) = application.expression {
                state.checked.core.record_expression(expression, checked_expression);
            }

            Ok(application.type_id)
        }

        lowering::ExpressionKind::IfThenElse { if_, then, else_ } => {
            forms::infer_if_then_else(state, context, *if_, *then, *else_)
        }

        lowering::ExpressionKind::LetIn { bindings, expression } => {
            form_let::check_let_chunks(state, context, bindings)?;

            let Some(expression) = expression else { return Ok(unknown) };

            infer_expression(state, context, *expression)
        }

        lowering::ExpressionKind::Lambda { binders, expression: body } => {
            forms::infer_lambda(state, context, expression, binders, *body)
        }

        lowering::ExpressionKind::CaseOf { trunk, branches } => {
            forms::infer_case_of(state, context, trunk, branches)
        }

        lowering::ExpressionKind::Do { bind, discard, statements } => {
            form_do::infer_do(state, context, *bind, *discard, statements)
        }

        lowering::ExpressionKind::Ado { map, apply, pure, statements, expression } => {
            form_ado::infer_ado(state, context, *map, *apply, *pure, statements, *expression)
        }

        lowering::ExpressionKind::Constructor { resolution } => {
            let Some((file_id, term_id)) = resolution else { return Ok(unknown) };
            let type_id = toolkit::lookup_file_term(state, context, *file_id, *term_id)?;
            let resolution = lowering::TermVariableResolution::Reference(*file_id, *term_id);
            let kind = CheckedExpressionKind::Variable { resolution };
            let checked_expression = state.checked.core.allocate_expression(type_id, kind);
            state.checked.core.record_expression(expression, checked_expression);
            Ok(type_id)
        }

        lowering::ExpressionKind::Variable { resolution } => {
            let Some(resolution) = *resolution else { return Ok(unknown) };
            let type_id = toolkit::lookup_term_variable(state, context, resolution)?;
            let kind = CheckedExpressionKind::Variable { resolution };
            let checked_expression = state.checked.core.allocate_expression(type_id, kind);
            state.checked.core.record_expression(expression, checked_expression);
            Ok(type_id)
        }

        lowering::ExpressionKind::OperatorName { resolution } => {
            let Some((file_id, term_id)) = resolution else { return Ok(unknown) };
            let type_id = toolkit::lookup_file_term(state, context, *file_id, *term_id)?;
            let resolution = lowering::TermVariableResolution::Reference(*file_id, *term_id);
            let kind = CheckedExpressionKind::Variable { resolution };
            let checked_expression = state.checked.core.allocate_expression(type_id, kind);
            state.checked.core.record_expression(expression, checked_expression);
            Ok(type_id)
        }

        lowering::ExpressionKind::Section => {
            if let Some(type_id) = state.checked.nodes.lookup_section(expression) {
                Ok(type_id)
            } else {
                Ok(unknown)
            }
        }

        lowering::ExpressionKind::Hole => {
            let kind = state.fresh_unification(context.queries, context.prim.t);
            let type_id = state.fresh_unification(context.queries, kind);

            let bindings = term_hole_bindings(state, context, expression)?;
            state.checked.holes.terms.insert(expression, TermHole { type_id, bindings });
            state.insert_error(ErrorKind::TermHole { source_term: expression });

            Ok(type_id)
        }

        lowering::ExpressionKind::String { kind, value } => {
            let literal = CheckedLiteral::String { kind: *kind, value: value.clone() };
            let kind = CheckedExpressionKind::Literal { literal };
            let type_id = context.prim.string;
            let checked_expression = state.checked.core.allocate_expression(type_id, kind);
            state.checked.core.record_expression(expression, checked_expression);
            Ok(type_id)
        }

        lowering::ExpressionKind::Char { value } => {
            let literal = CheckedLiteral::Char(*value);
            let kind = CheckedExpressionKind::Literal { literal };
            let type_id = context.prim.char;
            let checked_expression = state.checked.core.allocate_expression(type_id, kind);
            state.checked.core.record_expression(expression, checked_expression);
            Ok(type_id)
        }

        lowering::ExpressionKind::Boolean { boolean } => {
            let literal = CheckedLiteral::Boolean(*boolean);
            let kind = CheckedExpressionKind::Literal { literal };
            let type_id = context.prim.boolean;
            let checked_expression = state.checked.core.allocate_expression(type_id, kind);
            state.checked.core.record_expression(expression, checked_expression);
            Ok(type_id)
        }

        lowering::ExpressionKind::Integer { value } => {
            let literal = CheckedLiteral::Integer(*value);
            let kind = CheckedExpressionKind::Literal { literal };
            let type_id = context.prim.int;
            let checked_expression = state.checked.core.allocate_expression(type_id, kind);
            state.checked.core.record_expression(expression, checked_expression);
            Ok(type_id)
        }

        lowering::ExpressionKind::Number { value } => {
            let literal = CheckedLiteral::Number(value.clone());
            let kind = CheckedExpressionKind::Literal { literal };
            let type_id = context.prim.number;
            let checked_expression = state.checked.core.allocate_expression(type_id, kind);
            state.checked.core.record_expression(expression, checked_expression);
            Ok(type_id)
        }

        lowering::ExpressionKind::Array { array } => {
            collections::infer_array(state, context, array)
        }

        lowering::ExpressionKind::Record { record } => {
            collections::infer_record(state, context, record)
        }

        lowering::ExpressionKind::Parenthesized { parenthesized } => {
            let Some(parenthesized) = parenthesized else { return Ok(unknown) };
            let type_id = infer_expression(state, context, *parenthesized)?;
            if let Some(checked_expression) = state.checked.core.lookup_expression(*parenthesized) {
                state.checked.core.record_expression(expression, checked_expression);
            }
            Ok(type_id)
        }

        lowering::ExpressionKind::RecordAccess { record, labels } => {
            let Some(record) = *record else { return Ok(unknown) };
            let Some(labels) = labels else { return Ok(unknown) };
            collections::infer_record_access(state, context, record, labels)
        }

        lowering::ExpressionKind::RecordUpdate { record, updates } => {
            let Some(record) = *record else { return Ok(unknown) };
            collections::infer_record_update(state, context, record, updates)
        }
    }
}

fn term_hole_bindings<Q>(
    state: &CheckState,
    context: &CheckContext<Q>,
    expression: lowering::ExpressionId,
) -> QueryResult<Arc<[HoleBinding]>>
where
    Q: ExternalQueries,
{
    let mut seen = FxHashSet::default();
    let mut result = vec![];

    if let Some(scope_node) = context.lowered.nodes.expression_node(expression) {
        collect_graph_term_hole_bindings(state, context, scope_node, &mut seen, &mut result);
    }

    collect_resolved_term_hole_bindings(state, context, &mut seen, &mut result)?;
    Ok(Arc::from(result))
}

fn collect_graph_term_hole_bindings<Q>(
    state: &CheckState,
    context: &CheckContext<Q>,
    scope_node: lowering::GraphNodeId,
    seen: &mut FxHashSet<SmolStr>,
    result: &mut Vec<HoleBinding>,
) where
    Q: ExternalQueries,
{
    for (_, node) in context.lowered.graph.traverse(scope_node) {
        match node {
            GraphNode::Binder { binders, puns, .. } => {
                let mut binders = binders.iter().collect_vec();
                binders.sort_by_key(|(name, _)| *name);

                for (name, binder_id) in binders {
                    if seen.insert(SmolStr::clone(name))
                        && let Some(type_id) = state.checked.nodes.lookup_binder(*binder_id)
                    {
                        let name = SmolStr::clone(name);
                        result.push(HoleBinding { name, type_id });
                    }
                }

                let mut puns = puns.iter().collect_vec();
                puns.sort_by_key(|(name, _)| *name);

                for (name, pun_id) in puns {
                    if seen.insert(SmolStr::clone(name))
                        && let Some(type_id) = state.checked.nodes.lookup_pun(*pun_id)
                    {
                        let name = SmolStr::clone(name);
                        result.push(HoleBinding { name, type_id });
                    }
                }
            }
            GraphNode::Let { bindings, .. } => {
                let mut bindings = bindings.iter().collect_vec();
                bindings.sort_by_key(|(name, _)| *name);

                for (name, let_id) in bindings {
                    if seen.insert(SmolStr::clone(name))
                        && let Some(type_id) = state.checked.nodes.lookup_let(*let_id)
                    {
                        let name = SmolStr::clone(name);
                        result.push(HoleBinding { name, type_id });
                    }
                }
            }
            GraphNode::Forall { .. } | GraphNode::Implicit { .. } => {}
        }
    }
}

fn collect_resolved_term_hole_bindings<Q>(
    state: &CheckState,
    context: &CheckContext<Q>,
    seen: &mut FxHashSet<SmolStr>,
    result: &mut Vec<HoleBinding>,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for (name, file_id, term_id) in context.resolved.locals.iter_terms() {
        collect_resolved_term_hole_binding(state, context, name, file_id, term_id, seen, result)?;
    }

    for import in context.resolved.unqualified.values().flatten() {
        let visible_terms =
            import.iter_terms().filter(|(_, _, _, kind)| !matches!(kind, ImportKind::Hidden));

        for (name, file_id, term_id, _) in visible_terms {
            collect_resolved_term_hole_binding(
                state, context, name, file_id, term_id, seen, result,
            )?;
        }
    }

    if !context.resolved.unqualified.contains_key("Prim") {
        for (name, file_id, term_id) in context.prim_resolved.exports.iter_terms() {
            collect_resolved_term_hole_binding(
                state, context, name, file_id, term_id, seen, result,
            )?;
        }
    }

    Ok(())
}

fn collect_resolved_term_hole_binding<Q>(
    state: &CheckState,
    context: &CheckContext<Q>,
    name: &SmolStr,
    file_id: FileId,
    term_id: TermItemId,
    seen: &mut FxHashSet<SmolStr>,
    result: &mut Vec<HoleBinding>,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    if seen.contains(name) {
        return Ok(());
    }

    let Some((resolved_file_id, resolved_term_id)) =
        context.resolved.lookup_term(&context.prim_resolved, None, name)
    else {
        return Ok(());
    };

    if (resolved_file_id, resolved_term_id) != (file_id, term_id) {
        return Ok(());
    }

    let type_id = toolkit::lookup_file_term(state, context, resolved_file_id, resolved_term_id)?;
    let name = SmolStr::clone(name);
    seen.insert(SmolStr::clone(&name));
    result.push(HoleBinding { name, type_id });

    Ok(())
}
