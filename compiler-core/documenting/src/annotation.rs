use rowan::TextRange;
use rowan::ast::AstNode;
use stabilizing::StabilizedModule;
use syntax::{SyntaxKind, SyntaxNode, SyntaxNodePtr};

use indexing::{DataConstructorId, TermItem, TermItemKind, TypeItem, TypeItemKind};
use parsing::ParsedModule;
use stabilizing::AstId;

pub fn module_documentation(root: &SyntaxNode, parsed: &ParsedModule) -> String {
    let annotation = parsed
        .cst()
        .header()
        .and_then(|header| header.annotation())
        .map(|annotation| annotation.syntax().text_range());

    annotation.map(|range| extract_annotation(root, range)).unwrap_or_default()
}

pub fn term_documentation(
    stabilized: &StabilizedModule,
    root: &SyntaxNode,
    item: &TermItem,
) -> String {
    let range = match &item.kind {
        TermItemKind::ClassMember { id } => {
            signature_equation_range(stabilized, root, &Some(*id), &Some(*id))
        }
        TermItemKind::Constructor { id } => data_constructor_range(stabilized, root, *id),
        TermItemKind::Derive { id } => {
            signature_equation_range(stabilized, root, &Some(*id), &Some(*id))
        }
        TermItemKind::Foreign { id } => {
            signature_equation_range(stabilized, root, &Some(*id), &Some(*id))
        }
        TermItemKind::Instance { id } => {
            signature_equation_range(stabilized, root, &Some(*id), &Some(*id))
        }
        TermItemKind::Operator { id } => {
            signature_equation_range(stabilized, root, &Some(*id), &Some(*id))
        }
        TermItemKind::Value { signature, equations } => {
            let equation = equations.first().copied();
            signature_equation_range(stabilized, root, signature, &equation)
        }
    };

    range
        .and_then(|range| range.annotation)
        .map(|range| extract_annotation(root, range))
        .unwrap_or_default()
}

pub fn type_documentation(
    stabilized: &StabilizedModule,
    root: &SyntaxNode,
    item: &TypeItem,
) -> String {
    let range = match &item.kind {
        TypeItemKind::Data { signature, equation, .. } => {
            signature_equation_range(stabilized, root, signature, equation)
        }
        TypeItemKind::Newtype { signature, equation, .. } => {
            signature_equation_range(stabilized, root, signature, equation)
        }
        TypeItemKind::Synonym { signature, equation, .. } => {
            signature_equation_range(stabilized, root, signature, equation)
        }
        TypeItemKind::Class { signature, declaration, .. } => {
            signature_equation_range(stabilized, root, signature, declaration)
        }
        TypeItemKind::Foreign { id, .. } => {
            signature_equation_range(stabilized, root, &Some(*id), &Some(*id))
        }
        TypeItemKind::Operator { id } => {
            signature_equation_range(stabilized, root, &Some(*id), &Some(*id))
        }
    };

    range
        .and_then(|range| range.annotation)
        .map(|range| extract_annotation(root, range))
        .unwrap_or_default()
}

#[derive(Debug, Default)]
struct AnnotationRange {
    annotation: Option<TextRange>,
}

impl AnnotationRange {
    fn from_ptr(root: &SyntaxNode, ptr: &SyntaxNodePtr) -> Option<AnnotationRange> {
        ptr.try_to_node(root).map(|node| Self::from_node(&node))
    }

    fn from_node(node: &SyntaxNode) -> AnnotationRange {
        let mut children = node.children_with_tokens().peekable();

        let annotation = children.next_if(|child| {
            let kind = child.kind();
            matches!(kind, SyntaxKind::Annotation)
        });

        AnnotationRange { annotation: annotation.map(|child| child.text_range()) }
    }
}

