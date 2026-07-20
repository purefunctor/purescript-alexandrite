use std::collections::HashMap;
use std::path::Path;

use async_lsp::lsp_types::*;
use files::FileId;
use indexing::{
    ImplicitItems, ImportId, ImportItemId, TermItemId, TermItemKind, TypeItemId, TypeItemKind,
    TypeSelection,
};
use lowering::{BinderKind, ExpressionKind, TermVariableResolution, TypeKind};
use stabilizing::AstId;
use syntax::ast::{AstNode, AstPtr};
use syntax::{
    SyntaxKind, SyntaxNode, SyntaxNodePtr, SyntaxToken, TextRange, TokenAtOffset, WalkEvent, cst,
};

use crate::position::Utf8Range;
use crate::{AnalyzerError, LanguageContext, common, locate, position, references};

#[derive(Clone, Copy)]
enum RenameTarget {
    Term(FileId, TermItemId),
    Type(FileId, TypeItemId),
    Qualifier(FileId, ImportId),
    Module(FileId),
}

#[derive(Clone, Copy)]
enum NameKind {
    Lower,
    Upper,
    Operator,
    Module,
}

pub fn implementation(
    context: &LanguageContext,
    workspace_root: Option<&Path>,
    uri: Url,
    position: Position,
    new_name: String,
) -> Result<Option<WorkspaceEdit>, AnalyzerError> {
    let current_file = {
        let uri = uri.as_str();
        context.files.id(uri).ok_or(AnalyzerError::NonFatal)?
    };

    let content = context.engine.content(current_file);
    let utf8_position = position::protocol_position_to_utf8(&content, position, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;

    let target = if let Some(target) = qualifier_target(context, current_file, utf8_position)? {
        target
    } else {
        let located = locate::locate(context.engine, current_file, utf8_position)?;
        rename_target(context, current_file, located)?
    };

    let Some((old_name, name_kind)) = target_name(context, target)? else {
        return Ok(None);
    };
    if old_name == new_name || !valid_new_name(&new_name, name_kind) {
        return Ok(None);
    }

    let target_file = match target {
        RenameTarget::Term(file_id, _)
        | RenameTarget::Type(file_id, _)
        | RenameTarget::Qualifier(file_id, _)
        | RenameTarget::Module(file_id) => file_id,
    };
    if !editable_file(context, workspace_root, target_file) {
        return Ok(None);
    }

    if let RenameTarget::Qualifier(file_id, import_id) = target {
        let mut edits = vec![];
        qualifier_edits(context, file_id, import_id, &new_name, &mut edits)?;

        return finish_workspace_edit(context, edits);
    }

    if let RenameTarget::Module(file_id) = target {
        let mut edits = vec![];
        module_edits(context, workspace_root, file_id, &new_name, &mut edits)?;

        return finish_workspace_edit(context, edits);
    }

    let locations = references::implementation(context, uri, position)?.unwrap_or_default();
    let mut edits = vec![];

    for location in locations {
        let file_id = context.files.id(location.uri.as_str()).ok_or(AnalyzerError::NonFatal)?;
        if !editable_file(context, workspace_root, file_id) {
            continue;
        }

        let range = reference_name_range(context, file_id, location.range, name_kind)?;
        let replacement = replacement_text(context, file_id, range, &new_name)?;
        edits.push((file_id, TextEdit { range, new_text: replacement }));
    }

    declaration_edits(context, target, &new_name, &mut edits)?;
    item_surface_edits(context, workspace_root, target, &old_name, &new_name, &mut edits)?;

    finish_workspace_edit(context, edits)
}

fn qualifier_target(
    context: &LanguageContext,
    current_file: FileId,
    position: position::Utf8Position,
) -> Result<Option<RenameTarget>, AnalyzerError> {
    let content = context.engine.content(current_file);
    let offset =
        position::utf8_position_to_offset(&content, position).ok_or(AnalyzerError::NonFatal)?;
    let (parsed, _) = context.engine.parsed(current_file)?;
    let root = parsed.syntax_node();

    let token = match root.token_at_offset(offset) {
        TokenAtOffset::None => return Ok(None),
        TokenAtOffset::Single(token) => token,
        TokenAtOffset::Between(_, right) => right,
    };

    let qualifier_name = token.parent_ancestors().find_map(|node| {
        let qualifier = cst::Qualifier::cast(node)?;
        let parent = qualifier.syntax().parent()?;
        if !cst::QualifiedName::can_cast(parent.kind()) {
            return None;
        }

        let text = qualifier.text()?.text(&content);
        Some(text.trim_end_matches('.').to_string())
    });

    let alias_name = token.parent_ancestors().find_map(|node| {
        let module_name = cst::ModuleName::cast(node)?;
        let parent = module_name.syntax().parent()?;
        let parent_kind = parent.kind();
        let is_alias = cst::ImportAlias::can_cast(parent_kind);
        let is_export = cst::ExportModule::can_cast(parent_kind);

        if !is_alias && !is_export {
            return None;
        }

        Some(module_name.syntax().text(&content).to_string())
    });

    let Some(name) = qualifier_name.or(alias_name) else {
        return Ok(None);
    };

    let indexed = context.engine.indexed(current_file)?;
    let import_id = indexed.imports.iter().find_map(|(import_id, import)| {
        (import.alias.as_deref() == Some(name.as_str())).then_some(*import_id)
    });

    Ok(import_id.map(|import_id| RenameTarget::Qualifier(current_file, import_id)))
}

fn qualifier_edits(
    context: &LanguageContext,
    file_id: FileId,
    import_id: ImportId,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let indexed = context.engine.indexed(file_id)?;
    let old_name = indexed
        .imports
        .get(&import_id)
        .and_then(|import| import.alias.as_deref())
        .ok_or(AnalyzerError::NonFatal)?;

    let content = context.engine.content(file_id);
    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();

    let statements = root.preorder().filter_map(|event| {
        let WalkEvent::Enter(node) = event else {
            return None;
        };

        cst::ImportStatement::cast(node)
    });

    for statement in statements {
        let Some(module_name) = statement.import_alias().and_then(|alias| alias.module_name())
        else {
            continue;
        };
        if module_name.syntax().text(&content) != old_name {
            continue;
        }

        push_text_range_edit(context, file_id, module_name.syntax().text_range(), new_name, edits)?;
    }

    let qualified_names = root.preorder().filter_map(|event| {
        let WalkEvent::Enter(node) = event else {
            return None;
        };

        cst::QualifiedName::cast(node)
    });

    for qualified in qualified_names {
        let Some(qualifier) = qualified.qualifier() else {
            continue;
        };
        let Some(token) = qualifier.text() else {
            continue;
        };
        if token.text(&content).trim_end_matches('.') != old_name {
            continue;
        }

        let replacement = format!("{new_name}.");
        push_text_range_edit(context, file_id, token.text_range(), &replacement, edits)?;
    }

    let exports = root.preorder().filter_map(|event| {
        let WalkEvent::Enter(node) = event else {
            return None;
        };

        cst::ExportModule::cast(node)
    });

    for export in exports {
        let Some(module_name) = export.module_name() else {
            continue;
        };
        if module_name.syntax().text(&content) != old_name {
            continue;
        }

        push_text_range_edit(context, file_id, module_name.syntax().text_range(), new_name, edits)?;
    }

    Ok(())
}

fn module_edits(
    context: &LanguageContext,
    workspace_root: Option<&Path>,
    target_file: FileId,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let (parsed, _) = context.engine.parsed(target_file)?;
    let module_name =
        parsed.cst().header().and_then(|header| header.name()).ok_or(AnalyzerError::NonFatal)?;

    push_text_range_edit(context, target_file, module_name.syntax().text_range(), new_name, edits)?;

    for file_id in context.files.iter_id() {
        if !editable_file(context, workspace_root, file_id) {
            continue;
        }

        module_import_edits(context, file_id, target_file, new_name, edits)?;
        module_export_edits(context, file_id, target_file, new_name, edits)?;
    }

    Ok(())
}

fn module_import_edits(
    context: &LanguageContext,
    file_id: FileId,
    target_file: FileId,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();
    let indexed = context.engine.indexed(file_id)?;
    let stabilized = context.engine.stabilized(file_id)?;

    for (import_id, import) in &indexed.imports {
        let Some(name) = import.name.as_deref() else {
            continue;
        };
        if context.engine.module_file(name) != Some(target_file) {
            continue;
        }

        let ptr = stabilized.ast_ptr(*import_id).ok_or(AnalyzerError::NonFatal)?;
        let statement = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
        let module_name = statement.module_name().ok_or(AnalyzerError::NonFatal)?;

        push_text_range_edit(context, file_id, module_name.syntax().text_range(), new_name, edits)?;
    }

    Ok(())
}

fn module_export_edits(
    context: &LanguageContext,
    file_id: FileId,
    target_file: FileId,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let content = context.engine.content(file_id);
    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();
    let indexed = context.engine.indexed(file_id)?;
    let stabilized = context.engine.stabilized(file_id)?;
    let current_module = parsed.module_name(&content);

    for export in &indexed.exports.modules {
        let exports_self = file_id == target_file && current_module.as_ref() == Some(&export.name);
        let exports_import = indexed.imports.values().any(|import| {
            import.alias.is_none()
                && import.name.as_ref() == Some(&export.name)
                && context.engine.module_file(&export.name) == Some(target_file)
        });

        if !exports_self && !exports_import {
            continue;
        }

        let ptr = stabilized.ast_ptr(export.id).ok_or(AnalyzerError::NonFatal)?;
        let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
        let cst::ExportItem::ExportModule(export) = item else {
            return Err(AnalyzerError::NonFatal);
        };
        let module_name = export.module_name().ok_or(AnalyzerError::NonFatal)?;

        push_text_range_edit(context, file_id, module_name.syntax().text_range(), new_name, edits)?;
    }

    Ok(())
}

fn rename_target(
    context: &LanguageContext,
    current_file: FileId,
    located: locate::Located,
) -> Result<RenameTarget, AnalyzerError> {
    let lowered = context.engine.lowered(current_file)?;

    let target = match located {
        locate::Located::ModuleName(module_name) => {
            module_target(context, current_file, module_name)?
        }
        locate::Located::ImportItem(import_id) => import_target(context, current_file, import_id)?,
        locate::Located::Binder(binder_id) => {
            let kind = lowered.info.get_binder_kind(binder_id).ok_or(AnalyzerError::NonFatal)?;

            let BinderKind::Constructor { resolution: Some((file_id, term_id)), .. } = kind else {
                return Err(AnalyzerError::NonFatal);
            };

            RenameTarget::Term(*file_id, *term_id)
        }
        locate::Located::Expression(expression_id) => {
            let kind =
                lowered.info.get_expression_kind(expression_id).ok_or(AnalyzerError::NonFatal)?;

            let resolution = match kind {
                ExpressionKind::Constructor { resolution: Some(resolution) }
                | ExpressionKind::OperatorName { resolution: Some(resolution) } => *resolution,
                ExpressionKind::Variable {
                    resolution: Some(TermVariableResolution::Reference(file_id, term_id)),
                } => (*file_id, *term_id),
                _ => return Err(AnalyzerError::NonFatal),
            };

            RenameTarget::Term(resolution.0, resolution.1)
        }
        locate::Located::Type(type_id) => {
            let kind = lowered.info.get_type_kind(type_id).ok_or(AnalyzerError::NonFatal)?;

            let resolution = match kind {
                TypeKind::Constructor { resolution: Some(resolution) }
                | TypeKind::Operator { resolution: Some(resolution) } => *resolution,
                _ => return Err(AnalyzerError::NonFatal),
            };

            RenameTarget::Type(resolution.0, resolution.1)
        }
        locate::Located::TermOperator(operator_id) => {
            let (file_id, term_id) =
                lowered.info.get_term_operator(operator_id).ok_or(AnalyzerError::NonFatal)?;

            RenameTarget::Term(file_id, term_id)
        }
        locate::Located::TypeOperator(operator_id) => {
            let (file_id, type_id) =
                lowered.info.get_type_operator(operator_id).ok_or(AnalyzerError::NonFatal)?;

            RenameTarget::Type(file_id, type_id)
        }
        locate::Located::TermItem(term_id) => RenameTarget::Term(current_file, term_id),
        locate::Located::TypeItem(type_id) => RenameTarget::Type(current_file, type_id),
        _ => return Err(AnalyzerError::NonFatal),
    };

    Ok(target)
}

fn module_target(
    context: &LanguageContext,
    current_file: FileId,
    module_name: AstPtr<cst::ModuleName>,
) -> Result<RenameTarget, AnalyzerError> {
    let content = context.engine.content(current_file);
    let (parsed, _) = context.engine.parsed(current_file)?;
    let root = parsed.syntax_node();
    let module_name = module_name.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
    let parent = module_name.syntax().parent().ok_or(AnalyzerError::NonFatal)?;
    let parent_kind = parent.kind();

    if cst::ModuleHeader::can_cast(parent_kind) {
        return Ok(RenameTarget::Module(current_file));
    }

    if !cst::ImportStatement::can_cast(parent_kind) && !cst::ExportModule::can_cast(parent_kind) {
        return Err(AnalyzerError::NonFatal);
    }

    let name = module_name.syntax().text(&content);
    let file_id = context.engine.module_file(name).ok_or(AnalyzerError::NonFatal)?;

    Ok(RenameTarget::Module(file_id))
}

fn import_target(
    context: &LanguageContext,
    current_file: FileId,
    import_id: ImportItemId,
) -> Result<RenameTarget, AnalyzerError> {
    let content = context.engine.content(current_file);
    let (parsed, _) = context.engine.parsed(current_file)?;
    let stabilized = context.engine.stabilized(current_file)?;

    let root = parsed.syntax_node();
    let ptr = stabilized.ast_ptr(import_id).ok_or(AnalyzerError::NonFatal)?;
    let node = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;

    let statement = node
        .syntax()
        .ancestors()
        .find_map(cst::ImportStatement::cast)
        .ok_or(AnalyzerError::NonFatal)?;
    let module_name =
        statement.module_name().ok_or(AnalyzerError::NonFatal)?.syntax().text(&content).to_string();

    let imported_file = context.engine.module_file(&module_name).ok_or(AnalyzerError::NonFatal)?;
    let resolved = context.engine.resolved(imported_file)?;

    let target = match node {
        cst::ImportItem::ImportValue(item) => {
            let name = item.name_token().ok_or(AnalyzerError::NonFatal)?.text(&content);

            let (file_id, term_id) =
                resolved.exports.lookup_term(name).ok_or(AnalyzerError::NonFatal)?;

            RenameTarget::Term(file_id, term_id)
        }
        cst::ImportItem::ImportClass(item) => {
            let name = item.name_token().ok_or(AnalyzerError::NonFatal)?.text(&content);

            let (file_id, type_id) = resolved
                .exports
                .lookup_class(name)
                .or_else(|| resolved.exports.lookup_type(name))
                .ok_or(AnalyzerError::NonFatal)?;

            RenameTarget::Type(file_id, type_id)
        }
        cst::ImportItem::ImportType(item) => {
            let name = item.name_token().ok_or(AnalyzerError::NonFatal)?.text(&content);

            let (file_id, type_id) = resolved
                .exports
                .lookup_type(name)
                .or_else(|| resolved.exports.lookup_class(name))
                .ok_or(AnalyzerError::NonFatal)?;

            RenameTarget::Type(file_id, type_id)
        }
        cst::ImportItem::ImportOperator(item) => {
            let name = item.name_token().ok_or(AnalyzerError::NonFatal)?.text(&content);

            let (file_id, term_id) = resolved
                .exports
                .lookup_term(trim_operator_name(name))
                .ok_or(AnalyzerError::NonFatal)?;

            RenameTarget::Term(file_id, term_id)
        }
        cst::ImportItem::ImportTypeOperator(item) => {
            let name = item.name_token().ok_or(AnalyzerError::NonFatal)?.text(&content);

            let (file_id, type_id) = resolved
                .exports
                .lookup_type(trim_operator_name(name))
                .ok_or(AnalyzerError::NonFatal)?;

            RenameTarget::Type(file_id, type_id)
        }
    };

    Ok(target)
}

fn trim_operator_name(name: &str) -> &str {
    name.trim_start_matches('(').trim_end_matches(')')
}

fn target_name(
    context: &LanguageContext,
    target: RenameTarget,
) -> Result<Option<(String, NameKind)>, AnalyzerError> {
    let result = match target {
        RenameTarget::Term(file_id, term_id) => {
            let indexed = context.engine.indexed(file_id)?;
            let item = &indexed.items[term_id];

            let Some(name) = item.name.as_ref() else {
                return Ok(None);
            };

            let kind = match item.kind {
                TermItemKind::Constructor { .. } => NameKind::Upper,
                TermItemKind::Operator { .. } => NameKind::Operator,
                TermItemKind::Derive { .. } | TermItemKind::Instance { .. } => return Ok(None),
                _ => NameKind::Lower,
            };

            Some((name.to_string(), kind))
        }
        RenameTarget::Type(file_id, type_id) => {
            let indexed = context.engine.indexed(file_id)?;
            let item = &indexed.items[type_id];

            let Some(name) = item.name.as_ref() else {
                return Ok(None);
            };

            let kind = match item.kind {
                TypeItemKind::Operator { .. } => NameKind::Operator,
                _ => NameKind::Upper,
            };

            Some((name.to_string(), kind))
        }
        RenameTarget::Qualifier(file_id, import_id) => {
            let indexed = context.engine.indexed(file_id)?;
            let name = indexed.imports.get(&import_id).and_then(|import| import.alias.as_ref());

            name.map(|name| (name.to_string(), NameKind::Upper))
        }
        RenameTarget::Module(file_id) => {
            let content = context.engine.content(file_id);
            let (parsed, _) = context.engine.parsed(file_id)?;

            parsed.module_name(&content).map(|name| (name.to_string(), NameKind::Module))
        }
    };

    Ok(result)
}

fn valid_new_name(new_name: &str, kind: NameKind) -> bool {
    if matches!(kind, NameKind::Module) {
        return !new_name.is_empty()
            && new_name.split('.').all(|segment| valid_new_name(segment, NameKind::Upper));
    }

    let lexed = lexing::lex(new_name);
    let is_single_token = lexed.len() == 2 && lexed.kind(1) == SyntaxKind::END_OF_FILE;
    if !is_single_token {
        return false;
    }

    let has_lexing_error = lexed.error(0).is_some() || lexed.error(1).is_some();
    let has_qualifier = lexed.qualifier(0).is_some();
    if has_lexing_error || has_qualifier {
        return false;
    }

    if lexed.text(0) != new_name {
        return false;
    }

    let token_kind = lexed.kind(0);

    match kind {
        NameKind::Lower => token_kind == SyntaxKind::LOWER,
        NameKind::Upper => token_kind == SyntaxKind::UPPER,
        NameKind::Operator => matches!(
            token_kind,
            SyntaxKind::OPERATOR
                | SyntaxKind::COLON
                | SyntaxKind::MINUS
                | SyntaxKind::DOUBLE_PERIOD
                | SyntaxKind::LEFT_THICK_ARROW
        ),
        NameKind::Module => unreachable!(),
    }
}

fn editable_file(
    context: &LanguageContext,
    workspace_root: Option<&Path>,
    file_id: FileId,
) -> bool {
    let Some(workspace_root) = workspace_root else {
        return true;
    };

    let path = context.files.path(file_id);
    let Ok(uri) = Url::parse(&path) else {
        return false;
    };
    let Ok(path) = uri.to_file_path() else {
        return false;
    };

    path.starts_with(workspace_root)
}

fn reference_name_range(
    context: &LanguageContext,
    file_id: FileId,
    range: Range,
    name_kind: NameKind,
) -> Result<Range, AnalyzerError> {
    let content = context.engine.content(file_id);
    let position = position::protocol_position_to_utf8(&content, range.start, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;
    let offset =
        position::utf8_position_to_offset(&content, position).ok_or(AnalyzerError::NonFatal)?;

    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();

    let token = match root.token_at_offset(offset) {
        TokenAtOffset::None => return Err(AnalyzerError::NonFatal),
        TokenAtOffset::Single(token) => token,
        TokenAtOffset::Between(left, right) => {
            if qualified_name_token(&right, name_kind).is_some() { right } else { left }
        }
    };

    let token = qualified_name_token(&token, name_kind).ok_or(AnalyzerError::NonFatal)?;

    position::text_range_to_protocol(&content, token.text_range(), context.encoding)
        .ok_or(AnalyzerError::NonFatal)
}

fn qualified_name_token(token: &SyntaxToken, name_kind: NameKind) -> Option<SyntaxToken> {
    let qualified = token.parent_ancestors().find_map(cst::QualifiedName::cast)?;

    match name_kind {
        NameKind::Lower => qualified.lower(),
        NameKind::Upper => qualified.upper(),
        NameKind::Operator => qualified.operator().or_else(|| qualified.operator_name()),
        NameKind::Module => None,
    }
}

fn replacement_text(
    context: &LanguageContext,
    file_id: FileId,
    range: Range,
    new_name: &str,
) -> Result<String, AnalyzerError> {
    let content = context.engine.content(file_id);
    let start = position::protocol_position_to_utf8(&content, range.start, context.encoding)
        .and_then(|position| position::utf8_position_to_offset(&content, position))
        .ok_or(AnalyzerError::NonFatal)?;
    let end = position::protocol_position_to_utf8(&content, range.end, context.encoding)
        .and_then(|position| position::utf8_position_to_offset(&content, position))
        .ok_or(AnalyzerError::NonFatal)?;

    let range = TextRange::new(start, end);
    let text = &content[range];

    if text.starts_with('(') && text.ends_with(')') {
        Ok(format!("({new_name})"))
    } else {
        Ok(new_name.to_string())
    }
}

fn declaration_edits(
    context: &LanguageContext,
    target: RenameTarget,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    match target {
        RenameTarget::Term(file_id, term_id) => {
            term_declaration_edits(context, file_id, term_id, new_name, edits)
        }
        RenameTarget::Type(file_id, type_id) => {
            type_declaration_edits(context, file_id, type_id, new_name, edits)
        }
        RenameTarget::Qualifier(_, _) => Ok(()),
        RenameTarget::Module(_) => Ok(()),
    }
}

fn term_declaration_edits(
    context: &LanguageContext,
    file_id: FileId,
    term_id: TermItemId,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let indexed = context.engine.indexed(file_id)?;

    macro_rules! push_name_edits {
        ($range:expr; $($id:expr),+ $(,)?) => {
            $(push_name_edit(context, file_id, $id, $range, new_name, edits)?;)+
        };
    }

    match &indexed.items[term_id].kind {
        TermItemKind::ClassMember { id } => {
            push_name_edits!(position::class_member_name_range; Some(*id));
        }
        TermItemKind::Constructor { id } => {
            push_name_edits!(position::data_constructor_name_range; Some(*id));
        }
        TermItemKind::Derive { id } => {
            push_name_edits!(position::declaration_name_range; Some(*id));
        }
        TermItemKind::Foreign { id } => {
            push_name_edits!(position::declaration_name_range; Some(*id));
        }
        TermItemKind::Instance { id } => {
            push_name_edits!(position::instance_declaration_name_range; Some(*id));
        }
        TermItemKind::Operator { id } => {
            push_name_edits!(position::infix_operator_range; Some(*id));
        }
        TermItemKind::Value { signature, equations } => {
            push_name_edits!(position::declaration_name_range; *signature);

            for &equation in equations {
                push_name_edits!(position::declaration_name_range; Some(equation));
            }
        }
    }

    Ok(())
}

fn type_declaration_edits(
    context: &LanguageContext,
    file_id: FileId,
    type_id: TypeItemId,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let indexed = context.engine.indexed(file_id)?;

    macro_rules! push_name_edits {
        ($range:expr; $($id:expr),+ $(,)?) => {
            $(push_name_edit(context, file_id, $id, $range, new_name, edits)?;)+
        };
    }

    match indexed.items[type_id].kind {
        TypeItemKind::Data { signature, equation, role, .. } => {
            push_name_edits!(position::declaration_name_range; signature, equation, role);
        }
        TypeItemKind::Newtype { signature, equation, role, .. } => {
            push_name_edits!(position::declaration_name_range; signature, equation, role);
        }
        TypeItemKind::Synonym { signature, equation } => {
            push_name_edits!(position::declaration_name_range; signature, equation);
        }
        TypeItemKind::Class { signature, declaration, .. } => {
            push_name_edits!(position::declaration_name_range; signature, declaration);
        }
        TypeItemKind::Foreign { id, role } => {
            push_name_edits!(position::declaration_name_range; Some(id), role);
        }
        TypeItemKind::Operator { id } => {
            push_name_edits!(position::infix_operator_range; Some(id));
        }
    }

    Ok(())
}

fn item_surface_edits(
    context: &LanguageContext,
    workspace_root: Option<&Path>,
    target: RenameTarget,
    old_name: &str,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    for file_id in context.files.iter_id() {
        if !editable_file(context, workspace_root, file_id) {
            continue;
        }

        import_edits(context, file_id, target, old_name, new_name, edits)?;
        export_edits(context, file_id, target, old_name, new_name, edits)?;
    }

    Ok(())
}

fn import_edits(
    context: &LanguageContext,
    file_id: FileId,
    target: RenameTarget,
    old_name: &str,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let indexed = context.engine.indexed(file_id)?;
    let resolved = context.engine.resolved(file_id)?;

    let unqualified = resolved.unqualified.values().flatten();
    let qualified = resolved.qualified.values().flatten();

    for import in unqualified.chain(qualified) {
        let Some(indexed_import) = indexed.imports.get(&import.id) else {
            continue;
        };

        match target {
            RenameTarget::Term(target_file, target_term) => {
                let imported_terms = import.iter_terms().filter(|(_, file_id, term_id, _)| {
                    (*file_id, *term_id) == (target_file, target_term)
                });

                for (name, _, _, _) in imported_terms {
                    if let Some(import_item_id) = indexed_import.terms.get(name) {
                        push_import_item_edit(context, file_id, *import_item_id, new_name, edits)?;
                    }
                }

                constructor_import_edits(
                    context,
                    file_id,
                    import,
                    indexed_import,
                    (target_file, target_term),
                    (old_name, new_name),
                    edits,
                )?;
            }
            RenameTarget::Type(target_file, target_type) => {
                let imported_types = import.iter_types().chain(import.iter_classes()).filter(
                    |(_, file_id, type_id, _)| (*file_id, *type_id) == (target_file, target_type),
                );

                for (name, _, _, _) in imported_types {
                    if let Some((import_item_id, _)) = indexed_import.types.get(name) {
                        push_import_item_edit(context, file_id, *import_item_id, new_name, edits)?;
                    }
                }
            }
            RenameTarget::Qualifier(_, _) => {}
            RenameTarget::Module(_) => {}
        }
    }

    Ok(())
}

fn constructor_import_edits(
    context: &LanguageContext,
    file_id: FileId,
    import: &resolving::ResolvedImport,
    indexed_import: &indexing::IndexedImport,
    target: (FileId, TermItemId),
    names: (&str, &str),
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let (target_file, target_term) = target;
    let (old_name, new_name) = names;
    let target_indexed = context.engine.indexed(target_file)?;
    let Some(parent_type) = target_indexed.constructor_type(target_term) else {
        return Ok(());
    };

    let imported_types = import
        .iter_types()
        .filter(|(_, file_id, type_id, _)| (*file_id, *type_id) == (target_file, parent_type));

    for (name, _, _, _) in imported_types {
        let Some((import_item_id, Some(selection))) = indexed_import.types.get(name) else {
            continue;
        };
        let ImplicitItems::Enumerated(constructors) = selection else {
            continue;
        };
        if !constructors.iter().any(|constructor| constructor == old_name) {
            continue;
        }

        push_import_constructor_edit(context, file_id, *import_item_id, old_name, new_name, edits)?;
    }

    Ok(())
}

fn export_edits(
    context: &LanguageContext,
    file_id: FileId,
    target: RenameTarget,
    old_name: &str,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let indexed = context.engine.indexed(file_id)?;
    let resolved = context.engine.resolved(file_id)?;

    match target {
        RenameTarget::Term(target_file, target_term) => {
            for export in &indexed.exports.terms {
                if resolved.exports.lookup_term(&export.name) == Some((target_file, target_term)) {
                    push_export_item_edit(context, file_id, export.id, new_name, edits)?;
                }
            }

            let target_indexed = context.engine.indexed(target_file)?;
            let Some(parent_type) = target_indexed.constructor_type(target_term) else {
                return Ok(());
            };

            for export in &indexed.exports.types {
                if resolved.exports.lookup_type(&export.name) != Some((target_file, parent_type)) {
                    continue;
                }
                let Some(TypeSelection::Enumerated(constructors)) = &export.selection else {
                    continue;
                };
                if !constructors.iter().any(|constructor| constructor == old_name) {
                    continue;
                }

                push_export_constructor_edit(
                    context, file_id, export.id, old_name, new_name, edits,
                )?;
            }
        }
        RenameTarget::Type(target_file, target_type) => {
            for export in &indexed.exports.types {
                let exported_type = resolved
                    .exports
                    .lookup_type(&export.name)
                    .or_else(|| resolved.exports.lookup_class(&export.name));

                if exported_type == Some((target_file, target_type)) {
                    push_export_item_edit(context, file_id, export.id, new_name, edits)?;
                }
            }
        }
        RenameTarget::Qualifier(_, _) => {}
        RenameTarget::Module(_) => {}
    }

    Ok(())
}

fn push_import_item_edit(
    context: &LanguageContext,
    file_id: FileId,
    import_item_id: ImportItemId,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let content = context.engine.content(file_id);
    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();
    let stabilized = context.engine.stabilized(file_id)?;

    let ptr = stabilized.ast_ptr(import_item_id).ok_or(AnalyzerError::NonFatal)?;
    let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
    let range = position::import_item_name_range(&content, item).ok_or(AnalyzerError::NonFatal)?;

    push_utf8_edit(context, file_id, range, new_name, edits)
}

fn push_export_item_edit(
    context: &LanguageContext,
    file_id: FileId,
    export_item_id: indexing::ExportItemId,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let content = context.engine.content(file_id);
    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();
    let stabilized = context.engine.stabilized(file_id)?;

    let ptr = stabilized.ast_ptr(export_item_id).ok_or(AnalyzerError::NonFatal)?;
    let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
    let range = position::export_item_name_range(&content, item).ok_or(AnalyzerError::NonFatal)?;

    push_utf8_edit(context, file_id, range, new_name, edits)
}

fn push_import_constructor_edit(
    context: &LanguageContext,
    file_id: FileId,
    import_item_id: ImportItemId,
    old_name: &str,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();
    let stabilized = context.engine.stabilized(file_id)?;

    let ptr = stabilized.ast_ptr(import_item_id).ok_or(AnalyzerError::NonFatal)?;
    let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
    let cst::ImportItem::ImportType(item) = item else {
        return Err(AnalyzerError::NonFatal);
    };

    let type_items = item.type_items().ok_or(AnalyzerError::NonFatal)?;
    push_constructor_token_edit(context, file_id, type_items, old_name, new_name, edits)
}

fn push_export_constructor_edit(
    context: &LanguageContext,
    file_id: FileId,
    export_item_id: indexing::ExportItemId,
    old_name: &str,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();
    let stabilized = context.engine.stabilized(file_id)?;

    let ptr = stabilized.ast_ptr(export_item_id).ok_or(AnalyzerError::NonFatal)?;
    let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
    let cst::ExportItem::ExportType(item) = item else {
        return Err(AnalyzerError::NonFatal);
    };

    let type_items = item.type_items().ok_or(AnalyzerError::NonFatal)?;
    push_constructor_token_edit(context, file_id, type_items, old_name, new_name, edits)
}

fn push_constructor_token_edit(
    context: &LanguageContext,
    file_id: FileId,
    type_items: cst::TypeItems,
    old_name: &str,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let content = context.engine.content(file_id);
    let cst::TypeItems::TypeItemsList(items) = type_items else {
        return Ok(());
    };

    for token in items.name_tokens() {
        if token.text(&content) == old_name {
            let range = position::text_range_to_utf8_range(&content, token.text_range())
                .ok_or(AnalyzerError::NonFatal)?;

            push_utf8_edit(context, file_id, range, new_name, edits)?;
        }
    }

    Ok(())
}

fn push_text_range_edit(
    context: &LanguageContext,
    file_id: FileId,
    range: TextRange,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let content = context.engine.content(file_id);
    let range =
        position::text_range_to_utf8_range(&content, range).ok_or(AnalyzerError::NonFatal)?;

    push_utf8_edit(context, file_id, range, new_name, edits)
}

fn push_utf8_edit(
    context: &LanguageContext,
    file_id: FileId,
    range: Utf8Range,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError> {
    let content = context.engine.content(file_id);
    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;
    let replacement = replacement_text(context, file_id, range, new_name)?;

    edits.push((file_id, TextEdit { range, new_text: replacement }));

    Ok(())
}

fn push_name_edit<T>(
    context: &LanguageContext,
    file_id: FileId,
    id: Option<AstId<T>>,
    range: fn(&str, &SyntaxNode, &SyntaxNodePtr) -> Option<Utf8Range>,
    new_name: &str,
    edits: &mut Vec<(FileId, TextEdit)>,
) -> Result<(), AnalyzerError>
where
    T: AstNode,
{
    let Some(id) = id else {
        return Ok(());
    };

    let content = context.engine.content(file_id);
    let (parsed, _) = context.engine.parsed(file_id)?;
    let root = parsed.syntax_node();
    let stabilized = context.engine.stabilized(file_id)?;

    let ptr = stabilized.syntax_ptr(id).ok_or(AnalyzerError::NonFatal)?;
    let range = range(&content, &root, &ptr).ok_or(AnalyzerError::NonFatal)?;
    let range = position::utf8_range_to_protocol(&content, range, context.encoding)
        .ok_or(AnalyzerError::NonFatal)?;
    let replacement = replacement_text(context, file_id, range, new_name)?;

    edits.push((file_id, TextEdit { range, new_text: replacement }));

    Ok(())
}

fn finish_workspace_edit(
    context: &LanguageContext,
    mut edits: Vec<(FileId, TextEdit)>,
) -> Result<Option<WorkspaceEdit>, AnalyzerError> {
    edits.sort_by_key(|(file_id, edit)| {
        (
            file_id.into_raw().into_u32(),
            edit.range.start.line,
            edit.range.start.character,
            edit.range.end.line,
            edit.range.end.character,
        )
    });
    edits.dedup_by(|left, right| left.0 == right.0 && left.1.range == right.1.range);

    if edits.is_empty() {
        return Ok(None);
    }

    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::default();
    for (file_id, edit) in edits {
        let uri = common::file_uri(context, file_id)?;
        changes.entry(uri).or_default().push(edit);
    }

    Ok(Some(WorkspaceEdit { changes: Some(changes), ..WorkspaceEdit::default() }))
}

#[cfg(test)]
mod tests {
    use super::{NameKind, valid_new_name};

    #[test]
    fn validates_new_names_by_kind() {
        assert!(valid_new_name("renamed", NameKind::Lower));
        assert!(valid_new_name("Renamed", NameKind::Upper));
        assert!(valid_new_name("Library.Renamed", NameKind::Module));
        assert!(!valid_new_name("Renamed", NameKind::Lower));
        assert!(!valid_new_name("renamed", NameKind::Upper));
        assert!(!valid_new_name("", NameKind::Module));
        assert!(!valid_new_name("Library.renamed", NameKind::Module));
        assert!(!valid_new_name("Library..Renamed", NameKind::Module));
        assert!(!valid_new_name("Library Renamed", NameKind::Module));
        assert!(!valid_new_name("two names", NameKind::Lower));
        assert!(!valid_new_name("Library.renamed", NameKind::Lower));
        assert!(!valid_new_name("renamed ", NameKind::Lower));
    }

    #[test]
    fn validates_operator_names() {
        assert!(valid_new_name("<~>", NameKind::Operator));
        assert!(valid_new_name(":", NameKind::Operator));
        assert!(valid_new_name("-", NameKind::Operator));
        assert!(valid_new_name("..", NameKind::Operator));
        assert!(valid_new_name("<=", NameKind::Operator));
        assert!(!valid_new_name("renamed", NameKind::Operator));
        assert!(!valid_new_name("(<~>)", NameKind::Operator));
    }
}
