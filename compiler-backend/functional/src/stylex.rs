//! Functional identities and semantic expressions for StyleX compiler intrinsics.

use std::sync::Arc;

use crate::tree::ExpressionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleXExpression {
    Call { target: StyleXCallTarget, arguments: Arc<[ExpressionId]> },
    Conditional { condition: ExpressionId, style: ExpressionId },
    ConditionalCase(StyleXConditionalCase),
    ConditionalValue { default: ExpressionId, cases: Arc<[StyleXConditionalCase]> },
}

impl StyleXExpression {
    pub fn children(&self) -> Vec<ExpressionId> {
        match self {
            StyleXExpression::Call { arguments, .. } => arguments.to_vec(),
            StyleXExpression::Conditional { condition, style } => vec![*condition, *style],
            StyleXExpression::ConditionalCase(case) => case.children(),
            StyleXExpression::ConditionalValue { default, cases } => {
                let mut children = vec![*default];
                for case in cases.iter() {
                    children.extend(case.children());
                }
                children
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleXConditionalCase {
    pub relation: StyleXWhenRelation,
    pub selector: ExpressionId,
    pub marker: Option<ExpressionId>,
    pub value: ExpressionId,
}

impl StyleXConditionalCase {
    fn children(&self) -> Vec<ExpressionId> {
        let mut children = vec![self.selector];
        children.extend(self.marker);
        children.push(self.value);
        children
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleXCallTarget {
    Root(StyleXRootCall),
    Types(StyleXTypeCall),
}

impl StyleXCallTarget {
    pub fn name(self) -> &'static str {
        match self {
            StyleXCallTarget::Root(call) => call.name(),
            StyleXCallTarget::Types(call) => call.name(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleXRootCall {
    Create,
    Props,
    Attrs,
    Keyframes,
    DefineConsts,
    DefineVars,
    CreateTheme,
    DefineMarker,
    DefaultMarker,
    ViewTransitionClass,
    PositionTry,
    FirstThatWorks,
}

impl StyleXRootCall {
    pub fn name(self) -> &'static str {
        match self {
            StyleXRootCall::Create => "create",
            StyleXRootCall::Props => "props",
            StyleXRootCall::Attrs => "attrs",
            StyleXRootCall::Keyframes => "keyframes",
            StyleXRootCall::DefineConsts => "defineConsts",
            StyleXRootCall::DefineVars => "defineVars",
            StyleXRootCall::CreateTheme => "createTheme",
            StyleXRootCall::DefineMarker => "defineMarker",
            StyleXRootCall::DefaultMarker => "defaultMarker",
            StyleXRootCall::ViewTransitionClass => "viewTransitionClass",
            StyleXRootCall::PositionTry => "positionTry",
            StyleXRootCall::FirstThatWorks => "firstThatWorks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleXWhenRelation {
    Ancestor,
    Descendant,
    SiblingBefore,
    SiblingAfter,
    AnySibling,
}

impl StyleXWhenRelation {
    pub fn name(self) -> &'static str {
        match self {
            StyleXWhenRelation::Ancestor => "ancestor",
            StyleXWhenRelation::Descendant => "descendant",
            StyleXWhenRelation::SiblingBefore => "siblingBefore",
            StyleXWhenRelation::SiblingAfter => "siblingAfter",
            StyleXWhenRelation::AnySibling => "anySibling",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleXTypeCall {
    Angle,
    Color,
    Url,
    Image,
    Integer,
    LengthPercentage,
    Length,
    Percentage,
    Number,
    Resolution,
    Time,
    TransformFunction,
    TransformList,
}

impl StyleXTypeCall {
    pub fn name(self) -> &'static str {
        match self {
            StyleXTypeCall::Angle => "angle",
            StyleXTypeCall::Color => "color",
            StyleXTypeCall::Url => "url",
            StyleXTypeCall::Image => "image",
            StyleXTypeCall::Integer => "integer",
            StyleXTypeCall::LengthPercentage => "lengthPercentage",
            StyleXTypeCall::Length => "length",
            StyleXTypeCall::Percentage => "percentage",
            StyleXTypeCall::Number => "number",
            StyleXTypeCall::Resolution => "resolution",
            StyleXTypeCall::Time => "time",
            StyleXTypeCall::TransformFunction => "transformFunction",
            StyleXTypeCall::TransformList => "transformList",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StyleXIntrinsic {
    Root(StyleXRootIntrinsic),
    When { relation: StyleXWhenRelation, marker: bool },
    Types(StyleXTypeCall),
}

impl StyleXIntrinsic {
    pub(crate) fn name(self) -> &'static str {
        match self {
            StyleXIntrinsic::Root(intrinsic) => intrinsic.name(),
            StyleXIntrinsic::When { relation, marker: false } => relation.name(),
            StyleXIntrinsic::When { relation: StyleXWhenRelation::Ancestor, marker: true } => {
                "ancestorMarker"
            }
            StyleXIntrinsic::When { relation: StyleXWhenRelation::Descendant, marker: true } => {
                "descendantMarker"
            }
            StyleXIntrinsic::When { relation: StyleXWhenRelation::SiblingBefore, marker: true } => {
                "siblingBeforeMarker"
            }
            StyleXIntrinsic::When { relation: StyleXWhenRelation::SiblingAfter, marker: true } => {
                "siblingAfterMarker"
            }
            StyleXIntrinsic::When { relation: StyleXWhenRelation::AnySibling, marker: true } => {
                "anySiblingMarker"
            }
            StyleXIntrinsic::Types(call) => call.name(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StyleXRootIntrinsic {
    Call(StyleXRootCall),
    RecordProps,
    RecordAttrs,
    Conditional,
    ConditionalValue,
}

impl StyleXRootIntrinsic {
    fn name(self) -> &'static str {
        match self {
            StyleXRootIntrinsic::Call(call) => call.name(),
            StyleXRootIntrinsic::RecordProps => "recordProps",
            StyleXRootIntrinsic::RecordAttrs => "recordAttrs",
            StyleXRootIntrinsic::Conditional => "conditional",
            StyleXRootIntrinsic::ConditionalValue => "conditionalValue",
        }
    }
}
