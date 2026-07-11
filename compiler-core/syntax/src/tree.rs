use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

use syntree::pointer::PointerUsize;
pub use text_size::{TextRange, TextSize};

use crate::SyntaxKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementCategory {
    Node,
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxValue {
    pub kind: SyntaxKind,
    pub category: ElementCategory,
}

pub type Syntree = syntree::Tree<SyntaxValue, syntree::FlavorDefault>;

#[derive(Debug)]
pub struct TreeOwner {
    tree: RwLock<Syntree>,
    root: PointerUsize,
}

impl TreeOwner {
    pub fn new(tree: Syntree) -> Arc<Self> {
        let root = tree.first().expect("syntax tree must have a root").id();
        Arc::new(Self { tree: RwLock::new(tree), root })
    }
}
impl PartialEq for TreeOwner {
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        *self.tree.read().unwrap() == *other.tree.read().unwrap()
    }
}
impl Eq for TreeOwner {}

#[derive(Clone)]
pub struct SyntaxNode {
    owner: Arc<TreeOwner>,
    id: PointerUsize,
}
#[derive(Clone)]
pub struct SyntaxToken {
    owner: Arc<TreeOwner>,
    id: PointerUsize,
}

macro_rules! handle_impls {
    ($ty:ident) => {
        impl PartialEq for $ty {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.owner, &other.owner) && self.id == other.id
            }
        }
        impl Eq for $ty {}
        impl Hash for $ty {
            fn hash<H: Hasher>(&self, state: &mut H) {
                Arc::as_ptr(&self.owner).hash(state);
                self.id.hash(state);
            }
        }
    };
}
handle_impls!(SyntaxNode);
handle_impls!(SyntaxToken);

fn range(node: &syntree::Node<'_, SyntaxValue, syntree::FlavorDefault>) -> TextRange {
    TextRange::new(node.span().start.into(), node.span().end.into())
}
fn element(owner: &Arc<TreeOwner>, id: PointerUsize, value: SyntaxValue) -> SyntaxElement {
    match value.category {
        ElementCategory::Node => SyntaxNode { owner: owner.clone(), id }.into(),
        ElementCategory::Token => SyntaxToken { owner: owner.clone(), id }.into(),
    }
}

impl SyntaxNode {
    pub fn new_root(owner: Arc<TreeOwner>) -> Self {
        let id = owner.root;
        Self { owner, id }
    }
    pub fn owner(&self) -> Arc<TreeOwner> {
        self.owner.clone()
    }
    pub fn kind(&self) -> SyntaxKind {
        self.owner.tree.read().unwrap().get(self.id).unwrap().value().kind
    }
    pub fn text_range(&self) -> TextRange {
        range(&self.owner.tree.read().unwrap().get(self.id).unwrap())
    }
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[usize::from(self.text_range().start())..usize::from(self.text_range().end())]
    }
    pub fn parent(&self) -> Option<Self> {
        let id = self.owner.tree.read().unwrap().get(self.id)?.parent()?.id();
        Some(Self { owner: self.owner.clone(), id })
    }
    pub fn ancestors(&self) -> impl Iterator<Item = Self> {
        std::iter::successors(Some(self.clone()), Self::parent)
    }
    pub fn parent_ancestors(&self) -> impl Iterator<Item = Self> {
        std::iter::successors(self.parent(), Self::parent)
    }
    pub fn children(&self) -> SyntaxNodeChildren {
        SyntaxNodeChildren(self.elements(false).into_iter())
    }
    pub fn children_with_tokens(&self) -> SyntaxElementChildren {
        SyntaxElementChildren(self.elements(true).into_iter())
    }
    fn elements(&self, tokens: bool) -> Vec<SyntaxElement> {
        let tree = self.owner.tree.read().unwrap();
        tree.get(self.id)
            .unwrap()
            .children()
            .filter_map(|node| {
                let value = node.value();
                (tokens || value.category == ElementCategory::Node)
                    .then(|| element(&self.owner, node.id(), value))
            })
            .collect()
    }
    pub fn first_child(&self) -> Option<Self> {
        self.children().next()
    }
    pub fn first_token(&self) -> Option<SyntaxToken> {
        self.preorder_with_tokens().find_map(|event| match event {
            WalkEvent::Enter(element) => element.into_token(),
            _ => None,
        })
    }
    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement> {
        sibling(&self.owner, self.id, true)
    }
    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement> {
        sibling(&self.owner, self.id, false)
    }
    pub fn next_sibling(&self) -> Option<Self> {
        node_sibling(self, true)
    }
    pub fn prev_sibling(&self) -> Option<Self> {
        node_sibling(self, false)
    }
    pub fn preorder(&self) -> Preorder {
        Preorder(
            self.preorder_with_tokens()
                .filter_map(|event| match event {
                    WalkEvent::Enter(e) => e.into_node().map(WalkEvent::Enter),
                    WalkEvent::Leave(e) => e.into_node().map(WalkEvent::Leave),
                })
                .collect::<Vec<_>>()
                .into_iter(),
        )
    }
    pub fn preorder_with_tokens(&self) -> PreorderWithTokens {
        let tree = self.owner.tree.read().unwrap();
        let mut events = Vec::new();
        collect_events(&self.owner, tree.get(self.id).unwrap(), &mut events);
        PreorderWithTokens(events.into_iter())
    }
    pub fn token_at_offset(&self, offset: TextSize) -> TokenAtOffset<SyntaxToken> {
        token_at_offset(self, offset)
    }
    pub fn debug<'a>(&'a self, source: &'a str) -> DebugSyntax<'a> {
        DebugSyntax { node: self, source }
    }
}

