use std::collections::{BTreeMap, BTreeSet};

use corefn::{
    Annotation, Bind, Binder, CaseAlternative, ConstructorType, Expression as CoreFnExpression,
    GuardedExpression, Literal, Meta, Module as CoreFnModule, PureScriptString, Qualified, RecBind,
    SourceSpan,
};

use crate::ast::{
    BinaryOperator, Comment, Export, Expression, Import, JavaScriptString, Module, Statement,
};
use crate::names::{identifier_to_javascript, module_name_to_javascript};

pub fn generate_module(module: &CoreFnModule) -> Module {
    Generator::new(module).generate()
}

#[derive(Clone, Default)]
struct Scope {
    lazy_bindings: BTreeMap<String, String>,
    shadowed: BTreeSet<String>,
}

impl Scope {
    fn add_lazy_binding(&mut self, identifier: &str, wrapper: String) {
        self.shadowed.remove(identifier);
        self.lazy_bindings.insert(identifier.to_owned(), wrapper);
    }

    fn shadow(&mut self, identifier: &str) {
        self.lazy_bindings.remove(identifier);
        self.shadowed.insert(identifier.to_owned());
    }

    fn lazy_binding(&self, identifier: &str) -> Option<&str> {
        if self.shadowed.contains(identifier) {
            None
        } else {
            self.lazy_bindings.get(identifier).map(String::as_str)
        }
    }
}

struct Generator<'a> {
    module: &'a CoreFnModule,
    module_name: String,
    import_names: BTreeMap<String, String>,
    used_modules: BTreeSet<String>,
    needs_foreign: bool,
    needs_runtime_lazy: bool,
    generated: u32,
}

