use std::fmt::Write;

use analyzer::QueryEngine;
use checking::core::CheckedDataDeclarationKind;
use checking::semantic::pretty;
use checking::{CheckedModule, PrettyQueries};
use files::FileId;
use indexing::{TermItem, TermItemId, TypeItem, TypeItemId};
use lowering::{Equation, GuardedExpression, TermItemIr};

struct SemanticPrinter<'a> {
    engine: &'a QueryEngine,
    file_id: FileId,
    checked: &'a CheckedModule,
    lowered: &'a lowering::LoweredModule,
    pretty: pretty::Pretty<'a, QueryEngine>,
}

impl<'a> SemanticPrinter<'a> {
    fn new(
        engine: &'a QueryEngine,
        file_id: FileId,
        checked: &'a CheckedModule,
        lowered: &'a lowering::LoweredModule,
    ) -> SemanticPrinter<'a> {
        let pretty = pretty::Pretty::new(engine, checked, lowered);
        SemanticPrinter { engine, file_id, checked, lowered, pretty }
    }

    fn write_item(&mut self, output: &mut String, item_id: TermItemId, item: &TermItem) {
        let Some(name) = item.name.as_deref() else { return };
        let Some(type_id) = self.checked.lookup_term(item_id) else { return };
        let Some(TermItemIr::ValueGroup { equations, .. }) =
            self.lowered.info.get_term_item(item_id)
        else {
            return;
        };

        self.pretty.reset();
        let signature = self.pretty.render_signature(name, type_id);
        writeln!(output, "{signature}").unwrap();

        if let Some(expression) = self.checked.core.lookup_term_root(item_id) {
            let expression = self.pretty.render_expression(expression);
            writeln!(output, "{name} = {expression}").unwrap();
        } else {
            for equation in equations.iter() {
                self.write_equation(output, name, equation);
            }
        }

        writeln!(output).unwrap();
    }

    fn write_data_declaration(
        &mut self,
        output: &mut String,
        item_id: TypeItemId,
        item: &TypeItem,
    ) {
        let Some(name) = item.name.as_deref() else { return };
        let Some(declaration) = self.checked.lookup_data_declaration(item_id) else { return };

        self.pretty.reset();
        let type_parameters = declaration.type_parameters.iter().map(|&binder_id| {
            let binder = self.engine.lookup_forall_binder(binder_id);
            let name = self.pretty.display_name(binder.name);
            let kind = self.pretty.render_type(binder.kind);
            format!("({name} :: {kind})")
        });
        let type_parameters = type_parameters.collect::<Vec<_>>();
        let head = if type_parameters.is_empty() {
            name.to_string()
        } else {
            format!("{name} {}", type_parameters.join(" "))
        };

        let constructors = declaration.constructors.iter().map(|constructor| {
            let name = self.item_name(self.file_id, constructor.item_id);
            let arguments = constructor
                .arguments
                .iter()
                .map(|&argument| self.pretty.render_type(argument))
                .collect::<Vec<_>>();
            if arguments.is_empty() { name } else { format!("{name} ({})", arguments.join(") (")) }
        });
        let constructors = constructors.collect::<Vec<_>>().join(" | ");
        let keyword = match declaration.kind {
            CheckedDataDeclarationKind::Data => "data",
            CheckedDataDeclarationKind::Newtype => "newtype",
        };
        writeln!(output, "{keyword} {head} = {constructors}").unwrap();
        writeln!(output).unwrap();
    }

    fn write_equation(&mut self, output: &mut String, name: &str, equation: &Equation) {
        let mut binders = Vec::with_capacity(equation.binders.len());
        for binder in equation.binders.iter() {
            let Some(checked) = self.checked.core.lookup_binder(*binder) else {
                binders.push("<unimplemented>".to_string());
                continue;
            };
            binders.push(self.pretty.render_binder(checked).to_string());
        }
        let head = if binders.is_empty() {
            name.to_string()
        } else {
            format!("{name} {}", binders.join(" "))
        };

        match &equation.guarded {
            Some(GuardedExpression::Unconditional { where_expression }) => {
                let where_expression = where_expression
                    .as_ref()
                    .filter(|where_expression| where_expression.bindings.is_empty());
                let expression =
                    where_expression.and_then(|where_expression| where_expression.expression);
                self.write_equation_body(output, &head, expression);
            }
            Some(GuardedExpression::Conditionals { .. }) => {
                self.write_equation_body(output, &head, None)
            }
            None => self.write_equation_body(output, &head, None),
        }
    }

    fn write_equation_body(
        &mut self,
        output: &mut String,
        head: &str,
        source: Option<lowering::ExpressionId>,
    ) {
        let expression = source.and_then(|source| self.checked.core.lookup_expression(source));
        let body = expression
            .map(|expression| self.pretty.render_expression(expression).to_string())
            .unwrap_or_else(|| "<unimplemented>".to_string());
        writeln!(output, "{head} = {body}").unwrap();
    }

    fn item_name(&self, file_id: FileId, item_id: TermItemId) -> String {
        let indexed = self.engine.indexed(file_id).ok();
        indexed
            .and_then(|indexed| indexed.items[item_id].name.clone())
            .map(String::from)
            .unwrap_or_else(|| format!("item{}", item_id.into_raw().into_u32()))
    }
}

pub fn report(engine: &QueryEngine, id: FileId, name: &str) -> String {
    let checked = engine.checked(id).unwrap();
    let lowered = engine.lowered(id).unwrap();
    let indexed = engine.indexed(id).unwrap();
    let mut printer = SemanticPrinter::new(engine, id, &checked, &lowered);

    let mut output = String::new();
    writeln!(output, "module {name} where").unwrap();
    writeln!(output).unwrap();

    for (item_id, item) in indexed.items.iter_types() {
        printer.write_data_declaration(&mut output, item_id, item);
    }

    for (item_id, item) in indexed.items.iter_terms() {
        printer.write_item(&mut output, item_id, item);
    }

    output
}
