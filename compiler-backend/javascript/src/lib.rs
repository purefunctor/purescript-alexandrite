//! Direct JavaScript code generation from functional trees.
//!
//! Generated modules target ES2022 and require Node.js 16 or newer. In particular, source names that
//! are not JavaScript identifiers use string-literal module export names.

mod convert;
mod error;
mod module;
mod tree;
mod writer;

pub use convert::convert_module;
pub use error::{ModuleError, ModuleResult, UnsupportedState};
pub use module::{
    Module, foreign_module_filename, module_filename, runtime_filename, runtime_source,
};
