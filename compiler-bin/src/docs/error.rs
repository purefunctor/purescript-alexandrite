use std::io;
use std::path::PathBuf;

use building::QueryError;
use documentation::Error as DocumentationError;
use thiserror::Error;

use crate::package;

#[derive(Error, Debug)]
pub enum DocsError {
    #[error("QueryError: {0}")]
    QueryError(#[from] QueryError),
    #[error("DocumentationError: {0}")]
    DocumentationError(#[from] DocumentationError),
    #[error("Duplicate module name: {0}")]
    DuplicateModuleName(String),
    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),
    #[error("Failed to parse file {0}")]
    PathParseFail(PathBuf),
    #[error("IoError: {0}")]
    IoError(#[from] io::Error),
    #[error("GlobSetError: {0}")]
    GlobSetError(#[from] globset::Error),
    #[error(transparent)]
    PackageError(#[from] package::PackageError),
    #[error("JsonError: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("SpagoLockError: {0}")]
    SpagoLockError(#[from] spago::LockfileGlobSetError),
}
