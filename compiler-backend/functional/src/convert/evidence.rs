use std::sync::Arc;

use building_types::QueryResult;
use checking::evidence::{
    Evidence, EvidenceBinderId, EvidenceId, EvidenceState, EvidenceVarId, InstanceCandidateOrigin,
};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use smol_str::{SmolStr, format_smolstr};

use crate::error::UnsupportedState;
use crate::optimize::{expression_globals, reachable_expressions};
use crate::tree::{
    Binding, Declaration, DeclarationKind, Expression, ExpressionId, ExpressionKind, Global,
    GlobalId, InstanceIdentity, Parameter, ReflectableEvidence, ReflectableOrdering, Storage,
    SynthesizedEvidence,
};

use super::{BindingSource, Context, ConversionResult, lowercase_initial};

const MAX_EVIDENCE_NAME_FRAGMENTS: usize = 4;

#[derive(Default)]
pub(super) struct EvidenceScope {
    // Evidence containing local dictionary parameters cannot escape its lexical scope.
    constructions: FxHashMap<EvidenceKey, EvidenceConstruction>,
    bindings: Vec<EvidenceBinding>,
}

#[derive(Default)]
pub(super) struct EvidenceHoisting {
    // Closed evidence is collected across lexical scopes so repetition can introduce a module global.
    occurrences: FxHashMap<ClosedEvidenceKey, EvidenceOccurrences>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum ClosedEvidenceKey {
    Dictionary(EvidenceKey),
    Member { member: (files::FileId, indexing::TermItemId), evidence: EvidenceKey },
}

#[derive(Default)]
struct EvidenceOccurrences {
    expressions: Vec<ExpressionId>,
    constraint_name: Option<SmolStr>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum EvidenceKey {
    Given(EvidenceBinderId),
    Instance { origin: InstanceCandidateOrigin, subgoals: Vec<EvidenceKey> },
    Superclass { parent: Box<EvidenceKey>, superclass: checking::evidence::SuperclassId },
    Opaque(EvidenceId),
}

impl EvidenceKey {
    fn is_closed(&self) -> bool {
        match self {
            EvidenceKey::Given(_) | EvidenceKey::Opaque(_) => false,
            EvidenceKey::Instance { subgoals, .. } => subgoals.iter().all(EvidenceKey::is_closed),
            EvidenceKey::Superclass { parent, .. } => parent.is_closed(),
        }
    }

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

impl ClosedEvidenceKey {
    fn dependency_order(&self) -> usize {
        match self {
            ClosedEvidenceKey::Dictionary(evidence) => evidence.dependency_order(),
            ClosedEvidenceKey::Member { evidence, .. } => {
                evidence.dependency_order().saturating_add(1)
            }
        }
    }

