use files::FileId;
use itertools::Itertools;
use ssa::tree::{CallingConvention, Field, Literal, Projection};

use crate::error::{ModuleError, ModuleResult, UnsupportedState};
use crate::tree::{BinaryOperator, ExpressionId, Tree};

pub(super) fn literal_expression(
    tree: &mut Tree,
    literal: &Literal,
    file_id: FileId,
) -> ModuleResult<ExpressionId> {
    match literal {
        Literal::String { value } => Ok(tree.string(value.as_str())),
        Literal::Char { value } => Ok(tree.string(value.to_string())),
        Literal::Boolean { value } => Ok(tree.boolean(*value)),
        Literal::Integer { value } => Ok(integer_expression(tree, *value)),
        Literal::Number { value } => {
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

pub(super) fn integer_expression(tree: &mut Tree, value: i32) -> ExpressionId {
    let value = tree.number(value.to_string());
    let zero = tree.number("0");
    tree.binary(BinaryOperator::BitwiseOr, value, zero)
}

pub(super) fn call_expression(
    tree: &mut Tree,
    convention: CallingConvention,
    function: ExpressionId,
    arguments: Vec<ExpressionId>,
) -> ExpressionId {
    match convention {
        CallingConvention::Initializer => tree.call(function, arguments),
        CallingConvention::Source | CallingConvention::Effect => {
            let mut arguments = arguments.into_iter();
            let Some(argument) = arguments.next() else {
                return tree.call(function, vec![]);
            };
            let function = tree.call(function, vec![argument]);
            arguments.fold(function, |function, argument| tree.call(function, vec![argument]))
        }
    }
}

pub(super) fn project_field(tree: &mut Tree, record: ExpressionId, field: &Field) -> ExpressionId {
    tree.member(record, field.name.as_str())
}

pub(super) fn projection_expression(
    tree: &mut Tree,
    value: ExpressionId,
    projection: &Projection,
) -> ExpressionId {
    let index = match projection {
        Projection::ArrayElement { index } => *index,
        Projection::ConstructorArgument { index, .. } => index + 1,
    };
    let index = tree.number(index.to_string());
    tree.index(value, index)
}

pub(super) fn constructor_expression(tree: &mut Tree, name: &str, arity: usize) -> ExpressionId {
    let arguments = (0..arity).map(|index| format!("$value{index}"));
    let arguments = arguments.collect_vec();
    let values = arguments.iter().map(|argument| tree.identifier(argument));
    let values = values.collect_vec();
    let tag = tree.string(name);
    let elements = std::iter::once(tag).chain(values).collect_vec();
    let mut expression = tree.array(elements);
    for argument in arguments.into_iter().rev() {
        expression = tree.arrow(vec![argument], expression);
    }
    expression
}
