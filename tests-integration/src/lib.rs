pub mod fixtures;
pub mod generated;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use building::QueryEngine;
use files::{Files, ForeignFiles};
use glob::glob;
use prim_constants::MODULE_MAP;
use tempfile::TempDir;
use url::Url;

static PRIM_DIRECTORY: LazyLock<TempDir> =
    LazyLock::new(|| TempDir::new().expect("invariant violated: failed to create PRIM_DIRECTORY"));

fn configure_materialized_prim(engine: &QueryEngine, files: &mut Files) {
    for (name, content) in MODULE_MAP {
        let path = PRIM_DIRECTORY.path().join(format!("{name}.purs"));
        fs::write(&path, content).expect("invariant violated: failed to materialize Prim module");

        let uri = Url::from_file_path(path)
            .expect("invariant violated: failed to create Prim module file URL");
        let id = files.insert(uri.as_str(), *content);

        engine.set_content(id, *content);
        engine.set_module_file(name, id);
    }
}

fn load_file(
    engine: &mut QueryEngine,
    files: &mut Files,
    foreign_files: &mut ForeignFiles,
    path: &Path,
) {
    let url = Url::from_file_path(path).unwrap();
    let file = fs::read_to_string(path).unwrap();
    let file = file.replace("\r\n", "\n");

    let uri = url.to_string();
    let id = files.insert(uri, file);
    let content = files.content(id);

    engine.set_content(id, content.clone());
    let Ok((parsed, _)) = engine.parsed(id) else {
        return;
    };

    if let Some(name) = parsed.module_name(&content) {
        engine.set_module_file(&name, id);
    }

    let foreign_path = path.with_extension("js");
    if foreign_path.is_file() {
        let foreign_url = Url::from_file_path(&foreign_path).unwrap();
        let foreign_content = fs::read_to_string(foreign_path).unwrap();
        let foreign_content = foreign_content.replace("\r\n", "\n");
        let foreign_id = foreign_files.insert(foreign_url.as_str(), foreign_content);
        engine.set_foreign_content(foreign_id, foreign_files.content(foreign_id));
        engine.set_foreign_file(id, foreign_id);
    }
}

fn load_folder(folder: &Path) -> impl Iterator<Item = PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let packages = manifest.join(folder);
    let pattern = format!("{}/**/*.purs", packages.to_str().unwrap());
    glob(&pattern).unwrap().filter_map(Result::ok)
}

pub fn load_compiler(folder: &Path) -> (QueryEngine, Files) {
    let mut engine = QueryEngine::default();
    let mut files = Files::default();
    let mut foreign_files = ForeignFiles::default();
    configure_materialized_prim(&engine, &mut files);

    if folder.starts_with("fixtures/backend/")
        || folder.starts_with("fixtures/checking/")
        || folder.starts_with("fixtures/semantic/")
    {
        let prelude = Path::new("fixtures/checking/prelude");
        load_folder(prelude).for_each(|path| {
            load_file(&mut engine, &mut files, &mut foreign_files, &path);
        });
    }

    load_folder(folder).for_each(|path| {
        load_file(&mut engine, &mut files, &mut foreign_files, &path);
    });
    (engine, files)
}
