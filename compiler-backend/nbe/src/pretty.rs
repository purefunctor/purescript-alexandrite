//! Stable text rendering for functional-tree snapshots.

use pretty::{Arena, DocAllocator, DocBuilder};

use crate::tree::{
    Declaration, DeclarationKind, EffectExpression, ExpressionId, ExpressionKind, Field, GlobalId,
    Guard, Literal, Module, Parameter, PatternId, PatternKind, RecordUpdate, ReflectableEvidence,
    ReflectableOrdering, SynthesizedEvidence,
};

type Doc<'a> = DocBuilder<'a, Arena<'a>, ()>;

const DEFAULT_WIDTH: usize = 100;

pub fn render(module: &Module) -> String {
    let arena = Arena::new();
    let printer = Printer { arena: &arena, module };
    let declarations =
        module.declarations.iter().map(|declaration| printer.declaration(declaration));
    let declaration_separator = arena.hardline().append(arena.hardline());
    let document = arena.intersperse(declarations, declaration_separator);
    let mut output = String::new();
    document
        .render_fmt(DEFAULT_WIDTH, &mut output)
        .expect("critical failure: failed to render normalized functional tree");
    output
}

struct Printer<'a, 'm> {
    arena: &'a Arena<'a>,
    module: &'m Module,
}

