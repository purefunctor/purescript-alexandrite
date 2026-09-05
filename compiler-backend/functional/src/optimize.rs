//! Simplification of functional trees before backend-specific code generation.

use std::convert::Infallible;

use rustc_hash::FxHashSet;
use smol_str::{SmolStr, format_smolstr};

use crate::tree::{
    Binding, EffectExpression, ExpressionId, ExpressionKind, GlobalId, Guard, Literal, LocalId,
    RecordUpdate, Storage, UnaryOperator,
};

pub fn inline_simple_bindings(
    storage: &mut Storage,
    expression: ExpressionId,
    recursive_globals: &FxHashSet<GlobalId>,
) {
    let mut visited = FxHashSet::default();
    optimize_expression(storage, expression, recursive_globals, &mut visited);
}

pub(crate) fn reachable_expressions(
    storage: &Storage,
    roots: impl IntoIterator<Item = ExpressionId>,
) -> FxHashSet<ExpressionId> {
    let mut reachable = FxHashSet::default();
    let mut pending = roots.into_iter().collect::<Vec<_>>();
    while let Some(expression) = pending.pop() {
        if !reachable.insert(expression) {
            continue;
        }
        for_each_expression_child(&storage[expression].kind, |child| pending.push(child));
    }
    reachable
}

pub(crate) fn expression_globals(
    storage: &Storage,
    expression: ExpressionId,
) -> FxHashSet<GlobalId> {
    let reachable = reachable_expressions(storage, [expression]);
    let globals = reachable.into_iter().filter_map(|expression| match &storage[expression].kind {
        ExpressionKind::Constructor { global } | ExpressionKind::Global { global } => {
            Some(global.id)
        }
        _ => None,
    });
    globals.collect()
}

fn optimize_expression(
    storage: &mut Storage,
    expression: ExpressionId,
    recursive_globals: &FxHashSet<GlobalId>,
    visited: &mut FxHashSet<ExpressionId>,
) {
    if !visited.insert(expression) {
        return;
    }

    let kind = storage[expression].kind.clone();
    for_each_expression_child(&kind, |child| {
        optimize_expression(storage, child, recursive_globals, visited);
    });

    if fold_literal_negation(storage, expression) {
        return;
    }

    let ExpressionKind::Let { recursive: false, bindings, body } = kind else {
        return;
    };
    let mut bindings = bindings.to_vec();
    while let Some(position) = bindings.iter().position(|binding| {
        let uses = binding_uses(storage, &bindings, body, binding.parameter.id);
        uses > 0
            && (is_trivial_expression(storage, binding.expression, recursive_globals)
                || uses == 1
                    && is_simple_expression(storage, binding.expression, recursive_globals))
    }) {
        let binding = bindings.remove(position);
        for remaining in &bindings {
            substitute_local(
                storage,
                remaining.expression,
                binding.parameter.id,
                binding.expression,
            );
        }
        substitute_local(storage, body, binding.parameter.id, binding.expression);
    }

    let replacement = if bindings.is_empty() {
        storage[body].kind.clone()
    } else {
        ExpressionKind::Let { recursive: false, bindings: bindings.into(), body }
    };
    storage.replace_expression_kind(expression, replacement);
    fold_literal_negation(storage, expression);
}

fn fold_literal_negation(storage: &mut Storage, expression: ExpressionId) -> bool {
    let (operator, value) = match &storage[expression].kind {
        ExpressionKind::Unary { operator, value } => (*operator, *value),
        _ => return false,
    };
    let Some(literal) = folded_negation(storage, operator, value) else {
        return false;
    };
    storage.replace_expression_kind(expression, ExpressionKind::Literal { literal });
    true
}

fn folded_negation(
    storage: &Storage,
    operator: UnaryOperator,
    value: ExpressionId,
) -> Option<Literal> {
    match (operator, &storage[value].kind) {
        (
            UnaryOperator::IntegerNegate,
            ExpressionKind::Literal { literal: Literal::Integer(value) },
        ) => Some(Literal::Integer(value.wrapping_neg())),
        (
            UnaryOperator::NumberNegate,
            ExpressionKind::Literal { literal: Literal::Number(value) },
        ) => Some(Literal::Number(negated_number(value))),
        (
            UnaryOperator::BooleanNot | UnaryOperator::IntegerNegate | UnaryOperator::NumberNegate,
            _,
        ) => None,
    }
}

fn negated_number(value: &str) -> SmolStr {
    if value.parse::<f64>().ok() == Some(0.0) {
        return SmolStr::new("0.0");
    }
    match value.strip_prefix('-') {
        Some(value) => SmolStr::new(value),
        None => format_smolstr!("-{value}"),
    }
}

fn binding_uses(
    storage: &Storage,
    bindings: &[Binding],
    body: ExpressionId,
    parameter: LocalId,
) -> usize {
    let binding_uses = bindings
        .iter()
        .map(|binding| local_uses(storage, binding.expression, parameter))
        .sum::<usize>();
    binding_uses + local_uses(storage, body, parameter)
}

