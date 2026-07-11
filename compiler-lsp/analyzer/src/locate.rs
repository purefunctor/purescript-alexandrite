//! Abstractions for identifying syntax at a given location.

use std::iter;

use building::QueryEngine;
use files::FileId;
use indexing::{ImportItemId, IndexedModule, TermItemId, TypeItemId};
use lowering::{
    BinderId, ExpressionId, LetBindingNameGroupId, LoweredModule, RecordPunId, TermOperatorId,
    TypeId, TypeOperatorId,
};
use stabilizing::{AstId, StabilizedModule};
use syntax::ast::{AstNode, AstPtr};
use syntax::{SyntaxNode, SyntaxNodePtr, SyntaxToken, TokenAtOffset, cst};

use crate::extract::AnnotationSyntaxRange;
use crate::position::{Utf8Position, Utf8Range};
use crate::{AnalyzerError, position};

pub fn syntax_range(content: &str, root: &SyntaxNode, ptr: &SyntaxNodePtr) -> Option<Utf8Range> {
    let range = AnnotationSyntaxRange::from_ptr(root, ptr);
    range.syntax.and_then(|range| position::text_range_to_utf8_range(content, range))
}

pub fn id_range<T>(
    content: &str,
    parsed: &parsing::ParsedModule,
    stabilized: &StabilizedModule,
    item_id: AstId<T>,
) -> Option<Utf8Range>
where
    T: AstNode,
{
    let root = parsed.syntax_node();
    let ptr = stabilized.syntax_ptr(item_id)?;
    syntax_range(content, &root, &ptr)
}

pub fn value_equation_ranges(
    content: &str,
    root: &SyntaxNode,
    stabilized: &StabilizedModule,
    indexed: &IndexedModule,
    term_id: TermItemId,
) -> Option<Vec<Utf8Range>> {
    let indexing::TermItemKind::Value { signature, equations } = &indexed.items[term_id].kind
    else {
        return None;
    };

    let mut ranges = vec![];

    if let Some(sig_id) = signature
        && let Some(ptr) = stabilized.ast_ptr(*sig_id)
        && let Some(node) = ptr.try_to_node(root)
        && let Some(tok) = node.name_token()
        && let Some(range) = position::text_range_to_utf8_range(content, tok.text_range())
    {
        ranges.push(range);
    }

    for eq_id in equations {
        if let Some(ptr) = stabilized.ast_ptr(*eq_id)
            && let Some(node) = ptr.try_to_node(root)
            && let Some(tok) = node.name_token()
            && let Some(range) = position::text_range_to_utf8_range(content, tok.text_range())
        {
            ranges.push(range);
        }
    }

    Some(ranges)
}

type ModuleNamePtr = AstPtr<cst::ModuleName>;

#[derive(Debug, PartialEq, Eq)]
pub enum Located {
    ModuleName(ModuleNamePtr),
    ImportItem(ImportItemId),
    Binder(BinderId),
    Expression(ExpressionId),
    Type(TypeId),
    BinderPun(RecordPunId),
    ExpressionPun(RecordPunId),
    TermOperator(TermOperatorId),
    TypeOperator(TypeOperatorId),
    TermItem(TermItemId),
    TypeItem(TypeItemId),
    LetBinding(LetBindingNameGroupId),
    Nothing,
}

pub fn locate(
    engine: &QueryEngine,
    id: FileId,
    position: Utf8Position,
) -> Result<Located, AnalyzerError> {
    let content = engine.content(id);

    let (parsed, _) = engine.parsed(id)?;
    let stabilized = engine.stabilized(id)?;
    let indexed = engine.indexed(id)?;
    let lowered = engine.lowered(id)?;

    let Some(offset) = position::utf8_position_to_offset(&content, position) else {
        return Ok(Located::Nothing);
    };

    let node = parsed.syntax_node();
    let token = node.token_at_offset(offset);

    Ok(match token {
        TokenAtOffset::None => Located::Nothing,
        TokenAtOffset::Single(token) => locate_single(&stabilized, &indexed, &lowered, token),
        TokenAtOffset::Between(left, right) => {
            locate_between(&stabilized, &indexed, &lowered, left, right)
        }
    })
}

