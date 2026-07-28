use std::collections::HashMap;

use building_types::QueryProxy;
use files::FileId;
use indexing::{
    ImplicitItems, ImportId, ImportItemId, TermItemId, TermItemKind, TypeItemId, TypeItemKind,
    TypeSelection,
};
use itertools::Itertools;
use lsp_types::*;
use stabilizing::AstId;
use syntax::ast::{AstNode, support};
use syntax::{SyntaxNode, SyntaxNodePtr, SyntaxToken, TextRange, TokenAtOffset, cst};

use super::{NameKind, RenameTarget, qualifier_name};
use crate::position::Utf8Range;
use crate::{AnalyzerContext, AnalyzerError, AnalyzerHost, common, position};

// A macro permits one invocation to contain IDs with different AstId<T> types.
macro_rules! push_name_edits {
    ($self:expr, $file_id:expr, $new_name:expr, $range:expr; $($id:expr),+ $(,)?) => {
        $($self.push_name_edit($file_id, $id, $range, $new_name)?;)+
    };
}

pub(super) struct RenameEdits<'edits, 'language, Host>
where
    Host: AnalyzerHost,
{
    context: &'edits AnalyzerContext<'language, Host>,
    edits: Vec<(FileId, TextEdit)>,
}

impl<'edits, 'language, Host> RenameEdits<'edits, 'language, Host>
where
    Host: AnalyzerHost,
{
    pub(super) fn new(
        context: &'edits AnalyzerContext<'language, Host>,
    ) -> RenameEdits<'edits, 'language, Host> {
        RenameEdits { context, edits: vec![] }
    }
}

