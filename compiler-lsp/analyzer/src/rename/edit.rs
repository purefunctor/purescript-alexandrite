use std::collections::HashMap;
use std::path::Path;

use files::FileId;
use indexing::{
    ImplicitItems, ImportId, ImportItemId, TermItemId, TermItemKind, TypeItemId, TypeItemKind,
    TypeSelection,
};
use lsp_types::*;
use stabilizing::AstId;
use syntax::ast::AstNode;
use syntax::{SyntaxNode, SyntaxNodePtr, SyntaxToken, TextRange, TokenAtOffset, WalkEvent, cst};

use super::{NameKind, RenameTarget, editable_file};
use crate::position::Utf8Range;
use crate::{AnalyzerError, AnalyzerQueries, FileCatalog, LanguageContext, common, position};

macro_rules! push_name_edits {
    ($self:expr, $file_id:expr, $new_name:expr, $range:expr; $($id:expr),+ $(,)?) => {
        $($self.push_name_edit($file_id, $id, $range, $new_name)?;)+
    };
}

pub(super) struct RenameEdits<'edits, 'language, Queries, Catalog>
where
    Queries: AnalyzerQueries,
    Catalog: FileCatalog,
{
    context: &'edits LanguageContext<'language, Queries, Catalog>,
    edits: Vec<(FileId, TextEdit)>,
}

impl<'edits, 'language, Queries, Catalog> RenameEdits<'edits, 'language, Queries, Catalog>
where
    Queries: AnalyzerQueries,
    Catalog: FileCatalog,
{
    pub(super) fn new(
        context: &'edits LanguageContext<'language, Queries, Catalog>,
    ) -> RenameEdits<'edits, 'language, Queries, Catalog> {
        RenameEdits { context, edits: vec![] }
    }
}

