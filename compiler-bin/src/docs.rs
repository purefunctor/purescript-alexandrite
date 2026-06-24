pub mod error;
pub mod schema;

use std::collections::LinkedList;
use std::path::PathBuf;
use std::{env, fs, process};

use analyzer::{QueryEngine, prim};
use checking::core::pretty::Pretty;
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

#[derive(Default)]
struct Compiler {
    files: Files,
    engine: QueryEngine,
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

    let modules = document_modules(&compiler)?;

    let has_output_folder = fs::exists(&config.output)?;
    if !has_output_folder {
        fs::create_dir_all(&config.output)?;
    };

    for module in modules {
        if let Some(name) = module.name.as_deref() {
            let name = format!("{}.json", name);
            let module = serde_json::to_string(&module)?;
            fs::write(&config.output.join(name), module)?;
        }
    }

    Ok(())
}

fn document_modules(compiler: &Compiler) -> Result<Vec<schema::Module>, DocsError> {
    let mut modules = vec![];

    for id in compiler.files.iter_id() {
        let mut items = vec![];

        let (parsed, _) = compiler.engine.parsed(id)?;
        let indexed = compiler.engine.indexed(id)?;
        let checked = compiler.engine.checked(id)?;

        for (id, item) in indexed.items.iter_terms() {
            let name = item.name.as_deref().map(str::to_string);
            let signature = checked.lookup_term(id).map(|id| {
                let mut pretty = Pretty::new(&compiler.engine, &checked);
                pretty.render(id).to_string()
            });
            items.push(schema::Item { name, signature, kind: schema::Kind::Term });
        }

        for (id, item) in indexed.items.iter_types() {
            let name = item.name.as_deref().map(str::to_string);
            let signature = checked.lookup_type(id).map(|id| {
                let mut pretty = Pretty::new(&compiler.engine, &checked);
                pretty.render(id).to_string()
            });
            items.push(schema::Item { name, signature, kind: schema::Kind::Type });
        }

        let name = parsed.module_name().as_deref().map(str::to_string);
        modules.push(schema::Module { name, items });
    }

    Ok(modules)
}
