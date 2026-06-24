use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Module {
    pub name: Option<String>,
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub name: Option<String>,
    pub signature: Option<String>,
    pub kind: Kind,
}

#[derive(Serialize, Deserialize)]
pub enum Kind {
    Term,
    Type,
}