impl<'edits, 'language, Queries, Catalog> RenameEdits<'edits, 'language, Queries, Catalog>
where
    Queries: AnalyzerQueries,
    Catalog: FileCatalog,
{
    pub(super) fn collect_qualifier(
        &mut self,
        file_id: FileId,
        import_id: ImportId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let indexed = self.context.engine.indexed(file_id)?;
        let old_name = indexed
            .imports
            .get(&import_id)
            .and_then(|import| import.alias.as_deref())
            .ok_or(AnalyzerError::NonFatal)?;

        let content = self.context.engine.content(file_id);
        let (parsed, _) = self.context.engine.parsed(file_id)?;
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

            self.push_text_range_edit(file_id, module_name.syntax().text_range(), new_name)?;
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

            let new_name = format!("{new_name}.");
            self.push_text_range_edit(file_id, token.text_range(), &new_name)?;
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

            self.push_text_range_edit(file_id, module_name.syntax().text_range(), new_name)?;
        }

        Ok(())
    }

    pub(super) fn collect_module(
        &mut self,
        workspace_root: Option<&Path>,
        target_file: FileId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let (parsed, _) = self.context.engine.parsed(target_file)?;
        let module_name = parsed
            .cst()
            .header()
            .and_then(|header| header.name())
            .ok_or(AnalyzerError::NonFatal)?;

        self.push_text_range_edit(target_file, module_name.syntax().text_range(), new_name)?;

        for file_id in self.context.files.active_files() {
            if !editable_file(self.context, workspace_root, file_id) {
                continue;
            }

            self.module_import_edits(file_id, target_file, new_name)?;
            self.module_export_edits(file_id, target_file, new_name)?;
        }

        Ok(())
    }

    fn module_import_edits(
        &mut self,
        file_id: FileId,
        target_file: FileId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let (parsed, _) = self.context.engine.parsed(file_id)?;
        let root = parsed.syntax_node();
        let indexed = self.context.engine.indexed(file_id)?;
        let stabilized = self.context.engine.stabilized(file_id)?;

        for (import_id, import) in &indexed.imports {
            let Some(name) = import.name.as_deref() else {
                continue;
            };
            if self.context.engine.module_file(name) != Some(target_file) {
                continue;
            }

            let ptr = stabilized.ast_ptr(*import_id).ok_or(AnalyzerError::NonFatal)?;
            let statement = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
            let module_name = statement.module_name().ok_or(AnalyzerError::NonFatal)?;

            self.push_text_range_edit(file_id, module_name.syntax().text_range(), new_name)?;
        }

        Ok(())
    }

    fn module_export_edits(
        &mut self,
        file_id: FileId,
        target_file: FileId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.engine.content(file_id);
        let (parsed, _) = self.context.engine.parsed(file_id)?;
        let root = parsed.syntax_node();
        let indexed = self.context.engine.indexed(file_id)?;
        let stabilized = self.context.engine.stabilized(file_id)?;
        let current_module = parsed.module_name(&content);

        for export in &indexed.exports.modules {
            let exports_self =
                file_id == target_file && current_module.as_ref() == Some(&export.name);
            let exports_import = indexed.imports.values().any(|import| {
                import.alias.is_none()
                    && import.name.as_ref() == Some(&export.name)
                    && self.context.engine.module_file(&export.name) == Some(target_file)
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

            self.push_text_range_edit(file_id, module_name.syntax().text_range(), new_name)?;
        }

        Ok(())
    }
}

impl<'edits, 'language, Queries, Catalog> RenameEdits<'edits, 'language, Queries, Catalog>
where
    Queries: AnalyzerQueries,
    Catalog: FileCatalog,
{
    pub(super) fn collect_references(
        &mut self,
        workspace_root: Option<&Path>,
        locations: Vec<Location>,
        name_kind: NameKind,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        for location in locations {
            let file_id =
                self.context.files.file_id(location.uri.as_str()).ok_or(AnalyzerError::NonFatal)?;
            if !editable_file(self.context, workspace_root, file_id) {
                continue;
            }

            let range = self.reference_name_range(file_id, location.range, name_kind)?;
            self.push_protocol_edit(file_id, range, new_name)?;
        }

        Ok(())
    }

    fn reference_name_range(
        &self,
        file_id: FileId,
        range: Range,
        name_kind: NameKind,
    ) -> Result<Range, AnalyzerError> {
        let content = self.context.engine.content(file_id);
        let position =
            position::protocol_position_to_utf8(&content, range.start, self.context.encoding)
                .ok_or(AnalyzerError::NonFatal)?;
        let offset =
            position::utf8_position_to_offset(&content, position).ok_or(AnalyzerError::NonFatal)?;

        let (parsed, _) = self.context.engine.parsed(file_id)?;
        let root = parsed.syntax_node();

        let token = match root.token_at_offset(offset) {
            TokenAtOffset::None => return Err(AnalyzerError::NonFatal),
            TokenAtOffset::Single(token) => token,
            TokenAtOffset::Between(left, right) => {
                if Self::qualified_name_token(&right, name_kind).is_some() { right } else { left }
            }
        };

        let token = Self::qualified_name_token(&token, name_kind).ok_or(AnalyzerError::NonFatal)?;

        position::text_range_to_protocol(&content, token.text_range(), self.context.encoding)
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
}

impl<'edits, 'language, Queries, Catalog> RenameEdits<'edits, 'language, Queries, Catalog>
where
    Queries: AnalyzerQueries,
    Catalog: FileCatalog,
{
    pub(super) fn collect_declaration(
        &mut self,
        target: RenameTarget,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        match target {
            RenameTarget::Term(file_id, term_id) => {
                self.term_declaration_edits(file_id, term_id, new_name)
            }
            RenameTarget::Type(file_id, type_id) => {
                self.type_declaration_edits(file_id, type_id, new_name)
            }
            RenameTarget::Qualifier(_, _) => Ok(()),
            RenameTarget::Module(_) => Ok(()),
        }
    }

    fn term_declaration_edits(
        &mut self,
        file_id: FileId,
        term_id: TermItemId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let indexed = self.context.engine.indexed(file_id)?;

        match &indexed.items[term_id].kind {
            TermItemKind::ClassMember { id } => {
                push_name_edits!(self, file_id, new_name, position::class_member_name_range; Some(*id));
            }
            TermItemKind::Constructor { id } => {
                push_name_edits!(self, file_id, new_name, position::data_constructor_name_range; Some(*id));
            }
            TermItemKind::Derive { id } => {
                push_name_edits!(self, file_id, new_name, position::declaration_name_range; Some(*id));
            }
            TermItemKind::Foreign { id } => {
                push_name_edits!(self, file_id, new_name, position::declaration_name_range; Some(*id));
            }
            TermItemKind::Instance { id } => {
                push_name_edits!(self, file_id, new_name, position::instance_declaration_name_range; Some(*id));
            }
            TermItemKind::Operator { id } => {
                push_name_edits!(self, file_id, new_name, position::infix_operator_range; Some(*id));
            }
            TermItemKind::Value { signature, equations } => {
                push_name_edits!(self, file_id, new_name, position::declaration_name_range; *signature);

                for &equation in equations {
                    push_name_edits!(self, file_id, new_name, position::declaration_name_range; Some(equation));
                }
            }
        }

        Ok(())
    }

    fn type_declaration_edits(
        &mut self,
        file_id: FileId,
        type_id: TypeItemId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let indexed = self.context.engine.indexed(file_id)?;

        match indexed.items[type_id].kind {
            TypeItemKind::Data { signature, equation, role, .. } => {
                push_name_edits!(self, file_id, new_name, position::declaration_name_range; signature, equation, role);
            }
            TypeItemKind::Newtype { signature, equation, role, .. } => {
                push_name_edits!(self, file_id, new_name, position::declaration_name_range; signature, equation, role);
            }
            TypeItemKind::Synonym { signature, equation } => {
                push_name_edits!(self, file_id, new_name, position::declaration_name_range; signature, equation);
            }
            TypeItemKind::Class { signature, declaration, .. } => {
                push_name_edits!(self, file_id, new_name, position::declaration_name_range; signature, declaration);
            }
            TypeItemKind::Foreign { id, role } => {
                push_name_edits!(self, file_id, new_name, position::declaration_name_range; Some(id), role);
            }
            TypeItemKind::Operator { id } => {
                push_name_edits!(self, file_id, new_name, position::infix_operator_range; Some(id));
            }
        }

        Ok(())
    }
}

impl<'edits, 'language, Queries, Catalog> RenameEdits<'edits, 'language, Queries, Catalog>
where
    Queries: AnalyzerQueries,
    Catalog: FileCatalog,
{
    pub(super) fn collect_item_surfaces(
        &mut self,
        workspace_root: Option<&Path>,
        target: RenameTarget,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        for file_id in self.context.files.active_files() {
            if !editable_file(self.context, workspace_root, file_id) {
                continue;
            }

            self.import_edits(file_id, target, old_name, new_name)?;
            self.export_edits(file_id, target, old_name, new_name)?;
        }

        Ok(())
    }

    fn import_edits(
        &mut self,
        file_id: FileId,
        target: RenameTarget,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let indexed = self.context.engine.indexed(file_id)?;
        let resolved = self.context.engine.resolved(file_id)?;

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
                            self.push_import_item_edit(file_id, *import_item_id, new_name)?;
                        }
                    }

                    self.constructor_import_edits(
                        file_id,
                        import,
                        indexed_import,
                        (target_file, target_term),
                        (old_name, new_name),
                    )?;
                }
                RenameTarget::Type(target_file, target_type) => {
                    let imported_types = import.iter_types().chain(import.iter_classes()).filter(
                        |(_, file_id, type_id, _)| {
                            (*file_id, *type_id) == (target_file, target_type)
                        },
                    );

                    for (name, _, _, _) in imported_types {
                        if let Some((import_item_id, _)) = indexed_import.types.get(name) {
                            self.push_import_item_edit(file_id, *import_item_id, new_name)?;
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
        &mut self,
        file_id: FileId,
        import: &resolving::ResolvedImport,
        indexed_import: &indexing::IndexedImport,
        target: (FileId, TermItemId),
        names: (&str, &str),
    ) -> Result<(), AnalyzerError> {
        let (target_file, target_term) = target;
        let (old_name, new_name) = names;
        let target_indexed = self.context.engine.indexed(target_file)?;
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

            self.push_import_constructor_edit(file_id, *import_item_id, old_name, new_name)?;
        }

        Ok(())
    }

    fn export_edits(
        &mut self,
        file_id: FileId,
        target: RenameTarget,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let indexed = self.context.engine.indexed(file_id)?;
        let resolved = self.context.engine.resolved(file_id)?;

        match target {
            RenameTarget::Term(target_file, target_term) => {
                for export in &indexed.exports.terms {
                    if resolved.exports.lookup_term(&export.name)
                        == Some((target_file, target_term))
                    {
                        self.push_export_item_edit(file_id, export.id, new_name)?;
                    }
                }

                let target_indexed = self.context.engine.indexed(target_file)?;
                let Some(parent_type) = target_indexed.constructor_type(target_term) else {
                    return Ok(());
                };

                for export in &indexed.exports.types {
                    if resolved.exports.lookup_type(&export.name)
                        != Some((target_file, parent_type))
                    {
                        continue;
                    }
                    let Some(TypeSelection::Enumerated(constructors)) = &export.selection else {
                        continue;
                    };
                    if !constructors.iter().any(|constructor| constructor == old_name) {
                        continue;
                    }

                    self.push_export_constructor_edit(file_id, export.id, old_name, new_name)?;
                }
            }
            RenameTarget::Type(target_file, target_type) => {
                for export in &indexed.exports.types {
                    let exported_type = resolved
                        .exports
                        .lookup_type(&export.name)
                        .or_else(|| resolved.exports.lookup_class(&export.name));

                    if exported_type == Some((target_file, target_type)) {
                        self.push_export_item_edit(file_id, export.id, new_name)?;
                    }
                }
            }
            RenameTarget::Qualifier(_, _) => {}
            RenameTarget::Module(_) => {}
        }

        Ok(())
    }

    fn push_import_item_edit(
        &mut self,
        file_id: FileId,
        import_item_id: ImportItemId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.engine.content(file_id);
        let (parsed, _) = self.context.engine.parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.engine.stabilized(file_id)?;

        let ptr = stabilized.ast_ptr(import_item_id).ok_or(AnalyzerError::NonFatal)?;
        let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
        let range =
            position::import_item_name_range(&content, item).ok_or(AnalyzerError::NonFatal)?;

        self.push_utf8_edit(file_id, range, new_name)
    }

    fn push_export_item_edit(
        &mut self,
        file_id: FileId,
        export_item_id: indexing::ExportItemId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.engine.content(file_id);
        let (parsed, _) = self.context.engine.parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.engine.stabilized(file_id)?;

        let ptr = stabilized.ast_ptr(export_item_id).ok_or(AnalyzerError::NonFatal)?;
        let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
        let range =
            position::export_item_name_range(&content, item).ok_or(AnalyzerError::NonFatal)?;

        self.push_utf8_edit(file_id, range, new_name)
    }

    fn push_import_constructor_edit(
        &mut self,
        file_id: FileId,
        import_item_id: ImportItemId,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let (parsed, _) = self.context.engine.parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.engine.stabilized(file_id)?;

        let ptr = stabilized.ast_ptr(import_item_id).ok_or(AnalyzerError::NonFatal)?;
        let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
        let cst::ImportItem::ImportType(item) = item else {
            return Err(AnalyzerError::NonFatal);
        };

        let type_items = item.type_items().ok_or(AnalyzerError::NonFatal)?;
        self.push_constructor_token_edit(file_id, type_items, old_name, new_name)
    }

    fn push_export_constructor_edit(
        &mut self,
        file_id: FileId,
        export_item_id: indexing::ExportItemId,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let (parsed, _) = self.context.engine.parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.engine.stabilized(file_id)?;

        let ptr = stabilized.ast_ptr(export_item_id).ok_or(AnalyzerError::NonFatal)?;
        let item = ptr.try_to_node(&root).ok_or(AnalyzerError::NonFatal)?;
        let cst::ExportItem::ExportType(item) = item else {
            return Err(AnalyzerError::NonFatal);
        };

        let type_items = item.type_items().ok_or(AnalyzerError::NonFatal)?;
        self.push_constructor_token_edit(file_id, type_items, old_name, new_name)
    }

    fn push_constructor_token_edit(
        &mut self,
        file_id: FileId,
        type_items: cst::TypeItems,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.engine.content(file_id);
        let cst::TypeItems::TypeItemsList(items) = type_items else {
            return Ok(());
        };

        for token in items.name_tokens() {
            if token.text(&content) == old_name {
                let range = position::text_range_to_utf8_range(&content, token.text_range())
                    .ok_or(AnalyzerError::NonFatal)?;

                self.push_utf8_edit(file_id, range, new_name)?;
            }
        }

        Ok(())
    }
}

impl<'edits, 'language, Queries, Catalog> RenameEdits<'edits, 'language, Queries, Catalog>
where
    Queries: AnalyzerQueries,
    Catalog: FileCatalog,
{
    fn push_text_range_edit(
        &mut self,
        file_id: FileId,
        range: TextRange,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.engine.content(file_id);
        let range =
            position::text_range_to_utf8_range(&content, range).ok_or(AnalyzerError::NonFatal)?;

        self.push_utf8_edit(file_id, range, new_name)
    }

    fn push_utf8_edit(
        &mut self,
        file_id: FileId,
        range: Utf8Range,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.engine.content(file_id);
        let range = position::utf8_range_to_protocol(&content, range, self.context.encoding)
            .ok_or(AnalyzerError::NonFatal)?;
        self.push_protocol_edit(file_id, range, new_name)
    }

    fn push_name_edit<T>(
        &mut self,
        file_id: FileId,
        id: Option<AstId<T>>,
        range: fn(&str, &SyntaxNode, &SyntaxNodePtr) -> Option<Utf8Range>,
        new_name: &str,
    ) -> Result<(), AnalyzerError>
    where
        T: AstNode,
    {
        let Some(id) = id else {
            return Ok(());
        };

        let content = self.context.engine.content(file_id);
        let (parsed, _) = self.context.engine.parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.engine.stabilized(file_id)?;

        let ptr = stabilized.syntax_ptr(id).ok_or(AnalyzerError::NonFatal)?;
        let range = range(&content, &root, &ptr).ok_or(AnalyzerError::NonFatal)?;
        let range = position::utf8_range_to_protocol(&content, range, self.context.encoding)
            .ok_or(AnalyzerError::NonFatal)?;
        self.push_protocol_edit(file_id, range, new_name)
    }

    fn push_protocol_edit(
        &mut self,
        file_id: FileId,
        range: Range,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let new_name = self.new_name_text(file_id, range, new_name)?;
        self.edits.push((file_id, TextEdit { range, new_text: new_name }));
        Ok(())
    }

    fn new_name_text(
        &self,
        file_id: FileId,
        range: Range,
        new_name: &str,
    ) -> Result<String, AnalyzerError> {
        let content = self.context.engine.content(file_id);
        let start =
            position::protocol_position_to_utf8(&content, range.start, self.context.encoding)
                .and_then(|position| position::utf8_position_to_offset(&content, position))
                .ok_or(AnalyzerError::NonFatal)?;
        let end = position::protocol_position_to_utf8(&content, range.end, self.context.encoding)
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

    pub(super) fn finish(mut self) -> Result<Option<WorkspaceEdit>, AnalyzerError> {
        self.edits.sort_by_key(|(file_id, edit)| {
            (
                file_id.into_raw().into_u32(),
                edit.range.start.line,
                edit.range.start.character,
                edit.range.end.line,
                edit.range.end.character,
            )
        });
        self.edits.dedup_by(|left, right| left.0 == right.0 && left.1.range == right.1.range);

        if self.edits.is_empty() {
            return Ok(None);
        }

        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::default();
        for (file_id, edit) in self.edits {
            let uri = common::file_uri(self.context, file_id)?;
            changes.entry(uri).or_default().push(edit);
        }

        Ok(Some(WorkspaceEdit { changes: Some(changes), ..WorkspaceEdit::default() }))
    }
}
