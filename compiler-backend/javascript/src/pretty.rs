use pretty::{Arena, DocAllocator, DocBuilder};

use crate::tree::{BinaryOperator, Expression, ExpressionId, ObjectProperty, Tree, UnaryOperator};

type Doc<'a> = DocBuilder<'a, Arena<'a>, ()>;

const DEFAULT_WIDTH: usize = 100;

struct Printer<'a, 't> {
    arena: &'a Arena<'a>,
    tree: &'t Tree,
}

impl<'a> Printer<'a, '_> {
    fn expression(&self, expression: ExpressionId) -> Doc<'a> {
        self.expression_at(expression, Precedence::Lowest)
    }

    fn expression_at(&self, expression: ExpressionId, parent: Precedence) -> Doc<'a> {
        let precedence = self.precedence(expression);
        let document = self.expression_unparenthesized(expression);
        if precedence < parent {
            self.arena.text("(").append(document).append(")")
        } else {
            document
        }
    }

    fn expression_unparenthesized(&self, expression: ExpressionId) -> Doc<'a> {
        match &self.tree[expression] {
            Expression::Identifier(name) | Expression::Number(name) => {
                self.arena.text(name.to_owned())
            }
            Expression::String(value) => self.arena.text(render_string(value)),
            Expression::Boolean(value) => self.arena.text(if *value { "true" } else { "false" }),
            Expression::Array(elements) => {
                let elements = elements
                    .iter()
                    .map(|element| self.expression_at(*element, Precedence::Assignment));
                self.delimited("[", elements, "]")
            }
            Expression::Object(properties) => {
                if properties.is_empty() {
                    return self.arena.text("{}");
                }
                let properties = properties.iter().map(|property| match property {
                    ObjectProperty::Field { name, value } => self
                        .arena
                        .text(render_property_name(name))
                        .append(": ")
                        .append(self.expression(*value)),
                    ObjectProperty::Spread(value) => self
                        .arena
                        .text("...")
                        .append(self.expression_at(*value, Precedence::Assignment)),
                });
                self.spaced_delimited("{", properties, "}")
            }
            Expression::Call { callee, arguments } => {
                let callee = self.expression_at(*callee, Precedence::Call);
                let arguments = arguments
                    .iter()
                    .map(|argument| self.expression_at(*argument, Precedence::Assignment));
                callee.append(self.delimited("(", arguments, ")"))
            }
            Expression::Member { object, property } => {
                let object = self.expression_at(*object, Precedence::Member);
                if property_is_identifier(property) {
                    object.append(".").append(self.arena.text(property.to_owned()))
                } else {
                    object.append("[").append(self.arena.text(render_string(property))).append("]")
                }
            }
            Expression::Index { object, index } => self
                .expression_at(*object, Precedence::Member)
                .append("[")
                .append(self.expression(*index))
                .append("]"),
            Expression::Unary { operator, value } => self
                .arena
                .text(operator.source())
                .append(self.expression_at(*value, Precedence::Unary)),
            Expression::Binary { operator, left, right } => {
                let precedence = operator.precedence();
                self.expression_at(*left, precedence)
                    .append(" ")
                    .append(operator.source())
                    .append(" ")
                    .append(self.expression_at(*right, precedence.next()))
            }
            Expression::Arrow { parameters, body } => {
                let body_is_object = matches!(&self.tree[*body], Expression::Object(_));
                let parameters = match parameters.as_slice() {
                    [parameter] => self.arena.text(parameter.to_owned()),
                    _ => {
                        let parameters = parameters
                            .iter()
                            .map(|parameter| self.arena.text(parameter.to_owned()));
                        self.delimited("(", parameters, ")")
                    }
                };
                let body = self.expression_at(*body, Precedence::Assignment);
                let body = if body_is_object {
                    self.arena.text("(").append(body).append(")")
                } else {
                    body
                };
                parameters.append(" => ").append(body)
            }
        }
    }

    fn precedence(&self, expression: ExpressionId) -> Precedence {
        match &self.tree[expression] {
            Expression::Arrow { .. } => Precedence::Assignment,
            Expression::Unary { .. } => Precedence::Unary,
            Expression::Binary { operator, .. } => operator.precedence(),
            Expression::Call { .. } => Precedence::Call,
            Expression::Member { .. } | Expression::Index { .. } => Precedence::Member,
            Expression::Identifier(_)
            | Expression::String(_)
            | Expression::Number(_)
            | Expression::Boolean(_)
            | Expression::Array(_)
            | Expression::Object(_) => Precedence::Primary,
        }
    }

    fn delimited<I>(&self, open: &'static str, documents: I, close: &'static str) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        let separator = self.arena.text(",").append(self.arena.line());
        let documents = self.arena.intersperse(documents, separator);
        self.arena
            .text(open)
            .append(self.arena.line_().append(documents).nest(2))
            .append(self.arena.line_())
            .append(close)
            .group()
    }

    fn spaced_delimited<I>(&self, open: &'static str, documents: I, close: &'static str) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        let separator = self.arena.text(",").append(self.arena.line());
        let documents = self.arena.intersperse(documents, separator);
        self.arena
            .text(open)
            .append(self.arena.line().append(documents).nest(2))
            .append(self.arena.line())
            .append(close)
            .group()
    }
}

