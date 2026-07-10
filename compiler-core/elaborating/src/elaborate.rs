use std::collections::BTreeSet;

use checking::CheckedModule;
use checking::evidence::{
    EvidenceAbstractionSite, EvidenceApplicationSite, EvidenceBinderId, EvidenceVarId,
    InstanceCandidateOrigin,
};
use files::FileId;
use indexing::{DeriveId, IndexedModule, InstanceId, TermItemId, TermItemKind};
use lowering::{
    BinderId, BinderKind, BinderRecordItem, CaseBranch, DoStatement, DoStatementId, Equation,
    ExpressionArgument, ExpressionId, ExpressionKind, ExpressionRecordItem, GroupedModule,
    GuardedExpression, InfixPair, LetBindingChunk, LetBindingNameGroupId, LoweredModule,
    PatternGuard, PatternGuarded, RecordUpdate, Scc, TermItemIr, TermVariableResolution,
    WhereExpression,
};
use petgraph::algo::tarjan_scc;
use petgraph::prelude::DiGraphMap;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use sugar::{Bracketed, OperatorTree, Sectioned};

use crate::evidence::EvidenceResolver;
use crate::{
    CoreBinding, CoreBindingGroup, CoreBindingGroupId, CoreBindingSource, CoreBindingValue,
    CoreDeriveStrategy, CoreDerivedBinder, CoreDerivedEvidence, CoreDerivedRequirement, CoreError,
    CoreExpression, CoreExpressionId, CoreExternalBinding, CoreLabel, CoreLiteral, CoreModule,
    CorePattern, CorePatternId, CoreRecordField, CoreRecordPatternField, CoreRecordUpdate,
    CoreSuperclassField, CoreTypeArgument, CoreVariable,
};

#[derive(Clone, Copy)]
pub struct ElaborationInput<'a> {
    pub file_id: FileId,
    pub indexed: &'a IndexedModule,
    pub lowered: &'a LoweredModule,
    pub grouped: &'a GroupedModule,
    pub bracketed: &'a Bracketed,
    pub sectioned: &'a Sectioned,
    pub checked: &'a CheckedModule,
}

pub fn elaborate_module(input: ElaborationInput<'_>) -> CoreModule {
    Elaborator::new(input).elaborate()
}

struct Elaborator<'a> {
    input: ElaborationInput<'a>,
    core: CoreModule,
    section_variables: FxHashMap<ExpressionId, u32>,
    unconsumed_evidence_applications: FxHashSet<EvidenceApplicationSite>,
    unconsumed_evidence_abstractions: FxHashSet<EvidenceAbstractionSite>,
    unconsumed_instance_superclasses: FxHashSet<InstanceId>,
    unconsumed_derived_requirements: FxHashSet<DeriveId>,
    next_synthetic: u32,
}

impl<'a> Elaborator<'a> {
    fn new(input: ElaborationInput<'a>) -> Self {
        let placements = &input.checked.placements;
        Elaborator {
            input,
            core: CoreModule::default(),
            section_variables: FxHashMap::default(),
            unconsumed_evidence_applications: placements.applications.keys().copied().collect(),
            unconsumed_evidence_abstractions: placements.abstractions.keys().copied().collect(),
            unconsumed_instance_superclasses: placements
                .instance_superclasses
                .keys()
                .copied()
                .collect(),
            unconsumed_derived_requirements: placements
                .derived_requirements
                .keys()
                .copied()
                .collect(),
            next_synthetic: 0,
        }
    }

    fn elaborate(mut self) -> CoreModule {
        for strongly_connected in &self.input.grouped.term_scc {
            for &item in strongly_connected.as_slice() {
                let binding = self.elaborate_top_level_binding(item);
                self.core.items.insert(item, binding);
            }
        }
        self.group_top_level_bindings();
        self.validate_evidence_placements();
        self.core
    }

    fn validate_evidence_placements(&self) {
        assert!(
            self.unconsumed_evidence_applications.is_empty(),
            "invariant violated: unconsumed evidence application placements: {:?}",
            self.unconsumed_evidence_applications,
        );
        assert!(
            self.unconsumed_evidence_abstractions.is_empty(),
            "invariant violated: unconsumed evidence abstraction placements: {:?}",
            self.unconsumed_evidence_abstractions,
        );
        assert!(
            self.unconsumed_instance_superclasses.is_empty(),
            "invariant violated: unconsumed instance superclass placements: {:?}",
            self.unconsumed_instance_superclasses,
        );
        assert!(
            self.unconsumed_derived_requirements.is_empty(),
            "invariant violated: unconsumed derived requirement placements: {:?}",
            self.unconsumed_derived_requirements,
        );
    }

    fn group_top_level_bindings(&mut self) {
        let top_level: FxHashSet<_> = self.core.items.values().copied().collect();
        let mut graph: DiGraphMap<crate::CoreBindingId, (), FxBuildHasher> = DiGraphMap::default();
        for &binding in &top_level {
            graph.add_node(binding);
        }

        for &binding in &top_level {
            let mut dependencies = FxHashSet::default();
            let mut visited_expressions = FxHashSet::default();
            self.collect_binding_dependencies(
                binding,
                &top_level,
                &mut visited_expressions,
                &mut dependencies,
            );
            for dependency in dependencies {
                graph.add_edge(binding, dependency, ());
            }
        }

        let mut components = tarjan_scc(&graph);
        for bindings in &mut components {
            bindings.sort_unstable_by_key(|&binding| self.top_level_binding_order(binding));
        }

        let mut component_by_binding = FxHashMap::default();
        for (component, bindings) in components.iter().enumerate() {
            for &binding in bindings {
                component_by_binding.insert(binding, component);
            }
        }

        let mut dependencies = vec![FxHashSet::default(); components.len()];
        let mut dependents = vec![Vec::new(); components.len()];
        for (binding, dependency, _) in graph.all_edges() {
            let component = component_by_binding[&binding];
            let dependency = component_by_binding[&dependency];
            if component != dependency && dependencies[component].insert(dependency) {
                dependents[dependency].push(component);
            }
        }

        let mut ready = BTreeSet::new();
        for (component, component_dependencies) in dependencies.iter().enumerate() {
            if component_dependencies.is_empty() {
                ready.insert((self.top_level_binding_order(components[component][0]), component));
            }
        }

        let mut emitted = 0;
        while let Some(&(order, component)) = ready.iter().next() {
            ready.remove(&(order, component));
            emitted += 1;

            let bindings = components[component].clone();
            let recursive = bindings.len() > 1
                || bindings.first().is_some_and(|binding| graph.contains_edge(*binding, *binding));
            let group = self.core.allocate_binding_group(CoreBindingGroup { recursive, bindings });
            self.core.top_level.push(group);

            for &dependent in &dependents[component] {
                dependencies[dependent].remove(&component);
                if dependencies[dependent].is_empty() {
                    ready.insert((
                        self.top_level_binding_order(components[dependent][0]),
                        dependent,
                    ));
                }
            }
        }
        debug_assert_eq!(emitted, components.len(), "condensed Core graph must be acyclic");
    }

