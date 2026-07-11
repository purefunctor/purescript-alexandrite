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
    source: &str,
    parsed: &ParsedModule,
    stabilized: &StabilizedModule,
    indexed: &IndexedModule,
) -> State {
    let root = parsed.syntax_node();

    let annotations = annotation::AnnotationIndex::new(source, &root);
    let documentation = annotation::module_documentation(source, parsed);

    let terms = indexed.items.iter_terms().map(|(id, item)| {
        let documentation = annotation::term_documentation(stabilized, &annotations, item);
        (id, DocumentedTerm { documentation })
    });

    let terms = terms.collect();

    let types = indexed.items.iter_types().map(|(id, item)| {
        let documentation = annotation::type_documentation(stabilized, &annotations, item);
        (id, DocumentedType { documentation })
    });

    let types = types.collect();

    State { documentation, terms, types }
}
