pub mod error;
pub mod schema;
pub mod warm;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs, process};

use analyzer::{QueryEngine, prim};
use checking::PrettyQueries;
use checking::core::pretty::PrettyNames;
use files::{FileId, Files};
use itertools::Itertools;
use rayon::iter::ParallelIterator;
use serde::Deserialize;
use ts_rs::{Config as TypeScriptExportConfig, TS};

use crate::docs::error::DocsError;
use crate::docs::warm::warm_documentation_queries;
use crate::walk;

pub struct DocsConfig {
    pub output: PathBuf,
    pub spago_project: Option<PathBuf>,
    pub packages: Vec<PathBuf>,
}

pub struct TypeScriptConfig {
    pub output: PathBuf,
}

pub fn start(config: DocsConfig) {
    if let Err(error) = generate_documentation(config) {
        eprintln!("Documentation exited: {error}");
        tracing::error!(?error, "Documentation exited");
        process::exit(1);
    }
}

pub fn typescript(config: TypeScriptConfig) {
    if let Err(error) = write_typescript(config) {
        eprintln!("TypeScript schema generation exited: {error}");
        tracing::error!(?error, "TypeScript schema generation exited");
        process::exit(1);
    }
}

fn write_typescript(config: TypeScriptConfig) -> Result<(), DocsError> {
    let typescript = TypeScriptExportConfig::new().with_out_dir(config.output);

    schema::Package::export_all(&typescript)?;
    schema::Module::export_all(&typescript)?;

    Ok(())
}

#[derive(Default)]
struct Compiler {
    files: Files,
    engine: QueryEngine,
}

struct Package {
    name: String,
    version: String,
    license: Option<String>,
    description: Option<String>,
    dependencies: BTreeMap<String, String>,
    modules: Vec<FileId>,
}

#[derive(Debug, Default)]
struct PackageMetadata {
    name: Option<String>,
    version: Option<String>,
    license: Option<String>,
    description: Option<String>,
    include_files: Vec<String>,
    exclude_files: Vec<String>,
    dependencies: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PursManifest {
    name: String,
    version: String,
    license: Option<String>,
    description: Option<String>,
    #[serde(default)]
    include_files: Vec<String>,
    #[serde(default)]
    exclude_files: Vec<String>,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
}

struct TypeEncoder<'a> {
    engine: &'a QueryEngine,
    checked: &'a checking::CheckedModule,
    package_by_file: &'a [(FileId, &'a str)],
    names: PrettyNames,
}