    fn top_level_binding_order(&self, binding: crate::CoreBindingId) -> u32 {
        match self.core.bindings[binding].source {
            CoreBindingSource::Item(item) => item.into_raw().into_u32(),
            CoreBindingSource::Let(_) | CoreBindingSource::Synthetic(_) => {
                debug_assert!(false, "top-level Core binding must originate from an item");
                u32::MAX
            }
        }
    }

    fn collect_binding_dependencies(
        &self,
        binding: crate::CoreBindingId,
        top_level: &FxHashSet<crate::CoreBindingId>,
        visited: &mut FxHashSet<CoreExpressionId>,
        dependencies: &mut FxHashSet<crate::CoreBindingId>,
    ) {
        let CoreBindingValue::Expression(expression) = self.core.bindings[binding].value else {
            return;
        };
        self.collect_expression_dependencies(expression, top_level, visited, dependencies);
    }

    fn collect_expression_dependencies(
        &self,
        expression: CoreExpressionId,
        top_level: &FxHashSet<crate::CoreBindingId>,
        visited: &mut FxHashSet<CoreExpressionId>,
        dependencies: &mut FxHashSet<crate::CoreBindingId>,
    ) {
        if !visited.insert(expression) {
            return;
        }

        match &self.core.expressions[expression] {
            CoreExpression::Variable(CoreVariable::Item(file, item))
                if *file == self.input.file_id =>
            {
                if let Some(&binding) = self.core.items.get(item) {
                    dependencies.insert(binding);
                }
            }
            CoreExpression::Variable(CoreVariable::Instance(origin)) => {
                if let Some(&binding) = self.core.instances.get(origin) {
                    dependencies.insert(binding);
                }
            }
            CoreExpression::Variable(_) | CoreExpression::Literal(_) | CoreExpression::Error(_) => {
            }
            CoreExpression::Lambda { body, .. } => {
                self.collect_expression_dependencies(*body, top_level, visited, dependencies);
            }
            CoreExpression::Apply { function, argument } => {
                self.collect_expression_dependencies(*function, top_level, visited, dependencies);
                self.collect_expression_dependencies(*argument, top_level, visited, dependencies);
            }
            CoreExpression::TypeApply { function, .. } => {
                self.collect_expression_dependencies(*function, top_level, visited, dependencies);
            }
            CoreExpression::Let { group, body } => {
                for &binding in &self.core.binding_groups[*group].bindings {
                    if !top_level.contains(&binding) {
                        self.collect_binding_dependencies(
                            binding,
                            top_level,
                            visited,
                            dependencies,
                        );
                    }
                }
                self.collect_expression_dependencies(*body, top_level, visited, dependencies);
            }
            CoreExpression::Case { scrutinees, alternatives } => {
                for &scrutinee in scrutinees {
                    self.collect_expression_dependencies(
                        scrutinee,
                        top_level,
                        visited,
                        dependencies,
                    );
                }
                for &alternative in alternatives {
                    self.collect_expression_dependencies(
                        self.core.alternatives[alternative].body,
                        top_level,
                        visited,
                        dependencies,
                    );
                }
            }
            CoreExpression::IfThenElse { condition, then, else_ } => {
                for expression in [condition, then, else_] {
                    self.collect_expression_dependencies(
                        *expression,
                        top_level,
                        visited,
                        dependencies,
                    );
                }
            }
            CoreExpression::Array(elements) => {
                for &element in elements {
                    self.collect_expression_dependencies(element, top_level, visited, dependencies);
                }
            }
            CoreExpression::Record(fields) => {
                for field in fields {
                    self.collect_expression_dependencies(
                        field.value,
                        top_level,
                        visited,
                        dependencies,
                    );
                }
            }
            CoreExpression::Dictionary { superclasses, members } => {
                for superclass in superclasses {
                    if let CoreSuperclassField::Runtime(expression) = superclass {
                        self.collect_expression_dependencies(
                            *expression,
                            top_level,
                            visited,
                            dependencies,
                        );
                    }
                }
                for member in members {
                    self.collect_expression_dependencies(
                        member.value,
                        top_level,
                        visited,
                        dependencies,
                    );
                }
            }
            CoreExpression::DerivedDictionary { requirements, .. } => {
                for requirement in requirements {
                    if let CoreDerivedEvidence::Runtime(expression) = requirement.evidence {
                        self.collect_expression_dependencies(
                            expression,
                            top_level,
                            visited,
                            dependencies,
                        );
                    }
                }
            }
            CoreExpression::Access { record, .. } => {
                self.collect_expression_dependencies(*record, top_level, visited, dependencies);
            }
            CoreExpression::Update { record, updates } => {
                self.collect_expression_dependencies(*record, top_level, visited, dependencies);
                for update in updates {
                    self.collect_expression_dependencies(
                        update.value,
                        top_level,
                        visited,
                        dependencies,
                    );
                }
            }
            CoreExpression::SuperclassProjection { dictionary, .. } => {
                self.collect_expression_dependencies(*dictionary, top_level, visited, dependencies);
            }
        }
    }