pub fn local_uses(storage: &Storage, expression: ExpressionId, parameter: LocalId) -> usize {
    if matches!(
        &storage[expression].kind,
        ExpressionKind::Local { parameter: local } if local.id == parameter
    ) {
        return 1;
    }
    let mut uses = 0;
    for_each_expression_child(&storage[expression].kind, |child| {
        uses += local_uses(storage, child, parameter);
    });
    uses
}

fn substitute_local(
    storage: &mut Storage,
    expression: ExpressionId,
    parameter: LocalId,
    replacement: ExpressionId,
) {
    if matches!(
        &storage[expression].kind,
        ExpressionKind::Local { parameter: local } if local.id == parameter
    ) {
        let replacement = storage[replacement].kind.clone();
        storage.replace_expression_kind(expression, replacement);
        return;
    }
    let kind = storage[expression].kind.clone();
    for_each_expression_child(&kind, |child| {
        substitute_local(storage, child, parameter, replacement);
    });
}

fn is_trivial_expression(
    storage: &Storage,
    expression: ExpressionId,
    recursive_globals: &FxHashSet<GlobalId>,
) -> bool {
    match &storage[expression].kind {
        ExpressionKind::Literal { .. }
        | ExpressionKind::Constructor { .. }
        | ExpressionKind::Local { .. }
        | ExpressionKind::SynthesizedEvidence { .. }
        | ExpressionKind::TrivialEvidence => true,
        ExpressionKind::Global { global } => !recursive_globals.contains(&global.id),
        _ => false,
    }
}

fn is_simple_expression(
    storage: &Storage,
    expression: ExpressionId,
    recursive_globals: &FxHashSet<GlobalId>,
) -> bool {
    match &storage[expression].kind {
        ExpressionKind::Literal { .. }
        | ExpressionKind::Constructor { .. }
        | ExpressionKind::Local { .. }
        | ExpressionKind::Abstraction { .. }
        | ExpressionKind::UncurriedAbstraction { .. }
        | ExpressionKind::SynthesizedEvidence { .. }
        | ExpressionKind::TrivialEvidence => true,
        ExpressionKind::Global { global } => !recursive_globals.contains(&global.id),
        ExpressionKind::Array { elements } => elements
            .iter()
            .all(|element| is_simple_expression(storage, *element, recursive_globals)),
        ExpressionKind::Record { fields } => fields
            .iter()
            .all(|field| is_simple_expression(storage, field.expression, recursive_globals)),
        ExpressionKind::Project { record, .. } | ExpressionKind::Unary { value: record, .. } => {
            is_simple_expression(storage, *record, recursive_globals)
        }
        ExpressionKind::Binary { left, right, .. } => {
            is_simple_expression(storage, *left, recursive_globals)
                && is_simple_expression(storage, *right, recursive_globals)
        }
        ExpressionKind::RecordUpdate { .. }
        | ExpressionKind::Error
        | ExpressionKind::Application { .. }
        | ExpressionKind::UncurriedApplication { .. }
        | ExpressionKind::StyleX(_)
        | ExpressionKind::IfThenElse { .. }
        | ExpressionKind::Case { .. }
        | ExpressionKind::Guarded { .. }
        | ExpressionKind::Let { .. }
        | ExpressionKind::LetPattern { .. }
        | ExpressionKind::Effect { .. } => false,
    }
}

pub fn for_each_expression_child(kind: &ExpressionKind, mut visit: impl FnMut(ExpressionId)) {
    let result: Result<(), Infallible> = try_for_each_expression_child(kind, |child| {
        visit(child);
        Ok(())
    });
    let Ok(()) = result;
}

