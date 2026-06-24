pub mod error;

use std::collections::LinkedList;
use std::path::PathBuf;
use std::{env, fs, process};

use analyzer::{QueryEngine, prim};
use files::Files;
use rayon::iter::ParallelIterator;

use crate::cli::PackageSpecs;
use crate::docs::error::DocsError;
use crate::walk;

pub struct DocsConfig {
    pub output: PathBuf,
    pub packages: PackageSpecs,
}

pub fn start(config: DocsConfig) {
    if let Err(error) = generate_documentation(config) {
        tracing::error!(?error, "Documentation exited");
        process::exit(1);
    }
}

fn generate_documentation(config: DocsConfig) -> Result<(), DocsError> {
    let root = env::current_dir()?;

    let paths = config.packages.packages.into_iter().flat_map(|package| package.sources);
    let walk::Walk { files, .. } = walk::walk(&root, paths)?;

    let mut compiler = Compiler::default();
    prim::configure(&mut compiler.engine, &mut compiler.files);

    for file in &files {
        let url = url::Url::from_file_path(file).map_err(|_| {
            let file = PathBuf::clone(file);
            DocsError::PathParseFail(file)
        })?;

        let uri = url.as_str();
        let text = fs::read_to_string(file)?;

        let id = compiler.files.insert(uri, &*text);
        compiler.engine.set_content(id, &*text);
    }

    let results = compiler.files.par_iter_id().map(|id| {
        let (parsed, _) = compiler.engine.parsed(id)?;
        Ok((id, parsed))
    });

    let results: LinkedList<Vec<Result<_, DocsError>>> = results.collect_vec_list();

    for entry in results.into_iter().flatten() {
        let (id, parsed) = entry?;
        if let Some(name) = parsed.module_name() {
            compiler.engine.set_module_file(&name, id);
        }
    }

    Ok(())
}

#[derive(Default)]
struct Compiler {
    files: Files,
    engine: QueryEngine,
}
