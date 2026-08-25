//! JavaScript expressions for atomic functional syntax.

use files::FileId;
use functional::tree::{
    BinaryOperator as FunctionalBinaryOperator, Literal, ReflectableEvidence, ReflectableOrdering,
    SynthesizedEvidence, UnaryOperator as FunctionalUnaryOperator,
};
use itertools::Itertools;

use super::super::super::names::identifier_is_binding;
use crate::error::{ModuleError, ModuleResult, UnsupportedState};
use crate::pretty::render_string;
use crate::tree::{BinaryOperator, ExpressionId, ObjectProperty, Tree, UnaryOperator};

pub(super) fn literal_expression(
    tree: &mut Tree,
    literal: &Literal,
    file_id: FileId,
) -> ModuleResult<ExpressionId> {
    match literal {
        Literal::String(value) => Ok(tree.string(value.as_str())),
        Literal::Char(value) => Ok(tree.string(value.to_string())),
        Literal::Boolean(value) => Ok(tree.boolean(*value)),
        Literal::Integer(value) => Ok(integer_expression(tree, *value)),
        Literal::Number(value) => {
            let number = value.parse::<f64>().map_err(|_| ModuleError::Unsupported {
                file_id,
                state: UnsupportedState::InvalidNumber { value: value.to_string() },
            })?;
            if !number.is_finite() {
                return Err(ModuleError::Unsupported {
                    file_id,
                    state: UnsupportedState::InvalidNumber { value: value.to_string() },
                });
            }
            Ok(tree.number(number.to_string()))
        }
    }
}

fn integer_expression(tree: &mut Tree, value: i32) -> ExpressionId {
    let value = tree.number(value.to_string());
    integer_coercion_expression(tree, value)
}

pub(super) fn integer_coercion_expression(tree: &mut Tree, value: ExpressionId) -> ExpressionId {
    let zero = tree.number("0");
    tree.binary(BinaryOperator::BitwiseOr, value, zero)
}

pub(super) fn unary_expression(
    tree: &mut Tree,
    operator: FunctionalUnaryOperator,
    value: ExpressionId,
) -> ExpressionId {
    match operator {
        FunctionalUnaryOperator::BooleanNot => tree.unary(UnaryOperator::LogicalNot, value),
        FunctionalUnaryOperator::IntegerNegate => {
            let value = tree.unary(UnaryOperator::Negate, value);
            integer_coercion_expression(tree, value)
        }
    }
}

pub(super) fn binary_expression(
    tree: &mut Tree,
    operator: FunctionalBinaryOperator,
    left: ExpressionId,
    right: ExpressionId,
) -> ExpressionId {
    let operator = match operator {
        FunctionalBinaryOperator::IntegerAdd => BinaryOperator::Add,
        FunctionalBinaryOperator::IntegerSubtract => BinaryOperator::Subtract,
        FunctionalBinaryOperator::IntegerMultiply => BinaryOperator::Multiply,
    };
    let value = tree.binary(operator, left, right);
    integer_coercion_expression(tree, value)
}

pub(super) fn curried_call_expression(
    tree: &mut Tree,
    function: ExpressionId,
    arguments: Vec<ExpressionId>,
) -> ExpressionId {
    if arguments.is_empty() {
        return tree.call(function, vec![]);
    }
    let arguments = arguments.into_iter();
    arguments.fold(function, |function, argument| tree.call(function, vec![argument]))
}

pub(super) fn constructor_expression(tree: &mut Tree, name: &str, arity: usize) -> ExpressionId {
    if arity == 0 {
        return tree.string(name);
    }

    let arguments = (0..arity).map(|index| format!("$value{index}")).collect_vec();
    let values = arguments.iter().map(|argument| tree.identifier(argument)).collect_vec();
    let tag = tree.string(name);
    let mut elements = Vec::with_capacity(values.len() + 1);
    elements.push(tag);
    elements.extend(values);
    let mut expression = tree.array(elements);
    for argument in arguments.into_iter().rev() {
        expression = tree.arrow(vec![argument], expression);
    }
    expression
}

pub(super) fn synthesized_evidence_expression(
    tree: &mut Tree,
    evidence: &SynthesizedEvidence,
) -> ExpressionId {
    match evidence {
        SynthesizedEvidence::IsSymbol(symbol) => {
            let symbol = tree.string(symbol.as_str());
            let reflect = tree.arrow(vec!["$proxy".to_owned()], symbol);
            tree.object(vec![ObjectProperty::Field {
                name: "reflectSymbol".to_owned(),
                value: reflect,
            }])
        }
        SynthesizedEvidence::Reflectable(evidence) => {
            let value = match evidence {
                ReflectableEvidence::Integer(value) => integer_expression(tree, *value),
                ReflectableEvidence::String(value) => tree.string(value.as_str()),
                ReflectableEvidence::Boolean(value) => tree.boolean(*value),
                ReflectableEvidence::Ordering(ordering) => {
                    let tag = match ordering {
                        ReflectableOrdering::Less => "LT",
                        ReflectableOrdering::Equal => "EQ",
                        ReflectableOrdering::Greater => "GT",
                    };
                    tree.string(tag)
                }
            };
            let reflect = tree.arrow(vec!["$proxy".to_owned()], value);
            tree.object(vec![ObjectProperty::Field {
                name: "reflectType".to_owned(),
                value: reflect,
            }])
        }
    }
}

pub(super) fn combine_conditions(
    tree: &mut Tree,
    conditions: &[ExpressionId],
) -> Option<ExpressionId> {
    let mut conditions = conditions.iter().copied();
    let first = conditions.next()?;
    Some(
        conditions.fold(first, |condition, next| {
            tree.binary(BinaryOperator::LogicalAnd, condition, next)
        }),
    )
}

pub(super) fn module_export_name(name: &str) -> String {
    if identifier_is_binding(name) { name.to_owned() } else { render_string(name) }
}
