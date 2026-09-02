//! Recognition and lowering for the virtual StyleX module.

use building_types::QueryResult;
use files::FileId;
use indexing::TermItemId;
use itertools::Itertools;
use rustc_hash::FxHashSet;

use crate::error::UnsupportedState;
use crate::optimize::expression_children;
use crate::stylex::StyleXIntrinsic;
use crate::tree::{
    BinaryOperator, Binding, Declaration, DeclarationKind, ExpressionId, ExpressionKind, Field,
    GlobalId, RecordField,
};

use super::{Context, ConversionResult};

impl<'c, Q> Context<'c, Q>
where
    Q: checking::ExternalQueries,
{
    pub(super) fn stylex_intrinsic(
        &mut self,
        expression: ExpressionId,
        arguments: &[ExpressionId],
        result_type: Option<checking::TypeId>,
    ) -> ConversionResult<Option<ExpressionId>> {
        let ExpressionKind::Global { global } = &self.storage[expression].kind else {
            return Ok(None);
        };
        let GlobalId::Term(file_id, term_id) = global.id else { return Ok(None) };
        let Some(intrinsic) = self.stylex_intrinsic_identity(file_id, term_id)? else {
            return Ok(None);
        };
        let expression = match (intrinsic, arguments) {
            (StyleXIntrinsic::Create, [_, argument])
            | (StyleXIntrinsic::Props, [_, argument])
            | (StyleXIntrinsic::Keyframes, [argument]) => {
                Some(self.expression(ExpressionKind::StyleX { intrinsic, argument: *argument }))
            }
            (StyleXIntrinsic::RecordProps, [_, argument]) => {
                let Some(result_type) = result_type else { return Ok(None) };
                self.stylex_record_props(*argument, result_type)?
            }
            (StyleXIntrinsic::Conditional, [condition, style]) => {
                Some(self.expression(ExpressionKind::Binary {
                    operator: BinaryOperator::StyleXConditional,
                    left: *condition,
                    right: *style,
                }))
            }
            _ => None,
        };
        Ok(expression)
    }

    fn stylex_record_props(
        &mut self,
        argument: ExpressionId,
        result_type: checking::TypeId,
    ) -> ConversionResult<Option<ExpressionId>> {
        let checking::Type::Application(_, mut row_type) = self.queries.lookup_type(result_type)
        else {
            return Ok(None);
        };

        let mut labels = vec![];
        loop {
            let checking::Type::Row(row_id) = self.queries.lookup_type(row_type) else {
                return Ok(None);
            };
            let row = self.queries.lookup_row_type(row_id);
            labels.extend(row.fields.iter().map(|field| field.label.clone()));
            let Some(tail) = row.tail else { break };
            row_type = tail;
        }

        let stable = self.expression_is_stable(argument);
        let (record, parameter) = if stable {
            (argument, None)
        } else {
            let parameter = self.fresh_parameter("stylexStyles".into())?;
            let record = self.expression(ExpressionKind::Local { parameter: parameter.clone() });
            (record, Some(parameter))
        };

        let fields = labels.into_iter().map(|label| {
            let field = self.label_field(label);
            let style =
                self.expression(ExpressionKind::Project { record, field: Field::clone(&field) });
            let expression = self.expression(ExpressionKind::StyleX {
                intrinsic: StyleXIntrinsic::Props,
                argument: style,
            });
            RecordField { field, expression }
        });

        let fields = fields.collect();
        let body = self.expression(ExpressionKind::Record { fields });

        let Some(parameter) = parameter else { return Ok(Some(body)) };
        let binding = Binding { parameter, expression: argument, source_order: 0 };

        Ok(Some(self.expression(ExpressionKind::Let {
            recursive: false,
            bindings: [binding].into(),
            body,
        })))
    }

    fn stylex_intrinsic_identity(
        &self,
        file_id: FileId,
        term_id: TermItemId,
    ) -> QueryResult<Option<StyleXIntrinsic>> {
        if self.queries.module_file("Alexandrite.StyleX") != Some(file_id) {
            return Ok(None);
        }
        let indexed = self.indexed_module(file_id)?;
        let intrinsic = match indexed.items[term_id].name.as_deref() {
            Some("create") => Some(StyleXIntrinsic::Create),
            Some("props") => Some(StyleXIntrinsic::Props),
            Some("recordProps") => Some(StyleXIntrinsic::RecordProps),
            Some("conditional") => Some(StyleXIntrinsic::Conditional),
            Some("keyframes") => Some(StyleXIntrinsic::Keyframes),
            _ => None,
        };
        Ok(intrinsic)
    }

    pub(super) fn validate_stylex_uses(
        &self,
        declarations: &[Declaration],
    ) -> ConversionResult<()> {
        let pending = declarations.iter().filter_map(|declaration| match declaration.kind {
            DeclarationKind::Value(expression) => Some((expression, declaration.global.id)),
            DeclarationKind::Constructor { .. } | DeclarationKind::Foreign => None,
        });
        let mut pending = pending.collect_vec();
        let mut visited = FxHashSet::default();
        while let Some((expression, declaration)) = pending.pop() {
            if !visited.insert(expression) {
                continue;
            }
            if let ExpressionKind::Global { global } = &self.storage[expression].kind
                && let GlobalId::Term(file_id, term_id) = global.id
                && let Some(intrinsic) = self.stylex_intrinsic_identity(file_id, term_id)?
            {
                let state = UnsupportedState::InvalidStyleXUse {
                    function: intrinsic.name().to_owned(),
                    declaration,
                };
                return Err(self.unsupported(state));
            }
            let children = expression_children(&self.storage[expression].kind);
            pending.extend(children.into_iter().map(|expression| (expression, declaration)));
        }
        Ok(())
    }

    pub(super) fn module_is_virtual(&self, file_id: FileId) -> bool {
        self.queries.module_file("Alexandrite.StyleX") == Some(file_id)
    }

    pub(super) fn validate_runtime_reference(
        &self,
        file_id: FileId,
        term_id: TermItemId,
    ) -> ConversionResult<()> {
        if !self.module_is_virtual(file_id) {
            return Ok(());
        }
        let module_name = self.source_module_name(file_id)?.to_string();
        let indexed = self.indexed_module(file_id)?;
        let item_name = indexed.items[term_id]
            .name
            .clone()
            .unwrap_or_else(|| self.term_fallback(term_id))
            .to_string();
        let state = UnsupportedState::VirtualModuleRuntimeReference { module_name, item_name };
        Err(self.unsupported(state))
    }
}
