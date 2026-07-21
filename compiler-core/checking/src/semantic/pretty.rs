//! Implements the pretty printer for checked Core semantic trees.

use files::FileId;
use indexing::{TermItemId, TermItemKind};
use itertools::Itertools;
use lowering::{BinderKind, GraphNode, LetBindingNameGroupId, TermVariableResolution};
use pretty::{Arena, DocAllocator, DocBuilder};
use smol_str::{SmolStr, SmolStrBuilder};

use crate::core::Name;
use crate::evidence::{
    Evidence, EvidenceBinderId, EvidenceId, EvidenceState, EvidenceVarId, InstanceCandidateOrigin,
    ReflectableEvidence, ReflectableOrdering, SuperclassId, SynthesizedEvidence,
};
use crate::semantic::{
    CheckedAdoExpression, CheckedAdoStep, CheckedBinaryApplication, CheckedBinderId,
    CheckedBinderKind, CheckedBlockStatement, CheckedCaseAlternative, CheckedDoExpression,
    CheckedDoStep, CheckedExpressionId, CheckedExpressionKind, CheckedGuardedExpression,
    CheckedLetBinding, CheckedLetStatement, CheckedLiteral, CheckedPatternGuard,
    CheckedRecordField, CheckedRecordUpdate, CheckedUnaryApplication,
};
use crate::{CheckedModule, PrettyQueries, TypeId};

type Doc<'a> = DocBuilder<'a, Arena<'a>, ()>;

pub struct Pretty<'a, Q: ?Sized> {
    queries: &'a Q,
    checked: &'a CheckedModule,
    lowered: &'a lowering::LoweredModule,
    types: crate::core::pretty::Pretty<'a, Q>,
    width: usize,
}

impl<'a, Q> Pretty<'a, Q>
where
    Q: PrettyQueries + ?Sized,
{
    pub fn new(
        queries: &'a Q,
        checked: &'a CheckedModule,
        lowered: &'a lowering::LoweredModule,
    ) -> Pretty<'a, Q> {
        let types = crate::core::pretty::Pretty::new(queries, checked);
        Pretty { queries, checked, lowered, types, width: 100 }
    }

    pub fn width(mut self, width: usize) -> Pretty<'a, Q> {
        self.width = width;
        self.types = self.types.width(width);
        self
    }

    pub fn reset(&mut self) {
        self.types.reset();
    }

    pub fn display_name(&mut self, name: Name) -> SmolStr {
        self.types.display_name(name)
    }

    pub fn render_type(&mut self, type_id: TypeId) -> SmolStr {
        self.types.render(type_id)
    }

    pub fn render_signature(&mut self, name: &str, type_id: TypeId) -> SmolStr {
        self.types.render_signature(name, type_id)
    }

    pub fn render_expression(&mut self, expression_id: CheckedExpressionId) -> SmolStr {
        let width = self.width;
        let arena = Arena::new();
        let mut printer =
            Printer::new(&arena, self.queries, self.checked, self.lowered, &mut self.types);
        let document = printer.expression(expression_id, Precedence::Abstraction);

        let mut output = SmolStrBuilder::new();
        document
            .render_fmt(width, &mut output)
            .expect("critical failure: failed to render checked expression");
        output.finish()
    }

    pub fn render_binder(&mut self, binder_id: CheckedBinderId) -> SmolStr {
        let width = self.width;
        let arena = Arena::new();
        let printer =
            Printer::new(&arena, self.queries, self.checked, self.lowered, &mut self.types);
        let document = printer.binder(binder_id);

        let mut output = SmolStrBuilder::new();
        document
            .render_fmt(width, &mut output)
            .expect("critical failure: failed to render checked binder");
        output.finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Abstraction,
    Application,
    RecordUpdate,
    Atom,
}

struct Printer<'arena, 'context, 'state, Q>
where
    Q: PrettyQueries + ?Sized,
{
    arena: &'arena Arena<'arena>,
    queries: &'context Q,
    checked: &'context CheckedModule,
    lowered: &'context lowering::LoweredModule,
    types: &'state mut crate::core::pretty::Pretty<'context, Q>,
}

