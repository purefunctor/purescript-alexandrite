use rowan::TextRange;
use rowan::ast::AstNode;
use stabilizing::StabilizedModule;
use syntax::{SyntaxKind, SyntaxNode, SyntaxNodePtr};

use indexing::{TermItem, TermItemKind, TypeItem, TypeItemKind};
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
        TermItemKind::Constructor { id } => {
            signature_equation_range(stabilized, root, &Some(*id), &Some(*id))
        }
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

fn extract_annotation(root: &SyntaxNode, range: TextRange) -> String {
    let text = root.text().slice(range);

    let mut annotation = String::default();

    text.for_each_chunk(|chunk| {
        let lines = chunk.lines().filter_map(|line| {
            let trimmed = line.trim_start_matches("-- |");
            if line != trimmed { Some(trimmed.trim_matches(' ')) } else { None }
        });

        let mut lines = lines.peekable();
        if let Some(line) = lines.next() {
            annotation.push_str(line);
        }

        lines.for_each(|line| {
            annotation.push('\n');
            annotation.push_str(line);
        });
    });

    annotation
}
