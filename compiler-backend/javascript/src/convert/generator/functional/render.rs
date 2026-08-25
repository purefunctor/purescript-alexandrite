//! Rendering functional trees as JavaScript modules.

mod analysis;
mod inline;
mod structure;
mod syntax;

use files::FileId;
use functional::tree::{
    Binding, CaseAlternative, Declaration, DeclarationKind, EffectExpression,
    ExpressionId as FunctionalExpressionId, ExpressionKind, Global, GlobalId, Guard,
    GuardedAlternative, LocalId, Module as FunctionalModule, Parameter, PatternId, PatternKind,
    RecordUpdate,
};
use itertools::Itertools;
use pretty::Arena as DocumentArena;
use rustc_hash::{FxHashMap, FxHashSet};

use super::super::names::NameAllocator;
use crate::error::{ModuleError, ModuleResult, UnsupportedState};
use crate::module::{Module, module_filename, runtime_filename};
use crate::pretty::Writer;
use crate::tree::{BinaryOperator, ExpressionId, ObjectProperty, Tree};

use self::analysis::{VisitState, visit_initializer};
use self::inline::{is_abstraction, pattern_parameter};
use self::structure::{
    collect_module_references, cyclic_instance_initializers, has_local_lazy_initializers,
};
use self::syntax::{
    binary_expression, combine_conditions, constructor_expression, curried_call_expression,
    literal_expression, module_export_name, synthesized_evidence_expression, unary_expression,
};

pub(crate) struct Generator<'m> {
    module: &'m FunctionalModule,
    global_names: FxHashMap<GlobalId, String>,
    external_module_namespaces: FxHashMap<FileId, String>,
    external_references: Vec<Global>,
    foreign_namespace: Option<String>,
    runtime_namespace: Option<String>,
    lazy_global_names: FxHashMap<GlobalId, String>,
    reserved_module_names: FxHashSet<String>,
}

#[derive(Debug, Clone)]
enum LocalBinding {
    Direct(String),
    Lazy(String),
}

#[derive(Clone)]
struct FunctionContext {
    allocator: NameAllocator,
    locals: FxHashMap<LocalId, LocalBinding>,
}

#[derive(Clone, Copy)]
enum Destination<'a> {
    Return,
    Assign(&'a str),
    AssignAndBreak { name: &'a str, label: &'a str },
}

#[derive(Default)]
struct PatternPlan {
    conditions: Vec<ExpressionId>,
    bindings: Vec<(String, ExpressionId)>,
}

struct ModuleRenderer<'a, 'm, 'd> {
    generator: &'a Generator<'m>,
    tree: &'a mut Tree,
    writer: &'a mut Writer<'d>,
}

struct FunctionRenderer<'a, 'm, 'd> {
    generator: &'a Generator<'m>,
    tree: &'a mut Tree,
    writer: &'a mut Writer<'d>,
    context: &'a mut FunctionContext,
}

impl FunctionContext {
    fn new(reserved: &FxHashSet<String>) -> FunctionContext {
        FunctionContext {
            allocator: NameAllocator::with_reserved(reserved.iter().cloned()),
            locals: FxHashMap::default(),
        }
    }

    fn allocate(&mut self, preferred: &str) -> String {
        self.allocator.allocate(preferred)
    }

    fn bind_direct(&mut self, parameter: &Parameter, name: String) {
        self.locals.insert(parameter.id, LocalBinding::Direct(name));
    }

    fn bind_lazy(&mut self, parameter: &Parameter, name: String) {
        self.locals.insert(parameter.id, LocalBinding::Lazy(name));
    }
}