    fn elaborate_top_level_binding(&mut self, item: TermItemId) -> crate::CoreBindingId {
        let value = match self.input.lowered.info.get_term_item(item) {
            Some(TermItemIr::ValueGroup { equations, .. }) => {
                let expression = self.elaborate_equations(equations);
                let expression =
                    self.abstract_evidence(EvidenceAbstractionSite::Term(item), expression);
                CoreBindingValue::Expression(expression)
            }
            Some(TermItemIr::Operator { resolution, .. }) => {
                let expression = self.elaborate_resolution(*resolution);
                let expression =
                    self.abstract_evidence(EvidenceAbstractionSite::Term(item), expression);
                CoreBindingValue::Expression(expression)
            }
            Some(TermItemIr::Instance { members, .. }) => {
                let instance = match &self.input.indexed.items[item].kind {
                    TermItemKind::Instance { id } => Some(*id),
                    _ => None,
                };
                let expression = self.elaborate_instance(instance, members);
                let expression =
                    self.abstract_evidence(EvidenceAbstractionSite::Term(item), expression);
                CoreBindingValue::Expression(expression)
            }
            Some(TermItemIr::Derive { newtype, resolution, .. }) => {
                let derive = match self.input.indexed.items[item].kind {
                    TermItemKind::Derive { id } => Some(id),
                    _ => None,
                };
                let expression = self.elaborate_derived(derive, *newtype, *resolution);
                let expression =
                    self.abstract_evidence(EvidenceAbstractionSite::Term(item), expression);
                CoreBindingValue::Expression(expression)
            }
            Some(TermItemIr::Foreign { .. }) => {
                CoreBindingValue::External(CoreExternalBinding::Foreign)
            }
            Some(TermItemIr::Constructor { .. }) => {
                CoreBindingValue::External(CoreExternalBinding::Constructor)
            }
            Some(TermItemIr::ClassMember { .. }) => {
                CoreBindingValue::External(CoreExternalBinding::ClassMember)
            }
            None => match self.input.indexed.items[item].kind {
                TermItemKind::ClassMember { .. } => {
                    CoreBindingValue::External(CoreExternalBinding::ClassMember)
                }
                TermItemKind::Constructor { .. } => {
                    CoreBindingValue::External(CoreExternalBinding::Constructor)
                }
                TermItemKind::Foreign { .. } => {
                    CoreBindingValue::External(CoreExternalBinding::Foreign)
                }
                TermItemKind::Derive { .. }
                | TermItemKind::Instance { .. }
                | TermItemKind::Operator { .. }
                | TermItemKind::Value { .. } => {
                    let error = self.error(CoreError::MissingExpression);
                    CoreBindingValue::Expression(error)
                }
            },
        };

        let binding = CoreBinding { source: CoreBindingSource::Item(item), value };
        let binding = self.core.allocate_binding(binding);
        match self.input.indexed.items[item].kind {
            TermItemKind::Instance { id } => {
                let origin = InstanceCandidateOrigin::Instance(self.input.file_id, id);
                self.core.instances.insert(origin, binding);
            }
            TermItemKind::Derive { id } => {
                let origin = InstanceCandidateOrigin::Derive(self.input.file_id, id);
                self.core.instances.insert(origin, binding);
            }
            _ => {}
        }
        let type_id = match self.input.indexed.items[item].kind {
            TermItemKind::Instance { id } => {
                self.input.checked.lookup_instance(id).map(|instance| instance.signature)
            }
            TermItemKind::Derive { id } => {
                self.input.checked.lookup_derived(id).map(|instance| instance.signature)
            }
            _ => self.input.checked.lookup_term(item),
        };
        if let Some(type_id) = type_id {
            self.core.binding_types.insert(binding, type_id);
        }
        binding
    }

    fn elaborate_instance(
        &mut self,
        instance: Option<InstanceId>,
        members: &[lowering::InstanceMemberGroup],
    ) -> CoreExpressionId {
        if let Some(instance) = instance
            && self.input.checked.placements.instance_superclasses.contains_key(&instance)
        {
            assert!(
                self.unconsumed_instance_superclasses.remove(&instance),
                "invariant violated: instance superclass placement consumed more than once: {instance:?}",
            );
        }
        let superclasses = instance
            .and_then(|instance| self.input.checked.placements.instance_superclasses.get(&instance))
            .cloned()
            .unwrap_or_default();
        let superclasses = superclasses
            .into_iter()
            .map(|evidence| {
                self.resolve_evidence(evidence)
                    .map(CoreSuperclassField::Runtime)
                    .unwrap_or(CoreSuperclassField::Erased)
            })
            .collect();

        let mut fields = Vec::with_capacity(members.len());
        for (index, member) in members.iter().enumerate() {
            let mut value = self.elaborate_equations(&member.equations);
            if let Some(instance) = instance {
                value = self.abstract_evidence(
                    EvidenceAbstractionSite::InstanceMember { instance, member: index as u32 },
                    value,
                );
            }
            let label = member
                .resolution
                .map(|(file, item)| CoreLabel::Item(file, item))
                .unwrap_or(CoreLabel::Missing);
            fields.push(CoreRecordField { label, value });
        }
        self.core.allocate_expression(CoreExpression::Dictionary { superclasses, members: fields })
    }

    fn elaborate_derived(
        &mut self,
        derive: Option<indexing::DeriveId>,
        newtype: bool,
        class: Option<(FileId, indexing::TypeItemId)>,
    ) -> CoreExpressionId {
        if let Some(derive) = derive {
            let abstraction = EvidenceAbstractionSite::Derived(derive);
            if self.input.checked.placements.abstractions.contains_key(&abstraction) {
                assert!(
                    self.unconsumed_evidence_abstractions.remove(&abstraction),
                    "invariant violated: evidence abstraction placement consumed more than once: {abstraction:?}",
                );
            }
            if self.input.checked.placements.derived_requirements.contains_key(&derive) {
                assert!(
                    self.unconsumed_derived_requirements.remove(&derive),
                    "invariant violated: derived requirement placement consumed more than once: {derive:?}",
                );
            }
        }
        let local_binders = derive
            .and_then(|derive| {
                self.input
                    .checked
                    .placements
                    .abstractions
                    .get(&EvidenceAbstractionSite::Derived(derive))
            })
            .into_iter()
            .flatten()
            .map(|&binder| {
                let evidence = self.input.checked.evidence.binder(binder);
                CoreDerivedBinder {
                    binder,
                    constraint: evidence.constraint,
                    erased: evidence.erased,
                }
            })
            .collect();
        let variables = derive
            .and_then(|derive| self.input.checked.placements.derived_requirements.get(&derive))
            .cloned()
            .unwrap_or_default();
        let requirements = variables
            .into_iter()
            .map(|variable| {
                let constraint = self.input.checked.evidence.variable(variable).constraint;
                let evidence = self
                    .resolve_evidence(variable)
                    .map(CoreDerivedEvidence::Runtime)
                    .unwrap_or(CoreDerivedEvidence::Erased);
                CoreDerivedRequirement { constraint, evidence }
            })
            .collect();
        let strategy =
            if newtype { CoreDeriveStrategy::Newtype } else { CoreDeriveStrategy::Stock };
        self.core.allocate_expression(CoreExpression::DerivedDictionary {
            strategy,
            class,
            local_binders,
            requirements,
        })
    }

    fn elaborate_expression(&mut self, source: ExpressionId) -> CoreExpressionId {
        let sections = self.input.sectioned.expressions.get(&source).cloned().unwrap_or_default();

        let mut previous = Vec::with_capacity(sections.len());
        for &section in &sections {
            let variable = self.fresh_synthetic();
            previous.push((section, self.section_variables.insert(section, variable)));
        }

        let mut expression = self.elaborate_expression_kind(source);
        expression = self.apply_evidence(EvidenceApplicationSite::Expression(source), expression);

        for &section in sections.iter().rev() {
            let variable = self.section_variables[&section];
            let pattern = self
                .core
                .allocate_pattern(CorePattern::Variable(CoreVariable::Synthetic(variable)));
            expression =
                self.core.allocate_expression(CoreExpression::Lambda { pattern, body: expression });
        }

        for (section, old) in previous {
            if let Some(old) = old {
                self.section_variables.insert(section, old);
            } else {
                self.section_variables.remove(&section);
            }
        }

        expression =
            self.abstract_evidence(EvidenceAbstractionSite::Expression(source), expression);
        self.record_source_expression(source, expression);
        expression
    }

