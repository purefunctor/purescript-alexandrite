//! Conversion from the checked semantic tree into the owned functional tree.

use std::sync::Arc;

use building_types::{QueryError, QueryResult};
use checking::evidence::{
    Evidence, EvidenceBinderId, EvidenceId, EvidenceState, EvidenceVarId, InstanceCandidateOrigin,
};
use checking::tree as checking_tree;
use files::FileId;
use indexing::{
    DeriveItemId, IndexedTermItemKind, IndexedTypeItemKind, InstanceItemId, OrderedTermItemId,
    TermItemId,
};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use smol_str::{SmolStr, format_smolstr};
use thiserror::Error;

use crate::error::{ModuleError, ModuleResult, UnsupportedState};
use crate::tree::{
    BinaryOperator, Binding, CaseAlternative, Declaration, DeclarationKind, Expression,
    ExpressionId, ExpressionKind, Field, FieldIdentity, Global, GlobalId, Guard,
    GuardedAlternative, IndirectModuleExports, InstanceIdentity, Literal, LocalId, Module,
    ModuleDependency, ModuleSurface, Parameter, Pattern, PatternId, PatternKind, RecordField,
    RecordPatternField, RecordUpdate, RecursiveGroupId, ReflectableEvidence, ReflectableOrdering,
    Storage, SuperclassIdentity, SynthesizedEvidence, UnaryOperator,
};

const MAX_EVIDENCE_NAME_FRAGMENTS: usize = 4;

type ConversionResult<T> = Result<T, ConversionError>;

#[derive(Debug, Error)]
enum ConversionError {
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    Module(#[from] ModuleError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BindingSource {
    SourceBinder(lowering::BinderId),
    CheckedBinder(checking_tree::BinderId),
    Let(lowering::LetBindingNameGroupId),
    RecordPun(lowering::RecordPunId),
    Evidence(EvidenceBinderId),
}

pub fn convert_module(
    queries: &impl checking::ExternalQueries,
    file_id: FileId,
) -> QueryResult<ModuleResult<Module>> {
    match convert_module_inner(queries, file_id) {
        Ok(module) => Ok(Ok(module)),
        Err(ConversionError::Query(error)) => Err(error),
        Err(ConversionError::Module(error)) => Ok(Err(error)),
    }
}

fn convert_module_inner(
    queries: &impl checking::ExternalQueries,
    file_id: FileId,
) -> ConversionResult<Module> {
    let context = Context::new(queries, file_id)?;
    convert(context)
}

struct Context<'c, Q> {
    queries: &'c Q,
    file_id: FileId,
    module_name: SmolStr,
    indexed: Arc<indexing::IndexedModule>,
    lowered: Arc<lowering::LoweredModule>,
    checked: Arc<checking::CheckedModule>,
    recursive_groups: FxHashMap<TermItemId, RecursiveGroupId>,
    record_pun_names: FxHashMap<lowering::RecordPunId, SmolStr>,
    dependencies: FxHashMap<FileId, SmolStr>,

    parameters: FxHashMap<BindingSource, Parameter>,
    next_local: u32,
    lowering_evidence: FxHashSet<EvidenceVarId>,
    evidence_scopes: Vec<EvidenceScope>,

    storage: Storage,
}