impl<'m> Generator<'m> {
    pub(crate) fn new(module: &'m FunctionalModule) -> Generator<'m> {
        let mut allocator = NameAllocator::default();
        let mut global_names = FxHashMap::default();
        for declaration in module.declarations.iter() {
            let name = allocator.allocate(&declaration.global.item_name);
            global_names.insert(declaration.global.id, name);
        }

        let external_references = collect_module_references(module);
        let mut external_module_namespaces = FxHashMap::default();
        for global in &external_references {
            let file_id = global_file(global.id);
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
        let requires_runtime = !lazy_globals.is_empty() || has_local_lazy_initializers(module);
        let runtime_namespace = requires_runtime.then(|| allocator.allocate("$runtime"));
        let lazy_global_names = lazy_globals.into_iter().map(|id| {
            let global_name = &global_names[&id];
            let lazy_name = allocator.allocate(&format!("$lazy_{global_name}"));
            (id, lazy_name)
        });
        let lazy_global_names = lazy_global_names.collect();
        let reserved_module_names = allocator.allocated_names().cloned().collect();

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

    pub(crate) fn generate(self) -> ModuleResult<Module> {
        let mut tree = Tree::default();
        let documents = DocumentArena::new();
        let mut writer = Writer::new(&documents);
        {
            let mut renderer =
                ModuleRenderer { generator: &self, tree: &mut tree, writer: &mut writer };
            render_imports(&mut renderer);
            render_constructors(&mut renderer);
            render_source_functions(&mut renderer)?;
            render_foreign_declarations(&mut renderer);
            render_lazy_initializers(&mut renderer)?;
            render_value_declarations(&mut renderer)?;
            render_exports(&mut renderer);
        }

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

    fn renderer<'a, 'd>(
        &'a self,
        tree: &'a mut Tree,
        writer: &'a mut Writer<'d>,
        context: &'a mut FunctionContext,
    ) -> FunctionRenderer<'a, 'm, 'd> {
        FunctionRenderer { generator: self, tree, writer, context }
    }
}

fn render_imports(renderer: &mut ModuleRenderer<'_, '_, '_>) {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let mut files = generator
        .external_module_namespaces
        .keys()
        .map(|&file_id| {
            let dependency = generator.module_dependency(file_id);
            (file_id, dependency.module_name.as_str())
        })
        .collect_vec();
    files.sort_by_key(|(_, module_name)| *module_name);
    for (file_id, module_name) in files {
        let namespace = &generator.external_module_namespaces[&file_id];
        let path = tree.string(format!("../{}", module_filename(module_name)));
        writer.expression_line(format!("import * as {namespace} from "), tree, path, ";");
    }
    if let Some(namespace) = &generator.foreign_namespace {
        writer.line(format!("import * as {namespace} from \"./foreign.js\";"));
    }
    if let Some(namespace) = &generator.runtime_namespace {
        writer.line(format!("import * as {namespace} from \"../{}\";", runtime_filename()));
    }
    if !generator.external_references.is_empty()
        || generator.foreign_namespace.is_some()
        || generator.runtime_namespace.is_some()
    {
        writer.blank();
    }
}

fn render_constructors(renderer: &mut ModuleRenderer<'_, '_, '_>) {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let mut rendered = false;
    for declaration in generator.module.declarations.iter() {
        let DeclarationKind::Constructor { arity } = declaration.kind else {
            continue;
        };
        let name = generator.global_name(declaration.global.id);
        let expression = constructor_expression(tree, &declaration.global.item_name, arity);
        let export =
            if generator.declaration_is_inline_exported(declaration) { "export " } else { "" };
        writer.expression_line(format!("{export}const {name} = "), tree, expression, ";");
        rendered = true;
    }
    if rendered {
        writer.blank();
    }
}

fn render_source_functions(renderer: &mut ModuleRenderer<'_, '_, '_>) -> ModuleResult<()> {
    let generator = renderer.generator;
    for declaration in generator.module.declarations.iter() {
        let DeclarationKind::Value(expression) = declaration.kind else {
            continue;
        };
        let kind = &generator.module.storage[expression].kind;
        if !matches!(
            kind,
            ExpressionKind::Abstraction { .. } | ExpressionKind::UncurriedAbstraction { .. }
        ) {
            continue;
        }
        let name = generator.global_name(declaration.global.id);
        let exported = generator.declaration_is_inline_exported(declaration);
        let mut context = FunctionContext::new(&generator.reserved_module_names);
        let mut function_renderer =
            generator.renderer(renderer.tree, renderer.writer, &mut context);
        render_named_function(&mut function_renderer, name, expression, exported)?;
        renderer.writer.blank();
    }
    Ok(())
}

fn render_named_function(
    renderer: &mut FunctionRenderer<'_, '_, '_>,
    name: &str,
    expression: FunctionalExpressionId,
    exported: bool,
) -> ModuleResult<()> {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let context = &mut *renderer.context;
    let export = if exported { "export " } else { "" };
    match &generator.module.storage[expression].kind {
        ExpressionKind::Abstraction { parameters, body } => {
            let (argument, parameter) = generator.first_argument(parameters, context);
            let header = format!("{export}function {name}({argument}) {{");
            writer.block(header, "}", |writer| {
                if let Some(parameter) = parameter {
                    generator.render_curried_parameter(
                        tree,
                        writer,
                        parameter,
                        &argument,
                        &parameters[1..],
                        *body,
                        context,
                    )
                } else {
                    generator.render_expression(tree, writer, *body, Destination::Return, context)
                }
            })
        }
        ExpressionKind::UncurriedAbstraction { parameters, body } => {
            let arguments = parameters
                .iter()
                .map(|pattern| generator.allocate_pattern_argument(*pattern, context))
                .collect_vec();
            let header = format!("{export}function {name}({}) {{", arguments.join(", "));
            writer.block(header, "}", |writer| {
                generator.render_uncurried_parameters(
                    tree, writer, parameters, &arguments, 0, *body, context,
                )
            })
        }
        _ => unreachable!("invariant violated: named JavaScript function is not an abstraction"),
    }
}

impl Generator<'_> {
    fn first_argument(
        &self,
        parameters: &[PatternId],
        context: &mut FunctionContext,
    ) -> (String, Option<PatternId>) {
        match parameters.first().copied() {
            Some(pattern) => (self.allocate_pattern_argument(pattern, context), Some(pattern)),
            None => (String::new(), None),
        }
    }

    fn render_curried_parameter(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        pattern: PatternId,
        argument: &str,
        remaining: &[PatternId],
        body: FunctionalExpressionId,
        context: &mut FunctionContext,
    ) -> ModuleResult<()> {
        let value = tree.identifier(argument);
        let plan = self.pattern_plan(tree, pattern, value, Some(argument), context)?;
        self.render_pattern_scope(tree, writer, plan, context, |tree, writer, context| {
            let Some((pattern, remaining)) = remaining.split_first() else {
                return self.render_expression(tree, writer, body, Destination::Return, context);
            };
            let argument = self.allocate_pattern_argument(*pattern, context);
            writer.block(format!("return {argument} => {{"), "};", |writer| {
                self.render_curried_parameter(
                    tree, writer, *pattern, &argument, remaining, body, context,
                )
            })
        })
    }

    fn render_uncurried_parameters(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        patterns: &[PatternId],
        arguments: &[String],
        position: usize,
        body: FunctionalExpressionId,
        context: &mut FunctionContext,
    ) -> ModuleResult<()> {
        let Some(pattern) = patterns.get(position).copied() else {
            return self.render_expression(tree, writer, body, Destination::Return, context);
        };
        let argument = &arguments[position];
        let value = tree.identifier(argument);
        let plan = self.pattern_plan(tree, pattern, value, Some(argument), context)?;
        self.render_pattern_scope(tree, writer, plan, context, |tree, writer, context| {
            self.render_uncurried_parameters(
                tree,
                writer,
                patterns,
                arguments,
                position + 1,
                body,
                context,
            )
        })
    }
}

fn render_foreign_declarations(renderer: &mut ModuleRenderer<'_, '_, '_>) {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let Some(namespace) = &generator.foreign_namespace else {
        return;
    };
    let mut rendered = false;
    for declaration in generator.module.declarations.iter() {
        if !matches!(declaration.kind, DeclarationKind::Foreign) {
            continue;
        }
        let name = generator.global_name(declaration.global.id);
        let object = tree.identifier(namespace);
        let index = tree.string(declaration.global.item_name.as_str());
        let access = tree.index(object, index);
        let export =
            if generator.declaration_is_inline_exported(declaration) { "export " } else { "" };
        writer.expression_line(format!("{export}const {name} = "), tree, access, ";");
        rendered = true;
    }
    if rendered {
        writer.blank();
    }
}

fn render_lazy_initializers(renderer: &mut ModuleRenderer<'_, '_, '_>) -> ModuleResult<()> {
    let generator = renderer.generator;
    let Some(runtime) = &generator.runtime_namespace else {
        return Ok(());
    };
    for declaration in generator.module.declarations.iter() {
        let Some(lazy_name) = generator.lazy_global_names.get(&declaration.global.id) else {
            continue;
        };
        let DeclarationKind::Value(expression) = declaration.kind else {
            unreachable!("invariant violated: lazy JavaScript declaration is not a value")
        };
        let name = renderer.tree.string(declaration.global.item_name.as_str());
        let mut context = FunctionContext::new(&generator.reserved_module_names);
        renderer.writer.expression_block(
            format!("const {lazy_name} = {runtime}.binding("),
            renderer.tree,
            name,
            ", () => {",
            "});",
            |tree, writer| {
                generator.render_expression(
                    tree,
                    writer,
                    expression,
                    Destination::Return,
                    &mut context,
                )
            },
        )?;
        renderer.writer.blank();
    }
    Ok(())
}

fn render_value_declarations(renderer: &mut ModuleRenderer<'_, '_, '_>) -> ModuleResult<()> {
    let generator = renderer.generator;
    for declaration in sorted_value_declarations(generator)? {
        let DeclarationKind::Value(expression) = declaration.kind else {
            unreachable!("invariant violated: sorted JavaScript declaration is not a value")
        };
        if is_abstraction(&generator.module.storage[expression].kind) {
            continue;
        }
        let name = generator.global_name(declaration.global.id);
        let export =
            if generator.declaration_is_inline_exported(declaration) { "export " } else { "" };
        if let Some(lazy_name) = generator.lazy_global_names.get(&declaration.global.id) {
            renderer.writer.line(format!("{export}const {name} = {lazy_name}();"));
            renderer.writer.blank();
            continue;
        }

        let mut context = FunctionContext::new(&generator.reserved_module_names);
        if let Some(value) =
            generator.try_inline_expression(renderer.tree, expression, &mut context)?
        {
            renderer.writer.expression_line(
                format!("{export}const {name} = "),
                renderer.tree,
                value,
                ";",
            );
        } else {
            renderer.writer.block(
                format!("{export}const {name} = (() => {{"),
                "})();",
                |writer| {
                    generator.render_expression(
                        renderer.tree,
                        writer,
                        expression,
                        Destination::Return,
                        &mut context,
                    )
                },
            )?;
        }
        renderer.writer.blank();
    }
    Ok(())
}

impl Generator<'_> {
    fn render_expression(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        expression: FunctionalExpressionId,
        destination: Destination<'_>,
        context: &mut FunctionContext,
    ) -> ModuleResult<()> {
        match &self.module.storage[expression].kind {
            ExpressionKind::IfThenElse { condition, then, else_ } => {
                let condition = self.expression_value(tree, writer, *condition, context)?;
                writer.if_else_with_state(
                    tree,
                    condition,
                    context,
                    |tree, writer, context| {
                        self.render_expression(tree, writer, *then, destination, context)
                    },
                    |tree, writer, context| {
                        self.render_expression(tree, writer, *else_, destination, context)
                    },
                )
            }
            ExpressionKind::Case { scrutinees, alternatives } => {
                let mut renderer = self.renderer(tree, writer, context);
                render_case(&mut renderer, scrutinees, alternatives, destination)
            }
            ExpressionKind::Guarded { alternatives } => {
                let mut renderer = self.renderer(tree, writer, context);
                render_guarded(&mut renderer, alternatives, destination)
            }
            ExpressionKind::Let { recursive, bindings, body } => {
                let mut renderer = self.renderer(tree, writer, context);
                render_let(&mut renderer, *recursive, bindings)?;
                self.render_expression(tree, writer, *body, destination, context)
            }
            ExpressionKind::LetPattern { pattern, value, body } => {
                let source = *value;
                let value = self.expression_value(tree, writer, source, context)?;
                let value = self.materialize_pattern_value(tree, writer, source, value, context);
                let plan = self.pattern_plan(tree, *pattern, value, None, context)?;
                self.render_pattern_scope(tree, writer, plan, context, |tree, writer, context| {
                    self.render_expression(tree, writer, *body, destination, context)
                })
            }
            _ => {
                let value = self.expression_value(tree, writer, expression, context)?;
                self.render_destination(tree, writer, value, destination);
                Ok(())
            }
        }
    }

