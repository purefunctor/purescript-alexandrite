//! JavaScript module generation.

mod analysis;
mod names;
mod render;
mod syntax;

use files::FileId;
use itertools::Itertools;
use pretty::Arena as DocumentArena;
use rustc_hash::{FxHashMap, FxHashSet};
use ssa::tree::{
    CallingConvention, Declaration, DeclarationKind, Function, Global, GlobalIdentity, Instruction,
    Terminator, ValueId,
};

use crate::error::{ModuleError, ModuleResult, UnsupportedState};
use crate::module::{Module, module_filename, runtime_filename};
use crate::pretty::Writer;
use crate::tree::{ExpressionId, Tree};

use self::analysis::{
    FunctionContext, InlineExpressionContext, VisitState, collect_module_references,
    cyclic_instance_initializers, function_globals, identity_file, initializer_value_is_inlineable,
    instruction_value_uses, visit_initializer,
};
use self::names::NameAllocator;
use self::syntax::constructor_expression;

pub(super) struct Generator<'m> {
    module: &'m ssa::tree::Module,
    global_names: FxHashMap<GlobalIdentity, String>,
    external_module_namespaces: FxHashMap<FileId, String>,
    external_references: Vec<&'m Global>,
    foreign_namespace: Option<String>,
    runtime_namespace: Option<String>,
    lazy_global_names: FxHashMap<GlobalIdentity, String>,
    reserved_module_names: FxHashSet<String>,
}

