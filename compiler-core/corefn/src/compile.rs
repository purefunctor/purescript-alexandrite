use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use building_types::{QueryProxy, QueryResult};
use checking::CheckedModule;
use checking::evidence::{
    Evidence, EvidenceId, EvidenceState, EvidenceVarId, ReflectableEvidence, ReflectableOrdering,
    SuperclassId, SynthesizedEvidence,
};
use checking::tree::{
    BinderId, BinderKind, BinderSource, DeclarationAbstraction, Equation, ExpressionId,
    ExpressionKind, GuardedExpression, InstanceDeclaration, InstanceImplementation,
    LetBindingChunk, PatternGuard, RecordBinderField, RecordExpressionField,
    RecordExpressionUpdate, TermDeclarationKind, TypeDeclarationKind, VariableResolution,
    WhereExpression,
};
use files::FileId;
use indexing::{IndexedTermItemKind, TermItemId, TypeItemId};
use line_index::{LineCol, LineIndex};
use lowering::{GroupedModule, LoweredModule, TermVariableResolution};
use stabilizing::{AstId, StabilizedModule};
use syntax::ast::AstNode;
use syntax::{TextRange, TextSize};

use crate::model::{
    Annotation, Bind, Binder, CaseAlternative, Comment, ConstructorType, CoreFnNumber, Expression,
    Identifier, Import, Literal, Meta, Module, ModuleName, PureScriptString, Qualified, RecBind,
    SourcePosition, SourceSpan,
};

pub trait ExternalQueries:
    checking::PrettyQueries<
        Parsed = parsing::FullParsedModule,
        Indexed = Arc<indexing::IndexedModule>,
        Lowered = Arc<LoweredModule>,
    > + QueryProxy<
        Parsed = parsing::FullParsedModule,
        Stabilized = Arc<StabilizedModule>,
        Indexed = Arc<indexing::IndexedModule>,
        Lowered = Arc<LoweredModule>,
        Grouped = Arc<GroupedModule>,
        Resolved = Arc<resolving::ResolvedModule>,
        Checked = Arc<CheckedModule>,
    >
{
}

impl<T> ExternalQueries for T where
    T: checking::PrettyQueries<
            Parsed = parsing::FullParsedModule,
            Indexed = Arc<indexing::IndexedModule>,
            Lowered = Arc<LoweredModule>,
        > + QueryProxy<
            Parsed = parsing::FullParsedModule,
            Stabilized = Arc<StabilizedModule>,
            Indexed = Arc<indexing::IndexedModule>,
            Lowered = Arc<LoweredModule>,
            Grouped = Arc<GroupedModule>,
            Resolved = Arc<resolving::ResolvedModule>,
            Checked = Arc<CheckedModule>,
        >
{
}

pub fn compile_module(
    queries: &impl ExternalQueries,
    file_id: FileId,
    module_path: impl Into<String>,
) -> QueryResult<Module> {
    let source = queries.content(file_id);
    let (parsed, _) = queries.parsed(file_id)?;
    let module_name = parsed
        .module_name(&source)
        .map(|name| ModuleName::from_dotted(&name))
        .unwrap_or_else(|| ModuleName::from_dotted("Main"));
    let module_span = parsed.syntax_node().text_range();

    let compiler = Compiler {
        queries,
        file_id,
        line_index: LineIndex::new(&source),
        source,
        stabilized: queries.stabilized(file_id)?,
        indexed: queries.indexed(file_id)?,
        lowered: queries.lowered(file_id)?,
        grouped: queries.grouped(file_id)?,
        checked: queries.checked(file_id)?,
        module_name,
        module_span,
        generated: Cell::new(0),
    };
    compiler.compile(module_path.into())
}

struct Compiler<'a, Q: ?Sized> {
    queries: &'a Q,
    file_id: FileId,
    source: Arc<str>,
    line_index: LineIndex,
    stabilized: Arc<StabilizedModule>,
    indexed: Arc<indexing::IndexedModule>,
    lowered: Arc<LoweredModule>,
    grouped: Arc<GroupedModule>,
    checked: Arc<CheckedModule>,
    module_name: ModuleName,
    module_span: TextRange,
    generated: Cell<u32>,
}