    fn render_destination(
        &self,
        tree: &Tree,
        writer: &mut Writer<'_>,
        value: ExpressionId,
        destination: Destination<'_>,
    ) {
        match destination {
            Destination::Return => writer.expression_line("return ", tree, value, ";"),
            Destination::Assign(name) => {
                writer.expression_line(format!("{name} = "), tree, value, ";");
            }
            Destination::AssignAndBreak { name, label } => {
                writer.expression_line(format!("{name} = "), tree, value, ";");
                writer.line(format!("break {label};"));
            }
        }
    }

    fn expression_value(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        expression: FunctionalExpressionId,
        context: &mut FunctionContext,
    ) -> ModuleResult<ExpressionId> {
        if let Some(expression) = self.try_inline_expression(tree, expression, context)? {
            return Ok(expression);
        }
        match &self.module.storage[expression].kind {
            ExpressionKind::Array { elements } => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements.iter() {
                    let value = self.expression_value(tree, writer, *element, context)?;
                    let value = self.materialize_value(tree, writer, value, "$element", context);
                    values.push(value);
                }
                let array = tree.array(values);
                Ok(self.materialize_value(tree, writer, array, "$array", context))
            }
            ExpressionKind::Record { fields } => {
                let mut properties = Vec::with_capacity(fields.len());
                for field in fields.iter() {
                    let value = self.expression_value(tree, writer, field.expression, context)?;
                    let value = self.materialize_value(tree, writer, value, "$field", context);
                    properties
                        .push(ObjectProperty::Field { name: field.field.name.to_string(), value });
                }
                let record = tree.object(properties);
                Ok(self.materialize_value(tree, writer, record, "$record", context))
            }
            ExpressionKind::RecordUpdate { record, updates } => {
                let update =
                    self.record_update_expression(tree, writer, *record, updates, context)?;
                Ok(self.materialize_value(tree, writer, update, "$update", context))
            }
            ExpressionKind::Project { record, field } => {
                let record = self.expression_value(tree, writer, *record, context)?;
                let record = self.materialize_value(tree, writer, record, "$record", context);
                let value = tree.member(record, field.name.as_str());
                Ok(self.materialize_value(tree, writer, value, "$field", context))
            }
            ExpressionKind::Unary { operator, value } => {
                let value = self.expression_value(tree, writer, *value, context)?;
                let value = self.materialize_value(tree, writer, value, "$operand", context);
                let result = unary_expression(tree, *operator, value);
                Ok(self.materialize_value(tree, writer, result, "$result", context))
            }
            ExpressionKind::Binary { operator, left, right } => {
                let left = self.expression_value(tree, writer, *left, context)?;
                let left = self.materialize_value(tree, writer, left, "$left", context);
                let right = self.expression_value(tree, writer, *right, context)?;
                let right = self.materialize_value(tree, writer, right, "$right", context);
                let result = binary_expression(tree, *operator, left, right);
                Ok(self.materialize_value(tree, writer, result, "$result", context))
            }
            ExpressionKind::Abstraction { parameters, body } => {
                let name = context.allocate("$closure");
                self.render_abstraction_binding(
                    tree, writer, &name, parameters, *body, false, context,
                )?;
                Ok(tree.identifier(name))
            }
            ExpressionKind::UncurriedAbstraction { parameters, body } => {
                let name = context.allocate("$closure");
                self.render_abstraction_binding(
                    tree, writer, &name, parameters, *body, true, context,
                )?;
                Ok(tree.identifier(name))
            }
            ExpressionKind::Application { function, arguments } => {
                let function = self.expression_value(tree, writer, *function, context)?;
                let mut function =
                    self.materialize_value(tree, writer, function, "$function", context);
                if arguments.is_empty() {
                    let call = tree.call(function, vec![]);
                    return Ok(self.materialize_value(tree, writer, call, "$call", context));
                }
                for argument in arguments.iter() {
                    let value = self.expression_value(tree, writer, *argument, context)?;
                    let value = self.materialize_value(tree, writer, value, "$argument", context);
                    let call = tree.call(function, vec![value]);
                    function = self.materialize_value(tree, writer, call, "$call", context);
                }
                Ok(function)
            }
            ExpressionKind::UncurriedApplication { function, arguments } => {
                let function = self.expression_value(tree, writer, *function, context)?;
                let function = self.materialize_value(tree, writer, function, "$function", context);
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments.iter() {
                    let value = self.expression_value(tree, writer, *argument, context)?;
                    let value = self.materialize_value(tree, writer, value, "$argument", context);
                    values.push(value);
                }
                let call = tree.call(function, values);
                Ok(self.materialize_value(tree, writer, call, "$call", context))
            }
            ExpressionKind::Effect { effect } => {
                let mut renderer = self.renderer(tree, writer, context);
                effect_expression(&mut renderer, effect)
            }
            ExpressionKind::IfThenElse { .. }
            | ExpressionKind::Case { .. }
            | ExpressionKind::Guarded { .. }
            | ExpressionKind::Let { .. }
            | ExpressionKind::LetPattern { .. } => {
                let name = context.allocate("$result");
                writer.line(format!("let {name};"));
                self.render_expression(
                    tree,
                    writer,
                    expression,
                    Destination::Assign(&name),
                    context,
                )?;
                Ok(tree.identifier(name))
            }
            ExpressionKind::Literal { .. }
            | ExpressionKind::Constructor { .. }
            | ExpressionKind::Global { .. }
            | ExpressionKind::Local { .. }
            | ExpressionKind::SynthesizedEvidence { .. }
            | ExpressionKind::TrivialEvidence => {
                unreachable!("invariant violated: atomic functional expression was not inlineable")
            }
        }
    }

    fn inline_expression(
        &self,
        tree: &mut Tree,
        expression: FunctionalExpressionId,
        context: &mut FunctionContext,
    ) -> ModuleResult<Option<ExpressionId>> {
        let expression = match &self.module.storage[expression].kind {
            ExpressionKind::Literal { literal } => {
                literal_expression(tree, literal, self.module.file_id)?
            }
            ExpressionKind::Array { elements } => {
                let Some(elements) = elements
                    .iter()
                    .map(|element| self.inline_expression(tree, *element, context))
                    .collect::<ModuleResult<Option<Vec<_>>>>()?
                else {
                    return Ok(None);
                };
                tree.array(elements)
            }
            ExpressionKind::Record { fields } => {
                let mut properties = Vec::with_capacity(fields.len());
                for field in fields.iter() {
                    let Some(value) = self.inline_expression(tree, field.expression, context)?
                    else {
                        return Ok(None);
                    };
                    properties
                        .push(ObjectProperty::Field { name: field.field.name.to_string(), value });
                }
                tree.object(properties)
            }
            ExpressionKind::Project { record, field } => {
                let Some(record) = self.inline_expression(tree, *record, context)? else {
                    return Ok(None);
                };
                tree.member(record, field.name.as_str())
            }
            ExpressionKind::Unary { operator, value } => {
                let Some(value) = self.inline_expression(tree, *value, context)? else {
                    return Ok(None);
                };
                unary_expression(tree, *operator, value)
            }
            ExpressionKind::Binary { operator, left, right } => {
                let Some(left) = self.inline_expression(tree, *left, context)? else {
                    return Ok(None);
                };
                let Some(right) = self.inline_expression(tree, *right, context)? else {
                    return Ok(None);
                };
                binary_expression(tree, *operator, left, right)
            }
            ExpressionKind::Constructor { global } => self.global_expression(tree, global)?,
            ExpressionKind::Global { global } => self.global_expression(tree, global)?,
            ExpressionKind::Local { parameter } => {
                local_expression(self, tree, parameter, context)?
            }
            ExpressionKind::Abstraction { parameters, body } => {
                let Some(expression) =
                    self.inline_abstraction(tree, parameters, *body, false, context)?
                else {
                    return Ok(None);
                };
                expression
            }
            ExpressionKind::UncurriedAbstraction { parameters, body } => {
                let Some(expression) =
                    self.inline_abstraction(tree, parameters, *body, true, context)?
                else {
                    return Ok(None);
                };
                expression
            }
            ExpressionKind::Application { function, arguments } => {
                let Some(function) = self.inline_expression(tree, *function, context)? else {
                    return Ok(None);
                };
                let Some(arguments) = arguments
                    .iter()
                    .map(|argument| self.inline_expression(tree, *argument, context))
                    .collect::<ModuleResult<Option<Vec<_>>>>()?
                else {
                    return Ok(None);
                };
                curried_call_expression(tree, function, arguments)
            }
            ExpressionKind::UncurriedApplication { function, arguments } => {
                let Some(function) = self.inline_expression(tree, *function, context)? else {
                    return Ok(None);
                };
                let Some(arguments) = arguments
                    .iter()
                    .map(|argument| self.inline_expression(tree, *argument, context))
                    .collect::<ModuleResult<Option<Vec<_>>>>()?
                else {
                    return Ok(None);
                };
                tree.call(function, arguments)
            }
            ExpressionKind::SynthesizedEvidence { evidence } => {
                synthesized_evidence_expression(tree, evidence)
            }
            ExpressionKind::TrivialEvidence => tree.object(vec![]),
            ExpressionKind::RecordUpdate { .. }
            | ExpressionKind::IfThenElse { .. }
            | ExpressionKind::Case { .. }
            | ExpressionKind::Guarded { .. }
            | ExpressionKind::Let { .. }
            | ExpressionKind::LetPattern { .. }
            | ExpressionKind::Effect { .. } => return Ok(None),
        };
        Ok(Some(expression))
    }

    fn try_inline_expression(
        &self,
        tree: &mut Tree,
        expression: FunctionalExpressionId,
        context: &mut FunctionContext,
    ) -> ModuleResult<Option<ExpressionId>> {
        let mut inline_context = context.clone();
        let expression = self.inline_expression(tree, expression, &mut inline_context)?;
        if expression.is_some() {
            *context = inline_context;
        }
        Ok(expression)
    }

    fn inline_abstraction(
        &self,
        tree: &mut Tree,
        parameters: &[PatternId],
        body: FunctionalExpressionId,
        uncurried: bool,
        context: &mut FunctionContext,
    ) -> ModuleResult<Option<ExpressionId>> {
        let mut arguments = Vec::with_capacity(parameters.len());
        for pattern in parameters {
            let argument = self.allocate_pattern_argument(*pattern, context);
            if !self.bind_inline_pattern(*pattern, &argument, context) {
                return Ok(None);
            }
            arguments.push(argument);
        }
        let Some(mut body) = self.inline_expression(tree, body, context)? else {
            return Ok(None);
        };
        if uncurried {
            body = tree.arrow(arguments, body);
        } else if arguments.is_empty() {
            body = tree.arrow(vec![], body);
        } else {
            for argument in arguments.into_iter().rev() {
                body = tree.arrow(vec![argument], body);
            }
        }
        Ok(Some(body))
    }

    fn bind_inline_pattern(
        &self,
        pattern: PatternId,
        argument: &str,
        context: &mut FunctionContext,
    ) -> bool {
        match &self.module.storage[pattern].kind {
            PatternKind::Variable(parameter) => {
                context.bind_direct(parameter, argument.to_owned());
                true
            }
            PatternKind::Named { parameter, pattern } => {
                context.bind_direct(parameter, argument.to_owned());
                self.bind_inline_pattern(*pattern, argument, context)
            }
            PatternKind::Wildcard => true,
            PatternKind::Literal(_)
            | PatternKind::Array(_)
            | PatternKind::Record(_)
            | PatternKind::Constructor { .. } => false,
        }
    }

    fn render_abstraction_binding(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        name: &str,
        parameters: &[PatternId],
        body: FunctionalExpressionId,
        uncurried: bool,
        context: &mut FunctionContext,
    ) -> ModuleResult<()> {
        if uncurried {
            let arguments = parameters
                .iter()
                .map(|pattern| self.allocate_pattern_argument(*pattern, context))
                .collect_vec();
            writer.block(
                format!("const {name} = ({}) => {{", arguments.join(", ")),
                "};",
                |writer| {
                    self.render_uncurried_parameters(
                        tree, writer, parameters, &arguments, 0, body, context,
                    )
                },
            )
        } else {
            let (argument, parameter) = self.first_argument(parameters, context);
            let header = if parameter.is_some() {
                format!("const {name} = {argument} => {{")
            } else {
                format!("const {name} = () => {{")
            };
            writer.block(header, "};", |writer| {
                if let Some(parameter) = parameter {
                    self.render_curried_parameter(
                        tree,
                        writer,
                        parameter,
                        &argument,
                        &parameters[1..],
                        body,
                        context,
                    )
                } else {
                    self.render_expression(tree, writer, body, Destination::Return, context)
                }
            })
        }
    }
}

