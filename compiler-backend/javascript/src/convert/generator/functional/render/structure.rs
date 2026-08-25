//! Module-level references and lazy binding requirements.

use functional::tree::{DeclarationKind, ExpressionKind, Global, GlobalId, Module};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    collect_expression_globals, collect_expression_references, global_file, is_abstraction,
    reaches_initializer,
};

pub(super) fn collect_module_references(module: &Module) -> Vec<Global> {
    let mut seen = FxHashSet::default();
    let mut globals = Vec::new();
    for declaration in module.declarations.iter() {
        let DeclarationKind::Value(expression) = declaration.kind else {
            continue;
        };
        collect_expression_references(module, expression, &mut seen, &mut globals);
    }
    globals.into_iter().filter(|global| global_file(global.id) != module.file_id).collect()
}

pub(super) fn cyclic_instance_initializers(module: &Module) -> FxHashSet<GlobalId> {
    let values = module
        .declarations
        .iter()
        .filter_map(|declaration| match declaration.kind {
            DeclarationKind::Value(expression)
                if !is_abstraction(&module.storage[expression].kind) =>
            {
                Some((declaration.global.id, expression))
            }
            DeclarationKind::Value(_)
            | DeclarationKind::Constructor { .. }
            | DeclarationKind::Foreign => None,
        })
        .collect_vec();
    let positions = values
        .iter()
        .enumerate()
        .map(|(position, (id, _))| (*id, position))
        .collect::<FxHashMap<_, _>>();
    let mut dependencies = vec![Vec::new(); values.len()];
    for (position, (_, expression)) in values.iter().enumerate() {
        let mut globals = FxHashSet::default();
        collect_expression_globals(module, *expression, true, &mut globals);
        dependencies[position]
            .extend(globals.into_iter().filter_map(|global| positions.get(&global).copied()));
        dependencies[position].sort_unstable();
        dependencies[position].dedup();
    }
    let mut cyclic = FxHashSet::default();
    for position in 0..values.len() {
        let mut visited = FxHashSet::default();
        if reaches_initializer(position, position, &dependencies, &mut visited) {
            cyclic.insert(values[position].0);
        }
    }
    if cyclic.iter().all(|id| matches!(id, GlobalId::Instance(_))) {
        cyclic
    } else {
        FxHashSet::default()
    }
}

pub(super) fn has_local_lazy_initializers(module: &Module) -> bool {
    module.storage.expressions().any(|(_, expression)| {
        matches!(
            &expression.kind,
            ExpressionKind::Let { recursive: true, bindings, .. }
                if !bindings
                    .iter()
                    .all(|binding| is_abstraction(&module.storage[binding.expression].kind))
        )
    })
}
