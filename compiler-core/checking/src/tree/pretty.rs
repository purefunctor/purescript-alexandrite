//! Implements the pretty printer for the checked semantic tree.

use building_types::QueryResult;
use files::FileId;
use indexing::{TermItem, TypeItem};
use lowering::TermVariableResolution;
use pretty::{Arena, DocAllocator, DocBuilder};
use smol_str::{SmolStr, SmolStrBuilder};

use crate::CheckedModule;
use crate::core::Type;
use crate::core::pretty::{Pretty as TypePretty, PrettyQueries};
use crate::evidence::Evidence;
use crate::tree::{
    BinderId, BinderKind, ExpressionId, ExpressionKind, GuardedAlternative, GuardedExpression,
    PatternGuard, TermDeclarationKind, TypeDeclarationKind,
};

type Doc<'a> = DocBuilder<'a, Arena<'a>, ()>;

pub struct Pretty<'a, Q: ?Sized> {
    queries: &'a Q,
    width: usize,
    checked: &'a CheckedModule,
}

impl<'a, Q> Pretty<'a, Q>
where
    Q: PrettyQueries + ?Sized,
{
    pub fn new(queries: &'a Q, checked: &'a CheckedModule) -> Pretty<'a, Q> {
        Pretty { queries, width: 100, checked }
    }

    pub fn width(mut self, width: usize) -> Pretty<'a, Q> {
        self.width = width;
        self
    }

    pub fn render(&self, file_id: FileId) -> QueryResult<SmolStr> {
        let indexed = self.queries.indexed(file_id)?;
        let lowered = self.queries.lowered(file_id)?;
        let arena = Arena::new();
        let mut printer = Printer::new(
            &arena,
            self.queries,
            file_id,
            &indexed,
            &lowered,
            self.checked,
            self.width,
        );
        let document = printer.module()?;

        let mut output = SmolStrBuilder::new();
        document
            .render_fmt(self.width, &mut output)
            .expect("critical failure: failed to render checked semantic tree");
        Ok(output.finish())
    }
}

struct Printer<'arena, 'context, 'module, Q>
where
    Q: PrettyQueries + ?Sized,
{
    arena: &'arena Arena<'arena>,
    queries: &'context Q,
    file_id: FileId,
    indexed: &'module indexing::IndexedModule,
    lowered: &'module lowering::LoweredModule,
    checked: &'context CheckedModule,
    type_pretty: TypePretty<'context, Q>,
    width: usize,
}