fn locate_single(
    stabilized: &StabilizedModule,
    indexed: &IndexedModule,
    lowered: &LoweredModule,
    token: SyntaxToken,
) -> Located {
    token
        .parent_ancestors()
        .find_map(|node| locate_node(stabilized, indexed, lowered, node))
        .unwrap_or(Located::Nothing)
}

fn locate_node(
    stabilized: &StabilizedModule,
    indexed: &IndexedModule,
    lowered: &LoweredModule,
    node: SyntaxNode,
) -> Option<Located> {
    let kind = node.kind();
    let ptr = SyntaxNodePtr::new(&node);
    if cst::Annotation::can_cast(kind) {
        Some(Located::Nothing)
    } else if cst::ModuleName::can_cast(kind) {
        let ptr = ptr.cast()?;
        Some(Located::ModuleName(ptr))
    } else if cst::ImportItem::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        Some(Located::ImportItem(id))
    } else if cst::Binder::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        Some(Located::Binder(id))
    } else if cst::Expression::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        Some(Located::Expression(id))
    } else if cst::Type::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        Some(Located::Type(id))
    } else if cst::RecordPun::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;

        let mut parents = iter::successors(Some(node), |node| node.parent());
        parents.find_map(|node| {
            let kind = node.kind();
            if cst::Binder::can_cast(kind) {
                Some(Located::BinderPun(id))
            } else if cst::Expression::can_cast(kind) {
                Some(Located::ExpressionPun(id))
            } else {
                None
            }
        })
    } else if cst::TermOperator::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        Some(Located::TermOperator(id))
    } else if cst::TypeOperator::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        Some(Located::TypeOperator(id))
    } else if cst::Declaration::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        None.or_else(|| indexed.pairs.declaration_to_term(id).map(Located::TermItem))
            .or_else(|| indexed.pairs.declaration_to_type(id).map(Located::TypeItem))
    } else if cst::LetBinding::can_cast(kind) {
        let node = cst::LetBinding::cast(node)?;
        match node {
            cst::LetBinding::LetBindingPattern(_) => None,
            cst::LetBinding::LetBindingSignature(signature) => {
                let ptr = AstPtr::new(&signature);
                let id = stabilized.lookup_ptr(&ptr)?;
                lowered.info.find_let_binding_group_by_signature(id).map(Located::LetBinding)
            }
            cst::LetBinding::LetBindingEquation(equation) => {
                let ptr = AstPtr::new(&equation);
                let id = stabilized.lookup_ptr(&ptr)?;
                lowered.info.find_let_binding_group_by_equation(id).map(Located::LetBinding)
            }
        }
    } else if cst::DataConstructor::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        let id = indexed.pairs.constructor_to_term(id)?;
        Some(Located::TermItem(id))
    } else if cst::ClassMemberStatement::can_cast(kind) {
        let ptr = ptr.cast()?;
        let id = stabilized.lookup_ptr(&ptr)?;
        let id = indexed.pairs.class_member_to_term(id)?;
        Some(Located::TermItem(id))
    } else {
        None
    }
}

fn locate_between(
    stabilized: &StabilizedModule,
    indexed: &IndexedModule,
    lowered: &LoweredModule,
    left: SyntaxToken,
    right: SyntaxToken,
) -> Located {
    let left = locate_single(stabilized, indexed, lowered, left);
    let right = locate_single(stabilized, indexed, lowered, right);
    match (&left, &right) {
        // If left/right share an ancestor;
        (_, _) if left == right => left,
        (_, Located::Nothing) => left,
        (Located::Nothing, _) => right,
        // otherwise, lean towards the right.
        (_, _) => right,
    }
}

#[cfg(test)]
mod tests;
