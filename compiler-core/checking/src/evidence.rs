//! Type class evidence produced by constraint solving.
//!
//! Evidence variables identify wanted occurrences rather than canonical
//! constraints. Multiple occurrences of the same constraint therefore remain
//! distinguishable even when the solver deduplicates their work.

use crate::TypeId;
pub use crate::core::constraint::instances::InstanceCandidateOrigin;
use crate::implication::WantedConstraint;
use crate::state::CheckState;
use indexing::{DeriveId, InstanceId, TermItemId};
use lowering::{DoStatementId, ExpressionId, LetBindingNameGroupId, RecordPunId, TermOperatorId};
use rustc_hash::FxHashMap;

/// A solver-written evidence variable for one wanted occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceVarId(pub u32);

/// An identifier for a term in a module's evidence table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceId(pub u32);

/// A dictionary parameter introduced by a given or generalised constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceBinderId(pub u32);

/// An explicit capability for introducing wanted constraints.
///
/// The capability records what will consume the resulting evidence. Passing it
/// through constraint-producing functions makes evidence placement visible in
/// their signatures without borrowing or wrapping [`crate::state::CheckState`].
#[derive(Debug, PartialEq, Eq)]
pub struct WantedCollector {
    destination: WantedDestination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WantedDestination {
    Application(EvidenceApplicationSite),
    DerivedRequirement(DeriveId),
    InstanceSuperclass(InstanceId),
    Compiler,
    NonCollecting,
}

impl WantedCollector {
    pub fn application(site: EvidenceApplicationSite) -> WantedCollector {
        WantedCollector { destination: WantedDestination::Application(site) }
    }

    pub fn derived_requirement(derive: DeriveId) -> WantedCollector {
        WantedCollector { destination: WantedDestination::DerivedRequirement(derive) }
    }

    pub fn instance_superclass(instance: InstanceId) -> WantedCollector {
        WantedCollector { destination: WantedDestination::InstanceSuperclass(instance) }
    }

    /// Evidence for a compiler-known constraint which is erased from Core.
    pub fn compiler() -> WantedCollector {
        WantedCollector { destination: WantedDestination::Compiler }
    }

    /// Evidence which is solved during checking but not collected for Core.
    pub fn non_collecting() -> WantedCollector {
        WantedCollector { destination: WantedDestination::NonCollecting }
    }

    pub fn collect(&mut self, state: &mut CheckState, constraint: TypeId) -> EvidenceVarId {
        let evidence = state.checked.evidence.fresh_variable();
        self.record(&mut state.checked.placements, evidence);
        let implications = state.implications.current_mut();
        implications.wanted.push_back(WantedConstraint { constraint, evidence });
        evidence
    }

    fn record(&mut self, placements: &mut EvidencePlacements, variable: EvidenceVarId) {
        match self.destination {
            WantedDestination::Application(site) => {
                placements.applications.entry(site).or_default().push(variable);
            }
            WantedDestination::DerivedRequirement(derive) => {
                placements.derived_requirements.entry(derive).or_default().push(variable);
            }
            WantedDestination::InstanceSuperclass(instance) => {
                placements.instance_superclasses.entry(instance).or_default().push(variable);
            }
            WantedDestination::Compiler | WantedDestination::NonCollecting => {}
        }
    }
}

/// A semantic application boundary where solved evidence is passed to a term.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceApplicationSite {
    Expression(ExpressionId),
    Application { expression: ExpressionId, argument: u32 },
    Operator(TermOperatorId),
    OperatorResult(TermOperatorId),
    Infix { expression: ExpressionId, pair: u32, argument: u8 },
    Negate(ExpressionId),
    RecordPun(RecordPunId),
    RecordAccess { expression: ExpressionId, label: u32 },
    Do { expression: ExpressionId, statement: DoStatementId, argument: u8 },
    DoResult { expression: ExpressionId, statement: DoStatementId },
    Ado { expression: ExpressionId, statement: DoStatementId, argument: u8 },
    AdoResult { expression: ExpressionId, statement: DoStatementId },
    AdoPure(ExpressionId),
}

/// A semantic abstraction boundary where dictionary parameters are introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceAbstractionSite {
    Expression(ExpressionId),
    Operator(TermOperatorId),
    RecordPun(RecordPunId),
    /// Local proof assumptions used inside a compiler-generated dictionary.
    Derived(DeriveId),
    Term(TermItemId),
    Let(LetBindingNameGroupId),
    InstanceMember {
        instance: InstanceId,
        member: u32,
    },
}

