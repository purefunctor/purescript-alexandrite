use building_types::QueryProxy;
use files::FileId;
use indexing::{ImportId, ImportItemId, TermItemId, TermItemKind, TypeItemId, TypeItemKind};
use lowering::{BinderKind, ExpressionKind, TermVariableResolution, TypeKind};
use lsp_types::*;
use syntax::ast::{AstNode, AstPtr};
use syntax::{TokenAtOffset, cst};

use crate::{AnalyzerContext, AnalyzerError, locate, position, references};

mod edit;

#[derive(Clone, Copy)]
enum RenameTarget {
    Term(FileId, TermItemId),
    Type(FileId, TypeItemId),
    Qualifier(FileId, ImportId),
    Module(FileId),
}

impl RenameTarget {
    fn file(self) -> FileId {
        match self {
            RenameTarget::Term(file_id, _)
            | RenameTarget::Type(file_id, _)
            | RenameTarget::Qualifier(file_id, _)
            | RenameTarget::Module(file_id) => file_id,
        }
    }
}

#[derive(Clone, Copy)]
enum NameKind {
    Lower,
    Upper,
    Operator,
    Module,
}

pub fn implementation(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    uri: Url,
    position: Position,
    new_name: String,
) -> Result<Option<WorkspaceEdit>, AnalyzerError> {
    let current_file = {
        let uri = uri.as_str();
        context.file_id(uri).ok_or(AnalyzerError::NonFatal)?
    };

    let content = context.queries().content(current_file);
    let utf8_position =
        position::protocol_position_to_utf8(&content, position, context.position_encoding())
            .ok_or(AnalyzerError::NonFatal)?;

    let target = if let Some(target) = qualifier_target(context, current_file, utf8_position)? {
        target
    } else {
        let located = locate::locate(context.queries(), current_file, utf8_position)?;
        rename_target(context, current_file, located)?
    };

    let Some((old_name, name_kind)) = target_name(context, target)? else {
        return Ok(None);
    };
    if old_name == new_name || !valid_new_name(&new_name, name_kind) {
        return Ok(None);
    }

    if !context.is_editable(target.file()) {
        return Ok(None);
    }

    let mut edits = edit::RenameEdits::new(context);
    match target {
        RenameTarget::Qualifier(file_id, import_id) => {
            edits.collect_qualifier(file_id, import_id, &new_name)?;
        }
        RenameTarget::Module(file_id) => {
            edits.collect_module(file_id, &new_name)?;
        }
        RenameTarget::Term(_, _) | RenameTarget::Type(_, _) => {
            let locations = references::implementation(context, uri, position)?.unwrap_or_default();
            edits.collect_references(locations, name_kind, &new_name)?;
            edits.collect_declaration(target, &new_name)?;
            edits.collect_item_surfaces(target, &old_name, &new_name)?;
        }
    }

    edits.finish()
}

fn qualifier_target(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    current_file: FileId,
    position: position::Utf8Position,
) -> Result<Option<RenameTarget>, AnalyzerError> {
    let content = context.queries().content(current_file);
    let offset =
        position::utf8_position_to_offset(&content, position).ok_or(AnalyzerError::NonFatal)?;
    let (parsed, _) = context.queries().parsed(current_file)?;
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

    let indexed = context.queries().indexed(current_file)?;
    let import_id = indexed.imports.iter().find_map(|(import_id, import)| {
        (import.alias.as_deref() == Some(name.as_str())).then_some(*import_id)
    });

    Ok(import_id.map(|import_id| RenameTarget::Qualifier(current_file, import_id)))
}

fn rename_target(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    current_file: FileId,
    located: locate::Located,
) -> Result<RenameTarget, AnalyzerError> {
    let lowered = context.queries().lowered(current_file)?;

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
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    current_file: FileId,
    module_name: AstPtr<cst::ModuleName>,
) -> Result<RenameTarget, AnalyzerError> {
    let content = context.queries().content(current_file);
    let (parsed, _) = context.queries().parsed(current_file)?;
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
    let file_id = context.queries().module_file(name).ok_or(AnalyzerError::NonFatal)?;

    Ok(RenameTarget::Module(file_id))
}

fn import_target(
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    current_file: FileId,
    import_id: ImportItemId,
) -> Result<RenameTarget, AnalyzerError> {
    let content = context.queries().content(current_file);
    let (parsed, _) = context.queries().parsed(current_file)?;
    let stabilized = context.queries().stabilized(current_file)?;

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

    let imported_file =
        context.queries().module_file(&module_name).ok_or(AnalyzerError::NonFatal)?;
    let resolved = context.queries().resolved(imported_file)?;

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
    context: &AnalyzerContext<impl crate::AnalyzerHost>,
    target: RenameTarget,
) -> Result<Option<(String, NameKind)>, AnalyzerError> {
    let result = match target {
        RenameTarget::Term(file_id, term_id) => {
            let indexed = context.queries().indexed(file_id)?;
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
            let indexed = context.queries().indexed(file_id)?;
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
            let indexed = context.queries().indexed(file_id)?;
            let name = indexed.imports.get(&import_id).and_then(|import| import.alias.as_ref());

            name.map(|name| (name.to_string(), NameKind::Upper))
        }
        RenameTarget::Module(file_id) => {
            let content = context.queries().content(file_id);
            let (parsed, _) = context.queries().parsed(file_id)?;

            parsed.module_name(&content).map(|name| (name.to_string(), NameKind::Module))
        }
    };

    Ok(result)
}

fn valid_new_name(new_name: &str, kind: NameKind) -> bool {
    match kind {
        NameKind::Lower => lexing::is_lower_name(new_name),
        NameKind::Upper => lexing::is_upper_name(new_name),
        NameKind::Operator => lexing::is_operator_name(new_name),
        NameKind::Module => new_name.split('.').all(lexing::is_upper_name),
    }
}

#[cfg(test)]
mod tests {
    use super::{NameKind, valid_new_name};

    #[test]
    fn validates_new_names_by_kind() {
        assert!(valid_new_name("renamed", NameKind::Lower));
        assert!(valid_new_name("éclair", NameKind::Lower));
        assert!(valid_new_name("Renamed", NameKind::Upper));
        assert!(valid_new_name("Éclair", NameKind::Upper));
        assert!(valid_new_name("Library.Renamed", NameKind::Module));
        assert!(!valid_new_name("Renamed", NameKind::Lower));
        assert!(!valid_new_name("renamed", NameKind::Upper));
        assert!(!valid_new_name("module", NameKind::Lower));
        assert!(!valid_new_name("_", NameKind::Lower));
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
        assert!(valid_new_name("⊗", NameKind::Operator));
        assert!(!valid_new_name("renamed", NameKind::Operator));
        assert!(!valid_new_name("(<~>)", NameKind::Operator));
        assert!(!valid_new_name("=", NameKind::Operator));
        assert!(!valid_new_name("->", NameKind::Operator));
        assert!(!valid_new_name("--", NameKind::Operator));
    }
}
