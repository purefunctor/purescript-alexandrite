use checking::TypeId;
use checking::evidence::{EvidenceBinderId, InstanceCandidateOrigin};
use files::FileId;
use indexing::{TermItemId, TypeItemId};
use la_arena::{Arena, Idx};
use lowering::{BinderId, ExpressionId, LetBindingNameGroupId, RecordPunId, StringKind};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

pub type CoreExpressionId = Idx<CoreExpression>;
pub type CorePatternId = Idx<CorePattern>;
pub type CoreAlternativeId = Idx<CoreAlternative>;
pub type CoreBindingId = Idx<CoreBinding>;
pub type CoreBindingGroupId = Idx<CoreBindingGroup>;

/// A module-local Core program.
///
/// Core nodes deliberately have no type field. Checked types remain available
/// as side-table facts, which keeps recovery nodes and desugaring-generated
/// nodes from requiring invented checker identities.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CoreModule {
    pub expressions: Arena<CoreExpression>,
    pub patterns: Arena<CorePattern>,
    pub alternatives: Arena<CoreAlternative>,
    pub bindings: Arena<CoreBinding>,
    pub binding_groups: Arena<CoreBindingGroup>,

    /// Top-level groups in dependency order.
    pub top_level: Vec<CoreBindingGroupId>,
    /// Every grouped source term item points at its Core binding.
    pub items: FxHashMap<TermItemId, CoreBindingId>,
    /// Dictionary origins resolve directly to their module-local definitions.
    pub instances: FxHashMap<InstanceCandidateOrigin, CoreBindingId>,
    pub lets: FxHashMap<LetBindingNameGroupId, CoreBindingId>,

    /// Source expressions alias their final elaborated node. Parentheses and
    /// type annotations allocate no Core wrapper; any distinct final node is
    /// solely a checker-requested evidence application or abstraction.
    pub expressions_by_source: FxHashMap<ExpressionId, CoreExpressionId>,
    pub expressions_by_core: FxHashMap<CoreExpressionId, ExpressionId>,
    /// Checked source facts remain keyed by source identity. Synthetic Core
    /// applications deliberately do not claim checker-produced types.
    pub expression_types: FxHashMap<ExpressionId, TypeId>,
    pub pun_types: FxHashMap<RecordPunId, TypeId>,
    pub patterns_by_source: FxHashMap<BinderId, CorePatternId>,
    pub pattern_types: FxHashMap<BinderId, TypeId>,
    pub binding_types: FxHashMap<CoreBindingId, TypeId>,
}

impl CoreModule {
    pub fn allocate_expression(&mut self, kind: CoreExpression) -> CoreExpressionId {
        self.expressions.alloc(kind)
    }

    pub fn allocate_pattern(&mut self, kind: CorePattern) -> CorePatternId {
        self.patterns.alloc(kind)
    }

    pub fn allocate_alternative(
        &mut self,
        patterns: Vec<CorePatternId>,
        body: CoreExpressionId,
    ) -> CoreAlternativeId {
        self.alternatives.alloc(CoreAlternative { patterns, body })
    }

    pub fn allocate_binding(&mut self, binding: CoreBinding) -> CoreBindingId {
        self.bindings.alloc(binding)
    }

    pub fn allocate_binding_group(&mut self, group: CoreBindingGroup) -> CoreBindingGroupId {
        self.binding_groups.alloc(group)
    }

    pub fn lookup_expression(&self, source: ExpressionId) -> Option<CoreExpressionId> {
        self.expressions_by_source.get(&source).copied()
    }
}