impl<'a> Printer<'a, '_> {
    fn declaration(&self, declaration: &Declaration) -> Doc<'a> {
        let name = &declaration.global.name;
        match declaration.kind {
            DeclarationKind::Foreign => self.arena.text(format!("foreign {name}")),
            DeclarationKind::Value(expression) => {
                let prefix = match declaration.global.id {
                    GlobalId::Term(..) => "",
                    GlobalId::Instance(..) => "instance ",
                };
                let recursion = declaration
                    .recursive_group
                    .map(|group| format!("recursive[{}] ", group.0))
                    .unwrap_or_default();
                let declaration = self.arena.text(format!("{prefix}{recursion}{name} ="));
                let expression = self.expression(expression);
                declaration.append(self.arena.space()).append(expression)
            }
        }
    }

    fn expression(&self, expression_id: ExpressionId) -> Doc<'a> {
        self.expression_at(expression_id, ExpressionPrecedence::Abstraction)
    }

    fn expression_at(
        &self,
        expression_id: ExpressionId,
        required_precedence: ExpressionPrecedence,
    ) -> Doc<'a> {
        let expression = &self.module.storage[expression_id];
        let precedence = match expression.kind {
            ExpressionKind::Abstraction { .. }
            | ExpressionKind::IfThenElse { .. }
            | ExpressionKind::Case { .. }
            | ExpressionKind::Guarded { .. }
            | ExpressionKind::Let { .. }
            | ExpressionKind::LetPattern { .. } => ExpressionPrecedence::Abstraction,
            ExpressionKind::Application { .. } | ExpressionKind::Effect { .. } => {
                ExpressionPrecedence::Application
            }
            ExpressionKind::RecordUpdate { .. } => ExpressionPrecedence::RecordUpdate,
            _ => ExpressionPrecedence::Atom,
        };
        let document = self.expression_unparenthesized(expression_id);
        if precedence < required_precedence {
            self.arena.text("(").append(document).append(")")
        } else {
            document
        }
    }

    fn expression_unparenthesized(&self, expression_id: ExpressionId) -> Doc<'a> {
        let expression = &self.module.storage[expression_id];
        match &expression.kind {
            ExpressionKind::Literal { literal } => self.literal(literal),
            ExpressionKind::Array { elements } => {
                let elements = elements.iter().map(|element| self.expression(*element));
                self.delimited("[", elements, "]")
            }
            ExpressionKind::Record { fields } => {
                let fields = fields.iter().map(|field| {
                    let label = self.field(&field.field);
                    let expression = self.expression(field.expression);
                    let expression = self.arena.line().append(expression).nest(2);
                    self.arena.text(format!("{label}:")).append(expression).group()
                });
                self.braced(fields)
            }
            ExpressionKind::RecordUpdate { record, updates } => {
                let record = self.expression_at(*record, ExpressionPrecedence::Atom);
                let updates = updates.iter().map(|update| self.record_update(update));
                let updates = self.braced(updates);
                record.append(self.arena.space()).append(updates)
            }
            ExpressionKind::Project { record, field } => {
                let record = self.expression_at(*record, ExpressionPrecedence::Atom);
                let field = self.field(field);
                record.append(self.arena.text(format!(".{field}")))
            }
            ExpressionKind::Constructor { global } | ExpressionKind::Global { global } => {
                self.arena.text(global.name.to_string())
            }
            ExpressionKind::Local { parameter } => self.parameter(parameter),
            ExpressionKind::Abstraction { parameters, body } => {
                let parameters = parameters.iter().map(|pattern| self.pattern(*pattern));
                let parameters = self.arena.intersperse(parameters, self.arena.space());
                let abstraction = self.arena.text("\\").append(parameters).append(" ->");
                let body_document = self.expression(*body);
                if self.expression_requires_body_break(*body) {
                    let body = self.arena.hardline().append(body_document).nest(2);
                    abstraction.append(body)
                } else {
                    let body = self.arena.line().append(body_document).nest(2);
                    abstraction.append(body).group()
                }
            }
            ExpressionKind::Application { function, arguments } => {
                if let Some(application) = self.try_render_bind_application(*function, arguments) {
                    application
                } else {
                    let function = self.expression_at(*function, ExpressionPrecedence::Application);
                    let arguments = arguments
                        .iter()
                        .map(|argument| self.expression_at(*argument, ExpressionPrecedence::Atom));
                    let arguments = self.arena.intersperse(arguments, self.arena.line());
                    let arguments = self.arena.line().append(arguments).nest(2);
                    function.append(arguments).group()
                }
            }
            ExpressionKind::IfThenElse { condition, then, else_ } => {
                let condition = self.expression(*condition);
                let then = self.expression(*then);
                let then = self.arena.line().append("then ").append(then).nest(2);
                let else_ = self.expression(*else_);
                let else_ = self.arena.line().append("else ").append(else_).nest(2);
                self.arena.text("if ").append(condition).append(then).append(else_).group()
            }
            ExpressionKind::Case { scrutinees, alternatives } => {
                let scrutinees = scrutinees.iter().map(|scrutinee| self.expression(*scrutinee));
                let scrutinees = self.arena.intersperse(scrutinees, self.arena.text(", "));
                let alternatives = alternatives.iter().map(|alternative| {
                    let patterns =
                        alternative.patterns.iter().map(|pattern| self.pattern(*pattern));
                    let patterns = self.arena.intersperse(patterns, self.arena.text(", "));
                    let expression = self.expression(alternative.expression);
                    let expression = self.arena.hardline().append(expression).nest(2);
                    patterns.append(" ->").append(expression)
                });
                let alternatives = self.arena.intersperse(alternatives, self.arena.hardline());
                let alternatives = self.arena.hardline().append(alternatives).nest(2);
                self.arena.text("case ").append(scrutinees).append(" of").append(alternatives)
            }
            ExpressionKind::Guarded { alternatives } => {
                let alternatives = alternatives.iter().map(|alternative| {
                    let guards = alternative.guards.iter().map(|guard| self.guard(guard));
                    let guards = self.arena.intersperse(guards, self.arena.text(", "));
                    let expression = self.expression(alternative.expression);
                    let expression = self.arena.line().append(expression).nest(2);
                    self.arena.text("| ").append(guards).append(" ->").append(expression).group()
                });
                self.arena.intersperse(alternatives, self.arena.hardline())
            }
            ExpressionKind::Let { recursive, bindings, body } => {
                let bindings = bindings.iter().map(|binding| {
                    let recursive = if *recursive { "rec " } else { "" };
                    let parameter = self.parameter(&binding.parameter);
                    let expression = self.expression(binding.expression);
                    self.arena.text(recursive).append(parameter).append(" = ").append(expression)
                });
                let bindings = self.arena.intersperse(bindings, self.arena.hardline());
                let bindings = self.arena.hardline().append(bindings).nest(2);
                let body = self.expression(*body);
                self.arena
                    .text("let")
                    .append(bindings)
                    .append(self.arena.hardline())
                    .append("in ")
                    .append(body)
            }
            ExpressionKind::LetPattern { pattern, value, body } => {
                let pattern = self.pattern(*pattern);
                let value = self.expression(*value);
                let binding = pattern.append(" = ").append(value);
                let binding = self.arena.hardline().append(binding).nest(2);
                let body = self.expression(*body);
                self.arena
                    .text("let")
                    .append(binding)
                    .append(self.arena.hardline())
                    .append("in ")
                    .append(body)
            }
            ExpressionKind::Effect { effect } => self.effect(effect),
            ExpressionKind::SynthesizedEvidence { evidence } => self.synthesized_evidence(evidence),
            ExpressionKind::TrivialEvidence => self.arena.text("<trivial evidence>"),
        }
    }

    fn pattern(&self, pattern_id: PatternId) -> Doc<'a> {
        self.pattern_at(pattern_id, PatternPrecedence::Application)
    }

    fn pattern_at(&self, pattern_id: PatternId, required_precedence: PatternPrecedence) -> Doc<'a> {
        let pattern = &self.module.storage[pattern_id];
        let precedence = match &pattern.kind {
            PatternKind::Constructor { arguments, .. } if !arguments.is_empty() => {
                PatternPrecedence::Application
            }
            _ => PatternPrecedence::Atom,
        };
        let document = match &pattern.kind {
            PatternKind::Variable(parameter) => self.parameter(parameter),
            PatternKind::Named { parameter, pattern } => {
                let parameter = self.parameter(parameter);
                let pattern = self.pattern(*pattern);
                parameter.append("@(").append(pattern).append(")")
            }
            PatternKind::Wildcard => self.arena.text("_"),
            PatternKind::Literal(literal) => self.literal(literal),
            PatternKind::Array(elements) => {
                let elements = elements.iter().map(|element| self.pattern(*element));
                self.delimited("[", elements, "]")
            }
            PatternKind::Record(fields) => {
                let fields = fields.iter().map(|field| {
                    let label = self.field(&field.field);
                    let pattern = self.pattern(field.pattern);
                    self.arena.text(format!("{label}: ")).append(pattern)
                });
                self.braced(fields)
            }
            PatternKind::Constructor { global, arguments } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| self.pattern_at(*argument, PatternPrecedence::Atom));
                let arguments = arguments.map(|argument| self.arena.space().append(argument));
                let arguments = self.arena.concat(arguments);
                self.arena.text(global.name.to_string()).append(arguments)
            }
        };
        if precedence < required_precedence {
            self.arena.text("(").append(document).append(")")
        } else {
            document
        }
    }

    fn guard(&self, guard: &Guard) -> Doc<'a> {
        match guard {
            Guard::Boolean(expression) => self.expression(*expression),
            Guard::Pattern { expression, pattern } => {
                let pattern = self.pattern(*pattern);
                let expression = self.expression(*expression);
                pattern.append(" <- ").append(expression)
            }
        }
    }

    fn record_update(&self, update: &RecordUpdate) -> Doc<'a> {
        match update {
            RecordUpdate::Leaf { field, expression } => {
                let field = self.field(field);
                let expression = self.expression(*expression);
                let expression = self.arena.line().append(expression).nest(2);
                self.arena.text(format!("{field} =")).append(expression).group()
            }
            RecordUpdate::Branch { field, updates } => {
                let field = self.field(field);
                let updates = updates.iter().map(|update| self.record_update(update));
                let updates = self.braced(updates);
                self.arena.text(field).append(self.arena.space()).append(updates)
            }
        }
    }

    fn try_render_bind_application(
        &self,
        function: ExpressionId,
        arguments: &[ExpressionId],
    ) -> Option<Doc<'a>> {
        let (function, mut all_arguments) = self.application_spine(function);
        all_arguments.extend_from_slice(arguments);
        let ExpressionKind::Global { global } = &self.module.storage[function].kind else {
            return None;
        };
        if global.name != "bind" {
            return None;
        }
        let (&continuation, arguments) = all_arguments.split_last()?;
        let ExpressionKind::Abstraction { parameters, body } =
            &self.module.storage[continuation].kind
        else {
            return None;
        };
        let arguments = arguments
            .iter()
            .map(|argument| self.expression_at(*argument, ExpressionPrecedence::Atom));
        let arguments = arguments.map(|argument| self.arena.line().append(argument));
        let arguments = self.arena.concat(arguments);
        let function = self.expression_at(function, ExpressionPrecedence::Application);
        let application = function.append(arguments);
        let parameters = parameters.iter().map(|parameter| self.pattern(*parameter));
        let parameters = self.arena.intersperse(parameters, self.arena.space());
        let continuation = self.arena.text("\\").append(parameters).append(" ->");
        let continuation = self.arena.line().append(continuation).nest(2);
        let application = application.append(continuation).group();
        let body = self.expression(*body);
        let body = self.arena.hardline().append(body).nest(2);
        Some(application.append(body))
    }

    fn application_spine(&self, expression: ExpressionId) -> (ExpressionId, Vec<ExpressionId>) {
        let ExpressionKind::Application { function, arguments } =
            &self.module.storage[expression].kind
        else {
            return (expression, Vec::new());
        };
        let (function, mut spine) = self.application_spine(*function);
        spine.extend_from_slice(arguments);
        (function, spine)
    }

    fn effect(&self, effect: &EffectExpression) -> Doc<'a> {
        match effect {
            EffectExpression::Pure(expression) => {
                let expression = self.expression_at(*expression, ExpressionPrecedence::Atom);
                self.arena.text("effect.pure ").append(expression)
            }
            EffectExpression::Bind { action, parameter, body } => {
                let action = self.expression_at(*action, ExpressionPrecedence::Atom);
                let parameter = self.parameter(parameter);
                let body = self.expression(*body);
                let continuation = self
                    .arena
                    .line()
                    .append("\\")
                    .append(parameter)
                    .append(" -> ")
                    .append(body)
                    .nest(2);
                self.arena.text("effect.bind ").append(action).append(continuation).group()
            }
        }
    }

    fn synthesized_evidence(&self, evidence: &SynthesizedEvidence) -> Doc<'a> {
        let rendered = match evidence {
            SynthesizedEvidence::IsSymbol(symbol) => format!("symbol {symbol:?}"),
            SynthesizedEvidence::Reflectable(ReflectableEvidence::Integer(value)) => {
                format!("reflectable {value}")
            }
            SynthesizedEvidence::Reflectable(ReflectableEvidence::String(value)) => {
                format!("reflectable {value:?}")
            }
            SynthesizedEvidence::Reflectable(ReflectableEvidence::Boolean(value)) => {
                format!("reflectable {value}")
            }
            SynthesizedEvidence::Reflectable(ReflectableEvidence::Ordering(ordering)) => {
                match ordering {
                    ReflectableOrdering::Less => "reflectable LT".into(),
                    ReflectableOrdering::Equal => "reflectable EQ".into(),
                    ReflectableOrdering::Greater => "reflectable GT".into(),
                }
            }
        };
        self.arena.text(rendered)
    }

    fn literal(&self, literal: &Literal) -> Doc<'a> {
        let rendered = match literal {
            Literal::String(value) => format!("{value:?}"),
            Literal::Char(value) => format!("{value:?}"),
            Literal::Boolean(value) => value.to_string(),
            Literal::Integer(value) => value.to_string(),
            Literal::Number(value) => value.to_string(),
        };
        self.arena.text(rendered)
    }

    fn parameter(&self, parameter: &Parameter) -> Doc<'a> {
        self.arena.text(format!("{}%{}", parameter.name, parameter.id.0))
    }

    fn field(&self, field: &Field) -> String {
        let name = field.name.as_str();
        let is_valid_identifier = !name.is_empty()
            && name.chars().enumerate().all(|(position, character)| {
                if position == 0 {
                    character.is_ascii_lowercase() || character == '_'
                } else {
                    character.is_ascii_alphanumeric() || character == '_' || character == '\''
                }
            });
        if is_valid_identifier { name.to_string() } else { format!("{name:?}") }
    }

    fn expression_requires_body_break(&self, expression_id: ExpressionId) -> bool {
        matches!(
            self.module.storage[expression_id].kind,
            ExpressionKind::IfThenElse { .. }
                | ExpressionKind::Case { .. }
                | ExpressionKind::Guarded { .. }
                | ExpressionKind::Let { .. }
                | ExpressionKind::LetPattern { .. }
        )
    }

    fn delimited<I>(&self, open: &'static str, documents: I, close: &'static str) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        let separator = self.arena.text(",").append(self.arena.line());
        let documents = self.arena.intersperse(documents, separator);
        let documents = self.arena.softline_().append(documents).nest(2);
        let closing_line = self.arena.softline_();
        self.arena.text(open).append(documents).append(closing_line).append(close).group()
    }

    fn braced<I>(&self, documents: I) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        let separator = self.arena.text(",").append(self.arena.line());
        let documents = self.arena.intersperse(documents, separator);
        let documents = self.arena.line().append(documents).nest(2);
        let closing_line = self.arena.line();
        self.arena.text("{").append(documents).append(closing_line).append("}").group()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ExpressionPrecedence {
    Abstraction,
    RecordUpdate,
    Application,
    Atom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PatternPrecedence {
    Application,
    Atom,
}
