mod algorithm;
mod annotation;

use std::sync::Arc;

use rustc_hash::FxHashMap;

use indexing::{TermItemId, TypeItemId};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DocumentedModule {
    pub documentation: String,
    pub terms: FxHashMap<TermItemId, DocumentedTerm>,
    pub types: FxHashMap<TypeItemId, DocumentedType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentedTerm {
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentedType {
    pub documentation: String,
}

pub fn document_module(
    source: &str,
    parsed: &parsing::ParsedModule,
    stabilized: &stabilizing::StabilizedModule,
    indexed: &indexing::IndexedModule,
) -> Arc<DocumentedModule> {
    let algorithm::State { documentation, terms, types } =
        algorithm::document_module(source, parsed, stabilized, indexed);
    Arc::new(DocumentedModule { documentation, terms, types })
}