    fn elaborate_expression_kind(&mut self, source: ExpressionId) -> CoreExpressionId {
        let Some(kind) = self.input.lowered.info.get_expression_kind(source) else {
            return self.error(CoreError::MissingExpression);
        };

        match kind {
            ExpressionKind::Typed { expression, .. }
            | ExpressionKind::Parenthesized { parenthesized: expression } => {
                self.elaborate_optional_expression(*expression)
            }
            ExpressionKind::OperatorChain { .. } => {
                let Some(tree) = self.input.bracketed.expressions.get(&source) else {
                    return self.error(CoreError::MalformedOperator);
                };
                match tree {
                    Ok(tree) => self.elaborate_operator_expression(tree),
                    Err(_) => self.error(CoreError::MalformedOperator),
                }
            }
            ExpressionKind::InfixChain { head, tail } => self.elaborate_infix(source, *head, tail),
            ExpressionKind::Negate { negate, expression } => {
                let function = self.elaborate_variable_resolution(*negate);
                let function =
                    self.apply_evidence(EvidenceApplicationSite::Negate(source), function);
                let argument = self.elaborate_optional_expression(*expression);
                self.apply(function, argument)
            }
            ExpressionKind::Application { function, arguments } => {
                let mut function = self.elaborate_optional_expression(*function);
                for (index, argument) in arguments.iter().enumerate() {
                    function = self.apply_evidence(
                        EvidenceApplicationSite::Application {
                            expression: source,
                            argument: index as u32,
                        },
                        function,
                    );
                    function = match argument {
                        ExpressionArgument::Term(argument) => {
                            let argument = self.elaborate_optional_expression(*argument);
                            self.apply(function, argument)
                        }
                        ExpressionArgument::Type(argument) => {
                            let argument = argument
                                .and_then(|id| self.input.checked.nodes.lookup_type_expression(id))
                                .map(CoreTypeArgument::Checked)
                                .unwrap_or(CoreTypeArgument::Missing);
                            self.core.allocate_expression(CoreExpression::TypeApply {
                                function,
                                argument,
                            })
                        }
                    };
                }
                function
            }
            ExpressionKind::IfThenElse { if_, then, else_ } => {
                let condition = self.elaborate_optional_expression(*if_);
                let then = self.elaborate_optional_expression(*then);
                let else_ = self.elaborate_optional_expression(*else_);
                self.core.allocate_expression(CoreExpression::IfThenElse { condition, then, else_ })
            }
            ExpressionKind::LetIn { bindings, expression } => {
                let body = self.elaborate_optional_expression(*expression);
                self.elaborate_let_chunks(bindings, body)
            }
            ExpressionKind::Lambda { binders, expression } => {
                let mut body = self.elaborate_optional_expression(*expression);
                for &binder in binders.iter().rev() {
                    let pattern = self.elaborate_pattern(binder);
                    body = self.core.allocate_expression(CoreExpression::Lambda { pattern, body });
                }
                body
            }
            ExpressionKind::CaseOf { trunk, branches } => self.elaborate_case(trunk, branches),
            ExpressionKind::Do { bind, discard, statements } => {
                self.elaborate_do(source, *bind, *discard, statements)
            }
            ExpressionKind::Ado { map, apply, pure, statements, expression } => {
                self.elaborate_ado(source, *map, *apply, *pure, statements, *expression)
            }
            ExpressionKind::Constructor { resolution }
            | ExpressionKind::OperatorName { resolution } => self.elaborate_resolution(*resolution),
            ExpressionKind::Variable { resolution } => {
                self.elaborate_variable_resolution(*resolution)
            }
            ExpressionKind::Section => {
                let Some(&variable) = self.section_variables.get(&source) else {
                    return self.error(CoreError::MalformedSection);
                };
                self.core.allocate_expression(CoreExpression::Variable(CoreVariable::Synthetic(
                    variable,
                )))
            }
            ExpressionKind::Hole => self.error(CoreError::Hole),
            ExpressionKind::String { kind, value } => {
                self.literal(CoreLiteral::String { kind: *kind, value: value.clone() })
            }
            ExpressionKind::Char { value } => self.literal(CoreLiteral::Char(*value)),
            ExpressionKind::Boolean { boolean } => self.literal(CoreLiteral::Boolean(*boolean)),
            ExpressionKind::Integer { value } => self.literal(CoreLiteral::Integer(*value)),
            ExpressionKind::Number { value } => {
                self.literal(CoreLiteral::Number { negative: false, value: value.clone() })
            }
            ExpressionKind::Array { array } => {
                let elements = array.iter().map(|&id| self.elaborate_expression(id)).collect();
                self.core.allocate_expression(CoreExpression::Array(elements))
            }
            ExpressionKind::Record { record } => self.elaborate_record(record),
            ExpressionKind::RecordAccess { record, labels } => {
                let mut record = self.elaborate_optional_expression(*record);
                if let Some(labels) = labels {
                    for (index, label) in labels.iter().enumerate() {
                        record = self.apply_evidence(
                            EvidenceApplicationSite::RecordAccess {
                                expression: source,
                                label: index as u32,
                            },
                            record,
                        );
                        record = self.core.allocate_expression(CoreExpression::Access {
                            record,
                            label: CoreLabel::Source(label.clone()),
                        });
                    }
                } else {
                    record = self.core.allocate_expression(CoreExpression::Access {
                        record,
                        label: CoreLabel::Missing,
                    });
                }
                record
            }
            ExpressionKind::RecordUpdate { record, updates } => {
                let record = self.elaborate_optional_expression(*record);
                let mut elaborated = Vec::new();
                self.elaborate_record_updates(updates, &mut Vec::new(), &mut elaborated);
                self.core
                    .allocate_expression(CoreExpression::Update { record, updates: elaborated })
            }
        }
    }

    fn elaborate_operator_expression(
        &mut self,
        tree: &OperatorTree<ExpressionId>,
    ) -> CoreExpressionId {
        match tree {
            OperatorTree::Leaf(expression) => self.elaborate_optional_expression(*expression),
            OperatorTree::Branch(operator, children) => {
                let mut function = self
                    .input
                    .checked
                    .nodes
                    .lookup_term_operator_target(*operator)
                    .map(|(file, item)| {
                        self.core.allocate_expression(CoreExpression::Variable(CoreVariable::Item(
                            file, item,
                        )))
                    })
                    .unwrap_or_else(|| self.error(CoreError::MalformedOperator));
                function =
                    self.apply_evidence(EvidenceApplicationSite::Operator(*operator), function);
                let left = self.elaborate_operator_expression(&children[0]);
                function = self.apply(function, left);
                let right = self.elaborate_operator_expression(&children[1]);
                let expression = self.apply(function, right);
                let expression = self
                    .apply_evidence(EvidenceApplicationSite::OperatorResult(*operator), expression);
                self.abstract_evidence(EvidenceAbstractionSite::Operator(*operator), expression)
            }
        }
    }

