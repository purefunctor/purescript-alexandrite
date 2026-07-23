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
    BinderId, BinderKind, Equation, ExpressionId, ExpressionKind, GuardedAlternative,
    GuardedExpression, InstanceDeclaration, PatternGuard, RecordBinderField, TermDeclarationKind,
    TypeDeclarationKind,
};

type Doc<'a> = DocBuilder<'a, Arena<'a>, ()>;

fn character_literal(value: char) -> String {
    match value {
        '\n' => "'\\n'".to_string(),
        '\r' => "'\\r'".to_string(),
        '\t' => "'\\t'".to_string(),
        '\\' => "'\\\\'".to_string(),
        '\'' => "'\\\''".to_string(),
        '\0' => "'\\0'".to_string(),
        value if value.is_control() => format!("'\\x{:02x}'", value as u32),
        value => format!("'{value}'"),
    }
}

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
            let (keyword, is_class) = match &declaration.declaration {
                TypeDeclarationKind::Data(_) => ("data", false),
                TypeDeclarationKind::Newtype(_) => ("newtype", false),
                TypeDeclarationKind::Class(_) => ("interface", true),
            };
            let kind = declaration.kind;

            self.type_pretty.reset();
            let signature = self.type_pretty.render_kind_signature(name, kind);
            let signature = self.arena.text(format!("{keyword} {signature}"));

            let declaration = if is_class {
                self.class_declaration(type_id, name)?
            } else {
                self.data_declaration(type_id, keyword, name)
            };

            declarations.push(signature.append(self.arena.hardline()).append(declaration));
        }

        let mut term_names = PrettyNames::new();
        for (_, TermItem { name, .. }) in self.indexed.items.iter_terms() {
            if let Some(name) = name {
                term_names.allocate_display_name(SmolStr::clone(name));
            }
        }

        for (term_id, TermItem { name, .. }) in self.indexed.items.iter_terms() {
            let Some(declaration_id) = self.checked.tree.lookup_term(term_id) else {
                continue;
            };
            let declaration = &self.checked.tree[declaration_id];
            match &declaration.kind {
                TermDeclarationKind::Value(_) => {
                    let Some(name) = name else { continue };
                    let Some(declaration) = self.value_declaration(term_id, name)? else {
                        continue;
                    };
                    declarations.push(declaration);
                }
                TermDeclarationKind::Constructor(_) => {}
                TermDeclarationKind::Instance(instance) => {
                    let name = if let Some(name) = name {
                        SmolStr::clone(name)
                    } else {
                        let base = self.dictionary_base_name(declaration.type_id, instance)?;
                        term_names.allocate_display_name(base)
                    };
                    let declaration = self.instance_declaration(term_id, &name)?;
                    declarations.push(declaration);
                }
            }
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
            TypeDeclarationKind::Class(_) => {
                unreachable!("invariant violated: class is not a data declaration")
            }
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

    fn class_declaration(
        &mut self,
        type_id: indexing::TypeItemId,
        name: &str,
    ) -> QueryResult<Doc<'arena>> {
        let declaration_id = self
            .checked
            .tree
            .lookup_type_declaration(type_id)
            .expect("invariant violated: missing checked type declaration");
        let declaration = &self.checked.tree[declaration_id];
        let TypeDeclarationKind::Class(class) = &declaration.declaration else {
            unreachable!("invariant violated: type declaration is not a class");
        };

        for &parameter in class.kind_binders.iter() {
            let parameter = self.queries.lookup_forall_binder(parameter);
            self.type_pretty.display_name(parameter.name);
        }

        let mut head = self.arena.text(format!("interface {name}"));
        for &parameter in class.type_parameters.iter() {
            let parameter = self.queries.lookup_forall_binder(parameter);
            let parameter = self.type_pretty.display_name(parameter.name);
            head = head.append(self.arena.text(format!(" {parameter}")));
        }
        let mut declaration = head.append(self.arena.text(" where"));

        let mut field_names = PrettyNames::new();
        for member_id in self.indexed.class_members(type_id) {
            let TermItem { name: Some(name), .. } = &self.indexed.items[member_id] else {
                continue;
            };
            field_names.allocate_display_name(SmolStr::clone(name));
        }

        for superclass in class.superclasses.iter() {
            let base = self.evidence_base_name(superclass.constraint)?;
            let field_name = field_names.allocate_display_name(base);
            let mut type_pretty =
                TypePretty::new(self.queries, self.checked).without_rigid_kinds().width(self.width);
            let field_type = type_pretty.render(superclass.constraint);
            let field = self.arena.text(format!("  superclass {field_name} :: {field_type}"));
            declaration = declaration.append(self.arena.hardline()).append(field);
        }

        for member in class.members.iter() {
            let TermItem { name: Some(name), .. } = &self.indexed.items[member.source] else {
                continue;
            };
            let mut type_pretty =
                TypePretty::new(self.queries, self.checked).without_rigid_kinds().width(self.width);
            let field_type = type_pretty.render(member.field_type);
            let field = self.arena.text(format!("  {name} :: {field_type}"));
            declaration = declaration.append(self.arena.hardline()).append(field);
        }

        Ok(declaration)
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

        let Some(equations) = self.equation_declarations(
            name,
            "",
            &value.evidences,
            &value.equations,
            &mut evidence_names,
            &[],
        )?
        else {
            return Ok(None);
        };

        Ok(Some(signature.append(self.arena.hardline()).append(equations)))
    }

    fn instance_declaration(
        &mut self,
        term_id: indexing::TermItemId,
        name: &str,
    ) -> QueryResult<Doc<'arena>> {
        let declaration_id = self
            .checked
            .tree
            .lookup_term(term_id)
            .expect("invariant violated: missing checked instance declaration");
        let declaration = &self.checked.tree[declaration_id];
        let TermDeclarationKind::Instance(instance) = &declaration.kind else {
            unreachable!("invariant violated: term declaration is not an instance");
        };

        let mut outer_evidence_names = EvidenceNames::new();
        for evidence in instance.evidences.iter() {
            if let Evidence::Given(binder) = &evidence.evidence {
                self.evidence_binder_name(&mut outer_evidence_names, *binder)?;
            }
        }

        let (signature, rigid_names) = self.dictionary_signature(
            name,
            declaration.type_id,
            instance,
            &mut outer_evidence_names,
        )?;
        let mut declaration =
            signature.append(self.arena.hardline()).append(self.arena.text("where"));

        let superclass_names = self.instance_superclass_field_names(instance)?;
        for (superclass, field_name) in instance.superclasses.iter().zip(superclass_names) {
            let evidence =
                self.evidence_variable_name(&mut outer_evidence_names, superclass.evidence)?;
            let field = self.arena.text(format!("  superclass {field_name} = {evidence}"));
            declaration = declaration.append(self.arena.hardline()).append(field);
        }

        for member in instance.members.iter() {
            let Some(member_name) = self.term_name(member.resolution.0, member.resolution.1)?
            else {
                continue;
            };

            let mut evidence_names = EvidenceNames::new();
            let instance_evidences = instance.evidences.iter().map(|evidence| &evidence.evidence);
            for evidence in instance_evidences.chain(member.evidences.iter()) {
                if let Evidence::Given(binder) = evidence {
                    self.evidence_binder_name(&mut evidence_names, *binder)?;
                }
            }

            let Some(equations) = self.equation_declarations(
                &member_name,
                "  ",
                &member.evidences,
                &member.equations,
                &mut evidence_names,
                &rigid_names,
            )?
            else {
                continue;
            };
            declaration = declaration.append(self.arena.hardline()).append(equations);
        }

        Ok(declaration)
    }

    fn dictionary_signature(
        &self,
        name: &str,
        type_id: crate::TypeId,
        instance: &InstanceDeclaration,
        evidence_names: &mut EvidenceNames,
    ) -> QueryResult<(Doc<'arena>, Vec<(crate::TypeId, SmolStr)>)> {
        let mut binders = vec![];
        let mut current = type_id;
        while let Type::Forall(binder, inner) = self.queries.lookup_type(current) {
            binders.push(binder);
            current = inner;
        }
        debug_assert_eq!(binders.len(), instance.rigid_parameters.len());

        while let Type::Constrained(_, inner) = self.queries.lookup_type(current) {
            current = inner;
        }

        let mut type_pretty =
            TypePretty::new(self.queries, self.checked).without_rigid_kinds().width(self.width);
        let binder_names = binders.iter().map(|&binder| {
            let binder = self.queries.lookup_forall_binder(binder);
            type_pretty.display_name(binder.name)
        });
        let binder_names = binder_names.collect::<Vec<_>>();
        let rigid_names =
            instance.rigid_parameters.iter().copied().zip(binder_names.iter().cloned());
        let rigid_names = rigid_names.collect::<Vec<_>>();

        let mut lines = vec![];
        if !binder_names.is_empty() {
            lines.push(format!("forall {}.", binder_names.join(" ")));
        }

        let mut fields = vec![];
        for evidence in instance.evidences.iter() {
            let field_name = self.evidence_name(evidence_names, &evidence.evidence)?;
            let constraint = type_pretty.render(evidence.constraint);
            fields.push((field_name, constraint));
        }
        if let [(field_name, constraint)] = fields.as_slice() {
            lines.push(format!("{{ {field_name} :: {constraint} }} =>"));
        } else if let Some((first_name, first_constraint)) = fields.first() {
            lines.push(format!("{{ {first_name} :: {first_constraint}"));
            for (field_name, constraint) in fields.iter().skip(1) {
                lines.push(format!(", {field_name} :: {constraint}"));
            }
            lines.push("} =>".to_string());
        }

        lines.push(type_pretty.render(current).to_string());

        let mut signature = self.arena.text(format!("dictionary {name} ::"));
        if lines.len() == 1 {
            signature = signature.append(self.arena.text(format!(" {}", lines[0])));
        } else {
            for line in lines {
                signature = signature
                    .append(self.arena.hardline())
                    .append(self.arena.text(format!("  {line}")));
            }
        }
        Ok((signature, rigid_names))
    }

    fn instance_superclass_field_names(
        &self,
        instance: &InstanceDeclaration,
    ) -> QueryResult<Vec<SmolStr>> {
        let indexed = if instance.class.0 == self.file_id {
            None
        } else {
            Some(self.queries.indexed(instance.class.0)?)
        };
        let indexed = indexed.as_deref().unwrap_or(self.indexed);

        let mut field_names = PrettyNames::new();
        for member_id in indexed.class_members(instance.class.1) {
            if let Some(name) = &indexed.items[member_id].name {
                field_names.allocate_display_name(SmolStr::clone(name));
            }
        }

        let mut superclasses = vec![];
        for superclass in instance.superclasses.iter() {
            let base = self.evidence_base_name(superclass.constraint)?;
            superclasses.push(field_names.allocate_display_name(base));
        }
        Ok(superclasses)
    }

    fn dictionary_base_name(
        &self,
        type_id: crate::TypeId,
        instance: &InstanceDeclaration,
    ) -> QueryResult<SmolStr> {
        let class_name = self.type_name(instance.class.0, instance.class.1)?;
        let Some(class_name) = class_name else {
            return Ok(SmolStr::new("dictionary"));
        };
        let mut characters = class_name.chars();
        let Some(first) = characters.next() else {
            return Ok(SmolStr::new("dictionary"));
        };
        let first = first.to_lowercase().collect::<String>();
        let mut base = format!("{first}{}", characters.as_str());

        let mut current = type_id;
        loop {
            match self.queries.lookup_type(current) {
                Type::Forall(_, inner) | Type::Constrained(_, inner) | Type::Kinded(inner, _) => {
                    current = inner;
                }
                Type::Application(_, argument) => {
                    if let Some(argument_name) = self.outer_type_constructor_name(argument)? {
                        base.push_str(&argument_name);
                    }
                    break;
                }
                Type::KindApplication(function, _) => current = function,
                _ => break,
            }
        }

        Ok(SmolStr::new(base))
    }

    fn outer_type_constructor_name(
        &self,
        mut type_id: crate::TypeId,
    ) -> QueryResult<Option<String>> {
        loop {
            match self.queries.lookup_type(type_id) {
                Type::Application(function, _)
                | Type::KindApplication(function, _)
                | Type::Kinded(function, _) => type_id = function,
                Type::Constructor(file_id, item_id) => {
                    return self.type_name(file_id, item_id);
                }
                _ => return Ok(None),
            }
        }
    }

    fn equation_declarations(
        &self,
        name: &str,
        prefix: &str,
        evidences: &[Evidence],
        equations: &[Equation],
        evidence_names: &mut EvidenceNames,
        rigid_names: &[(crate::TypeId, SmolStr)],
    ) -> QueryResult<Option<Doc<'arena>>> {
        let mut rendered_equations = vec![];
        let mut type_pretty =
            TypePretty::new(self.queries, self.checked).without_rigid_kinds().width(self.width);
        for (rigid, display) in rigid_names {
            if let Type::Rigid(name, _, _) = self.queries.lookup_type(*rigid) {
                type_pretty.assign_display_name(name, SmolStr::clone(display));
            }
        }
        for equation in equations.iter() {
            let mut expression = self.guarded_expression(
                &equation.guarded_expression,
                evidence_names,
                &mut type_pretty,
            )?;

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
            for evidence in evidences.iter().rev() {
                let binder = self.evidence_name(evidence_names, evidence)?;
                expression = self
                    .arena
                    .text(format!("\\{{{binder}}} ->"))
                    .append(self.arena.line().append(expression).nest(2))
                    .group();
            }

            let equation = self
                .arena
                .text(format!("{prefix}{name} ="))
                .append(self.arena.line().append(expression).nest(2))
                .group();
            rendered_equations.push(equation);
        }

        let mut equations = rendered_equations.into_iter();
        let Some(first) = equations.next() else { return Ok(None) };
        let equations = equations.fold(first, |document, equation| {
            document.append(self.arena.hardline()).append(equation)
        });
        Ok(Some(equations))
    }

    fn guarded_expression(
        &self,
        guarded: &GuardedExpression,
        evidence_names: &mut EvidenceNames,
        type_pretty: &mut TypePretty<'context, Q>,
    ) -> QueryResult<Doc<'arena>> {
        if let [alternative] = guarded.alternatives.as_ref()
            && alternative.pattern_guards.is_empty()
        {
            return self.expression(
                alternative.where_expression.expression,
                evidence_names,
                type_pretty,
            );
        }

        let mut alternatives = vec![];
        for alternative in guarded.alternatives.iter() {
            alternatives.push(self.guarded_alternative(
                alternative,
                evidence_names,
                type_pretty,
            )?);
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
        type_pretty: &mut TypePretty<'context, Q>,
    ) -> QueryResult<Doc<'arena>> {
        let mut pattern_guards = vec![];
        for pattern_guard in alternative.pattern_guards.iter() {
            pattern_guards.push(self.pattern_guard(pattern_guard, evidence_names, type_pretty)?);
        }

        let expression =
            self.expression(alternative.where_expression.expression, evidence_names, type_pretty)?;
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
        type_pretty: &mut TypePretty<'context, Q>,
    ) -> QueryResult<Doc<'arena>> {
        match *pattern_guard {
            PatternGuard::Boolean { expression } => {
                self.expression(expression, evidence_names, type_pretty)
            }
            PatternGuard::Pattern { binder, expression } => {
                let binder = self.binder(binder)?;
                let expression = self.expression(expression, evidence_names, type_pretty)?;
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
            BinderKind::String { value } => {
                let text = format!("{:?}", value.as_str());
                Ok(self.arena.text(text))
            }
            BinderKind::Char { value } => {
                let text = character_literal(*value);
                Ok(self.arena.text(text))
            }
            BinderKind::Boolean { value } => {
                let text = if *value { "true" } else { "false" };
                Ok(self.arena.text(text))
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
        type_pretty: &mut TypePretty<'context, Q>,
    ) -> QueryResult<Doc<'arena>> {
        let expression = &self.checked.tree[expression_id];
        match &expression.kind {
            ExpressionKind::Error => Ok(self.arena.text("<error>")),
            ExpressionKind::String { kind, value } => match kind {
                lowering::StringKind::String => {
                    let text = format!("\"{value}\"");
                    Ok(self.arena.text(text))
                }
                lowering::StringKind::RawString => {
                    let text = format!("\"\"\"{value}\"\"\"");
                    Ok(self.arena.text(text))
                }
            },
            ExpressionKind::Char { value } => {
                let text = character_literal(*value);
                Ok(self.arena.text(text))
            }
            ExpressionKind::Boolean { value } => {
                let text = if *value { "true" } else { "false" };
                Ok(self.arena.text(text))
            }
            ExpressionKind::Integer { value } => {
                let text = value.to_string();
                Ok(self.arena.text(text))
            }
            ExpressionKind::Number { value } => {
                let text = value.to_string();
                Ok(self.arena.text(text))
            }
            ExpressionKind::Constructor { resolution } => {
                let name = self.term_name(resolution.0, resolution.1)?;
                let text = name.unwrap_or_else(|| "?".to_string());
                Ok(self.arena.text(text))
            }
            ExpressionKind::Variable { resolution } => {
                let name = match *resolution {
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
                let function = self.expression(*function, evidence_names, type_pretty)?;
                let argument_expression = &self.checked.tree[*argument];
                let argument = self.expression(*argument, evidence_names, type_pretty)?;
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
                let function = self.expression(*function, evidence_names, type_pretty)?;
                let argument = type_pretty.render(*argument);
                Ok(function.append(self.arena.text(format!(" @{argument}"))))
            }
            ExpressionKind::EvidenceApplication { function, evidence } => {
                let function = self.expression(*function, evidence_names, type_pretty)?;
                let evidence = self.evidence_variable_name(evidence_names, *evidence)?;
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
