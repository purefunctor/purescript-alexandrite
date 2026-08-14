use std::io::{self, Write};

use crate::names::{exported_identifier, is_valid_javascript_identifier};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub(crate) comments: Vec<Comment>,
    pub(crate) imports: Vec<Import>,
    pub(crate) statements: Vec<Statement>,
    pub(crate) exports: Vec<Export>,
}

impl Module {
    pub fn serialize(&self, writer: impl Write) -> io::Result<()> {
        Printer::new(writer).write_module(self)
    }

    pub fn to_source(&self) -> String {
        let mut output = Vec::new();
        self.serialize(&mut output).expect("writing JavaScript to a byte vector cannot fail");
        String::from_utf8(output).expect("the JavaScript printer only emits UTF-8")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Comment {
    Line(String),
    Block(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Import {
    pub(crate) namespace: String,
    pub(crate) path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Export {
    pub(crate) identifiers: Vec<String>,
    pub(crate) path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Statement {
    Variable { name: String, value: Option<Expression> },
    Function { name: String, arguments: Vec<String>, body: Vec<Statement> },
    Return(Expression),
    If { condition: Expression, body: Vec<Statement> },
    Throw(Expression),
    ForIn { key: String, object: Expression, body: Vec<Statement> },
    Assignment { target: Expression, value: Expression },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Expression {
    Identifier(String),
    Number(String),
    String(JavaScriptString),
    Boolean(bool),
    Array(Vec<Expression>),
    Object(Vec<(JavaScriptString, Expression)>),
    Function { name: Option<String>, arguments: Vec<String>, body: Vec<Statement> },
    Call { function: Box<Expression>, arguments: Vec<Expression> },
    New { constructor: Box<Expression>, arguments: Vec<Expression> },
    Access { expression: Box<Expression>, property: JavaScriptString },
    Index { expression: Box<Expression>, index: Box<Expression> },
    Not(Box<Expression>),
    Binary { left: Box<Expression>, operator: BinaryOperator, right: Box<Expression> },
}

impl Expression {
    pub(crate) fn identifier(name: impl Into<String>) -> Expression {
        Expression::Identifier(name.into())
    }

    pub(crate) fn string(value: impl AsRef<str>) -> Expression {
        Expression::String(JavaScriptString::from_str(value.as_ref()))
    }

    pub(crate) fn call(function: Expression, arguments: Vec<Expression>) -> Expression {
        Expression::Call { function: Box::new(function), arguments }
    }

    pub(crate) fn access(expression: Expression, property: impl AsRef<str>) -> Expression {
        Expression::Access {
            expression: Box::new(expression),
            property: JavaScriptString::from_str(property.as_ref()),
        }
    }

    pub(crate) fn index(expression: Expression, index: Expression) -> Expression {
        Expression::Index { expression: Box::new(expression), index: Box::new(index) }
    }

    pub(crate) fn binary(
        left: Expression,
        operator: BinaryOperator,
        right: Expression,
    ) -> Expression {
        Expression::Binary { left: Box::new(left), operator, right: Box::new(right) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryOperator {
    Add,
    StrictEqual,
    InstanceOf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JavaScriptString(pub(crate) Vec<u16>);

impl JavaScriptString {
    pub(crate) fn from_str(value: &str) -> JavaScriptString {
        JavaScriptString(value.encode_utf16().collect())
    }

    pub(crate) fn from_code_units(value: &[u16]) -> JavaScriptString {
        JavaScriptString(value.to_vec())
    }

    fn as_string(&self) -> Option<String> {
        String::from_utf16(&self.0).ok()
    }
}

struct Printer<W> {
    writer: W,
    indentation: usize,
}

impl<W> Printer<W>
where
    W: Write,
{
    fn new(writer: W) -> Printer<W> {
        Printer { writer, indentation: 0 }
    }

    fn write_module(&mut self, module: &Module) -> io::Result<()> {
        for comment in &module.comments {
            self.write_comment(comment)?;
        }
        if !module.comments.is_empty() {
            writeln!(self.writer)?;
        }

        for import in &module.imports {
            write!(self.writer, "import * as {} from ", import.namespace)?;
            self.write_string(&JavaScriptString::from_str(&import.path))?;
            writeln!(self.writer, ";")?;
        }
        if !module.imports.is_empty() {
            writeln!(self.writer)?;
        }

        self.write_statements(&module.statements)?;
        if !module.statements.is_empty() && !module.exports.is_empty() {
            writeln!(self.writer)?;
        }

        for export in &module.exports {
            let local = export.path.is_none();
            let identifiers = export
                .identifiers
                .iter()
                .map(|identifier| exported_identifier(identifier, local))
                .collect::<Vec<_>>()
                .join(", ");
            write!(self.writer, "export {{ {identifiers} }}")?;
            if let Some(path) = &export.path {
                write!(self.writer, " from ")?;
                self.write_string(&JavaScriptString::from_str(path))?;
            }
            writeln!(self.writer, ";")?;
        }
        Ok(())
    }

    fn write_comment(&mut self, comment: &Comment) -> io::Result<()> {
        match comment {
            Comment::Line(comment) => {
                for line in comment.lines() {
                    writeln!(self.writer, "//{line}")?;
                }
            }
            Comment::Block(comment) => {
                let comment = comment.replace("*/", "* /");
                writeln!(self.writer, "/**{comment}*/")?;
            }
        }
        Ok(())
    }

    fn write_statements(&mut self, statements: &[Statement]) -> io::Result<()> {
        for statement in statements {
            self.write_statement(statement)?;
        }
        Ok(())
    }

    fn write_statement(&mut self, statement: &Statement) -> io::Result<()> {
        self.write_indentation()?;
        match statement {
            Statement::Variable { name, value } => {
                write!(self.writer, "var {name}")?;
                if let Some(value) = value {
                    write!(self.writer, " = ")?;
                    self.write_expression(value)?;
                }
                writeln!(self.writer, ";")?;
            }
            Statement::Function { name, arguments, body } => {
                write!(self.writer, "function {name}({}) ", arguments.join(", "))?;
                self.write_block(body)?;
                writeln!(self.writer)?;
            }
            Statement::Return(expression) => {
                write!(self.writer, "return ")?;
                self.write_expression(expression)?;
                writeln!(self.writer, ";")?;
            }
            Statement::If { condition, body } => {
                write!(self.writer, "if (")?;
                self.write_expression(condition)?;
                write!(self.writer, ") ")?;
                self.write_block(body)?;
                writeln!(self.writer)?;
            }
            Statement::Throw(expression) => {
                write!(self.writer, "throw ")?;
                self.write_expression(expression)?;
                writeln!(self.writer, ";")?;
            }
            Statement::ForIn { key, object, body } => {
                write!(self.writer, "for (var {key} in ")?;
                self.write_expression(object)?;
                write!(self.writer, ") ")?;
                self.write_block(body)?;
                writeln!(self.writer)?;
            }
            Statement::Assignment { target, value } => {
                self.write_expression(target)?;
                write!(self.writer, " = ")?;
                self.write_expression(value)?;
                writeln!(self.writer, ";")?;
            }
        }
        Ok(())
    }

    fn write_block(&mut self, statements: &[Statement]) -> io::Result<()> {
        writeln!(self.writer, "{{")?;
        self.indentation += 1;
        self.write_statements(statements)?;
        self.indentation -= 1;
        self.write_indentation()?;
        write!(self.writer, "}}")
    }

    fn write_expression(&mut self, expression: &Expression) -> io::Result<()> {
        match expression {
            Expression::Identifier(identifier) => write!(self.writer, "{identifier}"),
            Expression::Number(number) => write!(self.writer, "{number}"),
            Expression::String(string) => self.write_string(string),
            Expression::Boolean(boolean) => write!(self.writer, "{boolean}"),
            Expression::Array(elements) => {
                write!(self.writer, "[")?;
                self.write_expression_list(elements)?;
                write!(self.writer, "]")
            }
            Expression::Object(properties) => {
                write!(self.writer, "{{")?;
                for (index, (property, value)) in properties.iter().enumerate() {
                    if index > 0 {
                        write!(self.writer, ", ")?;
                    }
                    self.write_property(property)?;
                    write!(self.writer, ": ")?;
                    self.write_expression(value)?;
                }
                write!(self.writer, "}}")
            }
            Expression::Function { name, arguments, body } => {
                write!(self.writer, "function")?;
                if let Some(name) = name {
                    write!(self.writer, " {name}")?;
                }
                write!(self.writer, "({}) ", arguments.join(", "))?;
                self.write_block(body)
            }
            Expression::Call { function, arguments } => {
                write!(self.writer, "(")?;
                self.write_expression(function)?;
                write!(self.writer, ")(")?;
                self.write_expression_list(arguments)?;
                write!(self.writer, ")")
            }
            Expression::New { constructor, arguments } => {
                write!(self.writer, "new (")?;
                self.write_expression(constructor)?;
                write!(self.writer, ")(")?;
                self.write_expression_list(arguments)?;
                write!(self.writer, ")")
            }
            Expression::Access { expression, property } => {
                write!(self.writer, "(")?;
                self.write_expression(expression)?;
                write!(self.writer, ")")?;
                if let Some(property) =
                    property.as_string().filter(|p| is_valid_javascript_identifier(p))
                {
                    write!(self.writer, ".{property}")
                } else {
                    write!(self.writer, "[")?;
                    self.write_string(property)?;
                    write!(self.writer, "]")
                }
            }
            Expression::Index { expression, index } => {
                write!(self.writer, "(")?;
                self.write_expression(expression)?;
                write!(self.writer, ")[")?;
                self.write_expression(index)?;
                write!(self.writer, "]")
            }
            Expression::Not(expression) => {
                write!(self.writer, "!(")?;
                self.write_expression(expression)?;
                write!(self.writer, ")")
            }
            Expression::Binary { left, operator, right } => {
                write!(self.writer, "(")?;
                self.write_expression(left)?;
                let operator = match operator {
                    BinaryOperator::Add => "+",
                    BinaryOperator::StrictEqual => "===",
                    BinaryOperator::InstanceOf => "instanceof",
                };
                write!(self.writer, " {operator} ")?;
                self.write_expression(right)?;
                write!(self.writer, ")")
            }
        }
    }

    fn write_expression_list(&mut self, expressions: &[Expression]) -> io::Result<()> {
        for (index, expression) in expressions.iter().enumerate() {
            if index > 0 {
                write!(self.writer, ", ")?;
            }
            self.write_expression(expression)?;
        }
        Ok(())
    }

    fn write_property(&mut self, property: &JavaScriptString) -> io::Result<()> {
        if let Some(property) = property.as_string().filter(|p| is_valid_javascript_identifier(p)) {
            write!(self.writer, "{property}")
        } else {
            self.write_string(property)
        }
    }

    fn write_string(&mut self, string: &JavaScriptString) -> io::Result<()> {
        write!(self.writer, "\"")?;
        for &code_unit in &string.0 {
            match code_unit {
                0x22 => write!(self.writer, "\\\"")?,
                0x5c => write!(self.writer, "\\\\")?,
                0x08 => write!(self.writer, "\\b")?,
                0x09 => write!(self.writer, "\\t")?,
                0x0a => write!(self.writer, "\\n")?,
                0x0b => write!(self.writer, "\\v")?,
                0x0c => write!(self.writer, "\\f")?,
                0x0d => write!(self.writer, "\\r")?,
                0x20..=0x7e => {
                    write!(self.writer, "{}", char::from_u32(code_unit as u32).unwrap())?
                }
                _ => write!(self.writer, "\\u{code_unit:04x}")?,
            }
        }
        write!(self.writer, "\"")
    }

    fn write_indentation(&mut self) -> io::Result<()> {
        for _ in 0..self.indentation {
            write!(self.writer, "    ")?;
        }
        Ok(())
    }
}
