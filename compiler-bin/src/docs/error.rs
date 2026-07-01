use std::io;
use std::path::PathBuf;

use analyzer::QueryError;
use thiserror::Error;

use crate::walk;

#[derive(Error, Debug)]
pub enum DocsError {
    #[error("QueryError: {0}")]
    QueryError(#[from] QueryError),
    #[error("Failed to parse file {0}")]
    PathParseFail(PathBuf),
    #[error("IoError: {0}")]
    IoError(#[from] io::Error),
    #[error("GlobSetError: {0}")]
    GlobSetError(#[from] globset::Error),
    #[error("WalkError: {0}")]
    WalkError(#[from] walk::Error),
    #[error("JsonError: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("SpagoLockError: {0}")]
    SpagoLockError(#[from] spago::LockfileGlobSetError),
    #[error("TypeScript export error: {0}")]
    TypeScriptExportError(#[from] ts_rs::ExportError),
}
