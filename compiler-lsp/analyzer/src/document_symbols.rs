use async_lsp::lsp_types::*;
use building::QueryEngine;
use files::Files;
use indexing::{TermItemKind, TypeItemKind};

use crate::{AnalyzerError, common};

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
    engine: &QueryEngine,
    files: &Files,
    uri: Url,
) -> Result<Option<DocumentSymbolResponse>, AnalyzerError> {
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

        let location = common::file_term_location(engine, uri.clone(), current_file, term_id)?;
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

        let location = common::file_type_location(engine, uri.clone(), current_file, type_id)?;
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

        let location = common::file_type_location(engine, uri.clone(), current_file, type_id)?;
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

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use stabilizing::AstId;
    use syntax::cst;

    use super::*;

    fn ast_id<N: rowan::ast::AstNode<Language = syntax::PureScript>>() -> AstId<N> {
        AstId::new(NonZeroU32::new(1).unwrap())
    }

    #[test]
    fn maps_term_item_kinds_to_symbol_kinds() {
        let cases = [
            (
                TermItemKind::Constructor { id: ast_id::<cst::DataConstructor>() },
                SymbolKind::CONSTRUCTOR,
            ),
            (
                TermItemKind::ClassMember { id: ast_id::<cst::ClassMemberStatement>() },
                SymbolKind::METHOD,
            ),
            (
                TermItemKind::Operator { id: ast_id::<cst::InfixDeclaration>() },
                SymbolKind::OPERATOR,
            ),
            (
                TermItemKind::Value {
                    signature: Some(ast_id::<cst::ValueSignature>()),
                    equations: vec![ast_id::<cst::ValueEquation>()],
                },
                SymbolKind::FUNCTION,
            ),
            (
                TermItemKind::Foreign { id: ast_id::<cst::ForeignImportValueDeclaration>() },
                SymbolKind::FUNCTION,
            ),
            (TermItemKind::Derive { id: ast_id::<cst::DeriveDeclaration>() }, SymbolKind::FUNCTION),
            (
                TermItemKind::Instance { id: ast_id::<cst::InstanceDeclaration>() },
                SymbolKind::FUNCTION,
            ),
        ];

        for (kind, expected) in cases {
            assert_eq!(term_symbol_kind(&kind), expected);
        }
    }

    #[test]
    fn maps_type_item_kinds_to_symbol_kinds() {
        let cases = [
            (
                TypeItemKind::Class {
                    signature: Some(ast_id::<cst::ClassSignature>()),
                    declaration: Some(ast_id::<cst::ClassDeclaration>()),
                },
                SymbolKind::INTERFACE,
            ),
            (
                TypeItemKind::Operator { id: ast_id::<cst::InfixDeclaration>() },
                SymbolKind::OPERATOR,
            ),
            (
                TypeItemKind::Data {
                    signature: Some(ast_id::<cst::DataSignature>()),
                    equation: Some(ast_id::<cst::DataEquation>()),
                    role: Some(ast_id::<cst::TypeRoleDeclaration>()),
                },
                SymbolKind::ENUM,
            ),
            (
                TypeItemKind::Newtype {
                    signature: Some(ast_id::<cst::NewtypeSignature>()),
                    equation: Some(ast_id::<cst::NewtypeEquation>()),
                    role: Some(ast_id::<cst::TypeRoleDeclaration>()),
                },
                SymbolKind::STRUCT,
            ),
            (
                TypeItemKind::Synonym {
                    signature: Some(ast_id::<cst::TypeSynonymSignature>()),
                    equation: Some(ast_id::<cst::TypeSynonymEquation>()),
                },
                SymbolKind::STRUCT,
            ),
            (
                TypeItemKind::Foreign {
                    id: ast_id::<cst::ForeignImportDataDeclaration>(),
                    role: Some(ast_id::<cst::TypeRoleDeclaration>()),
                },
                SymbolKind::STRUCT,
            ),
        ];

        for (kind, expected) in cases {
            assert_eq!(type_symbol_kind(&kind), expected);
        }
    }
}
