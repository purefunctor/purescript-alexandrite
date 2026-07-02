use building_types::QueryResult;
use files::FileId;
use rustc_hash::FxHashMap;

use indexing::{TermItemId, TypeItemId};

use crate::{DocumentedTerm, DocumentedType, ExternalQueries, annotation};

pub struct State {
    pub documentation: String,
    pub terms: FxHashMap<TermItemId, DocumentedTerm>,
    pub types: FxHashMap<TypeItemId, DocumentedType>,
}

pub fn document_module(queries: &impl ExternalQueries, file_id: FileId) -> QueryResult<State> {
    let (parsed, _) = queries.parsed(file_id)?;
    let root = parsed.syntax_node();

    let stabilized = queries.stabilized(file_id)?;
    let indexed = queries.indexed(file_id)?;

    let documentation = annotation::module_documentation(&root, &parsed);

    let terms = indexed.items.iter_terms().filter_map(|(id, item)| {
        let documentation = annotation::term_documentation(&stabilized, &root, item);
        Some((id, DocumentedTerm { documentation }))
    });

    let terms = terms.collect();

    let types = indexed.items.iter_types().filter_map(|(id, item)| {
        let documentation = annotation::type_documentation(&stabilized, &root, item);
        Some((id, DocumentedType { documentation }))
    });

    let types = types.collect();

    Ok(State { documentation, terms, types })
}
