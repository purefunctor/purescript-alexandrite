//! Functional abstraction and parameter classification.

use nbe::tree::{ExpressionKind, Parameter, PatternId, PatternKind, Storage};

pub(super) fn is_abstraction(kind: &ExpressionKind) -> bool {
    matches!(kind, ExpressionKind::Abstraction { .. } | ExpressionKind::UncurriedAbstraction { .. })
}

pub(super) fn pattern_parameter(storage: &Storage, pattern: PatternId) -> Option<&Parameter> {
    match &storage[pattern].kind {
        PatternKind::Variable(parameter) | PatternKind::Named { parameter, .. } => Some(parameter),
        PatternKind::Wildcard
        | PatternKind::Literal(_)
        | PatternKind::Array(_)
        | PatternKind::Record(_)
        | PatternKind::Constructor { .. } => None,
    }
}
