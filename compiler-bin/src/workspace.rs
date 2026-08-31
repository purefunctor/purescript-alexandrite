use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

use ignore::WalkBuilder;
use serde::Deserialize;
use thiserror::Error;

const MANIFEST: &str = "spago.yaml";

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("no Spago workspace found from {0}")]
    MissingWorkspace(PathBuf),
    #[error("workspace package '{requested}' was not found; available packages: {available}")]
    UnknownPackage { requested: String, available: String },
    #[error("workspace contains no packages")]
    NoPackages,
    #[error("workspace package selection is ambiguous; use --package <NAME>")]
    AmbiguousPackage,
    #[error("duplicate workspace package name '{name}' in {first} and {second}")]
    DuplicatePackage { name: String, first: PathBuf, second: PathBuf },
    #[error("failed to read {path}: {source}")]
    ReadManifest {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to canonicalize working directory {path}: {source}")]
    CanonicalizeDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    ParseManifest {
        path: PathBuf,
        #[source]
        source: serde_yml::Error,
    },
    #[error(transparent)]
    Walk(#[from] ignore::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub workspace: Option<serde_yml::Value>,
    pub package: Option<PackageManifest>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageManifest {
    pub name: String,
    pub run: Option<ExecutionConfig>,
    pub test: Option<ExecutionConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionConfig {
    pub main: Option<String>,
    #[serde(default)]
    pub exec_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspacePackage {
    pub root: PathBuf,
    pub manifest: PackageManifest,
    pub has_tests: bool,
}

#[derive(Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub packages: BTreeMap<String, WorkspacePackage>,
    pub selected: Option<String>,
}

impl Workspace {
    pub fn find_ancestor(current_directory: &Path) -> Result<Option<PathBuf>, WorkspaceError> {
        for directory in current_directory.ancestors().skip(1) {
            let path = directory.join(MANIFEST);
            if path.is_file() && read_manifest(&path)?.workspace.is_some() {
                return Ok(Some(directory.to_path_buf()));
            }
        }
        Ok(None)
    }

    pub fn discover(
        current_directory: &Path,
        requested_package: Option<&str>,
    ) -> Result<Workspace, WorkspaceError> {
        let current_directory = current_directory.canonicalize().map_err(|source| {
            WorkspaceError::CanonicalizeDirectory { path: current_directory.to_path_buf(), source }
        })?;
        let (root, inferred_package) = find_root(&current_directory)?;
        let packages = discover_packages(&root)?;
        if packages.is_empty() {
            return Err(WorkspaceError::NoPackages);
        }

        let selected = if let Some(requested) = requested_package {
            if !packages.contains_key(requested) {
                let available = packages.keys().cloned().collect::<Vec<_>>().join(", ");
                return Err(WorkspaceError::UnknownPackage {
                    requested: requested.to_owned(),
                    available,
                });
            }
            Some(requested.to_owned())
        } else if let Some(inferred) = inferred_package.filter(|name| packages.contains_key(name)) {
            Some(inferred)
        } else if packages.len() == 1 {
            packages.keys().next().cloned()
        } else {
            None
        };

        Ok(Workspace { root, packages, selected })
    }

    pub fn require_selected(&self) -> Result<&WorkspacePackage, WorkspaceError> {
        let Some(name) = &self.selected else {
            return Err(WorkspaceError::AmbiguousPackage);
        };
        Ok(&self.packages[name])
    }
}

fn find_root(current_directory: &Path) -> Result<(PathBuf, Option<String>), WorkspaceError> {
    let mut inferred_package = None;
    for directory in current_directory.ancestors() {
        let path = directory.join(MANIFEST);
        if !path.is_file() {
            continue;
        }
        let manifest = read_manifest(&path)?;
        if manifest.workspace.is_some() {
            return Ok((directory.to_path_buf(), inferred_package));
        }
        if inferred_package.is_none()
            && let Some(package) = manifest.package
        {
            inferred_package = Some(package.name);
        }
    }
    Err(WorkspaceError::MissingWorkspace(current_directory.to_path_buf()))
}

fn discover_packages(root: &Path) -> Result<BTreeMap<String, WorkspacePackage>, WorkspaceError> {
    let mut builder = WalkBuilder::new(root);
    builder.hidden(false);
    builder.ignore(false);
    builder.require_git(false);
    builder.git_global(false);
    builder.git_exclude(false);
    builder.parents(false);
    let filter_root = root.to_path_buf();
    builder.filter_entry(move |entry| !excluded_directory(entry.path(), &filter_root));
    let manifests = builder.build().filter_map(|entry| {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => return Some(Err(error)),
        };
        if !entry.file_type().is_some_and(|file_type| file_type.is_file()) {
            return None;
        }
        if entry.file_name() != MANIFEST {
            return None;
        }
        Some(Ok(entry.into_path()))
    });
    let mut manifests = manifests.collect::<Result<Vec<_>, _>>()?;
    manifests.sort_by_key(|path| path.components().count());

    let mut nested_workspaces = Vec::new();
    let mut packages = BTreeMap::new();
    for path in manifests {
        let package_root = path.parent().expect("spago.yaml path has no parent");
        if package_root != root
            && nested_workspaces.iter().any(|nested: &PathBuf| package_root.starts_with(nested))
        {
            continue;
        }

        let manifest = read_manifest(&path)?;
        if package_root != root && manifest.workspace.is_some() {
            nested_workspaces.push(package_root.to_path_buf());
            continue;
        }
        let Some(package) = manifest.package else {
            continue;
        };
        let name = package.name.clone();
        let workspace_package = WorkspacePackage {
            root: package_root.to_path_buf(),
            has_tests: package_root.join("test").is_dir(),
            manifest: package,
        };
        if let Some(previous) = packages.insert(name.clone(), workspace_package) {
            return Err(WorkspaceError::DuplicatePackage {
                name,
                first: previous.root,
                second: package_root.to_path_buf(),
            });
        }
    }
    Ok(packages)
}

fn excluded_directory(path: &Path, root: &Path) -> bool {
    if path == root {
        return false;
    }
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | ".spago" | "node_modules")
    )
}

