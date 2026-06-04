use std::sync::Arc;

use async_lsp::lsp_types::*;
use indexing::{TermItemKind, TypeItemKind};
use radix_trie::Trie;

use crate::{AnalyzerError, LanguageContext, common};

fn term_symbol_kind(kind: &TermItemKind) -> SymbolKind {
    match kind {
        TermItemKind::Constructor { .. } => SymbolKind::CONSTRUCTOR,
        TermItemKind::ClassMember { .. } => SymbolKind::METHOD,
        TermItemKind::Operator { .. } => SymbolKind::OPERATOR,
        TermItemKind::Value { .. }
        | TermItemKind::Foreign { .. }
        | TermItemKind::Derive { .. }
        | TermItemKind::Instance { .. } => SymbolKind::FUNCTION,
    }
}

fn type_symbol_kind(kind: &TypeItemKind) -> SymbolKind {
    match kind {
        // Note: type classes are partitioned out of `iter_types()` and exposed via `iter_classes()`.
        // Keep this arm for exhaustiveness in case that invariant changes.
        TypeItemKind::Class { .. } => SymbolKind::INTERFACE,
        TypeItemKind::Operator { .. } => SymbolKind::OPERATOR,
        TypeItemKind::Data { .. } => SymbolKind::ENUM,
        TypeItemKind::Synonym { .. } => SymbolKind::TYPE_PARAMETER,
        TypeItemKind::Newtype { .. } | TypeItemKind::Foreign { .. } => SymbolKind::STRUCT,
    }
}

pub fn document(
    context: &LanguageContext,
    uri: Url,
) -> Result<Option<DocumentSymbolResponse>, AnalyzerError> {
    let engine = context.engine;
    let files = context.files;

    let current_file = {
        let uri = uri.as_str();
        files.id(uri).ok_or(AnalyzerError::NonFatal)?
    };

    let resolved = engine.resolved(current_file)?;
    let indexed = engine.indexed(current_file)?;

    let mut symbols = vec![];

    for (name, file_id, term_id) in resolved.locals.iter_terms() {
        if file_id != current_file {
            continue;
        }
        let kind = term_symbol_kind(&indexed.items[term_id].kind);
        let uri = Url::clone(&uri);
        let location = common::file_term_location(context, uri, current_file, term_id)?;
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
        let location = common::file_type_location(context, uri, current_file, type_id)?;
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
        let location = common::file_type_location(context, uri, current_file, type_id)?;
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
    context: &LanguageContext,
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
    context: &LanguageContext,
    query: &str,
) -> Result<Vec<SymbolInformation>, AnalyzerError> {
    let mut symbols = vec![];

    for file_id in context.files.iter_id() {
        let resolved = context.engine.resolved(file_id)?;
        let indexed = context.engine.indexed(file_id)?;
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
