//! Implements the pretty printer for the checked semantic tree.

use building_types::QueryResult;
use files::FileId;
use indexing::{TermItem, TypeItem};
use lowering::TermVariableResolution;
use pretty::{Arena, DocAllocator, DocBuilder};
use rustc_hash::FxHashMap;
use smol_str::{SmolStr, SmolStrBuilder};

use crate::CheckedModule;
use crate::core::Type;
use crate::core::pretty::{Pretty as TypePretty, PrettyNames, PrettyQueries};
use crate::evidence::{Evidence, EvidenceBinderId, EvidenceState, EvidenceVarId};
use crate::tree::{
    BinderId, BinderKind, ExpressionId, ExpressionKind, GuardedAlternative, GuardedExpression,
    PatternGuard, RecordBinderField, TermDeclarationKind, TypeDeclarationKind,
};

type Doc<'a> = DocBuilder<'a, Arena<'a>, ()>;

struct EvidenceNames {
    display_by_binder: FxHashMap<EvidenceBinderId, SmolStr>,
    names: PrettyNames,
}

impl EvidenceNames {
    fn new() -> EvidenceNames {
        EvidenceNames { display_by_binder: FxHashMap::default(), names: PrettyNames::new() }
    }
}

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

        let mut evidence_names = EvidenceNames::new();
        for evidence in value.evidences.iter() {
            if let Evidence::Given(binder) = evidence {
                self.evidence_binder_name(&mut evidence_names, *binder)?;
            }
        }

        let mut equations = vec![];
        for equation in value.equations.iter() {
            let mut expression =
                self.guarded_expression(&equation.guarded_expression, &mut evidence_names)?;

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
                let binder = self.evidence_name(&mut evidence_names, evidence)?;
                expression = self
                    .arena
                    .text(format!("\\{{{binder}}} ->"))
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

    fn guarded_expression(
        &self,
        guarded: &GuardedExpression,
        evidence_names: &mut EvidenceNames,
    ) -> QueryResult<Doc<'arena>> {
        if let [alternative] = guarded.alternatives.as_ref()
            && alternative.pattern_guards.is_empty()
        {
            return self.expression(alternative.where_expression.expression, evidence_names);
        }

        let mut alternatives = vec![];
        for alternative in guarded.alternatives.iter() {
            alternatives.push(self.guarded_alternative(alternative, evidence_names)?);
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

    fn guarded_alternative(
        &self,
        alternative: &GuardedAlternative,
        evidence_names: &mut EvidenceNames,
    ) -> QueryResult<Doc<'arena>> {
        let mut pattern_guards = vec![];
        for pattern_guard in alternative.pattern_guards.iter() {
            pattern_guards.push(self.pattern_guard(pattern_guard, evidence_names)?);
        }

        let expression =
            self.expression(alternative.where_expression.expression, evidence_names)?;
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

    fn pattern_guard(
        &self,
        pattern_guard: &PatternGuard,
        evidence_names: &mut EvidenceNames,
    ) -> QueryResult<Doc<'arena>> {
        match *pattern_guard {
            PatternGuard::Boolean { expression } => self.expression(expression, evidence_names),
            PatternGuard::Pattern { binder, expression } => {
                let binder = self.binder(binder)?;
                let expression = self.expression(expression, evidence_names)?;
                Ok(binder.append(self.arena.text(" <- ")).append(expression))
            }
        }
    }

    fn binder(&self, binder_id: BinderId) -> QueryResult<Doc<'arena>> {
        let binder = &self.checked.tree[binder_id];
        match &binder.kind {
            BinderKind::Error => Ok(self.arena.text("<error>")),
            BinderKind::Typed { binder, annotation } => {
                let binder = self.binder(*binder)?;
                let mut type_pretty = TypePretty::new(self.queries, self.checked)
                    .without_rigid_kinds()
                    .width(self.width);
                let annotation = type_pretty.render(*annotation);
                Ok(self
                    .arena
                    .text("(")
                    .append(binder)
                    .append(self.arena.text(format!(" :: {annotation})"))))
            }
            BinderKind::Integer { value } => {
                let value =
                    if value.is_negative() { format!("({value})") } else { value.to_string() };
                Ok(self.arena.text(value))
            }
            BinderKind::Number { negative, value } => {
                let value = if *negative { format!("(-{value})") } else { value.to_string() };
                Ok(self.arena.text(value))
            }
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
            BinderKind::Named { name, binder } => {
                let binder = self.binder(*binder)?;
                Ok(self.arena.text(format!("{name}@")).append(binder))
            }
            BinderKind::Wildcard => Ok(self.arena.text("_")),
            BinderKind::String { value } => Ok(self.arena.text(format!("{:?}", value.as_str()))),
            BinderKind::Char { value } => Ok(self.arena.text(format!("{value:?}"))),
            BinderKind::Boolean { value } => {
                Ok(self.arena.text(if *value { "true" } else { "false" }))
            }
            BinderKind::Array { elements } => {
                let mut array = self.arena.text("[");
                for (position, &element) in elements.iter().enumerate() {
                    if position > 0 {
                        array = array.append(self.arena.text(", "));
                    }
                    array = array.append(self.binder(element)?);
                }
                Ok(array.append(self.arena.text("]")))
            }
            BinderKind::Record { fields } => {
                let mut record = self.arena.text("{ ");
                for (position, field) in fields.iter().enumerate() {
                    if position > 0 {
                        record = record.append(self.arena.text(", "));
                    }
                    match field {
                        RecordBinderField::Field { label, binder } => {
                            let binder = self.binder(*binder)?;
                            record =
                                record.append(self.arena.text(format!("{label}: "))).append(binder);
                        }
                        RecordBinderField::Pun { label } => {
                            record = record.append(self.arena.text(label.to_string()));
                        }
                    }
                }
                Ok(record.append(self.arena.text(" }")))
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

    fn expression(
        &self,
        expression_id: ExpressionId,
        evidence_names: &mut EvidenceNames,
    ) -> QueryResult<Doc<'arena>> {
        let expression = &self.checked.tree[expression_id];
        match expression.kind {
            ExpressionKind::Error => Ok(self.arena.text("<error>")),
            ExpressionKind::Constructor { resolution } => {
                let name = self.term_name(resolution.0, resolution.1)?;
                Ok(self.arena.text(name.unwrap_or_else(|| "?".to_string())))
            }
            ExpressionKind::Variable { resolution } => {
                let name = match resolution {
                    TermVariableResolution::Binder(binder) => {
                        let kind =
                            self.lowered.info.get_binder_kind(binder).expect(
                                "invariant violated: variable expression binder is missing",
                            );
                        match kind {
                            lowering::BinderKind::Variable { variable: Some(variable) } => {
                                variable.to_string()
                            }
                            lowering::BinderKind::Named { named: Some(named), .. } => {
                                named.to_string()
                            }
                            _ => {
                                let index = binder.into_raw().get();
                                format!("<binder#{index}>")
                            }
                        }
                    }
                    TermVariableResolution::Reference(file_id, term_id) => {
                        self.term_name(file_id, term_id)?.unwrap_or_else(|| "?".to_string())
                    }
                    TermVariableResolution::Let(let_binding) => {
                        let index = let_binding.into_raw().into_u32();
                        format!("<let#{index}>")
                    }
                    TermVariableResolution::RecordPun(record_pun) => {
                        self.record_pun_name(record_pun).unwrap_or_else(|| {
                            let index = record_pun.into_raw().get();
                            format!("<pun#{index}>")
                        })
                    }
                };
                Ok(self.arena.text(name))
            }
            ExpressionKind::TermApplication { function, argument } => {
                let function = self.expression(function, evidence_names)?;
                let argument_expression = &self.checked.tree[argument];
                let argument = self.expression(argument, evidence_names)?;
                let argument = match argument_expression.kind {
                    ExpressionKind::TermApplication { .. }
                    | ExpressionKind::TypeApplication { .. }
                    | ExpressionKind::EvidenceApplication { .. } => {
                        self.arena.text("(").append(argument).append(self.arena.text(")"))
                    }
                    _ => argument,
                };
                Ok(function.append(self.arena.space()).append(argument))
            }
            ExpressionKind::TypeApplication { function, argument } => {
                let function = self.expression(function, evidence_names)?;
                let mut type_pretty = TypePretty::new(self.queries, self.checked)
                    .without_rigid_kinds()
                    .width(self.width);
                let argument = type_pretty.render(argument);
                Ok(function.append(self.arena.text(format!(" @{argument}"))))
            }
            ExpressionKind::EvidenceApplication { function, evidence } => {
                let function = self.expression(function, evidence_names)?;
                let evidence = self.evidence_variable_name(evidence_names, evidence)?;
                Ok(function.append(self.arena.text(format!(" {{{evidence}}}"))))
            }
        }
    }

    fn evidence_variable_name(
        &self,
        names: &mut EvidenceNames,
        evidence: EvidenceVarId,
    ) -> QueryResult<SmolStr> {
        match self.checked.evidence[evidence].state {
            EvidenceState::Solved(proof) => {
                self.evidence_name(names, &self.checked.evidence[proof])
            }
            EvidenceState::Unsolved => Ok(SmolStr::new("unsolved")),
            EvidenceState::Error => Ok(SmolStr::new("error")),
        }
    }

    fn evidence_name(
        &self,
        names: &mut EvidenceNames,
        evidence: &Evidence,
    ) -> QueryResult<SmolStr> {
        match evidence {
            Evidence::Variable(evidence) => self.evidence_variable_name(names, *evidence),
            Evidence::Given(binder) => self.evidence_binder_name(names, *binder),
            Evidence::Instance { .. } => Ok(SmolStr::new("instance")),
            Evidence::Superclass { .. } => Ok(SmolStr::new("superclass")),
            Evidence::Trivial => Ok(SmolStr::new("trivial")),
            Evidence::Synthesized(_) => Ok(SmolStr::new("synthesized")),
        }
    }

    fn evidence_binder_name(
        &self,
        names: &mut EvidenceNames,
        binder: EvidenceBinderId,
    ) -> QueryResult<SmolStr> {
        if let Some(display) = names.display_by_binder.get(&binder) {
            return Ok(SmolStr::clone(display));
        }

        let constraint = self.checked.evidence[binder].constraint;
        let base = self.evidence_base_name(constraint)?;
        let display = names.names.allocate_display_name(base);
        names.display_by_binder.insert(binder, SmolStr::clone(&display));
        Ok(display)
    }

    fn evidence_base_name(&self, mut constraint: crate::TypeId) -> QueryResult<SmolStr> {
        let class_name = loop {
            match self.queries.lookup_type(constraint) {
                Type::Application(function, _)
                | Type::KindApplication(function, _)
                | Type::Kinded(function, _) => constraint = function,
                Type::Constructor(file_id, type_id) => {
                    break self.type_name(file_id, type_id)?;
                }
                _ => break None,
            }
        };
        let Some(class_name) = class_name else {
            return Ok(SmolStr::new("evidenceDict"));
        };
        let mut characters = class_name.chars();
        let Some(first) = characters.next() else {
            return Ok(SmolStr::new("evidenceDict"));
        };
        let first = first.to_lowercase().collect::<String>();
        Ok(SmolStr::new(format!("{first}{}Dict", characters.as_str())))
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

    fn record_pun_name(&self, record_pun: lowering::RecordPunId) -> Option<String> {
        self.lowered.info.iter_binder().find_map(|(_, kind)| {
            let lowering::BinderKind::Record { record } = kind else {
                return None;
            };
            record.iter().find_map(|item| {
                let lowering::BinderRecordItem::RecordPun { id, name } = item else {
                    return None;
                };
                if *id == record_pun { name.as_ref().map(ToString::to_string) } else { None }
            })
        })
    }

    fn type_name(
        &self,
        file_id: FileId,
        type_id: indexing::TypeItemId,
    ) -> QueryResult<Option<String>> {
        if file_id == self.file_id {
            return Ok(self.indexed.items[type_id].name.as_ref().map(ToString::to_string));
        }

        let indexed = self.queries.indexed(file_id)?;
        Ok(indexed.items[type_id].name.as_ref().map(ToString::to_string))
    }
}
