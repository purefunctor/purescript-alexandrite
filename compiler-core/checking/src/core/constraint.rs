//! Implements the constraint solver for PureScript.
//!
//! The [`solve_implication`] function is the entrypoint for solving the root
//! [`Implication`]; the [`solve_implication_id`] function solves a single
//! [`Implication`] with respect to inheritance; and the [`solve_constraints`]
//! function implements the equality-driven constraint solver.
//!
//! [`Implication`]: crate::implication::Implication

pub mod canonical;
pub mod compiler;
pub mod elaborate;
pub mod instances;
pub mod matching;

pub use canonical::{CanonicalConstraint, CanonicalConstraintId, Canonicals};
use itertools::Itertools;

use std::collections::VecDeque;
use std::mem;
use std::rc::Rc;

use building_types::QueryResult;
use indexmap::IndexSet;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};

use crate::context::CheckContext;
use crate::core::fd::{compute_closure, get_functional_dependencies};
use crate::core::{KindOrType, TypeId, unification};
use crate::error::{CheckingError, ErrorKind};
use crate::implication::{ImplicationId, Patterns};
use crate::state::CheckState;
use crate::{ExternalQueries, Type};

pub fn solve_implication<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
) -> QueryResult<Vec<ConstraintInScope>>
where
    Q: ExternalQueries,
{
    let implication = state.implications.current();
    let constraints = collect_constraints(state, context, implication)?;
    solve_constraints(state, context, constraints)
}

pub struct Work {
    unifications: Vec<(TypeId, TypeId)>,
    unclassified: VecDeque<ConstraintInScope>,
    constraints: VecDeque<ConstraintInScope>,
}

impl Work {
    fn new(unclassified: VecDeque<ConstraintInScope>) -> Work {
        Work { unifications: vec![], unclassified, constraints: VecDeque::default() }
    }

    fn extend_from_parts<Unifications, Constraints>(
        &mut self,
        unifications: Unifications,
        constraints: Constraints,
    ) where
        Unifications: IntoIterator<Item = (TypeId, TypeId)>,
        Constraints: IntoIterator<Item = ConstraintInScope>,
    {
        self.unifications.extend(unifications);
        self.extend_constraints(constraints);
    }

    fn extend_constraints<Constraints>(&mut self, constraints: Constraints)
    where
        Constraints: IntoIterator<Item = ConstraintInScope>,
    {
        self.unclassified.extend(constraints);
    }

    fn pop_next<Q>(
        &mut self,
        state: &mut CheckState,
        context: &CheckContext<Q>,
    ) -> QueryResult<Option<ConstraintInScope>>
    where
        Q: ExternalQueries,
    {
        while let Some(constraint) = self.unclassified.pop_front() {
            if is_improving_constraint(state, context, constraint.wanted)? {
                return Ok(Some(constraint));
            }

            self.constraints.push_back(constraint);
        }

        Ok(self.constraints.pop_front())
    }
}

#[derive(Default)]
struct Stuck {
    by_unification: FxHashMap<u32, Vec<ConstraintInScope>>,
    by_constraint: FxHashMap<ConstraintInScope, StuckConstraint>,
    next_order: usize,
    entries: usize,
}

struct StuckConstraint {
    blockers: FxHashSet<u32>,
    order: usize,
}

impl Stuck {
    fn register(
        &mut self,
        constraint: ConstraintInScope,
        blockers: FxHashSet<u32>,
        statistics: &mut WakeStatistics,
    ) {
        let order = self.next_order;
        self.next_order += 1;

        let entry = self
            .by_constraint
            .entry(constraint.clone())
            .or_insert_with(|| StuckConstraint { blockers: FxHashSet::default(), order });

        for blocker in blockers {
            if !entry.blockers.insert(blocker) {
                statistics.duplicate_registrations += 1;
                continue;
            }

            self.by_unification.entry(blocker).or_default().push(constraint.clone());
            self.entries += 1;
        }

        statistics.peak_stuck_entries = statistics.peak_stuck_entries.max(self.entries);
    }

    fn wake(
        &mut self,
        solved: Vec<u32>,
        statistics: &mut WakeStatistics,
    ) -> Vec<ConstraintInScope> {
        let mut awake = FxHashSet::default();

        for id in solved {
            statistics.solved_ids_examined += 1;
            statistics.stuck_keys_scanned += 1;

            let Some(constraints) = self.by_unification.remove(&id) else { continue };
            statistics.stuck_entries_scanned += constraints.len();
            awake.extend(constraints);
        }

        let mut awake = awake
            .into_iter()
            .filter_map(|constraint| {
                let entry = self.by_constraint.remove(&constraint)?;
                self.entries -= entry.blockers.len();

                for blocker in &entry.blockers {
                    let Some(constraints) = self.by_unification.get_mut(blocker) else { continue };
                    statistics.stuck_entries_scanned += constraints.len();
                    constraints.retain(|registered| registered != &constraint);
                    if constraints.is_empty() {
                        self.by_unification.remove(blocker);
                    }
                }

                Some((entry.order, constraint))
            })
            .collect_vec();
        awake.sort_unstable_by_key(|(order, _)| *order);
        statistics.awakened_constraints += awake.len();

        let awake = awake.into_iter().map(|(_, constraint)| constraint);
        awake.collect()
    }