fn render_let(
    renderer: &mut FunctionRenderer<'_, '_, '_>,
    recursive: bool,
    bindings: &[Binding],
) -> ModuleResult<()> {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let context = &mut *renderer.context;
    if recursive
        && bindings
            .iter()
            .all(|binding| is_abstraction(&generator.module.storage[binding.expression].kind))
    {
        let names =
            bindings.iter().map(|binding| context.allocate(&binding.parameter.name)).collect_vec();
        for (binding, name) in bindings.iter().zip(&names) {
            context.bind_direct(&binding.parameter, name.clone());
        }
        for (binding, name) in bindings.iter().zip(&names) {
            match &generator.module.storage[binding.expression].kind {
                ExpressionKind::Abstraction { parameters, body } => {
                    generator.render_abstraction_binding(
                        tree, writer, name, parameters, *body, false, context,
                    )?;
                }
                ExpressionKind::UncurriedAbstraction { parameters, body } => {
                    generator.render_abstraction_binding(
                        tree, writer, name, parameters, *body, true, context,
                    )?;
                }
                _ => {
                    unreachable!("invariant violated: recursive closure group contains a value")
                }
            }
        }
        return Ok(());
    }
    if recursive {
        let mut renderer = generator.renderer(tree, writer, context);
        return render_lazy_let(&mut renderer, bindings);
    }
    for binding in bindings {
        let name = context.allocate(&binding.parameter.name);
        match &generator.module.storage[binding.expression].kind {
            ExpressionKind::Abstraction { parameters, body } => {
                context.bind_direct(&binding.parameter, name.clone());
                generator.render_abstraction_binding(
                    tree, writer, &name, parameters, *body, false, context,
                )?;
            }
            ExpressionKind::UncurriedAbstraction { parameters, body } => {
                context.bind_direct(&binding.parameter, name.clone());
                generator.render_abstraction_binding(
                    tree, writer, &name, parameters, *body, true, context,
                )?;
            }
            _ => {
                let value =
                    generator.expression_value(tree, writer, binding.expression, context)?;
                writer.expression_line(format!("const {name} = "), tree, value, ";");
                context.bind_direct(&binding.parameter, name);
            }
        }
    }
    Ok(())
}