/// Evidence arguments and parameters associated with semantic Core boundaries.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EvidencePlacements {
    pub applications: FxHashMap<EvidenceApplicationSite, Vec<EvidenceVarId>>,
    pub abstractions: FxHashMap<EvidenceAbstractionSite, Vec<EvidenceBinderId>>,
    pub instance_superclasses: FxHashMap<InstanceId, Vec<EvidenceVarId>>,
    /// Solved dictionaries required by a compiler-generated derived instance.
    pub derived_requirements: FxHashMap<DeriveId, Vec<EvidenceVarId>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Unsolved,
    Solved(EvidenceId),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceEntry {
    /// The latest canonical form known for this wanted occurrence.
    pub constraint: Option<TypeId>,
    pub state: EvidenceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceBinder {
    /// The canonical constraint represented by this dictionary parameter.
    pub constraint: Option<TypeId>,
    /// Whether the dictionary is a compiler-only proof with no runtime argument.
    pub erased: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    /// Indirection from a deduplicated wanted occurrence to its representative.
    Variable(EvidenceVarId),
    /// A dictionary parameter in scope.
    Given(EvidenceBinderId),
    /// A declared or derived instance applied to its prerequisite dictionaries.
    Instance { origin: InstanceCandidateOrigin, subgoals: Vec<EvidenceVarId> },
    /// A superclass dictionary projected from another dictionary.
    Superclass { parent: EvidenceId, index: usize },
    /// Evidence discharged by compiler-known machinery and erased downstream.
    Compiler,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Evidences {
    variables: Vec<EvidenceEntry>,
    binders: Vec<EvidenceBinder>,
    terms: Vec<Evidence>,
}

impl Evidences {
    pub fn fresh_variable(&mut self) -> EvidenceVarId {
        let id = EvidenceVarId(self.variables.len() as u32);
        let entry = EvidenceEntry { constraint: None, state: EvidenceState::Unsolved };
        self.variables.push(entry);
        id
    }

    pub fn fresh_binder(&mut self) -> EvidenceBinderId {
        let id = EvidenceBinderId(self.binders.len() as u32);
        self.binders.push(EvidenceBinder { constraint: None, erased: false });
        id
    }

    pub fn bind_variable(&mut self, id: EvidenceVarId, constraint: TypeId) {
        self.variables[id.0 as usize].constraint = Some(constraint);
    }

    pub fn bind_binder(&mut self, id: EvidenceBinderId, constraint: TypeId) {
        self.binders[id.0 as usize].constraint = Some(constraint);
    }

    pub fn erase_binder(&mut self, id: EvidenceBinderId) {
        self.binders[id.0 as usize].erased = true;
    }

    pub fn allocate(&mut self, evidence: Evidence) -> EvidenceId {
        let id = EvidenceId(self.terms.len() as u32);
        self.terms.push(evidence);
        id
    }

    pub fn given(&mut self, binder: EvidenceBinderId) -> EvidenceId {
        self.allocate(Evidence::Given(binder))
    }

    pub fn compiler(&mut self) -> EvidenceId {
        self.allocate(Evidence::Compiler)
    }

    pub fn solve(&mut self, id: EvidenceVarId, evidence: EvidenceId) {
        let entry = &mut self.variables[id.0 as usize];
        debug_assert!(
            matches!(entry.state, EvidenceState::Unsolved),
            "invariant violated: evidence variable solved more than once",
        );
        if matches!(entry.state, EvidenceState::Unsolved) {
            entry.state = EvidenceState::Solved(evidence);
        }
    }

    pub fn merge_duplicate(&mut self, duplicate: EvidenceVarId, representative: EvidenceVarId) {
        if duplicate == representative {
            return;
        }

        match self.variables[duplicate.0 as usize].state {
            EvidenceState::Unsolved => {}
            EvidenceState::Solved(term)
                if self.terms[term.0 as usize] == Evidence::Variable(representative) =>
            {
                return;
            }
            EvidenceState::Solved(_) | EvidenceState::Error => {
                debug_assert!(
                    false,
                    "invariant violated: resolved evidence occurrence deduplicated"
                );
                return;
            }
        }

        let evidence = self.allocate(Evidence::Variable(representative));
        self.solve(duplicate, evidence);
    }

    pub fn mark_error(&mut self, id: EvidenceVarId) {
        let entry = &mut self.variables[id.0 as usize];
        match entry.state {
            EvidenceState::Unsolved => entry.state = EvidenceState::Error,
            EvidenceState::Error => {}
            EvidenceState::Solved(_) => {
                debug_assert!(
                    false,
                    "invariant violated: solved evidence variable marked as errored"
                );
            }
        }
    }

    pub fn variable(&self, id: EvidenceVarId) -> &EvidenceEntry {
        &self.variables[id.0 as usize]
    }

    pub fn binder(&self, id: EvidenceBinderId) -> &EvidenceBinder {
        &self.binders[id.0 as usize]
    }

    pub fn evidence(&self, id: EvidenceId) -> &Evidence {
        &self.terms[id.0 as usize]
    }

    pub fn variables(&self) -> impl Iterator<Item = (EvidenceVarId, &EvidenceEntry)> {
        self.variables.iter().enumerate().map(|(index, entry)| (EvidenceVarId(index as u32), entry))
    }

    pub fn binders(&self) -> impl Iterator<Item = (EvidenceBinderId, &EvidenceBinder)> {
        self.binders
            .iter()
            .enumerate()
            .map(|(index, binder)| (EvidenceBinderId(index as u32), binder))
    }

    pub fn assert_finished(&self) {
        debug_assert!(
            self.variables().all(|(_, entry)| {
                matches!(entry.state, EvidenceState::Solved(_) | EvidenceState::Error)
            }),
            "invariant violated: unsolved evidence variables remain",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{Evidence, EvidenceState, Evidences};

    #[test]
    fn compiler_known_binders_retain_erasure_metadata() {
        let mut evidence = Evidences::default();
        let binder = evidence.fresh_binder();

        evidence.erase_binder(binder);

        assert!(evidence.binder(binder).erased);
    }

    #[test]
    fn duplicate_occurrences_retain_indirection() {
        let mut evidence = Evidences::default();
        let representative = evidence.fresh_variable();
        let duplicate = evidence.fresh_variable();

        evidence.merge_duplicate(duplicate, representative);

        let EvidenceState::Solved(term) = evidence.variable(duplicate).state else {
            panic!("duplicate evidence was not solved");
        };
        assert_eq!(evidence.evidence(term), &Evidence::Variable(representative));

        evidence.merge_duplicate(duplicate, representative);
    }
}
