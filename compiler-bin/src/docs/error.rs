use std::io;
use std::path::PathBuf;

use analyzer::QueryError;
use documentation::Error as DocumentationError;
use thiserror::Error;

use crate::walk;

#[derive(Error, Debug)]
pub enum DocsError {
    #[error("QueryError: {0}")]
    QueryError(#[from] QueryError),
    #[error("DocumentationError: {0}")]
    DocumentationError(#[from] DocumentationError),
    #[error("Invalid package glob: {0}")]
    InvalidPackageGlob(String),
    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),
    #[error("Missing package folder: {0}")]
    MissingPackageFolder(PathBuf),
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
}