fn render_lazy_let(
    renderer: &mut FunctionRenderer<'_, '_, '_>,
    bindings: &[Binding],
) -> ModuleResult<()> {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let context = &mut *renderer.context;
    let runtime = generator
        .runtime_namespace
        .as_deref()
        .expect("invariant violated: recursive local values require the JavaScript runtime");
    let mut bindings = bindings.iter().collect_vec();
    bindings.sort_by_key(|binding| binding.source_order);
    let names =
        bindings.iter().map(|binding| context.allocate(&binding.parameter.name)).collect_vec();
    let accessors =
        names.iter().map(|name| context.allocate(&format!("$lazy_{name}"))).collect_vec();
    for ((binding, name), accessor) in bindings.iter().zip(&names).zip(&accessors) {
        let _ = name;
        context.bind_lazy(&binding.parameter, accessor.clone());
        writer.line(format!("let {accessor};"));
    }
    for (binding, accessor) in bindings.iter().zip(&accessors) {
        let source_name = tree.string(binding.parameter.name.as_str());
        writer.expression_block(
            format!("{accessor} = {runtime}.binding("),
            tree,
            source_name,
            ", () => {",
            "});",
            |tree, writer| {
                generator.render_expression(
                    tree,
                    writer,
                    binding.expression,
                    Destination::Return,
                    context,
                )
            },
        )?;
    }
    for ((binding, name), accessor) in bindings.iter().zip(&names).zip(&accessors) {
        writer.line(format!("const {name} = {accessor}();"));
        context.bind_direct(&binding.parameter, name.clone());
    }
    Ok(())
}

fn render_case(
    renderer: &mut FunctionRenderer<'_, '_, '_>,
    scrutinees: &[FunctionalExpressionId],
    alternatives: &[CaseAlternative],
    destination: Destination<'_>,
) -> ModuleResult<()> {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let context = &mut *renderer.context;
    let mut values = Vec::with_capacity(scrutinees.len());
    for scrutinee in scrutinees {
        let value = generator.expression_value(tree, writer, *scrutinee, context)?;
        let value = generator.materialize_pattern_value(tree, writer, *scrutinee, value, context);
        values.push(value);
    }
    let scrutinees = values;
    match destination {
        Destination::Return | Destination::AssignAndBreak { .. } => generator
            .render_case_alternatives(
                tree,
                writer,
                &scrutinees,
                alternatives,
                destination,
                context,
            ),
        Destination::Assign(name) => {
            let label = context.allocate("$case");
            writer.block(format!("{label}: {{"), "}", |writer| {
                generator.render_case_alternatives(
                    tree,
                    writer,
                    &scrutinees,
                    alternatives,
                    Destination::AssignAndBreak { name, label: &label },
                    context,
                )
            })
        }
    }
}

impl Generator<'_> {
    fn render_case_alternatives(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        scrutinees: &[ExpressionId],
        alternatives: &[CaseAlternative],
        destination: Destination<'_>,
        context: &mut FunctionContext,
    ) -> ModuleResult<()> {
        for alternative in alternatives {
            let mut plan = PatternPlan::default();
            for (pattern, value) in alternative.patterns.iter().zip(scrutinees) {
                self.extend_pattern_plan(tree, *pattern, *value, None, context, &mut plan)?;
            }
            let condition = combine_conditions(tree, &plan.conditions);
            if let Some(condition) = condition {
                writer.expression_block("if (", tree, condition, ") {", "}", |tree, writer| {
                    self.render_pattern_bindings(tree, writer, &plan);
                    self.render_case_alternative_expression(
                        tree,
                        writer,
                        alternative.expression,
                        destination,
                        context,
                    )
                })?;
            } else if matches!(
                self.module.storage[alternative.expression].kind,
                ExpressionKind::Guarded { .. }
            ) {
                writer.block("{", "}", |writer| {
                    self.render_pattern_bindings(tree, writer, &plan);
                    self.render_case_alternative_expression(
                        tree,
                        writer,
                        alternative.expression,
                        destination,
                        context,
                    )
                })?;
            } else {
                self.render_pattern_bindings(tree, writer, &plan);
                self.render_expression(tree, writer, alternative.expression, destination, context)?;
                return Ok(());
            }
        }
        self.render_pattern_failure(writer);
        Ok(())
    }

    fn render_case_alternative_expression(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        expression: FunctionalExpressionId,
        destination: Destination<'_>,
        context: &mut FunctionContext,
    ) -> ModuleResult<()> {
        if let ExpressionKind::Guarded { alternatives } = &self.module.storage[expression].kind {
            self.render_guard_alternatives(tree, writer, alternatives, destination, context)
        } else {
            self.render_expression(tree, writer, expression, destination, context)
        }
    }
}

fn render_guarded(
    renderer: &mut FunctionRenderer<'_, '_, '_>,
    alternatives: &[GuardedAlternative],
    destination: Destination<'_>,
) -> ModuleResult<()> {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let context = &mut *renderer.context;
    match destination {
        Destination::Return | Destination::AssignAndBreak { .. } => {
            generator.render_guard_alternatives(
                tree,
                writer,
                alternatives,
                destination,
                context,
            )?;
            generator.render_pattern_failure(writer);
            Ok(())
        }
        Destination::Assign(name) => {
            let label = context.allocate("$guard");
            writer.block(format!("{label}: {{"), "}", |writer| {
                generator.render_guard_alternatives(
                    tree,
                    writer,
                    alternatives,
                    Destination::AssignAndBreak { name, label: &label },
                    context,
                )?;
                generator.render_pattern_failure(writer);
                Ok(())
            })
        }
    }
}

