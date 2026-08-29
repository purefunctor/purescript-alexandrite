//! Simplification of functional trees before backend-specific code generation.

use itertools::Itertools;
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
        pending.extend(expression_children(&storage[expression].kind));
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
    for child in expression_children(&kind) {
        optimize_expression(storage, child, recursive_globals, visited);
    }

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

fn local_uses(storage: &Storage, expression: ExpressionId, parameter: LocalId) -> usize {
    if matches!(
        &storage[expression].kind,
        ExpressionKind::Local { parameter: local } if local.id == parameter
    ) {
        return 1;
    }
    expression_children(&storage[expression].kind)
        .into_iter()
        .map(|child| local_uses(storage, child, parameter))
        .sum()
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
    for child in expression_children(&kind) {
        substitute_local(storage, child, parameter, replacement);
    }
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
        | ExpressionKind::Application { .. }
        | ExpressionKind::UncurriedApplication { .. }
        | ExpressionKind::StyleX { .. }
        | ExpressionKind::IfThenElse { .. }
        | ExpressionKind::Case { .. }
        | ExpressionKind::Guarded { .. }
        | ExpressionKind::Let { .. }
        | ExpressionKind::LetPattern { .. }
        | ExpressionKind::Effect { .. } => false,
    }
}

pub(crate) fn expression_children(kind: &ExpressionKind) -> Vec<ExpressionId> {
    match kind {
        ExpressionKind::Literal { .. }
        | ExpressionKind::Constructor { .. }
        | ExpressionKind::Global { .. }
        | ExpressionKind::Local { .. }
        | ExpressionKind::SynthesizedEvidence { .. }
        | ExpressionKind::TrivialEvidence => Vec::new(),
        ExpressionKind::Array { elements } => elements.to_vec(),
        ExpressionKind::Record { fields } => fields.iter().map(|field| field.expression).collect(),
        ExpressionKind::RecordUpdate { record, updates } => {
            let mut children = vec![*record];
            update_children(updates, &mut children);
            children
        }
        ExpressionKind::Project { record, .. } | ExpressionKind::Unary { value: record, .. } => {
            vec![*record]
        }
        ExpressionKind::Binary { left, right, .. } => vec![*left, *right],
        ExpressionKind::Abstraction { body, .. }
        | ExpressionKind::UncurriedAbstraction { body, .. } => vec![*body],
        ExpressionKind::Application { function, arguments, .. }
        | ExpressionKind::UncurriedApplication { function, arguments, .. } => {
            let mut children = Vec::with_capacity(arguments.len() + 1);
            children.push(*function);
            children.extend(arguments.iter().copied());
            children
        }
        ExpressionKind::StyleX { argument, .. } => vec![*argument],
        ExpressionKind::IfThenElse { condition, then, else_ } => {
            vec![*condition, *then, *else_]
        }
        ExpressionKind::Case { scrutinees, alternatives } => {
            let mut children = scrutinees.to_vec();
            children.extend(alternatives.iter().map(|alternative| alternative.expression));
            children
        }
        ExpressionKind::Guarded { alternatives } => {
            let mut children = Vec::new();
            for alternative in alternatives.iter() {
                for guard in alternative.guards.iter() {
                    let expression = match guard {
                        Guard::Boolean(expression) | Guard::Pattern { expression, .. } => {
                            *expression
                        }
                    };
                    children.push(expression);
                }
                children.push(alternative.expression);
            }
            children
        }
        ExpressionKind::Let { bindings, body, .. } => {
            let binding_expressions = bindings.iter().map(|binding| binding.expression);
            let mut children = binding_expressions.collect_vec();
            children.push(*body);
            children
        }
        ExpressionKind::LetPattern { value, body, .. } => vec![*value, *body],
        ExpressionKind::Effect { effect } => match effect {
            EffectExpression::Pure(value) => vec![*value],
            EffectExpression::Bind { action, body, .. } => vec![*action, *body],
            EffectExpression::Map { function, action } => vec![*function, *action],
            EffectExpression::Apply { function_action, argument_action } => {
                vec![*function_action, *argument_action]
            }
        },
    }
}

fn update_children(updates: &[RecordUpdate], children: &mut Vec<ExpressionId>) {
    for update in updates {
        match update {
            RecordUpdate::Leaf { expression, .. } => children.push(*expression),
            RecordUpdate::Branch { updates, .. } => update_children(updates, children),
        }
    }
}