fn signature_equation_range<S, E>(
    stabilized: &StabilizedModule,
    root: &SyntaxNode,
    signature: &Option<AstId<S>>,
    equation: &Option<AstId<E>>,
) -> Option<AnnotationRange>
where
    S: AstNode<Language = syntax::PureScript>,
    E: AstNode<Language = syntax::PureScript>,
{
    let signature = signature.and_then(|id| {
        let ptr = stabilized.syntax_ptr(id)?;
        AnnotationRange::from_ptr(root, &ptr)
    });

    let equation = || {
        let id = equation.as_ref()?;
        let ptr = stabilized.syntax_ptr(*id)?;
        AnnotationRange::from_ptr(root, &ptr)
    };

    signature.or_else(equation)
}

fn data_constructor_range(
    stabilized: &StabilizedModule,
    root: &SyntaxNode,
    id: DataConstructorId,
) -> Option<AnnotationRange> {
    let ptr = stabilized.syntax_ptr(id)?;
    let node = ptr.try_to_node(root)?;

    if let Some(annotation) = AnnotationRange::from_node(&node).annotation
        && contains_documentation(root, annotation)
    {
        return Some(AnnotationRange { annotation: Some(annotation) });
    }

    let separator = node.prev_sibling_or_token()?;
    if !matches!(separator.kind(), SyntaxKind::EQUAL | SyntaxKind::PIPE) {
        return None;
    }

    let annotation = separator.prev_sibling_or_token()?;
    if !matches!(annotation.kind(), SyntaxKind::Annotation) {
        return None;
    }

    let annotation = annotation.text_range();
    if contains_documentation(root, annotation) {
        Some(AnnotationRange { annotation: Some(annotation) })
    } else {
        None
    }
}

fn contains_documentation(root: &SyntaxNode, range: TextRange) -> bool {
    let text = root.text().slice(range).to_string();
    text.lines().any(|line| documentation_line_content(line).is_some())
}

fn documentation_line_content(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let line = line.strip_prefix("--")?;
    let line = line.trim_start_matches(' ');
    let line = line.strip_prefix('|')?;
    let line = line.strip_prefix(' ').unwrap_or(line);
    let line = line.trim_end();
    Some(line)
}

fn extract_annotation(root: &SyntaxNode, range: TextRange) -> String {
    let text = root.text().slice(range).to_string();

    let mut annotation = String::default();

    let lines = text.lines().filter_map(documentation_line_content);

    let mut lines = lines.peekable();
    if let Some(line) = lines.next() {
        annotation.push_str(line);
    }

    lines.for_each(|line| {
        annotation.push('\n');
        annotation.push_str(line);
    });

    annotation
}

#[cfg(test)]
mod tests {
    use indexing::TermItemKind;

    use super::*;

    #[test]
    fn data_constructor_documentation_before_separators() {
        let source = r#"module Main where

data Maybe a
  -- | `Nothing` is `null`.
  = Nothing
  -- | `Just x` is the non-null value `x`.
  | Just a
"#;

        let lexed = lexing::lex(source);
        let tokens = lexing::layout(&lexed);
        let (parsed, errors) = parsing::parse(&lexed, &tokens);
        assert!(errors.is_empty());

        let root = parsed.syntax_node();
        let cst = parsed.cst();

        let stabilized = stabilizing::stabilize_module(&root);
        let indexed = indexing::index_module(&cst, &stabilized);

        let documentation = indexed.items.iter_terms().filter_map(|(_, item)| {
            if !matches!(item.kind, TermItemKind::Constructor { .. }) {
                return None;
            }

            let name = item.name.as_deref()?;
            let documentation = term_documentation(&stabilized, &root, item);

            Some((name.to_string(), documentation))
        });

        let documentation: Vec<_> = documentation.collect();

        insta::assert_debug_snapshot!(documentation, @r###"
        [
            (
                "Nothing",
                "`Nothing` is `null`.",
            ),
            (
                "Just",
                "`Just x` is the non-null value `x`.",
            ),
        ]
        "###);
    }
}