impl UnaryOperator {
    fn source(self) -> &'static str {
        match self {
            UnaryOperator::LogicalNot => "!",
            UnaryOperator::Negate => "-",
        }
    }
}

impl BinaryOperator {
    fn source(self) -> &'static str {
        match self {
            BinaryOperator::StrictEqual => "===",
            BinaryOperator::BitwiseOr => "|",
            BinaryOperator::LogicalAnd => "&&",
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
        }
    }

    fn precedence(self) -> Precedence {
        match self {
            BinaryOperator::LogicalAnd => Precedence::LogicalAnd,
            BinaryOperator::BitwiseOr => Precedence::BitwiseOr,
            BinaryOperator::StrictEqual => Precedence::Equality,
            BinaryOperator::Add | BinaryOperator::Subtract => Precedence::Additive,
            BinaryOperator::Multiply => Precedence::Multiplicative,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Lowest,
    Assignment,
    LogicalAnd,
    BitwiseOr,
    Equality,
    Additive,
    Multiplicative,
    Unary,
    Call,
    Member,
    Primary,
}

impl Precedence {
    fn next(self) -> Precedence {
        match self {
            Precedence::Lowest => Precedence::Assignment,
            Precedence::Assignment => Precedence::LogicalAnd,
            Precedence::LogicalAnd => Precedence::BitwiseOr,
            Precedence::BitwiseOr => Precedence::Equality,
            Precedence::Equality => Precedence::Additive,
            Precedence::Additive => Precedence::Multiplicative,
            Precedence::Multiplicative => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Member,
            Precedence::Member | Precedence::Primary => Precedence::Primary,
        }
    }
}

fn render_property_name(name: &str) -> String {
    if name == "__proto__" {
        format!("[{}]", render_string(name))
    } else if property_is_identifier(name) {
        name.to_owned()
    } else {
        render_string(name)
    }
}

fn property_is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(initial) = characters.next() else {
        return false;
    };
    let valid_initial = initial.is_ascii_alphabetic() || initial == '_' || initial == '$';
    let valid_subsequent = characters
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '$');
    valid_initial && valid_subsequent
}

