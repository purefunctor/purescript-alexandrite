use std::sync::Arc;

use building_types::QueryResult;
use checking::evidence::{
    Evidence, EvidenceBinderId, EvidenceId, EvidenceState, EvidenceVarId, InstanceCandidateOrigin,
};
use rustc_hash::{FxHashMap, FxHashSet};
use smol_str::{SmolStr, format_smolstr};

use crate::error::UnsupportedState;
use crate::tree::{
    Binding, ExpressionId, ExpressionKind, Parameter, ReflectableEvidence, ReflectableOrdering,
    SynthesizedEvidence,
};

use super::{BindingSource, Context, ConversionResult, lowercase_initial};

const MAX_EVIDENCE_NAME_FRAGMENTS: usize = 4;

#[derive(Default)]
pub(super) struct EvidenceScope {
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

pub(super) fn evidence_variable(
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