pub fn try_for_each_expression_child<Error>(
    kind: &ExpressionKind,
    mut visit: impl FnMut(ExpressionId) -> Result<(), Error>,
) -> Result<(), Error> {
    match kind {
        ExpressionKind::Error
        | ExpressionKind::Literal { .. }
        | ExpressionKind::Constructor { .. }
        | ExpressionKind::Global { .. }
        | ExpressionKind::Local { .. }
        | ExpressionKind::SynthesizedEvidence { .. }
        | ExpressionKind::TrivialEvidence => {}
        ExpressionKind::Array { elements } => {
            for &element in elements.iter() {
                visit(element)?;
            }
        }
        ExpressionKind::Record { fields } => {
            for field in fields.iter() {
                visit(field.expression)?;
            }
        }
        ExpressionKind::RecordUpdate { record, updates } => {
            visit(*record)?;
            try_for_each_update_child(updates, &mut visit)?;
        }
        ExpressionKind::Project { record, .. } | ExpressionKind::Unary { value: record, .. } => {
            visit(*record)?;
        }
        ExpressionKind::Binary { left, right, .. } => {
            visit(*left)?;
            visit(*right)?;
        }
        ExpressionKind::Abstraction { body, .. }
        | ExpressionKind::UncurriedAbstraction { body, .. } => visit(*body)?,
        ExpressionKind::Application { function, arguments, .. }
        | ExpressionKind::UncurriedApplication { function, arguments, .. } => {
            visit(*function)?;
            for &argument in arguments.iter() {
                visit(argument)?;
            }
        }
        ExpressionKind::StyleX(stylex) => stylex.try_for_each_child(&mut visit)?,
        ExpressionKind::IfThenElse { condition, then, else_ } => {
            visit(*condition)?;
            visit(*then)?;
            visit(*else_)?;
        }
        ExpressionKind::Case { scrutinees, alternatives } => {
            for &scrutinee in scrutinees.iter() {
                visit(scrutinee)?;
            }
            for alternative in alternatives.iter() {
                visit(alternative.expression)?;
            }
        }
        ExpressionKind::Guarded { alternatives } => {
            for alternative in alternatives.iter() {
                for guard in alternative.guards.iter() {
                    let expression = match guard {
                        Guard::Boolean(expression) | Guard::Pattern { expression, .. } => {
                            *expression
                        }
                    };
                    visit(expression)?;
                }
                visit(alternative.expression)?;
            }
        }
        ExpressionKind::Let { bindings, body, .. } => {
            for binding in bindings.iter() {
                visit(binding.expression)?;
            }
            visit(*body)?;
        }
        ExpressionKind::LetPattern { value, body, .. } => {
            visit(*value)?;
            visit(*body)?;
        }
        ExpressionKind::Effect { effect } => match effect {
            EffectExpression::Pure(value) => visit(*value)?,
            EffectExpression::Bind { action, body, .. } => {
                visit(*action)?;
                visit(*body)?;
            }
            EffectExpression::Map { function, action } => {
                visit(*function)?;
                visit(*action)?;
            }
            EffectExpression::Apply { function_action, argument_action } => {
                visit(*function_action)?;
                visit(*argument_action)?;
            }
        },
    }
    Ok(())
}

fn try_for_each_update_child<Error>(
    updates: &[RecordUpdate],
    visit: &mut impl FnMut(ExpressionId) -> Result<(), Error>,
) -> Result<(), Error> {
    for update in updates {
        match update {
            RecordUpdate::Leaf { expression, .. } => visit(*expression)?,
            RecordUpdate::Branch { updates, .. } => try_for_each_update_child(updates, visit)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stylex::{StyleXConditionalCase, StyleXExpression, StyleXWhenRelation};
    use crate::tree::{Field, FieldIdentity};

    fn expression(index: u32) -> ExpressionId {
        ExpressionId::from_raw(index.into())
    }

    fn visited_children(kind: &ExpressionKind) -> Vec<ExpressionId> {
        let mut children = Vec::new();
        for_each_expression_child(kind, |child| children.push(child));
        children
    }

    #[test]
    fn nested_record_updates_visit_in_order_and_stop_at_first_error() {
        let field = Field { identity: FieldIdentity::Label("field".into()), name: "field".into() };
        let kind = ExpressionKind::RecordUpdate {
            record: expression(0),
            updates: [
                RecordUpdate::Leaf { field: field.clone(), expression: expression(1) },
                RecordUpdate::Branch {
                    field: field.clone(),
                    updates: [
                        RecordUpdate::Leaf { field: field.clone(), expression: expression(2) },
                        RecordUpdate::Leaf { field: field.clone(), expression: expression(3) },
                    ]
                    .into(),
                },
                RecordUpdate::Leaf { field, expression: expression(4) },
            ]
            .into(),
        };
        assert_eq!(visited_children(&kind), (0..5).map(expression).collect::<Vec<_>>());

        let mut visited = Vec::new();
        let result = try_for_each_expression_child(&kind, |child| {
            visited.push(child);
            if child == expression(2) { Err(child) } else { Ok(()) }
        });
        assert_eq!(result, Err(expression(2)));
        assert_eq!(visited, (0..3).map(expression).collect::<Vec<_>>());
    }

    #[test]
    fn stylex_conditional_cases_visit_optional_markers_in_order() {
        let kind = ExpressionKind::StyleX(StyleXExpression::ConditionalValue {
            default: expression(0),
            cases: [
                StyleXConditionalCase {
                    relation: StyleXWhenRelation::Ancestor,
                    selector: expression(1),
                    marker: None,
                    value: expression(2),
                },
                StyleXConditionalCase {
                    relation: StyleXWhenRelation::Descendant,
                    selector: expression(3),
                    marker: Some(expression(4)),
                    value: expression(5),
                },
            ]
            .into(),
        });
        assert_eq!(visited_children(&kind), (0..6).map(expression).collect::<Vec<_>>());

        let mut visited = Vec::new();
        let result = try_for_each_expression_child(&kind, |child| {
            visited.push(child);
            if child == expression(4) { Err(child) } else { Ok(()) }
        });
        assert_eq!(result, Err(expression(4)));
        assert_eq!(visited, (0..5).map(expression).collect::<Vec<_>>());
    }
}