impl<'m> Generator<'m> {
    pub(super) fn new(module: &'m ssa::tree::Module) -> Generator<'m> {
        let mut allocator = NameAllocator::default();
        let mut global_names = FxHashMap::default();

        for declaration in module.declarations.iter() {
            let name = allocator.allocate(&declaration.global.item_name);
            global_names.insert(declaration.global.identity, name.clone());
        }

        let external_references = collect_module_references(module);
        let mut external_module_namespaces = FxHashMap::default();
        for global in &external_references {
            let file_id = identity_file(global.identity);
            let dependency = module
                .dependencies
                .iter()
                .find(|dependency| dependency.file_id == file_id)
                .expect("invariant violated: external global has no module dependency");
            external_module_namespaces
                .entry(file_id)
                .or_insert_with(|| allocator.allocate(&dependency.module_name.replace('.', "_")));
        }

        let has_foreign = module
            .declarations
            .iter()
            .any(|declaration| matches!(declaration.kind, DeclarationKind::Foreign));
        let foreign_namespace = has_foreign.then(|| allocator.allocate("$foreign"));
        let lazy_globals = cyclic_instance_initializers(module);
        let runtime_namespace = (!lazy_globals.is_empty()).then(|| allocator.allocate("$runtime"));
        let lazy_global_names = lazy_globals.into_iter().map(|identity| {
            let global_name = &global_names[&identity];
            let lazy_name = allocator.allocate(&format!("$lazy_{global_name}"));
            (identity, lazy_name)
        });
        let lazy_global_names = lazy_global_names.collect();
        let reserved_module_names = allocator.allocated_names().cloned();
        let reserved_module_names = reserved_module_names.collect();

        Generator {
            module,
            global_names,
            external_module_namespaces,
            external_references,
            foreign_namespace,
            runtime_namespace,
            lazy_global_names,
            reserved_module_names,
        }
    }

    pub(super) fn generate(self) -> ModuleResult<Module> {
        let mut tree = Tree::default();
        let documents = DocumentArena::new();
        let mut writer = Writer::new(&documents);
        self.render_imports(&mut tree, &mut writer);
        self.render_constructors(&mut tree, &mut writer);
        self.render_source_functions(&mut tree, &mut writer)?;
        self.render_foreign_declarations(&mut tree, &mut writer);
        self.render_lazy_initializers(&mut tree, &mut writer)?;
        self.render_value_declarations(&mut tree, &mut writer)?;
        self.render_exports(&mut writer);

        let dependencies = self.module.dependencies.iter().map(|dependency| dependency.file_id);
        let dependencies = dependencies.collect_vec();
        let requires_foreign = self.foreign_namespace.is_some();
        let requires_runtime = self.runtime_namespace.is_some();
        Ok(Module::new(
            self.module.file_id,
            self.module.name.to_string(),
            writer.finish(),
            dependencies,
            requires_foreign,
            requires_runtime,
        ))
    }

    fn render_lazy_initializers(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
    ) -> ModuleResult<()> {
        let Some(runtime) = &self.runtime_namespace else { return Ok(()) };
        for declaration in self.module.declarations.iter() {
            let Some(lazy_name) = self.lazy_global_names.get(&declaration.global.identity) else {
                continue;
            };
            let DeclarationKind::Value { initializer } = declaration.kind else {
                unreachable!("invariant violated: lazy JavaScript declaration is not a value")
            };
            let name = tree.string(declaration.global.item_name.as_str());
            let function = &self.module.storage[initializer];
            let context = FunctionContext::new(self, function);
            writer.expression_block(
                format!("const {lazy_name} = {runtime}.binding("),
                tree,
                name,
                ", () => {",
                "});",
                |tree, writer| self.render_function_body(tree, writer, function, &context),
            )?;
            writer.blank();
        }
        Ok(())
    }

    fn render_imports(&self, tree: &mut Tree, writer: &mut Writer<'_>) {
        let files = self.external_module_namespaces.keys().map(|&file_id| {
            let dependency = self.module_dependency(file_id);
            (file_id, dependency.module_name.as_str())
        });
        let mut files = files.collect_vec();
        files.sort_by_key(|(_, module_name)| *module_name);
        for (file_id, module_name) in files {
            let namespace = &self.external_module_namespaces[&file_id];
            let path = format!("../{}", module_filename(module_name));
            let path = tree.string(path);
            writer.expression_line(format!("import * as {namespace} from "), tree, path, ";");
        }
        if let Some(namespace) = &self.foreign_namespace {
            writer.line(format!("import * as {namespace} from \"./foreign.js\";"));
        }
        if let Some(namespace) = &self.runtime_namespace {
            writer.line(format!("import * as {namespace} from \"../{}\";", runtime_filename()));
        }
        if !self.external_references.is_empty()
            || self.foreign_namespace.is_some()
            || self.runtime_namespace.is_some()
        {
            writer.blank();
        }
    }

    fn render_constructors(&self, tree: &mut Tree, writer: &mut Writer<'_>) {
        let mut rendered = false;
        for declaration in self.module.declarations.iter() {
            let DeclarationKind::Constructor { arity } = declaration.kind else {
                continue;
            };
            let name = self.global_name(declaration.global.identity);
            let expression =
                constructor_expression(tree, declaration.global.item_name.as_str(), arity);
            let export =
                if self.declaration_is_inline_exported(declaration) { "export " } else { "" };
            writer.expression_line(format!("{export}const {name} = "), tree, expression, ";");
            rendered = true;
        }
        if rendered {
            writer.blank();
        }
    }

    fn render_source_functions(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
    ) -> ModuleResult<()> {
        for declaration in self.module.declarations.iter() {
            let DeclarationKind::Function { function } = declaration.kind else {
                continue;
            };
            let function = &self.module.storage[function];
            let name = self.global_name(declaration.global.identity);
            let exported = self.declaration_is_inline_exported(declaration);
            self.render_function(tree, writer, name, function, exported)?;
            writer.blank();
        }
        Ok(())
    }

    fn render_foreign_declarations(&self, tree: &mut Tree, writer: &mut Writer<'_>) {
        let Some(namespace) = &self.foreign_namespace else {
            return;
        };
        let mut rendered = false;
        for declaration in self.module.declarations.iter() {
            if !matches!(declaration.kind, DeclarationKind::Foreign) {
                continue;
            }
            let name = self.global_name(declaration.global.identity);
            let object = tree.identifier(namespace);
            let index = tree.string(declaration.global.item_name.as_str());
            let access = tree.index(object, index);
            let export =
                if self.declaration_is_inline_exported(declaration) { "export " } else { "" };
            writer.expression_line(format!("{export}const {name} = "), tree, access, ";");
            rendered = true;
        }
        if rendered {
            writer.blank();
        }
    }

    fn render_value_declarations(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
    ) -> ModuleResult<()> {
        let declarations = self.sorted_value_declarations()?;
        for declaration in declarations {
            let DeclarationKind::Value { initializer } = declaration.kind else {
                unreachable!("invariant violated: sorted non-value JavaScript declaration")
            };
            let function = &self.module.storage[initializer];
            let name = self.global_name(declaration.global.identity);
            let export =
                if self.declaration_is_inline_exported(declaration) { "export " } else { "" };
            if let Some(lazy_name) = self.lazy_global_names.get(&declaration.global.identity) {
                writer.line(format!("{export}const {name} = {lazy_name}();"));
                writer.blank();
                continue;
            }
            if let Some(expression) = self.initializer_expression(tree, function)? {
                writer.expression_line(format!("{export}const {name} = "), tree, expression, ";");
            } else {
                let context = FunctionContext::new(self, function);
                writer.block(format!("{export}const {name} = (() => {{"), "})();", |writer| {
                    self.render_function_body(tree, writer, function, &context)
                })?;
            }
            writer.blank();
        }
        Ok(())
    }

    fn initializer_expression(
        &self,
        tree: &mut Tree,
        function: &Function,
    ) -> ModuleResult<Option<ExpressionId>> {
        assert!(
            matches!(function.calling_convention, CallingConvention::Initializer)
                && function.captures.is_empty()
                && function.parameters.is_empty(),
            "invariant violated: invalid SSA initializer function"
        );
        let [entry] = function.blocks.as_ref() else {
            return Ok(None);
        };
        let block = &self.module.storage[*entry];
        if *entry != function.entry || !block.parameters.is_empty() {
            return Ok(None);
        }
        let Terminator::Return { value: return_value } = block.terminator else {
            return Ok(None);
        };

        let mut pending = Vec::<(ValueId, ExpressionId)>::new();
        for instruction in &block.instructions {
            let Instruction::Assign { result, value } = instruction else {
                return Ok(None);
            };
            if !initializer_value_is_inlineable(value) {
                return Ok(None);
            }

            // An expression may replace its SSA operands only when they are the pending suffix in
            // JavaScript evaluation order. Consuming that suffix preserves the original instruction
            // order without classifying operations by purity or duplicating a value expression.
            let uses = instruction_value_uses(value);
            let Some(start) = pending.len().checked_sub(uses.len()) else {
                return Ok(None);
            };
            let pending_values = pending[start..].iter().map(|(value, _)| *value);
            let used_values = uses.iter().copied();
            if !pending_values.eq(used_values) {
                return Ok(None);
            }
            let expressions = pending.drain(start..);
            let expressions = expressions.collect_vec();
            let context = InlineExpressionContext::new(expressions);
            let expression = self.instruction_expression(tree, value, &context)?;
            assert!(
                context.is_empty(),
                "invariant violated: inline JavaScript expression left SSA operands unused"
            );
            pending.push((*result, expression));
        }

        let [(value, _)] = pending.as_slice() else {
            return Ok(None);
        };
        if *value != return_value {
            return Ok(None);
        }
        let (_, expression) =
            pending.pop().expect("invariant violated: missing inline initializer expression");
        Ok(Some(expression))
    }

    fn render_exports(&self, writer: &mut Writer<'_>) {
        let mut rendered = false;
        for declaration in self.module.declarations.iter() {
            if !declaration.exported {
                continue;
            }
            let local = self.global_name(declaration.global.identity);
            if local == declaration.global.item_name {
                continue;
            }
            let exported = module_export_name(&declaration.global.item_name);
            writer.line(format!("export {{ {local} as {exported} }};"));
            rendered = true;
        }

        for exports in self.module.surface.indirect.iter() {
            let specifiers =
                exports.globals.iter().map(|global| module_export_name(&global.item_name));
            let specifiers = specifiers.collect_vec();
            let dependency = self.module_dependency(exports.file_id);
            let path = format!("../{}", module_filename(&dependency.module_name));
            writer.re_export(specifiers, &path);
            rendered = true;
        }
        if rendered {
            writer.blank();
        }
    }

    fn declaration_is_inline_exported(&self, declaration: &Declaration) -> bool {
        let local = self.global_name(declaration.global.identity);
        declaration.exported && local == declaration.global.item_name
    }

    fn sorted_value_declarations(&self) -> ModuleResult<Vec<&Declaration>> {
        let values = self
            .module
            .declarations
            .iter()
            .filter(|declaration| matches!(declaration.kind, DeclarationKind::Value { .. }));
        let values = values.collect_vec();
        let positions = values
            .iter()
            .enumerate()
            .map(|(position, declaration)| (declaration.global.identity, position));
        let positions = positions.collect::<FxHashMap<_, _>>();
        let mut dependencies = vec![Vec::new(); values.len()];
        for (position, declaration) in values.iter().enumerate() {
            let DeclarationKind::Value { initializer } = declaration.kind else {
                unreachable!("invariant violated: expected value declaration")
            };
            for dependency in function_globals(self.module, initializer) {
                if let Some(dependency) = positions.get(&dependency) {
                    let source_is_lazy =
                        self.lazy_global_names.contains_key(&declaration.global.identity);
                    let dependency_is_lazy =
                        self.lazy_global_names.contains_key(&values[*dependency].global.identity);
                    if source_is_lazy && dependency_is_lazy {
                        continue;
                    }
                    dependencies[position].push(*dependency);
                }
            }
            dependencies[position].sort_unstable();
            dependencies[position].dedup();
        }

        let mut states = vec![VisitState::Unvisited; values.len()];
        let mut ordered = Vec::new();
        for position in 0..values.len() {
            visit_initializer(position, &dependencies, &mut states, &mut ordered)
                .map_err(|state| self.unsupported(state))?;
        }
        let ordered = ordered.into_iter().map(|position| values[position]);
        Ok(ordered.collect_vec())
    }

    fn global_name(&self, identity: GlobalIdentity) -> &str {
        self.global_names
            .get(&identity)
            .map(String::as_str)
            .expect("invariant violated: JavaScript global has no allocated name")
    }

    fn module_dependency(&self, file_id: FileId) -> &ssa::tree::ModuleDependency {
        self.module
            .dependencies
            .iter()
            .find(|dependency| dependency.file_id == file_id)
            .expect("invariant violated: referenced module has no dependency metadata")
    }

    fn global_expression(&self, tree: &mut Tree, global: &Global) -> ExpressionId {
        let file_id = identity_file(global.identity);
        if file_id == self.module.file_id {
            if let Some(lazy_name) = self.lazy_global_names.get(&global.identity) {
                let lazy = tree.identifier(lazy_name);
                tree.call(lazy, vec![])
            } else {
                tree.identifier(self.global_name(global.identity))
            }
        } else {
            let namespace = self
                .external_module_namespaces
                .get(&file_id)
                .expect("invariant violated: external JavaScript global has no module namespace");
            let namespace = tree.identifier(namespace);
            tree.member(namespace, global.item_name.as_str())
        }
    }

    fn unsupported(&self, state: UnsupportedState) -> ModuleError {
        ModuleError::Unsupported { file_id: self.module.file_id, state }
    }
}

fn module_export_name(name: &str) -> String {
    if self::names::identifier_is_binding(name) {
        name.to_owned()
    } else {
        crate::pretty::render_string(name)
    }
}
