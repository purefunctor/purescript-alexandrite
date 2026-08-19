//! Owned static single-assignment control-flow graphs lowered from functional trees.

use std::sync::Arc;

use building_types::QueryResult;
use files::FileId;

pub mod convert;
pub mod error;
pub mod pretty;
pub mod tree;

pub use convert::convert_module;
pub use error::{ModuleError, ModuleResult, UnsupportedState};

pub trait ExternalQueries {
    fn ssa(&self, file_id: FileId) -> QueryResult<ModuleResult<Arc<tree::Module>>>;
}
