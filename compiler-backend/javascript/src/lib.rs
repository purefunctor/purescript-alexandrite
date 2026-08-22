//! Direct JavaScript code generation from static single-assignment control-flow graphs.
//!
//! Generated modules target ES2022 and require Node.js 16 or newer. In particular, source names that
//! are not JavaScript identifiers use string-literal module export names.

mod convert;
mod error;
mod module;
mod pretty;
mod tree;

pub use convert::convert_module;
pub use error::{ModuleError, ModuleResult, UnsupportedState};
pub use module::{
    Module, foreign_module_filename, module_filename, runtime_filename, runtime_source,
};