    fn elaborate_infix(
        &mut self,
        source: ExpressionId,
        head: Option<ExpressionId>,
        tail: &[InfixPair<ExpressionId>],
    ) -> CoreExpressionId {
        let mut left = self.elaborate_optional_expression(head);
        for (pair, infix) in tail.iter().enumerate() {
            let mut function = self.elaborate_optional_expression(infix.tick);
            left = self.apply_evidence(
                EvidenceApplicationSite::Infix {
                    expression: source,
                    pair: pair as u32,
                    argument: 0,
                },
                left,
            );
            function = self.apply(function, left);
            function = self.apply_evidence(
                EvidenceApplicationSite::Infix {
                    expression: source,
                    pair: pair as u32,
                    argument: 1,
                },
                function,
            );
            let right = self.elaborate_optional_expression(infix.element);
            left = self.apply(function, right);
        }
        left
    }

    fn elaborate_record(&mut self, record: &[ExpressionRecordItem]) -> CoreExpressionId {
        let mut fields = Vec::with_capacity(record.len());
        for field in record {
            match field {
                ExpressionRecordItem::RecordField { name, value } => {
                    let label = name.clone().map(CoreLabel::Source).unwrap_or(CoreLabel::Missing);
                    let value = self.elaborate_optional_expression(*value);
                    fields.push(CoreRecordField { label, value });
                }
                ExpressionRecordItem::RecordPun { id, name, resolution } => {
                    let label = name.clone().map(CoreLabel::Source).unwrap_or(CoreLabel::Missing);
                    let mut value = self.elaborate_variable_resolution(*resolution);
                    value = self.apply_evidence(EvidenceApplicationSite::RecordPun(*id), value);
                    value = self.abstract_evidence(EvidenceAbstractionSite::RecordPun(*id), value);
                    fields.push(CoreRecordField { label, value });
                }
            }
        }
        self.core.allocate_expression(CoreExpression::Record(fields))
    }

    fn elaborate_record_updates(
        &mut self,
        updates: &[RecordUpdate],
        path: &mut Vec<CoreLabel>,
        result: &mut Vec<CoreRecordUpdate>,
    ) {
        for update in updates {
            match update {
                RecordUpdate::Leaf { name, expression } => {
                    path.push(name.clone().map(CoreLabel::Source).unwrap_or(CoreLabel::Missing));
                    let value = self.elaborate_optional_expression(*expression);
                    result.push(CoreRecordUpdate { path: path.clone(), value });
                    path.pop();
                }
                RecordUpdate::Branch { name, updates } => {
                    path.push(name.clone().map(CoreLabel::Source).unwrap_or(CoreLabel::Missing));
                    self.elaborate_record_updates(updates, path, result);
                    path.pop();
                }
            }
        }
    }

    fn elaborate_case(
        &mut self,
        trunk: &[ExpressionId],
        branches: &[CaseBranch],
    ) -> CoreExpressionId {
        let values: Vec<_> = trunk.iter().map(|&id| self.elaborate_expression(id)).collect();
        let (group, scrutinees) = self.bind_once(values);
        let mut failure = self.error(CoreError::PatternMatchFailure);

        for branch in branches.iter().rev() {
            let success = branch
                .guarded_expression
                .as_ref()
                .map(|guarded| self.elaborate_guarded(guarded, failure))
                .unwrap_or_else(|| self.error(CoreError::MissingExpression));
            let patterns = self.elaborate_patterns(&branch.binders, scrutinees.len());
            failure = self.case_with_fallback(scrutinees.clone(), patterns, success, failure);
        }

        if let Some(group) = group {
            self.core.allocate_expression(CoreExpression::Let { group, body: failure })
        } else {
            failure
        }
    }

    fn elaborate_equations(&mut self, equations: &[Equation]) -> CoreExpressionId {
        if equations.is_empty() {
            return self.error(CoreError::MissingExpression);
        }

        let arity = equations.iter().map(|equation| equation.binders.len()).max().unwrap_or(0);
        let mut parameters = Vec::with_capacity(arity);
        let mut scrutinees = Vec::with_capacity(arity);
        for _ in 0..arity {
            let variable = CoreVariable::Synthetic(self.fresh_synthetic());
            let pattern = self.core.allocate_pattern(CorePattern::Variable(variable));
            let expression = self.core.allocate_expression(CoreExpression::Variable(variable));
            parameters.push(pattern);
            scrutinees.push(expression);
        }

        let mut body = self.error(CoreError::PatternMatchFailure);
        for equation in equations.iter().rev() {
            let success = equation
                .guarded
                .as_ref()
                .map(|guarded| self.elaborate_guarded(guarded, body))
                .unwrap_or_else(|| self.error(CoreError::MissingExpression));
            let patterns = self.elaborate_patterns(&equation.binders, arity);
            body = self.case_with_fallback(scrutinees.clone(), patterns, success, body);
        }

        for pattern in parameters.into_iter().rev() {
            body = self.core.allocate_expression(CoreExpression::Lambda { pattern, body });
        }
        body
    }

    fn elaborate_guarded(
        &mut self,
        guarded: &GuardedExpression,
        failure: CoreExpressionId,
    ) -> CoreExpressionId {
        match guarded {
            GuardedExpression::Unconditional { where_expression } => where_expression
                .as_ref()
                .map(|expression| self.elaborate_where_expression(expression))
                .unwrap_or_else(|| self.error(CoreError::MissingExpression)),
            GuardedExpression::Conditionals { pattern_guarded } => {
                let mut result = failure;
                for guarded in pattern_guarded.iter().rev() {
                    result = self.elaborate_pattern_guarded(guarded, result);
                }
                result
            }
        }
    }

    fn elaborate_pattern_guarded(
        &mut self,
        guarded: &PatternGuarded,
        failure: CoreExpressionId,
    ) -> CoreExpressionId {
        let mut success = guarded
            .where_expression
            .as_ref()
            .map(|expression| self.elaborate_where_expression(expression))
            .unwrap_or_else(|| self.error(CoreError::MissingExpression));

        for guard in guarded.pattern_guards.iter().rev() {
            success = self.elaborate_pattern_guard(guard, success, failure);
        }
        success
    }

    fn elaborate_pattern_guard(
        &mut self,
        guard: &PatternGuard,
        success: CoreExpressionId,
        failure: CoreExpressionId,
    ) -> CoreExpressionId {
        let expression = self.elaborate_optional_expression(guard.expression);
        if let Some(binder) = guard.binder {
            let pattern = self.elaborate_pattern(binder);
            self.case_with_fallback(vec![expression], vec![pattern], success, failure)
        } else {
            self.core.allocate_expression(CoreExpression::IfThenElse {
                condition: expression,
                then: success,
                else_: failure,
            })
        }
    }

