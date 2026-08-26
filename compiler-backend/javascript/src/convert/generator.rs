//! JavaScript module generation.

mod functional;
mod names;

pub(crate) use functional::Generator;
pub(crate) use names::identifier_is_binding;
