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
use std::hash::{Hash, Hasher};
use std::mem;
use std::rc::Rc;

use building_types::QueryResult;
use indexmap::IndexSet;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};

use crate::context::CheckContext;
use crate::core::fd::{compute_closure, get_functional_dependencies};
use crate::core::{KindOrType, TypeId, unification};
use crate::error::{CheckingError, ErrorKind};
use crate::evidence::{Evidence, EvidenceId, EvidenceVarId};
use crate::implication::{GivenConstraint, ImplicationId, Patterns, WantedConstraint};
use crate::state::{CheckState, UnificationState};
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

type Stuck = FxHashMap<u32, Vec<ConstraintInScope>>;
type ConstraintSet = IndexSet<ConstraintInScope, FxBuildHasher>;
type Skolem = Vec<ConstraintInScope>;

fn insert_unique_constraint(
    state: &mut CheckState,
    constraints: &mut ConstraintSet,
    constraint: ConstraintInScope,
) {
    if let Some(existing) = constraints.get(&constraint) {
        state.checked.evidence.merge_duplicate(constraint.evidence, existing.evidence);
    } else {
        constraints.insert(constraint);
    }
}

fn scoped_constraints<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    given: &Rc<[(CanonicalConstraintId, EvidenceId)]>,
    scope: ImplicationId,
    constraints: Vec<CanonicalConstraintId>,
) -> Vec<ConstraintInScope>
where
    Q: ExternalQueries,
{
    constraints
        .into_iter()
        .map(|wanted| {
            let evidence = state.checked.evidence.fresh_variable();
            let canonical = state.canonicals.type_id(context, wanted);
            state.checked.evidence.bind_variable(evidence, canonical);
            let given = Rc::clone(given);
            ConstraintInScope { given, wanted, evidence, scope }
        })
        .collect()
}