impl<'a> TypeEncoder<'a> {
    fn new(
        engine: &'a QueryEngine,
        checked: &'a checking::CheckedModule,
        package_by_file: &'a [(FileId, &'a str)],
    ) -> TypeEncoder<'a> {
        TypeEncoder { engine, checked, package_by_file, names: PrettyNames::new() }
    }

    fn encode_signature(&mut self, id: checking::TypeId) -> Result<schema::Type, DocsError> {
        self.names.reset();
        self.encode_type(id)
    }

    fn encode_type(&mut self, id: checking::TypeId) -> Result<schema::Type, DocsError> {
        let expression = match self.engine.lookup_type(id) {
            checking::Type::Application(function, argument) => schema::Type::Application {
                function: self.encode_boxed_type(function)?,
                argument: self.encode_boxed_type(argument)?,
            },
            checking::Type::KindApplication(function, argument) => schema::Type::KindApplication {
                function: self.encode_boxed_type(function)?,
                argument: self.encode_boxed_type(argument)?,
            },
            checking::Type::Forall(binder, body) => schema::Type::Forall {
                binder: self.encode_binder(binder)?,
                body: self.encode_boxed_type(body)?,
            },
            checking::Type::Constrained(constraint, body) => schema::Type::Constrained {
                constraint: self.encode_boxed_type(constraint)?,
                body: self.encode_boxed_type(body)?,
            },
            checking::Type::Function(argument, result) => schema::Type::Function {
                argument: self.encode_boxed_type(argument)?,
                result: self.encode_boxed_type(result)?,
            },
            checking::Type::Kinded(expression, kind) => schema::Type::Kinded {
                expression: self.encode_boxed_type(expression)?,
                kind: self.encode_boxed_type(kind)?,
            },
            checking::Type::Constructor(file_id, type_id) => schema::Type::Constructor {
                reference: self.resolve_type_reference(file_id, type_id)?,
            },
            checking::Type::Integer(value) => schema::Type::Integer { value },
            checking::Type::String(kind, value_id) => {
                let kind = match kind {
                    lowering::StringKind::String => schema::StringLiteralKind::String,
                    lowering::StringKind::RawString => schema::StringLiteralKind::RawString,
                };
                let value = self.engine.lookup_smol_str(value_id).to_string();
                schema::Type::String { kind, value }
            }
            checking::Type::Row(row_id) => {
                let row = self.engine.lookup_row_type(row_id);
                let fields = row.fields.iter().map(|field| {
                    let t = self.encode_type(field.id)?;
                    Ok(schema::TypeRowField { label: field.label.to_string(), t })
                });

                let fields = fields.collect::<Result<Vec<_>, DocsError>>()?;
                let tail = row.tail.map(|id| self.encode_boxed_type(id)).transpose()?;

                schema::Type::Row { fields, tail }
            }
            checking::Type::Rigid(name, _, kind) => schema::Type::Rigid {
                name: self.display_name(name),
                kind: self.encode_boxed_type(kind)?,
            },
            checking::Type::Unification(id) => schema::Type::Unification { id },
            checking::Type::Free(name_id) => {
                schema::Type::Free { name: self.engine.lookup_smol_str(name_id).to_string() }
            }
            checking::Type::Unknown(name_id) => {
                schema::Type::Unknown { name: self.engine.lookup_smol_str(name_id).to_string() }
            }
        };

        Ok(expression)
    }

    fn encode_boxed_type(&mut self, id: checking::TypeId) -> Result<Box<schema::Type>, DocsError> {
        Ok(Box::new(self.encode_type(id)?))
    }

    fn encode_binder(
        &mut self,
        id: checking::core::ForallBinderId,
    ) -> Result<schema::TypeBinder, DocsError> {
        let binder = self.engine.lookup_forall_binder(id);
        let name = self.display_name(binder.name);
        let kind = self.encode_boxed_type(binder.kind)?;

        Ok(schema::TypeBinder { name, visible: binder.visible, kind })
    }

    fn resolve_type_reference(
        &self,
        file_id: FileId,
        type_id: indexing::TypeItemId,
    ) -> Result<schema::TypeReference, DocsError> {
        let package = self.package_by_file.iter().find_map(|&(id, package)| {
            if id == file_id { Some(package.to_string()) } else { None }
        });

        let (parsed, _) = self.engine.parsed(file_id)?;
        let module = parsed.module_name().map(|name| name.to_string());

        let indexed = self.engine.indexed(file_id)?;
        let name = indexed.items[type_id].name.as_ref().map(|name| name.to_string());

        Ok(schema::TypeReference { package, module, name })
    }

    fn display_name(&mut self, name: checking::core::Name) -> String {
        self.names.display_name(self.engine, &self.checked.names, name).to_string()
    }
}

fn generate_documentation(config: DocsConfig) -> Result<(), DocsError> {
    let mut compiler = Compiler::default();
    prim::configure(&mut compiler.engine, &mut compiler.files);

    let packages = load_packages(&config, &mut compiler)?;
    let modules = package_modules(&packages);

    warm_documentation_queries(&compiler.engine, &modules)?;
    write_packages_manifest(&compiler.engine, &config, &packages)?;

    let package_by_file = packages
        .iter()
        .flat_map(|package| package.modules.iter().map(|&id| (id, package.name.as_str())))
        .collect_vec();

    for package in &packages {
        generate_package_documentation(&config, &mut compiler, package, &package_by_file)?;
    }

    Ok(())
}

fn package_modules(packages: &[Package]) -> Vec<FileId> {
    packages.iter().flat_map(|package| package.modules.iter().copied()).collect_vec()
}

fn load_packages(config: &DocsConfig, compiler: &mut Compiler) -> Result<Vec<Package>, DocsError> {
    if let Some(spago_project) = &config.spago_project {
        return load_packages_from_spago_project(spago_project, compiler);
    }

    let root = env::current_dir()?;

    let mut packages = vec![];
    for path in &config.packages {
        let package = load_package_from_folder(compiler, &root, path, None, None)?;
        packages.push(package);
    }

    populate_module_file(compiler)?;

    Ok(packages)
}

