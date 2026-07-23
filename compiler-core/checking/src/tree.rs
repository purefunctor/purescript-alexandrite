pub mod pretty;

use std::ops::Index;
use std::sync::Arc;

use files::FileId;
use indexing::{EquationSourceId, TermItemId, TypeItemId};
use la_arena::{Arena, ArenaMap, Idx};
use lowering::LetBindingNameGroupId;
use smol_str::SmolStr;

use crate::TypeId;
use crate::core::{ForallBinderId, Role};
use crate::evidence::{Evidence, EvidenceVarId, SuperclassId};

pub type ExpressionId = Idx<Expression>;
pub type BinderId = Idx<Binder>;
pub type TermDeclarationId = Idx<TermDeclaration>;
pub type TypeDeclarationId = Idx<TypeDeclaration>;
pub type LocalDeclarationId = Idx<LocalDeclaration>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Module {
    pub(crate) arena: ModuleArena,
    terms: ArenaMap<TermItemId, TermDeclarationId>,
    types: ArenaMap<TypeItemId, TypeDeclarationId>,
    lets: ArenaMap<LetBindingNameGroupId, LocalDeclarationId>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleArena {
    pub(crate) expressions: Arena<Expression>,
    pub(crate) binders: Arena<Binder>,
    pub(crate) terms: Arena<TermDeclaration>,
    pub(crate) types: Arena<TypeDeclaration>,
    pub(crate) lets: Arena<LocalDeclaration>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TermDeclaration {
    pub type_id: TypeId,
    pub kind: TermDeclarationKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TermDeclarationKind {
    Value(ValueDeclaration),
    Constructor(DataConstructor),
    Instance(InstanceDeclaration),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValueDeclaration {
    pub evidences: Arc<[Evidence]>,
    pub equations: Arc<[Equation]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LocalDeclaration {
    pub source: LetBindingNameGroupId,
    pub type_id: TypeId,
    pub value: ValueDeclaration,
}

impl LocalDeclaration {
    pub fn new(
        source: LetBindingNameGroupId,
        type_id: TypeId,
        evidences: Arc<[Evidence]>,
        equations: Arc<[Equation]>,
    ) -> LocalDeclaration {
        let value = ValueDeclaration { evidences, equations };
        LocalDeclaration { source, type_id, value }
    }

    pub fn nullary(
        source: LetBindingNameGroupId,
        type_id: TypeId,
        equation_source: lowering::LetBindingEquationId,
        guarded_expression: GuardedExpression,
    ) -> LocalDeclaration {
        let equation = Equation::local(equation_source, [].into(), guarded_expression);
        LocalDeclaration::new(source, type_id, [].into(), [equation].into())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DataConstructor {
    pub arguments: Arc<[TypeId]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InstanceDeclaration {
    pub class: (FileId, TypeItemId),
    pub rigid_parameters: Arc<[TypeId]>,
    pub evidences: Arc<[InstanceEvidence]>,
    pub superclasses: Arc<[InstanceSuperclass]>,
    pub members: Arc<[InstanceMember]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceEvidence {
    pub constraint: TypeId,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceSuperclass {
    pub id: SuperclassId,
    pub constraint: TypeId,
    pub evidence: EvidenceVarId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceMember {
    pub resolution: (FileId, TermItemId),
    pub implementation_type: TypeId,
    pub evidences: Arc<[Evidence]>,
    pub equations: Arc<[Equation]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeDeclaration {
    pub kind: TypeId,
    pub roles: Arc<[Role]>,
    pub declaration: TypeDeclarationKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TypeDeclarationKind {
    Data(DataDeclaration),
    Newtype(DataDeclaration),
    Class(ClassDeclaration),
}

#[derive(Debug, PartialEq, Eq)]
pub struct DataDeclaration {
    pub parameters: Arc<[ForallBinderId]>,
    pub constructors: Arc<[TermDeclarationId]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClassDeclaration {
    pub kind_binders: Arc<[ForallBinderId]>,
    pub type_parameters: Arc<[ForallBinderId]>,
    pub superclasses: Arc<[ClassSuperclass]>,
    pub members: Arc<[ClassMember]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassSuperclass {
    pub id: SuperclassId,
    pub constraint: TypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassMember {
    pub source: TermItemId,
    pub field_type: TypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquationSource {
    Item(EquationSourceId),
    Local(lowering::LetBindingEquationId),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Equation {
    pub source: EquationSource,
    pub binders: Arc<[BinderId]>,
    pub guarded_expression: GuardedExpression,
}

impl Equation {
    pub fn item(
        source: EquationSourceId,
        binders: Arc<[BinderId]>,
        guarded_expression: GuardedExpression,
    ) -> Equation {
        let source = EquationSource::Item(source);
        Equation { source, binders, guarded_expression }
    }

    pub fn local(
        source: lowering::LetBindingEquationId,
        binders: Arc<[BinderId]>,
        guarded_expression: GuardedExpression,
    ) -> Equation {
        let source = EquationSource::Local(source);
        Equation { source, binders, guarded_expression }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct GuardedExpression {
    pub alternatives: Arc<[GuardedAlternative]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct GuardedAlternative {
    pub pattern_guards: Arc<[PatternGuard]>,
    pub where_expression: WhereExpression,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CaseAlternative {
    pub binders: Arc<[BinderId]>,
    pub guarded_expression: GuardedExpression,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PatternGuard {
    Boolean { expression: ExpressionId },
    Pattern { binder: BinderId, expression: ExpressionId },
}

#[derive(Debug, PartialEq, Eq)]
pub struct WhereExpression {
    pub bindings: LetBindings,
    pub expression: ExpressionId,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LetBindings {
    pub chunks: Arc<[LetBindingChunk]>,
}

impl WhereExpression {
    pub fn new(expression: ExpressionId) -> WhereExpression {
        WhereExpression { bindings: LetBindings::default(), expression }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LetBindingChunk {
    Pattern {
        source: lowering::LetBindingId,
        binder: BinderId,
        where_expression: WhereExpression,
    },
    PatternError {
        source: lowering::LetBindingId,
        binder_source: Option<lowering::BinderId>,
        where_expression: Option<WhereExpression>,
    },
    Names {
        declarations: Arc<[LocalDeclarationId]>,
        groups: Arc<[lowering::Scc<LetBindingNameGroupId>]>,
    },
}

impl GuardedExpression {
    pub fn unconditional(where_expression: WhereExpression) -> GuardedExpression {
        let alternative = GuardedAlternative { pattern_guards: Arc::from([]), where_expression };
        GuardedExpression { alternatives: Arc::from([alternative]) }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Binder {
    pub source: lowering::BinderId,
    pub type_id: TypeId,
    pub kind: BinderKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BinderKind {
    Error,
    Typed { binder: BinderId, annotation: TypeId },
    Integer { value: i32 },
    Number { negative: bool, value: SmolStr },
    Variable,
    Named { name: SmolStr, binder: BinderId },
    Wildcard,
    String { value: SmolStr },
    Char { value: char },
    Boolean { value: bool },
    Array { elements: Arc<[BinderId]> },
    Record { fields: Arc<[RecordBinderField]> },
    Constructor { resolution: (FileId, TermItemId), arguments: Arc<[BinderId]> },
}

#[derive(Debug, PartialEq, Eq)]
pub enum RecordBinderField {
    Field { label: SmolStr, binder: BinderId },
    Pun { label: SmolStr },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Expression {
    pub type_id: TypeId,
    pub kind: ExpressionKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExpressionKind {
    Error,
    String { kind: lowering::StringKind, value: SmolStr },
    Char { value: char },
    Boolean { value: bool },
    Integer { value: i32 },
    Number { value: SmolStr },
    Array { elements: Arc<[ExpressionId]> },
    Record { fields: Arc<[RecordExpressionField]> },
    Constructor { resolution: (FileId, TermItemId) },
    Variable { resolution: lowering::TermVariableResolution },
    RecordPun { source: lowering::RecordPunId, resolution: lowering::TermVariableResolution },
    TermApplication { function: ExpressionId, argument: ExpressionId },
    TypeApplication { function: ExpressionId, argument: TypeId },
    EvidenceApplication { function: ExpressionId, evidence: EvidenceVarId },
    Lambda { binders: Arc<[BinderId]>, expression: ExpressionId },
    Case { scrutinees: Arc<[ExpressionId]>, alternatives: Arc<[CaseAlternative]> },
    Let { bindings: LetBindings, expression: ExpressionId },
}

#[derive(Debug, PartialEq, Eq)]
pub enum RecordExpressionField {
    Field { label: SmolStr, expression: ExpressionId },
    Pun { source: lowering::RecordPunId, label: SmolStr, expression: ExpressionId },
}

impl Module {
    pub fn allocate_expression(&mut self, expression: Expression) -> ExpressionId {
        self.arena.expressions.alloc(expression)
    }

    pub fn allocate_binder(&mut self, binder: Binder) -> BinderId {
        self.arena.binders.alloc(binder)
    }

    pub fn insert_term(&mut self, source: TermItemId, term: TermDeclaration) -> TermDeclarationId {
        let term = self.arena.terms.alloc(term);
        self.terms.insert(source, term);
        term
    }

    pub fn lookup_term(&self, source: TermItemId) -> Option<TermDeclarationId> {
        self.terms.get(source).copied()
    }

    pub fn insert_let(&mut self, declaration: LocalDeclaration) -> LocalDeclarationId {
        let source = declaration.source;
        let declaration = self.arena.lets.alloc(declaration);
        let previous = self.lets.insert(source, declaration);
        assert!(previous.is_none(), "invariant violated: local declaration inserted twice");
        declaration
    }

    pub fn lookup_let(&self, source: LetBindingNameGroupId) -> Option<LocalDeclarationId> {
        self.lets.get(source).copied()
    }

    pub fn insert_type_declaration(
        &mut self,
        source: TypeItemId,
        declaration: TypeDeclaration,
    ) -> TypeDeclarationId {
        let declaration = self.arena.types.alloc(declaration);
        self.types.insert(source, declaration);
        declaration
    }

    pub fn lookup_type_declaration(&self, source: TypeItemId) -> Option<TypeDeclarationId> {
        self.types.get(source).copied()
    }
}

impl Index<ExpressionId> for Module {
    type Output = Expression;

    fn index(&self, index: ExpressionId) -> &Expression {
        &self.arena.expressions[index]
    }
}

impl Index<BinderId> for Module {
    type Output = Binder;

    fn index(&self, index: BinderId) -> &Binder {
        &self.arena.binders[index]
    }
}

impl Index<TermDeclarationId> for Module {
    type Output = TermDeclaration;

    fn index(&self, index: TermDeclarationId) -> &TermDeclaration {
        &self.arena.terms[index]
    }
}

impl Index<TypeDeclarationId> for Module {
    type Output = TypeDeclaration;

    fn index(&self, index: TypeDeclarationId) -> &TypeDeclaration {
        &self.arena.types[index]
    }
}

impl Index<LocalDeclarationId> for Module {
    type Output = LocalDeclaration;

    fn index(&self, index: LocalDeclarationId) -> &LocalDeclaration {
        &self.arena.lets[index]
    }
}
