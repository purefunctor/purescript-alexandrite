use std::sync::Arc;

use building_types::QueryResult;
use files::FileId;
use indexing::TypeItemId;
use itertools::Itertools;
use rustc_hash::FxHashSet;

use crate::context::CheckContext;
use crate::state::CheckState;
use crate::{ExternalQueries, safe_loop};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fd {
    pub determiners: FxHashSet<usize>,
    pub determined: FxHashSet<usize>,
}

impl Fd {
    pub fn new(
        determiners: impl IntoIterator<Item = usize>,
        determined: impl IntoIterator<Item = usize>,
    ) -> Fd {
        Fd {
            determiners: determiners.into_iter().collect(),
            determined: determined.into_iter().collect(),
        }
    }

    pub fn from_lowering(functional_dependency: &lowering::FunctionalDependency) -> Fd {
        Fd::new(
            functional_dependency.determiners.iter().map(|&position| position as usize),
            functional_dependency.determined.iter().map(|&position| position as usize),
        )
    }
}

pub fn get_functional_dependencies<Q>(
    state: &CheckState,
    context: &CheckContext<Q>,
    file_id: FileId,
    type_id: TypeItemId,
) -> QueryResult<Arc<[Fd]>>
where
    Q: ExternalQueries,
{
    let functional_dependencies = if file_id == context.id {
        state.checked.classes.get(&type_id).map(|class| Arc::clone(&class.functional_dependencies))
    } else {
        let checked = context.queries.checked(file_id)?;
        checked.classes.get(&type_id).map(|class| Arc::clone(&class.functional_dependencies))
    };

    Ok(functional_dependencies.unwrap_or_default())
}

pub fn get_all_determined(functional_dependencies: &[Fd]) -> FxHashSet<usize> {
    functional_dependencies.iter().flat_map(|fd| fd.determined.iter().copied()).collect()
}

