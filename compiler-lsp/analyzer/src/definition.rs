use std::iter;

use async_lsp::lsp_types::*;
use files::FileId;
use indexing::{ImportItemId, TermItemId, TypeItemId};
use lowering::{
    BinderId, BinderKind, ExpressionId, ExpressionKind, ImplicitTypeVariable,
    LetBindingNameGroupId, TermVariableResolution, TypeId, TypeKind, TypeVariableResolution,
};
use smol_str::ToSmolStr;
use syntax::ast::{AstNode, AstPtr};
use syntax::{SyntaxNode, SyntaxNodePtr, cst};

use crate::extract::AnnotationSyntaxRange;
use crate::position::Utf8Range;
use crate::{AnalyzerError, LanguageContext, common, locate, position};

pub fn implementation(
    context: &LanguageContext,
    uri: Url,
    position: Position,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let current_file = {
        let uri = uri.as_str();
        context.files.id(uri).ok_or(AnalyzerError::NonFatal)?
    };

    let content = context.engine.content(current_file);
    let position = position::protocol_position_to_utf8(&content, position, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;

    let located = locate::locate(context.engine, current_file, position)?;

    match located {
        locate::Located::ModuleName(module_name) => {
            definition_module_name(context, current_file, module_name)
        }
        locate::Located::ImportItem(import_id) => {
            definition_import(context, current_file, import_id)
        }
        locate::Located::Binder(binder_id) => definition_binder(context, current_file, binder_id),
        locate::Located::Expression(expression_id) => {
            definition_expression(context, uri, current_file, expression_id)
        }
        locate::Located::Type(type_id) => definition_type(context, uri, current_file, type_id),
        locate::Located::TermOperator(operator_id) => {
            let lowered = context.engine.lowered(current_file)?;
            let (f_id, t_id) =
                lowered.info.get_term_operator(operator_id).ok_or(AnalyzerError::NonFatal)?;
            definition_file_term(context, f_id, t_id)
        }
        locate::Located::TypeOperator(operator_id) => {
            let lowered = context.engine.lowered(current_file)?;
            let (f_id, t_id) =
                lowered.info.get_type_operator(operator_id).ok_or(AnalyzerError::NonFatal)?;
            definition_file_type(context, f_id, t_id)
        }
        locate::Located::TermItem(term_id) => definition_file_term(context, current_file, term_id),
        locate::Located::TypeItem(type_id) => definition_file_type(context, current_file, type_id),
        locate::Located::LetBinding(let_id) => {
            definition_let_binding(context, current_file, let_id)
        }
        locate::Located::BinderPun(_) => Ok(None),
        locate::Located::ExpressionPun(_) => Ok(None),
        locate::Located::Nothing => Ok(None),
    }
}

fn definition_module_name(
    context: &LanguageContext,
    current_file: FileId,
    module_name: AstPtr<cst::ModuleName>,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(current_file);
    let (parsed, _) = engine.parsed(current_file)?;

    let root = parsed.syntax_node();
    let module_name = module_name.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;

    let module_name = module_name.syntax().text(&content).to_smolstr();
    let module_id = engine.module_file(&module_name).ok_or(AnalyzerError::NonFatal)?;

    let content = engine.content(module_id);

    let (parsed, _) = engine.parsed(module_id)?;
    let root = parsed.syntax_node();

    let range = root.text_range();

    let uri = common::file_uri(context, module_id)?;
    let range = position::text_range_to_protocol(&content, range, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;

    Ok(Some(GotoDefinitionResponse::Scalar(Location { uri, range })))
}

fn definition_import(
    context: &LanguageContext,
    current_file: FileId,
    import_id: ImportItemId,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(current_file);
    let (parsed, _) = engine.parsed(current_file)?;
    let stabilized = engine.stabilized(current_file)?;

    let root = parsed.syntax_node();
    let ptr = stabilized.ast_ptr(import_id).ok_or(AnalyzerError::NonFatal)?;
    let node = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;

    let statement = node
        .syntax()
        .ancestors()
        .find_map(cst::ImportStatement::cast)
        .ok_or(AnalyzerError::NonFatal)?;
    let module_name = statement
        .module_name()
        .ok_or(AnalyzerError::NonFatal)?
        .syntax()
        .text(&content)
        .to_smolstr();

    let import_resolved = {
        let import_id = engine.module_file(&module_name).ok_or(AnalyzerError::NonFatal)?;
        engine.resolved(import_id)?
    };

    let goto_term = |name: &str| {
        let name = name.trim_start_matches("(").trim_end_matches(")");
        let (f_id, t_id) =
            import_resolved.exports.lookup_term(name).ok_or(AnalyzerError::NonFatal)?;
        definition_file_term(context, f_id, t_id)
    };

    let goto_type = |name: &str| {
        let name = name.trim_start_matches("(").trim_end_matches(")");
        let (f_id, t_id) = import_resolved
            .exports
            .lookup_type(name)
            .or_else(|| import_resolved.exports.lookup_class(name))
            .ok_or(AnalyzerError::NonFatal)?;
        definition_file_type(context, f_id, t_id)
    };

    let goto_class = |name: &str| {
        let name = name.trim_start_matches("(").trim_end_matches(")");
        let (f_id, t_id) = import_resolved
            .exports
            .lookup_class(name)
            .or_else(|| import_resolved.exports.lookup_type(name))
            .ok_or(AnalyzerError::NonFatal)?;
        definition_file_type(context, f_id, t_id)
    };

    match node {
        cst::ImportItem::ImportValue(cst) => {
            let token = cst.name_token().ok_or(AnalyzerError::NonFatal)?;
            let name = token.text(&content);
            goto_term(name)
        }
        cst::ImportItem::ImportClass(cst) => {
            let token = cst.name_token().ok_or(AnalyzerError::NonFatal)?;
            let name = token.text(&content);
            goto_class(name)
        }
        cst::ImportItem::ImportType(cst) => {
            let token = cst.name_token().ok_or(AnalyzerError::NonFatal)?;
            let name = token.text(&content);
            goto_type(name)
        }
        cst::ImportItem::ImportOperator(cst) => {
            let token = cst.name_token().ok_or(AnalyzerError::NonFatal)?;
            let name = token.text(&content);
            goto_term(name)
        }
        cst::ImportItem::ImportTypeOperator(cst) => {
            let token = cst.name_token().ok_or(AnalyzerError::NonFatal)?;
            let name = token.text(&content);
            goto_type(name)
        }
    }
}

fn definition_binder(
    context: &LanguageContext,
    current_file: FileId,
    binder_id: BinderId,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let lowered = context.engine.lowered(current_file)?;
    let kind = lowered.info.get_binder_kind(binder_id).ok_or(AnalyzerError::NonFatal)?;
    match kind {
        BinderKind::Constructor { resolution, .. } => {
            let (f_id, t_id) = resolution.as_ref().ok_or(AnalyzerError::NonFatal)?;
            definition_file_term(context, *f_id, *t_id)
        }
        _ => Ok(None),
    }
}

fn definition_expression(
    context: &LanguageContext,
    uri: Url,
    current_file: FileId,
    expression_id: ExpressionId,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(current_file);
    let (parsed, _) = engine.parsed(current_file)?;

    let stabilized = engine.stabilized(current_file)?;
    let lowered = engine.lowered(current_file)?;

    let kind = lowered.info.get_expression_kind(expression_id).ok_or(AnalyzerError::NonFatal)?;

    match kind {
        ExpressionKind::Constructor { resolution, .. } => {
            let (f_id, t_id) = resolution.as_ref().ok_or(AnalyzerError::NonFatal)?;
            definition_file_term(context, *f_id, *t_id)
        }
        ExpressionKind::Variable { resolution, .. } => {
            let resolution = resolution.as_ref().ok_or(AnalyzerError::NonFatal)?;
            match resolution {
                TermVariableResolution::Binder(id) => {
                    let root = parsed.syntax_node();
                    let ptr = stabilized.syntax_ptr(*id).ok_or(AnalyzerError::NonFatal)?;
                    let range = locate::syntax_range(&content, &root, &ptr)
                        .ok_or(AnalyzerError::NonFatal)?;
                    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
                        .ok_or(AnalyzerError::NonFatal)?;
                    Ok(Some(GotoDefinitionResponse::Scalar(Location { uri, range })))
                }
                TermVariableResolution::Let(binding_id) => {
                    let root = parsed.syntax_node();

                    let binding = lowered.info.get_let_binding_group(*binding_id);

                    let signature = binding
                        .signature
                        .and_then(|id| {
                            let ptr = stabilized.syntax_ptr(id)?;
                            locate::syntax_range(&content, &root, &ptr)
                        })
                        .into_iter();

                    let equations = binding.equations.iter().filter_map(|&id| {
                        let ptr = stabilized.syntax_ptr(id)?;
                        locate::syntax_range(&content, &root, &ptr)
                    });

                    let range = signature
                        .chain(equations)
                        .reduce(|start, end| Utf8Range { start: start.start, end: end.end })
                        .ok_or(AnalyzerError::NonFatal)?;
                    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
                        .ok_or(AnalyzerError::NonFatal)?;

                    Ok(Some(GotoDefinitionResponse::Scalar(Location { uri, range })))
                }
                TermVariableResolution::RecordPun(id) => {
                    let root = parsed.syntax_node();
                    let ptr = stabilized.syntax_ptr(*id).ok_or(AnalyzerError::NonFatal)?;
                    let range = record_pun_name_range(&content, &root, &ptr)
                        .ok_or(AnalyzerError::NonFatal)?;
                    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
                        .ok_or(AnalyzerError::NonFatal)?;
                    Ok(Some(GotoDefinitionResponse::Scalar(Location { uri, range })))
                }
                TermVariableResolution::Reference(f_id, t_id) => {
                    definition_file_term(context, *f_id, *t_id)
                }
            }
        }
        ExpressionKind::OperatorName { resolution, .. } => {
            let (f_id, t_id) = resolution.as_ref().ok_or(AnalyzerError::NonFatal)?;
            definition_file_term(context, *f_id, *t_id)
        }
        _ => Ok(None),
    }
}

fn record_pun_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let pun = cst::RecordPun::cast(node)?;

    let name = pun.name()?;
    let name = name.syntax();
    let range = AnnotationSyntaxRange::from_node(name).syntax?;

    position::text_range_to_utf8_range(content, range)
}

fn definition_type(
    context: &LanguageContext,
    uri: Url,
    current_file: FileId,
    type_id: TypeId,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(current_file);
    let (parsed, _) = engine.parsed(current_file)?;
    let stabilized = engine.stabilized(current_file)?;
    let lowered = engine.lowered(current_file)?;

    let kind = lowered.info.get_type_kind(type_id).ok_or(AnalyzerError::NonFatal)?;
    match kind {
        TypeKind::Constructor { resolution, .. } => {
            let (f_id, t_id) = resolution.as_ref().ok_or(AnalyzerError::NonFatal)?;
            definition_file_type(context, *f_id, *t_id)
        }
        TypeKind::Operator { resolution, .. } => {
            let (f_id, t_id) = resolution.as_ref().ok_or(AnalyzerError::NonFatal)?;
            definition_file_type(context, *f_id, *t_id)
        }
        TypeKind::Variable { resolution, .. } => {
            let resolution = resolution.as_ref().ok_or(AnalyzerError::NonFatal)?;
            match resolution {
                TypeVariableResolution::Forall(binding) => {
                    let root = parsed.syntax_node();
                    let ptr = stabilized
                        .ast_ptr(*binding)
                        .ok_or(AnalyzerError::NonFatal)?
                        .syntax_node_ptr();
                    let range = locate::syntax_range(&content, &root, &ptr)
                        .ok_or(AnalyzerError::NonFatal)?;
                    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
                        .ok_or(AnalyzerError::NonFatal)?;
                    Ok(Some(GotoDefinitionResponse::Scalar(Location { uri, range })))
                }
                TypeVariableResolution::Implicit(ImplicitTypeVariable { .. }) => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

fn definition_file_term(
    context: &LanguageContext,
    file_id: FileId,
    term_id: TermItemId,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let uri = common::file_uri(context, file_id)?;
    let location = common::file_term_location(context, uri, file_id, term_id)?;
    Ok(Some(GotoDefinitionResponse::Scalar(location)))
}

fn definition_file_type(
    context: &LanguageContext,
    file_id: FileId,
    type_id: TypeItemId,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let uri = common::file_uri(context, file_id)?;
    let location = common::file_type_location(context, uri, file_id, type_id)?;
    Ok(Some(GotoDefinitionResponse::Scalar(location)))
}

fn definition_let_binding(
    context: &LanguageContext,
    file_id: FileId,
    let_id: LetBindingNameGroupId,
) -> Result<Option<GotoDefinitionResponse>, AnalyzerError> {
    let engine = context.engine;
    let content = engine.content(file_id);
    let (parsed, _) = engine.parsed(file_id)?;
    let stabilized = engine.stabilized(file_id)?;
    let lowered = engine.lowered(file_id)?;

    let root = parsed.syntax_node();

    let uri = common::file_uri(context, file_id)?;
    let group = lowered.info.get_let_binding_group(let_id);

    let signature = group.signature.and_then(|signature| stabilized.syntax_ptr(signature));
    let equations = group.equations.iter().filter_map(|&equation| stabilized.syntax_ptr(equation));

    let pointers = iter::chain(signature, equations);
    let range = common::pointers_range(&content, root, pointers)?;
    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;

    Ok(Some(GotoDefinitionResponse::Scalar(Location { uri, range })))
}