impl SyntaxToken {
    pub fn kind(&self) -> SyntaxKind {
        self.owner.tree.read().unwrap().get(self.id).unwrap().value().kind
    }
    pub fn text_range(&self) -> TextRange {
        range(&self.owner.tree.read().unwrap().get(self.id).unwrap())
    }
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[usize::from(self.text_range().start())..usize::from(self.text_range().end())]
    }
    pub fn parent(&self) -> SyntaxNode {
        let id = self.owner.tree.read().unwrap().get(self.id).unwrap().parent().unwrap().id();
        SyntaxNode { owner: self.owner.clone(), id }
    }
    pub fn parent_ancestors(&self) -> impl Iterator<Item = SyntaxNode> {
        std::iter::successors(Some(self.parent()), SyntaxNode::parent)
    }
    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement> {
        sibling(&self.owner, self.id, true)
    }
    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement> {
        sibling(&self.owner, self.id, false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SyntaxElement {
    Node(SyntaxNode),
    Token(SyntaxToken),
}
impl SyntaxElement {
    pub fn into_node(self) -> Option<SyntaxNode> {
        if let Self::Node(value) = self { Some(value) } else { None }
    }
    pub fn into_token(self) -> Option<SyntaxToken> {
        if let Self::Token(value) = self { Some(value) } else { None }
    }
    pub fn as_node(&self) -> Option<&SyntaxNode> {
        if let Self::Node(value) = self { Some(value) } else { None }
    }
    pub fn as_token(&self) -> Option<&SyntaxToken> {
        if let Self::Token(value) = self { Some(value) } else { None }
    }
    pub fn kind(&self) -> SyntaxKind {
        match self {
            Self::Node(n) => n.kind(),
            Self::Token(t) => t.kind(),
        }
    }
    pub fn text_range(&self) -> TextRange {
        match self {
            Self::Node(n) => n.text_range(),
            Self::Token(t) => t.text_range(),
        }
    }
}
impl From<SyntaxNode> for SyntaxElement {
    fn from(value: SyntaxNode) -> Self {
        Self::Node(value)
    }
}
impl From<SyntaxToken> for SyntaxElement {
    fn from(value: SyntaxToken) -> Self {
        Self::Token(value)
    }
}

pub struct SyntaxNodeChildren(std::vec::IntoIter<SyntaxElement>);
impl Iterator for SyntaxNodeChildren {
    type Item = SyntaxNode;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.find_map(SyntaxElement::into_node)
    }
}
pub struct SyntaxElementChildren(std::vec::IntoIter<SyntaxElement>);
impl Iterator for SyntaxElementChildren {
    type Item = SyntaxElement;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkEvent<T> {
    Enter(T),
    Leave(T),
}
pub struct Preorder(std::vec::IntoIter<WalkEvent<SyntaxNode>>);
impl Iterator for Preorder {
    type Item = WalkEvent<SyntaxNode>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
pub struct PreorderWithTokens(std::vec::IntoIter<WalkEvent<SyntaxElement>>);
impl Iterator for PreorderWithTokens {
    type Item = WalkEvent<SyntaxElement>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
fn collect_events(
    owner: &Arc<TreeOwner>,
    node: syntree::Node<'_, SyntaxValue, syntree::FlavorDefault>,
    out: &mut Vec<WalkEvent<SyntaxElement>>,
) {
    let value = node.value();
    let current = element(owner, node.id(), value);
    out.push(WalkEvent::Enter(current.clone()));
    for child in node.children() {
        collect_events(owner, child, out);
    }
    out.push(WalkEvent::Leave(current));
}
fn sibling(owner: &Arc<TreeOwner>, id: PointerUsize, next: bool) -> Option<SyntaxElement> {
    let tree = owner.tree.read().unwrap();
    let node = tree.get(id)?;
    let node = if next { node.next()? } else { node.prev()? };
    Some(element(owner, node.id(), node.value()))
}
fn node_sibling(node: &SyntaxNode, next: bool) -> Option<SyntaxNode> {
    let mut element =
        if next { node.next_sibling_or_token() } else { node.prev_sibling_or_token() };
    loop {
        match element? {
            SyntaxElement::Node(node) => return Some(node),
            SyntaxElement::Token(token) => {
                element =
                    if next { token.next_sibling_or_token() } else { token.prev_sibling_or_token() }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenAtOffset<T> {
    None,
    Single(T),
    Between(T, T),
}
impl<T> TokenAtOffset<T> {
    pub fn left_biased(self) -> Option<T> {
        match self {
            Self::None => None,
            Self::Single(token) | Self::Between(token, _) => Some(token),
        }
    }
    pub fn right_biased(self) -> Option<T> {
        match self {
            Self::None => None,
            Self::Single(token) | Self::Between(_, token) => Some(token),
        }
    }
}
fn token_at_offset(node: &SyntaxNode, offset: TextSize) -> TokenAtOffset<SyntaxToken> {
    let tokens = node.preorder_with_tokens().filter_map(|event| match event {
        WalkEvent::Enter(element) => element.into_token(),
        _ => None,
    });
    let mut left = None;
    for token in tokens {
        let range = token.text_range();
        if !range.is_empty() && range.start() == offset {
            return left
                .filter(|token: &SyntaxToken| {
                    !token.text_range().is_empty() && token.text_range().end() == offset
                })
                .map_or(TokenAtOffset::Single(token.clone()), |left| {
                    TokenAtOffset::Between(left, token)
                });
        }
        if range.contains(offset) {
            return TokenAtOffset::Single(token);
        }
        if !range.is_empty() && range.end() == offset {
            left = Some(token);
        }
    }
    left.map_or(TokenAtOffset::None, TokenAtOffset::Single)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxNodePtr {
    id: PointerUsize,
    kind: SyntaxKind,
    range: TextRange,
}
impl SyntaxNodePtr {
    pub fn new(node: &SyntaxNode) -> Self {
        Self { id: node.id, kind: node.kind(), range: node.text_range() }
    }
    pub fn try_to_node(&self, root: &SyntaxNode) -> Option<SyntaxNode> {
        let tree = root.owner.tree.read().unwrap();
        let node = tree.get(self.id)?;
        (node.value().category == ElementCategory::Node
            && node.value().kind == self.kind
            && range(&node) == self.range)
            .then(|| SyntaxNode { owner: root.owner.clone(), id: self.id })
    }
    pub fn to_node(&self, root: &SyntaxNode) -> SyntaxNode {
        self.try_to_node(root).expect("syntax pointer does not belong to the supplied tree")
    }
    pub fn cast<N: crate::ast::AstNode>(self) -> Option<crate::ast::AstPtr<N>> {
        N::can_cast(self.kind).then(|| crate::ast::AstPtr::from_raw(self))
    }
}

pub struct DebugSyntax<'a> {
    node: &'a SyntaxNode,
    source: &'a str,
}
impl fmt::Debug for DebugSyntax<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write(
            node: &SyntaxNode,
            source: &str,
            f: &mut fmt::Formatter<'_>,
            depth: usize,
        ) -> fmt::Result {
            writeln!(
                f,
                "{:indent$}{:?}@{:?}",
                "",
                node.kind(),
                node.text_range(),
                indent = depth * 2
            )?;
            for element in node.children_with_tokens() {
                match element {
                    SyntaxElement::Node(node) => write(&node, source, f, depth + 1)?,
                    SyntaxElement::Token(token) => {
                        let text = token.text(source);
                        if text.len() < 25 {
                            writeln!(
                                f,
                                "{:indent$}{:?}@{:?} {:?}",
                                "",
                                token.kind(),
                                token.text_range(),
                                text,
                                indent = (depth + 1) * 2
                            )?;
                        } else {
                            let end = (21..25).find(|&index| text.is_char_boundary(index)).unwrap();
                            let text = format!("{} ...", &text[..end]);
                            writeln!(
                                f,
                                "{:indent$}{:?}@{:?} {:?}",
                                "",
                                token.kind(),
                                token.text_range(),
                                text,
                                indent = (depth + 1) * 2
                            )?;
                        }
                    }
                }
            }
            Ok(())
        }
        write(self.node, self.source, f, 0)
    }
}
impl fmt::Debug for SyntaxNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyntaxNode")
            .field("kind", &self.kind())
            .field("range", &self.text_range())
            .finish()
    }
}
impl fmt::Debug for SyntaxToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyntaxToken")
            .field("kind", &self.kind())
            .field("range", &self.text_range())
            .finish()
    }
}
