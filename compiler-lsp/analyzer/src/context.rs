use std::sync::Arc;

use building_types::QueryProxy;
use checking::core::pretty::PrettyQueries;
use files::{FileId, Files};
use lsp_types::Url;

use crate::position::PositionEncoding;

pub trait AnalyzerQueries:
    PrettyQueries
    + QueryProxy<
        Parsed = parsing::FullParsedModule,
        Stabilized = Arc<stabilizing::StabilizedModule>,
        Indexed = Arc<indexing::IndexedModule>,
        Lowered = Arc<lowering::LoweredModule>,
        Resolved = Arc<resolving::ResolvedModule>,
        Checked = Arc<checking::CheckedModule>,
    >
{
}

impl<Queries> AnalyzerQueries for Queries where
    Queries: PrettyQueries
        + QueryProxy<
            Parsed = parsing::FullParsedModule,
            Stabilized = Arc<stabilizing::StabilizedModule>,
            Indexed = Arc<indexing::IndexedModule>,
            Lowered = Arc<lowering::LoweredModule>,
            Resolved = Arc<resolving::ResolvedModule>,
            Checked = Arc<checking::CheckedModule>,
        >
{
}

pub trait FileCatalog {
    fn file_id(&self, uri: &str) -> Option<FileId>;
    fn file_uri(&self, file_id: FileId) -> Result<Option<Url>, url::ParseError>;
    fn active_files(&self) -> impl Iterator<Item = FileId>;
}

impl FileCatalog for Files {
    fn file_id(&self, uri: &str) -> Option<FileId> {
        self.id(uri)
    }

    fn file_uri(&self, file_id: FileId) -> Result<Option<Url>, url::ParseError> {
        let path = self.path(file_id);
        Url::parse(&path).map(Some)
    }

    fn active_files(&self) -> impl Iterator<Item = FileId> {
        self.iter_id()
    }
}

pub struct LanguageContext<'a, Queries, Catalog> {
    pub engine: &'a Queries,
    pub files: &'a Catalog,
    pub encoding: PositionEncoding,
}

impl<'a, Queries, Catalog> LanguageContext<'a, Queries, Catalog> {
    pub fn new(
        engine: &'a Queries,
        files: &'a Catalog,
        encoding: PositionEncoding,
    ) -> LanguageContext<'a, Queries, Catalog> {
        LanguageContext { engine, files, encoding }
    }
}
