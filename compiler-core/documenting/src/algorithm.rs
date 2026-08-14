use rustc_hash::FxHashMap;

use indexing::{DeriveItemId, IndexedModule, InstanceItemId, TermItemId, TypeItemId};
use parsing::ParsedModule;
use stabilizing::StabilizedModule;

use crate::{DocumentedTerm, DocumentedType, annotation};

pub struct State {
    pub documentation: String,
    pub terms: FxHashMap<TermItemId, DocumentedTerm>,
    pub types: FxHashMap<TypeItemId, DocumentedType>,
    pub instances: FxHashMap<InstanceItemId, DocumentedTerm>,
    pub derives: FxHashMap<DeriveItemId, DocumentedTerm>,
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

    let instances = indexed.items.iter_instances().map(|(id, item)| {
        let documentation = annotation::instance_documentation(stabilized, &annotations, item.id);
        (id, DocumentedTerm { documentation })
    });
    let instances = instances.collect();

    let derives = indexed.items.iter_derives().map(|(id, item)| {
        let documentation = annotation::derive_documentation(stabilized, &annotations, item.id);
        (id, DocumentedTerm { documentation })
    });
    let derives = derives.collect();

    State { documentation, terms, types, instances, derives }
}
