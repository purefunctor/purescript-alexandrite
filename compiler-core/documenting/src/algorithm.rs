use rustc_hash::FxHashMap;

use indexing::{IndexedModule, TermItemId, TypeItemId};
use parsing::ParsedModule;
use stabilizing::StabilizedModule;

use crate::{DocumentedTerm, DocumentedType, annotation};

pub struct State {
    pub documentation: String,
    pub terms: FxHashMap<TermItemId, DocumentedTerm>,
    pub types: FxHashMap<TypeItemId, DocumentedType>,
}

pub fn document_module(
    parsed: &ParsedModule,
    stabilized: &StabilizedModule,
    indexed: &IndexedModule,
) -> State {
    let root = parsed.syntax_node();

    let annotations = annotation::AnnotationIndex::new(&root);
    let documentation = annotation::module_documentation(parsed);

    let terms = indexed.items.iter_terms().filter_map(|(id, item)| {
        let documentation = annotation::term_documentation(&stabilized, &annotations, item);
        Some((id, DocumentedTerm { documentation }))
    });

    let terms = terms.collect();

    let types = indexed.items.iter_types().filter_map(|(id, item)| {
        let documentation = annotation::type_documentation(&stabilized, &annotations, item);
        Some((id, DocumentedType { documentation }))
    });

    let types = types.collect();

    State { documentation, terms, types }
}
