//! Oxc JavaScript program construction and code generation.

use itertools::Itertools;
use oxc_allocator::{Allocator, Vec as ArenaVec};
use oxc_ast::ast::{
    Argument, ArrowFunctionBody, AssignmentTarget, BindingIdentifier, BindingPattern, Declaration,
    ExportSpecifier, Expression, FormalParameter, FormalParameterKind, FormalParameters, Function,
    FunctionBody, FunctionType, ImportDeclarationSpecifier, ImportOrExportKind, LabelIdentifier,
    ModuleExportName, Program, Statement, StringLiteral, VariableDeclaration,
    VariableDeclarationKind, VariableDeclarator,
};
use oxc_ast::builder::AstBuilder;
use oxc_codegen::{Codegen, CodegenOptions, IndentChar};
use oxc_span::{SPAN, SourceType};
use oxc_syntax::operator::AssignmentOperator;

use crate::convert::identifier_is_binding;
use crate::tree::{ExpressionId, Tree};

pub(crate) struct Writer<'a> {
    allocator: &'a Allocator,
    builder: AstBuilder<'a>,
    statements: Vec<Statement<'a>>,
}

impl<'a> Writer<'a> {
    pub(crate) fn new(allocator: &'a Allocator) -> Writer<'a> {
        Writer { allocator, builder: AstBuilder::new(allocator), statements: Vec::new() }
    }

    fn text(&self, value: &str) -> &'a str {
        self.allocator.alloc_str(value)
    }

