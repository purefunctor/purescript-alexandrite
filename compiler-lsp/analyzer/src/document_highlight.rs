use async_lsp::lsp_types::*;
use files::{FileId, Files};
use lowering::{
    BinderId, BinderKind, ExpressionId, ExpressionKind, LetBindingNameGroupId,
    TermVariableResolution,
};
use rowan::ast::AstNode;
use syntax::{SyntaxNode, SyntaxNodePtr, cst};

use crate::position::{self, PositionEncoding, Utf8Range};
use crate::{AnalyzerError, LanguageContext, locate};

fn current_file_id(files: &Files, uri: &Url) -> Result<FileId, AnalyzerError> {
    let uri = uri.as_str();
    files.id(uri).ok_or(AnalyzerError::NonFatal)
}

fn push_highlight(
    content: &str,
    encoding: PositionEncoding,
    highlights: &mut Vec<DocumentHighlight>,
    range: Option<Utf8Range>,
) {
    if let Some(range) = range {
        let Some(range) = position::utf8_range_to_protocol(content, range, encoding) else {
            return;
        };
        highlights.push(DocumentHighlight { range, kind: None });
    }
}

fn push_highlights(
    content: &str,
    encoding: PositionEncoding,
    highlights: &mut Vec<DocumentHighlight>,
    ranges: impl IntoIterator<Item = Utf8Range>,
) {
    highlights.extend(ranges.into_iter().filter_map(|range| {
        let range = position::utf8_range_to_protocol(content, range, encoding)?;
        Some(DocumentHighlight { range, kind: None })
    }));
}

fn finish_highlights(mut highlights: Vec<DocumentHighlight>) -> Option<Vec<DocumentHighlight>> {
    highlights.sort_by_key(|DocumentHighlight { range, .. }| {
        (range.start.line, range.start.character, range.end.line, range.end.character)
    });
    highlights.dedup_by(|left, right| left.range == right.range);

    let has_highlights = !highlights.is_empty();
    has_highlights.then_some(highlights)
}

fn binder_name_range(content: &str, root: &SyntaxNode, ptr: &SyntaxNodePtr) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;

    if let Some(binder) = cst::BinderVariable::cast(node.clone()) {
        let tok = binder.name_token()?;
        return position::text_range_to_utf8_range(content, tok.text_range());
    }

    if let Some(binder) = cst::BinderNamed::cast(node) {
        let tok = binder.name_token()?;
        return position::text_range_to_utf8_range(content, tok.text_range());
    }

    None
}

fn let_signature_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let sig = cst::LetBindingSignature::cast(node)?;
    let tok = sig.name_token()?;
    position::text_range_to_utf8_range(content, tok.text_range())
}

fn let_equation_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let eq = cst::LetBindingEquation::cast(node)?;
    let tok = eq.name_token()?;
    position::text_range_to_utf8_range(content, tok.text_range())
}

fn highlight_binder(
    context: &LanguageContext,
    current_file: FileId,
    binder_id: BinderId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(current_file);
    let (parsed, _) = engine.parsed(current_file)?;
    let stabilized = engine.stabilized(current_file)?;
    let lowered = engine.lowered(current_file)?;

    let root = parsed.syntax_node();
    let ptr = stabilized.syntax_ptr(binder_id).ok_or(AnalyzerError::NonFatal)?;

    let mut highlights: Vec<DocumentHighlight> = vec![];

    push_highlight(
        &content,
        context.encoding,
        &mut highlights,
        binder_name_range(&content, &root, &ptr)
            .or_else(|| locate::syntax_range(&content, &root, &ptr)),
    );

    for (expr_id, expr_kind) in lowered.info.iter_expression() {
        if let ExpressionKind::Variable {
            resolution: Some(TermVariableResolution::Binder(id)), ..
        } = expr_kind
            && *id == binder_id
            && let Some(range) = locate::id_range(&content, &parsed, &stabilized, expr_id)
        {
            push_highlight(&content, context.encoding, &mut highlights, Some(range));
        }
    }

    Ok(finish_highlights(highlights))
}

fn highlight_let(
    context: &LanguageContext,
    current_file: FileId,
    let_binding_id: LetBindingNameGroupId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(current_file);
    let (parsed, _) = engine.parsed(current_file)?;
    let stabilized = engine.stabilized(current_file)?;
    let lowered = engine.lowered(current_file)?;

    let root = parsed.syntax_node();
    let binding = lowered.info.get_let_binding_group(let_binding_id);

    let mut highlights: Vec<DocumentHighlight> = vec![];

    if let Some(sig) = binding.signature {
        let ptr = stabilized.syntax_ptr(sig).ok_or(AnalyzerError::NonFatal)?;
        push_highlight(
            &content,
            context.encoding,
            &mut highlights,
            let_signature_name_range(&content, &root, &ptr)
                .or_else(|| locate::syntax_range(&content, &root, &ptr)),
        );
    }

    for &eq in binding.equations.iter() {
        let ptr = stabilized.syntax_ptr(eq).ok_or(AnalyzerError::NonFatal)?;
        push_highlight(
            &content,
            context.encoding,
            &mut highlights,
            let_equation_name_range(&content, &root, &ptr)
                .or_else(|| locate::syntax_range(&content, &root, &ptr)),
        );
    }

    for (expr_id, expr_kind) in lowered.info.iter_expression() {
        if let ExpressionKind::Variable {
            resolution: Some(TermVariableResolution::Let(id)), ..
        } = expr_kind
            && *id == let_binding_id
            && let Some(range) = locate::id_range(&content, &parsed, &stabilized, expr_id)
        {
            push_highlight(&content, context.encoding, &mut highlights, Some(range));
        }
    }

    Ok(finish_highlights(highlights))
}

