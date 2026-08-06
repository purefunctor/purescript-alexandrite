use building_types::QueryProxy;
use files::FileId;
use lowering::{GraphNodeId, LoweredModule};
use lsp_types::*;
use parsing::ParsedModule;
use resolving::ResolvedModule;
use smol_str::SmolStr;
use stabilizing::StabilizedModule;
use syntax::ast::{AstNode, AstPtr};
use syntax::{
    SyntaxKind, SyntaxNode, SyntaxNodePtr, SyntaxToken, TextRange, TextSize, TokenAtOffset, cst,
};

use crate::position::PositionEncoding;
use crate::{AnalyzerContext, AnalyzerError, position};

pub struct CompletionContext<'c, 'a, Host> {
    pub language: &'c AnalyzerContext<'c, Host>,
    pub current_file: FileId,
    pub content: &'a str,
    pub stabilized: &'a StabilizedModule,
    pub parsed: &'a ParsedModule,
    pub resolved: &'a ResolvedModule,

    pub prim_id: FileId,
    pub prim_resolved: &'a ResolvedModule,

    pub semantics: CursorSemantics,
    pub text: CursorText,
    pub range: Option<Range>,
    pub offset: TextSize,
}

impl<Host: crate::AnalyzerHost> CompletionContext<'_, '_, Host> {
    pub fn insert_import_range(&self) -> Option<Range> {
        let cst = self.parsed.cst();

        let range = cst.imports().map_or_else(
            || {
                let header = cst.header()?;
                Some(header.syntax().text_range())
            },
            |cst| Some(cst.syntax().text_range()),
        )?;

        let mut position = position::offset_to_utf8_position(self.content, range.end())?;

        position.line += 1;
        position.column = 0;

        let position = position::utf8_position_to_protocol(
            self.content,
            position,
            self.language.position_encoding(),
        )?;
        Some(Range::new(position, position))
    }

    pub fn collect_modules(&self) -> bool {
        matches!(self.semantics, CursorSemantics::Module)
    }

    pub fn collect_terms(&self) -> bool {
        matches!(self.semantics, CursorSemantics::Term)
    }

    pub fn collect_types(&self) -> bool {
        matches!(self.semantics, CursorSemantics::Type)
    }

    pub fn collect_import_classes(&self) -> bool {
        matches!(self.semantics, CursorSemantics::ImportClass)
    }

    pub fn import_file_at_cursor(&self) -> Option<FileId> {
        let root = self.parsed.syntax_node();
        let token = match root.token_at_offset(self.offset) {
            TokenAtOffset::None => return None,
            TokenAtOffset::Single(token) => token,
            TokenAtOffset::Between(left, right) => {
                let left_is_import_class =
                    left.parent_ancestors().any(|node| cst::ImportClass::can_cast(node.kind()));
                if left_is_import_class { left } else { right }
            }
        };

        let statement = token.parent_ancestors().find_map(cst::ImportStatement::cast)?;
        let module_name = statement.module_name()?.syntax().text(self.content);
        self.language.queries().module_file(&module_name)
    }

    pub fn collect_implicit_prim(&self) -> bool {
        self.resolved.unqualified.values().flatten().all(|import| import.file != self.prim_id)
    }

    pub fn has_qualified_import(&self, name: &str) -> bool {
        self.resolved.qualified.contains_key(name)
    }

    pub fn has_term_import(&self, qualifier: Option<&str>, name: &str) -> bool {
        self.resolved.lookup_term(self.prim_resolved, qualifier, name).is_some()
    }

    pub fn has_type_import(&self, qualifier: Option<&str>, name: &str) -> bool {
        self.resolved.lookup_type(self.prim_resolved, qualifier, name).is_some()
            || self.resolved.lookup_class(self.prim_resolved, qualifier, name).is_some()
    }