    fn binding(&self, name: &str) -> BindingIdentifier<'a> {
        BindingIdentifier::new(SPAN, self.text(name), &self.builder)
    }

    fn binding_pattern(&self, name: &str) -> BindingPattern<'a> {
        BindingPattern::new_binding_identifier(SPAN, self.text(name), &self.builder)
    }

    fn parameters(&self, parameters: &[String]) -> oxc_allocator::Box<'a, FormalParameters<'a>> {
        let parameters = parameters.iter().map(|parameter| {
            FormalParameter::new(
                SPAN,
                [],
                self.binding_pattern(parameter),
                None,
                None,
                false,
                None,
                false,
                false,
                &self.builder,
            )
        });
        let parameters = parameters.collect_vec();
        let parameters = ArenaVec::from_iter_in(parameters, &self.allocator);
        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::FormalParameter,
            parameters,
            None,
            &self.builder,
        )
    }

    fn function_body(
        &self,
        statements: Vec<Statement<'a>>,
    ) -> oxc_allocator::Box<'a, FunctionBody<'a>> {
        let statements = ArenaVec::from_iter_in(statements, &self.allocator);
        FunctionBody::boxed(SPAN, [], statements, &self.builder)
    }

    fn block_statement(&self, statements: Vec<Statement<'a>>) -> Statement<'a> {
        let statements = ArenaVec::from_iter_in(statements, &self.allocator);
        Statement::new_block_statement(SPAN, statements, &self.builder)
    }

    fn arrow_expression(
        &self,
        parameters: &[String],
        statements: Vec<Statement<'a>>,
    ) -> Expression<'a> {
        let parameters = self.parameters(parameters);
        let body = ArrowFunctionBody::new_function_body(
            SPAN,
            [],
            ArenaVec::from_iter_in(statements, &self.allocator),
            &self.builder,
        );
        Expression::new_arrow_function_expression(
            SPAN,
            false,
            None,
            parameters,
            None,
            body,
            &self.builder,
        )
    }

    fn variable_statement(
        &self,
        kind: VariableDeclarationKind,
        name: &str,
        value: Option<Expression<'a>>,
        exported: bool,
    ) -> Statement<'a> {
        let declarator = VariableDeclarator::new(
            SPAN,
            self.binding_pattern(name),
            None,
            value,
            false,
            &self.builder,
        );
        let declarations = ArenaVec::from_array_in([declarator], &self.allocator);
        let declaration =
            VariableDeclaration::boxed(SPAN, kind, declarations, false, &self.builder);
        if exported {
            Statement::new_export_declaration(
                SPAN,
                Declaration::VariableDeclaration(declaration),
                &self.builder,
            )
        } else {
            Statement::VariableDeclaration(declaration)
        }
    }

    pub(crate) fn import_namespace(&mut self, namespace: &str, path: &str) {
        let specifier = ImportDeclarationSpecifier::new_import_namespace_specifier(
            SPAN,
            self.binding(namespace),
            &self.builder,
        );
        let specifiers = ArenaVec::from_array_in([specifier], &self.allocator);
        let source = StringLiteral::new(SPAN, self.text(path), None, &self.builder);
        let statement = Statement::new_import_declaration(
            SPAN,
            Some(specifiers),
            source,
            None,
            None,
            ImportOrExportKind::Value,
            &self.builder,
        );
        self.statements.push(statement);
    }

    pub(crate) fn constant(
        &mut self,
        tree: &Tree<'_>,
        name: &str,
        value: ExpressionId,
        exported: bool,
    ) {
        let statement = self.variable_statement(
            VariableDeclarationKind::Const,
            name,
            Some(tree.expression_in(value, self.allocator)),
            exported,
        );
        self.statements.push(statement);
    }

    pub(crate) fn mutable(&mut self, name: &str) {
        let statement = self.variable_statement(VariableDeclarationKind::Let, name, None, false);
        self.statements.push(statement);
    }

    pub(crate) fn expression(&self, tree: &Tree<'_>, expression: ExpressionId) -> Expression<'a> {
        tree.expression_in(expression, self.allocator)
    }

    pub(crate) fn assign(&mut self, tree: &Tree<'_>, name: &str, value: ExpressionId) {
        let target = AssignmentTarget::new_assignment_target_identifier(
            SPAN,
            self.text(name),
            &self.builder,
        );
        let expression = Expression::new_assignment_expression(
            SPAN,
            AssignmentOperator::Assign,
            target,
            tree.expression_in(value, self.allocator),
            &self.builder,
        );
        self.statements.push(Statement::new_expression_statement(SPAN, expression, &self.builder));
    }

    pub(crate) fn return_expression(&mut self, tree: &Tree<'_>, value: ExpressionId) {
        self.statements.push(Statement::new_return_statement(
            SPAN,
            Some(tree.expression_in(value, self.allocator)),
            &self.builder,
        ));
    }

    pub(crate) fn break_label(&mut self, label: &str) {
        let label = LabelIdentifier::new(SPAN, self.text(label), &self.builder);
        self.statements.push(Statement::new_break_statement(SPAN, Some(label), &self.builder));
    }

    pub(crate) fn function<R>(
        &mut self,
        name: &str,
        parameters: Vec<String>,
        exported: bool,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(&mut body);
        let function = Function::boxed(
            SPAN,
            FunctionType::FunctionDeclaration,
            Some(self.binding(name)),
            false,
            false,
            false,
            None,
            None,
            self.parameters(&parameters),
            None,
            Some(self.function_body(body.statements)),
            &self.builder,
        );
        let statement = if exported {
            Statement::new_export_declaration(
                SPAN,
                Declaration::FunctionDeclaration(function),
                &self.builder,
            )
        } else {
            Statement::FunctionDeclaration(function)
        };
        self.statements.push(statement);
        result
    }

    pub(crate) fn constant_arrow<R>(
        &mut self,
        name: &str,
        parameters: Vec<String>,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(&mut body);
        let expression = self.arrow_expression(&parameters, body.statements);
        let statement =
            self.variable_statement(VariableDeclarationKind::Const, name, Some(expression), false);
        self.statements.push(statement);
        result
    }

    pub(crate) fn return_arrow<R>(
        &mut self,
        parameters: Vec<String>,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(&mut body);
        let expression = self.arrow_expression(&parameters, body.statements);
        self.statements.push(Statement::new_return_statement(
            SPAN,
            Some(expression),
            &self.builder,
        ));
        result
    }

    pub(crate) fn assign_arrow<R>(
        &mut self,
        name: &str,
        parameters: Vec<String>,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(&mut body);
        let expression = self.arrow_expression(&parameters, body.statements);
        let target = AssignmentTarget::new_assignment_target_identifier(
            SPAN,
            self.text(name),
            &self.builder,
        );
        let expression = Expression::new_assignment_expression(
            SPAN,
            AssignmentOperator::Assign,
            target,
            expression,
            &self.builder,
        );
        self.statements.push(Statement::new_expression_statement(SPAN, expression, &self.builder));
        result
    }

    pub(crate) fn constant_iife<R>(
        &mut self,
        name: &str,
        exported: bool,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(&mut body);
        let arrow = self.arrow_expression(&[], body.statements);
        let call = Expression::new_call_expression(SPAN, arrow, None, [], false, &self.builder);
        let statement =
            self.variable_statement(VariableDeclarationKind::Const, name, Some(call), exported);
        self.statements.push(statement);
        result
    }

    pub(crate) fn binding_call<R>(
        &mut self,
        target: BindingCallTarget<'_>,
        callee: Expression<'a>,
        name: Expression<'a>,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(&mut body);
        let body = self.arrow_expression(&[], body.statements);
        let arguments = [Argument::from(name), Argument::from(body)];
        let arguments = ArenaVec::from_array_in(arguments, &self.allocator);
        let call =
            Expression::new_call_expression(SPAN, callee, None, arguments, false, &self.builder);
        let statement = match target {
            BindingCallTarget::Constant(name) => {
                self.variable_statement(VariableDeclarationKind::Const, name, Some(call), false)
            }
            BindingCallTarget::Assignment(name) => {
                let target = AssignmentTarget::new_assignment_target_identifier(
                    SPAN,
                    self.text(name),
                    &self.builder,
                );
                let assignment = Expression::new_assignment_expression(
                    SPAN,
                    AssignmentOperator::Assign,
                    target,
                    call,
                    &self.builder,
                );
                Statement::new_expression_statement(SPAN, assignment, &self.builder)
            }
        };
        self.statements.push(statement);
        result
    }

    pub(crate) fn if_else<E>(
        &mut self,
        tree: &mut Tree<'_>,
        condition: ExpressionId,
        render_then: impl FnOnce(&mut Tree<'_>, &mut Writer<'a>) -> Result<(), E>,
        render_else: impl FnOnce(&mut Tree<'_>, &mut Writer<'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut then_writer = Writer::new(self.allocator);
        render_then(tree, &mut then_writer)?;
        let mut else_writer = Writer::new(self.allocator);
        render_else(tree, &mut else_writer)?;
        let consequent = self.block_statement(then_writer.statements);
        let alternate = self.block_statement(else_writer.statements);
        self.statements.push(Statement::new_if_statement(
            SPAN,
            tree.expression_in(condition, self.allocator),
            consequent,
            Some(alternate),
            &self.builder,
        ));
        Ok(())
    }

    pub(crate) fn if_else_with_state<E, S>(
        &mut self,
        tree: &mut Tree<'_>,
        condition: ExpressionId,
        state: &mut S,
        render_then: impl FnOnce(&mut Tree<'_>, &mut Writer<'a>, &mut S) -> Result<(), E>,
        render_else: impl FnOnce(&mut Tree<'_>, &mut Writer<'a>, &mut S) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut then_writer = Writer::new(self.allocator);
        render_then(tree, &mut then_writer, state)?;
        let mut else_writer = Writer::new(self.allocator);
        render_else(tree, &mut else_writer, state)?;
        let consequent = self.block_statement(then_writer.statements);
        let alternate = self.block_statement(else_writer.statements);
        self.statements.push(Statement::new_if_statement(
            SPAN,
            tree.expression_in(condition, self.allocator),
            consequent,
            Some(alternate),
            &self.builder,
        ));
        Ok(())
    }

    pub(crate) fn if_block<R>(
        &mut self,
        tree: &mut Tree<'_>,
        condition: ExpressionId,
        render: impl FnOnce(&mut Tree<'_>, &mut Writer<'a>) -> R,
    ) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(tree, &mut body);
        let consequent = self.block_statement(body.statements);
        self.statements.push(Statement::new_if_statement(
            SPAN,
            tree.expression_in(condition, self.allocator),
            consequent,
            None,
            &self.builder,
        ));
        result
    }

    pub(crate) fn block<R>(&mut self, render: impl FnOnce(&mut Writer<'a>) -> R) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(&mut body);
        let statement = self.block_statement(body.statements);
        self.statements.push(statement);
        result
    }

    pub(crate) fn labeled_block<R>(
        &mut self,
        label: &str,
        render: impl FnOnce(&mut Writer<'a>) -> R,
    ) -> R {
        let mut body = Writer::new(self.allocator);
        let result = render(&mut body);
        let body = self.block_statement(body.statements);
        let label = LabelIdentifier::new(SPAN, self.text(label), &self.builder);
        self.statements.push(Statement::new_labeled_statement(SPAN, label, body, &self.builder));
        result
    }

    pub(crate) fn throw_error(&mut self, message: &str) {
        let callee = Expression::new_identifier(SPAN, self.text("Error"), &self.builder);
        let message = Expression::new_string_literal(SPAN, self.text(message), None, &self.builder);
        let arguments = ArenaVec::from_array_in([Argument::from(message)], &self.allocator);
        let error = Expression::new_new_expression(SPAN, callee, None, arguments, &self.builder);
        self.statements.push(Statement::new_throw_statement(SPAN, error, &self.builder));
    }

    fn module_export_name(&self, name: &str, reference: bool) -> ModuleExportName<'a> {
        if identifier_is_binding(name) {
            if reference {
                ModuleExportName::new_identifier_reference(SPAN, self.text(name), &self.builder)
            } else {
                ModuleExportName::new_identifier_name(SPAN, self.text(name), &self.builder)
            }
        } else {
            ModuleExportName::new_string_literal(SPAN, self.text(name), None, &self.builder)
        }
    }

    pub(crate) fn export(&mut self, local: &str, exported: &str) {
        let specifier = ExportSpecifier::new(
            SPAN,
            self.module_export_name(local, true),
            self.module_export_name(exported, false),
            ImportOrExportKind::Value,
            &self.builder,
        );
        self.statements.push(Statement::new_export_named_declaration(
            SPAN,
            [specifier],
            ImportOrExportKind::Value,
            &self.builder,
        ));
    }

    pub(crate) fn re_export(&mut self, specifiers: Vec<String>, path: &str) {
        let specifiers = specifiers.into_iter().map(|name| {
            ExportSpecifier::new(
                SPAN,
                self.module_export_name(&name, false),
                self.module_export_name(&name, false),
                ImportOrExportKind::Value,
                &self.builder,
            )
        });
        let specifiers = specifiers.collect_vec();
        let specifiers = ArenaVec::from_iter_in(specifiers, &self.allocator);
        let source = StringLiteral::new(SPAN, self.text(path), None, &self.builder);
        self.statements.push(Statement::new_export_from_declaration(
            SPAN,
            specifiers,
            source,
            ImportOrExportKind::Value,
            None,
            &self.builder,
        ));
    }

    pub(crate) fn blank(&mut self) {}

    pub(crate) fn finish(self) -> String {
        let body = ArenaVec::from_iter_in(self.statements, &self.allocator);
        let program = Program::new(SPAN, SourceType::mjs(), "", [], None, [], body, &self.builder);
        let options = CodegenOptions {
            indent_char: IndentChar::Space,
            indent_width: 2,
            ..CodegenOptions::default()
        };
        Codegen::new().with_options(options).build(&program).code
    }
}

#[derive(Clone, Copy)]
pub(crate) enum BindingCallTarget<'a> {
    Constant(&'a str),
    Assignment(&'a str),
}
