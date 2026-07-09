use std::cell::RefCell;
use std::fmt::Write;

use analyzer::QueryEngine;
use checking::core::pretty;
use checking::evidence::{EvidenceBinderId, InstanceCandidateOrigin};
use checking::{PrettyQueries, Type};
use elaborating::{
    CoreAlternativeId, CoreBindingId, CoreBindingSource, CoreBindingValue, CoreDeriveStrategy,
    CoreDerivedEvidence, CoreError, CoreExpression, CoreExpressionId, CoreExternalBinding,
    CoreLabel, CoreLiteral, CoreModule, CorePattern, CorePatternId, CoreSuperclassField,
    CoreTypeArgument, CoreVariable,
};
use files::FileId;
use indexing::{TermItemId, TermItemKind, TypeItemId};
use lowering::{
    BinderKind, BinderRecordItem, ExpressionKind, ExpressionRecordItem, LetBindingNameGroupId,
    LoweredModule, RecordPunId, StringKind,
};
use syntax::ast::AstNode;

/// Render semantic Core in a deliberately small, PureScript-like language.
///
/// This is a fixture renderer, not a source pretty-printer: surface wrappers
/// and operator syntax have already disappeared, while evidence applications
/// and dictionary construction remain visible.
pub fn report(engine: &QueryEngine, id: FileId, name: &str) -> String {
    let lowered = engine.lowered(id).unwrap();
    let checked = engine.checked(id).unwrap();
    let core = engine.elaborated(id).unwrap();
    let renderer = Renderer {
        engine,
        file: id,
        lowered: &lowered,
        core: &core,
        pretty: RefCell::new(pretty::Pretty::new(engine, &checked)),
    };

    let mut output = String::new();
    writeln!(output, "module {name} where").unwrap();

    for &group in &core.top_level {
        writeln!(output).unwrap();
        writeln!(output, "{}", renderer.top_level_group(group)).unwrap();
    }

    output
}

struct Renderer<'a> {
    engine: &'a QueryEngine,
    file: FileId,
    lowered: &'a LoweredModule,
    core: &'a CoreModule,
    pretty: RefCell<pretty::Pretty<'a, QueryEngine>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Control,
    Application,
    Access,
    Atom,
}