fn load_packages_from_spago_project(
    spago_project: &Path,
    compiler: &mut Compiler,
) -> Result<Vec<Package>, DocsError> {
    let mut packages = vec![];
    let packages_by_source = spago::source_files_by_package(spago_project)?;

    for (name, sources) in packages_by_source {
        let name = name.to_string();
        let version = package_version(&sources.reference);
        let package = if let Some(package_root) = sources
            .roots
            .iter()
            .rev()
            .find(|root| spago_project.join(root).join("purs.json").is_file())
        {
            load_package_from_folder(
                compiler,
                spago_project,
                package_root,
                Some(name.clone()),
                Some(version.clone()),
            )?
        } else {
            let modules = load_modules(compiler, sources.sources)?;
            Package {
                name,
                version,
                license: None,
                description: None,
                dependencies: BTreeMap::new(),
                modules,
            }
        };

        packages.push(package);
    }

    populate_module_file(compiler)?;

    Ok(packages)
}

fn load_package_from_folder(
    compiler: &mut Compiler,
    root: &Path,
    path: &Path,
    name: Option<String>,
    version: Option<String>,
) -> Result<Package, DocsError> {
    let metadata = load_package_metadata(&root.join(path))?;
    let includes = package_include_globs(path, &metadata);
    let excludes = package_exclude_globs(path, &metadata);

    let walk::Walk { files, .. } = walk::walk_filtered(root, includes, excludes)?;

    let name = metadata.name.or(name).unwrap_or_else(|| fallback_package_name(path));
    let version = metadata.version.or(version).unwrap_or_else(|| "0.0.0".to_owned());
    let modules = load_modules(compiler, files)?;

    Ok(Package {
        name,
        version,
        license: metadata.license,
        description: metadata.description,
        dependencies: metadata.dependencies,
        modules,
    })
}

fn load_package_metadata(package_root: &Path) -> Result<PackageMetadata, DocsError> {
    let manifest = package_root.join("purs.json");
    if !manifest.exists() {
        return Ok(PackageMetadata::default());
    }

    let manifest = fs::read_to_string(manifest)?;
    let manifest: PursManifest = serde_json::from_str(&manifest)?;

    Ok(PackageMetadata {
        name: Some(manifest.name),
        version: Some(manifest.version),
        license: manifest.license,
        description: manifest.description,
        include_files: manifest.include_files,
        exclude_files: manifest.exclude_files,
        dependencies: manifest.dependencies,
    })
}

fn package_include_globs(package_root: &Path, metadata: &PackageMetadata) -> Vec<PathBuf> {
    let mut includes =
        vec![package_root.join("src/**/*.purs"), package_root.join("test/**/*.purs")];
    includes.extend(metadata.include_files.iter().map(|path| package_root.join(path)));
    includes
}

fn package_exclude_globs(package_root: &Path, metadata: &PackageMetadata) -> Vec<PathBuf> {
    metadata.exclude_files.iter().map(|path| package_root.join(path)).collect_vec()
}

fn fallback_package_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("package")
        .to_string()
}

fn package_version(reference: &spago::PackageReference) -> String {
    match reference {
        spago::PackageReference::Workspace | spago::PackageReference::Local => "0.0.0".to_owned(),
        spago::PackageReference::Git { rev } => rev.to_string(),
        spago::PackageReference::Registry { version } => version.to_string(),
    }
}

