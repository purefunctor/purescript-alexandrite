//! Implements the algorithm's core state structures.

use std::mem;
use std::sync::Arc;

use building_types::QueryResult;
use files::FileId;
use rustc_hash::FxHashMap;

use crate::context::CheckContext;
use crate::core::constraint::{CanonicalConstraintId, Canonicals, ConstraintInScope};
use crate::core::exhaustive::{
    ExhaustivenessReport, Pattern, PatternConstructor, PatternId, PatternInterner, PatternKind,
};
use crate::core::substitute::{NameToType, SubstituteName};
use crate::core::{Depth, Name, SmolStrId, Type, TypeId, constraint};
use crate::error::{CheckingError, ErrorCrumb, ErrorKind};
use crate::evidence::{EvidenceAbstractionSite, EvidenceBinderId, WantedCollector};
use crate::implication::{GivenConstraint, Implications, Patterns};
use crate::{CheckedModule, ExternalQueries};

/// Manages [`Name`] values for [`CheckState`].
pub struct Names {
    unique: u32,
    file: FileId,
}

impl Names {
    pub fn new(file: FileId) -> Names {
        Names { unique: 0, file }
    }

    pub fn fresh(&mut self) -> Name {
        let unique = self.unique;
        self.unique += 1;
        Name { file: self.file, unique }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnificationState {
    Unsolved,
    Solved(TypeId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnificationEntry {
    pub depth: Depth,
    pub kind: TypeId,
    pub state: UnificationState,
}

/// Manages unification variables for [`CheckState`].
#[derive(Debug, Default)]
pub struct Unifications {
    entries: Vec<UnificationEntry>,
    unique: u32,
}

impl Unifications {
    pub fn fresh(&mut self, depth: Depth, kind: TypeId) -> u32 {
        let unique = self.unique;

        self.unique += 1;
        self.entries.push(UnificationEntry { depth, kind, state: UnificationState::Unsolved });

        unique
    }

    pub fn get(&self, index: u32) -> &UnificationEntry {
        &self.entries[index as usize]
    }

    pub fn get_mut(&mut self, index: u32) -> &mut UnificationEntry {
        &mut self.entries[index as usize]
    }

    pub fn solve(&mut self, index: u32, solution: TypeId) {
        self.get_mut(index).state = UnificationState::Solved(solution);
    }

    pub fn iter(&self) -> impl Iterator<Item = &UnificationEntry> {
        self.entries.iter()
    }
}

/// Tracks type variable bindings during kind inference.
#[derive(Default)]
pub struct Bindings {
    forall: Vec<ForallBinding>,
    implicit: Vec<ImplicitBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ForallBinding {
    id: lowering::TypeVariableBindingId,
    name: Name,
    kind: TypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImplicitBinding {
    node: lowering::GraphNodeId,
    id: lowering::ImplicitBindingId,
    name: Name,
    kind: TypeId,
}

impl Bindings {
    pub fn bind_forall(&mut self, id: lowering::TypeVariableBindingId, name: Name, kind: TypeId) {
        self.forall.push(ForallBinding { id, name, kind });
    }

    pub fn lookup_forall(&self, id: lowering::TypeVariableBindingId) -> Option<(Name, TypeId)> {
        self.forall
            .iter()
            .rev()
            .find(|binding| binding.id == id)
            .map(|binding| (binding.name, binding.kind))
    }

    pub fn bind_implicit(
        &mut self,
        node: lowering::GraphNodeId,
        id: lowering::ImplicitBindingId,
        name: Name,
        kind: TypeId,
    ) {
        self.implicit.push(ImplicitBinding { node, id, name, kind });
    }

    fn bind_implicit_substitution<Q>(
        state: &mut CheckState,
        context: &CheckContext<Q>,
        substitution: &NameToType,
    ) -> QueryResult<()>
    where
        Q: ExternalQueries,
    {
        let scope = state.bindings.implicit.len();

        for binding in 0..scope {
            let ImplicitBinding { node, id, name, kind } = state.bindings.implicit[binding];
            let Some(&replacement) = substitution.get(&name) else { continue };

            let Type::Rigid(name, _, _) = context.lookup_type(replacement) else {
                unreachable!("invariant violated: expected a rigid variable");
            };

            let kind = SubstituteName::many(state, context, substitution, kind)?;
            state.bindings.implicit.push(ImplicitBinding { node, id, name, kind });
        }

        Ok(())
    }

    fn bind_forall_substitution<Q>(
        state: &mut CheckState,
        context: &CheckContext<Q>,
        substitution: &NameToType,
    ) -> QueryResult<()>
    where
        Q: ExternalQueries,
    {
        let scope = state.bindings.forall.len();

        for binding in 0..scope {
            let ForallBinding { id, name, kind } = state.bindings.forall[binding];
            let Some(&replacement) = substitution.get(&name) else { continue };

            let Type::Rigid(name, _, _) = context.lookup_type(replacement) else {
                unreachable!("invariant violated: expected a rigid variable");
            };

            let kind = SubstituteName::many(state, context, substitution, kind)?;
            state.bindings.forall.push(ForallBinding { id, name, kind });
        }

        Ok(())
    }

    pub fn lookup_implicit(
        &self,
        node: lowering::GraphNodeId,
        id: lowering::ImplicitBindingId,
    ) -> Option<(Name, TypeId)> {
        self.implicit
            .iter()
            .rev()
            .find(|binding| binding.node == node && binding.id == id)
            .map(|binding| (binding.name, binding.kind))
    }
}

/// The core state structure threaded through the algorithm.
pub struct CheckState {
    pub checked: CheckedModule,

    pub names: Names,
    pub bindings: Bindings,
    pub patterns: PatternInterner,

    pub unifications: Unifications,
    pub implications: Implications,
    pub canonicals: Canonicals,
    pub canonical_errors: FxHashMap<CanonicalConstraintId, Vec<ErrorKind>>,

    pub defer_expansion: bool,
    pub depth: Depth,

    pub crumbs: Vec<ErrorCrumb>,

    binder_captures: Vec<(EvidenceAbstractionSite, Vec<EvidenceBinderId>)>,
}

impl CheckState {
    pub fn new(file_id: FileId) -> CheckState {
        CheckState {
            checked: Default::default(),
            names: Names::new(file_id),
            bindings: Default::default(),
            patterns: Default::default(),
            unifications: Default::default(),
            implications: Default::default(),
            canonicals: Default::default(),
            canonical_errors: Default::default(),
            defer_expansion: Default::default(),
            depth: Depth(0),
            crumbs: Default::default(),
            binder_captures: Default::default(),
        }
    }

    pub fn with_depth<T>(&mut self, f: impl FnOnce(&mut CheckState) -> T) -> T {
        let depth = self.depth.increment();

        let previous = mem::replace(&mut self.depth, depth);
        let result = f(self);
        self.depth = previous;

        result
    }

    pub fn with_defer_expansion<T>(&mut self, f: impl FnOnce(&mut CheckState) -> T) -> T {
        let previous = mem::replace(&mut self.defer_expansion, true);
        let result = f(self);
        self.defer_expansion = previous;
        result
    }

    pub fn with_error_crumb<F, T>(&mut self, crumb: ErrorCrumb, f: F) -> T
    where
        F: FnOnce(&mut CheckState) -> T,
    {
        self.crumbs.push(crumb);
        let result = f(self);
        self.crumbs.pop();
        result
    }

    pub fn fresh_unification(&mut self, queries: &impl ExternalQueries, kind: TypeId) -> TypeId {
        let unification = self.unifications.fresh(self.depth, kind);
        queries.intern_type(Type::Unification(unification))
    }

    pub fn fresh_rigid(&mut self, queries: &impl ExternalQueries, kind: TypeId) -> TypeId {
        self.fresh_rigid_named(queries, kind, None)
    }

    pub fn fresh_rigid_named(
        &mut self,
        queries: &impl ExternalQueries,
        kind: TypeId,
        text: Option<SmolStrId>,
    ) -> TypeId {
        let name = self.names.fresh();
        if let Some(text) = text {
            self.checked.names.insert(name, text);
        }
        queries.intern_type(Type::Rigid(name, self.depth, kind))
    }

    pub fn insert_error(&mut self, kind: ErrorKind) {
        let crumbs = self.crumbs.iter().copied().collect();
        self.checked.errors.push(CheckingError { kind, crumbs });
    }

    pub fn push_given(&mut self, constraint: TypeId) -> EvidenceBinderId {
        let evidence = self.fresh_evidence_binder();
        self.push_given_with_evidence(constraint, evidence);
        evidence
    }

    pub fn push_given_with_evidence(&mut self, constraint: TypeId, evidence: EvidenceBinderId) {
        self.implications.current_mut().given.push(GivenConstraint { constraint, evidence });
    }

    pub fn fresh_evidence_binder(&mut self) -> EvidenceBinderId {
        let evidence = self.checked.evidence.fresh_binder();
        if let Some((_, binders)) = self.binder_captures.last_mut() {
            binders.push(evidence);
        }
        evidence
    }

    pub fn with_wanted_collector<T>(
        &mut self,
        mut collector: WantedCollector,
        f: impl FnOnce(&mut CheckState, &mut WantedCollector) -> T,
    ) -> T {
        f(self, &mut collector)
    }

    pub fn capture_binders<T>(
        &mut self,
        site: EvidenceAbstractionSite,
        f: impl FnOnce(&mut CheckState) -> T,
    ) -> T {
        self.binder_captures.push((site, vec![]));
        let result = f(self);
        let (captured_site, binders) = self
            .binder_captures
            .pop()
            .expect("invariant violated: missing binder evidence capture");
        debug_assert_eq!(captured_site, site);

        if !binders.is_empty() {
            self.checked.placements.abstractions.entry(site).or_default().extend(binders);
        }

        result
    }

    pub fn with_implication<T>(&mut self, f: impl FnOnce(&mut CheckState) -> T) -> T {
        let id = self.implications.push();
        let result = f(self);
        self.implications.pop(id);
        result
    }

    pub fn with_implicit<Q, T>(
        &mut self,
        context: &CheckContext<Q>,
        substitution: &NameToType,
        f: impl FnOnce(&mut CheckState) -> QueryResult<T>,
    ) -> QueryResult<T>
    where
        Q: ExternalQueries,
    {
        let forall_scope = self.bindings.forall.len();
        Bindings::bind_forall_substitution(self, context, substitution)?;
        let scope = self.bindings.implicit.len();
        Bindings::bind_implicit_substitution(self, context, substitution)?;
        let result = f(self);
        self.bindings.implicit.truncate(scope);
        self.bindings.forall.truncate(forall_scope);

        result
    }

    pub fn solve_constraints<Q>(
        &mut self,
        context: &CheckContext<Q>,
    ) -> QueryResult<Vec<ConstraintInScope>>
    where
        Q: ExternalQueries,
    {
        constraint::solve_implication(self, context)
    }

    pub fn report_exhaustiveness(&mut self, exhaustiveness: ExhaustivenessReport) {
        if let Some(patterns) = exhaustiveness.missing {
            let crumbs = self.crumbs.iter().copied().collect();
            let patterns = Patterns { patterns: Arc::from(patterns), crumbs };
            self.implications.current_mut().patterns.push(patterns);
        }

        if !exhaustiveness.redundant.is_empty() {
            let patterns = Arc::from(exhaustiveness.redundant);
            self.insert_error(ErrorKind::RedundantPatterns { patterns });
        }
    }

    pub fn allocate_pattern(&mut self, kind: PatternKind, t: TypeId) -> PatternId {
        let pattern = Pattern { kind, t };
        self.patterns.intern(pattern)
    }

    pub fn allocate_constructor(
        &mut self,
        constructor: PatternConstructor,
        t: TypeId,
    ) -> PatternId {
        let kind = PatternKind::Constructor { constructor };
        self.allocate_pattern(kind, t)
    }

    pub fn allocate_wildcard(&mut self, t: TypeId) -> PatternId {
        self.allocate_pattern(PatternKind::Wildcard, t)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::CheckState;
    use crate::TypeId;
    use crate::evidence::{EvidenceAbstractionSite, EvidenceApplicationSite, WantedCollector};

    #[test]
    fn nested_wanted_collectors_record_only_their_own_sites() {
        let mut files = files::Files::default();
        let mut state = CheckState::new(files.insert("Test.purs", ""));
        let constraint = TypeId::new(NonZeroU32::new(1).unwrap());
        let outer_expression = lowering::ExpressionId::new(NonZeroU32::new(1).unwrap());
        let inner_expression = lowering::ExpressionId::new(NonZeroU32::new(2).unwrap());
        let outer_application = EvidenceApplicationSite::Expression(outer_expression);
        let inner_application = EvidenceApplicationSite::Expression(inner_expression);

        let (outer_first, inner, outer_second) = state.with_wanted_collector(
            WantedCollector::application(outer_application),
            |state, outer| {
                let outer_first = outer.collect(state, constraint);
                let inner = state.with_wanted_collector(
                    WantedCollector::application(inner_application),
                    |state, inner| inner.collect(state, constraint),
                );
                let outer_second = outer.collect(state, constraint);
                (outer_first, inner, outer_second)
            },
        );

        assert_eq!(
            state.checked.placements.applications[&outer_application],
            [outer_first, outer_second]
        );
        assert_eq!(state.checked.placements.applications[&inner_application], [inner]);

        let outer_abstraction = EvidenceAbstractionSite::Expression(outer_expression);
        let inner_abstraction = EvidenceAbstractionSite::Expression(inner_expression);
        let (outer_first, inner, outer_second) =
            state.capture_binders(outer_abstraction, |state| {
                let outer_first = state.fresh_evidence_binder();
                let inner =
                    state.capture_binders(inner_abstraction, |state| state.fresh_evidence_binder());
                let outer_second = state.fresh_evidence_binder();
                (outer_first, inner, outer_second)
            });

        assert_eq!(
            state.checked.placements.abstractions[&outer_abstraction],
            [outer_first, outer_second]
        );
        assert_eq!(state.checked.placements.abstractions[&inner_abstraction], [inner]);
    }
}