impl<'a> Generator<'a> {
    fn new(module: &'a CoreFnModule) -> Generator<'a> {
        let module_name = module.module_name.to_dotted();
        let top_level_names = top_level_names(&module.decls);
        let import_names = import_names(module, &top_level_names);
        Generator {
            module,
            module_name,
            import_names,
            used_modules: BTreeSet::new(),
            needs_foreign: !module.foreign.is_empty(),
            needs_runtime_lazy: false,
            generated: 0,
        }
    }

    fn generate(mut self) -> Module {
        for module_name in self.module.re_exports.keys() {
            if !is_prim_module(module_name) {
                self.used_modules.insert(module_name.clone());
            }
        }

        let mut scope = Scope::default();
        let mut statements = vec![];
        self.compile_bindings(&self.module.decls, &mut scope, &mut statements);
        if self.needs_runtime_lazy {
            statements.insert(0, runtime_lazy());
        }

        let comments = self
            .module
            .comments
            .iter()
            .map(|comment| match comment {
                corefn::Comment::LineComment(comment) => Comment::Line(comment.clone()),
                corefn::Comment::BlockComment(comment) => Comment::Block(comment.clone()),
            })
            .collect();
        let imports = self.compile_imports();
        let exports = self.compile_exports();
        Module { comments, imports, statements, exports }
    }

    fn compile_imports(&self) -> Vec<Import> {
        let mut imports = vec![];
        let mut emitted = BTreeSet::new();
        if self.needs_foreign {
            imports
                .push(Import { namespace: "$foreign".to_owned(), path: "./foreign.js".to_owned() });
        }

        let module_names = self
            .module
            .imports
            .iter()
            .map(|import| import.module_name.to_dotted())
            .chain(self.module.re_exports.keys().cloned());
        for module_name in module_names {
            if !self.used_modules.contains(&module_name) || !emitted.insert(module_name.clone()) {
                continue;
            }
            let Some(namespace) = self.import_names.get(&module_name) else { continue };
            imports.push(Import {
                namespace: namespace.clone(),
                path: module_import_path(&module_name),
            });
        }
        imports
    }

    fn compile_exports(&self) -> Vec<Export> {
        let foreign = self.module.foreign.iter().map(|identifier| identifier.0.as_str());
        let foreign = foreign.collect::<BTreeSet<_>>();
        let emitted = emitted_top_level_names(&self.module.decls);

        let foreign_exports = self
            .module
            .exports
            .iter()
            .filter(|identifier| foreign.contains(identifier.0.as_str()))
            .map(|identifier| identifier.0.clone())
            .collect::<Vec<_>>();
        let local_exports = self
            .module
            .exports
            .iter()
            .filter(|identifier| {
                !foreign.contains(identifier.0.as_str()) && emitted.contains(&identifier.0)
            })
            .map(|identifier| identifier.0.clone())
            .collect::<Vec<_>>();

        let mut exports = vec![];
        if !foreign_exports.is_empty() {
            exports.push(Export {
                identifiers: foreign_exports,
                path: Some("./foreign.js".to_owned()),
            });
        }
        if !local_exports.is_empty() {
            exports.push(Export { identifiers: local_exports, path: None });
        }
        for (module_name, identifiers) in &self.module.re_exports {
            if is_prim_module(module_name) || identifiers.is_empty() {
                continue;
            }
            exports.push(Export {
                identifiers: identifiers.iter().map(|identifier| identifier.0.clone()).collect(),
                path: Some(module_import_path(module_name)),
            });
        }
        exports
    }

    fn compile_bindings(
        &mut self,
        bindings: &[Bind],
        scope: &mut Scope,
        statements: &mut Vec<Statement>,
    ) {
        for binding in bindings {
            match binding {
                Bind::NonRec { annotation, identifier, expression } => {
                    if !matches!(annotation.meta, Some(Meta::IsTypeClassConstructor)) {
                        let expression = self.compile_expression(expression, scope);
                        statements.push(Statement::Variable {
                            name: identifier_to_javascript(&identifier.0),
                            value: Some(expression),
                        });
                    }
                    scope.shadow(&identifier.0);
                }
                Bind::Rec { binds } => self.compile_recursive_bindings(binds, scope, statements),
            }
        }
    }

    fn compile_recursive_bindings(
        &mut self,
        bindings: &[RecBind],
        scope: &mut Scope,
        statements: &mut Vec<Statement>,
    ) {
        let bindings = bindings
            .iter()
            .filter(|binding| {
                !matches!(binding.annotation.meta, Some(Meta::IsTypeClassConstructor))
            })
            .collect::<Vec<_>>();
        if bindings.is_empty() {
            return;
        }

        self.needs_runtime_lazy = true;
        let mut recursive_scope = scope.clone();
        for binding in &bindings {
            let wrapper =
                format!("$lazy_{}", crate::names::any_name_to_javascript(&binding.identifier.0));
            recursive_scope.add_lazy_binding(&binding.identifier.0, wrapper);
        }

        for binding in &bindings {
            let wrapper = recursive_scope
                .lazy_binding(&binding.identifier.0)
                .expect("recursive bindings have lazy wrappers")
                .to_owned();
            let initializer = self.compile_expression(&binding.expression, &recursive_scope);
            let thunk = Expression::Function {
                name: None,
                arguments: vec![],
                body: vec![Statement::Return(initializer)],
            };
            let value = Expression::call(
                Expression::identifier("$runtime_lazy"),
                vec![
                    Expression::string(&binding.identifier.0),
                    Expression::string(&self.module_name),
                    thunk,
                ],
            );
            statements.push(Statement::Variable { name: wrapper, value: Some(value) });
        }

        for binding in bindings {
            let wrapper = recursive_scope
                .lazy_binding(&binding.identifier.0)
                .expect("recursive bindings have lazy wrappers")
                .to_owned();
            let value = Expression::call(
                Expression::identifier(wrapper),
                vec![Expression::Number(source_line(&binding.annotation).to_string())],
            );
            statements.push(Statement::Variable {
                name: identifier_to_javascript(&binding.identifier.0),
                value: Some(value),
            });
            scope.shadow(&binding.identifier.0);
        }
    }

    fn compile_expression(&mut self, expression: &CoreFnExpression, scope: &Scope) -> Expression {
        if matches!(expression, CoreFnExpression::App { .. }) {
            return self.compile_application(expression, scope);
        }

        match expression {
            CoreFnExpression::Var { annotation, value } => {
                self.compile_variable(annotation, value, scope, true)
            }
            CoreFnExpression::Literal { value, .. } => self.compile_literal(value, scope),
            CoreFnExpression::Constructor { annotation, constructor_name, field_names, .. } => {
                self.compile_constructor(annotation, constructor_name, field_names)
            }
            CoreFnExpression::Accessor { field_name, expression, .. } => {
                let expression = self.compile_expression(expression, scope);
                Expression::Access {
                    expression: Box::new(expression),
                    property: javascript_string(field_name),
                }
            }
            CoreFnExpression::ObjectUpdate { expression, copy, updates, .. } => {
                self.compile_object_update(expression, copy.as_deref(), updates, scope)
            }
            CoreFnExpression::Abs { annotation, argument, body } => {
                let mut body_scope = scope.clone();
                body_scope.shadow(&argument.0);
                let function = Expression::Function {
                    name: None,
                    arguments: vec![identifier_to_javascript(&argument.0)],
                    body: vec![Statement::Return(self.compile_expression(body, &body_scope))],
                };
                if matches!(annotation.meta, Some(Meta::IsNewtype)) {
                    Expression::Object(vec![(JavaScriptString::from_str("create"), function)])
                } else {
                    function
                }
            }
            CoreFnExpression::Case { annotation, case_expressions, case_alternatives } => {
                self.compile_case(annotation, case_expressions, case_alternatives, scope)
            }
            CoreFnExpression::Let { binds, expression, .. } => {
                let mut body_scope = scope.clone();
                let mut body = vec![];
                self.compile_bindings(binds, &mut body_scope, &mut body);
                body.push(Statement::Return(self.compile_expression(expression, &body_scope)));
                Expression::call(
                    Expression::Function { name: None, arguments: vec![], body },
                    vec![],
                )
            }
            CoreFnExpression::App { .. } => unreachable!("applications are handled above"),
        }
    }

    fn compile_application(&mut self, expression: &CoreFnExpression, scope: &Scope) -> Expression {
        let (head, arguments) = application_head(expression);
        if let CoreFnExpression::Var { annotation, .. } = head {
            if matches!(annotation.meta, Some(Meta::IsNewtype)) {
                let mut arguments = arguments.into_iter();
                let first = arguments
                    .next()
                    .expect("an application headed by a newtype constructor has an argument");
                let mut expression = self.compile_expression(first, scope);
                for argument in arguments {
                    let argument = self.compile_expression(argument, scope);
                    expression = Expression::call(expression, vec![argument]);
                }
                return expression;
            }

            if let Some(Meta::IsConstructor { identifiers, .. }) = &annotation.meta {
                if identifiers.len() == arguments.len() {
                    let constructor = match head {
                        CoreFnExpression::Var { annotation, value } => {
                            self.compile_variable(annotation, value, scope, false)
                        }
                        _ => unreachable!("checked variable head"),
                    };
                    let arguments = arguments
                        .into_iter()
                        .map(|argument| self.compile_expression(argument, scope))
                        .collect();
                    return Expression::New { constructor: Box::new(constructor), arguments };
                }
            }
        }

        let mut expression = self.compile_expression(head, scope);
        for argument in arguments {
            let argument = self.compile_expression(argument, scope);
            expression = Expression::call(expression, vec![argument]);
        }
        expression
    }

    fn compile_variable(
        &mut self,
        annotation: &Annotation,
        qualified: &Qualified<corefn::Identifier>,
        scope: &Scope,
        constructor_projection: bool,
    ) -> Expression {
        let (module_name, identifier) = match qualified {
            Qualified::ByModuleName { module_name, identifier } => {
                (Some(module_name.to_dotted()), identifier.0.as_str())
            }
            Qualified::BySourcePosition { identifier, .. } => (None, identifier.0.as_str()),
        };

        if matches!(annotation.meta, Some(Meta::IsForeign))
            && module_name.as_deref().is_none_or(|name| name == self.module_name)
        {
            self.needs_foreign = true;
            return Expression::access(Expression::identifier("$foreign"), identifier);
        }

        let current_module = module_name.as_deref().is_none_or(|name| name == self.module_name);
        let mut expression = if current_module {
            if let Some(wrapper) = scope.lazy_binding(identifier) {
                Expression::call(
                    Expression::identifier(wrapper),
                    vec![Expression::Number(source_line(annotation).to_string())],
                )
            } else {
                Expression::identifier(identifier_to_javascript(identifier))
            }
        } else if module_name.as_deref().is_some_and(is_prim_module) {
            Expression::identifier(identifier_to_javascript(identifier))
        } else {
            let module_name = module_name.expect("non-current qualified variables have modules");
            self.used_modules.insert(module_name.clone());
            let namespace = self
                .import_names
                .get(&module_name)
                .cloned()
                .unwrap_or_else(|| module_name_to_javascript(&module_name));
            Expression::access(Expression::identifier(namespace), identifier)
        };

        if constructor_projection {
            if let Some(Meta::IsConstructor { identifiers, .. }) = &annotation.meta {
                let property = if identifiers.is_empty() { "value" } else { "create" };
                expression = Expression::access(expression, property);
            }
        }
        expression
    }

    fn compile_literal(
        &mut self,
        literal: &Literal<Box<CoreFnExpression>>,
        scope: &Scope,
    ) -> Expression {
        match literal {
            Literal::IntLiteral(value) => Expression::Number(value.to_string()),
            Literal::NumberLiteral(value) => Expression::Number(javascript_number(value.0)),
            Literal::StringLiteral(value) => Expression::String(javascript_string(value)),
            Literal::CharLiteral(value) => Expression::string(value.to_string()),
            Literal::BooleanLiteral(value) => Expression::Boolean(*value),
            Literal::ArrayLiteral(elements) => {
                let elements = elements
                    .iter()
                    .map(|element| self.compile_expression(element, scope))
                    .collect();
                Expression::Array(elements)
            }
            Literal::ObjectLiteral(properties) => {
                let properties = properties
                    .iter()
                    .map(|(property, value)| {
                        (javascript_string(property), self.compile_expression(value, scope))
                    })
                    .collect();
                Expression::Object(properties)
            }
        }
    }

    fn compile_constructor(
        &mut self,
        annotation: &Annotation,
        constructor_name: &str,
        field_names: &[corefn::Identifier],
    ) -> Expression {
        let name = identifier_to_javascript(constructor_name);
        if matches!(annotation.meta, Some(Meta::IsNewtype)) {
            let value = "$value".to_owned();
            let create = Expression::Function {
                name: None,
                arguments: vec![value.clone()],
                body: vec![Statement::Return(Expression::identifier(value))],
            };
            return Expression::Object(vec![(JavaScriptString::from_str("create"), create)]);
        }

        let arguments =
            field_names.iter().map(|field| identifier_to_javascript(&field.0)).collect::<Vec<_>>();
        let assignments = field_names
            .iter()
            .zip(&arguments)
            .map(|(field, argument)| Statement::Assignment {
                target: Expression::access(Expression::identifier("this"), &field.0),
                value: Expression::identifier(argument),
            })
            .collect();
        let mut body = vec![Statement::Function {
            name: name.clone(),
            arguments: arguments.clone(),
            body: assignments,
        }];

        if arguments.is_empty() {
            let singleton = Expression::New {
                constructor: Box::new(Expression::identifier(&name)),
                arguments: vec![],
            };
            body.push(Statement::Assignment {
                target: Expression::access(Expression::identifier(&name), "value"),
                value: singleton,
            });
        } else {
            let mut create = Expression::New {
                constructor: Box::new(Expression::identifier(&name)),
                arguments: arguments.iter().cloned().map(Expression::identifier).collect(),
            };
            for argument in arguments.iter().rev() {
                create = Expression::Function {
                    name: None,
                    arguments: vec![argument.clone()],
                    body: vec![Statement::Return(create)],
                };
            }
            body.push(Statement::Assignment {
                target: Expression::access(Expression::identifier(&name), "create"),
                value: create,
            });
        }
        body.push(Statement::Return(Expression::identifier(name)));
        Expression::call(Expression::Function { name: None, arguments: vec![], body }, vec![])
    }

    fn compile_object_update(
        &mut self,
        source: &CoreFnExpression,
        copy: Option<&[PureScriptString]>,
        updates: &[(PureScriptString, CoreFnExpression)],
        scope: &Scope,
    ) -> Expression {
        let source_name = self.fresh_identifier("record");
        let output_name = self.fresh_identifier("copy");
        let source_value = self.compile_expression(source, scope);
        let mut body =
            vec![Statement::Variable { name: source_name.clone(), value: Some(source_value) }];

        if let Some(copy) = copy {
            let mut properties = copy
                .iter()
                .map(|property| {
                    let value = Expression::Access {
                        expression: Box::new(Expression::identifier(&source_name)),
                        property: javascript_string(property),
                    };
                    (javascript_string(property), value)
                })
                .collect::<Vec<_>>();
            properties.extend(updates.iter().map(|(property, value)| {
                (javascript_string(property), self.compile_expression(value, scope))
            }));
            body.push(Statement::Return(Expression::Object(properties)));
        } else {
            let key_name = self.fresh_identifier("key");
            body.push(Statement::Variable {
                name: output_name.clone(),
                value: Some(Expression::Object(vec![])),
            });
            let has_own_property = Expression::access(Expression::Object(vec![]), "hasOwnProperty");
            let has_own_call = Expression::access(has_own_property, "call");
            let condition = Expression::call(
                has_own_call,
                vec![Expression::identifier(&source_name), Expression::identifier(&key_name)],
            );
            let copy_property = Statement::Assignment {
                target: Expression::index(
                    Expression::identifier(&output_name),
                    Expression::identifier(&key_name),
                ),
                value: Expression::index(
                    Expression::identifier(&source_name),
                    Expression::identifier(&key_name),
                ),
            };
            body.push(Statement::ForIn {
                key: key_name,
                object: Expression::identifier(&source_name),
                body: vec![Statement::If { condition, body: vec![copy_property] }],
            });
            for (property, value) in updates {
                body.push(Statement::Assignment {
                    target: Expression::Access {
                        expression: Box::new(Expression::identifier(&output_name)),
                        property: javascript_string(property),
                    },
                    value: self.compile_expression(value, scope),
                });
            }
            body.push(Statement::Return(Expression::identifier(output_name)));
        }

        Expression::call(Expression::Function { name: None, arguments: vec![], body }, vec![])
    }

    fn compile_case(
        &mut self,
        annotation: &Annotation,
        case_expressions: &[CoreFnExpression],
        alternatives: &[CaseAlternative],
        scope: &Scope,
    ) -> Expression {
        let mut body = vec![];
        let mut values = vec![];
        for case_expression in case_expressions {
            let name = self.fresh_identifier("case");
            let value = self.compile_expression(case_expression, scope);
            body.push(Statement::Variable { name: name.clone(), value: Some(value) });
            values.push(Expression::identifier(name));
        }

        body.extend(self.compile_case_statements(
            annotation,
            case_expressions,
            alternatives,
            &values,
            scope,
        ));
        Expression::call(Expression::Function { name: None, arguments: vec![], body }, vec![])
    }

    fn compile_case_statements(
        &mut self,
        annotation: &Annotation,
        case_expressions: &[CoreFnExpression],
        alternatives: &[CaseAlternative],
        values: &[Expression],
        scope: &Scope,
    ) -> Vec<Statement> {
        let mut statements = vec![];
        for (index, alternative) in alternatives.iter().enumerate() {
            let final_alternative = index + 1 == alternatives.len();
            if final_alternative {
                if let Some(CoreFnExpression::Case {
                    annotation,
                    case_expressions,
                    case_alternatives,
                }) = unconditional_fallback(alternative)
                {
                    let mut nested_values = vec![];
                    for case_expression in case_expressions {
                        let name = self.fresh_identifier("case");
                        let value = self.compile_expression(case_expression, scope);
                        statements
                            .push(Statement::Variable { name: name.clone(), value: Some(value) });
                        nested_values.push(Expression::identifier(name));
                    }
                    statements.extend(self.compile_case_statements(
                        annotation,
                        case_expressions,
                        case_alternatives,
                        &nested_values,
                        scope,
                    ));
                    return statements;
                }
            }
            statements.extend(self.compile_case_alternative(alternative, values, scope));
        }
        statements.push(self.case_failure(annotation, case_expressions, values));
        statements
    }

    fn compile_case_alternative(
        &mut self,
        alternative: &CaseAlternative,
        values: &[Expression],
        scope: &Scope,
    ) -> Vec<Statement> {
        let (binders, mut body_scope) = match alternative {
            CaseAlternative::Unguarded { binders, .. }
            | CaseAlternative::Guarded { binders, .. } => (binders, scope.clone()),
        };
        for binder in binders {
            shadow_binder_names(binder, &mut body_scope);
        }

        let mut statements = match alternative {
            CaseAlternative::Unguarded { expression, .. } => {
                vec![Statement::Return(self.compile_expression(expression, &body_scope))]
            }
            CaseAlternative::Guarded { expressions, .. } => expressions
                .iter()
                .map(|guarded| self.compile_guarded_expression(guarded, &body_scope))
                .collect(),
        };
        for (binder, value) in binders.iter().zip(values).rev() {
            statements = self.compile_binder(binder, value.clone(), statements);
        }
        statements
    }

    fn compile_guarded_expression(
        &mut self,
        guarded: &GuardedExpression,
        scope: &Scope,
    ) -> Statement {
        Statement::If {
            condition: self.compile_expression(&guarded.guard, scope),
            body: vec![Statement::Return(self.compile_expression(&guarded.expression, scope))],
        }
    }

    fn compile_binder(
        &mut self,
        binder: &Binder,
        value: Expression,
        continuation: Vec<Statement>,
    ) -> Vec<Statement> {
        match binder {
            Binder::NullBinder { .. } => continuation,
            Binder::VarBinder { identifier, .. } => {
                let mut statements = vec![Statement::Variable {
                    name: identifier_to_javascript(&identifier.0),
                    value: Some(value),
                }];
                statements.extend(continuation);
                statements
            }
            Binder::NamedBinder { identifier, binder, .. } => {
                let mut statements = vec![Statement::Variable {
                    name: identifier_to_javascript(&identifier.0),
                    value: Some(value.clone()),
                }];
                statements.extend(self.compile_binder(binder, value, continuation));
                statements
            }
            Binder::LiteralBinder { literal, .. } => {
                self.compile_literal_binder(literal, value, continuation)
            }
            Binder::ConstructorBinder { annotation, constructor_name, binders, .. } => {
                match &annotation.meta {
                    Some(Meta::IsNewtype) => {
                        let mut statements = continuation;
                        for binder in binders.iter().rev() {
                            statements = self.compile_binder(binder, value.clone(), statements);
                        }
                        statements
                    }
                    Some(Meta::IsConstructor { constructor_type, identifiers }) => {
                        let mut statements = continuation;
                        for (index, binder) in binders.iter().enumerate().rev() {
                            let field = identifiers
                                .get(index)
                                .map(|identifier| identifier.0.clone())
                                .unwrap_or_else(|| format!("value{index}"));
                            let field_value = Expression::access(value.clone(), field);
                            statements = self.compile_binder(binder, field_value, statements);
                        }
                        if matches!(constructor_type, ConstructorType::SumType) {
                            let constructor = self.compile_qualified_name(constructor_name);
                            let condition =
                                Expression::binary(value, BinaryOperator::InstanceOf, constructor);
                            vec![Statement::If { condition, body: statements }]
                        } else {
                            statements
                        }
                    }
                    _ => continuation,
                }
            }
        }
    }

    fn compile_literal_binder(
        &mut self,
        literal: &Literal<Box<Binder>>,
        value: Expression,
        continuation: Vec<Statement>,
    ) -> Vec<Statement> {
        match literal {
            Literal::IntLiteral(expected) => {
                equality_binder(value, Expression::Number(expected.to_string()), continuation)
            }
            Literal::NumberLiteral(expected) => equality_binder(
                value,
                Expression::Number(javascript_number(expected.0)),
                continuation,
            ),
            Literal::StringLiteral(expected) => equality_binder(
                value,
                Expression::String(javascript_string(expected)),
                continuation,
            ),
            Literal::CharLiteral(expected) => {
                equality_binder(value, Expression::string(expected.to_string()), continuation)
            }
            Literal::BooleanLiteral(true) => {
                vec![Statement::If { condition: value, body: continuation }]
            }
            Literal::BooleanLiteral(false) => vec![Statement::If {
                condition: Expression::Not(Box::new(value)),
                body: continuation,
            }],
            Literal::ArrayLiteral(binders) => {
                let mut statements = continuation;
                for (index, binder) in binders.iter().enumerate().rev() {
                    let element =
                        Expression::index(value.clone(), Expression::Number(index.to_string()));
                    statements = self.compile_binder(binder, element, statements);
                }
                let length = Expression::access(value, "length");
                equality_binder(length, Expression::Number(binders.len().to_string()), statements)
            }
            Literal::ObjectLiteral(binders) => {
                let mut statements = continuation;
                for (property, binder) in binders.iter().rev() {
                    let property = Expression::Access {
                        expression: Box::new(value.clone()),
                        property: javascript_string(property),
                    };
                    statements = self.compile_binder(binder, property, statements);
                }
                statements
            }
        }
    }

    fn compile_qualified_name(&mut self, qualified: &Qualified<String>) -> Expression {
        match qualified {
            Qualified::BySourcePosition { identifier, .. } => {
                Expression::identifier(identifier_to_javascript(identifier))
            }
            Qualified::ByModuleName { module_name, identifier } => {
                let module_name = module_name.to_dotted();
                if module_name == self.module_name || is_prim_module(&module_name) {
                    Expression::identifier(identifier_to_javascript(identifier))
                } else {
                    self.used_modules.insert(module_name.clone());
                    let namespace = self
                        .import_names
                        .get(&module_name)
                        .cloned()
                        .unwrap_or_else(|| module_name_to_javascript(&module_name));
                    Expression::access(Expression::identifier(namespace), identifier)
                }
            }
        }
    }

    fn case_failure(
        &self,
        annotation: &Annotation,
        case_expressions: &[CoreFnExpression],
        values: &[Expression],
    ) -> Statement {
        let SourceSpan { start, end } = &annotation.source_span;
        let message = format!(
            "Failed pattern match at {} {}:{} - {}:{}: ",
            self.module_name, start.0[0], start.0[1], end.0[0], end.0[1]
        );
        let displays = case_expressions
            .iter()
            .zip(values)
            .map(|(source, value)| {
                if is_primitive_expression(source) {
                    value.clone()
                } else {
                    Expression::access(Expression::access(value.clone(), "constructor"), "name")
                }
            })
            .collect();
        let message = Expression::binary(
            Expression::string(message),
            BinaryOperator::Add,
            Expression::Array(displays),
        );
        let error = Expression::New {
            constructor: Box::new(Expression::identifier("Error")),
            arguments: vec![message],
        };
        Statement::Throw(error)
    }

    fn fresh_identifier(&mut self, purpose: &str) -> String {
        let identifier = format!("${purpose}{}", self.generated);
        self.generated += 1;
        identifier
    }
}

fn import_names(
    module: &CoreFnModule,
    top_level_names: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    let current_module = module.module_name.to_dotted();
    let mut used = top_level_names.clone();
    let mut names = BTreeMap::new();
    let module_names = module
        .imports
        .iter()
        .map(|import| import.module_name.to_dotted())
        .chain(module.re_exports.keys().cloned());
    for module_name in module_names {
        if names.contains_key(&module_name) {
            continue;
        }
        let base = module_name_to_javascript(&module_name);
        let name = if module_name != current_module && used.contains(&base) {
            let mut suffix = 1;
            loop {
                let candidate = format!("{base}_{suffix}");
                if !used.contains(&candidate) {
                    used.insert(candidate.clone());
                    break candidate;
                }
                suffix += 1;
            }
        } else {
            base
        };
        names.insert(module_name, name);
    }
    names
}

fn top_level_names(bindings: &[Bind]) -> BTreeSet<String> {
    let names = bindings.iter().flat_map(|binding| match binding {
        Bind::NonRec { identifier, .. } => vec![identifier.0.clone()],
        Bind::Rec { binds } => binds.iter().map(|binding| binding.identifier.0.clone()).collect(),
    });
    names.collect()
}

fn emitted_top_level_names(bindings: &[Bind]) -> BTreeSet<String> {
    let names = bindings.iter().flat_map(|binding| match binding {
        Bind::NonRec { annotation, identifier, .. } => {
            if matches!(annotation.meta, Some(Meta::IsTypeClassConstructor)) {
                vec![]
            } else {
                vec![identifier.0.clone()]
            }
        }
        Bind::Rec { binds } => binds
            .iter()
            .filter(|binding| {
                !matches!(binding.annotation.meta, Some(Meta::IsTypeClassConstructor))
            })
            .map(|binding| binding.identifier.0.clone())
            .collect(),
    });
    names.collect()
}

fn application_head(expression: &CoreFnExpression) -> (&CoreFnExpression, Vec<&CoreFnExpression>) {
    let mut head = expression;
    let mut arguments = vec![];
    while let CoreFnExpression::App { abstraction, argument, .. } = head {
        arguments.push(argument.as_ref());
        head = abstraction.as_ref();
    }
    arguments.reverse();
    (head, arguments)
}

fn unconditional_fallback(alternative: &CaseAlternative) -> Option<&CoreFnExpression> {
    let CaseAlternative::Unguarded { binders, expression, .. } = alternative else {
        return None;
    };
    binders.iter().all(|binder| matches!(binder, Binder::NullBinder { .. })).then_some(expression)
}

fn shadow_binder_names(binder: &Binder, scope: &mut Scope) {
    match binder {
        Binder::VarBinder { identifier, .. } => scope.shadow(&identifier.0),
        Binder::NamedBinder { identifier, binder, .. } => {
            scope.shadow(&identifier.0);
            shadow_binder_names(binder, scope);
        }
        Binder::LiteralBinder { literal, .. } => match literal {
            Literal::ArrayLiteral(binders) => {
                for binder in binders {
                    shadow_binder_names(binder, scope);
                }
            }
            Literal::ObjectLiteral(binders) => {
                for (_, binder) in binders {
                    shadow_binder_names(binder, scope);
                }
            }
            Literal::IntLiteral(_)
            | Literal::NumberLiteral(_)
            | Literal::StringLiteral(_)
            | Literal::CharLiteral(_)
            | Literal::BooleanLiteral(_) => {}
        },
        Binder::ConstructorBinder { binders, .. } => {
            for binder in binders {
                shadow_binder_names(binder, scope);
            }
        }
        Binder::NullBinder { .. } => {}
    }
}

fn equality_binder(
    value: Expression,
    expected: Expression,
    continuation: Vec<Statement>,
) -> Vec<Statement> {
    let condition = Expression::binary(value, BinaryOperator::StrictEqual, expected);
    vec![Statement::If { condition, body: continuation }]
}

fn runtime_lazy() -> Statement {
    let state = Expression::identifier("state");
    let value = Expression::identifier("value");
    let initialized = Expression::binary(
        state.clone(),
        BinaryOperator::StrictEqual,
        Expression::Number("2".to_owned()),
    );
    let initializing = Expression::binary(
        state.clone(),
        BinaryOperator::StrictEqual,
        Expression::Number("1".to_owned()),
    );
    let message = [
        Expression::identifier("name"),
        Expression::string(" was needed before it finished initializing (module "),
        Expression::identifier("moduleName"),
        Expression::string(", line "),
        Expression::identifier("lineNumber"),
        Expression::string(")"),
    ]
    .into_iter()
    .reduce(|left, right| Expression::binary(left, BinaryOperator::Add, right))
    .expect("runtime lazy message has components");
    let error = Expression::New {
        constructor: Box::new(Expression::identifier("ReferenceError")),
        arguments: vec![
            message,
            Expression::identifier("moduleName"),
            Expression::identifier("lineNumber"),
        ],
    };
    let force = Expression::Function {
        name: None,
        arguments: vec!["lineNumber".to_owned()],
        body: vec![
            Statement::If { condition: initialized, body: vec![Statement::Return(value.clone())] },
            Statement::If { condition: initializing, body: vec![Statement::Throw(error)] },
            Statement::Assignment {
                target: state.clone(),
                value: Expression::Number("1".to_owned()),
            },
            Statement::Assignment {
                target: value.clone(),
                value: Expression::call(Expression::identifier("initialize"), vec![]),
            },
            Statement::Assignment { target: state, value: Expression::Number("2".to_owned()) },
            Statement::Return(value),
        ],
    };
    let factory = Expression::Function {
        name: None,
        arguments: vec!["name".to_owned(), "moduleName".to_owned(), "initialize".to_owned()],
        body: vec![
            Statement::Variable {
                name: "state".to_owned(),
                value: Some(Expression::Number("0".to_owned())),
            },
            Statement::Variable { name: "value".to_owned(), value: None },
            Statement::Return(force),
        ],
    };
    Statement::Variable { name: "$runtime_lazy".to_owned(), value: Some(factory) }
}

fn javascript_string(value: &PureScriptString) -> JavaScriptString {
    match value {
        PureScriptString::String(value) => JavaScriptString::from_str(value),
        PureScriptString::CodeUnits(value) => JavaScriptString::from_code_units(value),
    }
}

fn javascript_number(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_owned()
    } else if value == f64::INFINITY {
        "Infinity".to_owned()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else {
        value.to_string()
    }
}

fn module_import_path(module_name: &str) -> String {
    format!("../{module_name}/index.js")
}

fn source_line(annotation: &Annotation) -> u32 {
    annotation.source_span.start.0[0]
}

fn is_prim_module(module_name: &str) -> bool {
    module_name == "Prim" || module_name.starts_with("Prim.")
}

fn is_primitive_expression(expression: &CoreFnExpression) -> bool {
    matches!(
        expression,
        CoreFnExpression::Literal {
            value: Literal::IntLiteral(_)
                | Literal::NumberLiteral(_)
                | Literal::StringLiteral(_)
                | Literal::CharLiteral(_)
                | Literal::BooleanLiteral(_),
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use corefn::{
        Annotation, Bind, Binder, CaseAlternative, Expression, Identifier, Literal, Meta, Module,
        ModuleName, Qualified, SourcePosition, SourceSpan,
    };

    use super::generate_module;

    fn annotation() -> Annotation {
        Annotation::new(SourceSpan { start: SourcePosition([1, 1]), end: SourcePosition([1, 10]) })
    }

    fn variable(name: &str, meta: Option<Meta>) -> Expression {
        let mut annotation = annotation();
        annotation.meta = meta;
        Expression::Var {
            annotation,
            value: Qualified::ByModuleName {
                module_name: ModuleName::from_dotted("Test.Main"),
                identifier: Identifier::from(name),
            },
        }
    }

    #[test]
    fn generates_newtypes_constructors_and_exports() {
        let dictionary = Bind::NonRec {
            annotation: Annotation::with_meta(
                annotation().source_span,
                Meta::IsTypeClassConstructor,
            ),
            identifier: Identifier::from("$DictClass"),
            expression: variable("unused", None),
        };
        let constructor = Bind::NonRec {
            annotation: annotation(),
            identifier: Identifier::from("Box"),
            expression: Expression::Constructor {
                annotation: annotation(),
                type_name: "Box".to_owned(),
                constructor_name: "Box".to_owned(),
                field_names: vec![Identifier::from("value0")],
            },
        };
        let newtype = Bind::NonRec {
            annotation: annotation(),
            identifier: Identifier::from("Wrap"),
            expression: Expression::Abs {
                annotation: Annotation::with_meta(annotation().source_span, Meta::IsNewtype),
                argument: Identifier::from("wrapped"),
                body: Box::new(variable("wrapped", None)),
            },
        };
        let newtype_value = Expression::App {
            annotation: annotation(),
            abstraction: Box::new(variable("$DictClass", Some(Meta::IsNewtype))),
            argument: Box::new(Expression::Literal {
                annotation: annotation(),
                value: Literal::ObjectLiteral(vec![]),
            }),
        };
        let boxed = Expression::App {
            annotation: annotation(),
            abstraction: Box::new(variable(
                "Box",
                Some(Meta::IsConstructor {
                    constructor_type: corefn::ConstructorType::ProductType,
                    identifiers: vec![Identifier::from("value0")],
                }),
            )),
            argument: Box::new(newtype_value),
        };
        let value = Bind::NonRec {
            annotation: annotation(),
            identifier: Identifier::from("value"),
            expression: boxed,
        };
        let module = Module {
            source_span: annotation().source_span,
            module_name: ModuleName::from_dotted("Test.Main"),
            module_path: "Test.Main.purs".to_owned(),
            imports: vec![],
            exports: vec![
                Identifier::from("$DictClass"),
                Identifier::from("Box"),
                Identifier::from("Wrap"),
                Identifier::from("value"),
            ],
            re_exports: BTreeMap::new(),
            foreign: vec![],
            decls: vec![dictionary, constructor, newtype, value],
            built_with: "test".to_owned(),
            comments: vec![],
        };

        let source = generate_module(&module).to_source();
        assert!(!source.contains("var $dollarDictClass"));
        assert!(!source.contains("$dollarDictClass as"));
        assert!(source.contains("var Box ="));
        assert!(source.contains("var Wrap = {create: function(wrapped)"));
        assert!(source.contains("new (Box)({})"));
        assert!(source.contains("export { Box, Wrap, value };"));
    }

    #[test]
    fn flattens_unconditional_case_fallbacks() {
        let inner = Expression::Case {
            annotation: annotation(),
            case_expressions: vec![Expression::Literal {
                annotation: annotation(),
                value: Literal::BooleanLiteral(true),
            }],
            case_alternatives: vec![CaseAlternative::unguarded(
                vec![Binder::LiteralBinder {
                    annotation: annotation(),
                    literal: Literal::BooleanLiteral(true),
                }],
                Expression::Literal { annotation: annotation(), value: Literal::IntLiteral(1) },
            )],
        };
        let outer = Expression::Case {
            annotation: annotation(),
            case_expressions: vec![Expression::Literal {
                annotation: annotation(),
                value: Literal::BooleanLiteral(true),
            }],
            case_alternatives: vec![CaseAlternative::unguarded(
                vec![Binder::NullBinder { annotation: annotation() }],
                inner,
            )],
        };
        let module = Module {
            source_span: annotation().source_span,
            module_name: ModuleName::from_dotted("Test.Main"),
            module_path: "Test.Main.purs".to_owned(),
            imports: vec![],
            exports: vec![Identifier::from("value")],
            re_exports: BTreeMap::new(),
            foreign: vec![],
            decls: vec![Bind::NonRec {
                annotation: annotation(),
                identifier: Identifier::from("value"),
                expression: outer,
            }],
            built_with: "test".to_owned(),
            comments: vec![],
        };

        let source = generate_module(&module).to_source();
        assert_eq!(source.matches("(function() ").count(), 1);
        assert_eq!(source.matches("var $case").count(), 2);
    }

    #[test]
    fn generates_runtime_laziness_for_recursive_bindings() {
        let recursive = corefn::RecBind {
            identifier: Identifier::from("loop"),
            annotation: annotation(),
            expression: Expression::Abs {
                annotation: annotation(),
                argument: Identifier::from("unit"),
                body: Box::new(variable("loop", None)),
            },
        };
        let module = Module {
            source_span: annotation().source_span,
            module_name: ModuleName::from_dotted("Test.Main"),
            module_path: "Test.Main.purs".to_owned(),
            imports: vec![],
            exports: vec![Identifier::from("loop")],
            re_exports: BTreeMap::new(),
            foreign: vec![],
            decls: vec![Bind::Rec { binds: vec![recursive] }],
            built_with: "test".to_owned(),
            comments: vec![],
        };

        let source = generate_module(&module).to_source();
        assert!(source.contains("var $runtime_lazy ="));
        assert!(source.contains("var $lazy_loop ="));
        assert!(source.contains("return ($lazy_loop)(1);"));
        assert!(source.contains("var loop = ($lazy_loop)(1);"));
    }

    use std::collections::BTreeMap;
}