pub(crate) fn render_string(value: &str) -> String {
    let mut output = String::new();
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{2028}' => output.push_str("\\u2028"),
            '\u{2029}' => output.push_str("\\u2029"),
            character if character.is_control() => {
                let escaped = format!("\\u{:04x}", character as u32);
                output.push_str(&escaped);
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

pub(crate) struct Writer<'a> {
    arena: &'a Arena<'a>,
    lines: Vec<Option<Doc<'a>>>,
}

impl<'a> Writer<'a> {
    pub(crate) fn new(arena: &'a Arena<'a>) -> Writer<'a> {
        Writer { arena, lines: Vec::new() }
    }

    pub(crate) fn line(&mut self, line: impl Into<String>) {
        let line = self.arena.text(line.into());
        self.push_line(line);
    }

    pub(crate) fn expression_line(
        &mut self,
        prefix: impl Into<String>,
        tree: &Tree,
        expression: ExpressionId,
        suffix: &'static str,
    ) {
        let expression = Printer { arena: self.arena, tree }.expression(expression);
        let line = self.arena.text(prefix.into()).append(expression).append(suffix);
        self.push_line(line);
    }

    pub(crate) fn expression_block<R>(
        &mut self,
        prefix: impl Into<String>,
        tree: &mut Tree,
        expression: ExpressionId,
        suffix: &'static str,
        footer: &'static str,
        render: impl FnOnce(&mut Tree, &mut Writer<'a>) -> R,
    ) -> R {
        let expression = Printer { arena: self.arena, tree }.expression(expression);
        let header = self.arena.text(prefix.into()).append(expression).append(suffix);
        self.document_block(header, footer, |writer| render(tree, writer))
    }

    pub(crate) fn block<R>(
        &mut self,
        header: impl Into<String>,
        footer: &'static str,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let header = self.arena.text(header.into());
        self.document_block(header, footer, render)
    }

    pub(crate) fn if_else<E>(
        &mut self,
        tree: &mut Tree,
        condition: ExpressionId,
        render_then: impl FnOnce(&mut Tree, &mut Writer<'a>) -> Result<(), E>,
        render_else: impl FnOnce(&mut Tree, &mut Writer<'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut then_writer = Writer::new(self.arena);
        render_then(tree, &mut then_writer)?;
        let mut else_writer = Writer::new(self.arena);
        render_else(tree, &mut else_writer)?;

        let condition = Printer { arena: self.arena, tree }.expression(condition);
        let then_document = then_writer.document();
        let else_document = else_writer.document();
        let document = self
            .arena
            .text("if (")
            .append(condition)
            .append(") {")
            .append(self.arena.hardline().append(then_document).nest(2))
            .append(self.arena.hardline())
            .append("} else {")
            .append(self.arena.hardline().append(else_document).nest(2))
            .append(self.arena.hardline())
            .append("}");
        self.push_line(document);
        Ok(())
    }

    pub(crate) fn re_export(&mut self, specifiers: impl IntoIterator<Item = String>, path: &str) {
        let specifiers = specifiers.into_iter().map(|specifier| self.arena.text(specifier));
        let separator = self.arena.text(",").append(self.arena.line());
        let specifiers = self.arena.intersperse(specifiers, separator);
        let specifiers = self.arena.line().append(specifiers).nest(2);
        let path = self.arena.text(render_string(path));
        let line = self
            .arena
            .text("export {")
            .append(specifiers)
            .append(self.arena.line())
            .append("} from ")
            .append(path)
            .append(";")
            .group();
        self.push_line(line);
    }

    fn push_line(&mut self, line: Doc<'a>) {
        self.lines.push(Some(line));
    }

    pub(crate) fn blank(&mut self) {
        if self.lines.last().is_some_and(Option::is_some) {
            self.lines.push(None);
        }
    }

    fn document_block<R>(
        &mut self,
        header: Doc<'a>,
        footer: &'static str,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let mut writer = Writer::new(self.arena);
        let result = render(&mut writer);
        let body = writer.document();
        let document = header
            .append(self.arena.hardline().append(body).nest(2))
            .append(self.arena.hardline())
            .append(footer);
        self.push_line(document);
        result
    }

    fn document(mut self) -> Doc<'a> {
        if self.lines.last().is_some_and(Option::is_none) {
            self.lines.pop();
        }
        let lines = self.lines.into_iter().map(|line| line.unwrap_or_else(|| self.arena.nil()));
        self.arena.intersperse(lines, self.arena.hardline())
    }

    pub(crate) fn finish(self) -> String {
        if self.lines.is_empty() {
            return String::new();
        }
        let arena = self.arena;
        let document = self.document().append(arena.hardline());
        let mut output = String::new();
        document
            .render_fmt(DEFAULT_WIDTH, &mut output)
            .expect("critical failure: failed to render JavaScript module");
        // Nested hard lines carry their indentation onto empty lines. Remove that rendering
        // artifact so generated modules contain no trailing whitespace.
        let mut normalized = String::with_capacity(output.len());
        for line in output.lines() {
            normalized.push_str(line.trim_end());
            normalized.push('\n');
        }
        normalized
    }
}