    fn elaborate_where_expression(&mut self, expression: &WhereExpression) -> CoreExpressionId {
        let body = self.elaborate_optional_expression(expression.expression);
        self.elaborate_let_chunks(&expression.bindings, body)
    }

    fn elaborate_let_chunks(
        &mut self,
        chunks: &[LetBindingChunk],
        mut body: CoreExpressionId,
    ) -> CoreExpressionId {
        for chunk in chunks.iter().rev() {
            match chunk {
                LetBindingChunk::Pattern { binder, where_expression } => {
                    let value = where_expression
                        .as_ref()
                        .map(|expression| self.elaborate_where_expression(expression))
                        .unwrap_or_else(|| self.error(CoreError::MissingExpression));
                    let pattern = binder
                        .map(|binder| self.elaborate_pattern(binder))
                        .unwrap_or_else(|| self.core.allocate_pattern(CorePattern::Wildcard));
                    let failure = self.error(CoreError::PatternMatchFailure);
                    body = self.case_with_fallback(vec![value], vec![pattern], body, failure);
                }
                LetBindingChunk::Names { scc, .. } => {
                    for strongly_connected in scc.iter().rev() {
                        let group = self.elaborate_let_group(strongly_connected);
                        body = self.core.allocate_expression(CoreExpression::Let { group, body });
                    }
                }
            }
        }
        body
    }

    fn elaborate_let_group(
        &mut self,
        strongly_connected: &Scc<LetBindingNameGroupId>,
    ) -> CoreBindingGroupId {
        let mut bindings = Vec::new();
        for &source in strongly_connected.as_slice() {
            let expression = self
                .input
                .lowered
                .info
                .get_let_binding(source)
                .map(|binding| self.elaborate_equations(&binding.equations))
                .unwrap_or_else(|| self.error(CoreError::MissingExpression));
            let expression =
                self.abstract_evidence(EvidenceAbstractionSite::Let(source), expression);
            let binding = CoreBinding {
                source: CoreBindingSource::Let(source),
                value: CoreBindingValue::Expression(expression),
            };
            let binding = self.core.allocate_binding(binding);
            self.core.lets.insert(source, binding);
            if let Some(type_id) = self.input.checked.nodes.lookup_let(source) {
                self.core.binding_types.insert(binding, type_id);
            }
            bindings.push(binding);
        }
        self.core.allocate_binding_group(CoreBindingGroup {
            recursive: strongly_connected.is_recursive(),
            bindings,
        })
    }

    fn elaborate_do(
        &mut self,
        source: ExpressionId,
        bind: Option<TermVariableResolution>,
        discard: Option<TermVariableResolution>,
        statements: &[DoStatementId],
    ) -> CoreExpressionId {
        let Some((&last, preceding)) = statements.split_last() else {
            return self.error(CoreError::MissingExpression);
        };

        let mut body = match self.input.lowered.info.get_do_statement(last) {
            Some(DoStatement::Bind { expression, .. })
            | Some(DoStatement::Discard { expression }) => {
                self.elaborate_optional_expression(*expression)
            }
            Some(DoStatement::Let { statements }) => {
                let error = self.error(CoreError::MissingExpression);
                self.elaborate_let_chunks(statements, error)
            }
            None => self.error(CoreError::MissingExpression),
        };

        for &statement_id in preceding.iter().rev() {
            let Some(statement) = self.input.lowered.info.get_do_statement(statement_id) else {
                body = self.error(CoreError::MissingExpression);
                continue;
            };
            match statement {
                DoStatement::Let { statements } => {
                    body = self.elaborate_let_chunks(statements, body);
                }
                DoStatement::Bind { binder, expression } => {
                    let mut function = self.elaborate_variable_resolution(bind);
                    function = self.apply_evidence(
                        EvidenceApplicationSite::Do {
                            expression: source,
                            statement: statement_id,
                            argument: 0,
                        },
                        function,
                    );
                    let action = self.elaborate_optional_expression(*expression);
                    function = self.apply(function, action);
                    function = self.apply_evidence(
                        EvidenceApplicationSite::Do {
                            expression: source,
                            statement: statement_id,
                            argument: 1,
                        },
                        function,
                    );
                    let pattern = binder
                        .map(|binder| self.elaborate_pattern(binder))
                        .unwrap_or_else(|| self.core.allocate_pattern(CorePattern::Wildcard));
                    let continuation =
                        self.core.allocate_expression(CoreExpression::Lambda { pattern, body });
                    body = self.apply(function, continuation);
                    body = self.apply_evidence(
                        EvidenceApplicationSite::DoResult {
                            expression: source,
                            statement: statement_id,
                        },
                        body,
                    );
                }
                DoStatement::Discard { expression } => {
                    let mut function = self.elaborate_variable_resolution(discard);
                    function = self.apply_evidence(
                        EvidenceApplicationSite::Do {
                            expression: source,
                            statement: statement_id,
                            argument: 0,
                        },
                        function,
                    );
                    let action = self.elaborate_optional_expression(*expression);
                    function = self.apply(function, action);
                    function = self.apply_evidence(
                        EvidenceApplicationSite::Do {
                            expression: source,
                            statement: statement_id,
                            argument: 1,
                        },
                        function,
                    );
                    let pattern = self.core.allocate_pattern(CorePattern::Wildcard);
                    let continuation =
                        self.core.allocate_expression(CoreExpression::Lambda { pattern, body });
                    body = self.apply(function, continuation);
                    body = self.apply_evidence(
                        EvidenceApplicationSite::DoResult {
                            expression: source,
                            statement: statement_id,
                        },
                        body,
                    );
                }
            }
        }
        body
    }

