use la_arena::Idx;
use smol_str::SmolStr;

use crate::source::*;

/// A term item assembled from the source declarations that share its module-level identity.
#[derive(Debug, PartialEq, Eq)]
pub struct IndexedTermItem {
    pub name: Option<SmolStr>,
    pub kind: IndexedTermItemKind,
    pub exported: bool,
}

/// The source declarations grouped into an [`IndexedTermItem`].
#[derive(Debug, PartialEq, Eq)]
pub enum IndexedTermItemKind {
    ClassMember { id: ClassMemberId, parent: TypeItemId },
    Constructor { id: DataConstructorId, type_id: Option<TypeItemId> },
    Foreign { id: ForeignValueId },
    Operator { id: InfixId },
    Value { signature: Option<ValueSignatureId>, equations: Vec<ValueEquationId> },
}

/// A module-local term identity preserved by later compiler representations.
pub type TermItemId = Idx<IndexedTermItem>;

/// An instance declaration that participates in instance checking but not the term namespace.
#[derive(Debug, PartialEq, Eq)]
pub struct IndexedInstanceItem {
    pub name: Option<SmolStr>,
    pub id: InstanceId,
}

/// A module-local identity for an instance declaration.
pub type InstanceItemId = Idx<IndexedInstanceItem>;

/// A derive declaration that participates in instance checking but not the term namespace.
#[derive(Debug, PartialEq, Eq)]
pub struct IndexedDeriveItem {
    pub name: Option<SmolStr>,
    pub id: DeriveId,
}

/// A module-local identity for a derive declaration.
pub type DeriveItemId = Idx<IndexedDeriveItem>;

/// The source order used when checking overlap between declared and derived instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceSourceItemId {
    Instance(InstanceItemId),
    Derive(DeriveItemId),
}

/// Term symbols, instance declarations, and derive declarations in source allocation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderedTermItemId {
    Term(TermItemId),
    Instance(InstanceItemId),
    Derive(DeriveItemId),
}

/// A type item assembled from the source declarations that share its module-level identity.
#[derive(Debug, PartialEq, Eq)]
pub struct IndexedTypeItem {
    pub name: Option<SmolStr>,
    pub kind: IndexedTypeItemKind,
    pub exported: bool,
}

/// The source declarations grouped into an [`IndexedTypeItem`].
#[derive(Debug, PartialEq, Eq)]
pub enum IndexedTypeItemKind {
    Data {
        signature: Option<DataSignatureId>,
        equation: Option<DataEquationId>,
        role: Option<TypeRoleId>,
        constructors: Vec<TermItemId>,
    },
    Newtype {
        signature: Option<NewtypeSignatureId>,
        equation: Option<NewtypeEquationId>,
        role: Option<TypeRoleId>,
        constructors: Vec<TermItemId>,
    },
    Synonym {
        signature: Option<TypeSignatureId>,
        equation: Option<TypeEquationId>,
    },
    Class {
        signature: Option<ClassSignatureId>,
        declaration: Option<ClassDeclarationId>,
        members: Vec<TermItemId>,
    },
    Foreign {
        id: ForeignDataId,
        role: Option<TypeRoleId>,
    },
    Operator {
        id: InfixId,
    },
}

/// A module-local type identity preserved by later compiler representations.
pub type TypeItemId = Idx<IndexedTypeItem>;
