//! Checked Core semantic trees produced by source checking rules.

use std::sync::Arc;

use files::FileId;
use indexing::TermItemId;
use la_arena::{Arena, Idx};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

use crate::TypeId;
use crate::evidence::{EvidenceBinderId, EvidenceVarId};

pub type CheckedExpressionId = Idx<CheckedExpression>;
pub type CheckedBinderId = Idx<CheckedBinder>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CheckedCore {
    pub expressions: Arena<CheckedExpression>,
    pub binders: Arena<CheckedBinder>,
    pub term_roots: FxHashMap<TermItemId, CheckedExpressionId>,
    pub expressions_by_source: FxHashMap<lowering::ExpressionId, CheckedExpressionId>,
    pub binders_by_source: FxHashMap<lowering::BinderId, CheckedBinderId>,
}

impl CheckedCore {
    pub fn allocate_expression(
        &mut self,
        type_id: TypeId,
        kind: CheckedExpressionKind,
    ) -> CheckedExpressionId {
        let expression = CheckedExpression { type_id, kind };
        self.expressions.alloc(expression)
    }

    pub fn allocate_source_binder(
        &mut self,
        source: lowering::BinderId,
        type_id: TypeId,
        kind: CheckedBinderKind,
    ) -> CheckedBinderId {
        let binder = CheckedBinder { source: Some(source), type_id, kind };
        let checked = self.binders.alloc(binder);
        self.record_binder(source, checked);
        checked
    }

    pub fn allocate_synthesized_binder(
        &mut self,
        type_id: TypeId,
        kind: CheckedBinderKind,
    ) -> CheckedBinderId {
        let binder = CheckedBinder { source: None, type_id, kind };
        self.binders.alloc(binder)
    }

    pub fn record_expression(
        &mut self,
        source: lowering::ExpressionId,
        checked: CheckedExpressionId,
    ) {
        let previous = self.expressions_by_source.insert(source, checked);
        assert!(previous.is_none(), "invariant violated: source expression checked twice");
    }

    pub fn record_binder(&mut self, source: lowering::BinderId, checked: CheckedBinderId) {
        let previous = self.binders_by_source.insert(source, checked);
        assert!(previous.is_none(), "invariant violated: source binder checked twice");
    }

    pub fn record_term_root(&mut self, source: TermItemId, checked: CheckedExpressionId) {
        let previous = self.term_roots.insert(source, checked);
        assert!(previous.is_none(), "invariant violated: term root checked twice");
    }

    pub fn lookup_expression(&self, source: lowering::ExpressionId) -> Option<CheckedExpressionId> {
        self.expressions_by_source.get(&source).copied()
    }

    pub fn lookup_binder(&self, source: lowering::BinderId) -> Option<CheckedBinderId> {
        self.binders_by_source.get(&source).copied()
    }

    pub fn lookup_term_root(&self, source: TermItemId) -> Option<CheckedExpressionId> {
        self.term_roots.get(&source).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedExpression {
    pub type_id: TypeId,
    pub kind: CheckedExpressionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedExpressionKind {
    Variable { resolution: lowering::TermVariableResolution },
    Constructor { file_id: FileId, item_id: TermItemId },
    Literal { literal: CheckedLiteral },
    Case { scrutinees: Arc<[CheckedExpressionId]>, alternatives: Arc<[CheckedCaseAlternative]> },
    Lambda { binders: Arc<[CheckedBinderId]>, expression: CheckedExpressionId },
    TermApplication { function: CheckedExpressionId, argument: CheckedExpressionId },
    TypeApplication { function: CheckedExpressionId, argument: TypeId },
    EvidenceApplication { expression: CheckedExpressionId, evidence: Arc<[EvidenceVarId]> },
    EvidenceAbstraction { binders: Arc<[EvidenceBinderId]>, expression: CheckedExpressionId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedCaseAlternative {
    pub binders: Arc<[CheckedBinderId]>,
    pub results: Arc<[CheckedGuardedExpression]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedGuardedExpression {
    pub guards: Arc<[CheckedPatternGuard]>,
    pub expression: CheckedExpressionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedPatternGuard {
    Boolean { expression: CheckedExpressionId },
    Pattern { binder: CheckedBinderId, expression: CheckedExpressionId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedLiteral {
    String { kind: lowering::StringKind, value: Option<SmolStr> },
    Char(Option<char>),
    Boolean(bool),
    Integer(Option<i32>),
    Number(Option<SmolStr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedBinder {
    /// The canonical allocation origin; additional lowering aliases live in `binders_by_source`.
    pub source: Option<lowering::BinderId>,
    pub type_id: TypeId,
    pub kind: CheckedBinderKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedBinderKind {
    Variable,
    Named { binder: CheckedBinderId },
    Wildcard,
    Constructor { file_id: FileId, item_id: TermItemId, arguments: Arc<[CheckedBinderId]> },
}
