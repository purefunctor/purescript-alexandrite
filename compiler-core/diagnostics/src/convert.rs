use checking::error::{CheckError, ErrorKind};
use indexing::{IndexingError, TypeItemKind};
use itertools::Itertools;
use lowering::LoweringError;
use resolving::ResolvingError;
use rowan::ast::AstNode;

use crate::{Diagnostic, DiagnosticsContext, ExternalQueries, Severity};

pub trait ToDiagnostics {
    fn to_diagnostics<Q>(&self, context: &DiagnosticsContext<'_, Q>) -> Vec<Diagnostic>
    where
        Q: ExternalQueries;
}

impl ToDiagnostics for LoweringError {
    fn to_diagnostics<Q>(&self, context: &DiagnosticsContext<'_, Q>) -> Vec<Diagnostic>
    where
        Q: ExternalQueries,
    {
        match self {
            LoweringError::NotInScope(not_in_scope) => {
                let (ptr, name) = match not_in_scope {
                    lowering::NotInScope::ExprConstructor { id } => {
                        (context.stabilized.syntax_ptr(*id), None)
                    }
                    lowering::NotInScope::ExprVariable { id } => {
                        (context.stabilized.syntax_ptr(*id), None)
                    }
                    lowering::NotInScope::ExprOperatorName { id } => {
                        (context.stabilized.syntax_ptr(*id), None)
                    }
                    lowering::NotInScope::TypeConstructor { id } => {
                        (context.stabilized.syntax_ptr(*id), None)
                    }
                    lowering::NotInScope::TypeVariable { id } => {
                        (context.stabilized.syntax_ptr(*id), None)
                    }
                    lowering::NotInScope::TypeOperatorName { id } => {
                        (context.stabilized.syntax_ptr(*id), None)
                    }
                    lowering::NotInScope::NegateFn { id } => {
                        (context.stabilized.syntax_ptr(*id), Some("negate"))
                    }
                    lowering::NotInScope::DoFn { kind, id } => (
                        context.stabilized.syntax_ptr(*id),
                        match kind {
                            lowering::DoFn::Bind => Some("bind"),
                            lowering::DoFn::Discard => Some("discard"),
                        },
                    ),
                    lowering::NotInScope::AdoFn { kind, id } => (
                        context.stabilized.syntax_ptr(*id),
                        match kind {
                            lowering::AdoFn::Map => Some("map"),
                            lowering::AdoFn::Apply => Some("apply"),
                            lowering::AdoFn::Pure => Some("pure"),
                        },
                    ),
                    lowering::NotInScope::TermOperator { id } => {
                        (context.stabilized.syntax_ptr(*id), None)
                    }
                    lowering::NotInScope::TypeOperator { id } => {
                        (context.stabilized.syntax_ptr(*id), None)
                    }
                };

                let Some(ptr) = ptr else { return vec![] };
                let Some(span) = context.span_from_syntax_ptr(&ptr) else { return vec![] };

                let message = if let Some(name) = name {
                    format!("'{name}' is not in scope")
                } else {
                    let text = context.text_of(span).trim();
                    format!("'{text}' is not in scope")
                };

                vec![Diagnostic::error("NotInScope", message, span, "lowering")]
            }

            LoweringError::RecursiveSynonym(group) => convert_recursive_group(
                context,
                &group.group,
                "RecursiveSynonym",
                "Invalid type synonym cycle",
            ),

            LoweringError::RecursiveKinds(group) => convert_recursive_group(
                context,
                &group.group,
                "RecursiveKinds",
                "Invalid kind cycle",
            ),
        }
    }
}

fn convert_recursive_group<Q>(
    context: &DiagnosticsContext<'_, Q>,
    group: &[indexing::TypeItemId],
    code: &'static str,
    message: &'static str,
) -> Vec<Diagnostic>
where
    Q: ExternalQueries,
{
    let spans = group.iter().filter_map(|id| {
        let ptr = match context.indexed.items[*id].kind {
            TypeItemKind::Synonym { equation, .. } => context.stabilized.syntax_ptr(equation?)?,
            TypeItemKind::Data { equation, .. } => context.stabilized.syntax_ptr(equation?)?,
            TypeItemKind::Newtype { equation, .. } => context.stabilized.syntax_ptr(equation?)?,
            _ => return None,
        };
        context.span_from_syntax_ptr(&ptr)
    });

    let spans = spans.collect_vec();

    let Some(&primary) = spans.first() else { return vec![] };

    let mut diagnostic = Diagnostic::error(code, message, primary, "lowering");

    for &span in &spans[1..] {
        diagnostic = diagnostic.with_related(span, "Includes this type");
    }

    vec![diagnostic]
}

impl ToDiagnostics for ResolvingError {
    fn to_diagnostics<Q>(&self, context: &DiagnosticsContext<'_, Q>) -> Vec<Diagnostic>
    where
        Q: ExternalQueries,
    {
        match self {
            ResolvingError::TermExportConflict { .. }
            | ResolvingError::TypeExportConflict { .. }
            | ResolvingError::ExistingTerm { .. }
            | ResolvingError::ExistingType { .. } => {
                vec![]
            }

            ResolvingError::InvalidImportStatement { id } => {
                let Some(ptr) = context.stabilized.ast_ptr(*id) else { return vec![] };

                let message = {
                    let cst = ptr.to_node(context.root);
                    let name = cst.module_name().map(|cst| {
                        let range = cst.syntax().text_range();
                        context.content[range].trim()
                    });
                    let name = name.unwrap_or("<ParseError>");
                    format!("Cannot import module '{name}'")
                };

                let Some(span) = context.span_from_ast_ptr(&ptr) else { return vec![] };

                vec![Diagnostic::error("InvalidImportStatement", message, span, "resolving")]
            }

            ResolvingError::InvalidImportItem { id } => {
                let Some(ptr) = context.stabilized.syntax_ptr(*id) else { return vec![] };
                let Some(span) = context.span_from_syntax_ptr(&ptr) else { return vec![] };

                let text = context.text_of(span).trim();
                let message = format!("Cannot import item '{text}'");

                vec![Diagnostic::error("InvalidImportItem", message, span, "resolving")]
            }
        }
    }
}

impl ToDiagnostics for IndexingError {
    fn to_diagnostics<Q>(&self, context: &DiagnosticsContext<'_, Q>) -> Vec<Diagnostic>
    where
        Q: ExternalQueries,
    {
        match self {
            IndexingError::DuplicateImport { duplicate, existing } => {
                let Some(ptr) = context.stabilized.syntax_ptr(*duplicate) else { return vec![] };
                let Some(span) = context.span_from_syntax_ptr(&ptr) else { return vec![] };

                let text = context.text_of(span).trim();
                let message = format!("Import list contains multiple references to '{text}'");

                let mut diagnostic =
                    Diagnostic::warning("DuplicateImport", message, span, "indexing");

                if let Some(existing_ptr) = context.stabilized.syntax_ptr(*existing)
                    && let Some(existing_span) = context.span_from_syntax_ptr(&existing_ptr)
                {
                    diagnostic = diagnostic.with_related(existing_span, "First imported here");
                }

                vec![diagnostic]
            }
            IndexingError::DuplicateItem { .. }
            | IndexingError::MismatchedItem { .. }
            | IndexingError::InvalidRole { .. }
            | IndexingError::InvalidExport { .. }
            | IndexingError::DuplicateExport { .. } => vec![],
        }
    }
}

impl ToDiagnostics for CheckError {
    fn to_diagnostics<Q>(&self, context: &DiagnosticsContext<'_, Q>) -> Vec<Diagnostic>
    where
        Q: ExternalQueries,
    {
        let span = context.primary_span_from_crumbs(&self.crumbs);
        let lookup_message = |id| context.queries.lookup_checking_smol_str(id);
        let render_type = |id| context.render_type(id);

        let (severity, code, message) = match &self.kind {
            ErrorKind::AmbiguousConstraint { constraint } => {
                let msg = render_type(*constraint);
                (Severity::Error, "AmbiguousConstraint", format!("Ambiguous constraint: {msg}"))
            }
            ErrorKind::CannotDeriveClass { .. } => {
                (Severity::Error, "CannotDeriveClass", "Cannot derive this class".to_string())
            }
            ErrorKind::CannotDeriveForType { type_message } => {
                let msg = render_type(*type_message);
                (Severity::Error, "CannotDeriveForType", format!("Cannot derive for type: {msg}"))
            }
            ErrorKind::ContravariantOccurrence { type_message } => {
                let msg = render_type(*type_message);
                (
                    Severity::Error,
                    "ContravariantOccurrence",
                    format!("Type variable occurs in contravariant position: {msg}"),
                )
            }
            ErrorKind::CovariantOccurrence { type_message } => {
                let msg = render_type(*type_message);
                (
                    Severity::Error,
                    "CovariantOccurrence",
                    format!("Type variable occurs in covariant position: {msg}"),
                )
            }
            ErrorKind::CannotUnify { t1, t2 } => {
                let t1 = render_type(*t1);
                let t2 = render_type(*t2);
                (Severity::Error, "CannotUnify", format!("Cannot unify '{t1}' with '{t2}'"))
            }
            ErrorKind::DeriveInvalidArity { expected, actual, .. } => (
                Severity::Error,
                "DeriveInvalidArity",
                format!("Invalid arity for derive: expected {expected}, got {actual}"),
            ),
            ErrorKind::DeriveNotSupportedYet { .. } => (
                Severity::Error,
                "DeriveNotSupportedYet",
                "Deriving this class is not supported yet".to_string(),
            ),
            ErrorKind::DeriveMissingFunctor => (
                Severity::Error,
                "DeriveMissingFunctor",
                "Deriving Functor requires Data.Functor to be in scope".to_string(),
            ),
            ErrorKind::EmptyAdoBlock => {
                (Severity::Error, "EmptyAdoBlock", "Empty ado block".to_string())
            }
            ErrorKind::EmptyDoBlock => {
                (Severity::Error, "EmptyDoBlock", "Empty do block".to_string())
            }
            ErrorKind::InvalidFinalBind => (
                Severity::Warning,
                "InvalidFinalBind",
                "Invalid final bind statement in do expression".to_string(),
            ),
            ErrorKind::InvalidFinalLet => (
                Severity::Error,
                "InvalidFinalLet",
                "Invalid final let statement in do expression".to_string(),
            ),
            ErrorKind::InstanceHeadMismatch { expected, actual, .. } => (
                Severity::Error,
                "InstanceHeadMismatch",
                format!("Instance head mismatch: expected {expected} arguments, got {actual}"),
            ),
            ErrorKind::InstanceHeadLabeledRow { position, type_message, .. } => {
                let type_msg = render_type(*type_message);
                (
                    Severity::Error,
                    "InstanceHeadLabeledRow",
                    format!(
                        "Instance argument at position {position} contains a labeled row, \
                         but this position is not determined by any functional dependency. \
                         Only the `( | r )` form is allowed. Got '{type_msg}' instead."
                    ),
                )
            }
            ErrorKind::InstanceMemberTypeMismatch { expected, actual } => {
                let expected = render_type(*expected);
                let actual = render_type(*actual);
                (
                    Severity::Error,
                    "InstanceMemberTypeMismatch",
                    format!("Instance member type mismatch: expected '{expected}', got '{actual}'"),
                )
            }
            ErrorKind::InvalidTypeApplication { function_type, function_kind, argument_type } => {
                let function_type = render_type(*function_type);
                let function_kind = render_type(*function_kind);
                let argument_type = render_type(*argument_type);
                (
                    Severity::Error,
                    "InvalidTypeApplication",
                    format!(
                        "Cannot apply type '{function_type}' to '{argument_type}'. \
                         '{function_type}' has kind '{function_kind}', which is not a function kind."
                    ),
                )
            }
            ErrorKind::ExpectedNewtype { type_message } => {
                let msg = render_type(*type_message);
                (Severity::Error, "ExpectedNewtype", format!("Expected a newtype, got: {msg}"))
            }
            ErrorKind::InvalidNewtypeDeriveSkolemArguments => (
                Severity::Error,
                "InvalidNewtypeDeriveSkolemArguments",
                "Cannot derive newtype instance where skolemised arguments do not appear trailing in the inner type."
                    .to_string(),
            ),
            ErrorKind::NonLocalNewtype { type_message } => {
                let msg = render_type(*type_message);
                (Severity::Error, "NonLocalNewtype", format!("Expected a local newtype, got: {msg}"))
            }
            ErrorKind::NoInstanceFound { constraint, .. } => {
                let constraint = render_type(*constraint);
                let message = format!("No instance found for: {constraint}");
                (Severity::Error, "NoInstanceFound", message)
            }
            ErrorKind::NoVisibleTypeVariable { function_type } => {
                let msg = render_type(*function_type);
                (
                    Severity::Error,
                    "NoVisibleTypeVariable",
                    format!("No visible type variable for type application in: {msg}"),
                )
            }
            ErrorKind::PartialSynonymApplication { .. } => (
                Severity::Error,
                "PartialSynonymApplication",
                "Partial type synonym application".to_string(),
            ),
            ErrorKind::RecursiveSynonymExpansion { .. } => (
                Severity::Error,
                "RecursiveSynonymExpansion",
                "Recursive type synonym expansion".to_string(),
            ),
            ErrorKind::TooManyBinders { expected, actual, .. } => (
                Severity::Error,
                "TooManyBinders",
                format!("Too many binders: expected {expected}, got {actual}"),
            ),
            ErrorKind::TypeSignatureVariableMismatch { expected, actual, .. } => (
                Severity::Error,
                "TypeSignatureVariableMismatch",
                format!(
                    "Type signature variable mismatch: expected {expected} variables, got {actual}"
                ),
            ),
            ErrorKind::InvalidRoleDeclaration { declared, inferred, .. } => (
                Severity::Error,
                "InvalidRoleDeclaration",
                format!("Invalid role declaration: declared {declared:?}, inferred {inferred:?}"),
            ),
            ErrorKind::CoercibleConstructorNotInScope { .. } => (
                Severity::Error,
                "CoercibleConstructorNotInScope",
                "Constructor not in scope for Coercible".to_string(),
            ),
            ErrorKind::RedundantPatterns { patterns } => {
                let patterns = patterns.join(", ");
                (
                    Severity::Warning,
                    "RedundantPattern",
                    format!("Pattern match has redundant patterns: {patterns}"),
                )
            }
            ErrorKind::MissingPatterns { patterns } => {
                let patterns = patterns.join(", ");
                (
                    Severity::Warning,
                    "MissingPatterns",
                    format!("Pattern match is not exhaustive. Missing: {patterns}"),
                )
            }
            ErrorKind::CustomWarning { message_id } => {
                let msg = lookup_message(*message_id);
                (Severity::Warning, "CustomWarning", msg.to_string())
            }
            ErrorKind::CustomFailure { message_id } => {
                let msg = lookup_message(*message_id);
                (Severity::Error, "CustomFailure", msg.to_string())
            }
            ErrorKind::PropertyIsMissing { labels } => {
                let labels_str = labels.join(", ");
                (
                    Severity::Error,
                    "PropertyIsMissing",
                    format!("Missing required properties: {labels_str}"),
                )
            }
            ErrorKind::AdditionalProperty { labels } => {
                let labels_str = labels.join(", ");
                (
                    Severity::Error,
                    "AdditionalProperty",
                    format!("Additional properties not allowed: {labels_str}"),
                )
            }
        };

        let mut diagnostic = match severity {
            Severity::Error => Diagnostic::error(code, message, span, "checking"),
            Severity::Warning => Diagnostic::warning(code, message, span, "checking"),
        };

        if let ErrorKind::NoInstanceFound { given, .. } = &self.kind {
            for &given in given.iter() {
                let given = render_type(given);
                let trivia = format!("{given} is in scope");
                diagnostic = diagnostic.with_trivia(trivia)
            }
        }

        vec![diagnostic]
    }
}
