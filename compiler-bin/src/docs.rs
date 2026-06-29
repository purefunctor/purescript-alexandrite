pub mod error;
pub mod schema;

use std::path::PathBuf;
use std::{env, fs, process};

use analyzer::{QueryEngine, prim};
use files::{FileId, Files};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::cli::{PackageSpec, PackageSpecs};
use crate::docs::error::DocsError;
use crate::walk;

pub struct DocsConfig {
    pub output: PathBuf,
    pub packages: PackageSpecs,
}

pub fn start(config: DocsConfig) {
    if let Err(error) = generate_documentation(config) {
        eprintln!("Documentation exited: {error}");
        tracing::error!(?error, "Documentation exited");
        process::exit(1);
    }
}

#[derive(Default)]
struct Compiler {
    files: Files,
    engine: QueryEngine,
}

struct Package {
    name: String,
    version: String,
    modules: Vec<FileId>,
}

fn generate_documentation(config: DocsConfig) -> Result<(), DocsError> {
    let mut compiler = Compiler::default();
    prim::configure(&mut compiler.engine, &mut compiler.files);

    let packages = load_packages(&config, &mut compiler)?;
    write_packages_manifest(&config, &mut compiler, &packages)?;

    for package in packages {
        generate_package_documentation(&config, &mut compiler, &package)?;
    }

    Ok(())
}

fn load_packages(config: &DocsConfig, compiler: &mut Compiler) -> Result<Vec<Package>, DocsError> {
    let root = env::current_dir()?;

    let mut packages = vec![];
    for PackageSpec { name, version, sources } in &config.packages.packages {
        let walk::Walk { files, .. } = walk::walk(&root, sources)?;

        let name = String::clone(name);
        let version = String::clone(version);
        let modules = load_modules(compiler, files)?;

        packages.push(Package { name, version, modules })
    }

    populate_module_file(compiler)?;

    Ok(packages)
}

fn write_packages_manifest(
    config: &DocsConfig,
    compiler: &mut Compiler,
    packages: &[Package],
) -> Result<(), DocsError> {
    let root = env::current_dir()?;

    for package in packages {
        let modules = package.modules.par_iter().map(|&id| {
            let (parsed, _) = compiler.engine.parsed(id)?;
            Ok(parsed.module_name().map(|name| name.to_string()))
        });

        let modules = modules.collect::<Result<Vec<_>, DocsError>>()?;
        let modules = modules.into_iter().flatten().collect();

        let name = String::clone(&package.name);
        let version = String::clone(&package.version);
        let package = schema::Package { name, version, modules };

        let package_folder = root.join(&config.output).join(&package.name);
        fs::create_dir_all(&package_folder)?;

        let manifest = package_folder.join("manifest.json");
        let package = serde_json::to_string(&package)?;
        fs::write(manifest, package)?;
    }

    Ok(())
}

fn generate_package_documentation(
    config: &DocsConfig,
    compiler: &mut Compiler,
    package: &Package,
) -> Result<(), DocsError> {
    let root = env::current_dir()?;

    let modules_folder = root.join(&config.output).join(&package.name).join("modules");
    fs::create_dir_all(&modules_folder)?;

    for &id in &package.modules {
        let (parsed, _) = compiler.engine.parsed(id)?;
        let indexed = compiler.engine.indexed(id)?;
        let documented = compiler.engine.documented(id)?;

        let Some(name) = parsed.module_name().map(|name| name.to_string()) else {
            continue;
        };

        let mut terms = vec![];
        let mut types = vec![];

        for (term_id, term_item) in indexed.items.iter_terms() {
            let term_documentation = documented.terms.get(&term_id);

            let name = term_item.name.as_ref().map(|name| name.to_string());
            let documentation = term_documentation.map(|t| t.documentation.to_string());
            let signature = term_documentation.map(|t| t.signature.to_string());

            let kind = match &term_item.kind {
                indexing::TermItemKind::ClassMember { .. } => schema::TermKind::ClassMember,
                indexing::TermItemKind::Constructor { .. } => schema::TermKind::Constructor,
                indexing::TermItemKind::Derive { .. } => schema::TermKind::Derive,
                indexing::TermItemKind::Foreign { .. } => schema::TermKind::Foreign,
                indexing::TermItemKind::Instance { .. } => schema::TermKind::Instance,
                indexing::TermItemKind::Operator { .. } => schema::TermKind::Operator,
                indexing::TermItemKind::Value { .. } => schema::TermKind::Value,
            };

            terms.push(schema::Term { name, documentation, signature, kind });
        }

        for (type_id, type_item) in indexed.items.iter_types() {
            let type_documentation = documented.types.get(&type_id);

            let name = type_item.name.as_ref().map(|name| name.to_string());
            let documentation = type_documentation.map(|t| t.documentation.to_string());
            let signature = type_documentation.map(|t| t.signature.to_string());

            let kind = match &type_item.kind {
                indexing::TypeItemKind::Data { .. } => schema::TypeKind::Data,
                indexing::TypeItemKind::Newtype { .. } => schema::TypeKind::Newtype,
                indexing::TypeItemKind::Synonym { .. } => schema::TypeKind::Synonym,
                indexing::TypeItemKind::Class { .. } => schema::TypeKind::Class,
                indexing::TypeItemKind::Foreign { .. } => schema::TypeKind::Foreign,
                indexing::TypeItemKind::Operator { .. } => schema::TypeKind::Operator,
            };

            types.push(schema::Type { name, documentation, signature, kind });
        }

        let module_file = modules_folder.join(format!("{name}.json"));

        let module = schema::Module { name, terms, types };
        let module = serde_json::to_string_pretty(&module)?;

        fs::write(module_file, module)?;
    }
    Ok(())
}

fn load_modules(compiler: &mut Compiler, files: Vec<PathBuf>) -> Result<Vec<FileId>, DocsError> {
    let mut modules = vec![];

    for file in &files {
        let url = url::Url::from_file_path(file).map_err(|_| {
            let file = PathBuf::clone(file);
            DocsError::PathParseFail(file)
        })?;

        let uri = url.as_str();
        let text = fs::read_to_string(file)?;

        let id = compiler.files.insert(uri, &*text);
        compiler.engine.set_content(id, &*text);

        modules.push(id);
    }

    Ok(modules)
}

fn populate_module_file(compiler: &mut Compiler) -> Result<(), DocsError> {
    let results = compiler.files.par_iter_id().map(|id| {
        let (parsed, _) = compiler.engine.parsed(id)?;
        Ok((id, parsed))
    });

    let results = results.collect::<Result<Vec<_>, DocsError>>()?;
    for (id, parsed) in results {
        if let Some(name) = parsed.module_name() {
            compiler.engine.set_module_file(&name, id);
        }
    }

    Ok(())
}