impl<'edits, 'language, Host> RenameEdits<'edits, 'language, Host>
where
    Host: AnalyzerHost,
{
    pub(super) fn collect_qualifier(
        &mut self,
        file_id: FileId,
        import_id: ImportId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let indexed = self.context.queries().indexed(file_id)?;
        let old_name =
            indexed.imports.get(&import_id).and_then(|import| import.alias.as_deref()).ok_or_else(
                || {
                    AnalyzerError::RenameRejected(
                        "Rename could not resolve the target qualifier".to_string(),
                    )
                },
            )?;

        let content = self.context.queries().content(file_id);
        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();

        let statements = support::descendants::<cst::ImportStatement>(&root);

        for statement in statements {
            let Some(module_name) = statement.import_alias().and_then(|alias| alias.module_name())
            else {
                continue;
            };
            if module_name.syntax().text(&content) != old_name {
                continue;
            }

            let _ = self.push_text_range_edit(file_id, module_name.syntax().text_range(), new_name);
        }

        let qualified_names = support::descendants::<cst::QualifiedName>(&root);
        let qualified_new_name = format!("{new_name}.");

        for qualified in qualified_names {
            let Some(qualifier) = qualified.qualifier() else {
                continue;
            };
            let Some(token) = qualifier.text() else {
                continue;
            };
            if qualifier_name(token.text(&content)) != Some(old_name) {
                continue;
            }

            let _ = self.push_text_range_edit(file_id, token.text_range(), &qualified_new_name);
        }

        let exports = support::descendants::<cst::ExportModule>(&root);

        for export in exports {
            let Some(module_name) = export.module_name() else {
                continue;
            };
            if module_name.syntax().text(&content) != old_name {
                continue;
            }

            let _ = self.push_text_range_edit(file_id, module_name.syntax().text_range(), new_name);
        }

        Ok(())
    }

    pub(super) fn collect_module(
        &mut self,
        target_file: FileId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let (parsed, _) = self.context.queries().parsed(target_file)?;
        let module_name =
            parsed.cst().header().and_then(|header| header.name()).ok_or_else(|| {
                AnalyzerError::RenameRejected(
                    "Rename could not resolve the target module name".to_string(),
                )
            })?;

        self.push_text_range_edit(target_file, module_name.syntax().text_range(), new_name)
            .ok_or_else(|| {
                AnalyzerError::RenameRejected(
                    "Rename could not edit the target module name".to_string(),
                )
            })?;

        for file_id in self.context.active_files() {
            if !self.context.is_editable(file_id) {
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
        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();
        let indexed = self.context.queries().indexed(file_id)?;
        let stabilized = self.context.queries().stabilized(file_id)?;

        for (import_id, import) in &indexed.imports {
            let Some(name) = import.name.as_deref() else {
                continue;
            };
            if self.context.queries().module_file(name) != Some(target_file) {
                continue;
            }

            let Some(ptr) = stabilized.ast_ptr(*import_id) else {
                continue;
            };
            let Some(statement) = ptr.try_to_node(&root) else {
                continue;
            };
            let Some(module_name) = statement.module_name() else {
                continue;
            };

            let _ = self.push_text_range_edit(file_id, module_name.syntax().text_range(), new_name);
        }

        Ok(())
    }

    fn module_export_edits(
        &mut self,
        file_id: FileId,
        target_file: FileId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.queries().content(file_id);
        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();
        let indexed = self.context.queries().indexed(file_id)?;
        let stabilized = self.context.queries().stabilized(file_id)?;
        let current_module = parsed.module_name(&content);

        for export in &indexed.exports.modules {
            let exports_self =
                file_id == target_file && current_module.as_ref() == Some(&export.name);
            let exports_target_module =
                self.context.queries().module_file(&export.name) == Some(target_file);
            let exports_import = exports_target_module
                && indexed.imports.values().any(|import| {
                    import.alias.is_none() && import.name.as_ref() == Some(&export.name)
                });

            if !exports_self && !exports_import {
                continue;
            }

            let Some(ptr) = stabilized.ast_ptr(export.id) else {
                continue;
            };
            let Some(item) = ptr.try_to_node(&root) else {
                continue;
            };
            let cst::ExportItem::ExportModule(export) = item else {
                continue;
            };
            let Some(module_name) = export.module_name() else {
                continue;
            };

            let _ = self.push_text_range_edit(file_id, module_name.syntax().text_range(), new_name);
        }

        Ok(())
    }
}

impl<'edits, 'language, Host> RenameEdits<'edits, 'language, Host>
where
    Host: AnalyzerHost,
{
    pub(super) fn collect_references(
        &mut self,
        locations: Vec<Location>,
        name_kind: NameKind,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        for location in locations {
            let Some(file_id) = self.context.file_id(location.uri.as_str()) else {
                continue;
            };
            if !self.context.is_editable(file_id) {
                continue;
            }

            let Some(range) = self.reference_name_range(file_id, location.range, name_kind)? else {
                continue;
            };
            if self.push_protocol_edit(file_id, range, new_name).is_none() {
                continue;
            }
        }

        Ok(())
    }

    fn reference_name_range(
        &self,
        file_id: FileId,
        range: Range,
        name_kind: NameKind,
    ) -> Result<Option<Range>, AnalyzerError> {
        let content = self.context.queries().content(file_id);
        let Some(position) = position::protocol_position_to_utf8(
            &content,
            range.start,
            self.context.position_encoding(),
        ) else {
            return Ok(None);
        };
        let Some(offset) = position::utf8_position_to_offset(&content, position) else {
            return Ok(None);
        };

        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();

        let token = match root.token_at_offset(offset) {
            TokenAtOffset::None => return Ok(None),
            TokenAtOffset::Single(token) => token,
            TokenAtOffset::Between(left, right) => {
                if RenameEdits::<Host>::qualified_name_token(&right, name_kind).is_some() {
                    right
                } else {
                    left
                }
            }
        };

        let Some(token) = RenameEdits::<Host>::qualified_name_token(&token, name_kind) else {
            return Ok(None);
        };

        let range = token.text_range();
        let range =
            position::text_range_to_protocol(&content, range, self.context.position_encoding());
        Ok(range)
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

impl<'edits, 'language, Host> RenameEdits<'edits, 'language, Host>
where
    Host: AnalyzerHost,
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
        let indexed = self.context.queries().indexed(file_id)?;

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
        let indexed = self.context.queries().indexed(file_id)?;

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

impl<'edits, 'language, Host> RenameEdits<'edits, 'language, Host>
where
    Host: AnalyzerHost,
{
    pub(super) fn collect_item_surfaces(
        &mut self,
        target: RenameTarget,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        for file_id in self.context.active_files() {
            if !self.context.is_editable(file_id) {
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
        let indexed = self.context.queries().indexed(file_id)?;
        let resolved = self.context.queries().resolved(file_id)?;

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
        let target_indexed = self.context.queries().indexed(target_file)?;
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
        let indexed = self.context.queries().indexed(file_id)?;
        let resolved = self.context.queries().resolved(file_id)?;

        match target {
            RenameTarget::Term(target_file, target_term) => {
                for export in &indexed.exports.terms {
                    if resolved.exports.lookup_term(&export.name)
                        == Some((target_file, target_term))
                    {
                        self.push_export_item_edit(file_id, export.id, new_name)?;
                    }
                }

                let target_indexed = self.context.queries().indexed(target_file)?;
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
        let content = self.context.queries().content(file_id);
        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.queries().stabilized(file_id)?;

        let Some(ptr) = stabilized.ast_ptr(import_item_id) else {
            return Ok(());
        };
        let Some(item) = ptr.try_to_node(&root) else {
            return Ok(());
        };
        let Some(range) = position::import_item_name_range(&content, item) else {
            return Ok(());
        };

        let _ = self.push_utf8_edit(file_id, range, new_name);
        Ok(())
    }

    fn push_export_item_edit(
        &mut self,
        file_id: FileId,
        export_item_id: indexing::ExportItemId,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.queries().content(file_id);
        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.queries().stabilized(file_id)?;

        let Some(ptr) = stabilized.ast_ptr(export_item_id) else {
            return Ok(());
        };
        let Some(item) = ptr.try_to_node(&root) else {
            return Ok(());
        };
        let Some(range) = position::export_item_name_range(&content, item) else {
            return Ok(());
        };

        let _ = self.push_utf8_edit(file_id, range, new_name);
        Ok(())
    }

    fn push_import_constructor_edit(
        &mut self,
        file_id: FileId,
        import_item_id: ImportItemId,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.queries().stabilized(file_id)?;

        let Some(ptr) = stabilized.ast_ptr(import_item_id) else {
            return Ok(());
        };
        let Some(item) = ptr.try_to_node(&root) else {
            return Ok(());
        };
        let cst::ImportItem::ImportType(item) = item else {
            return Ok(());
        };

        let Some(type_items) = item.type_items() else {
            return Ok(());
        };
        self.push_constructor_token_edit(file_id, type_items, old_name, new_name)
    }

    fn push_export_constructor_edit(
        &mut self,
        file_id: FileId,
        export_item_id: indexing::ExportItemId,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.queries().stabilized(file_id)?;

        let Some(ptr) = stabilized.ast_ptr(export_item_id) else {
            return Ok(());
        };
        let Some(item) = ptr.try_to_node(&root) else {
            return Ok(());
        };
        let cst::ExportItem::ExportType(item) = item else {
            return Ok(());
        };

        let Some(type_items) = item.type_items() else {
            return Ok(());
        };
        self.push_constructor_token_edit(file_id, type_items, old_name, new_name)
    }

    fn push_constructor_token_edit(
        &mut self,
        file_id: FileId,
        type_items: cst::TypeItems,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), AnalyzerError> {
        let content = self.context.queries().content(file_id);
        let cst::TypeItems::TypeItemsList(items) = type_items else {
            return Ok(());
        };

        for token in items.name_tokens() {
            if token.text(&content) == old_name {
                let Some(range) = position::text_range_to_utf8_range(&content, token.text_range())
                else {
                    continue;
                };

                let _ = self.push_utf8_edit(file_id, range, new_name);
            }
        }

        Ok(())
    }
}

impl<'edits, 'language, Host> RenameEdits<'edits, 'language, Host>
where
    Host: AnalyzerHost,
{
    fn push_text_range_edit(
        &mut self,
        file_id: FileId,
        range: TextRange,
        new_name: &str,
    ) -> Option<()> {
        let content = self.context.queries().content(file_id);
        let range = position::text_range_to_utf8_range(&content, range)?;

        self.push_utf8_edit(file_id, range, new_name)
    }

    fn push_utf8_edit(&mut self, file_id: FileId, range: Utf8Range, new_name: &str) -> Option<()> {
        let content = self.context.queries().content(file_id);
        let range =
            position::utf8_range_to_protocol(&content, range, self.context.position_encoding())?;
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

        let content = self.context.queries().content(file_id);
        let (parsed, _) = self.context.queries().parsed(file_id)?;
        let root = parsed.syntax_node();
        let stabilized = self.context.queries().stabilized(file_id)?;

        let rejected = || {
            AnalyzerError::RenameRejected(
                "Rename could not edit the target declaration".to_string(),
            )
        };
        let ptr = stabilized.syntax_ptr(id).ok_or_else(rejected)?;
        let range = range(&content, &root, &ptr).ok_or_else(rejected)?;
        let range =
            position::utf8_range_to_protocol(&content, range, self.context.position_encoding())
                .ok_or_else(rejected)?;
        self.push_protocol_edit(file_id, range, new_name).ok_or_else(rejected)
    }

    fn push_protocol_edit(&mut self, file_id: FileId, range: Range, new_name: &str) -> Option<()> {
        let new_name = self.new_name_text(file_id, range, new_name)?;
        self.edits.push((file_id, TextEdit { range, new_text: new_name }));
        Some(())
    }

    fn new_name_text(&self, file_id: FileId, range: Range, new_name: &str) -> Option<String> {
        let content = self.context.queries().content(file_id);
        let start = position::protocol_position_to_utf8(
            &content,
            range.start,
            self.context.position_encoding(),
        )
        .and_then(|position| position::utf8_position_to_offset(&content, position))?;
        let end = position::protocol_position_to_utf8(
            &content,
            range.end,
            self.context.position_encoding(),
        )
        .and_then(|position| position::utf8_position_to_offset(&content, position))?;

        let range = TextRange::new(start, end);
        let text = &content[range];

        if text.starts_with('(') && text.ends_with(')') {
            Some(format!("({new_name})"))
        } else {
            Some(new_name.to_string())
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
        self.edits.dedup_by(|(left_file, left_edit), (right_file, right_edit)| {
            left_file == right_file
                && left_edit.range == right_edit.range
                && left_edit.new_text == right_edit.new_text
        });

        let mut edit_windows = self.edits.iter().tuple_windows();
        let conflicting_edits = edit_windows.any(|(left, right)| {
            let (left_file, left_edit) = left;
            let (right_file, right_edit) = right;

            left_file == right_file
                && (right_edit.range == left_edit.range
                    || right_edit.range.start < left_edit.range.end)
        });
        if conflicting_edits {
            return Err(AnalyzerError::RenameRejected(
                "Rename produced conflicting text edits".to_string(),
            ));
        }

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
