use async_lsp::lsp_types::*;
use files::FileId;
use indexing::TermItemId;
use lowering::{
    BinderId, BinderKind, ExpressionId, ExpressionKind, LetBindingNameGroupId, RecordPunId,
    TermOperatorId, TermVariableResolution,
};
use rowan::ast::AstNode;
use syntax::{SyntaxNode, SyntaxNodePtr, cst};

use crate::position::{PositionEncoding, Utf8Range};
use crate::{AnalyzerError, LanguageContext, locate, position};

pub fn implementation(
    context: &LanguageContext,
    uri: Url,
    position: Position,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let current_file = {
        let uri = uri.as_str();
        context.files.id(uri).ok_or(AnalyzerError::NonFatal)?
    };

    let content = context.engine.content(current_file);
    let position = position::protocol_position_to_utf8(&content, position, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;

    let located = locate::locate(context.engine, current_file, position)?;
    match located {
        locate::Located::Binder(binder_id) => highlight_binder(context, current_file, binder_id),
        locate::Located::Expression(expression_id) => {
            highlight_expression(context, current_file, expression_id)
        }
        locate::Located::TermItem(term_id) => {
            highlight_file_term(context, current_file, current_file, term_id)
        }
        locate::Located::LetBinding(let_binding_id) => {
            highlight_let(context, current_file, let_binding_id)
        }
        locate::Located::BinderPun(pun_id) => highlight_binder_pun(context, current_file, pun_id),
        locate::Located::ExpressionPun(pun_id) => {
            highlight_expression_pun(context, current_file, pun_id)
        }
        locate::Located::TermOperator(operator_id) => {
            highlight_term_operator(context, current_file, operator_id)
        }
        locate::Located::ModuleName(_)
        | locate::Located::ImportItem(_)
        | locate::Located::Type(_)
        | locate::Located::TypeOperator(_)
        | locate::Located::TypeItem(_)
        | locate::Located::Nothing => Ok(None),
    }
}

fn highlight_binder(
    context: &LanguageContext,
    current_file: FileId,
    binder_id: BinderId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let content = context.engine.content(current_file);
    let (parsed, _) = context.engine.parsed(current_file)?;
    let stabilized = context.engine.stabilized(current_file)?;
    let lowered = context.engine.lowered(current_file)?;

    let kind = lowered.info.get_binder_kind(binder_id).ok_or(AnalyzerError::NonFatal)?;

    if let BinderKind::Constructor { resolution: Some((file_id, term_id)), .. } = kind {
        return highlight_file_term(context, current_file, *file_id, *term_id);
    }

    let root = parsed.syntax_node();
    let ptr = stabilized.syntax_ptr(binder_id).ok_or(AnalyzerError::NonFatal)?;

    let mut highlights: Vec<DocumentHighlight> = vec![];

    highlights.extend(
        binder_name_range(&content, &root, &ptr)
            .or_else(|| locate::syntax_range(&content, &root, &ptr))
            .and_then(|range| document_highlight(&content, context.encoding, range)),
    );

    for (expr_id, expr_kind) in lowered.info.iter_expression() {
        if let ExpressionKind::Variable {
            resolution: Some(TermVariableResolution::Binder(id)), ..
        } = expr_kind
            && *id == binder_id
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, expr_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    for (pun_id, resolution) in lowered.info.iter_expression_pun() {
        if let TermVariableResolution::Binder(id) = resolution
            && id == binder_id
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, pun_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    Ok(finish_highlights(highlights))
}

fn highlight_expression(
    context: &LanguageContext,
    current_file: FileId,
    expression_id: ExpressionId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let lowered = context.engine.lowered(current_file)?;
    let kind = lowered.info.get_expression_kind(expression_id).ok_or(AnalyzerError::NonFatal)?;

    match kind {
        ExpressionKind::Constructor { resolution: Some((file_id, term_id)) }
        | ExpressionKind::OperatorName { resolution: Some((file_id, term_id)) } => {
            highlight_file_term(context, current_file, *file_id, *term_id)
        }
        ExpressionKind::Variable { resolution: Some(resolution), .. } => match resolution {
            TermVariableResolution::Binder(binder_id) => {
                highlight_binder(context, current_file, *binder_id)
            }
            TermVariableResolution::Let(let_binding_id) => {
                highlight_let(context, current_file, *let_binding_id)
            }
            TermVariableResolution::Reference(file_id, term_id) => {
                highlight_file_term(context, current_file, *file_id, *term_id)
            }
            TermVariableResolution::RecordPun(pun_id) => {
                highlight_binder_pun(context, current_file, *pun_id)
            }
        },
        _ => Ok(None),
    }
}

fn highlight_file_term(
    context: &LanguageContext,
    current_file: FileId,
    file_id: FileId,
    term_id: TermItemId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let content = context.engine.content(current_file);
    let (parsed, _) = context.engine.parsed(current_file)?;
    let stabilized = context.engine.stabilized(current_file)?;
    let lowered = context.engine.lowered(current_file)?;

    let mut highlights = vec![];

    for (expression_id, expression_kind) in lowered.info.iter_expression() {
        if let ExpressionKind::Constructor { resolution: Some((f_id, t_id)) }
        | ExpressionKind::OperatorName { resolution: Some((f_id, t_id)) }
        | ExpressionKind::Variable {
            resolution: Some(TermVariableResolution::Reference(f_id, t_id)),
            ..
        } = expression_kind
            && (*f_id, *t_id) == (file_id, term_id)
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, expression_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    for (binder_id, binder_kind) in lowered.info.iter_binder() {
        if let BinderKind::Constructor { resolution: Some((f_id, t_id)), .. } = binder_kind
            && (*f_id, *t_id) == (file_id, term_id)
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, binder_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    for (operator_id, f_id, t_id) in lowered.info.iter_term_operator() {
        if (f_id, t_id) == (file_id, term_id) {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, operator_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    for (pun_id, resolution) in lowered.info.iter_expression_pun() {
        if let TermVariableResolution::Reference(f_id, t_id) = resolution
            && (f_id, t_id) == (file_id, term_id)
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, pun_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    if file_id == current_file
        && let Some(definition_highlights) =
            value_equation_highlights(context, current_file, term_id)?
    {
        highlights.extend(definition_highlights);
    }

    Ok(finish_highlights(highlights))
}

fn highlight_term_operator(
    context: &LanguageContext,
    current_file: FileId,
    operator_id: TermOperatorId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let lowered = context.engine.lowered(current_file)?;
    let (file_id, term_id) =
        lowered.info.get_term_operator(operator_id).ok_or(AnalyzerError::NonFatal)?;
    highlight_file_term(context, current_file, file_id, term_id)
}

fn highlight_let(
    context: &LanguageContext,
    current_file: FileId,
    let_binding_id: LetBindingNameGroupId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let content = context.engine.content(current_file);
    let (parsed, _) = context.engine.parsed(current_file)?;
    let stabilized = context.engine.stabilized(current_file)?;
    let lowered = context.engine.lowered(current_file)?;

    let root = parsed.syntax_node();
    let binding = lowered.info.get_let_binding_group(let_binding_id);

    let mut highlights: Vec<DocumentHighlight> = vec![];

    if let Some(signature) = binding.signature {
        let ptr = stabilized.syntax_ptr(signature).ok_or(AnalyzerError::NonFatal)?;
        highlights.extend(
            let_signature_name_range(&content, &root, &ptr)
                .or_else(|| locate::syntax_range(&content, &root, &ptr))
                .and_then(|range| document_highlight(&content, context.encoding, range)),
        );
    }

    for &equation in binding.equations.iter() {
        let ptr = stabilized.syntax_ptr(equation).ok_or(AnalyzerError::NonFatal)?;
        highlights.extend(
            let_equation_name_range(&content, &root, &ptr)
                .or_else(|| locate::syntax_range(&content, &root, &ptr))
                .and_then(|range| document_highlight(&content, context.encoding, range)),
        );
    }

    for (expr_id, expr_kind) in lowered.info.iter_expression() {
        if let ExpressionKind::Variable {
            resolution: Some(TermVariableResolution::Let(id)), ..
        } = expr_kind
            && *id == let_binding_id
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, expr_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    for (pun_id, resolution) in lowered.info.iter_expression_pun() {
        if let TermVariableResolution::Let(id) = resolution
            && id == let_binding_id
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, pun_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    Ok(finish_highlights(highlights))
}

fn highlight_binder_pun(
    context: &LanguageContext,
    current_file: FileId,
    pun_id: RecordPunId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let content = context.engine.content(current_file);
    let (parsed, _) = context.engine.parsed(current_file)?;
    let stabilized = context.engine.stabilized(current_file)?;
    let lowered = context.engine.lowered(current_file)?;

    let mut highlights = vec![];

    highlights.extend(
        locate::id_range(&content, &parsed, &stabilized, pun_id)
            .and_then(|range| document_highlight(&content, context.encoding, range)),
    );

    for (expression_id, expression_kind) in lowered.info.iter_expression() {
        if let ExpressionKind::Variable {
            resolution: Some(TermVariableResolution::RecordPun(candidate_id)),
        } = expression_kind
            && *candidate_id == pun_id
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, expression_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    for (expression_pun_id, resolution) in lowered.info.iter_expression_pun() {
        if let TermVariableResolution::RecordPun(candidate_id) = resolution
            && candidate_id == pun_id
        {
            highlights.extend(
                locate::id_range(&content, &parsed, &stabilized, expression_pun_id)
                    .and_then(|range| document_highlight(&content, context.encoding, range)),
            );
        }
    }

    Ok(finish_highlights(highlights))
}

fn highlight_expression_pun(
    context: &LanguageContext,
    current_file: FileId,
    pun_id: RecordPunId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let lowered = context.engine.lowered(current_file)?;
    match lowered.info.get_expression_pun(pun_id).ok_or(AnalyzerError::NonFatal)? {
        TermVariableResolution::Binder(binder_id) => {
            highlight_binder(context, current_file, binder_id)
        }
        TermVariableResolution::Let(let_binding_id) => {
            highlight_let(context, current_file, let_binding_id)
        }
        TermVariableResolution::RecordPun(pun_id) => {
            highlight_binder_pun(context, current_file, pun_id)
        }
        TermVariableResolution::Reference(file_id, term_id) => {
            highlight_file_term(context, current_file, file_id, term_id)
        }
    }
}

fn value_equation_highlights(
    context: &LanguageContext,
    current_file: FileId,
    term_id: TermItemId,
) -> Result<Option<Vec<DocumentHighlight>>, AnalyzerError> {
    let content = context.engine.content(current_file);

    let (parsed, _) = context.engine.parsed(current_file)?;
    let root = parsed.syntax_node();

    let stabilized = context.engine.stabilized(current_file)?;
    let indexed = context.engine.indexed(current_file)?;

    let Some(equations) =
        locate::value_equation_ranges(&content, &root, &stabilized, &indexed, term_id)
    else {
        return Ok(None);
    };

    let highlights = equations.into_iter().filter_map(|range| {
        let range = position::utf8_range_to_protocol(&content, range, context.encoding)?;
        Some(DocumentHighlight { range, kind: None })
    });

    let highlights = highlights.collect();
    Ok(finish_highlights(highlights))
}

fn binder_name_range(content: &str, root: &SyntaxNode, ptr: &SyntaxNodePtr) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;

    if let Some(binder) = cst::BinderVariable::cast(node.clone()) {
        let token = binder.name_token()?;
        return position::text_range_to_utf8_range(content, token.text_range());
    }

    if let Some(binder) = cst::BinderNamed::cast(node) {
        let token = binder.name_token()?;
        return position::text_range_to_utf8_range(content, token.text_range());
    }

    None
}

fn let_signature_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let signature = cst::LetBindingSignature::cast(node)?;
    let token = signature.name_token()?;
    position::text_range_to_utf8_range(content, token.text_range())
}

fn let_equation_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let equation = cst::LetBindingEquation::cast(node)?;
    let token = equation.name_token()?;
    position::text_range_to_utf8_range(content, token.text_range())
}

fn document_highlight(
    content: &str,
    encoding: PositionEncoding,
    range: Utf8Range,
) -> Option<DocumentHighlight> {
    let range = position::utf8_range_to_protocol(content, range, encoding)?;
    Some(DocumentHighlight { range, kind: None })
}

fn finish_highlights(mut highlights: Vec<DocumentHighlight>) -> Option<Vec<DocumentHighlight>> {
    highlights.sort_by_key(|DocumentHighlight { range, .. }| {
        (range.start.line, range.start.character, range.end.line, range.end.character)
    });
    highlights.dedup_by(|left, right| left.range == right.range);

    let has_highlights = !highlights.is_empty();
    has_highlights.then_some(highlights)
}
