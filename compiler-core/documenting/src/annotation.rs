use rowan::ast::AstNode;
use rowan::{NodeOrToken, WalkEvent};
use rustc_hash::FxHashMap;
use stabilizing::StabilizedModule;
use syntax::{SyntaxKind, SyntaxNode, SyntaxNodePtr};

use indexing::{DataConstructorId, TermItem, TermItemKind, TypeItem, TypeItemKind};
use parsing::ParsedModule;
use stabilizing::AstId;

pub struct AnnotationIndex {
    documentation: FxHashMap<SyntaxNodePtr, String>,
    constructors: FxHashMap<SyntaxNodePtr, String>,
}

impl AnnotationIndex {
    pub fn new(root: &SyntaxNode) -> AnnotationIndex {
        let mut documentation = FxHashMap::default();
        let mut constructors = FxHashMap::default();

        for event in root.preorder() {
            let WalkEvent::Enter(node) = event else { continue };
            let ptr = SyntaxNodePtr::new(&node);

            if let Some(text) = first_child_documentation(&node) {
                documentation.insert(ptr, text);
            }

            if matches!(node.kind(), SyntaxKind::DataConstructor)
                && let Some(text) = data_constructor_documentation(&node)
            {
                constructors.insert(ptr, text);
            }
        }

        AnnotationIndex { documentation, constructors }
    }

    fn documentation(&self, ptr: SyntaxNodePtr) -> &str {
        self.documentation.get(&ptr).map(String::as_str).unwrap_or_default()
    }

    fn data_constructor_documentation(&self, ptr: SyntaxNodePtr) -> &str {
        self.constructors.get(&ptr).map(String::as_str).unwrap_or_default()
    }
}

pub fn module_documentation(parsed: &ParsedModule) -> String {
    parsed
        .cst()
        .header()
        .and_then(|header| header.annotation())
        .and_then(|annotation| annotation_documentation(annotation.syntax()))
        .unwrap_or_default()
}

pub fn term_documentation(
    stabilized: &StabilizedModule,
    annotations: &AnnotationIndex,
    item: &TermItem,
) -> String {
    match &item.kind {
        TermItemKind::ClassMember { id } => {
            signature_equation_documentation(stabilized, annotations, &Some(*id), &Some(*id))
        }
        TermItemKind::Constructor { id } => {
            data_constructor_item_documentation(stabilized, annotations, *id)
        }
        TermItemKind::Derive { id } => {
            signature_equation_documentation(stabilized, annotations, &Some(*id), &Some(*id))
        }
        TermItemKind::Foreign { id } => {
            signature_equation_documentation(stabilized, annotations, &Some(*id), &Some(*id))
        }
        TermItemKind::Instance { id } => {
            signature_equation_documentation(stabilized, annotations, &Some(*id), &Some(*id))
        }
        TermItemKind::Operator { id } => {
            signature_equation_documentation(stabilized, annotations, &Some(*id), &Some(*id))
        }
        TermItemKind::Value { signature, equations } => {
            let equation = equations.first().copied();
            signature_equation_documentation(stabilized, annotations, signature, &equation)
        }
    }
}

pub fn type_documentation(
    stabilized: &StabilizedModule,
    annotations: &AnnotationIndex,
    item: &TypeItem,
) -> String {
    match &item.kind {
        TypeItemKind::Data { signature, equation, .. } => {
            signature_equation_documentation(stabilized, annotations, signature, equation)
        }
        TypeItemKind::Newtype { signature, equation, .. } => {
            signature_equation_documentation(stabilized, annotations, signature, equation)
        }
        TypeItemKind::Synonym { signature, equation, .. } => {
            signature_equation_documentation(stabilized, annotations, signature, equation)
        }
        TypeItemKind::Class { signature, declaration, .. } => {
            signature_equation_documentation(stabilized, annotations, signature, declaration)
        }
        TypeItemKind::Foreign { id, .. } => {
            signature_equation_documentation(stabilized, annotations, &Some(*id), &Some(*id))
        }
        TypeItemKind::Operator { id } => {
            signature_equation_documentation(stabilized, annotations, &Some(*id), &Some(*id))
        }
    }
}