fn highlight_expression(
    context: &LanguageContext,
    current_file: FileId,
    expression_id: ExpressionId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let engine = context.engine;
    let lowered = engine.lowered(current_file)?;
    let kind = lowered.info.get_expression_kind(expression_id).ok_or(AnalyzerError::NonFatal)?;

    if let ExpressionKind::Variable { resolution: Some(resolution), .. } = kind {
        match resolution {
            TermVariableResolution::Binder(binder_id) => {
                highlight_binder(context, current_file, *binder_id)
            }
            TermVariableResolution::Let(let_binding_id) => {
                highlight_let(context, current_file, *let_binding_id)
            }
            TermVariableResolution::RecordPun(_) | TermVariableResolution::Reference(..) => {
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

fn value_equation_highlights(
    context: &LanguageContext,
    current_file: FileId,
    term_id: indexing::TermItemId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(current_file);
    let (parsed, _) = engine.parsed(current_file)?;
    let stabilized = engine.stabilized(current_file)?;
    let indexed = engine.indexed(current_file)?;

    let root = parsed.syntax_node();
    let Some(ranges) =
        locate::value_equation_ranges(&content, &root, &stabilized, &indexed, term_id)
    else {
        return Ok(None);
    };

    let mut highlights = vec![];
    push_highlights(&content, context.encoding, &mut highlights, ranges);
    Ok(finish_highlights(highlights))
}

pub fn implementation(
    context: &LanguageContext,
    uri: Url,
    position: Position,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let engine = context.engine;
    let current_file = current_file_id(context.files, &uri)?;
    let content = engine.content(current_file);
    let utf8_position = position::protocol_position_to_utf8(&content, position, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;

    // If the cursor resolves to a top-level value in the current file, include
    // the definition name tokens alongside the reference highlights.
    let mut extra_highlights: Vec<DocumentHighlight> = vec![];

    // Local binders/lets do not have stable (file, item) identities in the workspace
    // index, so `textDocument/references` intentionally returns `None` for them.
    // For `textDocument/documentHighlight`, we still want local occurrences.
    let located = locate::locate(engine, current_file, utf8_position)?;
    match located {
        locate::Located::Binder(binder_id) => {
            let lowered = engine.lowered(current_file)?;
            let kind = lowered.info.get_binder_kind(binder_id).ok_or(AnalyzerError::NonFatal)?;
            if !matches!(kind, BinderKind::Constructor { .. })
                && let Some(highlights) = highlight_binder(context, current_file, binder_id)?
            {
                return Ok(Some(highlights));
            }
        }
        locate::Located::Expression(expression_id) => {
            if let Some(highlights) = highlight_expression(context, current_file, expression_id)? {
                return Ok(Some(highlights));
            }

            let lowered = engine.lowered(current_file)?;
            if let Some(ExpressionKind::Variable {
                resolution: Some(TermVariableResolution::Reference(f_id, t_id)),
                ..
            }) = lowered.info.get_expression_kind(expression_id)
                && (*f_id) == current_file
                && let Some(highlights) = value_equation_highlights(context, current_file, *t_id)?
            {
                extra_highlights = highlights;
            }
        }
        locate::Located::TermItem(term_id) => {
            if let Some(highlights) = value_equation_highlights(context, current_file, term_id)? {
                extra_highlights = highlights;
            }
        }
        _ => {}
    }

    // If the cursor is on a `where`/`let` *definition* name token, `locate` currently
    // returns `Nothing` (it's a LetBinding* node, not an Expression). Recover by
    // mapping the CST let binding to its lowering group id.
    {
        let content = engine.content(current_file);
        let (parsed, _) = engine.parsed(current_file)?;
        let stabilized = engine.stabilized(current_file)?;
        let lowered = engine.lowered(current_file)?;

        if let Some(offset) = position::utf8_position_to_offset(&content, utf8_position) {
            let root = parsed.syntax_node();
            let token = match root.token_at_offset(offset) {
                rowan::TokenAtOffset::None => None,
                rowan::TokenAtOffset::Single(token) => Some(token),
                rowan::TokenAtOffset::Between(_, right) => Some(right),
            };

            if let Some(token) = token {
                for node in token.parent_ancestors() {
                    if let Some(eq) = cst::LetBindingEquation::cast(node.clone())
                        && let Some(eq_id) = stabilized.lookup_cst(&eq)
                        && let Some(group_id) = lowered.info.let_binding_group_for_equation(eq_id)
                        && let Some(highlights) = highlight_let(context, current_file, group_id)?
                    {
                        return Ok(Some(highlights));
                    }
                    if let Some(sig) = cst::LetBindingSignature::cast(node)
                        && let Some(sig_id) = stabilized.lookup_cst(&sig)
                        && let Some(group_id) = lowered.info.let_binding_group_for_signature(sig_id)
                        && let Some(highlights) = highlight_let(context, current_file, group_id)?
                    {
                        return Ok(Some(highlights));
                    }
                }
            }
        }
    }

    let Some(locations) = crate::references::implementation(context, uri.clone(), position)? else {
        return Ok(None);
    };

    let mut highlights: Vec<DocumentHighlight> = locations
        .into_iter()
        .filter(|location| location.uri == uri)
        .map(|location| DocumentHighlight { range: location.range, kind: None })
        .collect();
    highlights.extend(extra_highlights);

    Ok(finish_highlights(highlights))
}