    fn elaborate_ado(
        &mut self,
        source: ExpressionId,
        map: Option<TermVariableResolution>,
        apply: Option<TermVariableResolution>,
        pure: Option<TermVariableResolution>,
        statements: &[DoStatementId],
        expression: Option<ExpressionId>,
    ) -> CoreExpressionId {
        let mut continuation = self.elaborate_optional_expression(expression);
        let mut actions = Vec::new();

        for &statement_id in statements {
            match self.input.lowered.info.get_do_statement(statement_id) {
                Some(DoStatement::Let { .. }) => {}
                Some(DoStatement::Bind { binder, expression }) => {
                    actions.push((statement_id, *binder, *expression));
                }
                Some(DoStatement::Discard { expression }) => {
                    actions.push((statement_id, None, *expression));
                }
                None => {}
            }
        }

        // Ado action expressions are deliberately lowered in the surrounding
        // scope, while binders and lets scope over the continuation. Rebuild
        // that continuation from the end so a let after an action is nested
        // under that action's lambda, rather than escaping around the whole
        // applicative pipeline.
        for &statement_id in statements.iter().rev() {
            match self.input.lowered.info.get_do_statement(statement_id) {
                Some(DoStatement::Let { statements }) => {
                    continuation = self.elaborate_let_chunks(statements, continuation);
                }
                Some(DoStatement::Bind { binder, .. }) => {
                    let pattern = binder
                        .map(|binder| self.elaborate_pattern(binder))
                        .unwrap_or_else(|| self.core.allocate_pattern(CorePattern::Wildcard));
                    continuation = self.core.allocate_expression(CoreExpression::Lambda {
                        pattern,
                        body: continuation,
                    });
                }
                Some(DoStatement::Discard { .. }) => {
                    let pattern = self.core.allocate_pattern(CorePattern::Wildcard);
                    continuation = self.core.allocate_expression(CoreExpression::Lambda {
                        pattern,
                        body: continuation,
                    });
                }
                None => {}
            }
        }

        if actions.is_empty() {
            let mut function = self.elaborate_variable_resolution(pure);
            function = self.apply_evidence(EvidenceApplicationSite::AdoPure(source), function);
            self.apply(function, continuation)
        } else {
            let (first_statement, _, first_expression) = actions[0];
            let mut function = self.elaborate_variable_resolution(map);
            function = self.apply_evidence(
                EvidenceApplicationSite::Ado {
                    expression: source,
                    statement: first_statement,
                    argument: 0,
                },
                function,
            );
            function = self.apply(function, continuation);
            function = self.apply_evidence(
                EvidenceApplicationSite::Ado {
                    expression: source,
                    statement: first_statement,
                    argument: 1,
                },
                function,
            );
            let argument = self.elaborate_optional_expression(first_expression);
            let mut result = self.apply(function, argument);

            for &(statement, _, expression) in &actions[1..] {
                let mut function = self.elaborate_variable_resolution(apply);
                function = self.apply_evidence(
                    EvidenceApplicationSite::Ado { expression: source, statement, argument: 0 },
                    function,
                );
                result = self.apply_evidence(
                    EvidenceApplicationSite::AdoResult { expression: source, statement },
                    result,
                );
                function = self.apply(function, result);
                function = self.apply_evidence(
                    EvidenceApplicationSite::Ado { expression: source, statement, argument: 1 },
                    function,
                );
                let argument = self.elaborate_optional_expression(expression);
                result = self.apply(function, argument);
            }
            result
        }
    }

    fn elaborate_pattern(&mut self, source: BinderId) -> CorePatternId {
        let Some(kind) = self.input.lowered.info.get_binder_kind(source) else {
            let pattern = self.core.allocate_pattern(CorePattern::Error(CoreError::MissingPattern));
            self.record_source_pattern(source, pattern);
            return pattern;
        };

        let pattern = match kind {
            BinderKind::Typed { binder, .. }
            | BinderKind::Parenthesized { parenthesized: binder } => {
                binder.map(|binder| self.elaborate_pattern(binder)).unwrap_or_else(|| {
                    self.core.allocate_pattern(CorePattern::Error(CoreError::MissingPattern))
                })
            }
            BinderKind::OperatorChain { .. } => {
                let Some(tree) = self.input.bracketed.binders.get(&source) else {
                    return self.record_error_pattern(source, CoreError::MalformedOperator);
                };
                match tree {
                    Ok(tree) => self.elaborate_operator_pattern(tree),
                    Err(_) => self.record_error_pattern(source, CoreError::MalformedOperator),
                }
            }
            BinderKind::Integer { value } => {
                self.core.allocate_pattern(CorePattern::Literal(CoreLiteral::Integer(*value)))
            }
            BinderKind::Number { negative, value } => {
                self.core.allocate_pattern(CorePattern::Literal(CoreLiteral::Number {
                    negative: *negative,
                    value: value.clone(),
                }))
            }
            BinderKind::Constructor { resolution, arguments } => {
                let arguments =
                    arguments.iter().map(|&argument| self.elaborate_pattern(argument)).collect();
                self.core.allocate_pattern(CorePattern::Constructor {
                    constructor: *resolution,
                    arguments,
                })
            }
            BinderKind::Variable { .. } => {
                self.core.allocate_pattern(CorePattern::Variable(CoreVariable::Binder(source)))
            }
            BinderKind::Named { binder, .. } => {
                let pattern = binder
                    .map(|binder| self.elaborate_pattern(binder))
                    .unwrap_or_else(|| self.core.allocate_pattern(CorePattern::Wildcard));
                self.core.allocate_pattern(CorePattern::Named {
                    variable: CoreVariable::Binder(source),
                    pattern,
                })
            }
            BinderKind::Wildcard => self.core.allocate_pattern(CorePattern::Wildcard),
            BinderKind::String { kind, value } => {
                self.core.allocate_pattern(CorePattern::Literal(CoreLiteral::String {
                    kind: *kind,
                    value: value.clone(),
                }))
            }
            BinderKind::Char { value } => {
                self.core.allocate_pattern(CorePattern::Literal(CoreLiteral::Char(*value)))
            }
            BinderKind::Boolean { boolean } => {
                self.core.allocate_pattern(CorePattern::Literal(CoreLiteral::Boolean(*boolean)))
            }
            BinderKind::Array { array } => {
                let patterns = array.iter().map(|&id| self.elaborate_pattern(id)).collect();
                self.core.allocate_pattern(CorePattern::Array(patterns))
            }
            BinderKind::Record { record } => self.elaborate_record_pattern(record),
        };
        self.record_source_pattern(source, pattern);
        pattern
    }

    fn elaborate_operator_pattern(&mut self, tree: &OperatorTree<BinderId>) -> CorePatternId {
        match tree {
            OperatorTree::Leaf(pattern) => {
                pattern.map(|pattern| self.elaborate_pattern(pattern)).unwrap_or_else(|| {
                    self.core.allocate_pattern(CorePattern::Error(CoreError::MissingPattern))
                })
            }
            OperatorTree::Branch(operator, children) => {
                let constructor = self.input.checked.nodes.lookup_term_operator_target(*operator);
                let left = self.elaborate_operator_pattern(&children[0]);
                let right = self.elaborate_operator_pattern(&children[1]);
                self.core.allocate_pattern(CorePattern::Constructor {
                    constructor,
                    arguments: vec![left, right],
                })
            }
        }
    }

    fn elaborate_record_pattern(&mut self, record: &[BinderRecordItem]) -> CorePatternId {
        let mut fields = Vec::with_capacity(record.len());
        for field in record {
            match field {
                BinderRecordItem::RecordField { name, value } => {
                    let label = name.clone().map(CoreLabel::Source).unwrap_or(CoreLabel::Missing);
                    let pattern =
                        value.map(|value| self.elaborate_pattern(value)).unwrap_or_else(|| {
                            self.core
                                .allocate_pattern(CorePattern::Error(CoreError::MissingPattern))
                        });
                    fields.push(CoreRecordPatternField { label, pattern });
                }
                BinderRecordItem::RecordPun { id, name } => {
                    let label = name.clone().map(CoreLabel::Source).unwrap_or(CoreLabel::Missing);
                    let pattern = self
                        .core
                        .allocate_pattern(CorePattern::Variable(CoreVariable::RecordPun(*id)));
                    fields.push(CoreRecordPatternField { label, pattern });
                }
            }
        }
        self.core.allocate_pattern(CorePattern::Record(fields))
    }