/// Core expressions contain only semantic forms. In particular, source
/// annotations, parentheses, operator chains, do notation, and equations have
/// already disappeared before a value enters this arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreExpression {
    Variable(CoreVariable),
    Literal(CoreLiteral),
    Lambda {
        pattern: CorePatternId,
        body: CoreExpressionId,
    },
    Apply {
        function: CoreExpressionId,
        argument: CoreExpressionId,
    },
    TypeApply {
        function: CoreExpressionId,
        argument: CoreTypeArgument,
    },
    Let {
        group: CoreBindingGroupId,
        body: CoreExpressionId,
    },
    Case {
        scrutinees: Vec<CoreExpressionId>,
        alternatives: Vec<CoreAlternativeId>,
    },
    IfThenElse {
        condition: CoreExpressionId,
        then: CoreExpressionId,
        else_: CoreExpressionId,
    },
    Array(Vec<CoreExpressionId>),
    Record(Vec<CoreRecordField>),
    Dictionary {
        superclasses: Vec<CoreSuperclassField>,
        members: Vec<CoreRecordField>,
    },
    /// A compiler-generated dictionary. The checked requirements are already
    /// solved; a backend implements the class-specific stock/newtype primitive
    /// without re-running constraint solving.
    DerivedDictionary {
        strategy: CoreDeriveStrategy,
        class: Option<(FileId, TypeItemId)>,
        /// Method-local proof assumptions (for example the `Eq a` assumed
        /// while validating an `Eq1 f` delegate) used by `requirements`.
        local_binders: Vec<CoreDerivedBinder>,
        requirements: Vec<CoreDerivedRequirement>,
    },
    Access {
        record: CoreExpressionId,
        label: CoreLabel,
    },
    Update {
        record: CoreExpressionId,
        updates: Vec<CoreRecordUpdate>,
    },
    SuperclassProjection {
        dictionary: CoreExpressionId,
        index: usize,
    },
    Error(CoreError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreTypeArgument {
    Checked(TypeId),
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreVariable {
    Binder(BinderId),
    Synthetic(u32),
    Let(LetBindingNameGroupId),
    RecordPun(RecordPunId),
    Item(FileId, TermItemId),
    Evidence(EvidenceBinderId),
    Instance(InstanceCandidateOrigin),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreLiteral {
    String { kind: StringKind, value: Option<SmolStr> },
    Char(Option<char>),
    Boolean(bool),
    Integer(Option<i32>),
    Number { negative: bool, value: Option<SmolStr> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorePattern {
    Variable(CoreVariable),
    Literal(CoreLiteral),
    Constructor { constructor: Option<(FileId, TermItemId)>, arguments: Vec<CorePatternId> },
    Named { variable: CoreVariable, pattern: CorePatternId },
    Wildcard,
    Array(Vec<CorePatternId>),
    Record(Vec<CoreRecordPatternField>),
    Error(CoreError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreRecordPatternField {
    pub label: CoreLabel,
    pub pattern: CorePatternId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreAlternative {
    pub patterns: Vec<CorePatternId>,
    pub body: CoreExpressionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreBinding {
    pub source: CoreBindingSource,
    pub value: CoreBindingValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreBindingSource {
    Item(TermItemId),
    Let(LetBindingNameGroupId),
    Synthetic(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreBindingValue {
    Expression(CoreExpressionId),
    External(CoreExternalBinding),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreExternalBinding {
    ClassMember,
    Constructor,
    Foreign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreBindingGroup {
    pub recursive: bool,
    pub bindings: Vec<CoreBindingId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreRecordField {
    pub label: CoreLabel,
    pub value: CoreExpressionId,
}

/// A class dictionary keeps declaration-order superclass slots explicit.
/// Compiler-known proofs have no runtime expression, but retaining their slot
/// keeps superclass projection indices stable for the remaining dictionaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreSuperclassField {
    Runtime(CoreExpressionId),
    Erased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDeriveStrategy {
    Stock,
    Newtype,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreDerivedBinder {
    pub binder: EvidenceBinderId,
    pub constraint: Option<TypeId>,
    pub erased: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreDerivedRequirement {
    /// The final checker-canonicalized constraint for this generated use.
    pub constraint: Option<TypeId>,
    pub evidence: CoreDerivedEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDerivedEvidence {
    Runtime(CoreExpressionId),
    Erased,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreRecordUpdate {
    pub path: Vec<CoreLabel>,
    pub value: CoreExpressionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreLabel {
    Source(SmolStr),
    Item(FileId, TermItemId),
    Missing,
}

/// Recovery is explicit and limited to malformed or already-errored input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreError {
    /// A source-level typed hole, retained as deliberate recovery syntax.
    Hole,
    MissingExpression,
    MissingPattern,
    MalformedOperator,
    MalformedSection,
    PatternMatchFailure,
    Evidence,
}
