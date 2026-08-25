//! Dependency ordering for JavaScript initializers.

use crate::error::UnsupportedState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

pub(super) fn visit_initializer(
    position: usize,
    dependencies: &[Vec<usize>],
    states: &mut [VisitState],
    ordered: &mut Vec<usize>,
) -> Result<(), UnsupportedState> {
    match states[position] {
        VisitState::Visited => return Ok(()),
        VisitState::Visiting => return Err(UnsupportedState::CyclicInitializers),
        VisitState::Unvisited => {}
    }
    states[position] = VisitState::Visiting;
    for dependency in &dependencies[position] {
        visit_initializer(*dependency, dependencies, states, ordered)?;
    }
    states[position] = VisitState::Visited;
    ordered.push(position);
    Ok(())
}
