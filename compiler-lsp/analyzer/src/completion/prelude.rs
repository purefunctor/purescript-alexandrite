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

use crate::position::{PositionEncoding, Utf8Position};
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

        let token = match root.token_at_offset(self.offset) {
            TokenAtOffset::None => return Ok(None),
            TokenAtOffset::Single(token) => token,
            TokenAtOffset::Between(left, right) => {
                let left_annotation =
                    left.parent_ancestors().any(|node| node.kind() == SyntaxKind::Annotation);
                if left_annotation { right } else { left }
            }
        };
        let scope_node = self.scope_node_for_token(&lowered, token);

        Ok(scope_node)
    }

    fn scope_node_for_token(
        &self,
        lowered: &LoweredModule,
        token: SyntaxToken,
    ) -> Option<GraphNodeId> {
        token.parent_ancestors().find_map(|node| self.scope_node_for_syntax(lowered, node))
    }

    fn scope_node_for_do_statements(
        &self,
        lowered: &LoweredModule,
        statements: cst::DoStatements,
    ) -> Option<GraphNodeId> {
        cst::ExpressionDo::cast(statements.syntax().parent()?)?;

        let layout_end = statements.syntax().children_with_tokens().last()?.into_token()?;
        let layout_separator = layout_end.prev_sibling_or_token()?.into_token()?;
        if layout_end.kind() != SyntaxKind::LAYOUT_END
            || layout_separator.kind() != SyntaxKind::LAYOUT_SEPARATOR
            || layout_separator.text_range().start() != self.offset
        {
            return None;
        }

        let scopes = statements
            .children()
            .filter_map(|statement| self.scope_node_for_do_statement(lowered, &statement));
        scopes.last()
    }

    fn scope_node_for_do_statement(
        &self,
        lowered: &LoweredModule,
        statement: &cst::DoStatement,
    ) -> Option<GraphNodeId> {
        match statement {
            cst::DoStatement::DoStatementBind(statement) => {
                self.scope_node_for_binder(lowered, statement.binder()?)
            }
            cst::DoStatement::DoStatementLet(statement) => {
                let scopes = statement
                    .statements()?
                    .children()
                    .filter_map(|binding| self.scope_node_for_let_binding(lowered, &binding));
                scopes.last()
            }
            cst::DoStatement::DoStatementDiscard(_) => None,
        }
    }

    fn scope_node_for_binder(
        &self,
        lowered: &LoweredModule,
        binder: cst::Binder,
    ) -> Option<GraphNodeId> {
        let binder_id = self.stabilized.lookup_ptr(&AstPtr::new(&binder))?;
        lowered.nodes.binder_node(binder_id)
    }

    fn scope_node_for_let_binding(
        &self,
        lowered: &LoweredModule,
        binding: &cst::LetBinding,
    ) -> Option<GraphNodeId> {
        let let_binding_group_id = match binding {
            cst::LetBinding::LetBindingPattern(binding) => {
                return self.scope_node_for_binder(lowered, binding.binder()?);
            }
            cst::LetBinding::LetBindingSignature(signature) => {
                let signature_id = self.stabilized.lookup_ptr(&AstPtr::new(signature))?;
                lowered.tree.find_let_binding_group_by_signature(signature_id)?
            }
            cst::LetBinding::LetBindingEquation(equation) => {
                let equation_id = self.stabilized.lookup_ptr(&AstPtr::new(equation))?;
                lowered.tree.find_let_binding_group_by_equation(equation_id)?
            }
        };
        lowered.nodes.let_node(let_binding_group_id)
    }

    fn scope_node_for_where_expression(
        &self,
        lowered: &LoweredModule,
        expression: cst::WhereExpression,
    ) -> Option<GraphNodeId> {
        let bindings = expression.bindings()?;
        if self.cursor_within_node(bindings.syntax()) {
            return None;
        }

        let scopes = bindings
            .children()
            .filter_map(|binding| self.scope_node_for_let_binding(lowered, &binding));
        scopes.last()
    }

    fn scope_node_before_pattern_binding(
        &self,
        lowered: &LoweredModule,
        pattern: &cst::LetBindingPattern,
    ) -> Option<GraphNodeId> {
        let statements = cst::LetBindingStatements::cast(pattern.syntax().parent()?)?;
        let preceding =
            statements.children().take_while(|binding| binding.syntax() != pattern.syntax());
        let scopes =
            preceding.filter_map(|binding| self.scope_node_for_let_binding(lowered, &binding));
        scopes.last()
    }

    fn scope_node_for_pattern_binding(
        &self,
        lowered: &LoweredModule,
        pattern: &cst::LetBindingPattern,
    ) -> Option<GraphNodeId> {
        if !self.cursor_follows_rhs_delimiter(pattern.syntax()) {
            return None;
        }
        if let Some(expression) = pattern.where_expression() {
            if let Some(scope) = self.scope_node_for_where_expression(lowered, expression) {
                return Some(scope);
            }
        }
        self.scope_node_before_pattern_binding(lowered, pattern)
    }

    fn cursor_follows_rhs_delimiter(&self, node: &SyntaxNode) -> bool {
        let mut tokens = node.children_with_tokens().filter_map(|element| element.into_token());
        let delimiter = tokens
            .find(|token| matches!(token.kind(), SyntaxKind::EQUAL | SyntaxKind::RIGHT_ARROW));
        delimiter.is_some_and(|delimiter| delimiter.text_range().end() <= self.offset)
    }

    fn scope_node_for_unconditional(
        &self,
        lowered: &LoweredModule,
        unconditional: cst::Unconditional,
    ) -> Option<GraphNodeId> {
        if !self.cursor_follows_rhs_delimiter(unconditional.syntax()) {
            return None;
        }
        self.scope_node_for_where_expression(lowered, unconditional.where_expression()?)
    }

    fn scope_node_for_pattern_guarded(
        &self,
        lowered: &LoweredModule,
        guarded: cst::PatternGuarded,
    ) -> Option<GraphNodeId> {
        if !self.cursor_follows_rhs_delimiter(guarded.syntax()) {
            return None;
        }
        if let Some(expression) = guarded.where_expression() {
            if let Some(scope) = self.scope_node_for_where_expression(lowered, expression) {
                return Some(scope);
            }
        }

        let scopes = guarded.children().filter_map(|guard| match guard {
            cst::PatternGuard::PatternGuardBinder(guard) => {
                self.scope_node_for_binder(lowered, guard.binder()?)
            }
            cst::PatternGuard::PatternGuardExpression(_) => None,
        });
        scopes.last()
    }

    fn cursor_within_node(&self, node: &SyntaxNode) -> bool {
        let range = node.text_range();
        range.start() <= self.offset && self.offset <= range.end()
    }

    fn cursor_within_guarded_expression(&self, guarded: &cst::GuardedExpression) -> bool {
        self.cursor_within_node(guarded.syntax())
    }

    fn scope_node_for_syntax(
        &self,
        lowered: &LoweredModule,
        node: SyntaxNode,
    ) -> Option<GraphNodeId> {
        let kind = node.kind();
        let ptr = SyntaxNodePtr::new(&node);

        if cst::DoStatements::can_cast(kind) {
            let statements = cst::DoStatements::cast(node)?;
            self.scope_node_for_do_statements(lowered, statements)
        } else if cst::Binder::can_cast(kind) {
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
        } else if cst::WhereExpression::can_cast(kind) {
            let expression = cst::WhereExpression::cast(node)?;
            self.scope_node_for_where_expression(lowered, expression)
        } else if cst::Unconditional::can_cast(kind) {
            let unconditional = cst::Unconditional::cast(node)?;
            self.scope_node_for_unconditional(lowered, unconditional)
        } else if cst::PatternGuarded::can_cast(kind) {
            let guarded = cst::PatternGuarded::cast(node)?;
            self.scope_node_for_pattern_guarded(lowered, guarded)
        } else if cst::ValueEquation::can_cast(kind) {
            let equation = cst::ValueEquation::cast(node)?;
            let guarded = equation.guarded_expression()?;
            if !self.cursor_within_guarded_expression(&guarded) {
                return None;
            }
            let binder = equation.function_binders()?.children().next()?;
            self.scope_node_for_binder(lowered, binder)
        } else if cst::InstanceEquationStatement::can_cast(kind) {
            let equation = cst::InstanceEquationStatement::cast(node)?;
            let guarded = equation.guarded_expression()?;
            if !self.cursor_within_guarded_expression(&guarded) {
                return None;
            }
            let binder = equation.function_binders()?.children().next()?;
            self.scope_node_for_binder(lowered, binder)
        } else if cst::CaseBranch::can_cast(kind) {
            let branch = cst::CaseBranch::cast(node)?;
            let guarded = branch.guarded_expression()?;
            if !self.cursor_within_guarded_expression(&guarded) {
                return None;
            }
            let binder = branch.binders()?.children().next()?;
            self.scope_node_for_binder(lowered, binder)
        } else if cst::LetBinding::can_cast(kind) {
            let binding = cst::LetBinding::cast(node)?;
            match &binding {
                cst::LetBinding::LetBindingPattern(pattern) => {
                    self.scope_node_for_pattern_binding(lowered, pattern)
                }
                cst::LetBinding::LetBindingSignature(_) => {
                    self.scope_node_for_let_binding(lowered, &binding)
                }
                cst::LetBinding::LetBindingEquation(equation) => {
                    let function_scope = equation
                        .guarded_expression()
                        .filter(|guarded| self.cursor_within_guarded_expression(guarded))
                        .and_then(|_| equation.function_binders())
                        .and_then(|binders| binders.children().next())
                        .and_then(|binder| self.scope_node_for_binder(lowered, binder));
                    function_scope.or_else(|| self.scope_node_for_let_binding(lowered, &binding))
                }
            }
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
    General,
    Comment,
}

const COMPLETION_MARKER: &str = "Z'PureScript'Z";

impl CursorSemantics {
    pub fn new(content: &str, position: Utf8Position) -> CursorSemantics {
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

        let Some(offset) = position::utf8_position_to_offset(content, position) else {
            return CursorSemantics::General;
        };

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
    None,
    Prefix(SmolStr),
    Name(SmolStr),
    Both(SmolStr, SmolStr),
}

impl CursorText {
    pub fn new(
        content: &str,
        token: &SyntaxToken,
        encoding: PositionEncoding,
    ) -> (CursorText, Option<Range>) {
        CursorText::of_qualified(content, token, encoding)
            .or_else(|| CursorText::of_qualifier(content, token, encoding))
            .or_else(|| CursorText::of_module_name(content, token, encoding))
            .unwrap_or((CursorText::None, None))
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
