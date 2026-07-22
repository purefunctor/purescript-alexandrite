pub mod pretty;

use std::ops::Index;
use std::sync::Arc;

use files::FileId;
use indexing::{TermItemId, TypeItemId, ValueEquationId};
use la_arena::{Arena, ArenaMap, Idx};
use smol_str::SmolStr;

use crate::TypeId;
use crate::core::{ForallBinderId, Role};
use crate::evidence::{Evidence, EvidenceVarId};

pub type ExpressionId = Idx<Expression>;
pub type BinderId = Idx<Binder>;
pub type TermDeclarationId = Idx<TermDeclaration>;
pub type TypeDeclarationId = Idx<TypeDeclaration>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Module {
    pub(crate) arena: ModuleArena,
    terms: ArenaMap<TermItemId, TermDeclarationId>,
    types: ArenaMap<TypeItemId, TypeDeclarationId>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleArena {
    pub(crate) expressions: Arena<Expression>,
    pub(crate) binders: Arena<Binder>,
    pub(crate) terms: Arena<TermDeclaration>,
    pub(crate) types: Arena<TypeDeclaration>,
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
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValueDeclaration {
    pub evidences: Arc<[Evidence]>,
    pub equations: Arc<[Equation]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DataConstructor {
    pub arguments: Arc<[TypeId]>,
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
}

#[derive(Debug, PartialEq, Eq)]
pub struct DataDeclaration {
    pub parameters: Arc<[ForallBinderId]>,
    pub constructors: Arc<[TermDeclarationId]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Equation {
    pub source: ValueEquationId,
    pub binders: Arc<[BinderId]>,
    pub guarded_expression: GuardedExpression,
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
pub enum PatternGuard {
    Boolean { expression: ExpressionId },
    Pattern { binder: BinderId, expression: ExpressionId },
}

#[derive(Debug, PartialEq, Eq)]
pub struct WhereExpression {
    pub expression: ExpressionId,
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
    Constructor { resolution: (FileId, TermItemId) },
    Variable { resolution: lowering::TermVariableResolution },
    TypeApplication { function: ExpressionId, argument: TypeId },
    EvidenceApplication { function: ExpressionId, evidence: EvidenceVarId },
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