impl<'arena, 'context, 'state, Q> Printer<'arena, 'context, 'state, Q>
where
    Q: PrettyQueries + ?Sized,
{
    fn new(
        arena: &'arena Arena<'arena>,
        queries: &'context Q,
        checked: &'context CheckedModule,
        lowered: &'context lowering::LoweredModule,
        types: &'state mut crate::core::pretty::Pretty<'context, Q>,
    ) -> Printer<'arena, 'context, 'state, Q> {
        Printer { arena, queries, checked, lowered, types }
    }

    fn parenthesize_if(&self, condition: bool, document: Doc<'arena>) -> Doc<'arena> {
        if condition {
            self.arena.text("(").append(document).append(self.arena.text(")"))
        } else {
            document
        }
    }

    fn separated_by_space(&self, documents: Vec<Doc<'arena>>) -> Doc<'arena> {
        let mut documents = documents.into_iter();
        let Some(first) = documents.next() else {
            return self.arena.nil();
        };

        documents.fold(first, |document, next| document.append(self.arena.line()).append(next))
    }

    fn separated_by_comma(&self, documents: Vec<Doc<'arena>>) -> Doc<'arena> {
        let mut documents = documents.into_iter();
        let Some(first) = documents.next() else {
            return self.arena.nil();
        };

        documents.fold(first, |document, next| {
            document.append(self.arena.text(",")).append(self.arena.line()).append(next)
        })
    }

    fn expression(&mut self, expression_id: CheckedExpressionId, outer: Precedence) -> Doc<'arena> {
        let expression = self.checked.core.expressions[expression_id].clone();
        let (document, precedence) = match expression.kind {
            CheckedExpressionKind::Variable { resolution } => {
                (self.arena.text(self.variable(resolution)), Precedence::Atom)
            }
            CheckedExpressionKind::Constructor { file_id, item_id } => {
                let constructor = self.item_name(file_id, item_id);
                (self.arena.text(constructor), Precedence::Atom)
            }
            CheckedExpressionKind::Literal { literal } => {
                (self.arena.text(self.literal(literal)), Precedence::Atom)
            }
            CheckedExpressionKind::Array { elements } => {
                (self.array_expression(&elements), Precedence::Atom)
            }
            CheckedExpressionKind::Record { fields } => {
                (self.record_expression(&fields), Precedence::Atom)
            }
            CheckedExpressionKind::RecordAccess { record, labels } => {
                let record = self.expression(record, Precedence::Atom);
                let labels = labels.iter();
                let labels = labels.fold(record, |record, label| {
                    record.append(self.arena.text(".")).append(self.arena.text(label.to_string()))
                });
                (labels, Precedence::Atom)
            }
            CheckedExpressionKind::RecordUpdate { record, updates } => {
                let record = self.expression(record, Precedence::Atom);
                let updates = self.record_updates(&updates);
                let document = record.append(self.arena.text(" ")).append(updates);
                (document, Precedence::RecordUpdate)
            }
            CheckedExpressionKind::Error => (self.arena.text("<error>"), Precedence::Atom),
            CheckedExpressionKind::Do { expression } => {
                (self.do_expression(&expression), Precedence::Abstraction)
            }
            CheckedExpressionKind::Ado { expression } => {
                (self.ado_expression(&expression), Precedence::Abstraction)
            }
            CheckedExpressionKind::Case { scrutinees, alternatives } => {
                let document = self.case_expression(&scrutinees, &alternatives);
                (document, Precedence::Abstraction)
            }
            CheckedExpressionKind::Lambda { binders, expression } => {
                let binders = binders.iter().map(|binder_id| self.binder(*binder_id));
                let binders = binders.collect_vec();
                let expression = self.expression(expression, Precedence::Abstraction);
                let document = self.lambda_document(binders, expression);
                (document, Precedence::Abstraction)
            }
            CheckedExpressionKind::TermApplication { function, argument } => {
                let function = self.expression(function, Precedence::Application);
                let argument = self.expression(argument, Precedence::RecordUpdate);
                let application = self.term_application(function, argument);
                (application, Precedence::Application)
            }
            CheckedExpressionKind::TypeApplication { function, argument } => {
                let function = self.expression(function, Precedence::Application);
                let argument = self.type_document(argument);
                let argument =
                    self.arena.line().append(self.arena.text("@")).append(argument).nest(2);
                let application = function.append(argument).group();
                (application, Precedence::Application)
            }
            CheckedExpressionKind::EvidenceApplication { expression, evidence } => {
                let expression = self.expression(expression, Precedence::Application);
                let application = self.evidence_application(expression, &evidence);
                (application, Precedence::Application)
            }
            CheckedExpressionKind::EvidenceAbstraction { binders, expression } => {
                let binders = binders.iter().map(|binder_id| self.evidence_binder(*binder_id));
                let binders = binders.collect::<Vec<_>>();
                let binders = self.separated_by_comma(binders).group();
                let expression = self.expression(expression, Precedence::Abstraction);
                let document = self
                    .arena
                    .text("\\@{")
                    .append(binders)
                    .append(self.arena.text("} -> "))
                    .append(expression);
                (document, Precedence::Abstraction)
            }
        };

        self.parenthesize_if(precedence < outer, document)
    }

    fn array_expression(&mut self, elements: &[CheckedExpressionId]) -> Doc<'arena> {
        let elements =
            elements.iter().map(|element| self.expression(*element, Precedence::Abstraction));
        let elements = elements.collect_vec();
        let elements = self.separated_by_comma(elements).group();
        self.arena.text("[").append(elements).append(self.arena.text("]"))
    }

    fn record_expression(&mut self, fields: &[CheckedRecordField]) -> Doc<'arena> {
        if fields.is_empty() {
            return self.arena.text("{}");
        }

        let fields = fields.iter().map(|field| {
            let expression = self.expression(field.expression, Precedence::Abstraction);
            self.arena
                .text(field.label.to_string())
                .append(self.arena.text(": "))
                .append(expression)
        });
        let fields = fields.collect_vec();
        let fields = self.separated_by_comma(fields).group();
        self.arena.text("{ ").append(fields).append(self.arena.text(" }")).group()
    }

    fn record_updates(&mut self, updates: &[CheckedRecordUpdate]) -> Doc<'arena> {
        let updates = updates.iter().map(|update| match update {
            CheckedRecordUpdate::Leaf { label, expression } => {
                let expression = self.expression(*expression, Precedence::Abstraction);
                self.arena.text(label.to_string()).append(self.arena.text(" = ")).append(expression)
            }
            CheckedRecordUpdate::Branch { label, updates } => self
                .arena
                .text(label.to_string())
                .append(self.arena.text(" "))
                .append(self.record_updates(updates)),
        });
        let updates = updates.collect_vec();
        let updates = self.separated_by_comma(updates).group();
        self.arena.text("{ ").append(updates).append(self.arena.text(" }")).group()
    }

    fn do_expression(&mut self, expression: &CheckedDoExpression) -> Doc<'arena> {
        let mut continuation =
            self.expression(expression.final_expression, Precedence::Abstraction);

        for step in expression.steps.iter().rev() {
            match step {
                CheckedDoStep::Bind { binder, expression, application, .. }
                | CheckedDoStep::Discard { binder, expression, application, .. } => {
                    let binder = self.binder(*binder);
                    let lambda = self.lambda_document(vec![binder], continuation);
                    let lambda = self.parenthesize_if(true, lambda);
                    let action = self.expression(*expression, Precedence::Atom);
                    continuation = self.binary_application(application, action, lambda);
                }
                CheckedDoStep::Statement(statement) => {
                    continuation = self.block_statement(statement, continuation);
                }
            }
        }

        continuation
    }

    fn ado_expression(&mut self, expression: &CheckedAdoExpression) -> Doc<'arena> {
        match expression {
            CheckedAdoExpression::Pure { statements, expression, application } => {
                let argument = self.expression(*expression, Precedence::Atom);
                let mut application = self.unary_application(application, argument);
                for statement in statements.iter().rev() {
                    application = self.block_statement(statement, application);
                }
                application
            }
            CheckedAdoExpression::Error { statements, .. } => {
                let mut recovery = self.arena.text("<error-ado>");
                for statement in statements.iter().rev() {
                    recovery = self.block_statement(statement, recovery);
                }
                recovery
            }
            CheckedAdoExpression::Actions { steps, expression, .. } => {
                let binders = steps.iter().filter_map(|step| match step {
                    CheckedAdoStep::Map { binder, .. } | CheckedAdoStep::Apply { binder, .. } => {
                        Some(self.binder(*binder))
                    }
                    CheckedAdoStep::Statement(_) => None,
                });
                let binders = binders.collect_vec();
                let mut body = self.expression(*expression, Precedence::Abstraction);
                for step in steps.iter().rev() {
                    if let CheckedAdoStep::Statement(statement) = step {
                        body = self.block_statement(statement, body);
                    }
                }
                let lambda = self.lambda_document(binders, body);
                let mut continuation = self.parenthesize_if(true, lambda);

                for step in steps.iter() {
                    let (action, application, parenthesize_continuation) = match step {
                        CheckedAdoStep::Map { expression, application, .. } => {
                            (*expression, application, false)
                        }
                        CheckedAdoStep::Apply { expression, application, .. } => {
                            (*expression, application, true)
                        }
                        CheckedAdoStep::Statement(_) => continue,
                    };
                    if parenthesize_continuation {
                        continuation = self.parenthesize_if(true, continuation);
                    }
                    let action = self.expression(action, Precedence::Atom);
                    continuation = self.binary_application(application, continuation, action);
                }
                continuation
            }
        }
    }

    fn block_statement(
        &mut self,
        statement: &CheckedBlockStatement,
        continuation: Doc<'arena>,
    ) -> Doc<'arena> {
        match statement {
            CheckedBlockStatement::Let(statement) => {
                self.let_statement(statement).append(self.arena.text(" in ")).append(continuation)
            }
            CheckedBlockStatement::Error(_) => {
                self.arena.text("<error-statement>; ").append(continuation)
            }
        }
    }

    fn let_statement(&mut self, statement: &CheckedLetStatement) -> Doc<'arena> {
        let bindings = statement.bindings.iter().map(|binding| match binding {
            CheckedLetBinding::Pattern { binder: Some(binder) } => self.binder(*binder),
            CheckedLetBinding::Pattern { binder: None } => self.arena.text("<missing-pattern>"),
            CheckedLetBinding::Name { binding, type_id } => {
                let name = self.let_binding_name(*binding);
                let type_document = self.type_document(*type_id);
                self.arena.text(name).append(self.arena.text(" :: ")).append(type_document)
            }
        });
        let bindings = bindings.collect_vec();
        let bindings = self.separated_by_comma(bindings);
        self.arena.text("let { ").append(bindings).append(self.arena.text(" }")).group()
    }

    fn binary_application(
        &mut self,
        application: &CheckedBinaryApplication,
        first: Doc<'arena>,
        second: Doc<'arena>,
    ) -> Doc<'arena> {
        match application {
            CheckedBinaryApplication::Complete {
                function,
                first: first_step,
                second: second_step,
            } => {
                let function = self.expression(*function, Precedence::Application);
                let function = self.evidence_application(function, &first_step.evidence);
                let partial = self.term_application(function, first);
                let partial = self.evidence_application(partial, &second_step.evidence);
                self.term_application(partial, second)
            }
            CheckedBinaryApplication::Partial { function, first: first_step } => {
                let function = self.expression(*function, Precedence::Application);
                let function = self.evidence_application(function, &first_step.evidence);
                let partial = self.term_application(function, first);
                self.arena
                    .text("<partial-application ")
                    .append(partial)
                    .append(self.arena.text(" "))
                    .append(second)
                    .append(self.arena.text(">"))
            }
            CheckedBinaryApplication::Error { function } => {
                let function = self.expression(*function, Precedence::Application);
                self.arena
                    .text("<error-application ")
                    .append(function)
                    .append(self.arena.text(" "))
                    .append(first)
                    .append(self.arena.text(" "))
                    .append(second)
                    .append(self.arena.text(">"))
            }
        }
    }

    fn unary_application(
        &mut self,
        application: &CheckedUnaryApplication,
        argument: Doc<'arena>,
    ) -> Doc<'arena> {
        match application {
            CheckedUnaryApplication::Complete { function, application } => {
                let function = self.expression(*function, Precedence::Application);
                let function = self.evidence_application(function, &application.evidence);
                self.term_application(function, argument)
            }
            CheckedUnaryApplication::Error { function } => {
                let function = self.expression(*function, Precedence::Application);
                self.arena
                    .text("<error-application ")
                    .append(function)
                    .append(self.arena.text(" "))
                    .append(argument)
                    .append(self.arena.text(">"))
            }
        }
    }

    fn evidence_application(
        &mut self,
        expression: Doc<'arena>,
        evidence: &[EvidenceVarId],
    ) -> Doc<'arena> {
        if evidence.is_empty() {
            return expression;
        }

        let evidence = evidence.iter().map(|evidence_id| self.evidence_variable(*evidence_id));
        let evidence = evidence.collect_vec();
        let evidence = self.separated_by_comma(evidence).group();
        let argument = self
            .arena
            .line()
            .append(self.arena.text("@{"))
            .append(evidence)
            .append(self.arena.text("}"))
            .nest(2);
        expression.append(argument).group()
    }

    fn term_application(&self, function: Doc<'arena>, argument: Doc<'arena>) -> Doc<'arena> {
        let argument = self.arena.line().append(argument).nest(2);
        function.append(argument).group()
    }

    fn lambda_document(&self, binders: Vec<Doc<'arena>>, expression: Doc<'arena>) -> Doc<'arena> {
        let binders = self.separated_by_space(binders).group();
        self.arena.text("\\").append(binders).append(self.arena.text(" -> ")).append(expression)
    }

    fn case_expression(
        &mut self,
        scrutinees: &[CheckedExpressionId],
        alternatives: &[CheckedCaseAlternative],
    ) -> Doc<'arena> {
        let scrutinees =
            scrutinees.iter().map(|scrutinee| self.expression(*scrutinee, Precedence::Abstraction));
        let scrutinees = scrutinees.collect::<Vec<_>>();
        let scrutinees = self.separated_by_comma(scrutinees).group();
        let header = self.arena.text("case ").append(scrutinees).append(self.arena.text(" of"));

        let alternatives =
            alternatives.iter().map(|alternative| self.case_alternative(alternative));
        let alternatives = alternatives.collect_vec();
        let alternatives = alternatives.into_iter();
        let alternatives = alternatives.fold(self.arena.nil(), |document, alternative| {
            document.append(self.arena.hardline()).append(alternative)
        });

        header.append(alternatives.nest(2))
    }

    fn case_alternative(&mut self, alternative: &CheckedCaseAlternative) -> Doc<'arena> {
        let binders = alternative.binders.iter().map(|binder_id| self.binder(*binder_id));
        let binders = binders.collect::<Vec<_>>();
        let binders = self.separated_by_comma(binders).group();

        if let [result] = alternative.results.as_ref()
            && result.guards.is_empty()
        {
            let expression = self.expression(result.expression, Precedence::Abstraction);
            return binders.append(self.arena.text(" -> ")).append(expression);
        }

        let results = alternative.results.iter().map(|result| self.guarded_expression(result));
        let results = results.collect::<Vec<_>>();
        let results = results.into_iter();
        let results = results.fold(self.arena.nil(), |document, result| {
            document.append(self.arena.hardline()).append(result)
        });

        binders.append(results.nest(2))
    }

    fn guarded_expression(&mut self, guarded: &CheckedGuardedExpression) -> Doc<'arena> {
        let guards = guarded.guards.iter().map(|guard| self.pattern_guard(guard));
        let guards = guards.collect::<Vec<_>>();
        let expression = self.expression(guarded.expression, Precedence::Abstraction);

        if guards.is_empty() {
            return self.arena.text("-> ").append(expression);
        }

        let guards = self.separated_by_comma(guards).group();
        self.arena.text("| ").append(guards).append(self.arena.text(" -> ")).append(expression)
    }

    fn pattern_guard(&mut self, guard: &CheckedPatternGuard) -> Doc<'arena> {
        match *guard {
            CheckedPatternGuard::Boolean { expression } => {
                self.expression(expression, Precedence::Abstraction)
            }
            CheckedPatternGuard::Pattern { binder, expression } => {
                let binder = self.binder(binder);
                let expression = self.expression(expression, Precedence::Abstraction);
                binder.append(self.arena.text(" <- ")).append(expression)
            }
        }
    }

    fn type_document(&mut self, type_id: TypeId) -> Doc<'arena> {
        let rendered = self.types.render(type_id);
        let mut lines = rendered.split('\n');
        let first = lines.next().unwrap_or_default();
        let first = self.arena.text(first.to_string());

        lines.fold(first, |document, line| {
            document.append(self.arena.hardline()).append(self.arena.text(line.to_string()))
        })
    }

    fn variable(&self, variable: TermVariableResolution) -> String {
        match variable {
            TermVariableResolution::Binder(binder) => self.binder_name(binder),
            TermVariableResolution::Let(binding) => self.let_binding_name(binding),
            TermVariableResolution::RecordPun(pun) => {
                format!("pun{}", pun.into_raw().get())
            }
            TermVariableResolution::Reference(file_id, item_id) => self.item_name(file_id, item_id),
        }
    }

    fn let_binding_name(&self, binding: LetBindingNameGroupId) -> String {
        let graph_node = self.lowered.nodes.let_node(binding);
        graph_node
            .and_then(|graph_node| match &self.lowered.graph[graph_node] {
                GraphNode::Let { bindings, .. } => {
                    bindings.iter().find_map(|(name, &candidate)| {
                        (candidate == binding).then(|| name.to_string())
                    })
                }
                _ => None,
            })
            .unwrap_or_else(|| "<missing-let-name>".to_string())
    }

    fn item_name(&self, file_id: FileId, item_id: TermItemId) -> String {
        let indexed = self.queries.indexed(file_id).ok();
        indexed
            .and_then(|indexed| indexed.items[item_id].name.clone())
            .map(String::from)
            .unwrap_or_else(|| format!("item{}", item_id.into_raw().into_u32()))
    }

    fn binder(&self, binder_id: CheckedBinderId) -> Doc<'arena> {
        let checked_binder = &self.checked.core.binders[binder_id];
        match checked_binder.kind.clone() {
            CheckedBinderKind::Variable => {
                let name = checked_binder
                    .source
                    .map(|source| self.binder_name(source))
                    .unwrap_or_else(|| "<unimplemented>".to_string());
                self.arena.text(name)
            }
            CheckedBinderKind::Named { binder } => {
                let source = checked_binder.source;
                match source.and_then(|source| self.lowered.info.get_binder_kind(source)) {
                    Some(BinderKind::Named { named, .. }) => {
                        let named = named.as_deref().unwrap_or("<missing>");
                        let binder = self.binder(binder);
                        self.arena.text(format!("{named}@")).append(binder)
                    }
                    _ => self.arena.text("<invalid>"),
                }
            }
            CheckedBinderKind::Wildcard => self.arena.text("_"),
            CheckedBinderKind::Literal(literal) => self.arena.text(self.literal(literal)),
            CheckedBinderKind::Constructor { file_id, item_id, arguments } => {
                let constructor = self.item_name(file_id, item_id);
                if arguments.is_empty() {
                    return self.arena.text(constructor);
                }

                let arguments = arguments.iter().map(|argument| self.binder(*argument));
                let arguments = arguments.collect::<Vec<_>>();
                let arguments = self.separated_by_comma(arguments).group();
                self.arena
                    .text(constructor)
                    .append(self.arena.text("("))
                    .append(arguments)
                    .append(self.arena.text(")"))
            }
            CheckedBinderKind::Error => self.arena.text("<error>"),
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

    fn literal(&self, literal: CheckedLiteral) -> String {
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

    fn evidence_variable(&mut self, variable @ EvidenceVarId(index): EvidenceVarId) -> Doc<'arena> {
        match self.checked.evidence[variable].state {
            EvidenceState::Unsolved => self.arena.text(format!("ev{index}<unsolved>")),
            EvidenceState::Solved(evidence) => self.evidence(evidence),
            EvidenceState::Error => self.arena.text(format!("ev{index}<error>")),
        }
    }

    fn evidence(&mut self, evidence_id: EvidenceId) -> Doc<'arena> {
        match self.checked.evidence[evidence_id].clone() {
            Evidence::Variable(variable) => self.evidence_variable(variable),
            Evidence::Given(binder) => self.evidence_binder(binder),
            Evidence::Instance { origin, subgoals } => {
                let instance = self.instance_name(origin);
                if subgoals.is_empty() {
                    return self.arena.text(instance);
                }

                let subgoals = subgoals.into_iter().map(|subgoal| self.evidence_variable(subgoal));
                let subgoals = subgoals.collect::<Vec<_>>();
                let subgoals = self.separated_by_comma(subgoals).group();
                self.arena
                    .text(instance)
                    .append(self.arena.text(" @{"))
                    .append(subgoals)
                    .append(self.arena.text("}"))
            }
            Evidence::Superclass { parent, superclass } => {
                let superclass = self.superclass_name(superclass);
                let parent = self.evidence(parent);
                self.arena.text(format!("superclass[{superclass}] ")).append(parent)
            }
            Evidence::Trivial => self.arena.text("<trivial>"),
            Evidence::Synthesized(evidence) => self.synthesized_evidence(evidence),
        }
    }

    fn instance_name(&self, origin: InstanceCandidateOrigin) -> String {
        let file_id = match origin {
            InstanceCandidateOrigin::Instance(file_id, _)
            | InstanceCandidateOrigin::Derive(file_id, _) => file_id,
        };
        let indexed = self.queries.indexed(file_id).ok();
        let name = indexed.and_then(|indexed| {
            indexed.items.iter_terms().find_map(|(_, item)| {
                let matches = match (origin, &item.kind) {
                    (
                        InstanceCandidateOrigin::Instance(_, origin_id),
                        TermItemKind::Instance { id },
                    ) => origin_id == *id,
                    (
                        InstanceCandidateOrigin::Derive(_, origin_id),
                        TermItemKind::Derive { id },
                    ) => origin_id == *id,
                    _ => false,
                };
                matches.then(|| item.name.clone()).flatten()
            })
        });

        name.map(String::from).unwrap_or_else(|| match origin {
            InstanceCandidateOrigin::Instance(_, _) | InstanceCandidateOrigin::Derive(_, _) => {
                "<anonymous>".to_string()
            }
        })
    }

    fn superclass_name(&self, superclass: SuperclassId) -> String {
        let indexed = self.queries.indexed(superclass.file_id).ok();
        indexed
            .and_then(|indexed| indexed.items[superclass.type_id].name.clone())
            .map(String::from)
            .unwrap_or_else(|| "<anonymous>".to_string())
    }

    fn synthesized_evidence(&self, evidence: SynthesizedEvidence) -> Doc<'arena> {
        let rendered = match evidence {
            SynthesizedEvidence::IsSymbol(symbol) => format!("isSymbol({symbol:?})"),
            SynthesizedEvidence::Reflectable(evidence) => match evidence {
                ReflectableEvidence::Integer(integer) => format!("reflectable({integer})"),
                ReflectableEvidence::String(string) => format!("reflectable({string:?})"),
                ReflectableEvidence::Boolean(boolean) => format!("reflectable({boolean})"),
                ReflectableEvidence::Ordering(ordering) => {
                    let ordering = match ordering {
                        ReflectableOrdering::Less => "LT",
                        ReflectableOrdering::Equal => "EQ",
                        ReflectableOrdering::Greater => "GT",
                    };
                    format!("reflectable({ordering})")
                }
            },
        };
        self.arena.text(rendered)
    }

    fn evidence_binder(&self, EvidenceBinderId(index): EvidenceBinderId) -> Doc<'arena> {
        self.arena.text(format!("dict{index}"))
    }
}
