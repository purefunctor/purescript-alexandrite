//! JavaScript generation and serialization from PureScript CoreFn.

mod ast;
mod generate;
mod names;

pub use ast::Module;
pub use generate::generate_module;
