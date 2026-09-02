use std::sync::Arc;

use building_types::QueryProxy;
use indexing::{IndexedTermItemKind, IndexedTypeItemKind};
use line_index::LineIndex;
use lsp_types::*;
use radix_trie::Trie;

use crate::{AnalyzerContext, AnalyzerError, common};

fn term_symbol_kind(kind: &IndexedTermItemKind) -> SymbolKind {
    match kind {
        IndexedTermItemKind::Constructor { .. } => SymbolKind::CONSTRUCTOR,
        IndexedTermItemKind::ClassMember { .. } => SymbolKind::METHOD,
        IndexedTermItemKind::Operator { .. } => SymbolKind::OPERATOR,
        IndexedTermItemKind::Value { .. } | IndexedTermItemKind::Foreign { .. } => {
            SymbolKind::FUNCTION
        }
    }
}

fn type_symbol_kind(kind: &IndexedTypeItemKind) -> SymbolKind {
    match kind {
        // Note: type classes are partitioned out of `iter_types()` and exposed via `iter_classes()`.
        // Keep this arm for exhaustiveness in case that invariant changes.
        IndexedTypeItemKind::Class { .. } => SymbolKind::INTERFACE,
        IndexedTypeItemKind::Operator { .. } => SymbolKind::OPERATOR,
        IndexedTypeItemKind::Data { .. } => SymbolKind::ENUM,
        IndexedTypeItemKind::Synonym { .. } => SymbolKind::TYPE_PARAMETER,
        IndexedTypeItemKind::Newtype { .. } | IndexedTypeItemKind::Foreign { .. } => {
            SymbolKind::STRUCT
        }
    }
}

pub fn document(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    uri: Url,
) -> Result<Option<DocumentSymbolResponse>, AnalyzerError> {
    let engine = context.queries();

    let current_file = {
        let uri = uri.as_str();
        context.file_id(uri).ok_or(AnalyzerError::NonFatal)?
    };

    let resolved = engine.resolved(current_file)?;
    let indexed = engine.indexed(current_file)?;
    let content = engine.content(current_file)?;
    let line_index = LineIndex::new(&content);

    let mut symbols = vec![];

    for (name, file_id, term_id) in resolved.locals.iter_terms() {
        if file_id != current_file {
            continue;
        }
        let kind = term_symbol_kind(&indexed.items[term_id].kind);
        let uri = Url::clone(&uri);
        let location = common::file_term_location_with_line_index(
            context,
            uri,
            current_file,
            &line_index,
            term_id,
        )?;
        symbols.push(SymbolInformation {
            name: name.to_string(),
            kind,
            tags: None,
            #[allow(deprecated)]
            deprecated: None,
            location,
            container_name: None,
        });
    }

    for (name, file_id, type_id) in resolved.locals.iter_types() {
        if file_id != current_file {
            continue;
        }
        let kind = type_symbol_kind(&indexed.items[type_id].kind);
        let uri = Url::clone(&uri);
        let location = common::file_type_location_with_line_index(
            context,
            uri,
            current_file,
            &line_index,
            type_id,
        )?;
        symbols.push(SymbolInformation {
            name: name.to_string(),
            kind,
            tags: None,
            #[allow(deprecated)]
            deprecated: None,
            location,
            container_name: None,
        });
    }

    for (name, file_id, type_id) in resolved.locals.iter_classes() {
        if file_id != current_file {
            continue;
        }
        let kind = SymbolKind::INTERFACE;
        let uri = Url::clone(&uri);
        let location = common::file_type_location_with_line_index(
            context,
            uri,
            current_file,
            &line_index,
            type_id,
        )?;
        symbols.push(SymbolInformation {
            name: name.to_string(),
            kind,
            tags: None,
            #[allow(deprecated)]
            deprecated: None,
            location,
            container_name: None,
        });
    }

    symbols.sort_by_key(|s| (s.location.range.start.line, s.location.range.start.character));
    Ok(Some(DocumentSymbolResponse::Flat(symbols)))
}

pub fn workspace(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    cache: &mut WorkspaceSymbolsCache,
    query: &str,
) -> Result<Option<WorkspaceSymbolResponse>, AnalyzerError> {
    if query.is_empty() {
        return Ok(None);
    }

    let query = query.to_lowercase();

    if let Some(exact_symbols) = cache.get(&query) {
        tracing::debug!("Found exact match for '{query}'");
        let flat = Vec::clone(exact_symbols);
        return Ok(Some(WorkspaceSymbolResponse::Flat(flat)));
    }

    let symbols = if let Some(prefix_symbols) = cache.get_ancestor_value(&query) {
        tracing::debug!("Found prefix match for '{query}'");
        let filtered_symbols = filter_symbols(prefix_symbols, &query);
        if filtered_symbols.len() == prefix_symbols.len() {
            Arc::clone(prefix_symbols)
        } else {
            Arc::new(filtered_symbols)
        }
    } else {
        tracing::debug!("Initialising cache for '{query}'");
        let filtered_symbols = build_symbol_list(context, &query)?;
        Arc::new(filtered_symbols)
    };

    let key = String::clone(&query);
    let value = Arc::clone(&symbols);
    cache.insert(key, value);

    let flat = Vec::clone(&*symbols);
    Ok(Some(WorkspaceSymbolResponse::Flat(flat)))
}

fn filter_symbols(cached: &[SymbolInformation], query: &str) -> Vec<SymbolInformation> {
    cached.iter().filter(|symbol| symbol.name.to_lowercase().starts_with(query)).cloned().collect()
}

fn build_symbol_list(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    query: &str,
) -> Result<Vec<SymbolInformation>, AnalyzerError> {
    let mut symbols = vec![];

    for file_id in context.active_files() {
        let resolved = context.queries().resolved(file_id)?;
        let indexed = context.queries().indexed(file_id)?;
        let uri = common::file_uri(context, file_id)?;

        for (name, _, term_id) in resolved.locals.iter_terms() {
            if !name.to_lowercase().starts_with(query) {
                continue;
            }
            let kind = term_symbol_kind(&indexed.items[term_id].kind);
            let uri = Url::clone(&uri);
            let location = common::file_term_location(context, uri, file_id, term_id)?;
            symbols.push(SymbolInformation {
                name: name.to_string(),
                kind,
                tags: None,
                #[allow(deprecated)]
                deprecated: None,
                location,
                container_name: None,
            });
        }

        for (name, _, type_id) in resolved.locals.iter_types() {
            if !name.to_lowercase().starts_with(query) {
                continue;
            }
            let kind = type_symbol_kind(&indexed.items[type_id].kind);
            let uri = Url::clone(&uri);
            let location = common::file_type_location(context, uri, file_id, type_id)?;
            symbols.push(SymbolInformation {
                name: name.to_string(),
                kind,
                tags: None,
                #[allow(deprecated)]
                deprecated: None,
                location,
                container_name: None,
            });
        }

        for (name, _, type_id) in resolved.locals.iter_classes() {
            if !name.to_lowercase().starts_with(query) {
                continue;
            }
            let uri = Url::clone(&uri);
            let location = common::file_type_location(context, uri, file_id, type_id)?;
            symbols.push(SymbolInformation {
                name: name.to_string(),
                kind: SymbolKind::INTERFACE,
                tags: None,
                #[allow(deprecated)]
                deprecated: None,
                location,
                container_name: None,
            });
        }
    }

    Ok(symbols)
}

pub type WorkspaceSymbolsCache = Trie<String, Arc<Vec<SymbolInformation>>>;
