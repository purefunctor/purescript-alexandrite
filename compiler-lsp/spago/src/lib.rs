pub mod lockfile;

use std::path::Path;
use std::{fs, io};

pub use lockfile::{PackageReference, PackageSources, PackagesBySource};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LockfileGlobSetError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

pub fn source_files_by_package(
    root: impl AsRef<Path>,
) -> Result<PackagesBySource, LockfileGlobSetError> {
    let root = root.as_ref();
    let lockfile = root.join("spago.lock");

    let lockfile = fs::read_to_string(lockfile)?;
    let lockfile: lockfile::Lockfile = serde_json::from_str(&lockfile)?;

    let mut packages = lockfile.walk_by_package(root);
    for package in packages.values_mut() {
        package.sources.sort();
    }

    Ok(packages)
}