    fn into_constraints(self) -> Vec<ConstraintInScope> {
        let mut constraints = ConstraintSet::default();
        for registered in self.by_unification.into_values() {
            constraints.extend(registered);
        }
        constraints.into_iter().collect()
    }
}

type ConstraintSet = IndexSet<ConstraintInScope, FxBuildHasher>;
type Skolem = Vec<ConstraintInScope>;

#[derive(Default)]
struct WakeStatistics {
    wake_calls: usize,
    solved_ids_examined: usize,
    stuck_keys_scanned: usize,
    stuck_entries_scanned: usize,
    duplicate_registrations: usize,
    awakened_constraints: usize,
    peak_stuck_entries: usize,
}

impl Drop for WakeStatistics {
    fn drop(&mut self) {
        if std::env::var_os("ALEXANDRITE_PROFILE_CONSTRAINT_WAKE").is_some() {
            eprintln!(
                "constraint-wake wake_calls={} solved_ids_examined={} stuck_keys_scanned={} stuck_entries_scanned={} duplicate_registrations={} awakened_constraints={} peak_stuck_entries={}",
                self.wake_calls,
                self.solved_ids_examined,
                self.stuck_keys_scanned,
                self.stuck_entries_scanned,
                self.duplicate_registrations,
                self.awakened_constraints,
                self.peak_stuck_entries,
            );
        }
    }
}

fn is_improving_constraint<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    constraint: CanonicalConstraintId,
) -> QueryResult<bool>
where
    Q: ExternalQueries,
{
    let CanonicalConstraint { file_id, type_id, arguments } = state.canonicals[constraint].clone();
    let functional_dependencies = get_functional_dependencies(state, context, file_id, type_id)?;

    if functional_dependencies.is_empty() {
        return Ok(false);
    }

    let arguments = arguments.iter().filter_map(|argument| {
        if let KindOrType::Type(argument) = argument { Some(*argument) } else { None }
    });

    let arguments = arguments.collect_vec();

    let mut known_positions = FxHashSet::default();
    let mut blocking_by_position = vec![];

    for (position, &argument) in arguments.iter().enumerate() {
        let blocking = matching::collect_blocking(state, context, &[argument])?;
        if blocking.is_empty() {
            known_positions.insert(position);
        }
        blocking_by_position.push(blocking);
    }

    let closure = compute_closure(&functional_dependencies, &known_positions);
    for position in closure.difference(&known_positions) {
        if blocking_by_position.get(*position).is_some_and(|blocking| !blocking.is_empty()) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn wake_constraints(
    work: &mut Work,
    stuck: &mut Stuck,
    solved: Vec<u32>,
    statistics: &mut WakeStatistics,
) {
    statistics.wake_calls += 1;
    let awake = stuck.wake(solved, statistics);
    work.extend_constraints(awake);
}

fn solve_constraints<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    constraints: VecDeque<ConstraintInScope>,
) -> QueryResult<Vec<ConstraintInScope>>
where
    Q: ExternalQueries,
{
    let mut work = Work::new(constraints);
    let mut stuck = Stuck::default();
    let mut skolem = Skolem::default();
    let mut residuals = vec![];
    let mut wake_statistics = WakeStatistics::default();
    state.unifications.take_solved();

    'work: loop {
        for (t1, t2) in mem::take(&mut work.unifications) {
            unification::unify(state, context, t1, t2)?;
        }

        let solved = state.unifications.take_solved();
        if !solved.is_empty() {
            wake_constraints(&mut work, &mut stuck, solved, &mut wake_statistics);
            work.extend_constraints(mem::take(&mut skolem));
        }

        let Some(constraint) = work.pop_next(state, context)? else {
            break 'work;
        };

        let mut blocked = FxHashSet::default();
        let mut blocked_on_skolem = false;

        for &provided in &*constraint.given {
            match matching::match_provided(state, context, constraint.wanted, provided)? {
                matching::MatchInstance::Match { unifications, constraints } => {
                    let constraints = constraints.into_iter().map(|wanted| {
                        let given = Rc::clone(&constraint.given);
                        ConstraintInScope { given, wanted }
                    });
                    work.extend_from_parts(unifications, constraints);
                    continue 'work;
                }
                matching::MatchInstance::Stuck { stuck, skolem } => {
                    blocked.extend(stuck);
                    blocked_on_skolem |= skolem;
                }
                matching::MatchInstance::Apart => (),
            }
        }

        match compiler::match_compiler_instance(
            state,
            context,
            constraint.wanted,
            &constraint.given,
        )? {
            Some(matching::MatchInstance::Match { unifications, constraints }) => {
                let constraints = constraints.into_iter().map(|wanted| {
                    let given = Rc::clone(&constraint.given);
                    ConstraintInScope { given, wanted }
                });
                work.extend_from_parts(unifications, constraints);
                continue 'work;
            }
            Some(matching::MatchInstance::Stuck { stuck, skolem }) => {
                blocked.extend(stuck);
                blocked_on_skolem |= skolem;
            }
            Some(matching::MatchInstance::Apart) | None => (),
        }

        let search = instances::collect_instance_chains(state, context, constraint.wanted)?;
        'chain: for chain in search.chains {
            for candidate in chain {
                match matching::match_declared(state, context, constraint.wanted, candidate)? {
                    matching::MatchInstance::Match { unifications, constraints } => {
                        let constraints = constraints.into_iter().map(|wanted| {
                            let given = Rc::clone(&constraint.given);
                            ConstraintInScope { given, wanted }
                        });
                        work.extend_from_parts(unifications, constraints);
                        continue 'work;
                    }
                    matching::MatchInstance::Stuck { stuck, skolem } => {
                        blocked.extend(stuck);
                        blocked_on_skolem |= skolem;
                        continue 'chain;
                    }
                    matching::MatchInstance::Apart => (),
                }
            }
        }

        // If no candidate matched, the candidate search itself may also be incomplete
        // due to unsolved unification variables; we will wait for them to be solved.
        blocked.extend(search.blocking);

        if blocked.is_empty() {
            if blocked_on_skolem {
                skolem.push(constraint);
            } else {
                residuals.push(constraint);
            }
        } else {
            stuck.register(constraint, blocked, &mut wake_statistics);
        }
    }

    let mut unique_residuals = ConstraintSet::default();
    unique_residuals.extend(residuals);

    unique_residuals.extend(stuck.into_constraints());
    unique_residuals.extend(skolem);

    Ok(unique_residuals.into_iter().collect())
}

