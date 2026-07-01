use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use globset::Glob;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use smol_str::SmolStr;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
pub struct Lockfile {
    pub workspace: Workspace,
    pub packages: Packages,
}

#[derive(Debug, Deserialize)]
pub struct Workspace {
    pub packages: FxHashMap<SmolStr, WorkspacePackage>,

    #[serde(default)]
    pub extra_packages: FxHashMap<SmolStr, ExtraPackage>,
}

#[derive(Debug, Deserialize)]
pub struct WorkspacePackage {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct ExtraPackage {
    pub subdir: Option<PathBuf>,

    /// Present in Spago lockfile schema for some extra-packages (e.g. local/path-based).
    /// Not currently used by `Lockfile::sources()`.
    pub path: Option<PathBuf>,
}

pub type Packages = FxHashMap<SmolStr, PackageEntry>;

pub type PackagesBySource = BTreeMap<SmolStr, PackageSources>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSources {
    pub reference: PackageReference,
    pub sources: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageReference {
    Workspace,
    Git { rev: SmolStr },
    Local,
    Registry { version: SmolStr },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PackageEntry {
    Git {
        rev: SmolStr,
        #[serde(default)]
        subdir: Option<PathBuf>,
    },
    Local {
        path: SmolStr,
    },
    Registry {
        version: SmolStr,
    },
}

impl Lockfile {
    pub fn sources(&self) -> impl Iterator<Item = PathBuf> {
        self.sources_by_package().into_values().flat_map(|package| package.sources)
    }

    pub fn sources_by_package(&self) -> PackagesBySource {
        let mut packages = PackagesBySource::new();

        for (name, package) in &self.workspace.packages {
            let sources = vec![package.path.join("src"), package.path.join("test")];
            packages.insert(
                SmolStr::clone(name),
                PackageSources { reference: PackageReference::Workspace, sources },
            );
        }

        let base = Path::new(".spago").join("p");
        let git_revisions = self.git_revisions();

        for (name, package) in &self.packages {
            let mut sources = Vec::new();
            let reference = match package {
                PackageEntry::Git { rev, subdir } => {
                    sources.push(base.join(name).join(rev).join("src"));
                    sources.push(base.join(name).join(rev).join("test"));

                    let subdir = subdir.as_ref().or_else(|| {
                        self.workspace
                            .extra_packages
                            .get(name)
                            .and_then(|extra_package| extra_package.subdir.as_ref())
                    });

                    let subdir = subdir.filter(|subdir| is_safe_subdir(subdir));

                    if let Some(subdir) = subdir {
                        sources.push(base.join(name).join(rev).join(subdir).join("src"));
                        sources.push(base.join(name).join(rev).join(subdir).join("test"));
                    }

                    PackageReference::Git { rev: short_revision(rev, &git_revisions) }
                }
                PackageEntry::Local { path } => {
                    let base = Path::new(path);
                    sources.push(base.join("src"));
                    sources.push(base.join("test"));
                    PackageReference::Local
                }
                PackageEntry::Registry { version } => {
                    let name_version = format!("{name}-{version}");
                    sources.push(base.join(&name_version).join("src"));
                    sources.push(base.join(&name_version).join("test"));
                    PackageReference::Registry { version: SmolStr::clone(version) }
                }
            };

            packages.insert(SmolStr::clone(name), PackageSources { reference, sources });
        }

        packages
    }

    pub fn walk(&self, root: impl AsRef<Path>) -> impl Iterator<Item = PathBuf> {
        self.sources().filter_map(with_root(root)).flat_map(find_purs_files)
    }

    pub fn walk_by_package(&self, root: impl AsRef<Path>) -> PackagesBySource {
        let root = root.as_ref();
        let packages = self.sources_by_package().into_iter().map(|(name, package)| {
            let files =
                package.sources.into_iter().filter_map(with_root(root)).flat_map(find_purs_files);

            let files = files.collect();
            (name, PackageSources { reference: package.reference, sources: files })
        });

        packages.collect()
    }

    fn git_revisions(&self) -> Vec<&SmolStr> {
        let revisions = self.packages.values().filter_map(|package| match package {
            PackageEntry::Git { rev, .. } => Some(rev),
            PackageEntry::Local { .. } | PackageEntry::Registry { .. } => None,
        });

        revisions.collect()
    }
}

fn short_revision(rev: &SmolStr, revisions: &[&SmolStr]) -> SmolStr {
    for index in 1..=rev.len() {
        let prefix = &rev[..index];
        if revisions.iter().all(|other| **other == *rev || !other.starts_with(prefix)) {
            return SmolStr::new(prefix);
        }
    }

    SmolStr::clone(rev)
}

fn with_root(root: impl AsRef<Path>) -> impl Fn(PathBuf) -> Option<PathBuf> {
    move |source| root.as_ref().join(source).canonicalize().ok()
}

fn is_safe_subdir(subdir: &Path) -> bool {
    if subdir.as_os_str().is_empty() {
        return false;
    }

    // Treat lockfile paths as untrusted input: only allow "normal" relative paths.
    !subdir.components().any(|component| match component {
        Component::Prefix(_) | Component::RootDir | Component::ParentDir | Component::CurDir => {
            true
        }
        Component::Normal(_) => false,
    })
}

fn find_purs_files(root: PathBuf) -> impl Iterator<Item = PathBuf> {
    let matcher = Glob::new("**/*.purs").unwrap().compile_matcher();
    WalkDir::new(root).into_iter().filter_map(move |entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if matcher.is_match(path) { Some(path.to_path_buf()) } else { None }
    })
}