fn signature_equation_documentation<S, E>(
    stabilized: &StabilizedModule,
    annotations: &AnnotationIndex,
    signature: &Option<AstId<S>>,
    equation: &Option<AstId<E>>,
) -> String
where
    S: AstNode<Language = syntax::PureScript>,
    E: AstNode<Language = syntax::PureScript>,
{
    if let Some(id) = signature
        && let Some(ptr) = stabilized.syntax_ptr(*id)
    {
        let documentation = annotations.documentation(ptr);
        if !documentation.is_empty() {
            return documentation.to_owned();
        }
    }

    if let Some(id) = equation
        && let Some(ptr) = stabilized.syntax_ptr(*id)
    {
        return annotations.documentation(ptr).to_owned();
    }

    String::default()
}

fn data_constructor_item_documentation(
    stabilized: &StabilizedModule,
    annotations: &AnnotationIndex,
    id: DataConstructorId,
) -> String {
    stabilized
        .syntax_ptr(id)
        .map(|ptr| annotations.data_constructor_documentation(ptr).to_owned())
        .unwrap_or_default()
}

fn data_constructor_documentation(node: &SyntaxNode) -> Option<String> {
    if let Some(documentation) = first_child_documentation(node) {
        return Some(documentation);
    }

    let separator = node.prev_sibling_or_token()?;
    if !matches!(separator.kind(), SyntaxKind::EQUAL | SyntaxKind::PIPE) {
        return None;
    }

    let annotation = separator.prev_sibling_or_token()?;
    match annotation {
        NodeOrToken::Node(node) => annotation_documentation(&node),
        NodeOrToken::Token(_) => None,
    }
}

fn first_child_documentation(node: &SyntaxNode) -> Option<String> {
    let first_child = node.children_with_tokens().next()?;
    match first_child {
        NodeOrToken::Node(node) => annotation_documentation(&node),
        NodeOrToken::Token(_) => None,
    }
}

fn annotation_documentation(node: &SyntaxNode) -> Option<String> {
    if !matches!(node.kind(), SyntaxKind::Annotation) {
        return None;
    }

    let text = node.first_token()?.text().to_owned();
    extract_annotation(&text)
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

fn extract_annotation(text: &str) -> Option<String> {
    let mut annotation = String::default();

    let lines = text.lines().filter_map(documentation_line_content);

    let mut lines = lines.peekable();
    {
        let line = lines.next()?;
        annotation.push_str(line);
    }

    lines.for_each(|line| {
        annotation.push('\n');
        annotation.push_str(line);
    });

    Some(annotation)
}

#[cfg(test)]
mod tests {
    use indexing::TermItemKind;

    use super::*;

    #[test]
    fn value_equation_documentation_used_when_signature_has_no_documentation() {
        let source = r#"module Main where

value :: Int
-- | Equation documentation.
value = 1
"#;

        let lexed = lexing::lex(source);
        let tokens = lexing::layout(&lexed);
        let (parsed, errors) = parsing::parse(&lexed, &tokens);
        assert!(errors.is_empty());

        let root = parsed.syntax_node();
        let cst = parsed.cst();

        let stabilized = stabilizing::stabilize_module(&root);
        let indexed = indexing::index_module(&cst, &stabilized);
        let annotations = AnnotationIndex::new(&root);

        let id = indexed.names.terms.lookup("value").unwrap();
        let item = &indexed.items[id];
        assert!(matches!(item.kind, TermItemKind::Value { .. }));

        let documentation = term_documentation(&stabilized, &annotations, item);
        assert_eq!(documentation, "Equation documentation.");
    }

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
        let annotations = AnnotationIndex::new(&root);

        let documentation = indexed.items.iter_terms().filter_map(|(_, item)| {
            if !matches!(item.kind, TermItemKind::Constructor { .. }) {
                return None;
            }

            let name = item.name.as_deref()?;
            let documentation = term_documentation(&stabilized, &annotations, item);

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