pub fn is_type_error<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    constraint: TypeId,
) -> QueryResult<bool>
where
    Q: ExternalQueries,
{
    let Some(canonical) = canonical::canonicalise(state, context, constraint)? else {
        return Ok(false);
    };

    let canonical = &state.canonicals[canonical];
    Ok(canonical.file_id == context.prim_type_error.file_id
        && (canonical.type_id == context.prim_type_error.warn
            || canonical.type_id == context.prim_type_error.fail))
}

#[derive(Default, Clone)]
struct GivenInScope {
    constraints: Vec<CanonicalConstraintId>,
    seen: IndexSet<CanonicalConstraintId, FxBuildHasher>,
}

impl GivenInScope {
    fn insert(&mut self, constraint: CanonicalConstraintId) {
        if self.seen.insert(constraint) {
            self.constraints.push(constraint);
        }
    }

    fn contains(&self, constraint: CanonicalConstraintId) -> bool {
        self.seen.contains(&constraint)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConstraintInScope {
    pub given: Rc<[CanonicalConstraintId]>,
    pub wanted: CanonicalConstraintId,
}

impl ConstraintInScope {
    pub fn zonk<Q>(
        &self,
        state: &mut CheckState,
        context: &CheckContext<Q>,
    ) -> QueryResult<ConstraintInScope>
    where
        Q: ExternalQueries,
    {
        let given = self
            .given
            .iter()
            .map(|&given| canonical::zonk_canonical(state, context, given))
            .collect::<QueryResult<Rc<[_]>>>()?;
        let wanted = canonical::zonk_canonical(state, context, self.wanted)?;
        Ok(ConstraintInScope { given, wanted })
    }

    pub fn canonical<Q>(&self, state: &CheckState, context: &CheckContext<Q>) -> TypeId
    where
        Q: ExternalQueries,
    {
        state.canonicals.type_id(context, self.wanted)
    }

    pub fn is_partial<Q>(&self, state: &CheckState, context: &CheckContext<Q>) -> bool
    where
        Q: ExternalQueries,
    {
        let Type::Constructor(file_id, type_id) = context.lookup_type(context.prim.partial) else {
            unreachable!("critical violation: Partial is not Partial");
        };
        let constraint = &state.canonicals[self.wanted];
        constraint.file_id == file_id
            && constraint.type_id == type_id
            && constraint.arguments.is_empty()
    }
}

fn collect_constraints<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    root: ImplicationId,
) -> QueryResult<VecDeque<ConstraintInScope>>
where
    Q: ExternalQueries,
{
    let partial = canonical::canonicalise(state, context, context.prim.partial)?;

    let mut constraints = VecDeque::default();
    let mut stack = vec![(root, GivenInScope::default())];

    while let Some((implication_id, mut implication_given)) = stack.pop() {
        let (given, wanted, children, patterns) = {
            let implication = &mut state.implications[implication_id];
            (
                mem::take(&mut implication.given),
                mem::take(&mut implication.wanted),
                mem::take(&mut implication.children),
                mem::take(&mut implication.patterns),
            )
        };

        for given in given {
            if let Some(given) = canonical::canonicalise(state, context, given)? {
                implication_given.insert(given);
            }
        }

        let elide_missing_patterns =
            partial.is_some_and(|partial| implication_given.contains(partial));

        if !elide_missing_patterns {
            for Patterns { patterns, crumbs } in patterns {
                let kind = ErrorKind::MissingPatterns { patterns };
                state.checked.errors.push(CheckingError { kind, crumbs });
            }
        }

        if !wanted.is_empty() {
            let elaborate::ElaboratedGiven { given, substitution } =
                elaborate::elaborate_given(state, context, &implication_given.constraints)?;

            let given: Rc<[CanonicalConstraintId]> = Rc::from(given);

            for wanted in wanted {
                if let Some(wanted) = canonical::canonicalise(state, context, wanted)? {
                    let given = Rc::clone(&given);
                    let wanted =
                        canonical::substitute_canonical(state, context, &substitution, wanted)?;
                    constraints.push_back(ConstraintInScope { given, wanted });
                }
            }
        }

        let children = children
            .into_iter()
            .rev()
            .map(|child| (child, GivenInScope::clone(&implication_given)));

        stack.extend(children)
    }

    Ok(constraints)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::{
        CanonicalConstraintId, ConstraintInScope, Stuck, WakeStatistics, Work, wake_constraints,
    };
    use rustc_hash::FxHashSet;

    fn constraint(id: u32) -> ConstraintInScope {
        let wanted = CanonicalConstraintId::new(NonZeroU32::new(id).unwrap());
        ConstraintInScope { given: [].into(), wanted }
    }

    #[test]
    fn many_blockers_only_examine_solved_wait_lists() {
        const BLOCKERS: u32 = 1_000;

        let mut stuck = Stuck::default();
        let mut statistics = WakeStatistics::default();

        for blocker in 0..BLOCKERS {
            stuck.register(
                constraint(blocker + 1),
                FxHashSet::from_iter([blocker]),
                &mut statistics,
            );
        }

        let mut work = Work::new(Default::default());
        for blocker in (0..BLOCKERS).rev() {
            wake_constraints(&mut work, &mut stuck, vec![blocker], &mut statistics);
            assert!(work.unclassified.pop_front() == Some(constraint(blocker + 1)));
        }

        assert_eq!(statistics.wake_calls, BLOCKERS as usize);
        assert_eq!(statistics.solved_ids_examined, BLOCKERS as usize);
        assert_eq!(statistics.stuck_keys_scanned, BLOCKERS as usize);
        assert_eq!(statistics.stuck_entries_scanned, BLOCKERS as usize);
        assert_eq!(statistics.awakened_constraints, BLOCKERS as usize);
        assert_eq!(statistics.peak_stuck_entries, BLOCKERS as usize);
        assert!(stuck.into_constraints().is_empty());
    }

    #[test]
    fn multiple_solved_blockers_requeue_each_constraint_once_in_registration_order() {
        let first = constraint(1);
        let second = constraint(2);
        let mut stuck = Stuck::default();
        let mut statistics = WakeStatistics::default();

        stuck.register(first.clone(), FxHashSet::from_iter([10, 20]), &mut statistics);
        stuck.register(second.clone(), FxHashSet::from_iter([20]), &mut statistics);
        stuck.register(first.clone(), FxHashSet::from_iter([10, 20]), &mut statistics);

        let mut work = Work::new(Default::default());
        wake_constraints(&mut work, &mut stuck, vec![20, 10], &mut statistics);
        let awake = work.unclassified.into_iter().collect::<Vec<_>>();

        assert!(awake == [first, second]);
        assert_eq!(statistics.wake_calls, 1);
        assert_eq!(statistics.duplicate_registrations, 2);
        assert_eq!(statistics.awakened_constraints, 2);
        assert!(stuck.into_constraints().is_empty());
    }
}
