//! Elaboration from checked source modules into a small, semantic Core language.
//!
//! Type checking records facts and dictionary placement decisions. This crate
//! consumes those decisions; it neither rechecks source nor attempts to infer
//! where implicit arguments belong.

mod core;
mod elaborate;
mod evidence;

pub use core::*;
pub use elaborate::{ElaborationInput, elaborate_module};
