//! Owned static single-assignment control-flow graphs lowered from functional trees.

pub mod convert;
pub mod error;
pub mod pretty;
pub mod tree;

pub use convert::convert_module;
pub use error::{ConversionError, ConversionResult, UnsupportedState};
