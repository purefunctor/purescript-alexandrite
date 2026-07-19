use std::fmt::Write;

use analyzer::QueryEngine;
use checking::CheckedModule;
use checking::core::pretty;
use checking::evidence::{EvidenceBinderId, EvidenceVarId};
use checking::semantic::{
    CheckedBinderKind, CheckedExpressionId, CheckedExpressionKind, CheckedLiteral,
};
use files::FileId;
use indexing::{TermItem, TermItemId};
use lowering::{BinderKind, Equation, GuardedExpression, TermItemIr, TermVariableResolution};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Abstraction,
    Application,
    Atom,
}

struct SemanticPrinter<'a> {
    engine: &'a QueryEngine,
    checked: &'a CheckedModule,
    lowered: &'a lowering::LoweredModule,
    pretty: pretty::Pretty<'a, QueryEngine>,
}

impl<'a> SemanticPrinter<'a> {
    fn new(
        engine: &'a QueryEngine,
        checked: &'a CheckedModule,
        lowered: &'a lowering::LoweredModule,
    ) -> SemanticPrinter<'a> {
        let pretty = pretty::Pretty::new(engine, checked);
        SemanticPrinter { engine, checked, lowered, pretty }
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

        for equation in equations.iter() {
            self.write_equation(output, name, equation);
        }

        writeln!(output).unwrap();
    }

    fn write_equation(&mut self, output: &mut String, name: &str, equation: &Equation) {
        let binders = equation.binders.iter().map(|binder| self.binder(*binder));
        let binders = binders.collect::<Vec<_>>();
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
            .map(|expression| self.expression(expression, Precedence::Abstraction))
            .unwrap_or_else(|| "<unimplemented>".to_string());
        writeln!(output, "{head} = {body}").unwrap();
    }

    fn expression(&mut self, expression_id: CheckedExpressionId, outer: Precedence) -> String {
        let expression = self.checked.core.expressions[expression_id].clone();
        let (rendered, precedence) = match expression.kind {
            CheckedExpressionKind::Variable { resolution } => {
                (self.variable(resolution), Precedence::Atom)
            }
            CheckedExpressionKind::Literal { literal } => {
                (Self::literal(literal), Precedence::Atom)
            }
            CheckedExpressionKind::TermApplication { function, argument } => {
                let function = self.expression(function, Precedence::Application);
                let argument = self.expression(argument, Precedence::Atom);
                (format!("{function} {argument}"), Precedence::Application)
            }
            CheckedExpressionKind::TypeApplication { function, argument } => {
                let function = self.expression(function, Precedence::Application);
                let argument = self.pretty.render(argument);
                (format!("{function} @{argument}"), Precedence::Application)
            }
            CheckedExpressionKind::EvidenceApplication { expression, evidence } => {
                let expression = self.expression(expression, Precedence::Application);
                let evidence = evidence.iter().map(|evidence| Self::evidence_variable(*evidence));
                let evidence = evidence.collect::<Vec<_>>();
                (format!("{expression} @{{{}}}", evidence.join(", ")), Precedence::Application)
            }
            CheckedExpressionKind::EvidenceAbstraction { binders, expression } => {
                let binders = binders.iter().map(|binder| Self::evidence_binder(*binder));
                let binders = binders.collect::<Vec<_>>();
                let expression = self.expression(expression, Precedence::Abstraction);
                (format!("\\@{{{}}} -> {expression}", binders.join(", ")), Precedence::Abstraction)
            }
        };

        if precedence < outer { format!("({rendered})") } else { rendered }
    }

    fn variable(&self, variable: TermVariableResolution) -> String {
        match variable {
            TermVariableResolution::Binder(binder) => self.binder_name(binder),
            TermVariableResolution::Let(binding) => format!("let{}", binding.into_raw().into_u32()),
            TermVariableResolution::RecordPun(pun) => {
                format!("pun{}", pun.into_raw().get())
            }
            TermVariableResolution::Reference(file_id, item_id) => self.item_name(file_id, item_id),
        }
    }

    fn item_name(&self, file_id: FileId, item_id: TermItemId) -> String {
        let indexed = self.engine.indexed(file_id).ok();
        indexed
            .and_then(|indexed| indexed.items[item_id].name.clone())
            .map(String::from)
            .unwrap_or_else(|| format!("item{}", item_id.into_raw().into_u32()))
    }

    fn binder(&self, source: lowering::BinderId) -> String {
        let Some(checked) = self.checked.core.lookup_binder(source) else {
            return "<unimplemented>".to_string();
        };

        match self.checked.core.binders[checked].kind {
            CheckedBinderKind::Variable => self.binder_name(source),
            CheckedBinderKind::Named { .. } => match self.lowered.info.get_binder_kind(source) {
                Some(BinderKind::Named { named, binder }) => {
                    let named = named.as_deref().unwrap_or("<missing>");
                    let binder = binder.map(|binder| self.binder(binder));
                    let binder = binder.unwrap_or_else(|| "<unimplemented>".to_string());
                    format!("{named}@{binder}")
                }
                _ => "<invalid named binder>".to_string(),
            },
        }
    }

    fn binder_name(&self, source: lowering::BinderId) -> String {
        match self.lowered.info.get_binder_kind(source) {
            Some(BinderKind::Variable { variable }) => {
                variable.clone().map(String::from).unwrap_or_else(|| "_".to_string())
            }
            Some(BinderKind::Named { named, .. }) => {
                named.clone().map(String::from).unwrap_or_else(|| "_".to_string())
            }
            Some(BinderKind::Parenthesized { parenthesized: Some(binder) }) => {
                self.binder_name(*binder)
            }
            _ => format!("binder{}", source.into_raw().get()),
        }
    }

    fn literal(literal: CheckedLiteral) -> String {
        match literal {
            CheckedLiteral::String { kind: lowering::StringKind::String, value } => {
                value.map(|value| format!("{value:?}")).unwrap_or_else(|| "?string".to_string())
            }
            CheckedLiteral::String { kind: lowering::StringKind::RawString, value } => value
                .map(|value| format!("raw{value:?}"))
                .unwrap_or_else(|| "?raw-string".to_string()),
            CheckedLiteral::Char(value) => {
                value.map(|value| format!("{value:?}")).unwrap_or_else(|| "?char".to_string())
            }
            CheckedLiteral::Boolean(value) => value.to_string(),
            CheckedLiteral::Integer(value) => {
                value.map(|value| value.to_string()).unwrap_or_else(|| "?int".to_string())
            }
            CheckedLiteral::Number(value) => {
                value.map(String::from).unwrap_or_else(|| "?number".to_string())
            }
        }
    }

    fn evidence_variable(EvidenceVarId(id): EvidenceVarId) -> String {
        format!("ev{id}")
    }

    fn evidence_binder(EvidenceBinderId(id): EvidenceBinderId) -> String {
        format!("dict{id}")
    }
}

pub fn report(engine: &QueryEngine, id: FileId, name: &str) -> String {
    let checked = engine.checked(id).unwrap();
    let lowered = engine.lowered(id).unwrap();
    let indexed = engine.indexed(id).unwrap();
    let mut printer = SemanticPrinter::new(engine, &checked, &lowered);

    let mut output = String::new();
    writeln!(output, "module {name} where").unwrap();
    writeln!(output).unwrap();

    for (item_id, item) in indexed.items.iter_terms() {
        printer.write_item(&mut output, item_id, item);
    }

    output
}
