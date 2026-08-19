//! Owned functional trees used as input to normalization by evaluation.

pub mod convert;
pub mod error;
pub mod pretty;
pub mod tree;

pub use convert::convert_module;
pub use error::{ModuleError, ModuleResult, UnsupportedState};

use std::sync::Arc;

use building_types::QueryResult;
use files::FileId;

pub trait ExternalQueries {
    fn nbe(&self, file_id: FileId) -> QueryResult<ModuleResult<Arc<tree::Module>>>;
}