fn write_packages_manifest(
    engine: &QueryEngine,
    config: &DocsConfig,
    packages: &[Package],
) -> Result<(), DocsError> {
    let root = env::current_dir()?;

    for package in packages {
        let modules = package.modules.iter().map(|&id| {
            let (parsed, _) = engine.parsed(id)?;
            Ok(parsed.module_name().map(|name| name.to_string()))
        });

        let modules = modules.collect::<Result<Vec<_>, DocsError>>()?;

        let package = schema::Package {
            name: String::clone(&package.name),
            version: String::clone(&package.version),
            license: package.license.clone(),
            description: package.description.clone(),
            dependencies: BTreeMap::clone(&package.dependencies),
            modules: modules.into_iter().flatten().collect_vec(),
        };

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
    package_by_file: &[(FileId, &str)],
) -> Result<(), DocsError> {
    let root = env::current_dir()?;

    let modules_folder = root.join(&config.output).join(&package.name).join("modules");
    fs::create_dir_all(&modules_folder)?;

    for &id in &package.modules {
        let (parsed, _) = compiler.engine.parsed(id)?;
        let indexed = compiler.engine.indexed(id)?;
        let checked = compiler.engine.checked(id)?;
        let documented = compiler.engine.documented(id)?;

        let Some(name) = parsed.module_name().map(|name| name.to_string()) else {
            continue;
        };

        let mut terms = vec![];
        let mut types = vec![];
        let mut type_encoder = TypeEncoder::new(&compiler.engine, &checked, package_by_file);

        for (term_id, term_item) in indexed.items.iter_terms() {
            let term_documentation = documented.terms.get(&term_id);

            let name = term_item.name.as_ref().map(|name| name.to_string());
            let documentation = term_documentation.map(|t| t.documentation.to_string());
            let signature = checked
                .lookup_term(term_id)
                .map(|signature| type_encoder.encode_signature(signature))
                .transpose()?;

            let kind = match &term_item.kind {
                indexing::TermItemKind::ClassMember { .. } => schema::TermKind::ClassMember,
                indexing::TermItemKind::Constructor { .. } => schema::TermKind::Constructor,
                indexing::TermItemKind::Derive { .. } => schema::TermKind::Derive,
                indexing::TermItemKind::Foreign { .. } => schema::TermKind::Foreign,
                indexing::TermItemKind::Instance { .. } => schema::TermKind::Instance,
                indexing::TermItemKind::Operator { .. } => schema::TermKind::Operator,
                indexing::TermItemKind::Value { .. } => schema::TermKind::Value,
            };

            terms.push(schema::TermItem { name, documentation, signature, kind });
        }

        for (type_id, type_item) in indexed.items.iter_types() {
            let type_documentation = documented.types.get(&type_id);

            let name = type_item.name.as_ref().map(|name| name.to_string());
            let documentation = type_documentation.map(|t| t.documentation.to_string());
            let signature = checked
                .lookup_type(type_id)
                .map(|signature| type_encoder.encode_signature(signature))
                .transpose()?;

            let kind = match &type_item.kind {
                indexing::TypeItemKind::Data { .. } => schema::TypeKind::Data,
                indexing::TypeItemKind::Newtype { .. } => schema::TypeKind::Newtype,
                indexing::TypeItemKind::Synonym { .. } => schema::TypeKind::Synonym,
                indexing::TypeItemKind::Class { .. } => schema::TypeKind::Class,
                indexing::TypeItemKind::Foreign { .. } => schema::TypeKind::Foreign,
                indexing::TypeItemKind::Operator { .. } => schema::TypeKind::Operator,
            };

            types.push(schema::TypeItem { name, documentation, signature, kind });
        }

        let module_file = modules_folder.join(format!("{name}.json"));
        let documentation = Some(documented.documentation.to_string());
        let module = schema::Module { name, documentation, terms, types };
        let module = serde_json::to_string_pretty(&module)?;

        fs::write(module_file, module)?;
    }

    Ok(())
}

fn load_modules(compiler: &mut Compiler, files: Vec<PathBuf>) -> Result<Vec<FileId>, DocsError> {
    let mut modules = vec![];

    for file in &files {
        if !file.extension().is_some_and(|extension| extension == "purs") {
            continue;
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory() -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let directory = std::env::temp_dir().join(format!("alexandrite-docs-{nanos}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn purs_manifest_supplies_package_metadata() {
        let root = temporary_directory();
        fs::write(
            root.join("purs.json"),
            r#"{
              "name": "effect",
              "version": "4.0.0",
              "license": "BSD-3-Clause",
              "description": "Effect package",
              "includeFiles": ["examples/**/*.purs"],
              "excludeFiles": ["test/Excluded.purs"],
              "dependencies": { "prelude": ">=6.0.0 <7.0.0" }
            }"#,
        )
        .unwrap();

        let metadata = load_package_metadata(&root).unwrap();

        insta::assert_debug_snapshot!(metadata, @r#"
        PackageMetadata {
            name: Some(
                "effect",
            ),
            version: Some(
                "4.0.0",
            ),
            license: Some(
                "BSD-3-Clause",
            ),
            description: Some(
                "Effect package",
            ),
            include_files: [
                "examples/**/*.purs",
            ],
            exclude_files: [
                "test/Excluded.purs",
            ],
            dependencies: {
                "prelude": ">=6.0.0 <7.0.0",
            },
        }
        "#);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_globs_are_relative_to_the_package_root() {
        let metadata = PackageMetadata {
            include_files: vec!["examples/**/*.purs".to_owned()],
            exclude_files: vec!["test/Excluded.purs".to_owned()],
            ..PackageMetadata::default()
        };

        insta::assert_debug_snapshot!(
            (
                package_include_globs(Path::new("packages/effect"), &metadata),
                package_exclude_globs(Path::new("packages/effect"), &metadata),
            ),
            @r#"
        (
            [
                "packages/effect/src/**/*.purs",
                "packages/effect/test/**/*.purs",
                "packages/effect/examples/**/*.purs",
            ],
            [
                "packages/effect/test/Excluded.purs",
            ],
        )
        "#
        );
    }
}
