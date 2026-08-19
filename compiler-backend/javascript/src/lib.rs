//! Direct JavaScript code generation from static single-assignment control-flow graphs.

mod convert;
mod error;
mod module;
mod pretty;
mod tree;

pub use convert::convert_module;
pub use error::{ModuleError, ModuleResult, UnsupportedState};
pub use module::{Module, foreign_module_filename, module_filename};