impl Generator<'_> {
    fn render_guard_alternatives(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        alternatives: &[GuardedAlternative],
        destination: Destination<'_>,
        context: &mut FunctionContext,
    ) -> ModuleResult<()> {
        for alternative in alternatives {
            self.render_guards(
                tree,
                writer,
                &alternative.guards,
                0,
                alternative.expression,
                destination,
                context,
            )?;
        }
        Ok(())
    }

    fn render_guards(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        guards: &[Guard],
        position: usize,
        expression: FunctionalExpressionId,
        destination: Destination<'_>,
        context: &mut FunctionContext,
    ) -> ModuleResult<()> {
        let Some(guard) = guards.get(position) else {
            return self.render_expression(tree, writer, expression, destination, context);
        };
        match guard {
            Guard::Boolean(condition) => {
                let condition = self.expression_value(tree, writer, *condition, context)?;
                writer.expression_block("if (", tree, condition, ") {", "}", |tree, writer| {
                    self.render_guards(
                        tree,
                        writer,
                        guards,
                        position + 1,
                        expression,
                        destination,
                        context,
                    )
                })
            }
            Guard::Pattern { expression: value, pattern } => {
                let source = *value;
                let value = self.expression_value(tree, writer, source, context)?;
                let value = self.materialize_pattern_value(tree, writer, source, value, context);
                let plan = self.pattern_plan(tree, *pattern, value, None, context)?;
                let condition = combine_conditions(tree, &plan.conditions);
                if let Some(condition) = condition {
                    writer.expression_block("if (", tree, condition, ") {", "}", |tree, writer| {
                        self.render_pattern_bindings(tree, writer, &plan);
                        self.render_guards(
                            tree,
                            writer,
                            guards,
                            position + 1,
                            expression,
                            destination,
                            context,
                        )
                    })
                } else {
                    self.render_pattern_bindings(tree, writer, &plan);
                    self.render_guards(
                        tree,
                        writer,
                        guards,
                        position + 1,
                        expression,
                        destination,
                        context,
                    )
                }
            }
        }
    }

    fn render_pattern_scope(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        plan: PatternPlan,
        context: &mut FunctionContext,
        render: impl FnOnce(&mut Tree, &mut Writer<'_>, &mut FunctionContext) -> ModuleResult<()>,
    ) -> ModuleResult<()> {
        let condition = combine_conditions(tree, &plan.conditions);
        if let Some(condition) = condition {
            writer.if_else(
                tree,
                condition,
                |tree, writer| {
                    self.render_pattern_bindings(tree, writer, &plan);
                    render(tree, writer, context)
                },
                |_, writer| {
                    self.render_pattern_failure(writer);
                    Ok(())
                },
            )
        } else {
            self.render_pattern_bindings(tree, writer, &plan);
            render(tree, writer, context)
        }
    }

    fn render_pattern_bindings(&self, tree: &Tree, writer: &mut Writer<'_>, plan: &PatternPlan) {
        for (name, value) in &plan.bindings {
            writer.expression_line(format!("const {name} = "), tree, *value, ";");
        }
    }

    fn render_pattern_failure(&self, writer: &mut Writer<'_>) {
        writer.line("throw new Error(\"Pattern match failure\");");
    }

    fn pattern_plan(
        &self,
        tree: &mut Tree,
        pattern: PatternId,
        value: ExpressionId,
        root_name: Option<&str>,
        context: &mut FunctionContext,
    ) -> ModuleResult<PatternPlan> {
        let mut plan = PatternPlan::default();
        self.extend_pattern_plan(tree, pattern, value, root_name, context, &mut plan)?;
        Ok(plan)
    }

    fn extend_pattern_plan(
        &self,
        tree: &mut Tree,
        pattern: PatternId,
        value: ExpressionId,
        root_name: Option<&str>,
        context: &mut FunctionContext,
        plan: &mut PatternPlan,
    ) -> ModuleResult<()> {
        match &self.module.storage[pattern].kind {
            PatternKind::Variable(parameter) => {
                self.bind_pattern_parameter(parameter, value, root_name, context, plan);
            }
            PatternKind::Named { parameter, pattern } => {
                self.bind_pattern_parameter(parameter, value, root_name, context, plan);
                self.extend_pattern_plan(tree, *pattern, value, root_name, context, plan)?;
            }
            PatternKind::Wildcard => {}
            PatternKind::Literal(literal) => {
                let literal = literal_expression(tree, literal, self.module.file_id)?;
                plan.conditions.push(tree.binary(BinaryOperator::StrictEqual, value, literal));
            }
            PatternKind::Array(patterns) => {
                let array = tree.identifier("Array");
                let is_array = tree.member(array, "isArray");
                let is_array = tree.call(is_array, vec![value]);
                plan.conditions.push(is_array);
                let length = tree.member(value, "length");
                let expected = tree.number(patterns.len().to_string());
                plan.conditions.push(tree.binary(BinaryOperator::StrictEqual, length, expected));
                for (index, pattern) in patterns.iter().enumerate() {
                    let index = tree.number(index.to_string());
                    let element = tree.index(value, index);
                    self.extend_pattern_plan(tree, *pattern, element, None, context, plan)?;
                }
            }
            PatternKind::Record(fields) => {
                for field in fields.iter() {
                    let field_value = tree.member(value, field.field.name.as_str());
                    self.extend_pattern_plan(
                        tree,
                        field.pattern,
                        field_value,
                        None,
                        context,
                        plan,
                    )?;
                }
            }
            PatternKind::Constructor { global, arguments } => {
                let array = tree.identifier("Array");
                let is_array = tree.member(array, "isArray");
                let is_array = tree.call(is_array, vec![value]);
                plan.conditions.push(is_array);
                let zero = tree.number("0");
                let tag = tree.index(value, zero);
                let expected = tree.string(global.item_name.as_str());
                plan.conditions.push(tree.binary(BinaryOperator::StrictEqual, tag, expected));
                for (index, pattern) in arguments.iter().enumerate() {
                    let index = tree.number((index + 1).to_string());
                    let argument = tree.index(value, index);
                    self.extend_pattern_plan(tree, *pattern, argument, None, context, plan)?;
                }
            }
        }
        Ok(())
    }

    fn bind_pattern_parameter(
        &self,
        parameter: &Parameter,
        value: ExpressionId,
        root_name: Option<&str>,
        context: &mut FunctionContext,
        plan: &mut PatternPlan,
    ) {
        if let Some(root_name) = root_name {
            context.bind_direct(parameter, root_name.to_owned());
        } else {
            let name = context.allocate(&parameter.name);
            context.bind_direct(parameter, name.clone());
            plan.bindings.push((name, value));
        }
    }

    fn allocate_pattern_argument(
        &self,
        pattern: PatternId,
        context: &mut FunctionContext,
    ) -> String {
        let preferred = pattern_parameter(&self.module.storage, pattern)
            .map(|parameter| parameter.name.as_str())
            .unwrap_or("$argument");
        context.allocate(preferred)
    }

    fn materialize_pattern_value(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        source: FunctionalExpressionId,
        value: ExpressionId,
        context: &mut FunctionContext,
    ) -> ExpressionId {
        if matches!(
            self.module.storage[source].kind,
            ExpressionKind::Literal { .. }
                | ExpressionKind::Constructor { .. }
                | ExpressionKind::Global { .. }
                | ExpressionKind::Local { .. }
        ) {
            return value;
        }
        let name = context.allocate("$scrutinee");
        writer.expression_line(format!("const {name} = "), tree, value, ";");
        tree.identifier(name)
    }

    fn materialize_value(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        value: ExpressionId,
        preferred: &str,
        context: &mut FunctionContext,
    ) -> ExpressionId {
        let name = context.allocate(preferred);
        writer.expression_line(format!("const {name} = "), tree, value, ";");
        tree.identifier(name)
    }

    fn record_update_expression(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        record: FunctionalExpressionId,
        updates: &[RecordUpdate],
        context: &mut FunctionContext,
    ) -> ModuleResult<ExpressionId> {
        let record = self.expression_value(tree, writer, record, context)?;
        let record_name = context.allocate("$record");
        writer.expression_line(format!("const {record_name} = "), tree, record, ";");
        let record = tree.identifier(record_name);
        self.record_updates(tree, writer, record, updates, context)
    }

    fn record_updates(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        record: ExpressionId,
        updates: &[RecordUpdate],
        context: &mut FunctionContext,
    ) -> ModuleResult<ExpressionId> {
        let mut properties = Vec::with_capacity(updates.len() + 1);
        properties.push(ObjectProperty::Spread(record));
        for update in updates {
            match update {
                RecordUpdate::Leaf { field, expression } => {
                    let value = self.expression_value(tree, writer, *expression, context)?;
                    let value = self.materialize_value(tree, writer, value, "$field", context);
                    properties.push(ObjectProperty::Field { name: field.name.to_string(), value });
                }
                RecordUpdate::Branch { field, updates } => {
                    let nested = tree.member(record, field.name.as_str());
                    let value = self.record_updates(tree, writer, nested, updates, context)?;
                    properties.push(ObjectProperty::Field { name: field.name.to_string(), value });
                }
            }
        }
        Ok(tree.object(properties))
    }
}

fn effect_expression(
    renderer: &mut FunctionRenderer<'_, '_, '_>,
    effect: &EffectExpression,
) -> ModuleResult<ExpressionId> {
    let generator = renderer.generator;
    let tree = &mut *renderer.tree;
    let writer = &mut *renderer.writer;
    let context = &mut *renderer.context;
    match effect {
        EffectExpression::Pure(value) => {
            let value = generator.expression_value(tree, writer, *value, context)?;
            let value_name = context.allocate("$value");
            writer.expression_line(format!("const {value_name} = "), tree, value, ";");
            let value = tree.identifier(value_name);
            Ok(tree.arrow(vec![], value))
        }
        EffectExpression::Bind { action, parameter, body } => {
            let action = generator.expression_value(tree, writer, *action, context)?;
            let action_name = context.allocate("$action");
            writer.expression_line(format!("const {action_name} = "), tree, action, ";");
            let effect_name = context.allocate("$effect");
            writer.block(format!("const {effect_name} = () => {{"), "};", |writer| {
                let parameter_name = context.allocate(&parameter.name);
                let action = tree.identifier(&action_name);
                let call = tree.call(action, vec![]);
                writer.expression_line(format!("const {parameter_name} = "), tree, call, ";");
                context.bind_direct(parameter, parameter_name);
                let continuation = generator.expression_value(tree, writer, *body, context)?;
                let call = tree.call(continuation, vec![]);
                writer.expression_line("return ", tree, call, ";");
                Ok::<(), ModuleError>(())
            })?;
            Ok(tree.identifier(effect_name))
        }
    }
}