    pub fn scope_node(&self) -> Result<Option<GraphNodeId>, AnalyzerError> {
        let lowered = self.language.queries().lowered(self.current_file)?;
        let root = self.parsed.syntax_node();

        let scope_node = match root.token_at_offset(self.offset) {
            TokenAtOffset::None => None,
            TokenAtOffset::Single(token) => self.scope_node_for_token(&lowered, token),
            TokenAtOffset::Between(left, right) => {
                let left = self.scope_node_for_token(&lowered, left);
                let right = self.scope_node_for_token(&lowered, right);

                if left == right { left } else { right.or(left) }
            }
        };

        Ok(scope_node)
    }

    fn scope_node_for_token(
        &self,
        lowered: &LoweredModule,
        token: SyntaxToken,
    ) -> Option<GraphNodeId> {
        token.parent_ancestors().find_map(|node| self.scope_node_for_syntax(lowered, node))
    }

    fn scope_node_for_syntax(
        &self,
        lowered: &LoweredModule,
        node: SyntaxNode,
    ) -> Option<GraphNodeId> {
        let kind = node.kind();
        let ptr = SyntaxNodePtr::new(&node);

        if cst::Binder::can_cast(kind) {
            let ptr = ptr.cast()?;
            let id = self.stabilized.lookup_ptr(&ptr)?;
            lowered.nodes.binder_node(id)
        } else if cst::Expression::can_cast(kind) {
            let ptr = ptr.cast()?;
            let id = self.stabilized.lookup_ptr(&ptr)?;
            lowered.nodes.expression_node(id)
        } else if cst::Type::can_cast(kind) {
            let ptr = ptr.cast()?;
            let id = self.stabilized.lookup_ptr(&ptr)?;
            lowered.nodes.type_node(id)
        } else if cst::LetBinding::can_cast(kind) {
            let binding = cst::LetBinding::cast(node)?;
            let id = match binding {
                cst::LetBinding::LetBindingPattern(_) => None,
                cst::LetBinding::LetBindingSignature(signature) => {
                    let ptr = AstPtr::new(&signature);
                    let id = self.stabilized.lookup_ptr(&ptr)?;
                    lowered.tree.find_let_binding_group_by_signature(id)
                }
                cst::LetBinding::LetBindingEquation(equation) => {
                    let ptr = AstPtr::new(&equation);
                    let id = self.stabilized.lookup_ptr(&ptr)?;
                    lowered.tree.find_let_binding_group_by_equation(id)
                }
            }?;

            lowered.nodes.let_node(id)
        } else {
            None
        }
    }
}

/// A trait for completion sources.
pub trait CompletionSource {
    type T;

    fn collect_into<F: Filter>(
        &self,
        context: &CompletionContext<impl crate::AnalyzerHost>,
        filter: F,
        items: &mut Vec<CompletionItem>,
    ) -> Result<Self::T, AnalyzerError>;
}

/// A trait for describing completion filters.
pub trait Filter: Copy {
    fn matches(&self, name: &str) -> bool;
}

#[derive(Debug)]
pub enum CursorSemantics {
    Term,
    Type,
    Module,
    ImportClass,
    General,
    Comment,
}

const COMPLETION_MARKER: &str = "Z'PureScript'Z";

