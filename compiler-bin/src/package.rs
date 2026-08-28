use std::path::{Component, Path, PathBuf};
use std::{fs, io};

use serde::Deserialize;
use thiserror::Error;

use crate::walk;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("invalid package glob: {0}")]
    InvalidGlob(String),
    #[error("missing package folder: {0}")]
    MissingFolder(PathBuf),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Walk(#[from] walk::Error),
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageSources {
    #[serde(default)]
    include_files: Vec<String>,
    #[serde(default)]
    exclude_files: Vec<String>,
}

pub fn source_files(root: &Path, package: &Path) -> Result<Vec<PathBuf>, PackageError> {
    let package_root = root.join(package);
    if !package_root.is_dir() {
        return Err(PackageError::MissingFolder(package_root));
    }

    let manifest = package_root.join("purs.json");
    let sources = if manifest.exists() {
        let manifest = fs::read_to_string(manifest)?;
        serde_json::from_str(&manifest)?
    } else {
        PackageSources::default()
    };

    source_files_with_globs(root, package, &sources.include_files, &sources.exclude_files)
}

pub fn source_files_with_globs(
    root: &Path,
    package: &Path,
    include_files: &[String],
    exclude_files: &[String],
) -> Result<Vec<PathBuf>, PackageError> {
    let package_root = root.join(package);
    if !package_root.is_dir() {
        return Err(PackageError::MissingFolder(package_root));
    }

    let mut includes = vec![package.join("src/**/*.purs"), package.join("test/**/*.purs")];
    for path in include_files {
        validate_package_glob(path)?;
        includes.push(package.join(path));
    }

    let mut excludes = vec![];
    for path in exclude_files {
        validate_package_glob(path)?;
        excludes.push(package.join(path));
    }

    let walked = walk::walk_filtered(root, includes, excludes)?;
    Ok(walked.files)
}

fn validate_package_glob(path: &str) -> Result<(), PackageError> {
    let valid = !path.is_empty() && Path::new(path).components().all(is_package_relative_component);
    if !valid {
        return Err(PackageError::InvalidGlob(path.to_owned()));
    }

    Ok(())
}

fn is_package_relative_component(component: Component<'_>) -> bool {
    match component {
        Component::Normal(_) | Component::CurDir => true,
        Component::Prefix(_) | Component::RootDir | Component::ParentDir => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory() -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let directory = std::env::temp_dir().join(format!("alexandrite-package-{nanos}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    #[test]
    fn manifest_globs_extend_and_filter_package_sources() {
        let root = temporary_directory();
        let package = root.join("effect");
        touch(&package.join("src/Main.purs"));
        touch(&package.join("test/Test.Main.purs"));
        touch(&package.join("test/Excluded.purs"));
        touch(&package.join("examples/Example.purs"));
        fs::write(
            package.join("purs.json"),
            r#"{
              "includeFiles": ["examples/**/*.purs"],
              "excludeFiles": ["test/Excluded.purs"]
            }"#,
        )
        .unwrap();

        let files = source_files(&root, Path::new("effect")).unwrap();
        let files = files
            .iter()
            .map(|path| path.strip_prefix(&root).unwrap().to_string_lossy().to_string());
        let files = files.collect::<Vec<_>>();

        assert_eq!(
            files,
            ["effect/examples/Example.purs", "effect/src/Main.purs", "effect/test/Test.Main.purs"]
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_globs_reject_paths_outside_the_package_root() {
        let root = temporary_directory();
        fs::create_dir(root.join("effect")).unwrap();

        let result = source_files_with_globs(
            &root,
            Path::new("effect"),
            &["../Outside.purs".to_owned()],
            &[],
        );

        assert!(matches!(result, Err(PackageError::InvalidGlob(_))));
        fs::remove_dir_all(root).unwrap();
    }
}