fn local_expression(
    generator: &Generator<'_>,
    tree: &mut Tree,
    parameter: &Parameter,
    context: &FunctionContext,
) -> ModuleResult<ExpressionId> {
    match context.locals.get(&parameter.id) {
        Some(LocalBinding::Direct(name)) => Ok(tree.identifier(name)),
        Some(LocalBinding::Lazy(name)) => {
            let accessor = tree.identifier(name);
            Ok(tree.call(accessor, vec![]))
        }
        None => Err(generator
            .unsupported(UnsupportedState::MissingLocal { name: parameter.name.to_string() })),
    }
}

impl Generator<'_> {
    fn global_expression(&self, tree: &mut Tree, global: &Global) -> ModuleResult<ExpressionId> {
        let file_id = global_file(global.id);
        if file_id == self.module.file_id {
            if let Some(lazy_name) = self.lazy_global_names.get(&global.id) {
                let lazy = tree.identifier(lazy_name);
                return Ok(tree.call(lazy, vec![]));
            }
            let name = self.global_names.get(&global.id).ok_or_else(|| {
                self.unsupported(UnsupportedState::MissingGlobal {
                    name: global.item_name.to_string(),
                })
            })?;
            Ok(tree.identifier(name))
        } else {
            let namespace = self
                .external_module_namespaces
                .get(&file_id)
                .expect("invariant violated: external JavaScript global has no module namespace");
            let namespace = tree.identifier(namespace);
            Ok(tree.member(namespace, global.item_name.as_str()))
        }
    }
}

fn sorted_value_declarations<'m>(
    generator: &'m Generator<'_>,
) -> ModuleResult<Vec<&'m Declaration>> {
    let values = generator
        .module
        .declarations
        .iter()
        .filter(|declaration| matches!(declaration.kind, DeclarationKind::Value(_)))
        .collect_vec();
    let positions = values
        .iter()
        .enumerate()
        .map(|(position, declaration)| (declaration.global.id, position))
        .collect::<FxHashMap<_, _>>();
    let mut dependencies = vec![Vec::new(); values.len()];
    for (position, declaration) in values.iter().enumerate() {
        let DeclarationKind::Value(expression) = declaration.kind else {
            unreachable!("invariant violated: expected value declaration")
        };
        let mut globals = FxHashSet::default();
        collect_expression_globals(generator.module, expression, false, &mut globals);
        for global in globals {
            let Some(&dependency) = positions.get(&global) else {
                continue;
            };
            let source_is_lazy = generator.lazy_global_names.contains_key(&declaration.global.id);
            let dependency_is_lazy =
                generator.lazy_global_names.contains_key(&values[dependency].global.id);
            if source_is_lazy && dependency_is_lazy {
                continue;
            }
            dependencies[position].push(dependency);
        }
        dependencies[position].sort_unstable();
        dependencies[position].dedup();
    }

    let mut states = vec![VisitState::Unvisited; values.len()];
    let mut ordered = Vec::new();
    for position in 0..values.len() {
        visit_initializer(position, &dependencies, &mut states, &mut ordered)
            .map_err(|state| generator.unsupported(state))?;
    }
    Ok(ordered.into_iter().map(|position| values[position]).collect())
}

fn render_exports(renderer: &mut ModuleRenderer<'_, '_, '_>) {
    let generator = renderer.generator;
    let writer = &mut *renderer.writer;
    let mut rendered = false;
    for declaration in generator.module.declarations.iter() {
        if !declaration.exported {
            continue;
        }
        let local = generator.global_name(declaration.global.id);
        if local == declaration.global.item_name {
            continue;
        }
        let exported = module_export_name(&declaration.global.item_name);
        writer.line(format!("export {{ {local} as {exported} }};"));
        rendered = true;
    }
    for exports in generator.module.surface.indirect.iter() {
        let specifiers = exports.globals.iter().map(|global| module_export_name(&global.item_name));
        let dependency = generator.module_dependency(exports.file_id);
        let path = format!("../{}", module_filename(&dependency.module_name));
        writer.re_export(specifiers.collect_vec(), &path);
        rendered = true;
    }
    if rendered {
        writer.blank();
    }
}

impl Generator<'_> {
    fn declaration_is_inline_exported(&self, declaration: &Declaration) -> bool {
        let local = self.global_name(declaration.global.id);
        declaration.exported && local == declaration.global.item_name
    }

    fn global_name(&self, id: GlobalId) -> &str {
        self.global_names
            .get(&id)
            .map(String::as_str)
            .expect("invariant violated: JavaScript global has no allocated name")
    }

    fn module_dependency(&self, file_id: FileId) -> &functional::tree::ModuleDependency {
        self.module
            .dependencies
            .iter()
            .find(|dependency| dependency.file_id == file_id)
            .expect("invariant violated: referenced module has no dependency metadata")
    }

    fn unsupported(&self, state: UnsupportedState) -> ModuleError {
        ModuleError::Unsupported { file_id: self.module.file_id, state }
    }
}

fn global_file(id: GlobalId) -> FileId {
    match id {
        GlobalId::Term(file_id, _) => file_id,
        GlobalId::Instance(
            functional::tree::InstanceIdentity::Declared(file_id, _)
            | functional::tree::InstanceIdentity::Derived(file_id, _),
        ) => file_id,
    }
}

fn collect_expression_references(
    module: &FunctionalModule,
    expression: FunctionalExpressionId,
    seen: &mut FxHashSet<GlobalId>,
    globals: &mut Vec<Global>,
) {
    match &module.storage[expression].kind {
        ExpressionKind::Literal { .. }
        | ExpressionKind::Local { .. }
        | ExpressionKind::SynthesizedEvidence { .. }
        | ExpressionKind::TrivialEvidence => {}
        ExpressionKind::Constructor { global } | ExpressionKind::Global { global } => {
            if seen.insert(global.id) {
                globals.push(global.clone());
            }
        }
        ExpressionKind::Array { elements } => {
            for expression in elements.iter() {
                collect_expression_references(module, *expression, seen, globals);
            }
        }
        ExpressionKind::Record { fields } => {
            for field in fields.iter() {
                collect_expression_references(module, field.expression, seen, globals);
            }
        }
        ExpressionKind::RecordUpdate { record, updates } => {
            collect_expression_references(module, *record, seen, globals);
            collect_update_references(module, updates, seen, globals);
        }
        ExpressionKind::Project { record, .. } | ExpressionKind::Unary { value: record, .. } => {
            collect_expression_references(module, *record, seen, globals);
        }
        ExpressionKind::Binary { left, right, .. } => {
            collect_expression_references(module, *left, seen, globals);
            collect_expression_references(module, *right, seen, globals);
        }
        ExpressionKind::Abstraction { body, .. }
        | ExpressionKind::UncurriedAbstraction { body, .. } => {
            collect_expression_references(module, *body, seen, globals);
        }
        ExpressionKind::Application { function, arguments }
        | ExpressionKind::UncurriedApplication { function, arguments } => {
            collect_expression_references(module, *function, seen, globals);
            for argument in arguments.iter() {
                collect_expression_references(module, *argument, seen, globals);
            }
        }
        ExpressionKind::IfThenElse { condition, then, else_ } => {
            collect_expression_references(module, *condition, seen, globals);
            collect_expression_references(module, *then, seen, globals);
            collect_expression_references(module, *else_, seen, globals);
        }
        ExpressionKind::Case { scrutinees, alternatives } => {
            for scrutinee in scrutinees.iter() {
                collect_expression_references(module, *scrutinee, seen, globals);
            }
            for alternative in alternatives.iter() {
                for pattern in alternative.patterns.iter() {
                    collect_pattern_references(module, *pattern, seen, globals);
                }
                collect_expression_references(module, alternative.expression, seen, globals);
            }
        }
        ExpressionKind::Guarded { alternatives } => {
            collect_guarded_references(module, alternatives, seen, globals);
        }
        ExpressionKind::Let { bindings, body, .. } => {
            for binding in bindings.iter() {
                collect_expression_references(module, binding.expression, seen, globals);
            }
            collect_expression_references(module, *body, seen, globals);
        }
        ExpressionKind::LetPattern { pattern, value, body } => {
            collect_pattern_references(module, *pattern, seen, globals);
            collect_expression_references(module, *value, seen, globals);
            collect_expression_references(module, *body, seen, globals);
        }
        ExpressionKind::Effect { effect } => match effect {
            EffectExpression::Pure(value) => {
                collect_expression_references(module, *value, seen, globals);
            }
            EffectExpression::Bind { action, body, .. } => {
                collect_expression_references(module, *action, seen, globals);
                collect_expression_references(module, *body, seen, globals);
            }
        },
    }
}

