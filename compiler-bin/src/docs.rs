pub mod error;
pub mod schema;

use std::path::{Path, PathBuf};
use std::{env, fs, process};

use analyzer::{QueryEngine, prim};
use checking::PrettyQueries;
use checking::core::pretty::PrettyNames;
use files::{FileId, Files};
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use ts_rs::{Config as TypeScriptExportConfig, TS};

use crate::cli::{PackageSpec, PackageSpecs};
use crate::docs::error::DocsError;
use crate::walk;

pub struct DocsConfig {
    pub output: PathBuf,
    pub spago_project: Option<PathBuf>,
    pub packages: PackageSpecs,
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
    modules: Vec<FileId>,
}

struct TypeEncoder<'a> {
    engine: &'a QueryEngine,
    checked: &'a checking::CheckedModule,
    names: PrettyNames,
}

impl<'a> TypeEncoder<'a> {
    fn new(engine: &'a QueryEngine, checked: &'a checking::CheckedModule) -> TypeEncoder<'a> {
        TypeEncoder { engine, checked, names: PrettyNames::new() }
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
        let (parsed, _) = self.engine.parsed(file_id)?;
        let module = parsed.module_name().map(|name| name.to_string());
        let indexed = self.engine.indexed(file_id)?;
        let name = indexed.items[type_id].name.as_ref().map(|name| name.to_string());

        Ok(schema::TypeReference { module, name })
    }

    fn display_name(&mut self, name: checking::core::Name) -> String {
        self.names.display_name(self.engine, &self.checked.names, name).to_string()
    }
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
    if let Some(spago_project) = &config.spago_project {
        return load_packages_from_spago_project(spago_project, compiler);
    }

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

fn load_packages_from_spago_project(
    spago_project: &Path,
    compiler: &mut Compiler,
) -> Result<Vec<Package>, DocsError> {
    let mut packages = vec![];
    let packages_by_source = spago::source_files_by_package(spago_project)?;

    for (name, sources) in packages_by_source {
        let name = name.to_string();
        let version = package_version(&sources.reference);
        let modules = load_modules(compiler, sources.sources)?;

        packages.push(Package { name, version, modules });
    }

    populate_module_file(compiler)?;

    Ok(packages)
}

fn package_version(reference: &spago::PackageReference) -> String {
    match reference {
        spago::PackageReference::Workspace | spago::PackageReference::Local => "0.0.0".to_owned(),
        spago::PackageReference::Git { rev } => rev.to_string(),
        spago::PackageReference::Registry { version } => version.to_string(),
    }
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
        let modules = modules.into_iter().flatten().collect_vec();

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
        let checked = compiler.engine.checked(id)?;
        let documented = compiler.engine.documented(id)?;

        let Some(name) = parsed.module_name().map(|name| name.to_string()) else {
            continue;
        };

        let mut terms = vec![];
        let mut types = vec![];
        let mut type_encoder = TypeEncoder::new(&compiler.engine, &checked);

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