impl<'arena, 'context, 'module, Q> Printer<'arena, 'context, 'module, Q>
where
    Q: PrettyQueries + ?Sized,
{
    fn new(
        arena: &'arena Arena<'arena>,
        queries: &'context Q,
        file_id: FileId,
        indexed: &'module indexing::IndexedModule,
        lowered: &'module lowering::LoweredModule,
        checked: &'context CheckedModule,
        width: usize,
    ) -> Printer<'arena, 'context, 'module, Q> {
        let type_pretty = TypePretty::new(queries, checked).without_rigid_kinds().width(width);
        Printer { arena, queries, file_id, indexed, lowered, checked, type_pretty, width }
    }

    fn module(&mut self) -> QueryResult<Doc<'arena>> {
        let mut declarations = vec![];

        for (type_id, TypeItem { name, .. }) in self.indexed.items.iter_types() {
            let Some(name) = name else { continue };
            let Some(declaration_id) = self.checked.tree.lookup_type_declaration(type_id) else {
                continue;
            };

            let declaration = &self.checked.tree[declaration_id];
            let keyword = match &declaration.declaration {
                TypeDeclarationKind::Data(_) => "data",
                TypeDeclarationKind::Newtype(_) => "newtype",
            };
            let kind = declaration.kind;

            self.type_pretty.reset();
            let signature = self.type_pretty.render_kind_signature(name, kind);
            let signature = self.arena.text(format!("{keyword} {signature}"));

            let declaration = self.data_declaration(type_id, keyword, name);

            declarations.push(signature.append(self.arena.hardline()).append(declaration));
        }

        for (term_id, TermItem { name, .. }) in self.indexed.items.iter_terms() {
            let Some(name) = name else { continue };
            let Some(declaration_id) = self.checked.tree.lookup_term(term_id) else {
                continue;
            };
            let declaration = &self.checked.tree[declaration_id];
            let TermDeclarationKind::Value(_) = &declaration.kind else {
                continue;
            };
            let Some(declaration) = self.value_declaration(term_id, name)? else {
                continue;
            };
            declarations.push(declaration);
        }

        let mut declarations = declarations.into_iter();
        if let Some(first) = declarations.next() {
            Ok(declarations.fold(first, |document, declaration| {
                document
                    .append(self.arena.hardline())
                    .append(self.arena.hardline())
                    .append(declaration)
            }))
        } else {
            Ok(self.arena.nil())
        }
    }

    fn data_declaration(
        &mut self,
        type_id: indexing::TypeItemId,
        keyword: &str,
        name: &str,
    ) -> Doc<'arena> {
        let declaration_id = self
            .checked
            .tree
            .lookup_type_declaration(type_id)
            .expect("invariant violated: missing checked type declaration");
        let declaration = &self.checked.tree[declaration_id];
        let data = match &declaration.declaration {
            TypeDeclarationKind::Data(data) | TypeDeclarationKind::Newtype(data) => data,
        };

        let mut parameter_names = vec![];
        for &parameter in data.parameters.iter() {
            let parameter = self.queries.lookup_forall_binder(parameter);
            let name = self.type_pretty.display_name(parameter.name);
            parameter_names.push((name.to_string(), parameter.visible));
        }

        let mut head = self.arena.text(format!("{keyword} {name}"));
        for (parameter, visible) in &parameter_names {
            let parameter = if *visible { format!("@{parameter}") } else { parameter.to_string() };
            head = head.append(self.arena.text(format!(" {parameter}")));
        }

        let mut declaration = head;
        let constructors = self.indexed.data_constructors(type_id);
        for (&declaration_id, constructor_id) in data.constructors.iter().zip(constructors) {
            let TermItem { name: constructor_name, .. } = &self.indexed.items[constructor_id];
            let Some(constructor_name) = constructor_name else { continue };
            let constructor = &self.checked.tree[declaration_id];
            let TermDeclarationKind::Constructor(constructor) = &constructor.kind else {
                unreachable!("invariant violated: data declaration contains a value declaration");
            };

            let mut result = self.arena.text(name.to_string());
            for (parameter, _) in &parameter_names {
                result = result.append(self.arena.text(format!(" {parameter}")));
            }

            let mut constructor_type = result;
            for &argument_id in constructor.arguments.iter().rev() {
                let argument = self.type_pretty.render(argument_id);
                let argument = match self.queries.lookup_type(argument_id) {
                    Type::Forall(..)
                    | Type::Constrained(..)
                    | Type::Function(..)
                    | Type::Kinded(..) => format!("({argument})"),
                    _ => argument.to_string(),
                };

                let result = constructor_type;
                constructor_type = self
                    .arena
                    .text(argument)
                    .append(self.arena.text(" ->"))
                    .append(self.arena.line().append(result).nest(2))
                    .group();
            }

            let constructor_type = self.arena.line().append(constructor_type).nest(4);
            let constructor = self
                .arena
                .text(format!("  | {constructor_name} ::"))
                .append(constructor_type)
                .group();
            declaration = declaration.append(self.arena.hardline()).append(constructor);
        }

        declaration
    }

    fn value_declaration(
        &mut self,
        term_id: indexing::TermItemId,
        name: &str,
    ) -> QueryResult<Option<Doc<'arena>>> {
        let declaration_id = self
            .checked
            .tree
            .lookup_term(term_id)
            .expect("invariant violated: missing checked term declaration");
        let declaration = &self.checked.tree[declaration_id];
        let TermDeclarationKind::Value(value) = &declaration.kind else {
            unreachable!("invariant violated: term declaration is not a value");
        };

        let mut type_pretty = TypePretty::new(self.queries, self.checked)
            .without_rigid_kinds()
            .without_forall_kinds()
            .width(self.width);
        let type_id = type_pretty.render(declaration.type_id);
        let signature = self.arena.text(format!("{name} :: {type_id}"));

        let mut equations = vec![];
        for equation in value.equations.iter() {
            let mut expression = self.guarded_expression(&equation.guarded_expression)?;

            for &binder in equation.binders.iter().rev() {
                let binder = self.binder(binder)?;
                expression = self
                    .arena
                    .text("\\")
                    .append(binder)
                    .append(self.arena.text(" ->"))
                    .append(self.arena.line().append(expression).nest(2))
                    .group();
            }
            for evidence in value.evidences.iter().rev() {
                let binder = match evidence {
                    Evidence::Trivial => "{trivial}",
                    _ => return Ok(None),
                };
                expression = self
                    .arena
                    .text(format!("\\{binder} ->"))
                    .append(self.arena.line().append(expression).nest(2))
                    .group();
            }

            let equation = self
                .arena
                .text(format!("{name} ="))
                .append(self.arena.line().append(expression).nest(2))
                .group();
            equations.push(equation);
        }

        let mut equations = equations.into_iter();
        let Some(first) = equations.next() else { return Ok(None) };
        let equations = equations.fold(first, |document, equation| {
            document.append(self.arena.hardline()).append(equation)
        });

        Ok(Some(signature.append(self.arena.hardline()).append(equations)))
    }

    fn guarded_expression(&self, guarded: &GuardedExpression) -> QueryResult<Doc<'arena>> {
        if let [alternative] = guarded.alternatives.as_ref()
            && alternative.pattern_guards.is_empty()
        {
            return self.expression(alternative.where_expression.expression);
        }

        let mut alternatives = vec![];
        for alternative in guarded.alternatives.iter() {
            alternatives.push(self.guarded_alternative(alternative)?);
        }

        let mut alternatives = alternatives.into_iter();
        let Some(first) = alternatives.next() else {
            return Ok(self.arena.text("<error>"));
        };
        let alternatives = alternatives.fold(first, |document, alternative| {
            document.append(self.arena.hardline()).append(alternative)
        });
        Ok(alternatives)
    }

    fn guarded_alternative(&self, alternative: &GuardedAlternative) -> QueryResult<Doc<'arena>> {
        let mut pattern_guards = vec![];
        for pattern_guard in alternative.pattern_guards.iter() {
            pattern_guards.push(self.pattern_guard(pattern_guard)?);
        }

        let expression = self.expression(alternative.where_expression.expression)?;
        let mut pattern_guards = pattern_guards.into_iter();
        let Some(first) = pattern_guards.next() else {
            return Ok(self.arena.text("| -> ").append(expression));
        };
        let pattern_guards = pattern_guards.fold(first, |document, pattern_guard| {
            document.append(self.arena.text(", ")).append(pattern_guard)
        });
        let alternative = self
            .arena
            .text("| ")
            .append(pattern_guards)
            .append(self.arena.text(" -> "))
            .append(expression);
        Ok(alternative)
    }

    fn pattern_guard(&self, pattern_guard: &PatternGuard) -> QueryResult<Doc<'arena>> {
        match *pattern_guard {
            PatternGuard::Boolean { expression } => self.expression(expression),
            PatternGuard::Pattern { binder, expression } => {
                let binder = self.binder(binder)?;
                let expression = self.expression(expression)?;
                Ok(binder.append(self.arena.text(" <- ")).append(expression))
            }
        }
    }

    fn binder(&self, binder_id: BinderId) -> QueryResult<Doc<'arena>> {
        let binder = &self.checked.tree[binder_id];
        match &binder.kind {
            BinderKind::Variable => {
                let kind = self
                    .lowered
                    .info
                    .get_binder_kind(binder.source)
                    .expect("invariant violated: semantic variable binder has no source");
                let lowering::BinderKind::Variable { variable: Some(variable) } = kind else {
                    unreachable!("invariant violated: semantic variable binder has invalid source");
                };
                Ok(self.arena.text(variable.to_string()))
            }
            BinderKind::Constructor { resolution, arguments } => {
                let name = self.term_name(resolution.0, resolution.1)?;
                let name = name.unwrap_or_else(|| "?".to_string());
                let mut constructor = self.arena.text(name);
                if arguments.is_empty() {
                    return Ok(constructor);
                }
                for &argument in arguments.iter() {
                    let argument = self.binder(argument)?;
                    constructor = constructor.append(self.arena.space()).append(argument);
                }
                Ok(self.arena.text("(").append(constructor).append(self.arena.text(")")))
            }
        }
    }

    fn expression(&self, expression_id: ExpressionId) -> QueryResult<Doc<'arena>> {
        let expression = &self.checked.tree[expression_id];
        match expression.kind {
            ExpressionKind::Variable { resolution } => {
                let name = match resolution {
                    TermVariableResolution::Binder(binder) => {
                        let kind =
                            self.lowered.info.get_binder_kind(binder).expect(
                                "invariant violated: variable expression binder is missing",
                            );
                        let lowering::BinderKind::Variable { variable: Some(variable) } = kind
                        else {
                            unreachable!(
                                "invariant violated: variable expression resolves to invalid binder"
                            );
                        };
                        variable.to_string()
                    }
                    TermVariableResolution::Reference(file_id, term_id) => {
                        self.term_name(file_id, term_id)?.unwrap_or_else(|| "?".to_string())
                    }
                    TermVariableResolution::Let(_) | TermVariableResolution::RecordPun(_) => {
                        unreachable!("invariant violated: unsupported semantic variable resolution")
                    }
                };
                Ok(self.arena.text(name))
            }
        }
    }

    fn term_name(
        &self,
        file_id: FileId,
        term_id: indexing::TermItemId,
    ) -> QueryResult<Option<String>> {
        if file_id == self.file_id {
            return Ok(self.indexed.items[term_id].name.as_ref().map(ToString::to_string));
        }

        let indexed = self.queries.indexed(file_id)?;
        Ok(indexed.items[term_id].name.as_ref().map(ToString::to_string))
    }
}