    fn elaborate_patterns(&mut self, binders: &[BinderId], arity: usize) -> Vec<CorePatternId> {
        let mut patterns: Vec<_> =
            binders.iter().map(|&binder| self.elaborate_pattern(binder)).collect();
        while patterns.len() < arity {
            patterns.push(self.core.allocate_pattern(CorePattern::Wildcard));
        }
        patterns
    }

    fn case_with_fallback(
        &mut self,
        scrutinees: Vec<CoreExpressionId>,
        patterns: Vec<CorePatternId>,
        success: CoreExpressionId,
        failure: CoreExpressionId,
    ) -> CoreExpressionId {
        if scrutinees.is_empty() {
            debug_assert!(patterns.is_empty());
            return success;
        }

        let success = self.core.allocate_alternative(patterns, success);
        let wildcards = (0..scrutinees.len())
            .map(|_| self.core.allocate_pattern(CorePattern::Wildcard))
            .collect();
        let failure = self.core.allocate_alternative(wildcards, failure);
        self.core.allocate_expression(CoreExpression::Case {
            scrutinees,
            alternatives: vec![success, failure],
        })
    }

    fn bind_once(
        &mut self,
        values: Vec<CoreExpressionId>,
    ) -> (Option<CoreBindingGroupId>, Vec<CoreExpressionId>) {
        if values.is_empty() {
            return (None, Vec::new());
        }

        let mut bindings = Vec::with_capacity(values.len());
        let mut variables = Vec::with_capacity(values.len());
        for value in values {
            let variable = self.fresh_synthetic();
            let binding = CoreBinding {
                source: CoreBindingSource::Synthetic(variable),
                value: CoreBindingValue::Expression(value),
            };
            bindings.push(self.core.allocate_binding(binding));
            variables.push(
                self.core.allocate_expression(CoreExpression::Variable(CoreVariable::Synthetic(
                    variable,
                ))),
            );
        }
        let group =
            self.core.allocate_binding_group(CoreBindingGroup { recursive: false, bindings });
        (Some(group), variables)
    }

    fn apply_evidence(
        &mut self,
        site: EvidenceApplicationSite,
        mut function: CoreExpressionId,
    ) -> CoreExpressionId {
        if self.input.checked.placements.applications.contains_key(&site) {
            assert!(
                self.unconsumed_evidence_applications.remove(&site),
                "invariant violated: evidence application placement consumed more than once: {site:?}",
            );
        }
        let variables =
            self.input.checked.placements.applications.get(&site).cloned().unwrap_or_default();
        for variable in variables {
            let argument = self.resolve_evidence(variable);
            if let Some(argument) = argument {
                function = self.apply(function, argument);
            }
        }
        function
    }

    fn abstract_evidence(
        &mut self,
        site: EvidenceAbstractionSite,
        mut body: CoreExpressionId,
    ) -> CoreExpressionId {
        if self.input.checked.placements.abstractions.contains_key(&site) {
            assert!(
                self.unconsumed_evidence_abstractions.remove(&site),
                "invariant violated: evidence abstraction placement consumed more than once: {site:?}",
            );
        }
        let binders =
            self.input.checked.placements.abstractions.get(&site).cloned().unwrap_or_default();
        for binder in binders.into_iter().rev() {
            if self.input.checked.evidence.binder(binder).erased {
                continue;
            }
            body = self.abstract_evidence_binder(binder, body);
        }
        body
    }

    fn abstract_evidence_binder(
        &mut self,
        binder: EvidenceBinderId,
        body: CoreExpressionId,
    ) -> CoreExpressionId {
        let pattern =
            self.core.allocate_pattern(CorePattern::Variable(CoreVariable::Evidence(binder)));
        self.core.allocate_expression(CoreExpression::Lambda { pattern, body })
    }

    fn resolve_evidence(&mut self, variable: EvidenceVarId) -> Option<CoreExpressionId> {
        EvidenceResolver::new(&self.input.checked.evidence, &mut self.core)
            .resolve_variable(variable)
    }

    fn elaborate_optional_expression(
        &mut self,
        expression: Option<ExpressionId>,
    ) -> CoreExpressionId {
        expression
            .map(|expression| self.elaborate_expression(expression))
            .unwrap_or_else(|| self.error(CoreError::MissingExpression))
    }

    fn elaborate_resolution(
        &mut self,
        resolution: Option<(FileId, TermItemId)>,
    ) -> CoreExpressionId {
        resolution
            .map(|(file, item)| {
                self.core
                    .allocate_expression(CoreExpression::Variable(CoreVariable::Item(file, item)))
            })
            .unwrap_or_else(|| self.error(CoreError::MissingExpression))
    }

    fn elaborate_variable_resolution(
        &mut self,
        resolution: Option<TermVariableResolution>,
    ) -> CoreExpressionId {
        let variable = match resolution {
            Some(TermVariableResolution::Binder(id)) => CoreVariable::Binder(id),
            Some(TermVariableResolution::Let(id)) => CoreVariable::Let(id),
            Some(TermVariableResolution::RecordPun(id)) => CoreVariable::RecordPun(id),
            Some(TermVariableResolution::Reference(file, item)) => CoreVariable::Item(file, item),
            None => return self.error(CoreError::MissingExpression),
        };
        self.core.allocate_expression(CoreExpression::Variable(variable))
    }

    fn apply(
        &mut self,
        function: CoreExpressionId,
        argument: CoreExpressionId,
    ) -> CoreExpressionId {
        self.core.allocate_expression(CoreExpression::Apply { function, argument })
    }

    fn literal(&mut self, literal: CoreLiteral) -> CoreExpressionId {
        self.core.allocate_expression(CoreExpression::Literal(literal))
    }

    fn error(&mut self, error: CoreError) -> CoreExpressionId {
        self.core.allocate_expression(CoreExpression::Error(error))
    }

    fn fresh_synthetic(&mut self) -> u32 {
        let variable = self.next_synthetic;
        self.next_synthetic += 1;
        variable
    }

    fn record_source_expression(&mut self, source: ExpressionId, expression: CoreExpressionId) {
        self.core.expressions_by_source.insert(source, expression);
    }

    fn record_source_pattern(&mut self, source: BinderId, pattern: CorePatternId) {
        self.core.patterns_by_source.insert(source, pattern);
    }

    fn record_error_pattern(&mut self, source: BinderId, error: CoreError) -> CorePatternId {
        let pattern = self.core.allocate_pattern(CorePattern::Error(error));
        self.record_source_pattern(source, pattern);
        pattern
    }
}
