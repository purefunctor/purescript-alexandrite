use async_lsp::lsp_types::*;
use indexing::{TermItemKind, TypeItemKind};

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
        TypeItemKind::Newtype { .. }
        | TypeItemKind::Synonym { .. }
        | TypeItemKind::Foreign { .. } => SymbolKind::STRUCT,
    }
}

pub fn implementation(
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

    let mut symbols: Vec<SymbolInformation> = vec![];

    for (name, file_id, term_id) in resolved.locals.iter_terms() {
        if file_id != current_file {
            continue;
        }

        let kind = term_symbol_kind(&indexed.items[term_id].kind);

        let location = common::file_term_location(context, uri.clone(), current_file, term_id)?;
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

        let location = common::file_type_location(context, uri.clone(), current_file, type_id)?;
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

        let location = common::file_type_location(context, uri.clone(), current_file, type_id)?;
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

    // Provide stable output ordering for tests/clients.
    symbols.sort_by_key(|s| (s.location.range.start.line, s.location.range.start.character));

    Ok(Some(DocumentSymbolResponse::Flat(symbols)))
}