impl Renderer<'_> {
    fn top_level_group(&self, id: elaborating::CoreBindingGroupId) -> String {
        let group = &self.core.binding_groups[id];
        group
            .bindings
            .iter()
            .enumerate()
            .map(|(index, &binding)| {
                self.pretty.borrow_mut().reset();
                let prefix =
                    if group.recursive { if index == 0 { "rec " } else { "and " } } else { "" };
                format!("{prefix}{}", self.binding_declaration(binding, Precedence::Control))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn local_group(&self, id: elaborating::CoreBindingGroupId) -> String {
        let group = &self.core.binding_groups[id];
        group
            .bindings
            .iter()
            .enumerate()
            .map(|(index, &binding)| {
                let prefix =
                    if index == 0 { if group.recursive { "rec " } else { "" } } else { "and " };
                format!("{prefix}{}", self.binding_declaration(binding, Precedence::Application))
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn binding_declaration(&self, id: CoreBindingId, value_parent: Precedence) -> String {
        let name = self.binding_name(id);
        let type_ = self.core.binding_types.get(&id).map(|&type_| self.type_(type_));
        let value = match self.core.bindings[id].value {
            CoreBindingValue::Expression(expression) => self.expression(expression, value_parent),
            CoreBindingValue::External(CoreExternalBinding::ClassMember) => {
                "#external[class-member]".to_owned()
            }
            CoreBindingValue::External(CoreExternalBinding::Constructor) => {
                "#external[constructor]".to_owned()
            }
            CoreBindingValue::External(CoreExternalBinding::Foreign) => {
                "#external[foreign]".to_owned()
            }
        };
        match type_ {
            Some(type_) => format!("{name} : {type_} = {value}"),
            None => format!("{name} = {value}"),
        }
    }

    fn expression(&self, id: CoreExpressionId, parent: Precedence) -> String {
        let (precedence, rendered) = match &self.core.expressions[id] {
            CoreExpression::Variable(variable) => (Precedence::Atom, self.variable_name(*variable)),
            CoreExpression::Literal(literal) => (Precedence::Atom, self.literal(literal)),
            CoreExpression::Lambda { .. } => {
                let mut patterns = Vec::new();
                let mut body = id;
                while let CoreExpression::Lambda { pattern, body: inner } =
                    self.core.expressions[body]
                {
                    patterns.push(self.pattern(pattern));
                    body = inner;
                }
                let body = self.expression(body, Precedence::Control);
                (Precedence::Control, format!("\\{} -> {body}", patterns.join(" ")))
            }
            CoreExpression::Apply { function, argument } => {
                let function = self.expression(*function, Precedence::Application);
                let argument = self.expression(*argument, Precedence::Access);
                (Precedence::Application, format!("{function} {argument}"))
            }
            CoreExpression::TypeApply { function, argument } => {
                let function = self.expression(*function, Precedence::Application);
                let argument = match argument {
                    CoreTypeArgument::Checked(type_) => self.type_argument(*type_),
                    CoreTypeArgument::Missing => "#missing[type]".to_owned(),
                };
                (Precedence::Application, format!("{function} @{argument}"))
            }
            CoreExpression::Let { group, body } => {
                let group = self.local_group(*group);
                let body = self.expression(*body, Precedence::Control);
                (Precedence::Control, format!("let {group} in {body}"))
            }
            CoreExpression::Case { scrutinees, alternatives } => {
                let scrutinees = scrutinees
                    .iter()
                    .map(|&expression| self.expression(expression, Precedence::Application))
                    .collect::<Vec<_>>()
                    .join(", ");
                let alternatives = alternatives
                    .iter()
                    .map(|&alternative| self.alternative(alternative))
                    .collect::<Vec<_>>()
                    .join(" | ");
                (Precedence::Control, format!("case {scrutinees} of {alternatives}"))
            }
            CoreExpression::IfThenElse { condition, then, else_ } => {
                let condition = self.expression(*condition, Precedence::Application);
                let then = self.expression(*then, Precedence::Application);
                let else_ = self.expression(*else_, Precedence::Application);
                (Precedence::Control, format!("if {condition} then {then} else {else_}"))
            }
            CoreExpression::Array(elements) => {
                let elements = elements
                    .iter()
                    .map(|&element| self.expression(element, Precedence::Application))
                    .collect::<Vec<_>>()
                    .join(", ");
                (Precedence::Atom, format!("[{elements}]"))
            }
            CoreExpression::Record(fields) => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{} = {}",
                            self.label(&field.label),
                            self.expression(field.value, Precedence::Application)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                (Precedence::Atom, format!("{{ {fields} }}"))
            }
            CoreExpression::Dictionary { superclasses, members } => {
                let mut fields = Vec::new();
                for (index, superclass) in superclasses.iter().enumerate() {
                    let value = match superclass {
                        CoreSuperclassField::Runtime(expression) => {
                            self.expression(*expression, Precedence::Application)
                        }
                        CoreSuperclassField::Erased => "#erased".to_owned(),
                    };
                    fields.push(format!("$super{index} = {value}"));
                }
                fields.extend(members.iter().map(|field| {
                    format!(
                        "{} = {}",
                        self.label(&field.label),
                        self.expression(field.value, Precedence::Application)
                    )
                }));
                (Precedence::Atom, format!("dictionary {{ {} }}", fields.join(", ")))
            }
            CoreExpression::DerivedDictionary { strategy, class, local_binders, requirements } => {
                let strategy = match strategy {
                    CoreDeriveStrategy::Stock => "stock",
                    CoreDeriveStrategy::Newtype => "newtype",
                };
                let class = class
                    .map(|(file, item)| self.type_item_name(file, item))
                    .unwrap_or_else(|| "#missing[class]".to_owned());
                let binders = local_binders
                    .iter()
                    .map(|binder| {
                        let constraint = binder
                            .constraint
                            .map(|constraint| self.type_(constraint))
                            .unwrap_or_else(|| "#missing[constraint]".to_owned());
                        let erased = if binder.erased { " = #erased" } else { "" };
                        format!("{} : {constraint}{erased}", self.evidence_name(binder.binder))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let requirements = requirements
                    .iter()
                    .map(|requirement| {
                        let constraint = requirement
                            .constraint
                            .map(|constraint| self.type_(constraint))
                            .unwrap_or_else(|| "#missing[constraint]".to_owned());
                        let evidence = match requirement.evidence {
                            CoreDerivedEvidence::Runtime(expression) => {
                                self.expression(expression, Precedence::Application)
                            }
                            CoreDerivedEvidence::Erased => "#erased".to_owned(),
                        };
                        format!("{constraint} = {evidence}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                (
                    Precedence::Atom,
                    format!(
                        "derived {strategy} {class} {{ binders = [{binders}], requirements = [{requirements}] }}"
                    ),
                )
            }
            CoreExpression::Access { record, label } => {
                let record = self.expression(*record, Precedence::Access);
                (Precedence::Access, format!("{record}.{}", self.label(label)))
            }
            CoreExpression::Update { record, updates } => {
                let record = self.expression(*record, Precedence::Access);
                let updates = updates
                    .iter()
                    .map(|update| {
                        let path = update
                            .path
                            .iter()
                            .map(|label| self.label(label))
                            .collect::<Vec<_>>()
                            .join(".");
                        format!(
                            "{path} = {}",
                            self.expression(update.value, Precedence::Application)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                (Precedence::Control, format!("{record} {{ {updates} }}"))
            }
            CoreExpression::SuperclassProjection { dictionary, index } => {
                let dictionary = self.expression(*dictionary, Precedence::Access);
                (Precedence::Application, format!("#superclass[{index}] {dictionary}"))
            }
            CoreExpression::Error(error) => (Precedence::Atom, self.error(*error).to_owned()),
        };

        if precedence < parent { format!("({rendered})") } else { rendered }
    }

    fn alternative(&self, id: CoreAlternativeId) -> String {
        let alternative = &self.core.alternatives[id];
        let patterns = alternative
            .patterns
            .iter()
            .map(|&pattern| self.pattern(pattern))
            .collect::<Vec<_>>()
            .join(", ");
        let body = self.expression(alternative.body, Precedence::Application);
        format!("{patterns} -> {body}")
    }

    fn pattern(&self, id: CorePatternId) -> String {
        match &self.core.patterns[id] {
            CorePattern::Variable(variable) => self.variable_name(*variable),
            CorePattern::Literal(literal) => self.literal(literal),
            CorePattern::Constructor { constructor, arguments } => {
                let constructor = constructor
                    .map(|(file, item)| self.item_name(file, item))
                    .unwrap_or_else(|| "#missing[constructor]".to_owned());
                if arguments.is_empty() {
                    constructor
                } else {
                    let arguments = arguments
                        .iter()
                        .map(|&argument| self.pattern_atom(argument))
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("{constructor} {arguments}")
                }
            }
            CorePattern::Named { variable, pattern } => {
                format!("{}@{}", self.variable_name(*variable), self.pattern_atom(*pattern))
            }
            CorePattern::Wildcard => "_".to_owned(),
            CorePattern::Array(elements) => {
                let elements = elements
                    .iter()
                    .map(|&element| self.pattern(element))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{elements}]")
            }
            CorePattern::Record(fields) => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        format!("{}: {}", self.label(&field.label), self.pattern(field.pattern))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {fields} }}")
            }
            CorePattern::Error(error) => self.error(*error).to_owned(),
        }
    }

    fn pattern_atom(&self, id: CorePatternId) -> String {
        match &self.core.patterns[id] {
            CorePattern::Constructor { arguments, .. } if !arguments.is_empty() => {
                format!("({})", self.pattern(id))
            }
            CorePattern::Named { .. } => format!("({})", self.pattern(id)),
            CorePattern::Literal(CoreLiteral::Integer(Some(value))) if value.is_negative() => {
                format!("({})", self.pattern(id))
            }
            CorePattern::Literal(CoreLiteral::Number { negative: true, .. }) => {
                format!("({})", self.pattern(id))
            }
            _ => self.pattern(id),
        }
    }

    fn literal(&self, literal: &CoreLiteral) -> String {
        match literal {
            CoreLiteral::String { kind, value: Some(value) } => match kind {
                StringKind::String => format!("{:?}", value.as_str()),
                StringKind::RawString => format!("\"\"\"{value}\"\"\""),
            },
            CoreLiteral::String { value: None, .. } => "#missing[string]".to_owned(),
            CoreLiteral::Char(Some(value)) => format!("{value:?}"),
            CoreLiteral::Char(None) => "#missing[char]".to_owned(),
            CoreLiteral::Boolean(value) => value.to_string(),
            CoreLiteral::Integer(Some(value)) => value.to_string(),
            CoreLiteral::Integer(None) => "#missing[integer]".to_owned(),
            CoreLiteral::Number { negative, value: Some(value) } => {
                format!("{}{value}", if *negative { "-" } else { "" })
            }
            CoreLiteral::Number { value: None, .. } => "#missing[number]".to_owned(),
        }
    }

    fn variable_name(&self, variable: CoreVariable) -> String {
        match variable {
            CoreVariable::Binder(id) => match self.lowered.info.get_binder_kind(id) {
                Some(BinderKind::Variable { variable: Some(name) }) => name.to_string(),
                Some(BinderKind::Named { named: Some(name), .. }) => name.to_string(),
                _ => format!("$binder{}", id.into_raw().get()),
            },
            CoreVariable::Synthetic(id) => format!("$synthetic{id}"),
            CoreVariable::Let(id) => self.let_name(id),
            CoreVariable::RecordPun(id) => self.pun_name(id),
            CoreVariable::Item(file, item) => self.item_name(file, item),
            CoreVariable::Evidence(id) => self.evidence_name(id),
            CoreVariable::Instance(origin) => self.instance_name(origin),
        }
    }

    fn binding_name(&self, id: CoreBindingId) -> String {
        match self.core.bindings[id].source {
            CoreBindingSource::Item(item) => self.item_name(self.file, item),
            CoreBindingSource::Let(id) => self.let_name(id),
            CoreBindingSource::Synthetic(id) => format!("$synthetic{id}"),
        }
    }

    fn item_name(&self, file: FileId, id: TermItemId) -> String {
        let raw = self.engine.indexed(file).ok().map_or_else(
            || format!("$item{}", id.into_raw().into_u32()),
            |indexed| {
                let item = &indexed.items[id];
                item.name.as_ref().map(ToString::to_string).unwrap_or_else(|| match &item.kind {
                    TermItemKind::Derive { .. } => {
                        format!("$derive{}", self.anonymous_item_ordinal(file, id, true))
                    }
                    TermItemKind::Instance { .. } => {
                        format!("$instance{}", self.anonymous_item_ordinal(file, id, false))
                    }
                    _ => format!("$item{}", id.into_raw().into_u32()),
                })
            },
        );
        let raw = self.binding_identifier(&raw);
        if file == self.file {
            raw
        } else {
            self.module_name(file).map_or(raw.clone(), |module| format!("{module}.{raw}"))
        }
    }

    fn type_item_name(&self, file: FileId, id: TypeItemId) -> String {
        let raw = self
            .engine
            .indexed(file)
            .ok()
            .and_then(|indexed| indexed.items[id].name.clone())
            .map(|name| name.to_string())
            .unwrap_or_else(|| format!("$type{}", id.into_raw().into_u32()));
        if file == self.file {
            raw
        } else {
            self.module_name(file).map_or(raw.clone(), |module| format!("{module}.{raw}"))
        }
    }

    fn instance_name(&self, origin: InstanceCandidateOrigin) -> String {
        if let Some(&binding) = self.core.instances.get(&origin) {
            return self.binding_name(binding);
        }

        let item = match origin {
            InstanceCandidateOrigin::Instance(file, instance) => self
                .engine
                .indexed(file)
                .ok()
                .and_then(|indexed| {
                    indexed.items.iter_terms().find_map(|(item, term)| {
                        matches!(term.kind, TermItemKind::Instance { id } if id == instance)
                            .then_some(item)
                    })
                })
                .map(|item| (file, item)),
            InstanceCandidateOrigin::Derive(file, derive) => self
                .engine
                .indexed(file)
                .ok()
                .and_then(|indexed| {
                    indexed.items.iter_terms().find_map(|(item, term)| {
                        matches!(term.kind, TermItemKind::Derive { id } if id == derive)
                            .then_some(item)
                    })
                })
                .map(|item| (file, item)),
        };

        let Some((file, item)) = item else {
            let (file, raw) = match origin {
                InstanceCandidateOrigin::Instance(file, _) => (file, "$instance?".to_owned()),
                InstanceCandidateOrigin::Derive(file, _) => (file, "$derive?".to_owned()),
            };
            return if file == self.file {
                raw
            } else {
                self.module_name(file).map_or(raw.clone(), |module| format!("{module}.{raw}"))
            };
        };

        self.item_name(file, item)
    }

    fn anonymous_item_ordinal(&self, file: FileId, id: TermItemId, derive: bool) -> usize {
        self.engine
            .indexed(file)
            .ok()
            .and_then(|indexed| {
                indexed
                    .items
                    .iter_terms()
                    .filter(|(_, item)| {
                        matches!(
                            (&item.kind, derive),
                            (TermItemKind::Derive { .. }, true)
                                | (TermItemKind::Instance { .. }, false)
                        )
                    })
                    .position(|(candidate, _)| candidate == id)
            })
            .unwrap_or(0)
    }

    fn let_name(&self, id: LetBindingNameGroupId) -> String {
        let group = self.lowered.info.get_let_binding_group(id);
        let name = self.engine.parsed(self.file).ok().and_then(|parsed| {
            let content = self.engine.content(self.file);
            let stabilized = self.engine.stabilized(self.file).ok()?;
            let module = parsed.0.cst();
            if let Some(signature) = group.signature {
                let signature = stabilized.ast_ptr(signature)?.to_node(module.syntax());
                signature.name_token().map(|token| token.text(&content).to_string())
            } else {
                let equation = *group.equations.first()?;
                let equation = stabilized.ast_ptr(equation)?.to_node(module.syntax());
                equation.name_token().map(|token| token.text(&content).to_string())
            }
        });
        name.unwrap_or_else(|| format!("$let{}", id.into_raw().into_u32()))
    }

    fn pun_name(&self, id: RecordPunId) -> String {
        for (_, kind) in self.lowered.info.iter_expression() {
            if let ExpressionKind::Record { record } = kind {
                for field in record.iter() {
                    if let ExpressionRecordItem::RecordPun {
                        id: candidate, name: Some(name), ..
                    } = field
                        && *candidate == id
                    {
                        return name.to_string();
                    }
                }
            }
        }
        for (_, kind) in self.lowered.info.iter_binder() {
            if let BinderKind::Record { record } = kind {
                for field in record.iter() {
                    if let BinderRecordItem::RecordPun { id: candidate, name: Some(name) } = field
                        && *candidate == id
                    {
                        return name.to_string();
                    }
                }
            }
        }
        format!("$pun{}", id.into_raw().get())
    }

    fn evidence_name(&self, id: EvidenceBinderId) -> String {
        format!("$dict{}", id.0)
    }

    fn label(&self, label: &CoreLabel) -> String {
        let label = match label {
            CoreLabel::Source(label) => label.to_string(),
            CoreLabel::Item(file, item) => self
                .engine
                .indexed(*file)
                .ok()
                .and_then(|indexed| indexed.items[*item].name.clone())
                .map(|name| name.to_string())
                .unwrap_or_else(|| format!("$label{}", item.into_raw().into_u32())),
            CoreLabel::Missing => return "#missing[label]".to_owned(),
        };
        let mut characters = label.chars();
        let identifier = characters.next().is_some_and(|character| {
            character.is_alphabetic() || character == '_' || character == '$'
        }) && characters
            .all(|character| character.is_alphanumeric() || character == '_' || character == '\'');
        if identifier { label } else { format!("{label:?}") }
    }

    fn type_(&self, id: checking::TypeId) -> String {
        self.pretty.borrow_mut().render(id).to_string()
    }

    fn type_argument(&self, id: checking::TypeId) -> String {
        let rendered = self.type_(id);
        match self.engine.lookup_type(id) {
            Type::Constructor(..)
            | Type::Integer(..)
            | Type::String(..)
            | Type::Row(..)
            | Type::Rigid(..)
            | Type::Unification(..)
            | Type::Free(..)
            | Type::Unknown(..) => rendered,
            Type::Application(..)
            | Type::KindApplication(..)
            | Type::Forall(..)
            | Type::Constrained(..)
            | Type::Function(..)
            | Type::Kinded(..) => format!("({rendered})"),
        }
    }

    fn module_name(&self, id: FileId) -> Option<String> {
        let content = self.engine.content(id);
        let parsed = self.engine.parsed(id).ok()?;
        parsed.0.module_name(&content).map(|name| name.to_string())
    }

    fn binding_identifier(&self, name: &str) -> String {
        if name
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '_' | '\'' | '$'))
        {
            name.to_owned()
        } else {
            format!("({name})")
        }
    }

    fn error(&self, error: CoreError) -> &'static str {
        match error {
            CoreError::Hole => "#error[hole]",
            CoreError::MissingExpression => "#error[missing-expression]",
            CoreError::MissingPattern => "#error[missing-pattern]",
            CoreError::MalformedOperator => "#error[malformed-operator]",
            CoreError::MalformedSection => "#error[malformed-section]",
            CoreError::PatternMatchFailure => "#error[pattern-match-failure]",
            CoreError::Evidence => "#error[evidence]",
        }
    }
}
