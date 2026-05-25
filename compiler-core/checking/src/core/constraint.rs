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

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::fd::{compute_closure, get_functional_dependencies};
use crate::core::{KindOrType, TypeId, unification};
use crate::error::{CheckError, ErrorKind};
use crate::implication::{ImplicationId, Patterns};
use crate::state::{CheckState, UnificationState};

pub fn solve_implication<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
) -> QueryResult<Vec<CanonicalConstraintId>>
where
    Q: ExternalQueries,
{
    let implication = state.implications.current();
    let constraints = collect_scoped_constraints(state, context, implication)?;
    let constraints = solve_constraints(state, context, constraints)?;
    Ok(constraints.iter().map(|constraint| constraint.wanted).collect_vec())
}

pub struct Work {
    unifications: Vec<(TypeId, TypeId)>,
    unclassified: VecDeque<WantedInScope>,
    constraints: VecDeque<WantedInScope>,
}

impl Work {
    fn new(unclassified: VecDeque<WantedInScope>) -> Work {
        Work { unifications: vec![], unclassified, constraints: VecDeque::default() }
    }

    fn extend_from_parts<Unifications, Constraints>(
        &mut self,
        unifications: Unifications,
        constraints: Constraints,
    ) where
        Unifications: IntoIterator<Item = (TypeId, TypeId)>,
        Constraints: IntoIterator<Item = WantedInScope>,
    {
        self.unifications.extend(unifications);
        self.extend_constraints(constraints);
    }

    fn extend_constraints<Constraints>(&mut self, constraints: Constraints)
    where
        Constraints: IntoIterator<Item = WantedInScope>,
    {
        self.unclassified.extend(constraints);
    }

    fn pop_next<Q>(
        &mut self,
        state: &mut CheckState,
        context: &CheckContext<Q>,
    ) -> QueryResult<Option<WantedInScope>>
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

type Stuck = FxHashMap<u32, Vec<WantedInScope>>;
type ConstraintSet = IndexSet<WantedInScope, FxBuildHasher>;
type Skolem = Vec<WantedInScope>;

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

fn wake_constraints(work: &mut Work, stuck: &mut Stuck, state: &CheckState) {
    let mut awake = ConstraintSet::default();

    stuck.retain(|&id, constraints| {
        if let UnificationState::Solved(_) = state.unifications.get(id).state {
            awake.extend(constraints.iter().cloned());
            false
        } else {
            true
        }
    });

    if awake.is_empty() {
        return;
    }

    // For each constraint in the constraint set;
    for constraints in stuck.values_mut() {
        // keep only the constraints that are not awake;
        constraints.retain(|constraint| !awake.contains(constraint));
    }

    // and keep only the non-empty constraint sets.
    stuck.retain(|_, constraints| !constraints.is_empty());

    work.extend_constraints(awake);
}

fn solve_constraints<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    constraints: VecDeque<WantedInScope>,
) -> QueryResult<VecDeque<WantedInScope>>
where
    Q: ExternalQueries,
{
    let mut work = Work::new(constraints);
    let mut stuck = Stuck::default();
    let mut skolem = Skolem::default();
    let mut residuals = vec![];

    'work: loop {
        let mut has_unification = false;
        for (t1, t2) in mem::take(&mut work.unifications) {
            has_unification |= unification::unify(state, context, t1, t2)?;
        }

        if has_unification {
            wake_constraints(&mut work, &mut stuck, state);
            work.extend_constraints(mem::take(&mut skolem));
        }

        let Some(WantedInScope { given, wanted }) = work.pop_next(state, context)? else {
            break 'work;
        };

        let mut blocked = FxHashSet::default();
        let mut blocked_on_skolem = false;

        for &provided in &*given {
            match matching::match_provided(state, context, wanted, provided)? {
                matching::MatchInstance::Match { unifications, constraints } => {
                    let constraints = constraints.into_iter().map(|wanted| {
                        let given = Rc::clone(&given);
                        WantedInScope { given, wanted }
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

        match compiler::match_compiler_instance(state, context, wanted, &given)? {
            Some(matching::MatchInstance::Match { unifications, constraints }) => {
                let constraints = constraints.into_iter().map(|wanted| {
                    let given = Rc::clone(&given);
                    WantedInScope { given, wanted }
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

        let search = instances::collect_instance_chains(state, context, wanted)?;
        'chain: for chain in search.chains {
            for candidate in chain {
                match matching::match_declared(state, context, wanted, candidate)? {
                    matching::MatchInstance::Match { unifications, constraints } => {
                        let constraints = constraints.into_iter().map(|wanted| {
                            let given = Rc::clone(&given);
                            WantedInScope { given, wanted }
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

        let constraint = WantedInScope { given, wanted };
        if blocked.is_empty() {
            if blocked_on_skolem {
                skolem.push(constraint);
            } else {
                residuals.push(constraint);
            }
        } else {
            for id in blocked {
                let constraint = WantedInScope::clone(&constraint);
                stuck.entry(id).or_default().push(constraint);
            }
        }
    }

    let mut unique_residuals = ConstraintSet::default();
    unique_residuals.extend(residuals);

    for (_, constraints) in stuck {
        unique_residuals.extend(constraints);
    }
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
pub struct WantedInScope {
    given: Rc<[CanonicalConstraintId]>,
    wanted: CanonicalConstraintId,
}

fn collect_scoped_constraints<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    root: ImplicationId,
) -> QueryResult<VecDeque<WantedInScope>>
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
                state.checked.errors.push(CheckError { kind, crumbs });
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
                    constraints.push_back(WantedInScope { given, wanted });
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