#[derive(Default)]
struct EvidenceScope {
    constructions: FxHashMap<EvidenceKey, EvidenceConstruction>,
    bindings: Vec<EvidenceBinding>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum EvidenceKey {
    Given(EvidenceBinderId),
    Instance { origin: InstanceCandidateOrigin, subgoals: Vec<EvidenceKey> },
    Superclass { parent: Box<EvidenceKey>, superclass: checking::evidence::SuperclassId },
    Opaque(EvidenceId),
}

impl EvidenceKey {
    fn dependency_order(&self) -> usize {
        match self {
            EvidenceKey::Given(_) | EvidenceKey::Opaque(_) => 0,
            EvidenceKey::Instance { subgoals, .. } => {
                let orders = subgoals.iter().map(EvidenceKey::dependency_order);
                orders.max().unwrap_or(0).saturating_add(1)
            }
            EvidenceKey::Superclass { parent, .. } => parent.dependency_order().saturating_add(1),
        }
    }
}

#[derive(Clone)]
enum EvidenceConstruction {
    Inline { expression: ExpressionId, name: SmolStr },
    Shared(Parameter),
}

struct EvidenceBinding {
    evidence: EvidenceKey,
    binding: Binding,
}

impl<'c, Q> Context<'c, Q>
where
    Q: checking::ExternalQueries,
{
    fn new(queries: &'c Q, file_id: FileId) -> ConversionResult<Context<'c, Q>> {
        let content = queries.content(file_id)?;
        let (parsed, _) = queries.parsed(file_id)?;
        let module_name = parsed
            .module_name(&content)
            .expect("invariant violated: checked module has no source module name");
        let indexed = queries.indexed(file_id)?;
        let lowered = queries.lowered(file_id)?;
        let grouped = queries.grouped(file_id)?;
        let checked = queries.checked(file_id)?;

        let mut recursive_groups = FxHashMap::default();
        for (position, group) in grouped.term_scc.iter().enumerate() {
            if !group.is_recursive() {
                continue;
            }
            let group_id = RecursiveGroupId(position as u32);
            for &term_id in group.as_slice() {
                recursive_groups.insert(term_id, group_id);
            }
        }

        let binders = lowered.tree.iter_binder();
        let mut binders = binders.collect_vec();
        binders.sort_unstable_by_key(|(binder_id, _)| *binder_id);
        let mut record_pun_names = FxHashMap::default();
        for (_, binder) in binders {
            let lowering::BinderKind::Record { record } = binder else { continue };
            for field in record.iter() {
                let lowering::BinderRecordItem::RecordPun { id, name: Some(name) } = field else {
                    continue;
                };
                record_pun_names.insert(*id, SmolStr::clone(name));
            }
        }

        Ok(Context {
            queries,
            file_id,
            module_name,
            indexed,
            lowered,
            checked,
            recursive_groups,
            record_pun_names,
            dependencies: FxHashMap::default(),

            parameters: FxHashMap::default(),
            next_local: 0,
            lowering_evidence: FxHashSet::default(),
            evidence_scopes: Vec::new(),

            storage: Storage::default(),
        })
    }
}

fn convert(mut context: Context<'_, impl checking::ExternalQueries>) -> ConversionResult<Module> {
    let exported = context.queries.exported(context.file_id)?;
    let RuntimeExports { local, surface } = runtime_exports(&mut context, &exported)?;
    let ordered_terms = context.indexed.items.ordered_terms().iter().copied();
    let ordered_terms = ordered_terms.collect_vec();
    let mut declarations = Vec::new();
    for item_id in ordered_terms {
        let declaration = match item_id {
            OrderedTermItemId::Term(term_id) => {
                term_declaration(&mut context, term_id, local.contains(&term_id))?
            }
            OrderedTermItemId::Instance(instance_id) => {
                Some(instance_declaration(&mut context, instance_id)?)
            }
            OrderedTermItemId::Derive(derive_id) => {
                Some(derive_declaration(&mut context, derive_id)?)
            }
        };
        declarations.extend(declaration);
    }
    validate_runtime_exports(&context, &declarations, &surface)?;

    let dependencies = context.dependencies.iter().map(|(&file_id, module_name)| {
        ModuleDependency { file_id, module_name: SmolStr::clone(module_name) }
    });
    let mut dependencies = dependencies.collect_vec();
    dependencies.sort_by(|left, right| {
        left.module_name.cmp(&right.module_name).then_with(|| left.file_id.cmp(&right.file_id))
    });

    Ok(Module {
        file_id: context.file_id,
        name: context.module_name,
        dependencies: dependencies.into(),
        surface,
        declarations: declarations.into(),
        storage: context.storage,
    })
}

struct RuntimeExports {
    local: FxHashSet<TermItemId>,
    surface: ModuleSurface,
}

fn runtime_exports(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    exports: &resolving::ExportedModule,
) -> ConversionResult<RuntimeExports> {
    let mut local = FxHashSet::default();
    for &term_id in exports.local.iter() {
        let Some(global) = local_runtime_export(context, term_id)? else {
            continue;
        };
        let GlobalId::Term(file_id, term_id) = global.id else {
            unreachable!("invariant violated: resolved term export became an instance")
        };
        if file_id == context.file_id {
            local.insert(term_id);
        }
    }

    let mut indirect = Vec::new();
    for exports in exports.indirect.iter() {
        let mut globals = Vec::new();
        for &term_id in exports.terms.iter() {
            if let Some(global) = runtime_term_global(context, exports.file_id, term_id)? {
                globals.push(global);
            }
        }
        globals.sort_by(|left, right| left.item_name.cmp(&right.item_name));
        if !globals.is_empty() {
            indirect
                .push(IndirectModuleExports { file_id: exports.file_id, globals: globals.into() });
        }
    }
    indirect.sort_by(|left, right| {
        context.dependencies[&left.file_id]
            .cmp(&context.dependencies[&right.file_id])
            .then_with(|| left.file_id.cmp(&right.file_id))
    });

    let surface = ModuleSurface { indirect: indirect.into() };
    Ok(RuntimeExports { local, surface })
}

fn local_runtime_export(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    term_id: TermItemId,
) -> ConversionResult<Option<Global>> {
    let indexed = Arc::clone(&context.indexed);
    if !matches!(indexed.items[term_id].kind, IndexedTermItemKind::Operator { .. }) {
        return runtime_term_global(context, context.file_id, term_id);
    }

    let resolution = context.lowered.tree.get_term_item_kind(term_id).and_then(|kind| match kind {
        lowering::TermItemKind::Operator { resolution, .. } => *resolution,
        _ => None,
    });
    let Some((file_id, term_id)) = resolution else {
        let state = UnsupportedState::MissingRuntimeExportOperatorResolution { term_id };
        return Err(context.unsupported(state));
    };
    if file_id != context.file_id {
        return Ok(None);
    }
    runtime_term_global(context, file_id, term_id)
}

fn runtime_term_global(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    file_id: FileId,
    term_id: TermItemId,
) -> ConversionResult<Option<Global>> {
    let indexed = context.indexed_module(file_id)?;
    match &indexed.items[term_id].kind {
        IndexedTermItemKind::Value { .. } | IndexedTermItemKind::Foreign { .. } => {
            Ok(Some(context.term_global(file_id, term_id)?))
        }
        IndexedTermItemKind::Constructor { .. } => {
            if context.constructor_is_newtype(file_id, term_id)? {
                Ok(None)
            } else {
                Ok(Some(context.term_global(file_id, term_id)?))
            }
        }
        IndexedTermItemKind::ClassMember { .. } | IndexedTermItemKind::Operator { .. } => Ok(None),
    }
}

fn validate_runtime_exports(
    context: &Context<'_, impl checking::ExternalQueries>,
    declarations: &[Declaration],
    surface: &ModuleSurface,
) -> ConversionResult<()> {
    let local = declarations.iter().filter(|declaration| declaration.exported);
    let local = local.map(|declaration| &declaration.global);
    let indirect = surface.indirect.iter().flat_map(|exports| exports.globals.iter());
    let globals = local.chain(indirect);
    let mut names = FxHashMap::default();
    for global in globals {
        if let Some(existing) = names.insert(SmolStr::clone(&global.item_name), global.id)
            && existing != global.id
        {
            let state = UnsupportedState::ConflictingRuntimeExport {
                name: global.item_name.to_string(),
                existing,
                duplicate: global.id,
            };
            return Err(context.unsupported(state));
        }
    }
    Ok(())
}

fn term_declaration(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    term_id: TermItemId,
    exported: bool,
) -> ConversionResult<Option<Declaration>> {
    let indexed_module = Arc::clone(&context.indexed);
    let checked = Arc::clone(&context.checked);
    let indexed = &indexed_module.items[term_id];
    if matches!(
        indexed.kind,
        IndexedTermItemKind::ClassMember { .. } | IndexedTermItemKind::Operator { .. }
    ) {
        return Ok(None);
    }
    let declaration_id = checked
        .tree
        .lookup_term(term_id)
        .ok_or_else(|| context.unsupported(UnsupportedState::MissingTermDeclaration(term_id)))?;
    let declaration = &checked.tree[declaration_id];
    let item_name = match &indexed.name {
        Some(name) => SmolStr::clone(name),
        None => context.term_fallback(term_id),
    };
    let global = Global { id: GlobalId::Term(context.file_id, term_id), item_name };
    let recursive_group = context.recursive_groups.get(&term_id).copied();
    let kind = match &declaration.kind {
        checking_tree::TermDeclarationKind::Value(value) => {
            DeclarationKind::Value(value_declaration(context, value)?)
        }
        checking_tree::TermDeclarationKind::Foreign => DeclarationKind::Foreign,
        checking_tree::TermDeclarationKind::Constructor(constructor) => {
            if context.constructor_is_newtype(context.file_id, term_id)? {
                return Ok(None);
            }
            DeclarationKind::Constructor { arity: constructor.arguments.len() }
        }
        checking_tree::TermDeclarationKind::Instance(_) => {
            unreachable!("invariant violated: instance stored as a term declaration")
        }
    };
    Ok(Some(Declaration { global, exported, recursive_group, kind }))
}

fn instance_declaration(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    item_id: InstanceItemId,
) -> ConversionResult<Declaration> {
    let indexed = &context.indexed.items[item_id];
    let identity = InstanceIdentity::Declared(context.file_id, indexed.id);
    let declaration_id = context
        .checked
        .tree
        .lookup_instance(item_id)
        .ok_or_else(|| context.unsupported(UnsupportedState::MissingInstanceDeclaration))?;
    convert_instance_declaration(context, identity, declaration_id)
}

fn derive_declaration(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    item_id: DeriveItemId,
) -> ConversionResult<Declaration> {
    let indexed = &context.indexed.items[item_id];
    let identity = InstanceIdentity::Derived(context.file_id, indexed.id);
    let declaration_id = context
        .checked
        .tree
        .lookup_derive(item_id)
        .ok_or_else(|| context.unsupported(UnsupportedState::MissingInstanceDeclaration))?;
    convert_instance_declaration(context, identity, declaration_id)
}

fn convert_instance_declaration(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    identity: InstanceIdentity,
    declaration_id: checking_tree::TermDeclarationId,
) -> ConversionResult<Declaration> {
    let checked = Arc::clone(&context.checked);
    let declaration = &checked.tree[declaration_id];
    let checking_tree::TermDeclarationKind::Instance(instance) = &declaration.kind else {
        unreachable!("invariant violated: instance identity has non-instance declaration")
    };

    let parameters = instance.evidences.iter().map(|evidence| match evidence.evidence {
        Evidence::Given(binder) => context.evidence_parameter(binder),
        _ => Err(context.unsupported(UnsupportedState::InvalidInstancePrerequisite)),
    });
    let parameters = parameters.collect::<ConversionResult<Vec<_>>>()?;

    let body = context.evidence_scope(|context| match &instance.implementation {
        checking_tree::InstanceImplementation::Delegate { evidence, .. } => {
            evidence_variable(context, *evidence)
        }
        checking_tree::InstanceImplementation::Members(members) => {
            let mut fields = Vec::new();
            for superclass in instance.superclasses.iter() {
                let expression = evidence_variable(context, superclass.evidence)?;
                let expression = context.expression(ExpressionKind::Abstraction {
                    parameters: Arc::from([]),
                    body: expression,
                });
                let field = context.superclass_field(superclass.id)?;
                fields.push(RecordField { field, expression });
            }
            for member in members.iter() {
                let expression = member_declaration(context, member)?;
                let field = context.member_field(member.resolution)?;
                fields.push(RecordField { field, expression });
            }
            Ok(context.expression(ExpressionKind::Record { fields: fields.into() }))
        }
    })?;
    let value = context.parameter_abstraction(parameters, body);
    let item_name = context.instance_name(identity)?;
    let global = Global { id: GlobalId::Instance(identity), item_name };
    let kind = DeclarationKind::Value(value);
    Ok(Declaration { global, exported: true, recursive_group: None, kind })
}

fn member_declaration(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    member: &checking_tree::InstanceMember,
) -> ConversionResult<ExpressionId> {
    let value = checking_tree::ValueDeclaration {
        abstractions: Arc::clone(&member.abstractions),
        equations: Arc::clone(&member.equations),
    };
    value_declaration(context, &value)
}

fn value_declaration(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    value: &checking_tree::ValueDeclaration,
) -> ConversionResult<ExpressionId> {
    let body = equations(context, &value.equations)?;
    let mut evidence_parameters = Vec::new();
    for abstraction in value.abstractions.iter() {
        if let checking_tree::DeclarationAbstraction::Evidence { evidence, .. } = abstraction {
            let Evidence::Given(binder) = evidence else {
                return Err(context.unsupported(UnsupportedState::InvalidInstancePrerequisite));
            };
            evidence_parameters.push(context.evidence_parameter(*binder)?);
        }
    }
    Ok(context.parameter_abstraction(evidence_parameters, body))
}

fn equations(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    equations: &[checking_tree::Equation],
) -> ConversionResult<ExpressionId> {
    let Some(first) = equations.first() else {
        return Err(context.unsupported(UnsupportedState::MissingEquation));
    };
    if equations.len() == 1 {
        let body = context
            .evidence_scope(|context| guarded_expression(context, &first.guarded_expression))?;
        let patterns = function_patterns(context, &first.binders)?;
        return Ok(context.abstraction(patterns, body));
    }

    let arity = equations.iter().map(|equation| equation.binders.len()).max().unwrap_or(0);
    let mut parameter_patterns = Vec::with_capacity(arity);
    let mut scrutinees = Vec::with_capacity(arity);
    for position in 0..arity {
        let fallback = format_smolstr!("argument{position}");
        let type_id = equations
            .iter()
            .find_map(|equation| equation.binders.get(position))
            .map(|&binder| context.checked.tree[binder].type_id);
        let name = match type_id {
            Some(type_id) => context.type_parameter_name(type_id, fallback)?,
            None => fallback,
        };
        let parameter = context.fresh_parameter(name)?;
        let pattern = context.pattern(PatternKind::Variable(parameter.clone()));
        let scrutinee = context.expression(ExpressionKind::Local { parameter: parameter.clone() });
        parameter_patterns.push(pattern);
        scrutinees.push(scrutinee);
    }

    let body = context.evidence_scope(|context| {
        let mut alternatives = Vec::with_capacity(equations.len());
        for equation in equations {
            let mut patterns = patterns(context, &equation.binders)?;
            let supplied = patterns.len();
            while patterns.len() < arity {
                patterns.push(context.pattern(PatternKind::Wildcard));
            }
            let expression = guarded_expression(context, &equation.guarded_expression)?;
            let remaining_arguments = scrutinees.iter().skip(supplied).copied();
            let expression = context.application(expression, remaining_arguments)?;
            alternatives.push(CaseAlternative { patterns: patterns.into(), expression });
        }
        Ok(context.expression(ExpressionKind::Case {
            scrutinees: scrutinees.into(),
            alternatives: alternatives.into(),
        }))
    })?;
    Ok(context.abstraction(parameter_patterns, body))
}

fn function_patterns(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    binders: &[checking_tree::BinderId],
) -> ConversionResult<Vec<PatternId>> {
    let mut converted = Vec::with_capacity(binders.len());
    for (position, &binder) in binders.iter().enumerate() {
        let pattern = convert_pattern(context, binder)?;
        let named = matches!(
            context.storage[pattern].kind,
            PatternKind::Variable(_) | PatternKind::Named { .. }
        );
        if named {
            converted.push(pattern);
            continue;
        }
        let fallback = format_smolstr!("argument{position}");
        let type_id = context.checked.tree[binder].type_id;
        let name = context.type_parameter_name(type_id, fallback)?;
        let parameter = context.fresh_parameter(name)?;
        converted.push(context.pattern(PatternKind::Named { parameter, pattern }));
    }
    Ok(converted)
}

fn guarded_expression(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    guarded: &checking_tree::GuardedExpression,
) -> ConversionResult<ExpressionId> {
    if let [alternative] = guarded.alternatives.as_ref()
        && alternative.pattern_guards.is_empty()
    {
        return where_expression(context, &alternative.where_expression);
    }

    let alternatives =
        guarded.alternatives.iter().map(|alternative| guarded_alternative(context, alternative));
    let alternatives = alternatives.collect::<ConversionResult<Vec<_>>>()?;
    Ok(context.expression(ExpressionKind::Guarded { alternatives: alternatives.into() }))
}

fn guarded_alternative(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    alternative: &checking_tree::GuardedAlternative,
) -> ConversionResult<GuardedAlternative> {
    let mut guards = Vec::new();
    for guard in alternative.pattern_guards.iter() {
        let guard = match guard {
            checking_tree::PatternGuard::Boolean { expression } => {
                Guard::Boolean(convert_expression(context, *expression)?)
            }
            checking_tree::PatternGuard::Pattern { binder, expression } => Guard::Pattern {
                expression: convert_expression(context, *expression)?,
                pattern: convert_pattern(context, *binder)?,
            },
        };
        guards.push(guard);
    }
    let expression = where_expression(context, &alternative.where_expression)?;
    Ok(GuardedAlternative { guards: guards.into(), expression })
}

fn where_expression(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    where_expression: &checking_tree::WhereExpression,
) -> ConversionResult<ExpressionId> {
    let expression = convert_expression(context, where_expression.expression)?;
    let_bindings(context, &where_expression.bindings, expression)
}

fn let_bindings(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    bindings: &checking_tree::LetBindings,
    mut body: ExpressionId,
) -> ConversionResult<ExpressionId> {
    let checked = Arc::clone(&context.checked);
    for chunk in bindings.chunks.iter().rev() {
        match chunk {
            checking_tree::LetBindingChunk::Pattern { binder, where_expression: value, .. } => {
                let value = where_expression(context, value)?;
                let pattern = convert_pattern(context, *binder)?;
                body = context.expression(ExpressionKind::LetPattern { pattern, value, body });
            }
            checking_tree::LetBindingChunk::PatternError { source, .. } => {
                return Err(context.unsupported(UnsupportedState::PatternBindingError(*source)));
            }
            checking_tree::LetBindingChunk::Names { declarations, groups } => {
                let source_order = declarations
                    .iter()
                    .enumerate()
                    .map(|(position, &declaration)| (declaration, position));
                let source_order = source_order.collect::<FxHashMap<_, _>>();
                for group in groups.iter().rev() {
                    let mut converted = Vec::new();
                    for &source in group.as_slice() {
                        let Some(declaration_id) = checked.tree.lookup_let(source) else {
                            return Err(context
                                .unsupported(UnsupportedState::MissingLocalDeclaration(source)));
                        };
                        let declaration = &checked.tree[declaration_id];
                        let parameter = context.local_parameter(source)?;
                        let expression = value_declaration(context, &declaration.value)?;
                        let source_order = source_order[&declaration_id];
                        converted.push(Binding { parameter, expression, source_order });
                    }
                    body = context.expression(ExpressionKind::Let {
                        recursive: group.is_recursive(),
                        bindings: converted.into(),
                        body,
                    });
                }
            }
        }
    }
    Ok(body)
}

fn convert_expression(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    expression_id: checking_tree::ExpressionId,
) -> ConversionResult<ExpressionId> {
    let checked = Arc::clone(&context.checked);
    let expression = &checked.tree[expression_id];
    let kind = match &expression.kind {
        checking_tree::ExpressionKind::String { value, .. } => {
            ExpressionKind::Literal { literal: Literal::String(value.clone()) }
        }
        checking_tree::ExpressionKind::Char { value } => {
            ExpressionKind::Literal { literal: Literal::Char(*value) }
        }
        checking_tree::ExpressionKind::Boolean { value } => {
            ExpressionKind::Literal { literal: Literal::Boolean(*value) }
        }
        checking_tree::ExpressionKind::Integer { value } => {
            ExpressionKind::Literal { literal: Literal::Integer(*value) }
        }
        checking_tree::ExpressionKind::Number { value } => {
            ExpressionKind::Literal { literal: Literal::Number(value.clone()) }
        }
        checking_tree::ExpressionKind::Array { elements } => {
            let elements = expressions(context, elements)?;
            ExpressionKind::Array { elements: elements.into() }
        }
        checking_tree::ExpressionKind::Record { fields } => {
            let mut converted = Vec::new();
            for field in fields.iter() {
                let (label, expression) = match field {
                    checking_tree::RecordExpressionField::Field { label, expression }
                    | checking_tree::RecordExpressionField::Pun { label, expression, .. } => {
                        (label, expression)
                    }
                };
                let field = context.label_field(label.clone());
                let expression = convert_expression(context, *expression)?;
                converted.push(RecordField { field, expression });
            }
            ExpressionKind::Record { fields: converted.into() }
        }
        checking_tree::ExpressionKind::RecordAccess { record, labels } => {
            let mut record = convert_expression(context, *record)?;
            for label in labels.iter() {
                let field = context.label_field(label.clone());
                record = context.expression(ExpressionKind::Project { record, field });
            }
            return Ok(record);
        }
        checking_tree::ExpressionKind::RecordUpdate { record, updates } => {
            let record = convert_expression(context, *record)?;
            let updates = record_updates(context, updates)?;
            ExpressionKind::RecordUpdate { record, updates: updates.into() }
        }
        checking_tree::ExpressionKind::Constructor { resolution } => {
            let &(file_id, term_id) = resolution;
            if context.constructor_is_newtype(file_id, term_id)? {
                let parameter = context.fresh_parameter("value".into())?;
                let body =
                    context.expression(ExpressionKind::Local { parameter: parameter.clone() });
                return Ok(context.parameter_abstraction([parameter], body));
            }
            ExpressionKind::Constructor { global: context.term_global(file_id, term_id)? }
        }
        checking_tree::ExpressionKind::Variable { resolution }
        | checking_tree::ExpressionKind::RecordPun { resolution, .. } => {
            return variable(context, *resolution);
        }
        checking_tree::ExpressionKind::Section { binder } => {
            let parameter = context.checked_binder_parameter(*binder)?;
            ExpressionKind::Local { parameter }
        }
        checking_tree::ExpressionKind::TermApplication { function, argument } => {
            if let checking_tree::ExpressionKind::Constructor { resolution } =
                &checked.tree[*function].kind
            {
                let &(file_id, term_id) = resolution;
                if context.constructor_is_newtype(file_id, term_id)? {
                    return convert_expression(context, *argument);
                }
            }
            let function = convert_expression(context, *function)?;
            let argument = convert_expression(context, *argument)?;
            return context.application(function, [argument]);
        }
        checking_tree::ExpressionKind::EvidenceApplication { function, evidence, .. } => {
            let evidence = evidence_variable(context, *evidence)?;
            if let Some(resolution) = class_member_resolution(context, *function)? {
                let field = context.member_field(resolution)?;
                return Ok(context.expression(ExpressionKind::Project { record: evidence, field }));
            }
            let function = convert_expression(context, *function)?;
            return context.application(function, [evidence]);
        }
        checking_tree::ExpressionKind::EvidenceAbstraction { binder, expression } => {
            let parameter = context.evidence_parameter(*binder)?;
            let body =
                context.evidence_scope(|context| convert_expression(context, *expression))?;
            return Ok(context.parameter_abstraction([parameter], body));
        }
        checking_tree::ExpressionKind::Lambda { binders, expression } => {
            let parameters = function_patterns(context, binders)?;
            let body =
                context.evidence_scope(|context| convert_expression(context, *expression))?;
            return Ok(context.abstraction(parameters, body));
        }
        checking_tree::ExpressionKind::IfThenElse { condition, then, else_ } => {
            ExpressionKind::IfThenElse {
                condition: convert_expression(context, *condition)?,
                then: convert_expression(context, *then)?,
                else_: convert_expression(context, *else_)?,
            }
        }
        checking_tree::ExpressionKind::Case { scrutinees, alternatives } => {
            let scrutinees = expressions(context, scrutinees)?;
            let alternatives =
                alternatives.iter().map(|alternative| case_alternative(context, alternative));
            let alternatives = alternatives.collect::<ConversionResult<Vec<_>>>()?;
            ExpressionKind::Case {
                scrutinees: scrutinees.into(),
                alternatives: alternatives.into(),
            }
        }
        checking_tree::ExpressionKind::Let { bindings, expression } => {
            let body = convert_expression(context, *expression)?;
            return let_bindings(context, bindings, body);
        }
        checking_tree::ExpressionKind::Error => {
            return Err(context.unsupported(UnsupportedState::ExpressionError(expression_id)));
        }
    };
    Ok(context.expression(kind))
}

fn class_member_resolution(
    context: &Context<'_, impl checking::ExternalQueries>,
    expression_id: checking_tree::ExpressionId,
) -> QueryResult<Option<(FileId, TermItemId)>> {
    let expression = &context.checked.tree[expression_id];
    let resolution = match expression.kind {
        checking_tree::ExpressionKind::Variable { resolution }
        | checking_tree::ExpressionKind::RecordPun { resolution, .. } => resolution,
        _ => return Ok(None),
    };
    let checking_tree::VariableResolution::Source(resolution) = resolution else {
        return Ok(None);
    };
    let lowering::TermVariableResolution::Reference(file_id, term_id) = resolution else {
        return Ok(None);
    };
    let indexed = if file_id == context.file_id {
        Arc::clone(&context.indexed)
    } else {
        context.queries.indexed(file_id)?
    };
    let is_class_member =
        matches!(indexed.items[term_id].kind, IndexedTermItemKind::ClassMember { .. });
    Ok(is_class_member.then_some((file_id, term_id)))
}

fn case_alternative(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    alternative: &checking_tree::CaseAlternative,
) -> ConversionResult<CaseAlternative> {
    let patterns = patterns(context, &alternative.binders)?;
    let expression = guarded_expression(context, &alternative.guarded_expression)?;
    Ok(CaseAlternative { patterns: patterns.into(), expression })
}

fn expressions(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    expressions: &[checking_tree::ExpressionId],
) -> ConversionResult<Vec<ExpressionId>> {
    let expressions = expressions.iter().map(|&expression| convert_expression(context, expression));
    expressions.collect::<ConversionResult<Vec<_>>>()
}

fn record_updates(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    updates: &[checking_tree::RecordExpressionUpdate],
) -> ConversionResult<Vec<RecordUpdate>> {
    let mut converted = Vec::new();
    for update in updates {
        let update = match update {
            checking_tree::RecordExpressionUpdate::Leaf { label, expression } => {
                RecordUpdate::Leaf {
                    field: context.label_field(label.clone()),
                    expression: convert_expression(context, *expression)?,
                }
            }
            checking_tree::RecordExpressionUpdate::Branch { label, updates } => {
                let updates = record_updates(context, updates)?;
                RecordUpdate::Branch {
                    field: context.label_field(label.clone()),
                    updates: updates.into(),
                }
            }
            checking_tree::RecordExpressionUpdate::Error => {
                return Err(context.unsupported(UnsupportedState::RecordUpdateError));
            }
        };
        converted.push(update);
    }
    Ok(converted)
}

fn patterns(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    binders: &[checking_tree::BinderId],
) -> ConversionResult<Vec<PatternId>> {
    let patterns = binders.iter().map(|&binder| convert_pattern(context, binder));
    patterns.collect::<ConversionResult<Vec<_>>>()
}

fn convert_pattern(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    binder_id: checking_tree::BinderId,
) -> ConversionResult<PatternId> {
    let checked = Arc::clone(&context.checked);
    let binder = &checked.tree[binder_id];
    let kind = match &binder.kind {
        checking_tree::BinderKind::Typed { binder, .. } => {
            return convert_pattern(context, *binder);
        }
        checking_tree::BinderKind::Integer { value } => {
            PatternKind::Literal(Literal::Integer(*value))
        }
        checking_tree::BinderKind::Number { negative, value } => {
            let value = if *negative { format_smolstr!("-{value}") } else { value.clone() };
            PatternKind::Literal(Literal::Number(value))
        }
        checking_tree::BinderKind::Variable => {
            PatternKind::Variable(context.checked_binder_parameter(binder_id)?)
        }
        checking_tree::BinderKind::Named { name, binder } => PatternKind::Named {
            parameter: context.parameter(context.binding_source(binder_id), name.clone())?,
            pattern: convert_pattern(context, *binder)?,
        },
        checking_tree::BinderKind::Wildcard => PatternKind::Wildcard,
        checking_tree::BinderKind::String { value } => {
            PatternKind::Literal(Literal::String(value.clone()))
        }
        checking_tree::BinderKind::Char { value } => PatternKind::Literal(Literal::Char(*value)),
        checking_tree::BinderKind::Boolean { value } => {
            PatternKind::Literal(Literal::Boolean(*value))
        }
        checking_tree::BinderKind::Array { elements } => {
            PatternKind::Array(patterns(context, elements)?.into())
        }
        checking_tree::BinderKind::Record { fields } => {
            let converted =
                fields.iter().map(|field| record_pattern_field(context, binder_id, field));
            let converted = converted.collect::<ConversionResult<Vec<_>>>()?;
            PatternKind::Record(converted.into())
        }
        checking_tree::BinderKind::Constructor { resolution, arguments } => {
            let &(file_id, term_id) = resolution;
            if context.constructor_is_newtype(file_id, term_id)? {
                let [argument] = arguments.as_ref() else {
                    return Err(context.unsupported(UnsupportedState::BinderError(binder_id)));
                };
                return convert_pattern(context, *argument);
            }
            PatternKind::Constructor {
                global: context.term_global(file_id, term_id)?,
                arguments: patterns(context, arguments)?.into(),
            }
        }
        checking_tree::BinderKind::Error => {
            return Err(context.unsupported(UnsupportedState::BinderError(binder_id)));
        }
    };
    Ok(context.pattern(kind))
}

fn record_pattern_field(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    binder_id: checking_tree::BinderId,
    field: &checking_tree::RecordBinderField,
) -> ConversionResult<RecordPatternField> {
    let (label, pattern) = match field {
        checking_tree::RecordBinderField::Field { label, binder } => {
            (label.clone(), convert_pattern(context, *binder)?)
        }
        checking_tree::RecordBinderField::Pun { label } => {
            let Some(source) = context.record_pun_source(binder_id, label) else {
                return Err(context.unsupported(UnsupportedState::BinderError(binder_id)));
            };
            let parameter = context.record_pun_parameter(source, label.clone())?;
            (label.clone(), context.pattern(PatternKind::Variable(parameter)))
        }
    };
    Ok(RecordPatternField { field: context.label_field(label), pattern })
}

fn variable(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    resolution: checking_tree::VariableResolution,
) -> ConversionResult<ExpressionId> {
    match resolution {
        checking_tree::VariableResolution::Generated(binder) => {
            let parameter = context.checked_binder_parameter(binder)?;
            Ok(context.expression(ExpressionKind::Local { parameter }))
        }
        checking_tree::VariableResolution::Source(resolution) => match resolution {
            lowering::TermVariableResolution::Binder(binder) => {
                let name = context.source_binder_name(binder);
                let parameter = context.parameter(BindingSource::SourceBinder(binder), name)?;
                Ok(context.expression(ExpressionKind::Local { parameter }))
            }
            lowering::TermVariableResolution::Let(source) => {
                let parameter = context.local_parameter(source)?;
                Ok(context.expression(ExpressionKind::Local { parameter }))
            }
            lowering::TermVariableResolution::RecordPun(source) => {
                let name = context.record_pun_name(source);
                let parameter = context.record_pun_parameter(source, name)?;
                Ok(context.expression(ExpressionKind::Local { parameter }))
            }
            lowering::TermVariableResolution::Reference(file_id, term_id) => {
                let global = context.term_global(file_id, term_id)?;
                Ok(context.expression(ExpressionKind::Global { global }))
            }
        },
    }
}

fn evidence_variable(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    variable: EvidenceVarId,
) -> ConversionResult<ExpressionId> {
    if !context.lowering_evidence.insert(variable) {
        return Err(context.unsupported(UnsupportedState::CyclicEvidence(variable)));
    }
    let result = match context.checked.evidence[variable].state {
        EvidenceState::Unsolved => {
            Err(context.unsupported(UnsupportedState::UnsolvedEvidence(variable)))
        }
        EvidenceState::Solved(evidence) => convert_evidence(context, evidence),
        EvidenceState::Error => Err(context.unsupported(UnsupportedState::EvidenceError(variable))),
    };
    context.lowering_evidence.remove(&variable);
    result
}

fn convert_evidence(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    evidence_id: EvidenceId,
) -> ConversionResult<ExpressionId> {
    let checked = Arc::clone(&context.checked);
    match &checked.evidence[evidence_id] {
        Evidence::Variable(variable) => evidence_variable(context, *variable),
        Evidence::Given(binder) => {
            let parameter = context.evidence_parameter(*binder)?;
            Ok(context.expression(ExpressionKind::Local { parameter }))
        }
        Evidence::Instance { origin, subgoals } => {
            let evidence = context.evidence_key(evidence_id);
            if let Some(expression) = context.shared_evidence(&evidence)? {
                return Ok(expression);
            }
            let global = context.instance_global(*origin)?;
            let name = format_smolstr!("{}Dict", global.item_name);
            let function = context.expression(ExpressionKind::Global { global });
            let arguments = subgoals.iter().map(|&subgoal| evidence_variable(context, subgoal));
            let arguments = arguments.collect::<ConversionResult<Vec<_>>>()?;
            let construction = context.application(function, arguments)?;
            if subgoals.is_empty() {
                Ok(construction)
            } else {
                context.record_evidence(evidence, construction, name)
            }
        }
        Evidence::Superclass { parent, superclass } => {
            let evidence = context.evidence_key(evidence_id);
            if let Some(expression) = context.shared_evidence(&evidence)? {
                return Ok(expression);
            }
            let record = convert_evidence(context, *parent)?;
            let field = context.superclass_field(*superclass)?;
            let name = format_smolstr!("{}Dict", field.name);
            let accessor = context.expression(ExpressionKind::Project { record, field });
            let construction = context.expression(ExpressionKind::Application {
                function: accessor,
                arguments: Arc::from([]),
            });
            context.record_evidence(evidence, construction, name)
        }
        Evidence::Trivial => Ok(context.expression(ExpressionKind::TrivialEvidence)),
        Evidence::Synthesized(evidence) => {
            let evidence = synthesized_evidence(context, evidence);
            Ok(context.expression(ExpressionKind::SynthesizedEvidence { evidence }))
        }
    }
}

fn synthesized_evidence(
    _context: &Context<'_, impl checking::ExternalQueries>,
    evidence: &checking::evidence::SynthesizedEvidence,
) -> SynthesizedEvidence {
    match evidence {
        checking::evidence::SynthesizedEvidence::IsSymbol(symbol) => {
            SynthesizedEvidence::IsSymbol(symbol.clone())
        }
        checking::evidence::SynthesizedEvidence::Reflectable(reflectable) => {
            let reflectable = match reflectable {
                checking::evidence::ReflectableEvidence::Integer(value) => {
                    ReflectableEvidence::Integer(*value)
                }
                checking::evidence::ReflectableEvidence::String(value) => {
                    ReflectableEvidence::String(value.clone())
                }
                checking::evidence::ReflectableEvidence::Boolean(value) => {
                    ReflectableEvidence::Boolean(*value)
                }
                checking::evidence::ReflectableEvidence::Ordering(ordering) => {
                    let ordering = match ordering {
                        checking::evidence::ReflectableOrdering::Less => ReflectableOrdering::Less,
                        checking::evidence::ReflectableOrdering::Equal => {
                            ReflectableOrdering::Equal
                        }
                        checking::evidence::ReflectableOrdering::Greater => {
                            ReflectableOrdering::Greater
                        }
                    };
                    ReflectableEvidence::Ordering(ordering)
                }
            };
            SynthesizedEvidence::Reflectable(reflectable)
        }
    }
}

fn order_evidence_bindings(mut bindings: Vec<EvidenceBinding>) -> Vec<Binding> {
    // Every prerequisite precedes the evidence that consumes it, so this order places inputs
    // before the non-recursive bindings that reference them.
    bindings.sort_by_key(|binding| binding.evidence.dependency_order());
    let bindings = bindings.into_iter().map(|binding| binding.binding);
    bindings.collect()
}

impl<'c, Q> Context<'c, Q>
where
    Q: checking::ExternalQueries,
{
    fn evidence_scope(
        &mut self,
        convert: impl FnOnce(&mut Context<'c, Q>) -> ConversionResult<ExpressionId>,
    ) -> ConversionResult<ExpressionId> {
        self.evidence_scopes.push(EvidenceScope::default());
        let result = convert(self);
        let scope = self
            .evidence_scopes
            .pop()
            .expect("invariant violated: evidence scope disappeared during conversion");
        let body = result?;
        if scope.bindings.is_empty() {
            return Ok(body);
        }
        let bindings = order_evidence_bindings(scope.bindings);
        Ok(self.expression(ExpressionKind::Let {
            recursive: false,
            bindings: bindings.into(),
            body,
        }))
    }

    fn shared_evidence(
        &mut self,
        evidence: &EvidenceKey,
    ) -> ConversionResult<Option<ExpressionId>> {
        let Some(scope) = self.evidence_scopes.last() else { return Ok(None) };
        let Some(construction) = scope.constructions.get(evidence).cloned() else {
            return Ok(None);
        };
        let parameter = match construction {
            EvidenceConstruction::Shared(parameter) => parameter,
            EvidenceConstruction::Inline { expression, name } => {
                // The first occurrence stays inline until repetition justifies a binding. Replacing
                // its arena node with a local updates that occurrence without a separate tree pass.
                let parameter = self.fresh_parameter(name)?;
                let local = ExpressionKind::Local { parameter: parameter.clone() };
                let construction = self.storage.replace_expression_kind(expression, local);
                let construction = self.expression(construction);
                let scope = self
                    .evidence_scopes
                    .last_mut()
                    .expect("invariant violated: evidence scope disappeared during conversion");
                scope
                    .constructions
                    .insert(evidence.clone(), EvidenceConstruction::Shared(parameter.clone()));
                scope.bindings.push(EvidenceBinding {
                    evidence: evidence.clone(),
                    binding: Binding {
                        parameter: parameter.clone(),
                        expression: construction,
                        source_order: 0,
                    },
                });
                parameter
            }
        };
        Ok(Some(self.expression(ExpressionKind::Local { parameter })))
    }

    fn record_evidence(
        &mut self,
        evidence: EvidenceKey,
        construction: ExpressionId,
        name: SmolStr,
    ) -> ConversionResult<ExpressionId> {
        let Some(scope) = self.evidence_scopes.last_mut() else { return Ok(construction) };
        scope
            .constructions
            .insert(evidence, EvidenceConstruction::Inline { expression: construction, name });
        Ok(construction)
    }

    fn evidence_key(&self, evidence: EvidenceId) -> EvidenceKey {
        self.evidence_key_inner(evidence, &mut FxHashSet::default())
            .unwrap_or(EvidenceKey::Opaque(evidence))
    }

    fn evidence_key_inner(
        &self,
        evidence: EvidenceId,
        visiting: &mut FxHashSet<EvidenceId>,
    ) -> Option<EvidenceKey> {
        if !visiting.insert(evidence) {
            return None;
        }
        let key = match &self.checked.evidence[evidence] {
            Evidence::Variable(variable) => {
                let EvidenceState::Solved(evidence) = self.checked.evidence[*variable].state else {
                    return None;
                };
                self.evidence_key_inner(evidence, visiting)?
            }
            Evidence::Given(binder) => EvidenceKey::Given(*binder),
            Evidence::Instance { origin, subgoals } => {
                let mut keys = Vec::with_capacity(subgoals.len());
                for &subgoal in subgoals {
                    let EvidenceState::Solved(evidence) = self.checked.evidence[subgoal].state
                    else {
                        return None;
                    };
                    keys.push(self.evidence_key_inner(evidence, visiting)?);
                }
                EvidenceKey::Instance { origin: *origin, subgoals: keys }
            }
            Evidence::Superclass { parent, superclass } => EvidenceKey::Superclass {
                parent: Box::new(self.evidence_key_inner(*parent, visiting)?),
                superclass: *superclass,
            },
            Evidence::Trivial | Evidence::Synthesized(_) => EvidenceKey::Opaque(evidence),
        };
        visiting.remove(&evidence);
        Some(key)
    }

    fn checked_binder_parameter(
        &mut self,
        binder_id: checking_tree::BinderId,
    ) -> ConversionResult<Parameter> {
        let source = self.binding_source(binder_id);
        let name = self.checked_binder_name(binder_id);
        self.parameter(source, name)
    }

    fn binding_source(&self, binder_id: checking_tree::BinderId) -> BindingSource {
        match self.checked.tree[binder_id].source {
            checking_tree::BinderSource::Binder(source) => BindingSource::SourceBinder(source),
            _ => BindingSource::CheckedBinder(binder_id),
        }
    }

    fn checked_binder_name(&self, binder_id: checking_tree::BinderId) -> SmolStr {
        let binder = &self.checked.tree[binder_id];
        match &binder.kind {
            checking_tree::BinderKind::Named { name, .. } => name.clone(),
            _ => match binder.source {
                checking_tree::BinderSource::Binder(source) => self.source_binder_name(source),
                checking_tree::BinderSource::Generated { name, .. } => {
                    self.queries.lookup_smol_str(name)
                }
                checking_tree::BinderSource::Section(source) => {
                    format_smolstr!("section{}", source.into_raw().get())
                }
                _ => format_smolstr!("local{}", binder_id.into_raw().into_u32()),
            },
        }
    }

    fn source_binder_name(&self, binder: lowering::BinderId) -> SmolStr {
        match self.lowered.tree.get_binder_kind(binder) {
            Some(lowering::BinderKind::Variable { variable: Some(name) }) => name.clone(),
            Some(lowering::BinderKind::Named { named: Some(name), .. }) => name.clone(),
            _ => format_smolstr!("binder{}", binder.into_raw().get()),
        }
    }

    fn local_parameter(
        &mut self,
        source: lowering::LetBindingNameGroupId,
    ) -> ConversionResult<Parameter> {
        let group = self.lowered.tree.get_let_binding_group(source);
        let name = group
            .name
            .clone()
            .unwrap_or_else(|| format_smolstr!("binding{}", source.into_raw().into_u32()));
        self.parameter(BindingSource::Let(source), name)
    }

    fn record_pun_parameter(
        &mut self,
        source: lowering::RecordPunId,
        name: SmolStr,
    ) -> ConversionResult<Parameter> {
        self.parameter(BindingSource::RecordPun(source), name)
    }

    fn evidence_parameter(&mut self, binder: EvidenceBinderId) -> ConversionResult<Parameter> {
        let constraint = self.checked.evidence[binder].constraint;
        let name = self.evidence_parameter_name(constraint)?;
        self.parameter(BindingSource::Evidence(binder), name)
    }

    fn evidence_parameter_name(&self, constraint: checking::TypeId) -> QueryResult<SmolStr> {
        let mut current = constraint;
        let mut arguments = vec![];
        loop {
            match self.queries.lookup_type(current) {
                checking::Type::Application(function, argument) => {
                    arguments.push(argument);
                    current = function;
                }
                checking::Type::KindApplication(function, _)
                | checking::Type::Kinded(function, _) => current = function,
                checking::Type::Constructor(file_id, type_id) => {
                    let Some(class_name) = self.type_item_name(file_id, type_id)? else {
                        return Ok(SmolStr::new("dictionary"));
                    };
                    let Some(mut name) = lowercase_initial(&class_name) else {
                        return Ok(SmolStr::new("dictionary"));
                    };
                    let mut fragments = 0;
                    for argument in arguments.into_iter().rev() {
                        self.append_evidence_type_name(&mut name, argument, &mut fragments)?;
                    }
                    name.push_str("Dict");
                    return Ok(SmolStr::new(name));
                }
                _ => return Ok(SmolStr::new("dictionary")),
            }
        }
    }

    fn append_evidence_type_name(
        &self,
        name: &mut String,
        type_id: checking::TypeId,
        fragments: &mut usize,
    ) -> QueryResult<()> {
        if *fragments >= MAX_EVIDENCE_NAME_FRAGMENTS {
            return Ok(());
        }
        match self.queries.lookup_type(type_id) {
            checking::Type::Application(function, argument) => {
                self.append_evidence_type_name(name, function, fragments)?;
                self.append_evidence_type_name(name, argument, fragments)?;
            }
            checking::Type::KindApplication(function, _) | checking::Type::Kinded(function, _) => {
                self.append_evidence_type_name(name, function, fragments)?;
            }
            checking::Type::Forall(_, inner) | checking::Type::Constrained(_, inner) => {
                self.append_evidence_type_name(name, inner, fragments)?;
            }
            checking::Type::Function(_, _) => {
                append_evidence_name_fragment(name, "Function", fragments)
            }
            checking::Type::Constructor(file_id, type_id) => {
                if let Some(type_name) = self.type_item_name(file_id, type_id)? {
                    append_evidence_name_fragment(name, &type_name, fragments);
                }
            }
            checking::Type::Row(_) => append_evidence_name_fragment(name, "Row", fragments),
            checking::Type::Rigid(rigid, _, _) => {
                if let Some(type_name) = self.rigid_type_name(rigid)? {
                    append_evidence_name_fragment(name, &type_name, fragments);
                }
            }
            checking::Type::Integer(_)
            | checking::Type::String(..)
            | checking::Type::Unification(_)
            | checking::Type::Free(_)
            | checking::Type::Unknown(_) => {}
        }
        Ok(())
    }

    fn type_parameter_name(
        &self,
        type_id: checking::TypeId,
        fallback: SmolStr,
    ) -> QueryResult<SmolStr> {
        let name = match self.queries.lookup_type(type_id) {
            checking::Type::Application(function, _)
            | checking::Type::KindApplication(function, _)
            | checking::Type::Kinded(function, _)
            | checking::Type::Forall(_, function)
            | checking::Type::Constrained(_, function) => {
                return self.type_parameter_name(function, fallback);
            }
            checking::Type::Function(_, _) => Some(SmolStr::new("function")),
            checking::Type::Constructor(file_id, type_id) => self
                .type_item_name(file_id, type_id)?
                .and_then(|name| lowercase_initial(&name).map(SmolStr::new)),
            checking::Type::Row(_) => Some(SmolStr::new("record")),
            checking::Type::Rigid(rigid, _, _) => self.rigid_type_name(rigid)?,
            checking::Type::Integer(_)
            | checking::Type::String(..)
            | checking::Type::Unification(_)
            | checking::Type::Free(_)
            | checking::Type::Unknown(_) => None,
        };
        let name = name.unwrap_or(fallback);
        Ok(format_smolstr!("${name}"))
    }

    fn rigid_type_name(&self, rigid: checking::core::Name) -> QueryResult<Option<SmolStr>> {
        let checked = if rigid.file == self.file_id {
            Arc::clone(&self.checked)
        } else {
            self.queries.checked(rigid.file)?
        };
        Ok(checked.lookup_name(rigid).map(|name| self.queries.lookup_smol_str(name)))
    }

    fn parameter(&mut self, source: BindingSource, name: SmolStr) -> ConversionResult<Parameter> {
        if let Some(parameter) = self.parameters.get(&source) {
            return Ok(parameter.clone());
        }
        let parameter = self.fresh_parameter(name)?;
        self.parameters.insert(source, parameter.clone());
        Ok(parameter)
    }

    fn fresh_parameter(&mut self, name: SmolStr) -> ConversionResult<Parameter> {
        let id = LocalId(self.next_local);
        self.next_local = self
            .next_local
            .checked_add(1)
            .ok_or_else(|| self.unsupported(UnsupportedState::LocalIdentityOverflow))?;
        Ok(Parameter { id, name })
    }

    fn parameter_abstraction(
        &mut self,
        parameters: impl IntoIterator<Item = Parameter>,
        body: ExpressionId,
    ) -> ExpressionId {
        let patterns =
            parameters.into_iter().map(|parameter| self.pattern(PatternKind::Variable(parameter)));
        let patterns = patterns.collect_vec();
        self.abstraction(patterns, body)
    }

    fn abstraction(&mut self, parameters: Vec<PatternId>, body: ExpressionId) -> ExpressionId {
        if parameters.is_empty() {
            body
        } else {
            self.expression(ExpressionKind::Abstraction { parameters: parameters.into(), body })
        }
    }

    fn application(
        &mut self,
        function: ExpressionId,
        arguments: impl IntoIterator<Item = ExpressionId>,
    ) -> ConversionResult<ExpressionId> {
        let arguments = arguments.into_iter();
        let arguments = arguments.collect_vec();
        if arguments.is_empty() {
            return Ok(function);
        }
        let (known_function, known_arguments) = self.application_spine(function, &arguments);
        if let Some(arity) =
            self.known_numbered_term_arity(known_function, "Data.Function.Uncurried", "mkFn")?
            && arity >= 1
            && let [function] = known_arguments.as_slice()
            && let Some(function) = self.uncurry_abstraction(*function, arity)
        {
            return Ok(function);
        }
        if let Some(arity) =
            self.known_numbered_term_arity(known_function, "Data.Function.Uncurried", "runFn")?
            && let Some((function, arguments)) = known_arguments.split_first()
            && arguments.len() == arity
        {
            return Ok(self.expression(ExpressionKind::UncurriedApplication {
                function: *function,
                arguments: arguments.into(),
            }));
        }
        if self.known_term(known_function, "Data.Function", "apply")?
            && let [function, argument] = known_arguments.as_slice()
        {
            return self.application(*function, [*argument]);
        }
        if self.known_term(known_function, "Data.Function", "applyFlipped")?
            && let [argument, function] = known_arguments.as_slice()
        {
            return self.flipped_application(*argument, *function);
        }
        if self.known_instance_member(
            known_function,
            "Control.Category",
            "identity",
            "categoryFn",
        )? && let [argument] = known_arguments.as_slice()
        {
            return Ok(*argument);
        }
        if self.known_term(known_function, "Unsafe.Coerce", "unsafeCoerce")?
            && let [argument] = known_arguments.as_slice()
        {
            return Ok(*argument);
        }
        if self.known_instance_member(
            known_function,
            "Data.HeytingAlgebra",
            "not",
            "heytingAlgebraBoolean",
        )? && let [value] = known_arguments.as_slice()
        {
            return Ok(self.expression(ExpressionKind::Unary {
                operator: UnaryOperator::BooleanNot,
                value: *value,
            }));
        }
        if self.known_instance_member(known_function, "Data.Ring", "negate", "ringInt")?
            && let [value] = known_arguments.as_slice()
        {
            return Ok(self.expression(ExpressionKind::Unary {
                operator: UnaryOperator::IntegerNegate,
                value: *value,
            }));
        }
        let binary_operator =
            if self.known_instance_member(known_function, "Data.Semiring", "add", "semiringInt")? {
                Some(BinaryOperator::IntegerAdd)
            } else if self.known_instance_member(known_function, "Data.Ring", "sub", "ringInt")? {
                Some(BinaryOperator::IntegerSubtract)
            } else if self.known_instance_member(
                known_function,
                "Data.Semiring",
                "mul",
                "semiringInt",
            )? {
                Some(BinaryOperator::IntegerMultiply)
            } else {
                None
            };
        if let (Some(operator), [left, right]) = (binary_operator, known_arguments.as_slice()) {
            return Ok(self.expression(ExpressionKind::Binary {
                operator,
                left: *left,
                right: *right,
            }));
        }
        Ok(self.expression(ExpressionKind::Application { function, arguments: arguments.into() }))
    }

    fn uncurry_abstraction(
        &mut self,
        mut expression: ExpressionId,
        arity: usize,
    ) -> Option<ExpressionId> {
        let mut parameters = Vec::with_capacity(arity);
        while parameters.len() < arity {
            let ExpressionKind::Abstraction { parameters: abstraction, body } =
                &self.storage[expression].kind
            else {
                return None;
            };
            let abstraction = abstraction.to_vec();
            let body = *body;
            let remaining = arity - parameters.len();
            let split = abstraction.len().min(remaining);
            parameters.extend_from_slice(&abstraction[..split]);
            expression = if split == abstraction.len() {
                body
            } else {
                self.expression(ExpressionKind::Abstraction {
                    parameters: abstraction[split..].into(),
                    body,
                })
            };
        }
        Some(self.expression(ExpressionKind::UncurriedAbstraction {
            parameters: parameters.into(),
            body: expression,
        }))
    }

    fn application_spine(
        &self,
        mut function: ExpressionId,
        arguments: &[ExpressionId],
    ) -> (ExpressionId, Vec<ExpressionId>) {
        let mut groups = vec![arguments];
        while let ExpressionKind::Application { function: inner, arguments } =
            &self.storage[function].kind
        {
            function = *inner;
            groups.push(arguments);
        }
        let arguments = groups.into_iter().rev().flatten().copied();
        (function, arguments.collect())
    }

    fn flipped_application(
        &mut self,
        argument: ExpressionId,
        function: ExpressionId,
    ) -> ConversionResult<ExpressionId> {
        if self.expression_is_stable(argument) || self.expression_is_stable(function) {
            return self.application(function, [argument]);
        }

        let argument_parameter = self.fresh_parameter("applyArgument".into())?;
        let function_parameter = self.fresh_parameter("applyFunction".into())?;
        let argument_local =
            self.expression(ExpressionKind::Local { parameter: argument_parameter.clone() });
        let function_local =
            self.expression(ExpressionKind::Local { parameter: function_parameter.clone() });
        let body = self.application(function_local, [argument_local])?;
        let bindings = [
            Binding { parameter: argument_parameter, expression: argument, source_order: 0 },
            Binding { parameter: function_parameter, expression: function, source_order: 1 },
        ];
        Ok(self.expression(ExpressionKind::Let {
            recursive: false,
            bindings: bindings.into(),
            body,
        }))
    }

    fn expression_is_stable(&self, expression: ExpressionId) -> bool {
        matches!(
            self.storage[expression].kind,
            ExpressionKind::Literal { .. }
                | ExpressionKind::Constructor { .. }
                | ExpressionKind::Global { .. }
                | ExpressionKind::Local { .. }
        )
    }

    fn known_term(
        &self,
        expression: ExpressionId,
        module_name: &str,
        item_name: &str,
    ) -> QueryResult<bool> {
        let ExpressionKind::Global { global } = &self.storage[expression].kind else {
            return Ok(false);
        };
        let GlobalId::Term(file_id, _) = global.id else { return Ok(false) };
        Ok(global.item_name == item_name && self.source_module_name(file_id)? == module_name)
    }

    fn known_numbered_term_arity(
        &self,
        expression: ExpressionId,
        module_name: &str,
        item_prefix: &str,
    ) -> QueryResult<Option<usize>> {
        let ExpressionKind::Global { global } = &self.storage[expression].kind else {
            return Ok(None);
        };
        let GlobalId::Term(file_id, _) = global.id else { return Ok(None) };
        if self.source_module_name(file_id)? != module_name {
            return Ok(None);
        }
        let Some(arity) = global.item_name.strip_prefix(item_prefix) else {
            return Ok(None);
        };
        let Ok(arity) = arity.parse::<usize>() else { return Ok(None) };
        let canonical_name = format_smolstr!("{item_prefix}{arity}");
        Ok((arity <= 10 && global.item_name == canonical_name).then_some(arity))
    }

    fn known_instance_member(
        &self,
        expression: ExpressionId,
        module_name: &str,
        member_name: &str,
        instance_name: &str,
    ) -> QueryResult<bool> {
        let ExpressionKind::Project { record, field } = &self.storage[expression].kind else {
            return Ok(false);
        };
        let FieldIdentity::Member(member_file, _) = field.identity else {
            return Ok(false);
        };
        if field.name != member_name || self.source_module_name(member_file)? != module_name {
            return Ok(false);
        }
        let ExpressionKind::Global { global } = &self.storage[*record].kind else {
            return Ok(false);
        };
        let GlobalId::Instance(identity) = global.id else { return Ok(false) };
        let instance_file = match identity {
            InstanceIdentity::Declared(file_id, _) | InstanceIdentity::Derived(file_id, _) => {
                file_id
            }
        };
        Ok(global.item_name == instance_name
            && self.source_module_name(instance_file)? == module_name)
    }

    fn expression(&mut self, kind: ExpressionKind) -> ExpressionId {
        self.storage.allocate_expression(Expression { kind })
    }

    fn pattern(&mut self, kind: PatternKind) -> PatternId {
        self.storage.allocate_pattern(Pattern { kind })
    }

    fn label_field(&self, label: SmolStr) -> Field {
        Field { identity: FieldIdentity::Label(SmolStr::clone(&label)), name: label }
    }

    fn member_field(&self, (file_id, term_id): (FileId, TermItemId)) -> QueryResult<Field> {
        let indexed = self.indexed_module(file_id)?;
        let name = match &indexed.items[term_id].name {
            Some(name) => SmolStr::clone(name),
            None => self.term_fallback(term_id),
        };
        Ok(Field { identity: FieldIdentity::Member(file_id, term_id), name })
    }

    fn superclass_field(
        &self,
        superclass: checking::evidence::SuperclassId,
    ) -> ConversionResult<Field> {
        let identity = SuperclassIdentity {
            file_id: superclass.file_id,
            class: superclass.type_id,
            source: superclass.source_id,
        };
        let checked = if superclass.file_id == self.file_id {
            Arc::clone(&self.checked)
        } else {
            self.queries.checked(superclass.file_id)?
        };
        let declaration = checked.tree.lookup_type_declaration(superclass.type_id);
        let class = declaration.and_then(|declaration| {
            let declaration = &checked.tree[declaration];
            if let checking_tree::TypeDeclarationKind::Class(class) = &declaration.declaration {
                Some(class)
            } else {
                None
            }
        });
        let candidate = class.and_then(|class| {
            class.superclasses.iter().enumerate().find(|(_, candidate)| candidate.id == superclass)
        });
        let (position, base) = match candidate {
            Some((position, candidate)) => {
                let base = self.constraint_class_name(candidate.constraint)?;
                (position, base.unwrap_or_else(|| SmolStr::new("Superclass")))
            }
            None => (0, SmolStr::new("Superclass")),
        };
        let name = format_smolstr!("{base}{position}");
        Ok(Field { identity: FieldIdentity::Superclass(identity), name })
    }

    fn constraint_class_name(&self, constraint: checking::TypeId) -> QueryResult<Option<SmolStr>> {
        let mut current = constraint;
        loop {
            match self.queries.lookup_type(current) {
                checking::Type::Application(function, _)
                | checking::Type::KindApplication(function, _)
                | checking::Type::Kinded(function, _) => current = function,
                checking::Type::Constructor(file_id, type_id) => {
                    return self.type_item_name(file_id, type_id);
                }
                checking::Type::Forall(_, _)
                | checking::Type::Constrained(_, _)
                | checking::Type::Function(_, _)
                | checking::Type::Row(_)
                | checking::Type::Rigid(_, _, _)
                | checking::Type::Integer(_)
                | checking::Type::String(..)
                | checking::Type::Unification(_)
                | checking::Type::Free(_)
                | checking::Type::Unknown(_) => return Ok(None),
            }
        }
    }

    fn source_module_name(&self, file_id: FileId) -> QueryResult<SmolStr> {
        if file_id == self.file_id {
            return Ok(SmolStr::clone(&self.module_name));
        }
        let content = self.queries.content(file_id)?;
        let (parsed, _) = self.queries.parsed(file_id)?;
        let name = parsed
            .module_name(&content)
            .expect("invariant violated: referenced checked module has no source module name");
        Ok(name)
    }

    fn indexed_module(&self, file_id: FileId) -> QueryResult<Arc<indexing::IndexedModule>> {
        if file_id == self.file_id {
            Ok(Arc::clone(&self.indexed))
        } else {
            self.queries.indexed(file_id)
        }
    }

    fn type_item_name(
        &self,
        file_id: FileId,
        type_id: indexing::TypeItemId,
    ) -> QueryResult<Option<SmolStr>> {
        let indexed = self.indexed_module(file_id)?;
        Ok(indexed.items[type_id].name.clone())
    }

    fn term_global(&mut self, file_id: FileId, term_id: TermItemId) -> ConversionResult<Global> {
        let indexed = self.indexed_module(file_id)?;
        let item_name = if let Some(name) = &indexed.items[term_id].name {
            SmolStr::clone(name)
        } else {
            self.term_fallback(term_id)
        };
        self.register_dependency(file_id)?;
        Ok(Global { id: GlobalId::Term(file_id, term_id), item_name })
    }

    fn constructor_is_newtype(&self, file_id: FileId, term_id: TermItemId) -> QueryResult<bool> {
        let indexed = self.indexed_module(file_id)?;
        let Some(type_id) = indexed.constructor_type(term_id) else {
            return Ok(false);
        };
        Ok(matches!(indexed.items[type_id].kind, IndexedTypeItemKind::Newtype { .. }))
    }

    fn instance_global(&mut self, origin: InstanceCandidateOrigin) -> ConversionResult<Global> {
        let identity = match origin {
            InstanceCandidateOrigin::Instance(file_id, id) => {
                InstanceIdentity::Declared(file_id, id)
            }
            InstanceCandidateOrigin::Derive(file_id, id) => InstanceIdentity::Derived(file_id, id),
        };
        let item_name = self.instance_name(identity)?;
        let file_id = match identity {
            InstanceIdentity::Declared(file_id, _) | InstanceIdentity::Derived(file_id, _) => {
                file_id
            }
        };
        self.register_dependency(file_id)?;
        Ok(Global { id: GlobalId::Instance(identity), item_name })
    }

    fn register_dependency(&mut self, file_id: FileId) -> QueryResult<()> {
        if file_id == self.file_id || self.dependencies.contains_key(&file_id) {
            return Ok(());
        }
        let module_name = self.source_module_name(file_id)?;
        self.dependencies.insert(file_id, module_name);
        Ok(())
    }

    fn instance_name(&self, identity: InstanceIdentity) -> QueryResult<SmolStr> {
        let origin = match identity {
            InstanceIdentity::Declared(file_id, id) => {
                InstanceCandidateOrigin::Instance(file_id, id)
            }
            InstanceIdentity::Derived(file_id, id) => InstanceCandidateOrigin::Derive(file_id, id),
        };
        let pretty = checking::tree::pretty::Pretty::new(self.queries, &self.checked);
        pretty.render_instance_name(self.file_id, origin)
    }

    fn record_pun_source(
        &self,
        binder_id: checking_tree::BinderId,
        label: &str,
    ) -> Option<lowering::RecordPunId> {
        let checking_tree::BinderSource::Binder(source) = self.checked.tree[binder_id].source
        else {
            return None;
        };
        let lowering::BinderKind::Record { record } = self.lowered.tree.get_binder_kind(source)?
        else {
            return None;
        };
        record.iter().find_map(|field| {
            let lowering::BinderRecordItem::RecordPun { id, name } = field else {
                return None;
            };
            name.as_deref().filter(|name| *name == label).map(|_| *id)
        })
    }

    fn record_pun_name(&self, source: lowering::RecordPunId) -> SmolStr {
        match self.record_pun_names.get(&source) {
            Some(name) => SmolStr::clone(name),
            None => format_smolstr!("pun{}", source.into_raw().get()),
        }
    }

    fn term_fallback(&self, term_id: TermItemId) -> SmolStr {
        format_smolstr!("term{}", term_id.into_raw().into_u32())
    }

    fn unsupported(&self, state: UnsupportedState) -> ConversionError {
        crate::ModuleError::Unsupported { file_id: self.file_id, state }.into()
    }
}

fn append_evidence_name_fragment(name: &mut String, fragment: &str, fragments: &mut usize) {
    if *fragments >= MAX_EVIDENCE_NAME_FRAGMENTS {
        return;
    }
    let mut characters = fragment.chars();
    let Some(first) = characters.next() else { return };
    name.extend(first.to_uppercase());
    name.push_str(characters.as_str());
    *fragments += 1;
}

fn lowercase_initial(name: &str) -> Option<String> {
    let mut characters = name.chars();
    let first = characters.next()?;
    let first = first.to_lowercase().collect::<String>();
    Some(format!("{first}{}", characters.as_str()))
}
