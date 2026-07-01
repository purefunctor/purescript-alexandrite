mod algorithm;
mod annotation;

use std::sync::Arc;

use building_types::{QueryProxy, QueryResult};
use files::FileId;
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

pub trait ExternalQueries:
    QueryProxy<
        Parsed = parsing::FullParsedModule,
        Stabilized = Arc<stabilizing::StabilizedModule>,
        Indexed = Arc<indexing::IndexedModule>,
        Checked = Arc<checking::CheckedModule>,
    >
{
}

impl<Q> ExternalQueries for Q where
    Q: QueryProxy<
            Parsed = parsing::FullParsedModule,
            Stabilized = Arc<stabilizing::StabilizedModule>,
            Indexed = Arc<indexing::IndexedModule>,
            Checked = Arc<checking::CheckedModule>,
        >
{
}

pub fn document_module(
    queries: &impl ExternalQueries,
    file_id: FileId,
) -> QueryResult<Arc<DocumentedModule>> {
    let algorithm::State { documentation, terms, types } =
        algorithm::document_module(queries, file_id)?;

    Ok(Arc::new(DocumentedModule { documentation, terms, types }))
}
