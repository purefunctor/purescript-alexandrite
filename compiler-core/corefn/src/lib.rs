//! PureScript CoreFn's serialized representation and lowering from Alexandrite's checked tree.

mod compile;
mod model;

pub use compile::{ExternalQueries, compile_module};
pub use model::*;
