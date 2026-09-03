use building_types::QueryProxy;
use files::FileId;
use indexing::{DeriveItemId, InstanceItemId, TermItemId, TypeItemId};
use line_index::LineIndex;
use lsp_types::*;
use syntax::ast::AstNode;
use syntax::{SyntaxNode, SyntaxNodePtr};

use crate::position::Utf8Range;
use crate::{AnalyzerContext, AnalyzerError, locate, position};

pub fn file_term_location(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    uri: Url,
    file_id: FileId,
    line_index: &LineIndex,
    term_id: TermItemId,
) -> Result<Location, AnalyzerError> {
    let engine = context.queries();
    let (parsed, _) = engine.parsed(file_id)?;

    let stabilized = engine.stabilized(file_id)?;
    let indexed = engine.indexed(file_id)?;

    let root = parsed.syntax_node();
    let pointers = indexed.term_item_ptr(&stabilized, term_id);

    let range = pointers_range(line_index, root, pointers)?;
    let range = position::utf8_range_to_protocol(line_index, range, context.position_encoding())
        .ok_or(AnalyzerError::NonFatal)?;
    Ok(Location { uri, range })
}

pub fn file_type_location(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    uri: Url,
    file_id: FileId,
    line_index: &LineIndex,
    type_id: TypeItemId,
) -> Result<Location, AnalyzerError> {
    let engine = context.queries();
    let (parsed, _) = engine.parsed(file_id)?;

    let stabilized = engine.stabilized(file_id)?;
    let indexed = engine.indexed(file_id)?;

    let root = parsed.syntax_node();
    let pointers = indexed.type_item_ptr(&stabilized, type_id);

    let range = pointers_range(line_index, root, pointers)?;
    let range = position::utf8_range_to_protocol(line_index, range, context.position_encoding())
        .ok_or(AnalyzerError::NonFatal)?;

    Ok(Location { uri, range })
}

pub fn file_instance_location(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    uri: Url,
    file_id: FileId,
    item_id: InstanceItemId,
) -> Result<Location, AnalyzerError> {
    let indexed = context.queries().indexed(file_id)?;
    file_source_location(context, uri, file_id, indexed.items[item_id].id)
}

pub fn file_derive_location(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    uri: Url,
    file_id: FileId,
    item_id: DeriveItemId,
) -> Result<Location, AnalyzerError> {
    let indexed = context.queries().indexed(file_id)?;
    file_source_location(context, uri, file_id, indexed.items[item_id].id)
}

fn file_source_location<T: AstNode>(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    uri: Url,
    file_id: FileId,
    source_id: stabilizing::AstId<T>,
) -> Result<Location, AnalyzerError> {
    let content = context.queries().content(file_id)?;
    let line_index = LineIndex::new(&content);
    let stabilized = context.queries().stabilized(file_id)?;
    let range = locate::id_range(
        &line_index,
        &context.queries().parsed(file_id)?.0,
        &stabilized,
        source_id,
    )
    .ok_or(AnalyzerError::NonFatal)?;
    let range = position::utf8_range_to_protocol(&line_index, range, context.position_encoding())
        .ok_or(AnalyzerError::NonFatal)?;
    Ok(Location { uri, range })
}

pub fn file_uri(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    file_id: FileId,
) -> Result<Url, AnalyzerError> {
    context.file_uri(file_id)?.ok_or(AnalyzerError::NonFatal)
}

pub fn pointers_range(
    line_index: &LineIndex,
    root: SyntaxNode,
    pointers: impl Iterator<Item = SyntaxNodePtr>,
) -> Result<Utf8Range, AnalyzerError> {
    pointers
        .filter_map(|ptr| locate::syntax_range(line_index, &root, &ptr))
        .reduce(|start, end| Utf8Range { start: start.start, end: end.end })
        .ok_or(AnalyzerError::NonFatal)
}
