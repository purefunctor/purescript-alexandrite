use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub modules: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub terms: Vec<Term>,
    pub types: Vec<Type>,
}

#[derive(Serialize, Deserialize)]
pub struct Term {
    pub name: Option<String>,
    pub documentation: Option<String>,
    pub signature: Option<String>,
    pub kind: TermKind,
}

#[derive(Serialize, Deserialize)]
pub enum TermKind {
    ClassMember,
    Constructor,
    Derive,
    Foreign,
    Instance,
    Operator,
    Value,
}

#[derive(Serialize, Deserialize)]
pub struct Type {
    pub name: Option<String>,
    pub documentation: Option<String>,
    pub signature: Option<String>,
    pub kind: TypeKind,
}

#[derive(Serialize, Deserialize)]
pub enum TypeKind {
    Data,
    Newtype,
    Synonym,
    Class,
    Foreign,
    Operator,
}