pub fn compute_closure(
    functional_dependencies: &[Fd],
    initial_positions: &FxHashSet<usize>,
) -> FxHashSet<usize> {
    let mut determined = initial_positions.clone();
    safe_loop! {
        let mut changed = false;

        for functional_dependency in functional_dependencies {
            if functional_dependency.determiners.is_subset(&determined) {
                for &position in &functional_dependency.determined {
                    if determined.insert(position) {
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            return determined;
        }
    }
}

pub fn compute_covering_sets(
    functional_dependencies: &[Fd],
    argument_count: usize,
) -> Vec<FxHashSet<usize>> {
    let all_argument_positions: FxHashSet<_> = (0..argument_count).collect();

    let all_covering_sets =
        argument_position_subsets(argument_count).into_iter().filter(|argument_positions| {
            let determined_positions = compute_closure(functional_dependencies, argument_positions);
            determined_positions == all_argument_positions
        });

    let all_covering_sets = all_covering_sets.collect_vec();

    let mut minimal_covering_sets = all_covering_sets.clone();
    minimal_covering_sets.retain(|covering_set| {
        !all_covering_sets.iter().any(|other_covering_set| {
            other_covering_set != covering_set && other_covering_set.is_subset(covering_set)
        })
    });

    minimal_covering_sets
}

fn argument_position_subsets(argument_count: usize) -> Vec<FxHashSet<usize>> {
    let mut subsets = vec![FxHashSet::default()];

    for argument_position in (0..argument_count).rev() {
        for index in 0..subsets.len() {
            let mut subset = subsets[index].clone();
            subset.insert(argument_position);
            subsets.push(subset);
        }
    }

    subsets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_fundeps() {
        let initial = FxHashSet::from_iter([0, 1]);
        let result = compute_closure(&[], &initial);
        assert_eq!(result, initial);
    }

    #[test]
    fn test_single_fundep() {
        let fundeps = vec![Fd::new([0], [1])];
        let initial = FxHashSet::from_iter([0]);
        let result = compute_closure(&fundeps, &initial);
        assert_eq!(result, FxHashSet::from_iter([0, 1]));
    }

    #[test]
    fn test_fundep_not_triggered() {
        let fundeps = vec![Fd::new([0], [1])];
        let initial = FxHashSet::from_iter([1]);
        let result = compute_closure(&fundeps, &initial);
        assert_eq!(result, FxHashSet::from_iter([1]));
    }

    #[test]
    fn test_transitive_fundeps() {
        let fundeps = vec![Fd::new([0], [1]), Fd::new([1], [2])];
        let initial = FxHashSet::from_iter([0]);
        let result = compute_closure(&fundeps, &initial);
        assert_eq!(result, FxHashSet::from_iter([0, 1, 2]));
    }

    #[test]
    fn test_multi_determiner() {
        let fundeps = vec![Fd::new([0, 1], [2])];

        let initial = FxHashSet::from_iter([0]);
        let result = compute_closure(&fundeps, &initial);
        assert_eq!(result, FxHashSet::from_iter([0]));

        let initial = FxHashSet::from_iter([0, 1]);
        let result = compute_closure(&fundeps, &initial);
        assert_eq!(result, FxHashSet::from_iter([0, 1, 2]));
    }

    #[test]
    fn test_multiple_determined() {
        let fundeps = vec![Fd::new([0], [1, 2])];
        let initial = FxHashSet::from_iter([0]);
        let result = compute_closure(&fundeps, &initial);
        assert_eq!(result, FxHashSet::from_iter([0, 1, 2]));
    }

    #[test]
    fn test_empty_determiners() {
        let fundeps = vec![Fd::new([], [0])];
        let initial: FxHashSet<usize> = FxHashSet::default();
        let result = compute_closure(&fundeps, &initial);
        assert_eq!(result, FxHashSet::from_iter([0]));
    }

    #[test]
    fn test_all_determined_no_fundeps() {
        let result = get_all_determined(&[]);
        assert_eq!(result, FxHashSet::default());
    }

    #[test]
    fn test_all_determined_single_fundep() {
        let fundeps = vec![Fd::new([0], [1])];
        let result = get_all_determined(&fundeps);
        assert_eq!(result, FxHashSet::from_iter([1]));
    }

    #[test]
    fn test_all_determined_multiple_fundeps() {
        let fundeps = vec![Fd::new([0], [1]), Fd::new([1], [2])];
        let result = get_all_determined(&fundeps);
        assert_eq!(result, FxHashSet::from_iter([1, 2]));
    }

    #[test]
    fn test_all_determined_overlapping() {
        let fundeps = vec![Fd::new([0], [1, 2]), Fd::new([3], [1])];
        let result = get_all_determined(&fundeps);
        assert_eq!(result, FxHashSet::from_iter([1, 2]));
    }

    #[test]
    fn test_all_determined_empty_determiners() {
        let fundeps = vec![Fd::new([], [0])];
        let result = get_all_determined(&fundeps);
        assert_eq!(result, FxHashSet::from_iter([0]));
    }

    #[test]
    fn test_covering_sets_no_fundeps() {
        let result = compute_covering_sets(&[], 2);
        assert_eq!(result, vec![FxHashSet::from_iter([0, 1])]);
    }

    #[test]
    fn test_covering_sets_single_fundep() {
        let fundeps = vec![Fd::new([0], [1])];
        let result = compute_covering_sets(&fundeps, 2);
        assert_eq!(result, vec![FxHashSet::from_iter([0])]);
    }

    #[test]
    fn test_covering_sets_symmetric_fundeps() {
        let fundeps = vec![Fd::new([0], [1]), Fd::new([1], [0])];
        let result = compute_covering_sets(&fundeps, 2);

        assert_eq!(result.len(), 2);
        assert!(result.contains(&FxHashSet::from_iter([0])));
        assert!(result.contains(&FxHashSet::from_iter([1])));
    }

    #[test]
    fn test_covering_sets_empty_determiner() {
        let fundeps = vec![Fd::new([], [0])];
        let result = compute_covering_sets(&fundeps, 1);
        assert_eq!(result, vec![FxHashSet::default()]);
    }
}