impl<Q> Compiler<'_, Q>
where
    Q: ExternalQueries + ?Sized,
{
    fn compile(&self, module_path: String) -> QueryResult<Module> {
        let imports = self.compile_imports();
        let (exports, re_exports) = self.compile_exports()?;
        let foreign = self.compile_foreign();
        let decls = self.compile_declarations()?;

        Ok(Module {
            source_span: self.source_span(self.module_span),
            module_name: self.module_name.clone(),
            module_path,
            imports,
            exports,
            re_exports,
            foreign,
            decls,
            built_with: env!("CARGO_PKG_VERSION").to_owned(),
            comments: leading_comments(&self.source),
        })
    }

    fn compile_imports(&self) -> Vec<Import> {
        let mut imports = BTreeMap::new();
        for (source, import) in &self.indexed.imports {
            let Some(name) = &import.name else { continue };
            imports.entry(name.to_string()).or_insert_with(|| Import {
                annotation: self.annotation_for_ast(*source),
                module_name: ModuleName::from_dotted(name),
            });
        }
        imports.into_values().collect()
    }

    fn compile_exports(&self) -> QueryResult<(Vec<Identifier>, BTreeMap<String, Vec<Identifier>>)> {
        let mut exports = BTreeSet::new();
        let mut re_exports: BTreeMap<String, BTreeSet<Identifier>> = BTreeMap::new();

        for (name, file_id, _) in self.queries.resolved(self.file_id)?.exports.iter_terms() {
            if file_id == self.file_id {
                exports.insert(Identifier::from(name.to_string()));
            } else {
                let module_name = self.module_name_for(file_id)?.to_dotted();
                re_exports
                    .entry(module_name)
                    .or_default()
                    .insert(Identifier::from(name.to_string()));
            }
        }

        let re_exports = re_exports
            .into_iter()
            .map(|(module_name, identifiers)| (module_name, identifiers.into_iter().collect()))
            .collect();
        Ok((exports.into_iter().collect(), re_exports))
    }

    fn compile_foreign(&self) -> Vec<Identifier> {
        let mut foreign = self
            .indexed
            .items
            .iter_terms()
            .filter_map(|(_, item)| {
                matches!(item.kind, IndexedTermItemKind::Foreign { .. })
                    .then(|| item.name.as_ref())
                    .flatten()
                    .map(|name| Identifier::from(name.to_string()))
            })
            .collect::<Vec<_>>();
        foreign.sort();
        foreign.dedup();
        foreign
    }

    fn compile_declarations(&self) -> QueryResult<Vec<Bind>> {
        let mut declarations = vec![];

        let type_items =
            self.indexed.items.iter_types().map(|(item_id, _)| item_id).collect::<Vec<_>>();
        for item_id in type_items {
            if let Some(declaration) = self.compile_class_constructor(item_id)? {
                declarations.push(declaration);
            }
        }

        let term_groups = self.grouped.term_scc.clone();
        for group in &term_groups {
            let mut binds = vec![];
            for &term_id in group.as_slice() {
                if let Some(declaration) = self.compile_term(term_id)? {
                    binds.push(declaration);
                }
            }

            if binds.is_empty() {
                continue;
            }
            if !group.is_recursive() && binds.len() == 1 {
                let bind = binds.pop().expect("checked non-empty");
                declarations.push(Bind::NonRec {
                    annotation: bind.annotation,
                    identifier: bind.identifier,
                    expression: bind.expression,
                });
            } else {
                declarations.push(Bind::Rec { binds });
            }
        }

        Ok(declarations)
    }

    fn compile_class_constructor(&self, type_id: TypeItemId) -> QueryResult<Option<Bind>> {
        let Some(declaration_id) = self.checked.tree.lookup_type_declaration(type_id) else {
            return Ok(None);
        };
        let declaration = &self.checked.tree[declaration_id];
        if !matches!(declaration.declaration, TypeDeclarationKind::Class(_)) {
            return Ok(None);
        }
        let Some(name) = self.indexed.items[type_id].name.as_ref() else {
            return Ok(None);
        };
        let dictionary_name = Self::class_dictionary_name(name);

        let source_span = self.span_for_type(type_id);
        let argument = Identifier::from("$dictionary");
        let body = self.local_variable(argument.clone(), source_span.clone());
        let expression = Expression::Abs {
            annotation: Annotation::with_meta(source_span.clone(), Meta::IsNewtype),
            argument,
            body: Box::new(body),
        };
        Ok(Some(Bind::NonRec {
            annotation: Annotation::with_meta(source_span, Meta::IsTypeClassConstructor),
            identifier: Identifier::from(dictionary_name),
            expression,
        }))
    }

    fn compile_term(&self, term_id: TermItemId) -> QueryResult<Option<RecBind>> {
        let item = &self.indexed.items[term_id];
        let annotation = Annotation::new(self.span_for_term(term_id));
        let identifier = Identifier::from(self.term_name(self.file_id, term_id)?);

        let expression = if let Some(declaration_id) = self.checked.tree.lookup_term(term_id) {
            let declaration = &self.checked.tree[declaration_id];
            match &declaration.kind {
                TermDeclarationKind::Value(value) => {
                    Some(self.compile_value(&value.abstractions, &value.equations, &annotation)?)
                }
                TermDeclarationKind::Foreign => None,
                TermDeclarationKind::Constructor(constructor) => Some(self.compile_constructor(
                    term_id,
                    constructor.arguments.len(),
                    &annotation,
                )?),
                TermDeclarationKind::Instance(instance) => {
                    Some(self.compile_instance(instance, &annotation)?)
                }
            }
        } else if matches!(item.kind, IndexedTermItemKind::ClassMember { .. }) {
            Some(self.compile_class_accessor(term_id, &annotation)?)
        } else {
            None
        };

        Ok(expression.map(|expression| RecBind { identifier, annotation, expression }))
    }

    fn compile_constructor(
        &self,
        term_id: TermItemId,
        arity: usize,
        annotation: &Annotation,
    ) -> QueryResult<Expression> {
        let type_id = self.indexed.constructor_type(term_id);
        let is_newtype = type_id.is_some_and(|type_id| {
            let Some(declaration_id) = self.checked.tree.lookup_type_declaration(type_id) else {
                return false;
            };
            matches!(self.checked.tree[declaration_id].declaration, TypeDeclarationKind::Newtype(_))
        });
        if is_newtype {
            let argument = Identifier::from("$value");
            let body = self.local_variable(argument.clone(), annotation.source_span.clone());
            return Ok(Expression::Abs {
                annotation: Annotation::with_meta(annotation.source_span.clone(), Meta::IsNewtype),
                argument,
                body: Box::new(body),
            });
        }

        let type_name = if let Some(type_id) = type_id {
            self.type_name(self.file_id, type_id)?
        } else {
            "$Unknown".to_owned()
        };
        let constructor_name = self.term_name(self.file_id, term_id)?;
        Ok(Expression::Constructor {
            annotation: annotation.clone(),
            type_name,
            constructor_name,
            field_names: Self::constructor_fields(arity),
        })
    }

    fn compile_class_accessor(
        &self,
        term_id: TermItemId,
        annotation: &Annotation,
    ) -> QueryResult<Expression> {
        let class_id = self
            .class_for_member(term_id)
            .expect("invariant violated: class member has no declaring class");
        let class_name = self.type_name(self.file_id, class_id)?;
        let dictionary_name = Self::class_dictionary_name(&class_name);
        let dictionary_argument = Identifier::from("$dictionary");
        let dictionary =
            self.local_variable(dictionary_argument.clone(), annotation.source_span.clone());
        let record_argument = self.fresh_identifier("dictionaryRecord");
        let record = self.local_variable(record_argument.clone(), annotation.source_span.clone());
        let field_name = self.indexed.items[term_id]
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| self.generated_term_name(term_id));
        let body = Expression::Accessor {
            annotation: annotation.clone(),
            field_name: PureScriptString::from(field_name),
            expression: Box::new(record),
        };
        let binder_annotation =
            Annotation::with_meta(annotation.source_span.clone(), Meta::IsNewtype);
        let dictionary_name = Qualified::ByModuleName {
            module_name: self.module_name.clone(),
            identifier: dictionary_name,
        };
        let body = Expression::Case {
            annotation: annotation.clone(),
            case_expressions: vec![dictionary],
            case_alternatives: vec![CaseAlternative::unguarded(
                vec![Binder::ConstructorBinder {
                    annotation: binder_annotation,
                    type_name: dictionary_name.clone(),
                    constructor_name: dictionary_name,
                    binders: vec![Binder::VarBinder {
                        annotation: annotation.clone(),
                        identifier: record_argument,
                    }],
                }],
                body,
            )],
        };
        Ok(Expression::Abs {
            annotation: annotation.clone(),
            argument: dictionary_argument,
            body: Box::new(body),
        })
    }

    fn compile_value(
        &self,
        abstractions: &[DeclarationAbstraction],
        equations: &[Equation],
        annotation: &Annotation,
    ) -> QueryResult<Expression> {
        let arity = equations.iter().map(|equation| equation.binders.len()).max().unwrap_or(0);
        let arguments = (0..arity).map(|_| self.fresh_identifier("case")).collect::<Vec<_>>();
        let argument_expressions = arguments
            .iter()
            .cloned()
            .map(|argument| self.local_variable(argument, annotation.source_span.clone()))
            .collect::<Vec<_>>();

        let mut expression = if equations.len() == 1 && arity == 0 {
            self.compile_guarded_expression(&equations[0].guarded_expression, None, annotation)?
        } else {
            self.compile_equation_chain(&argument_expressions, equations, annotation)?
        };

        for argument in arguments.into_iter().rev() {
            expression = Expression::Abs {
                annotation: annotation.clone(),
                argument,
                body: Box::new(expression),
            };
        }
        for abstraction in abstractions.iter().rev() {
            let DeclarationAbstraction::Evidence { evidence, .. } = abstraction else {
                continue;
            };
            let argument = self.evidence_abstraction_name(evidence);
            expression = Expression::Abs {
                annotation: annotation.clone(),
                argument,
                body: Box::new(expression),
            };
        }
        Ok(expression)
    }

    fn compile_equation_chain(
        &self,
        arguments: &[Expression],
        equations: &[Equation],
        annotation: &Annotation,
    ) -> QueryResult<Expression> {
        let mut fallback = None;
        for equation in equations.iter().rev() {
            let success = self.compile_guarded_expression(
                &equation.guarded_expression,
                fallback.clone(),
                annotation,
            )?;
            let mut binders = equation
                .binders
                .iter()
                .map(|&binder| self.compile_binder(binder))
                .collect::<QueryResult<Vec<_>>>()?;
            while binders.len() < arguments.len() {
                binders.push(Binder::NullBinder { annotation: annotation.clone() });
            }

            let mut alternatives = vec![CaseAlternative::unguarded(binders, success)];
            if let Some(fallback_expression) = fallback {
                let binders = arguments
                    .iter()
                    .map(|_| Binder::NullBinder { annotation: annotation.clone() })
                    .collect();
                alternatives.push(CaseAlternative::unguarded(binders, fallback_expression));
            }
            fallback = Some(Expression::Case {
                annotation: annotation.clone(),
                case_expressions: arguments.to_vec(),
                case_alternatives: alternatives,
            });
        }
        Ok(fallback.unwrap_or_else(|| self.undefined_expression(annotation.source_span.clone())))
    }

    fn compile_guarded_expression(
        &self,
        guarded: &GuardedExpression,
        fallback: Option<Expression>,
        annotation: &Annotation,
    ) -> QueryResult<Expression> {
        let mut next = fallback;
        for alternative in guarded.alternatives.iter().rev() {
            let mut success = self.compile_where_expression(&alternative.where_expression)?;
            for guard in alternative.pattern_guards.iter().rev() {
                let guard_annotation = match guard {
                    PatternGuard::Boolean { expression } => {
                        self.annotation_for_expression(*expression)
                    }
                    PatternGuard::Pattern { binder, .. } => self.annotation_for_binder(*binder),
                };
                let (case_expression, binder) = match guard {
                    PatternGuard::Boolean { expression } => (
                        self.compile_expression(*expression)?,
                        Binder::LiteralBinder {
                            annotation: guard_annotation.clone(),
                            literal: Literal::BooleanLiteral(true),
                        },
                    ),
                    PatternGuard::Pattern { binder, expression } => {
                        (self.compile_expression(*expression)?, self.compile_binder(*binder)?)
                    }
                };
                let mut alternatives = vec![CaseAlternative::unguarded(vec![binder], success)];
                if let Some(fallback_expression) = next.clone() {
                    alternatives.push(CaseAlternative::unguarded(
                        vec![Binder::NullBinder { annotation: guard_annotation.clone() }],
                        fallback_expression,
                    ));
                }
                success = Expression::Case {
                    annotation: guard_annotation,
                    case_expressions: vec![case_expression],
                    case_alternatives: alternatives,
                };
            }
            next = Some(success);
        }
        Ok(next.unwrap_or_else(|| self.undefined_expression(annotation.source_span.clone())))
    }

    fn compile_where_expression(&self, expression: &WhereExpression) -> QueryResult<Expression> {
        let body = self.compile_expression(expression.expression)?;
        self.compile_let_chunks(&expression.bindings.chunks, body)
    }

    fn compile_let_chunks(
        &self,
        chunks: &[LetBindingChunk],
        mut body: Expression,
    ) -> QueryResult<Expression> {
        for chunk in chunks.iter().rev() {
            match chunk {
                LetBindingChunk::Pattern { binder, where_expression, .. } => {
                    let value = self.compile_where_expression(where_expression)?;
                    let annotation = self.annotation_for_binder(*binder);
                    body = Expression::Case {
                        annotation,
                        case_expressions: vec![value],
                        case_alternatives: vec![CaseAlternative::unguarded(
                            vec![self.compile_binder(*binder)?],
                            body,
                        )],
                    };
                }
                LetBindingChunk::PatternError { where_expression, .. } => {
                    if let Some(where_expression) = where_expression {
                        let value = self.compile_where_expression(where_expression)?;
                        let annotation = Annotation::new(self.null_span());
                        let identifier = self.fresh_identifier("pattern");
                        body = Expression::Let {
                            annotation: annotation.clone(),
                            binds: vec![Bind::NonRec { annotation, identifier, expression: value }],
                            expression: Box::new(body),
                        };
                    }
                }
                LetBindingChunk::Names { declarations, groups } => {
                    let mut binds = vec![];
                    for group in groups.iter() {
                        let mut recursive_binds = vec![];
                        for &source in group.as_slice() {
                            let Some(declaration_id) = self.checked.tree.lookup_let(source) else {
                                continue;
                            };
                            if !declarations.contains(&declaration_id) {
                                continue;
                            }
                            recursive_binds.push(self.compile_local_declaration(declaration_id)?);
                        }
                        if recursive_binds.is_empty() {
                            continue;
                        }
                        if !group.is_recursive() && recursive_binds.len() == 1 {
                            let bind = recursive_binds.pop().expect("checked non-empty");
                            binds.push(Bind::NonRec {
                                annotation: bind.annotation,
                                identifier: bind.identifier,
                                expression: bind.expression,
                            });
                        } else {
                            binds.push(Bind::Rec { binds: recursive_binds });
                        }
                    }
                    if !binds.is_empty() {
                        body = Expression::Let {
                            annotation: Annotation::new(self.null_span()),
                            binds,
                            expression: Box::new(body),
                        };
                    }
                }
            }
        }
        Ok(body)
    }

    fn compile_local_declaration(
        &self,
        declaration_id: checking::tree::LocalDeclarationId,
    ) -> QueryResult<RecBind> {
        let declaration = &self.checked.tree[declaration_id];
        let source_span = self.span_for_local(declaration.source);
        let annotation = Annotation::new(source_span);
        let identifier = Identifier::from(self.local_name(declaration.source));
        let expression = self.compile_value(
            &declaration.value.abstractions,
            &declaration.value.equations,
            &annotation,
        )?;
        Ok(RecBind { identifier, annotation, expression })
    }

    fn compile_expression(&self, expression_id: ExpressionId) -> QueryResult<Expression> {
        let expression = &self.checked.tree[expression_id];
        let annotation = self.annotation_for_expression(expression_id);
        match &expression.kind {
            ExpressionKind::Error => Ok(self.undefined_expression(annotation.source_span)),
            ExpressionKind::String { value, .. } => Ok(Expression::Literal {
                annotation,
                value: Literal::StringLiteral(value.to_string().into()),
            }),
            ExpressionKind::Char { value } => {
                Ok(Expression::Literal { annotation, value: Literal::CharLiteral(*value) })
            }
            ExpressionKind::Boolean { value } => {
                Ok(Expression::Literal { annotation, value: Literal::BooleanLiteral(*value) })
            }
            ExpressionKind::Integer { value } => {
                Ok(Expression::Literal { annotation, value: Literal::IntLiteral(*value) })
            }
            ExpressionKind::Number { value } => Ok(Expression::Literal {
                annotation,
                value: Literal::NumberLiteral(CoreFnNumber(parse_number(value))),
            }),
            ExpressionKind::Array { elements } => {
                let elements = elements
                    .iter()
                    .map(|&element| self.compile_expression(element).map(Box::new))
                    .collect::<QueryResult<Vec<_>>>()?;
                Ok(Expression::Literal { annotation, value: Literal::ArrayLiteral(elements) })
            }
            ExpressionKind::Record { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| match field {
                        RecordExpressionField::Field { label, expression }
                        | RecordExpressionField::Pun { label, expression, .. } => self
                            .compile_expression(*expression)
                            .map(|expression| (label.to_string().into(), Box::new(expression))),
                    })
                    .collect::<QueryResult<Vec<_>>>()?;
                Ok(Expression::Literal { annotation, value: Literal::ObjectLiteral(fields) })
            }
            ExpressionKind::RecordAccess { record, labels } => {
                let mut expression = self.compile_expression(*record)?;
                for label in labels.iter() {
                    expression = Expression::Accessor {
                        annotation: annotation.clone(),
                        field_name: label.to_string().into(),
                        expression: Box::new(expression),
                    };
                }
                Ok(expression)
            }
            ExpressionKind::RecordUpdate { record, updates } => {
                let record = self.compile_expression(*record)?;
                self.compile_record_updates(record, updates, annotation)
            }
            ExpressionKind::Constructor { resolution } => {
                self.compile_term_reference(resolution.0, resolution.1, annotation, true)
            }
            ExpressionKind::Variable { resolution }
            | ExpressionKind::RecordPun { resolution, .. } => {
                self.compile_variable(*resolution, annotation)
            }
            ExpressionKind::Section { binder } => {
                let name = Identifier::from(self.binder_name(*binder));
                Ok(self.local_variable(name, annotation.source_span))
            }
            ExpressionKind::TermApplication { function, argument } => Ok(Expression::App {
                annotation,
                abstraction: Box::new(self.compile_expression(*function)?),
                argument: Box::new(self.compile_expression(*argument)?),
            }),
            ExpressionKind::EvidenceApplication { function, evidence, .. } => Ok(Expression::App {
                annotation: Annotation::with_meta(annotation.source_span, Meta::IsSyntheticApp),
                abstraction: Box::new(self.compile_expression(*function)?),
                argument: Box::new(self.compile_evidence_variable(*evidence)?),
            }),
            ExpressionKind::EvidenceAbstraction { binder, expression } => Ok(Expression::Abs {
                annotation,
                argument: self.evidence_binder_name(*binder),
                body: Box::new(self.compile_expression(*expression)?),
            }),
            ExpressionKind::Lambda { binders, expression } => {
                let body = self.compile_expression(*expression)?;
                self.compile_lambda(binders, body, annotation)
            }
            ExpressionKind::IfThenElse { condition, then, else_ } => {
                let condition = self.compile_expression(*condition)?;
                let then = self.compile_expression(*then)?;
                let else_ = self.compile_expression(*else_)?;
                Ok(Expression::Case {
                    annotation: annotation.clone(),
                    case_expressions: vec![condition],
                    case_alternatives: vec![
                        CaseAlternative::unguarded(
                            vec![Binder::LiteralBinder {
                                annotation: annotation.clone(),
                                literal: Literal::BooleanLiteral(true),
                            }],
                            then,
                        ),
                        CaseAlternative::unguarded(
                            vec![Binder::NullBinder { annotation: annotation.clone() }],
                            else_,
                        ),
                    ],
                })
            }
            ExpressionKind::Case { scrutinees, alternatives } => {
                self.compile_case(scrutinees, alternatives, annotation)
            }
            ExpressionKind::Let { bindings, expression } => {
                let body = self.compile_expression(*expression)?;
                self.compile_let_chunks(&bindings.chunks, body)
            }
        }
    }

    fn compile_record_updates(
        &self,
        record: Expression,
        updates: &[RecordExpressionUpdate],
        annotation: Annotation,
    ) -> QueryResult<Expression> {
        let mut compiled = vec![];
        for update in updates {
            match update {
                RecordExpressionUpdate::Error => {}
                RecordExpressionUpdate::Leaf { label, expression } => {
                    compiled
                        .push((label.to_string().into(), self.compile_expression(*expression)?));
                }
                RecordExpressionUpdate::Branch { label, updates } => {
                    let nested_record = Expression::Accessor {
                        annotation: annotation.clone(),
                        field_name: label.to_string().into(),
                        expression: Box::new(record.clone()),
                    };
                    let nested =
                        self.compile_record_updates(nested_record, updates, annotation.clone())?;
                    compiled.push((label.to_string().into(), nested));
                }
            }
        }
        Ok(Expression::ObjectUpdate {
            annotation,
            expression: Box::new(record),
            copy: None,
            updates: compiled,
        })
    }

    fn compile_variable(
        &self,
        resolution: VariableResolution,
        annotation: Annotation,
    ) -> QueryResult<Expression> {
        match resolution {
            VariableResolution::Generated(binder) => {
                let identifier = Identifier::from(self.binder_name(binder));
                Ok(self.local_variable(identifier, annotation.source_span))
            }
            VariableResolution::Source(TermVariableResolution::Reference(file_id, term_id)) => {
                self.compile_term_reference(file_id, term_id, annotation, false)
            }
            VariableResolution::Source(TermVariableResolution::Binder(binder)) => {
                let identifier = Identifier::from(self.source_binder_name(binder));
                Ok(self.local_variable(identifier, annotation.source_span))
            }
            VariableResolution::Source(TermVariableResolution::Let(local)) => {
                let identifier = Identifier::from(self.local_name(local));
                Ok(self.local_variable(identifier, annotation.source_span))
            }
            VariableResolution::Source(TermVariableResolution::RecordPun(pun)) => {
                let name = self
                    .lowered
                    .tree
                    .get_expression_pun(pun)
                    .and_then(|resolution| match resolution {
                        TermVariableResolution::Binder(binder) => {
                            Some(self.source_binder_name(binder))
                        }
                        TermVariableResolution::Let(local) => Some(self.local_name(local)),
                        TermVariableResolution::RecordPun(_)
                        | TermVariableResolution::Reference(..) => None,
                    })
                    .unwrap_or_else(|| format!("$pun{}", pun.into_raw().get()));
                Ok(self.local_variable(Identifier::from(name), annotation.source_span))
            }
        }
    }

    fn compile_term_reference(
        &self,
        file_id: FileId,
        term_id: TermItemId,
        mut annotation: Annotation,
        constructor: bool,
    ) -> QueryResult<Expression> {
        let indexed = if file_id == self.file_id {
            Arc::clone(&self.indexed)
        } else {
            self.queries.indexed(file_id)?
        };
        if constructor {
            annotation.meta = Some(self.constructor_meta(file_id, term_id)?);
        } else if matches!(indexed.items[term_id].kind, IndexedTermItemKind::Foreign { .. }) {
            annotation.meta = Some(Meta::IsForeign);
        }
        let value = Qualified::ByModuleName {
            module_name: self.module_name_for(file_id)?,
            identifier: Identifier::from(self.term_name(file_id, term_id)?),
        };
        Ok(Expression::Var { annotation, value })
    }

    fn compile_lambda(
        &self,
        binders: &[BinderId],
        mut body: Expression,
        annotation: Annotation,
    ) -> QueryResult<Expression> {
        for &binder_id in binders.iter().rev() {
            let binder = &self.checked.tree[binder_id];
            let binder_annotation = self.annotation_for_binder(binder_id);
            let simple = matches!(binder.kind, BinderKind::Variable);
            let argument = if simple {
                Identifier::from(self.binder_name(binder_id))
            } else {
                self.fresh_identifier("argument")
            };
            if !simple {
                let variable =
                    self.local_variable(argument.clone(), binder_annotation.source_span.clone());
                body = Expression::Case {
                    annotation: binder_annotation,
                    case_expressions: vec![variable],
                    case_alternatives: vec![CaseAlternative::unguarded(
                        vec![self.compile_binder(binder_id)?],
                        body,
                    )],
                };
            }
            body =
                Expression::Abs { annotation: annotation.clone(), argument, body: Box::new(body) };
        }
        Ok(body)
    }

    fn compile_case(
        &self,
        scrutinees: &[ExpressionId],
        alternatives: &[checking::tree::CaseAlternative],
        annotation: Annotation,
    ) -> QueryResult<Expression> {
        let mut binds = vec![];
        let mut variables = vec![];
        for &scrutinee in scrutinees {
            let identifier = self.fresh_identifier("scrutinee");
            binds.push(Bind::NonRec {
                annotation: annotation.clone(),
                identifier: identifier.clone(),
                expression: self.compile_expression(scrutinee)?,
            });
            variables.push(self.local_variable(identifier, annotation.source_span.clone()));
        }

        let mut fallback = None;
        for alternative in alternatives.iter().rev() {
            let success = self.compile_guarded_expression(
                &alternative.guarded_expression,
                fallback.clone(),
                &annotation,
            )?;
            let binders = alternative
                .binders
                .iter()
                .map(|&binder| self.compile_binder(binder))
                .collect::<QueryResult<Vec<_>>>()?;
            let mut case_alternatives = vec![CaseAlternative::unguarded(binders, success)];
            if let Some(fallback_expression) = fallback {
                let binders = variables
                    .iter()
                    .map(|_| Binder::NullBinder { annotation: annotation.clone() })
                    .collect();
                case_alternatives.push(CaseAlternative::unguarded(binders, fallback_expression));
            }
            fallback = Some(Expression::Case {
                annotation: annotation.clone(),
                case_expressions: variables.clone(),
                case_alternatives,
            });
        }

        let body =
            fallback.unwrap_or_else(|| self.undefined_expression(annotation.source_span.clone()));
        if binds.is_empty() {
            Ok(body)
        } else {
            Ok(Expression::Let { annotation, binds, expression: Box::new(body) })
        }
    }

    fn compile_binder(&self, binder_id: BinderId) -> QueryResult<Binder> {
        let binder = &self.checked.tree[binder_id];
        let annotation = self.annotation_for_binder(binder_id);
        match &binder.kind {
            BinderKind::Error | BinderKind::Wildcard => Ok(Binder::NullBinder { annotation }),
            BinderKind::Typed { binder, .. } => self.compile_binder(*binder),
            BinderKind::Integer { value } => {
                Ok(Binder::LiteralBinder { annotation, literal: Literal::IntLiteral(*value) })
            }
            BinderKind::Number { negative, value } => {
                let value = if *negative { format!("-{value}") } else { value.to_string() };
                let value = CoreFnNumber(parse_number(&value));
                Ok(Binder::LiteralBinder { annotation, literal: Literal::NumberLiteral(value) })
            }
            BinderKind::Variable => Ok(Binder::VarBinder {
                annotation,
                identifier: Identifier::from(self.binder_name(binder_id)),
            }),
            BinderKind::Named { name, binder } => Ok(Binder::NamedBinder {
                annotation,
                identifier: Identifier::from(name.to_string()),
                binder: Box::new(self.compile_binder(*binder)?),
            }),
            BinderKind::String { value } => Ok(Binder::LiteralBinder {
                annotation,
                literal: Literal::StringLiteral(value.to_string().into()),
            }),
            BinderKind::Char { value } => {
                Ok(Binder::LiteralBinder { annotation, literal: Literal::CharLiteral(*value) })
            }
            BinderKind::Boolean { value } => {
                Ok(Binder::LiteralBinder { annotation, literal: Literal::BooleanLiteral(*value) })
            }
            BinderKind::Array { elements } => {
                let elements = elements
                    .iter()
                    .map(|&element| self.compile_binder(element).map(Box::new))
                    .collect::<QueryResult<Vec<_>>>()?;
                Ok(Binder::LiteralBinder { annotation, literal: Literal::ArrayLiteral(elements) })
            }
            BinderKind::Record { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| match field {
                        RecordBinderField::Field { label, binder } => self
                            .compile_binder(*binder)
                            .map(|binder| (label.to_string().into(), Box::new(binder))),
                        RecordBinderField::Pun { label } => Ok((
                            label.to_string().into(),
                            Box::new(Binder::VarBinder {
                                annotation: annotation.clone(),
                                identifier: Identifier::from(label.to_string()),
                            }),
                        )),
                    })
                    .collect::<QueryResult<Vec<_>>>()?;
                Ok(Binder::LiteralBinder { annotation, literal: Literal::ObjectLiteral(fields) })
            }
            BinderKind::Constructor { resolution, arguments } => {
                let module_name = self.module_name_for(resolution.0)?;
                let indexed = if resolution.0 == self.file_id {
                    Arc::clone(&self.indexed)
                } else {
                    self.queries.indexed(resolution.0)?
                };
                let type_name = indexed
                    .constructor_type(resolution.1)
                    .map(|type_id| self.type_name(resolution.0, type_id))
                    .transpose()?
                    .unwrap_or_else(|| "$Unknown".to_owned());
                let constructor_name = self.term_name(resolution.0, resolution.1)?;
                let binders = arguments
                    .iter()
                    .map(|&argument| self.compile_binder(argument))
                    .collect::<QueryResult<Vec<_>>>()?;
                let annotation = Annotation::with_meta(
                    annotation.source_span,
                    self.constructor_meta(resolution.0, resolution.1)?,
                );
                Ok(Binder::ConstructorBinder {
                    annotation,
                    type_name: Qualified::ByModuleName {
                        module_name: module_name.clone(),
                        identifier: type_name,
                    },
                    constructor_name: Qualified::ByModuleName {
                        module_name,
                        identifier: constructor_name,
                    },
                    binders,
                })
            }
        }
    }

    fn compile_instance(
        &self,
        instance: &InstanceDeclaration,
        annotation: &Annotation,
    ) -> QueryResult<Expression> {
        let mut expression = match &instance.implementation {
            InstanceImplementation::Delegate { evidence, .. } => {
                self.compile_evidence_variable(*evidence)?
            }
            InstanceImplementation::Members(members) => {
                let mut fields = vec![];
                for superclass in instance.superclasses.iter() {
                    let field = self.superclass_field_name(superclass.id)?;
                    let value = self.compile_evidence_variable(superclass.evidence)?;
                    let value = Expression::Abs {
                        annotation: Annotation::new(annotation.source_span.clone()),
                        argument: self.fresh_identifier("unit"),
                        body: Box::new(value),
                    };
                    fields.push((field.into(), Box::new(value)));
                }
                for member in members.iter() {
                    let field = self.term_name(member.resolution.0, member.resolution.1)?;
                    let member_annotation = Annotation::new(annotation.source_span.clone());
                    let value = self.compile_value(
                        &member.abstractions,
                        &member.equations,
                        &member_annotation,
                    )?;
                    fields.push((field.into(), Box::new(value)));
                }
                let record = Expression::Literal {
                    annotation: annotation.clone(),
                    value: Literal::ObjectLiteral(fields),
                };
                let constructor = Expression::Var {
                    annotation: Annotation::with_meta(
                        annotation.source_span.clone(),
                        Meta::IsNewtype,
                    ),
                    value: Qualified::ByModuleName {
                        module_name: self.module_name_for(instance.class.0)?,
                        identifier: Identifier::from(Self::class_dictionary_name(
                            &self.type_name(instance.class.0, instance.class.1)?,
                        )),
                    },
                };
                Expression::App {
                    annotation: Annotation::with_meta(
                        annotation.source_span.clone(),
                        Meta::IsSyntheticApp,
                    ),
                    abstraction: Box::new(constructor),
                    argument: Box::new(record),
                }
            }
        };

        for evidence in instance.evidences.iter().rev() {
            let argument = self.evidence_abstraction_name(&evidence.evidence);
            expression = Expression::Abs {
                annotation: annotation.clone(),
                argument,
                body: Box::new(expression),
            };
        }
        Ok(expression)
    }

    fn compile_evidence_variable(&self, evidence: EvidenceVarId) -> QueryResult<Expression> {
        match self.checked.evidence[evidence].state {
            EvidenceState::Solved(proof) => self.compile_evidence(proof),
            EvidenceState::Unsolved | EvidenceState::Error => {
                Ok(self.empty_record(self.null_span()))
            }
        }
    }

    fn compile_evidence(&self, evidence_id: EvidenceId) -> QueryResult<Expression> {
        let evidence = self.checked.evidence[evidence_id].clone();
        match evidence {
            Evidence::Variable(variable) => self.compile_evidence_variable(variable),
            Evidence::Given(binder) => {
                let identifier = self.evidence_binder_name(binder);
                Ok(self.local_variable(identifier, self.null_span()))
            }
            Evidence::Instance { origin, subgoals } => {
                let (file_id, term_id) = self.instance_term(origin)?;
                let annotation = Annotation::new(self.null_span());
                let mut expression =
                    self.compile_term_reference(file_id, term_id, annotation.clone(), false)?;
                for subgoal in subgoals {
                    expression = Expression::App {
                        annotation: Annotation::with_meta(
                            annotation.source_span.clone(),
                            Meta::IsSyntheticApp,
                        ),
                        abstraction: Box::new(expression),
                        argument: Box::new(self.compile_evidence_variable(subgoal)?),
                    };
                }
                Ok(expression)
            }
            Evidence::Superclass { parent, superclass } => {
                let source_span = self.null_span();
                let accessor = Expression::Accessor {
                    annotation: Annotation::new(source_span.clone()),
                    field_name: self.superclass_field_name(superclass)?.into(),
                    expression: Box::new(self.compile_evidence(parent)?),
                };
                Ok(Expression::App {
                    annotation: Annotation::with_meta(source_span.clone(), Meta::IsSyntheticApp),
                    abstraction: Box::new(accessor),
                    argument: Box::new(self.empty_record(source_span)),
                })
            }
            Evidence::Trivial => Ok(self.empty_record(self.null_span())),
            Evidence::Synthesized(evidence) => self.compile_synthesized_evidence(evidence),
        }
    }

    fn compile_synthesized_evidence(
        &self,
        evidence: SynthesizedEvidence,
    ) -> QueryResult<Expression> {
        let (field, value) = match evidence {
            SynthesizedEvidence::IsSymbol(value) => (
                "reflectSymbol",
                Expression::Literal {
                    annotation: Annotation::new(self.null_span()),
                    value: Literal::StringLiteral(value.to_string().into()),
                },
            ),
            SynthesizedEvidence::Reflectable(value) => {
                ("reflectType", self.compile_reflectable(value))
            }
        };
        let argument = Identifier::from("$proxy");
        let member = Expression::Abs {
            annotation: Annotation::new(self.null_span()),
            argument,
            body: Box::new(value),
        };
        Ok(Expression::Literal {
            annotation: Annotation::new(self.null_span()),
            value: Literal::ObjectLiteral(vec![(field.to_owned().into(), Box::new(member))]),
        })
    }

    fn compile_reflectable(&self, evidence: ReflectableEvidence) -> Expression {
        let annotation = Annotation::new(self.null_span());
        match evidence {
            ReflectableEvidence::Integer(value) => {
                Expression::Literal { annotation, value: Literal::IntLiteral(value) }
            }
            ReflectableEvidence::String(value) => Expression::Literal {
                annotation,
                value: Literal::StringLiteral(value.to_string().into()),
            },
            ReflectableEvidence::Boolean(value) => {
                Expression::Literal { annotation, value: Literal::BooleanLiteral(value) }
            }
            ReflectableEvidence::Ordering(ordering) => {
                let identifier = match ordering {
                    ReflectableOrdering::Less => "LT",
                    ReflectableOrdering::Equal => "EQ",
                    ReflectableOrdering::Greater => "GT",
                };
                Expression::Var {
                    annotation,
                    value: Qualified::ByModuleName {
                        module_name: ModuleName::from_dotted("Data.Ordering"),
                        identifier: Identifier::from(identifier),
                    },
                }
            }
        }
    }

    fn instance_term(
        &self,
        origin: checking::evidence::InstanceCandidateOrigin,
    ) -> QueryResult<(FileId, TermItemId)> {
        let file_id = match origin {
            checking::evidence::InstanceCandidateOrigin::Instance(file_id, _)
            | checking::evidence::InstanceCandidateOrigin::Derive(file_id, _) => file_id,
        };
        let indexed = if file_id == self.file_id {
            Arc::clone(&self.indexed)
        } else {
            self.queries.indexed(file_id)?
        };
        let term_id = indexed
            .items
            .iter_terms()
            .find_map(|(term_id, item)| {
                let matches = match (&item.kind, origin) {
                    (
                        IndexedTermItemKind::Instance { id },
                        checking::evidence::InstanceCandidateOrigin::Instance(_, origin_id),
                    ) => *id == origin_id,
                    (
                        IndexedTermItemKind::Derive { id },
                        checking::evidence::InstanceCandidateOrigin::Derive(_, origin_id),
                    ) => *id == origin_id,
                    _ => false,
                };
                matches.then_some(term_id)
            })
            .expect("invariant violated: instance origin is not indexed");
        Ok((file_id, term_id))
    }

    fn constructor_meta(&self, file_id: FileId, term_id: TermItemId) -> QueryResult<Meta> {
        let indexed = if file_id == self.file_id {
            Arc::clone(&self.indexed)
        } else {
            self.queries.indexed(file_id)?
        };
        let checked = if file_id == self.file_id {
            Arc::clone(&self.checked)
        } else {
            self.queries.checked(file_id)?
        };
        let Some(type_id) = indexed.constructor_type(term_id) else {
            return Ok(Meta::IsConstructor {
                constructor_type: ConstructorType::ProductType,
                identifiers: vec![],
            });
        };
        let Some(declaration_id) = checked.tree.lookup_type_declaration(type_id) else {
            return Ok(Meta::IsConstructor {
                constructor_type: ConstructorType::ProductType,
                identifiers: vec![],
            });
        };
        let declaration = &checked.tree[declaration_id];
        let data = match &declaration.declaration {
            TypeDeclarationKind::Newtype(_) => return Ok(Meta::IsNewtype),
            TypeDeclarationKind::Data(data) => data,
            TypeDeclarationKind::Class(_) => return Ok(Meta::IsTypeClassConstructor),
        };
        let constructor_type = if data.constructors.len() == 1 {
            ConstructorType::ProductType
        } else {
            ConstructorType::SumType
        };
        let arity = checked
            .tree
            .lookup_term(term_id)
            .and_then(|declaration_id| match &checked.tree[declaration_id].kind {
                TermDeclarationKind::Constructor(constructor) => Some(constructor.arguments.len()),
                _ => None,
            })
            .unwrap_or(0);
        Ok(Meta::IsConstructor { constructor_type, identifiers: Self::constructor_fields(arity) })
    }

    fn superclass_field_name(&self, superclass: SuperclassId) -> QueryResult<String> {
        let checked = if superclass.file_id == self.file_id {
            Arc::clone(&self.checked)
        } else {
            self.queries.checked(superclass.file_id)?
        };
        let Some(declaration_id) = checked.tree.lookup_type_declaration(superclass.type_id) else {
            return Ok(format!("$superclass{}", superclass.source_id.into_raw().get()));
        };
        let TypeDeclarationKind::Class(class) = &checked.tree[declaration_id].declaration else {
            return Ok(format!("$superclass{}", superclass.source_id.into_raw().get()));
        };
        let position =
            class.superclasses.iter().position(|candidate| candidate.id == superclass).unwrap_or(0);
        Ok(format!("$superclass{position}"))
    }

    fn term_name(&self, file_id: FileId, term_id: TermItemId) -> QueryResult<String> {
        let indexed = if file_id == self.file_id {
            Arc::clone(&self.indexed)
        } else {
            self.queries.indexed(file_id)?
        };
        Ok(indexed.items[term_id]
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| self.generated_term_name(term_id)))
    }

    fn generated_term_name(&self, term_id: TermItemId) -> String {
        format!("$instance{}", term_id.into_raw().into_u32())
    }

    fn type_name(&self, file_id: FileId, type_id: TypeItemId) -> QueryResult<String> {
        let indexed = if file_id == self.file_id {
            Arc::clone(&self.indexed)
        } else {
            self.queries.indexed(file_id)?
        };
        Ok(indexed.items[type_id]
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("$type{}", type_id.into_raw().into_u32())))
    }

    fn class_for_member(&self, member_id: TermItemId) -> Option<TypeItemId> {
        self.indexed.items.iter_types().find_map(|(type_id, _)| {
            self.indexed
                .class_members(type_id)
                .any(|candidate| candidate == member_id)
                .then_some(type_id)
        })
    }

    fn class_dictionary_name(class_name: &str) -> String {
        format!("{class_name}$Dict")
    }

    fn module_name_for(&self, file_id: FileId) -> QueryResult<ModuleName> {
        if file_id == self.file_id {
            return Ok(self.module_name.clone());
        }
        let content = self.queries.content(file_id);
        let (parsed, _) = self.queries.parsed(file_id)?;
        Ok(parsed
            .module_name(&content)
            .map(|name| ModuleName::from_dotted(&name))
            .unwrap_or_else(|| ModuleName::from_dotted("Main")))
    }

    fn binder_name(&self, binder_id: BinderId) -> String {
        let binder = &self.checked.tree[binder_id];
        match binder.source {
            BinderSource::Binder(source) => self.source_binder_name(source),
            BinderSource::DoStatement(source) => format!("$do{}", source.into_raw().get()),
            BinderSource::Operator(source) => format!("$operator{}", source.into_raw().get()),
            BinderSource::Section(source) => format!("$section{}", source.into_raw().get()),
            BinderSource::Generated { name, .. } => self.queries.lookup_smol_str(name).to_string(),
        }
    }

    fn source_binder_name(&self, binder: lowering::BinderId) -> String {
        match self.lowered.tree.get_binder_kind(binder) {
            Some(lowering::BinderKind::Variable { variable: Some(variable) }) => {
                variable.to_string()
            }
            Some(lowering::BinderKind::Named { named: Some(named), .. }) => named.to_string(),
            _ => format!("$binder{}", binder.into_raw().get()),
        }
    }

    fn local_name(&self, local: lowering::LetBindingNameGroupId) -> String {
        self.lowered
            .tree
            .get_let_binding_group(local)
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("$let{}", local.into_raw().into_u32()))
    }

    fn evidence_abstraction_name(&self, evidence: &Evidence) -> Identifier {
        match evidence {
            Evidence::Given(binder) => self.evidence_binder_name(*binder),
            _ => Identifier::from("$dictionary"),
        }
    }

    fn evidence_binder_name(&self, binder: checking::evidence::EvidenceBinderId) -> Identifier {
        Identifier::from(format!("$dict{}", binder.0))
    }

    fn fresh_identifier(&self, purpose: &str) -> Identifier {
        let generated = self.generated.get();
        let identifier = Identifier::from(format!("${purpose}{generated}"));
        self.generated.set(generated + 1);
        identifier
    }

    fn constructor_fields(arity: usize) -> Vec<Identifier> {
        (0..arity).map(|index| Identifier::from(format!("value{index}"))).collect()
    }

    fn local_variable(&self, identifier: Identifier, source_span: SourceSpan) -> Expression {
        Expression::Var {
            annotation: Annotation::new(source_span.clone()),
            value: Qualified::BySourcePosition { source_position: source_span.start, identifier },
        }
    }

    fn undefined_expression(&self, source_span: SourceSpan) -> Expression {
        Expression::Var {
            annotation: Annotation::new(source_span),
            value: Qualified::BySourcePosition {
                source_position: SourcePosition([0, 0]),
                identifier: Identifier::from("undefined"),
            },
        }
    }

    fn empty_record(&self, source_span: SourceSpan) -> Expression {
        Expression::Literal {
            annotation: Annotation::new(source_span),
            value: Literal::ObjectLiteral(vec![]),
        }
    }

    fn annotation_for_expression(&self, expression: ExpressionId) -> Annotation {
        let source_span = self
            .checked
            .tree
            .lookup_expression_source(expression)
            .and_then(|source| self.stabilized.syntax_ptr(source))
            .map(|pointer| self.source_span(pointer.text_range()))
            .unwrap_or_else(|| self.null_span());
        Annotation::new(source_span)
    }

    fn annotation_for_binder(&self, binder: BinderId) -> Annotation {
        let source_span = match self.checked.tree[binder].source {
            BinderSource::Binder(source) => self.span_for_ast(source),
            BinderSource::DoStatement(source) => self.span_for_ast(source),
            BinderSource::Operator(source) => self.span_for_ast(source),
            BinderSource::Section(source) => self.span_for_ast(source),
            BinderSource::Generated { derive, .. } => self.span_for_ast(derive),
        };
        Annotation::new(source_span)
    }

    fn span_for_term(&self, term_id: TermItemId) -> SourceSpan {
        self.indexed
            .term_item_ptr(&self.stabilized, term_id)
            .next()
            .map(|pointer| self.source_span(pointer.text_range()))
            .unwrap_or_else(|| self.null_span())
    }

    fn span_for_type(&self, type_id: TypeItemId) -> SourceSpan {
        self.indexed
            .type_item_ptr(&self.stabilized, type_id)
            .next()
            .map(|pointer| self.source_span(pointer.text_range()))
            .unwrap_or_else(|| self.null_span())
    }

    fn span_for_local(&self, local: lowering::LetBindingNameGroupId) -> SourceSpan {
        let group = self.lowered.tree.get_let_binding_group(local);
        if let Some(signature) = group.signature {
            return self.span_for_ast(signature);
        }
        group
            .equations
            .first()
            .map(|equation| self.span_for_ast(*equation))
            .unwrap_or_else(|| self.null_span())
    }

    fn annotation_for_ast<N: AstNode>(&self, id: AstId<N>) -> Annotation {
        Annotation::new(self.span_for_ast(id))
    }

    fn span_for_ast<N: AstNode>(&self, id: AstId<N>) -> SourceSpan {
        self.stabilized
            .syntax_ptr(id)
            .map(|pointer| pointer.text_range())
            .map(|range| self.source_span(range))
            .unwrap_or_else(|| self.null_span())
    }

    fn null_span(&self) -> SourceSpan {
        SourceSpan::null()
    }

    fn source_span(&self, range: TextRange) -> SourceSpan {
        SourceSpan {
            start: self.source_position(range.start()),
            end: self.source_position(range.end()),
        }
    }

    fn source_position(&self, offset: TextSize) -> SourcePosition {
        let LineCol { line, col } = self.line_index.line_col(offset);
        let line_start =
            self.line_index.line(line).map(|range| usize::from(range.start())).unwrap_or(0);
        let offset = usize::from(offset);
        let byte_column = usize::try_from(col).unwrap_or(0);
        let end = line_start.saturating_add(byte_column).min(offset).min(self.source.len());
        let column = self.source[line_start..end].chars().count() as u32;
        SourcePosition([line + 1, column + 1])
    }
}

