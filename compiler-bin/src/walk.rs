use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use path_absolutize::Absolutize;
use walkdir::WalkDir;

pub struct Walk {
    pub roots: BTreeSet<PathBuf>,
    pub globs: GlobSet,
    pub files: Vec<PathBuf>,
}

pub fn walk(
    root: &Path,
    paths: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<Walk, globset::Error> {
    let mut files = vec![];

    let mut roots = BTreeSet::default();
    let mut globs = GlobSetBuilder::new();

    for path in paths {
        let path = root.join(path);
        if let Ok(path) = path.absolutize()
            && let Some(path) = path.to_str()
            && let Ok(glob) = Glob::new(path)
        {
            roots.insert(glob_literal_base(path));
            globs.add(glob);
        } else {
            files.push(path);
        }
    }

    let globs = globs.build()?;

    let files_from_glob: BTreeSet<PathBuf> = roots
        .iter()
        .flat_map(|base| WalkDir::new(base).into_iter())
        .filter_map(|entry| Some(entry.ok()?.into_path()))
        .filter(|path| !globs.matches(path).is_empty())
        .collect();

    files.extend(files_from_glob);

    Ok(Walk { roots, globs, files })
}

fn glob_literal_base(pattern: &str) -> PathBuf {
    let mut base = PathBuf::new();
    for component in Path::new(pattern).components() {
        if component.as_os_str().to_string_lossy().chars().any(glob_syntax_character) {
            break;
        }
        base.push(component);
    }
    base
}

fn glob_syntax_character(character: char) -> bool {
    matches!(character, '*' | '?' | '[' | '{')
        || (character == '\\' && !std::path::is_separator('\\'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_base_stops_at_the_first_wildcard() {
        let base = glob_literal_base("/workspace/src/**/*.purs");
        assert_eq!(base, PathBuf::from("/workspace/src"));
    }

    #[test]
    fn literal_base_excludes_a_wildcard_in_the_final_component() {
        let base = glob_literal_base("/workspace/src/*.purs");
        assert_eq!(base, PathBuf::from("/workspace/src"));
    }

    #[test]
    fn literal_base_retains_parent_directories() {
        let base = glob_literal_base("/workspace/../shared/src/**/*.purs");
        assert_eq!(base, PathBuf::from("/workspace/../shared/src"));
    }

    #[test]
    fn literal_base_of_a_pattern_without_wildcards_is_the_whole_path() {
        let base = glob_literal_base("/workspace/src/Main.purs");
        assert_eq!(base, PathBuf::from("/workspace/src/Main.purs"));
    }

    #[test]
    fn literal_base_recognises_every_metacharacter() {
        for pattern in ["/a/b/*.purs", "/a/b/?.purs", "/a/b/[abc].purs", "/a/b/{x,y}.purs"] {
            assert_eq!(glob_literal_base(pattern), PathBuf::from("/a/b"), "pattern: {pattern}");
        }
    }

    #[test]
    fn literal_base_keeps_class_closer_as_a_literal() {
        let pattern = "/workspace/src/Main].purs";

        assert!(Glob::new(pattern).is_ok());
        assert_eq!(glob_literal_base(pattern), PathBuf::from(pattern));
    }

    #[cfg(unix)]
    #[test]
    fn literal_base_stops_at_backslash_escape() {
        let pattern = "/workspace/src/\\*.purs";

        assert!(Glob::new(pattern).is_ok());
        assert_eq!(glob_literal_base(pattern), PathBuf::from("/workspace/src"));
    }
}