impl CursorSemantics {
    pub fn new(content: &str, offset: TextSize, token: &SyntaxToken) -> CursorSemantics {
        // We insert a placeholder identifier at the current position of the
        // text cursor. This is done as an effort to produce as valid of a
        // parse tree as possible before we perform further analysis.
        //
        // This is particularly helpful for incomplete qualified names. Since
        // the parser represents qualifiers as "trivia" for the current token,
        // the following source string yields a lexing error:
        //
        // component = Halogen.
        //
        // Inserting a placeholder gets rid of this error, allowing the parser
        // to produce a valid parse tree that we can use for analysis:
        //
        // component = Halogen.Z'PureScript'Z

        let at_import_class_keyword = token.kind() == SyntaxKind::CLASS
            && token.text_range().end() == offset
            && token.parent_ancestors().any(|node| {
                cst::ImportClass::can_cast(node.kind()) || cst::ImportList::can_cast(node.kind())
            });
        if at_import_class_keyword {
            return CursorSemantics::ImportClass;
        }

        let (left, right) = content.split_at(offset.into());
        let source = format!("{left}{COMPLETION_MARKER}{right}");

        let lexed = lexing::lex(&source);
        let tokens = lexing::layout(&lexed);
        let (parsed, _) = parsing::parse(&lexed, &tokens);

        let node = parsed.syntax_node();
        let token = node.token_at_offset(offset);

        let token = match token {
            TokenAtOffset::None => {
                return CursorSemantics::General;
            }
            TokenAtOffset::Single(token) => token,
            TokenAtOffset::Between(left, right) => {
                if left.text(&source).contains(COMPLETION_MARKER) {
                    left
                } else if right.text(&source).contains(COMPLETION_MARKER) {
                    right
                } else {
                    return CursorSemantics::General;
                }
            }
        };

        token
            .parent_ancestors()
            .find_map(|node| {
                let kind = node.kind();
                if cst::Annotation::can_cast(kind) {
                    Some(CursorSemantics::Comment)
                } else if cst::Expression::can_cast(kind) {
                    Some(CursorSemantics::Term)
                } else if cst::Type::can_cast(kind) || cst::ExpressionTypeArgument::can_cast(kind) {
                    Some(CursorSemantics::Type)
                } else if cst::ImportClass::can_cast(kind) {
                    Some(CursorSemantics::ImportClass)
                } else if cst::ImportStatement::can_cast(kind) {
                    Some(CursorSemantics::Module)
                } else {
                    None
                }
            })
            .unwrap_or(CursorSemantics::General)
    }
}

#[derive(Debug)]
pub enum CursorText {
    ImportClassBoundary { name: Option<SmolStr>, name_range: Option<Range>, insert_space: bool },
    None,
    Prefix(SmolStr),
    Name(SmolStr),
    Both(SmolStr, SmolStr),
}

impl CursorText {
    pub fn new(
        content: &str,
        token: &SyntaxToken,
        offset: TextSize,
        encoding: PositionEncoding,
    ) -> (CursorText, Option<Range>) {
        CursorText::of_qualified(content, token, encoding)
            .or_else(|| CursorText::of_qualifier(content, token, encoding))
            .or_else(|| CursorText::of_import_class(content, token, offset, encoding))
            .or_else(|| CursorText::of_module_name(content, token, encoding))
            .unwrap_or((CursorText::None, None))
    }

    fn of_import_class(
        content: &str,
        token: &SyntaxToken,
        offset: TextSize,
        encoding: PositionEncoding,
    ) -> Option<(CursorText, Option<Range>)> {
        token.parent_ancestors().find_map(|node| {
            let import = cst::ImportClass::cast(node.clone());
            if import.is_none()
                && token.kind() == SyntaxKind::CLASS
                && token.text_range().end() == offset
                && cst::ImportList::can_cast(node.kind())
            {
                let range = TextRange::new(offset, offset);
                let range = position::text_range_to_protocol(content, range, encoding)?;
                let text = CursorText::ImportClassBoundary {
                    name: None,
                    name_range: None,
                    insert_space: true,
                };
                return Some((text, Some(range)));
            }
            let import = import?;
            let name_token = import.name_token();
            let name_start = name_token.as_ref().map(|token| token.text_range().start());

            if name_start.is_none_or(|name_start| offset <= name_start) {
                let class_boundary =
                    token.kind() == SyntaxKind::CLASS && token.text_range().end() == offset;
                let trivia =
                    name_start.map(|name_start| &content[TextRange::new(offset, name_start)]);
                let replace_inline = trivia.is_some_and(|trivia| {
                    !trivia.contains(['\n', '\r']) && trivia.chars().all(char::is_whitespace)
                });
                let end = if replace_inline {
                    name_token.as_ref().map(|token| token.text_range().end()).unwrap_or(offset)
                } else {
                    offset
                };
                let range = TextRange::new(offset, end);
                let range = position::text_range_to_protocol(content, range, encoding)?;
                let name_range = if replace_inline {
                    None
                } else {
                    name_token.as_ref().and_then(|token| {
                        let range = token.text_range();
                        position::text_range_to_protocol(content, range, encoding)
                    })
                };
                let name = name_token.map(|token| SmolStr::new(token.text(content)));
                let text = CursorText::ImportClassBoundary {
                    name,
                    name_range,
                    insert_space: class_boundary,
                };
                return Some((text, Some(range)));
            }

            let token = name_token?;
            let name = SmolStr::new(token.text(content));
            let range = position::text_range_to_protocol(content, token.text_range(), encoding)?;
            Some((CursorText::Name(name), Some(range)))
        })
    }

