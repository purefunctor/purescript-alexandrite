use async_lsp::lsp_types::*;
use building::prim;
use files::FileId;
use indexing::{TermItemId, TypeItemId};
use syntax::{SyntaxNode, SyntaxNodePtr};

use crate::position::Utf8Range;
use crate::{AnalyzerError, LanguageContext, locate, position};

pub fn file_term_location(
    context: &LanguageContext,
    uri: Url,
    file_id: FileId,
    term_id: TermItemId,
) -> Result<Location, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(file_id);
    let (parsed, _) = engine.parsed(file_id)?;

    let stabilized = engine.stabilized(file_id)?;
    let indexed = engine.indexed(file_id)?;

    let root = parsed.syntax_node();
    let pointers = indexed.term_item_ptr(&stabilized, term_id);

    let range = pointers_range(&content, root, pointers)?;
    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;
    Ok(Location { uri, range })
}

pub fn file_type_location(
    context: &LanguageContext,
    uri: Url,
    file_id: FileId,
    type_id: TypeItemId,
) -> Result<Location, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(file_id);
    let (parsed, _) = engine.parsed(file_id)?;

    let stabilized = engine.stabilized(file_id)?;
    let indexed = engine.indexed(file_id)?;

    let root = parsed.syntax_node();
    let pointers = indexed.type_item_ptr(&stabilized, type_id);

    let range = pointers_range(&content, root, pointers)?;
    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;

    Ok(Location { uri, range })
}

pub fn file_uri(context: &LanguageContext, file_id: FileId) -> Result<Url, AnalyzerError> {
    let path = context.files.path(file_id);
    let content = context.engine.content(file_id);

    let uri = Url::parse(&path)?;
    prim::handle_generated(uri, &content).ok_or(AnalyzerError::NonFatal)
}

pub fn pointers_range(
    content: &str,
    root: SyntaxNode,
    pointers: impl Iterator<Item = SyntaxNodePtr>,
) -> Result<Utf8Range, AnalyzerError> {
    pointers
        .filter_map(|ptr| locate::syntax_range(content, &root, &ptr))
        .reduce(|start, end| Utf8Range { start: start.start, end: end.end })
        .ok_or(AnalyzerError::NonFatal)
}