    fn contains_unsafe_instance(&self, unsafe_instances: &FxHashSet<InstanceIdentity>) -> bool {
        let evidence = match self {
            ClosedEvidenceKey::Dictionary(evidence)
            | ClosedEvidenceKey::Member { evidence, .. } => evidence,
        };
        evidence_contains_unsafe_instance(evidence, unsafe_instances)
    }
}

impl EvidenceHoisting {
    fn record(
        &mut self,
        key: ClosedEvidenceKey,
        expression: ExpressionId,
        constraint_name: Option<SmolStr>,
    ) {
        let occurrences = self.occurrences.entry(key).or_default();
        occurrences.expressions.push(expression);

        if occurrences.constraint_name.is_none() {
            occurrences.constraint_name = constraint_name;
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

pub(super) fn evidence_variable(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    variable: EvidenceVarId,
    constraint: Option<checking::TypeId>,
) -> ConversionResult<ExpressionId> {
    if !context.lowering_evidence.insert(variable) {
        return Err(context.unsupported(UnsupportedState::CyclicEvidence(variable)));
    }
    let result = match context.checked.evidence[variable].state {
        EvidenceState::Unsolved => {
            Err(context.unsupported(UnsupportedState::UnsolvedEvidence(variable)))
        }
        EvidenceState::Solved(evidence) => convert_evidence(context, evidence, constraint),
        EvidenceState::Error => Ok(context.expression(ExpressionKind::Error)),
    };
    context.lowering_evidence.remove(&variable);
    result
}

fn convert_evidence(
    context: &mut Context<'_, impl checking::ExternalQueries>,
    evidence_id: EvidenceId,
    constraint: Option<checking::TypeId>,
) -> ConversionResult<ExpressionId> {
    let checked = Arc::clone(&context.checked);
    match &checked.evidence[evidence_id] {
        Evidence::Variable(variable) => evidence_variable(context, *variable, constraint),
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
            let arguments =
                subgoals.iter().map(|&subgoal| evidence_variable(context, subgoal, None));
            let arguments = arguments.collect::<ConversionResult<Vec<_>>>()?;
            let construction = context.synthetic_application(function, arguments)?;
            if subgoals.is_empty() {
                Ok(construction)
            } else {
                context.record_evidence(evidence, construction, name, constraint)
            }
        }
        Evidence::Superclass { parent, superclass } => {
            let evidence = context.evidence_key(evidence_id);
            if let Some(expression) = context.shared_evidence(&evidence)? {
                return Ok(expression);
            }
            let record = convert_evidence(context, *parent, None)?;
            let field = context.superclass_field(*superclass)?;
            let name = format_smolstr!("{}Dict", field.name);
            let accessor = context.expression(ExpressionKind::Project { record, field });
            let construction = context.expression(ExpressionKind::Application {
                function: accessor,
                arguments: Arc::from([]),
                synthetic: true,
            });
            context.record_evidence(evidence, construction, name, constraint)
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
    pub(super) fn evidence_scope(
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

    pub(super) fn hoist_closed_evidence(
        &mut self,
        declarations: &mut Vec<Declaration>,
    ) -> ConversionResult<()> {
        let roots = declarations.iter().filter_map(|declaration| match declaration.kind {
            DeclarationKind::Value(expression) => Some(expression),
            DeclarationKind::Constructor { .. } | DeclarationKind::Foreign => None,
        });
        let reachable = reachable_expressions(&self.storage, roots);
        let unsafe_instances = unsafe_local_instances(&self.storage, declarations);
        let occurrences = std::mem::take(&mut self.evidence_hoisting.occurrences);

        let mut candidates = Vec::new();
        for (key, mut occurrences) in occurrences {
            let mut seen = FxHashSet::default();
            occurrences
                .expressions
                .retain(|expression| reachable.contains(expression) && seen.insert(*expression));

            // Evidence construction is shareable by compiler contract, but forcing
            // a local recursive initializer during module initialization is not.
            let repeated = occurrences.expressions.len() >= 2;
            let safe = !key.contains_unsafe_instance(&unsafe_instances);
            if !repeated || !safe {
                continue;
            }

            candidates.push((key, occurrences));
        }

        candidates.sort_by_key(|(key, occurrences)| {
            let first = occurrences.expressions[0].into_raw().into_u32();
            (key.dependency_order(), first)
        });

        for (key, occurrences) in candidates {
            let name = match occurrences.constraint_name {
                Some(name) => name,
                None => self.closed_evidence_name(&key)?,
            };
            let global = self.fresh_generated_global(name)?;
            let replacement = ExpressionKind::Global { global: Global::clone(&global) };

            let (first, remaining) = occurrences
                .expressions
                .split_first()
                .expect("invariant violated: repeated evidence has no occurrence");
            let initializer =
                self.storage.replace_expression_kind(*first, ExpressionKind::clone(&replacement));
            let initializer = self.storage.allocate_expression(Expression { kind: initializer });

            for &expression in remaining {
                self.storage
                    .replace_expression_kind(expression, ExpressionKind::clone(&replacement));
            }

            let declaration = Declaration {
                global,
                exported: false,
                recursive_group: None,
                kind: DeclarationKind::Value(initializer),
            };
            declarations.push(declaration);
        }

        Ok(())
    }

    pub(super) fn record_closed_member_selection(
        &mut self,
        function: ExpressionId,
        evidence_variable: EvidenceVarId,
        constraint: checking::TypeId,
        selection: ExpressionId,
    ) -> ConversionResult<ExpressionId> {
        let ExpressionKind::Global { global } = &self.storage[function].kind else {
            return Ok(selection);
        };
        let global = Global::clone(global);

        let GlobalId::Term(file_id, term_id) = global.id else {
            return Ok(selection);
        };
        let indexed = self.indexed_module(file_id)?;
        let indexing::IndexedTermItemKind::ClassMember { .. } = indexed.items[term_id].kind else {
            return Ok(selection);
        };

        let EvidenceState::Solved(evidence_id) = self.checked.evidence[evidence_variable].state
        else {
            return Ok(selection);
        };
        let evidence = self.evidence_key(evidence_id);
        if !evidence.is_closed() {
            return Ok(selection);
        }

        let dictionary_name = self.evidence_parameter_name(constraint)?;
        let member_name = uppercase_initial(&global.item_name);
        let name = format_smolstr!("{dictionary_name}{member_name}");

        let key = ClosedEvidenceKey::Member { member: (file_id, term_id), evidence };
        self.evidence_hoisting.record(key, selection, Some(name));

        Ok(selection)
    }

    fn shared_evidence(
        &mut self,
        evidence: &EvidenceKey,
    ) -> ConversionResult<Option<ExpressionId>> {
        if evidence.is_closed() {
            return Ok(None);
        }
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
        constraint: Option<checking::TypeId>,
    ) -> ConversionResult<ExpressionId> {
        if evidence.is_closed() {
            let name = constraint
                .map(|constraint| self.evidence_parameter_name(constraint))
                .transpose()?;
            let key = ClosedEvidenceKey::Dictionary(evidence);
            self.evidence_hoisting.record(key, construction, name);
            return Ok(construction);
        }
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

    fn evidence_dictionary_name(&self, evidence: &EvidenceKey) -> ConversionResult<SmolStr> {
        let base = self.evidence_name_base(evidence)?;
        Ok(format_smolstr!("{base}Dict"))
    }

    fn closed_evidence_name(&self, evidence: &ClosedEvidenceKey) -> ConversionResult<SmolStr> {
        match evidence {
            ClosedEvidenceKey::Dictionary(evidence) => self.evidence_dictionary_name(evidence),
            ClosedEvidenceKey::Member { member: (file_id, term_id), evidence } => {
                let dictionary_name = self.evidence_dictionary_name(evidence)?;
                let indexed = self.indexed_module(*file_id)?;
                let member_name = indexed.items[*term_id]
                    .name
                    .as_ref()
                    .map_or_else(|| String::from("Member"), |name| uppercase_initial(name));
                Ok(format_smolstr!("{dictionary_name}{member_name}"))
            }
        }
    }

    fn evidence_name_base(&self, evidence: &EvidenceKey) -> ConversionResult<SmolStr> {
        let mut name = match evidence {
            EvidenceKey::Instance { origin, .. } => {
                let identity = instance_identity(*origin);
                self.instance_name(identity)?.to_string()
            }
            EvidenceKey::Superclass { parent, superclass } => {
                let parent = self.evidence_name_base(parent)?;
                let field = self.superclass_field(*superclass)?;
                format!("{parent}{}", uppercase_initial(&field.name))
            }
            EvidenceKey::Given(_) | EvidenceKey::Opaque(_) => String::from("evidence"),
        };

        if let EvidenceKey::Instance { subgoals, .. } = evidence {
            for subgoal in subgoals {
                let subgoal = self.evidence_name_base(subgoal)?;
                name.push_str(&uppercase_initial(&subgoal));
            }
        }

        Ok(SmolStr::new(name))
    }

    pub(super) fn evidence_parameter(
        &mut self,
        binder: EvidenceBinderId,
    ) -> ConversionResult<Parameter> {
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

fn instance_identity(origin: InstanceCandidateOrigin) -> InstanceIdentity {
    match origin {
        InstanceCandidateOrigin::Instance(file_id, instance) => {
            InstanceIdentity::Declared(file_id, instance)
        }
        InstanceCandidateOrigin::Derive(file_id, derive) => {
            InstanceIdentity::Derived(file_id, derive)
        }
    }
}

fn evidence_contains_unsafe_instance(
    evidence: &EvidenceKey,
    unsafe_instances: &FxHashSet<InstanceIdentity>,
) -> bool {
    match evidence {
        EvidenceKey::Given(_) | EvidenceKey::Opaque(_) => false,
        EvidenceKey::Instance { origin, subgoals } => {
            unsafe_instances.contains(&instance_identity(*origin))
                || subgoals
                    .iter()
                    .any(|subgoal| evidence_contains_unsafe_instance(subgoal, unsafe_instances))
        }
        EvidenceKey::Superclass { parent, .. } => {
            evidence_contains_unsafe_instance(parent, unsafe_instances)
        }
    }
}

fn unsafe_local_instances(
    storage: &Storage,
    declarations: &[Declaration],
) -> FxHashSet<InstanceIdentity> {
    let values = declarations.iter().filter_map(|declaration| match declaration.kind {
        DeclarationKind::Value(expression) => {
            Some((declaration.global.id, declaration.recursive_group, expression))
        }
        DeclarationKind::Constructor { .. } | DeclarationKind::Foreign => None,
    });
    let values = values.collect_vec();

    let positions = values.iter().enumerate().map(|(position, (global, _, _))| (*global, position));
    let positions = positions.collect::<FxHashMap<_, _>>();

    let mut dependencies = vec![Vec::new(); values.len()];
    for (position, (_, _, expression)) in values.iter().enumerate() {
        let globals = expression_globals(storage, *expression);
        let dependency_positions =
            globals.into_iter().filter_map(|global| positions.get(&global).copied());
        dependencies[position].extend(dependency_positions);
        dependencies[position].sort_unstable();
        dependencies[position].dedup();
    }

    let mut hazards = FxHashSet::default();
    for (position, (_, recursive_group, _)) in values.iter().enumerate() {
        let mut visited = FxHashSet::default();
        if recursive_group.is_some()
            || reaches_declaration(position, position, &dependencies, &mut visited)
        {
            hazards.insert(position);
        }
    }

    let mut unsafe_instances = FxHashSet::default();
    for (position, (global, _, _)) in values.iter().enumerate() {
        let GlobalId::Instance(identity) = global else {
            continue;
        };
        let mut pending = vec![position];
        let mut visited = FxHashSet::default();
        let mut unsafe_instance = false;
        while let Some(dependency) = pending.pop() {
            if !visited.insert(dependency) {
                continue;
            }

            if hazards.contains(&dependency) {
                unsafe_instance = true;
                break;
            }

            let next_dependencies = dependencies[dependency].iter().copied();
            pending.extend(next_dependencies);
        }

        if unsafe_instance {
            unsafe_instances.insert(*identity);
        }
    }

    unsafe_instances
}

fn reaches_declaration(
    current: usize,
    target: usize,
    dependencies: &[Vec<usize>],
    visited: &mut FxHashSet<usize>,
) -> bool {
    for &dependency in &dependencies[current] {
        if dependency == target {
            return true;
        }
        if visited.insert(dependency)
            && reaches_declaration(dependency, target, dependencies, visited)
        {
            return true;
        }
    }
    false
}

fn uppercase_initial(name: &str) -> String {
    let mut characters = name.chars();
    let Some(first) = characters.next() else { return String::new() };
    let first = first.to_uppercase().collect::<String>();
    format!("{first}{}", characters.as_str())
}
