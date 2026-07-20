use std::sync::Arc;

use building_types::QueryResult;
use files::FileId;
use indexing::TypeItemId;
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
        let checked = context.checked_dependency(file_id)?;
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

/// Closure monotonicity makes this equivalent to `positions` containing a covering set.
/// Exact equality preserves behavior when a malformed dependency introduces out-of-range positions.
pub fn positions_cover_all(
    functional_dependencies: &[Fd],
    positions: &FxHashSet<usize>,
    all_positions: &FxHashSet<usize>,
) -> bool {
    compute_closure(functional_dependencies, positions) == *all_positions
}

#[cfg(test)]
mod tests;