fn leading_comments(source: &str) -> Vec<Comment> {
    let mut comments = vec![];
    let mut offset = 0;

    while offset < source.len() {
        let remaining = &source[offset..];
        if remaining.starts_with("--") {
            let content_start = offset + 2;
            let content_end = source[content_start..]
                .find('\n')
                .map(|length| content_start + length)
                .unwrap_or(source.len());
            comments.push(Comment::LineComment(source[content_start..content_end].to_owned()));
            offset = content_end;
            continue;
        }
        if remaining.starts_with("{-") {
            let content_start = offset + 2;
            let mut level = 1;
            offset = content_start;
            while offset < source.len() && level > 0 {
                let remaining = &source[offset..];
                if remaining.starts_with("{-") {
                    level += 1;
                    offset += 2;
                } else if remaining.starts_with("-}") {
                    level -= 1;
                    offset += 2;
                } else {
                    let character = remaining.chars().next().expect("checked non-empty");
                    offset += character.len_utf8();
                }
            }
            let content_end = if level == 0 { offset - 2 } else { source.len() };
            comments.push(Comment::BlockComment(source[content_start..content_end].to_owned()));
            continue;
        }

        let character = remaining.chars().next().expect("checked non-empty");
        if !character.is_whitespace() {
            break;
        }
        offset += character.len_utf8();
    }

    comments
}

fn parse_number(value: &str) -> f64 {
    if value.contains('_') {
        let value = value.replace('_', "");
        serde_json::from_str(&value).expect("invariant violated: checked number literal")
    } else {
        serde_json::from_str(value).expect("invariant violated: checked number literal")
    }
}

#[cfg(test)]
mod tests {
    use crate::Comment;

    use super::{leading_comments, parse_number};

    #[test]
    fn collects_comments_before_the_module_header() {
        let source = "-- line\n{- block {- nested -} end -}\nmodule Main where\n-- declaration";

        assert_eq!(
            leading_comments(source),
            vec![
                Comment::LineComment(" line".to_owned()),
                Comment::BlockComment(" block {- nested -} end ".to_owned()),
            ]
        );
    }

    #[test]
    fn parses_number_separators() {
        assert_eq!(parse_number("1_000.25e-2"), 10.0025);
    }
}