fn read_manifest(path: &Path) -> Result<Manifest, WorkspaceError> {
    let source = fs::read_to_string(path)
        .map_err(|source| WorkspaceError::ReadManifest { path: path.to_path_buf(), source })?;
    serde_yml::from_str(&source)
        .map_err(|source| WorkspaceError::ParseManifest { path: path.to_path_buf(), source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn discovers_current_package_in_workspace() {
        let temporary = tempdir().unwrap();
        write(&temporary.path().join("spago.yaml"), "workspace: {}\n");
        write(
            &temporary.path().join("packages/app/spago.yaml"),
            r#"package:
  name: app
"#,
        );
        write(
            &temporary.path().join("packages/library/spago.yaml"),
            r#"package:
  name: library
"#,
        );
        let current = temporary.path().join("packages/app/src");
        fs::create_dir_all(&current).unwrap();

        let workspace = Workspace::discover(&current, None).unwrap();
        assert_eq!(workspace.selected.as_deref(), Some("app"));
        assert_eq!(workspace.packages.keys().collect::<Vec<_>>(), vec!["app", "library"]);
    }

    #[test]
    fn workspace_root_does_not_select_its_package() {
        let temporary = tempdir().unwrap();
        write(
            &temporary.path().join("spago.yaml"),
            r#"workspace: {}
package:
  name: root
"#,
        );
        write(
            &temporary.path().join("packages/library/spago.yaml"),
            r#"package:
  name: library
"#,
        );

        let workspace = Workspace::discover(temporary.path(), None).unwrap();
        assert_eq!(workspace.selected, None);
    }

    #[test]
    fn root_package_subdirectory_does_not_select_it() {
        let temporary = tempdir().unwrap();
        write(
            &temporary.path().join("spago.yaml"),
            r#"workspace: {}
package:
  name: root
"#,
        );
        write(
            &temporary.path().join("packages/library/spago.yaml"),
            r#"package:
  name: library
"#,
        );
        let current = temporary.path().join("src");
        fs::create_dir_all(&current).unwrap();

        let workspace = Workspace::discover(&current, None).unwrap();
        assert_eq!(workspace.selected, None);
    }

    #[test]
    fn gitignored_packages_are_not_discovered() {
        let temporary = tempdir().unwrap();
        write(
            &temporary.path().join("spago.yaml"),
            r#"workspace: {}
package:
  name: root
"#,
        );
        write(&temporary.path().join(".gitignore"), "ignored/\n");
        write(
            &temporary.path().join("ignored/spago.yaml"),
            r#"package:
  name: ignored
"#,
        );

        let workspace = Workspace::discover(temporary.path(), None).unwrap();
        assert_eq!(workspace.packages.keys().collect::<Vec<_>>(), vec!["root"]);
    }

    #[test]
    fn nested_workspace_is_not_part_of_parent() {
        let temporary = tempdir().unwrap();
        write(
            &temporary.path().join("spago.yaml"),
            r#"workspace: {}
package:
  name: root
"#,
        );
        write(
            &temporary.path().join("nested/spago.yaml"),
            r#"workspace: {}
package:
  name: nested
"#,
        );
        write(
            &temporary.path().join("nested/packages/child/spago.yaml"),
            r#"package:
  name: child
"#,
        );

        let workspace = Workspace::discover(temporary.path(), None).unwrap();
        assert_eq!(workspace.packages.keys().collect::<Vec<_>>(), vec!["root"]);
    }

    #[test]
    fn finds_ancestor_workspace() {
        let temporary = tempdir().unwrap();
        write(&temporary.path().join("spago.yaml"), "workspace: {}\n");
        let nested = temporary.path().join("packages/application");
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(
            Workspace::find_ancestor(&nested).unwrap(),
            Some(temporary.path().to_path_buf())
        );
        assert_eq!(Workspace::find_ancestor(temporary.path()).unwrap(), None);
    }
}
