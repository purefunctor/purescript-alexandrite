use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub struct Package {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub description: Option<String>,
    pub dependencies: BTreeMap<String, String>,
    pub modules: Vec<String>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub struct Module {
    pub name: String,
    pub documentation: Option<String>,
    pub terms: Vec<TermItem>,
    pub types: Vec<TypeItem>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub struct TermItem {
    pub name: Option<String>,
    pub documentation: Option<String>,
    pub signature: Option<Type>,
    pub kind: TermKind,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub enum TermKind {
    ClassMember,
    Constructor,
    Derive,
    Foreign,
    Instance,
    Operator,
    Value,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub struct TypeItem {
    pub name: Option<String>,
    pub documentation: Option<String>,
    pub signature: Option<Type>,
    pub kind: TypeKind,
    pub constructors: Vec<TermItem>,
    pub members: Vec<TermItem>,
    pub instances: Vec<TermItem>,
    pub expansion: Option<Type>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub enum TypeKind {
    Data,
    Newtype,
    Synonym,
    Class,
    Foreign,
    Operator,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
#[serde(tag = "tag", rename_all = "camelCase")]
pub enum Type {
    Application { function: Box<Type>, argument: Box<Type> },
    KindApplication { function: Box<Type>, argument: Box<Type> },
    Forall { binder: TypeBinder, body: Box<Type> },
    Constrained { constraint: Box<Type>, body: Box<Type> },
    Function { argument: Box<Type>, result: Box<Type> },
    Kinded { expression: Box<Type>, kind: Box<Type> },
    Constructor { reference: TypeReference },
    Integer { value: i32 },
    String { kind: StringLiteralKind, value: String },
    Row { fields: Vec<TypeRowField>, tail: Option<Box<Type>> },
    Rigid { name: String, kind: Box<Type> },
    Unification { id: u32 },
    Free { name: String },
    Unknown { name: String },
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub struct TypeReference {
    pub package: Option<String>,
    pub module: Option<String>,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub struct TypeBinder {
    pub name: String,
    pub visible: bool,
    pub kind: Box<Type>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
pub struct TypeRowField {
    pub label: String,
    #[serde(rename = "type")]
    pub t: Type,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export_to = "docs-schema.ts")]
#[serde(rename_all = "camelCase")]
pub enum StringLiteralKind {
    String,
    RawString,
}

impl From<lowering::StringKind> for StringLiteralKind {
    fn from(kind: lowering::StringKind) -> StringLiteralKind {
        match kind {
            lowering::StringKind::String => StringLiteralKind::String,
            lowering::StringKind::RawString => StringLiteralKind::RawString,
        }
    }
}