    fn of_qualified(
        content: &str,
        token: &SyntaxToken,
        encoding: PositionEncoding,
    ) -> Option<(CursorText, Option<Range>)> {
        token.parent_ancestors().find_map(|node| {
            let qualified = cst::QualifiedName::cast(node)?;

            let prefix_token = qualified.qualifier().and_then(|qualifier| qualifier.text());
            let prefix_range = prefix_token.as_ref().map(|token| token.text_range());
            let prefix = prefix_token.map(|token| token.text(content).into());

            let name_token = qualified
                .lower()
                .or_else(|| qualified.upper())
                .or_else(|| qualified.operator())
                .or_else(|| qualified.operator_name());

            const ONE: TextSize = TextSize::new(1);

            let name_range = name_token.as_ref().and_then(|token| {
                let range = token.text_range();
                if matches!(token.kind(), SyntaxKind::OPERATOR_NAME) {
                    let start = range.start().checked_add(ONE)?;
                    let end = range.end().checked_sub(ONE)?;
                    Some(TextRange::new(start, end))
                } else {
                    Some(range)
                }
            });

            let name = name_token.map(|token| {
                token.text(content).trim_start_matches('(').trim_end_matches(')').into()
            });

            let range = match (prefix_range, name_range) {
                (Some(p), Some(n)) => Some(p.cover(n)),
                (Some(r), None) => Some(r),
                (None, Some(r)) => Some(r),
                (None, None) => None,
            };

            let range =
                range.and_then(|range| position::text_range_to_protocol(content, range, encoding));
            let text = match (prefix, name) {
                (None, None) => CursorText::None,
                (Some(p), None) => CursorText::Prefix(p),
                (None, Some(n)) => CursorText::Name(n),
                (Some(p), Some(n)) => CursorText::Both(p, n),
            };

            Some((text, range))
        })
    }

    fn of_qualifier(
        content: &str,
        token: &SyntaxToken,
        encoding: PositionEncoding,
    ) -> Option<(CursorText, Option<Range>)> {
        token.parent_ancestors().find_map(|node| {
            let qualifier = cst::Qualifier::cast(node)?;
            let token = qualifier.text()?;

            let prefix = token.text(content);
            let prefix = SmolStr::new(prefix);

            let range = token.text_range();
            let range = position::text_range_to_protocol(content, range, encoding)?;

            let range = Some(range);
            let text = CursorText::Prefix(prefix);

            Some((text, range))
        })
    }

    fn of_module_name(
        content: &str,
        token: &SyntaxToken,
        encoding: PositionEncoding,
    ) -> Option<(CursorText, Option<Range>)> {
        token.parent_ancestors().find_map(|node| {
            let module_name = cst::ModuleName::cast(node)?;

            let prefix_token = module_name.qualifier().and_then(|qualifier| qualifier.text());
            let prefix_range = prefix_token.as_ref().map(|token| token.text_range());
            let prefix = prefix_token.map(|token| token.text(content).into());

            let name_token = module_name.name_token();
            let name_range = name_token.as_ref().map(|token| token.text_range());
            let name = name_token.map(|token| token.text(content).into());

            let range = match (prefix_range, name_range) {
                (Some(p), Some(n)) => Some(p.cover(n)),
                (Some(r), None) => Some(r),
                (None, Some(r)) => Some(r),
                (None, None) => None,
            };

            let range =
                range.map(|range| position::text_range_to_protocol(content, range, encoding))?;
            let text = match (prefix, name) {
                (None, None) => CursorText::None,
                (Some(p), None) => CursorText::Prefix(p),
                (None, Some(n)) => CursorText::Name(n),
                (Some(p), Some(n)) => CursorText::Both(p, n),
            };

            Some((text, range))
        })
    }
}
