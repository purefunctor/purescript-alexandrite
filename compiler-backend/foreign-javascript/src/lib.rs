use std::collections::BTreeSet;
use std::iter;
use std::sync::Arc;

use building_types::QueryResult;
use files::{FileId, ForeignFileId};
use indexing::{IndexedModule, IndexedTermItemKind, TermItemId};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::module_record::{ExportEntry, ExportExportName};
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignModule {
    pub exports: BTreeSet<SmolStr>,
    pub errors: Arc<[Arc<str>]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignValidation {
    pub errors: Arc<[ForeignError]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForeignError {
    MissingModule { declaration: TermItemId, name: SmolStr },
    MissingImplementation { declaration: TermItemId, name: SmolStr },
    Parse { declaration: TermItemId, message: Arc<str> },
}

pub trait ForeignQueries {
    fn foreign_module(&self, id: ForeignFileId) -> QueryResult<Option<Arc<ForeignModule>>>;

    fn foreign_validation(&self, id: FileId) -> QueryResult<Arc<ForeignValidation>>;
}

pub fn parse_module(content: &str) -> ForeignModule {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, content, SourceType::mjs()).parse();

    let errors = parsed.diagnostics.into_iter().map(|diagnostic| diagnostic.to_string().into());
    let errors: Arc<[_]> = errors.collect();
    if !errors.is_empty() {
        return ForeignModule { exports: BTreeSet::new(), errors };
    }

    let local_exports = parsed.module_record.local_export_entries.iter();
    let indirect_exports = parsed.module_record.indirect_export_entries.iter();
    let entries = iter::chain(local_exports, indirect_exports);
    let exports = entries.filter_map(export_name);
    let exports = exports.collect();

    ForeignModule { exports, errors }
}

fn export_name(entry: &ExportEntry<'_>) -> Option<SmolStr> {
    let ExportExportName::Name(name) = &entry.export_name else {
        return None;
    };
    Some(SmolStr::new(name.name.as_str()))
}

pub fn validate_module(
    indexed: &IndexedModule,
    foreign: Option<&ForeignModule>,
) -> ForeignValidation {
    let declarations = indexed.items.iter_terms().filter_map(|(declaration, item)| {
        if !matches!(item.kind, IndexedTermItemKind::Foreign { .. }) {
            return None;
        }
        let name = item.name.clone()?;
        Some((declaration, name))
    });
    let declarations = declarations.collect::<Vec<_>>();

    let mut errors = Vec::new();
    let Some(foreign) = foreign else {
        let missing_modules = declarations
            .into_iter()
            .map(|(declaration, name)| ForeignError::MissingModule { declaration, name });
        errors.extend(missing_modules);
        return ForeignValidation { errors: errors.into() };
    };

    if let Some((declaration, _)) = declarations.first() {
        for message in foreign.errors.iter() {
            errors.push(ForeignError::Parse {
                declaration: *declaration,
                message: Arc::clone(message),
            });
        }
    }

    if !foreign.errors.is_empty() {
        return ForeignValidation { errors: errors.into() };
    }

    let missing_implementations = declarations.into_iter().filter_map(|(declaration, name)| {
        if foreign.exports.contains(&name) {
            None
        } else {
            Some(ForeignError::MissingImplementation { declaration, name })
        }
    });
    errors.extend(missing_implementations);

    ForeignValidation { errors: errors.into() }
}