fn canonical_givens(given: &[(CanonicalConstraintId, EvidenceId)]) -> Vec<CanonicalConstraintId> {
    given.iter().map(|&(constraint, _)| constraint).collect()
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

fn wake_constraints(work: &mut Work, stuck: &mut Stuck, state: &mut CheckState) {
    let mut awake = ConstraintSet::default();

    stuck.retain(|&id, constraints| {
        if let UnificationState::Solved(_) = state.unifications.get(id).state {
            for constraint in constraints.iter().cloned() {
                insert_unique_constraint(state, &mut awake, constraint);
            }
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
        constraints.retain(|constraint| {
            if let Some(existing) = awake.get(constraint) {
                state.checked.evidence.merge_duplicate(constraint.evidence, existing.evidence);
                false
            } else {
                true
            }
        });
    }

    // and keep only the non-empty constraint sets.
    stuck.retain(|_, constraints| !constraints.is_empty());

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

    'work: loop {
        let mut has_unification = false;
        for (t1, t2) in mem::take(&mut work.unifications) {
            has_unification |= unification::unify(state, context, t1, t2)?;
        }

        if has_unification {
            wake_constraints(&mut work, &mut stuck, state);
            work.extend_constraints(mem::take(&mut skolem));
        }

        let Some(constraint) = work.pop_next(state, context)? else {
            break 'work;
        };

        let mut blocked = FxHashSet::default();
        let mut blocked_on_skolem = false;

        for &(provided, provided_evidence) in &*constraint.given {
            match matching::match_provided(state, context, constraint.wanted, provided)? {
                matching::MatchInstance::Match { unifications, constraints } => {
                    let evidence = if compiler::is_compiler_known_constraint(
                        state,
                        context,
                        constraint.wanted,
                    ) {
                        state.checked.evidence.compiler()
                    } else {
                        provided_evidence
                    };
                    state.checked.evidence.solve(constraint.evidence, evidence);
                    let constraints = scoped_constraints(
                        state,
                        context,
                        &constraint.given,
                        constraint.scope,
                        constraints,
                    );
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

        let given = canonical_givens(&constraint.given);
        match compiler::match_compiler_instance(state, context, constraint.wanted, &given)? {
            Some(matching::MatchInstance::Match { unifications, constraints }) => {
                if compiler::is_compiler_error_constraint(state, context, constraint.wanted) {
                    state.checked.evidence.mark_error(constraint.evidence);
                } else {
                    let evidence = state.checked.evidence.compiler();
                    state.checked.evidence.solve(constraint.evidence, evidence);
                }
                let constraints = scoped_constraints(
                    state,
                    context,
                    &constraint.given,
                    constraint.scope,
                    constraints,
                );
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
                        let constraints = scoped_constraints(
                            state,
                            context,
                            &constraint.given,
                            constraint.scope,
                            constraints,
                        );
                        let subgoals =
                            constraints.iter().map(|constraint| constraint.evidence).collect();
                        let evidence = state
                            .checked
                            .evidence
                            .allocate(Evidence::Instance { origin: candidate.origin, subgoals });
                        state.checked.evidence.solve(constraint.evidence, evidence);
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
            for id in blocked {
                let constraint = ConstraintInScope::clone(&constraint);
                stuck.entry(id).or_default().push(constraint);
            }
        }
    }

    let mut unique_residuals = ConstraintSet::default();
    for residual in residuals {
        insert_unique_constraint(state, &mut unique_residuals, residual);
    }

    for (_, constraints) in stuck {
        for constraint in constraints {
            insert_unique_constraint(state, &mut unique_residuals, constraint);
        }
    }
    for constraint in skolem {
        insert_unique_constraint(state, &mut unique_residuals, constraint);
    }

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
    constraints: Vec<(CanonicalConstraintId, EvidenceId)>,
    seen: IndexSet<CanonicalConstraintId, FxBuildHasher>,
    scope: ImplicationId,
}

impl GivenInScope {
    fn insert(&mut self, constraint: CanonicalConstraintId, evidence: EvidenceId) {
        if self.seen.insert(constraint) {
            self.constraints.push((constraint, evidence));
        } else if let Some((_, existing)) =
            self.constraints.iter_mut().find(|(given, _)| *given == constraint)
        {
            // A local duplicate shadows the inherited dictionary. Canonical
            // identity remains deduplicated, but evidence must be lexical.
            *existing = evidence;
        }
    }

    fn contains(&self, constraint: CanonicalConstraintId) -> bool {
        self.seen.contains(&constraint)
    }
}

#[derive(Clone)]
pub struct ConstraintInScope {
    pub given: Rc<[(CanonicalConstraintId, EvidenceId)]>,
    pub wanted: CanonicalConstraintId,
    pub evidence: EvidenceVarId,
    /// Identity of the lexical dictionary environment used by this wanted.
    /// This prevents solver work deduplication from aliasing evidence across
    /// sibling scopes while keeping fresh evidence IDs out of canonical keys.
    scope: ImplicationId,
}

impl PartialEq for ConstraintInScope {
    fn eq(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.wanted == other.wanted
            && self.given.len() == other.given.len()
            && self
                .given
                .iter()
                .zip(other.given.iter())
                .all(|((left, _), (right, _))| left == right)
    }
}

impl Eq for ConstraintInScope {}

impl Hash for ConstraintInScope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.scope.hash(state);
        for (given, _) in self.given.iter() {
            given.hash(state);
        }
        self.wanted.hash(state);
    }
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
            .map(|&(given, evidence)| {
                canonical::zonk_canonical(state, context, given).map(|given| (given, evidence))
            })
            .collect::<QueryResult<Rc<[_]>>>()?;
        let wanted = canonical::zonk_canonical(state, context, self.wanted)?;
        let canonical = state.canonicals.type_id(context, wanted);
        state.checked.evidence.bind_variable(self.evidence, canonical);
        Ok(ConstraintInScope { given, wanted, evidence: self.evidence, scope: self.scope })
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

        let mut introduced_given = false;
        for GivenConstraint { constraint, evidence } in given {
            if let Some(given) = canonical::canonicalise(state, context, constraint)? {
                introduced_given = true;
                let erased = compiler::is_compiler_known_constraint(state, context, given);
                let canonical = state.canonicals.type_id(context, given);
                state.checked.evidence.bind_binder(evidence, canonical);
                if erased {
                    state.checked.evidence.erase_binder(evidence);
                }
                let evidence = state.checked.evidence.given(evidence);
                implication_given.insert(given, evidence);
            }
        }

        if introduced_given {
            implication_given.scope = implication_id;
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

            // Functional-dependency improvement may substitute rigid names in
            // a direct given. Keep its durable binder metadata aligned with
            // the canonical constraint the solver actually uses.
            for &(constraint, evidence) in &given {
                let binder = match state.checked.evidence.evidence(evidence) {
                    Evidence::Given(binder) => Some(*binder),
                    _ => None,
                };
                if let Some(binder) = binder {
                    let canonical = state.canonicals.type_id(context, constraint);
                    state.checked.evidence.bind_binder(binder, canonical);
                    if compiler::is_compiler_known_constraint(state, context, constraint) {
                        state.checked.evidence.erase_binder(binder);
                    }
                }
            }

            let given: Rc<[(CanonicalConstraintId, EvidenceId)]> = Rc::from(given);

            for WantedConstraint { constraint, evidence } in wanted {
                if let Some(wanted) = canonical::canonicalise(state, context, constraint)? {
                    let given = Rc::clone(&given);
                    let wanted =
                        canonical::substitute_canonical(state, context, &substitution, wanted)?;
                    let canonical = state.canonicals.type_id(context, wanted);
                    state.checked.evidence.bind_variable(evidence, canonical);
                    let scope = implication_given.scope;
                    constraints.push_back(ConstraintInScope { given, wanted, evidence, scope });
                } else {
                    state.checked.evidence.mark_error(evidence);
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