fn collect_update_references(
    module: &FunctionalModule,
    updates: &[RecordUpdate],
    seen: &mut FxHashSet<GlobalId>,
    globals: &mut Vec<Global>,
) {
    for update in updates {
        match update {
            RecordUpdate::Leaf { expression, .. } => {
                collect_expression_references(module, *expression, seen, globals);
            }
            RecordUpdate::Branch { updates, .. } => {
                collect_update_references(module, updates, seen, globals);
            }
        }
    }
}

fn collect_pattern_references(
    module: &FunctionalModule,
    pattern: PatternId,
    seen: &mut FxHashSet<GlobalId>,
    globals: &mut Vec<Global>,
) {
    match &module.storage[pattern].kind {
        PatternKind::Variable(_) | PatternKind::Wildcard | PatternKind::Literal(_) => {}
        PatternKind::Named { pattern, .. } => {
            collect_pattern_references(module, *pattern, seen, globals);
        }
        PatternKind::Array(patterns) => {
            for pattern in patterns.iter() {
                collect_pattern_references(module, *pattern, seen, globals);
            }
        }
        PatternKind::Record(fields) => {
            for field in fields.iter() {
                collect_pattern_references(module, field.pattern, seen, globals);
            }
        }
        PatternKind::Constructor { global, arguments } => {
            if seen.insert(global.id) {
                globals.push(global.clone());
            }
            for pattern in arguments.iter() {
                collect_pattern_references(module, *pattern, seen, globals);
            }
        }
    }
}

fn collect_guarded_references(
    module: &FunctionalModule,
    alternatives: &[GuardedAlternative],
    seen: &mut FxHashSet<GlobalId>,
    globals: &mut Vec<Global>,
) {
    for alternative in alternatives {
        for guard in alternative.guards.iter() {
            match guard {
                Guard::Boolean(expression) => {
                    collect_expression_references(module, *expression, seen, globals);
                }
                Guard::Pattern { expression, pattern } => {
                    collect_expression_references(module, *expression, seen, globals);
                    collect_pattern_references(module, *pattern, seen, globals);
                }
            }
        }
        collect_expression_references(module, alternative.expression, seen, globals);
    }
}

fn collect_expression_globals(
    module: &FunctionalModule,
    expression: FunctionalExpressionId,
    descend_abstractions: bool,
    globals: &mut FxHashSet<GlobalId>,
) {
    match &module.storage[expression].kind {
        ExpressionKind::Global { global } | ExpressionKind::Constructor { global } => {
            globals.insert(global.id);
        }
        ExpressionKind::Abstraction { body, .. }
        | ExpressionKind::UncurriedAbstraction { body, .. } => {
            if descend_abstractions {
                collect_expression_globals(module, *body, descend_abstractions, globals);
            }
        }
        ExpressionKind::Let { recursive, bindings, body } => {
            let lazy_values = *recursive
                && !bindings
                    .iter()
                    .all(|binding| is_abstraction(&module.storage[binding.expression].kind));
            if descend_abstractions || lazy_values {
                for binding in bindings.iter() {
                    collect_expression_globals(
                        module,
                        binding.expression,
                        descend_abstractions,
                        globals,
                    );
                }
            } else if !*recursive {
                for binding in bindings.iter() {
                    collect_expression_globals(module, binding.expression, false, globals);
                }
            }
            collect_expression_globals(module, *body, descend_abstractions, globals);
        }
        _ => collect_expression_children(module, expression, descend_abstractions, globals),
    }
}

fn collect_expression_children(
    module: &FunctionalModule,
    expression: FunctionalExpressionId,
    descend_abstractions: bool,
    globals: &mut FxHashSet<GlobalId>,
) {
    let mut seen = FxHashSet::default();
    let mut references = Vec::new();
    collect_expression_references(module, expression, &mut seen, &mut references);
    if descend_abstractions {
        globals.extend(references.into_iter().map(|global| global.id));
        return;
    }
    match &module.storage[expression].kind {
        ExpressionKind::Literal { .. }
        | ExpressionKind::Constructor { .. }
        | ExpressionKind::Global { .. }
        | ExpressionKind::Local { .. }
        | ExpressionKind::SynthesizedEvidence { .. }
        | ExpressionKind::TrivialEvidence => {}
        ExpressionKind::Array { elements } => {
            for expression in elements.iter() {
                collect_expression_globals(module, *expression, false, globals);
            }
        }
        ExpressionKind::Record { fields } => {
            for field in fields.iter() {
                collect_expression_globals(module, field.expression, false, globals);
            }
        }
        ExpressionKind::RecordUpdate { record, updates } => {
            collect_expression_globals(module, *record, false, globals);
            collect_update_globals(module, updates, false, globals);
        }
        ExpressionKind::Project { record, .. } | ExpressionKind::Unary { value: record, .. } => {
            collect_expression_globals(module, *record, false, globals);
        }
        ExpressionKind::Binary { left, right, .. } => {
            collect_expression_globals(module, *left, false, globals);
            collect_expression_globals(module, *right, false, globals);
        }
        ExpressionKind::Abstraction { .. } | ExpressionKind::UncurriedAbstraction { .. } => {}
        ExpressionKind::Application { function, arguments }
        | ExpressionKind::UncurriedApplication { function, arguments } => {
            collect_expression_globals(module, *function, false, globals);
            for argument in arguments.iter() {
                collect_expression_globals(module, *argument, false, globals);
            }
        }
        ExpressionKind::IfThenElse { condition, then, else_ } => {
            collect_expression_globals(module, *condition, false, globals);
            collect_expression_globals(module, *then, false, globals);
            collect_expression_globals(module, *else_, false, globals);
        }
        ExpressionKind::Case { scrutinees, alternatives } => {
            for expression in scrutinees.iter() {
                collect_expression_globals(module, *expression, false, globals);
            }
            for alternative in alternatives.iter() {
                collect_expression_globals(module, alternative.expression, false, globals);
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
                    collect_expression_globals(module, expression, false, globals);
                }
                collect_expression_globals(module, alternative.expression, false, globals);
            }
        }
        ExpressionKind::Let { .. } => unreachable!("let expressions are handled by the caller"),
        ExpressionKind::LetPattern { value, body, .. } => {
            collect_expression_globals(module, *value, false, globals);
            collect_expression_globals(module, *body, false, globals);
        }
        ExpressionKind::Effect { effect } => match effect {
            EffectExpression::Pure(value) => {
                collect_expression_globals(module, *value, false, globals);
            }
            EffectExpression::Bind { action, body, .. } => {
                collect_expression_globals(module, *action, false, globals);
                collect_expression_globals(module, *body, false, globals);
            }
        },
    }
}

fn collect_update_globals(
    module: &FunctionalModule,
    updates: &[RecordUpdate],
    descend_abstractions: bool,
    globals: &mut FxHashSet<GlobalId>,
) {
    for update in updates {
        match update {
            RecordUpdate::Leaf { expression, .. } => {
                collect_expression_globals(module, *expression, descend_abstractions, globals);
            }
            RecordUpdate::Branch { updates, .. } => {
                collect_update_globals(module, updates, descend_abstractions, globals);
            }
        }
    }
}

fn reaches_initializer(
    current: usize,
    target: usize,
    dependencies: &[Vec<usize>],
    visited: &mut FxHashSet<usize>,
) -> bool {
    for dependency in &dependencies[current] {
        if *dependency == target {
            return true;
        }
        if visited.insert(*dependency)
            && reaches_initializer(*dependency, target, dependencies, visited)
        {
            return true;
        }
    }
    false
}
