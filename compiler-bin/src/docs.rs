pub mod error;
mod location;

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;
use std::{env, fs, process};

use building::{QueryEngine, prim};
use documentation::schema::Location;
use files::{FileId, Files};
use indicatif::MultiProgress;
use itertools::Itertools;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

use crate::docs::error::DocsError;
use crate::docs::location::{manifest_location, package_reference_location};
use crate::{package, progress};

pub struct DocsConfig {
    pub output: PathBuf,
    pub spago_project: Option<PathBuf>,
    pub packages: Vec<PathBuf>,
    pub quiet: bool,
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
    documentation::export_typescript(config.output)?;
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
    location: Option<Location>,
    modules: Vec<FileId>,
}

struct RenderedPackage {
    manifest: documentation::schema::Package,
    modules: Vec<documentation::schema::Module>,
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
    location: Option<Location>,
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
    #[serde(default)]
    location: Option<PursLocation>,
    #[serde(default, rename = "ref")]
    reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PursLocation {
    GitHub {
        #[serde(rename = "githubOwner")]
        owner: String,
        #[serde(rename = "githubRepo")]
        repository: String,
        subdir: Option<String>,
    },
    Git {
        #[serde(rename = "gitUrl")]
        url: String,
        subdir: Option<String>,
    },
}

fn generate_documentation(config: DocsConfig) -> Result<(), DocsError> {
    let started = Instant::now();
    let show_progress = !config.quiet;
    let preparation_progress = progress::bar(1, "Preparing", show_progress);
    let mut compiler = Compiler::default();
    prim::configure(&mut compiler.engine, &mut compiler.files);

    let packages = load_packages(&config, &mut compiler)?;
    let modules = package_modules(&packages);
    preparation_progress.inc(1);
    progress::finish(&preparation_progress);

    prepare_documentation_queries(&compiler.engine, &modules, show_progress)?;
    let rendered = render_documentation(&compiler.engine, &packages, show_progress)?;
    write_documentation(&config.output, &rendered, show_progress)?;

    if show_progress {
        progress::report_completion(started.elapsed());
    }

    Ok(())
}

fn package_modules(packages: &[Package]) -> Vec<FileId> {
    packages.iter().flat_map(|package| package.modules.iter().copied()).collect_vec()
}

fn prepare_documentation_queries(
    engine: &QueryEngine,
    modules: &[FileId],
    show_progress: bool,
) -> Result<(), DocsError> {
    let progress = MultiProgress::new();
    progress.set_move_cursor(true);
    let analysing_progress = progress::phase(&progress, modules.len(), "Analyse", show_progress);
    let elaborating_progress =
        progress::phase(&progress, modules.len(), "Elaborate", show_progress);

    let analysed = modules.par_iter().map(|&file_id| {
        let engine = engine.snapshot();
        let module_name = analyse_module(&engine, file_id)?;
        if let Some(module_name) = &module_name {
            progress::set_message(&analysing_progress, module_name);
        }
        analysing_progress.inc(1);
        Ok::<_, DocsError>(module_name)
    });
    let module_names = analysed.collect::<Result<Vec<_>, _>>()?;
    progress::finish(&analysing_progress);

    let elaborated = modules.par_iter().zip(&module_names).map(|(&file_id, module_name)| {
        let engine = engine.snapshot();
        if let Some(module_name) = module_name {
            progress::set_message(&elaborating_progress, module_name);
        }
        engine.checked(file_id)?;
        elaborating_progress.inc(1);
        Ok::<_, DocsError>(())
    });
    elaborated.collect::<Result<Vec<_>, _>>()?;
    progress::finish(&elaborating_progress);

    Ok(())
}

fn analyse_module(engine: &QueryEngine, file_id: FileId) -> Result<Option<String>, DocsError> {
    let content = engine.content(file_id)?;
    let (parsed, _) = engine.parsed(file_id)?;
    let module_name = parsed.module_name(&content).map(|name| name.to_string());

    engine.indexed(file_id)?;
    engine.resolved(file_id)?;
    engine.lowered(file_id)?;
    engine.grouped(file_id)?;
    engine.bracketed(file_id)?;
    engine.sectioned(file_id)?;

    Ok(module_name)
}

fn load_packages(config: &DocsConfig, compiler: &mut Compiler) -> Result<Vec<Package>, DocsError> {
    if let Some(spago_project) = &config.spago_project {
        return load_packages_from_spago_project(spago_project, compiler);
    }

    let root = env::current_dir()?;

    let mut packages = vec![];
    for path in &config.packages {
        let package = load_package_from_folder(compiler, &root, path, None, None, None)?;
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
        let location = package_reference_location(&sources.reference);
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
                location.clone(),
            )?
        } else {
            validate_package_name(&name)?;

            let modules = load_modules(compiler, sources.sources)?;
            Package {
                name,
                version,
                license: None,
                description: None,
                dependencies: BTreeMap::new(),
                location,
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
    location: Option<Location>,
) -> Result<Package, DocsError> {
    let package_root = root.join(path);
    let metadata = load_package_metadata(&package_root)?;
    let files = package::source_files_with_globs(
        root,
        path,
        &metadata.include_files,
        &metadata.exclude_files,
    )?;

    let name = metadata.name.or(name).unwrap_or_else(|| fallback_package_name(path));
    validate_package_name(&name)?;

    let version = metadata.version.or(version).unwrap_or_else(|| "0.0.0".to_owned());
    let location = metadata.location.or(location);
    let modules = load_modules(compiler, files)?;

    Ok(Package {
        name,
        version,
        license: metadata.license,
        description: metadata.description,
        dependencies: metadata.dependencies,
        location,
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
    let location = manifest_location(manifest.location, manifest.reference);

    Ok(PackageMetadata {
        name: Some(manifest.name),
        version: Some(manifest.version),
        license: manifest.license,
        description: manifest.description,
        include_files: manifest.include_files,
        exclude_files: manifest.exclude_files,
        dependencies: manifest.dependencies,
        location,
    })
}

fn fallback_package_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("package")
        .to_string()
}

fn validate_package_name(name: &str) -> Result<(), DocsError> {
    if !is_single_path_segment(name) {
        return Err(DocsError::InvalidPackageName(name.to_owned()));
    }

    Ok(())
}

fn is_single_path_segment(path: &str) -> bool {
    if path.contains('/') || path.contains('\\') {
        return false;
    }

    let mut components = Path::new(path).components();
    let first_component = components.next();
    let extra_component = components.next();

    matches!(first_component, Some(Component::Normal(_))) && extra_component.is_none()
}

fn package_version(reference: &spago::PackageReference) -> String {
    match reference {
        spago::PackageReference::Workspace | spago::PackageReference::Local => "0.0.0".to_owned(),
        spago::PackageReference::Git { version, .. } => version.to_string(),
        spago::PackageReference::Registry { version } => version.to_string(),
    }
}

fn render_documentation(
    engine: &QueryEngine,
    packages: &[Package],
    show_progress: bool,
) -> Result<Vec<RenderedPackage>, DocsError> {
    let package_by_file = packages.iter().flat_map(|package| {
        package.modules.iter().map(|&file_id| (file_id, package.name.as_str()))
    });
    let package_by_file = package_by_file.collect_vec();
    let module_counts = packages.iter().map(|package| package.modules.len());
    let module_count = module_counts.sum();
    let progress = progress::bar(module_count, "Document", show_progress);

    let mut rendered = vec![];
    for package in packages {
        let package_input = documentation::PackageInput {
            name: &package.name,
            version: &package.version,
            license: package.license.as_deref(),
            description: package.description.as_deref(),
            dependencies: &package.dependencies,
            location: package.location.as_ref(),
            modules: &package.modules,
        };
        let manifest = documentation::render_package_manifest(engine, &package_input)?;

        progress::set_message(&progress, &package.name);
        let modules = package.modules.par_iter().map(|&file_id| {
            let engine = engine.snapshot();
            let module = documentation::render_module(&engine, file_id, &package_by_file)?;
            if let Some(module) = &module {
                progress::set_message(&progress, &module.name);
            }
            progress.inc(1);
            Ok::<_, DocsError>(module)
        });
        let modules = modules.collect::<Result<Vec<_>, _>>()?;
        let modules = modules.into_iter().flatten().collect_vec();

        rendered.push(RenderedPackage { manifest, modules });
    }
    progress::finish(&progress);

    Ok(rendered)
}

fn write_documentation(
    output: &Path,
    packages: &[RenderedPackage],
    show_progress: bool,
) -> Result<(), DocsError> {
    if output.exists() {
        fs::remove_dir_all(output)?;
    }

    let file_counts = packages.iter().map(|package| package.modules.len() + 1);
    let file_count = file_counts.sum();
    let progress = progress::bar(file_count, "Output", show_progress);

    for package in packages {
        let package_folder = output.join(&package.manifest.name);
        let modules_folder = package_folder.join("modules");
        fs::create_dir_all(&modules_folder)?;

        progress::set_message(&progress, &package.manifest.name);
        let manifest_file = package_folder.join("manifest.json");
        let manifest = serde_json::to_string(&package.manifest)?;
        fs::write(manifest_file, manifest)?;
        progress.inc(1);

        for module in &package.modules {
            progress::set_message(&progress, &module.name);
            let module_file = modules_folder.join(format!("{}.json", module.name));
            let module = serde_json::to_string_pretty(module)?;

            fs::write(module_file, module)?;
            progress.inc(1);
        }
    }
    progress::finish(&progress);

    Ok(())
}

fn load_modules(compiler: &mut Compiler, files: Vec<PathBuf>) -> Result<Vec<FileId>, DocsError> {
    let mut modules = vec![];

    for file in &files {
        if file.extension().is_none_or(|extension| extension != "purs") {
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
        let content = compiler.engine.content(id)?;
        let (parsed, _) = compiler.engine.parsed(id)?;
        Ok((id, content, parsed))
    });

    let results = results.collect::<Result<Vec<_>, DocsError>>()?;
    let mut module_files = BTreeMap::new();
    for (id, content, parsed) in results {
        if let Some(name) = parsed.module_name(&content) {
            let name = name.to_string();
            if module_files.insert(String::clone(&name), id).is_some() {
                return Err(DocsError::DuplicateModuleName(name));
            }
        }
    }

    for (name, id) in module_files {
        compiler.engine.set_module_file(&name, id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMPORARY_DIRECTORY_INDEX: AtomicUsize = AtomicUsize::new(0);

    fn temporary_directory() -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let index = TEMPORARY_DIRECTORY_INDEX.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!("alexandrite-docs-{nanos}-{index}"));
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
              "location": {
                "githubOwner": "purescript",
                "githubRepo": "purescript-effect"
              },
              "ref": "v4.0.0",
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
            location: Some(
                GitHub {
                    url: "https://github.com/purescript/purescript-effect",
                    owner: "purescript",
                    repository: "purescript-effect",
                    reference: Some(
                        "v4.0.0",
                    ),
                    subdir: None,
                },
            ),
        }
        "#);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_manifest_location_detects_github_urls() {
        let root = temporary_directory();
        fs::write(
            root.join("purs.json"),
            r#"{
              "name": "effect",
              "version": "4.0.0",
              "license": "BSD-3-Clause",
              "location": {
                "gitUrl": "https://github.com/purescript/purescript-effect.git",
                "subdir": "packages/effect"
              },
              "ref": "v4.0.0",
              "dependencies": {}
            }"#,
        )
        .unwrap();

        let metadata = load_package_metadata(&root).unwrap();

        insta::assert_debug_snapshot!(metadata.location, @r#"
        Some(
            GitHub {
                url: "https://github.com/purescript/purescript-effect",
                owner: "purescript",
                repository: "purescript-effect",
                reference: Some(
                    "v4.0.0",
                ),
                subdir: Some(
                    "packages/effect",
                ),
            },
        )
        "#);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_manifest_location_preserves_generic_git_urls() {
        let root = temporary_directory();
        fs::write(
            root.join("purs.json"),
            r#"{
              "name": "image",
              "version": "1.0.0",
              "license": "BSD-3-Clause",
              "location": {
                "gitUrl": "https://example.com/purefunctor/purescript-package.git",
                "subdir": "libs/package"
              },
              "ref": "v1.0.0",
              "dependencies": {}
            }"#,
        )
        .unwrap();

        let metadata = load_package_metadata(&root).unwrap();

        insta::assert_debug_snapshot!(metadata.location, @r#"
        Some(
            Git {
                url: "https://example.com/purefunctor/purescript-package.git",
                reference: Some(
                    "v1.0.0",
                ),
                subdir: Some(
                    "libs/package",
                ),
            },
        )
        "#);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_names_must_be_single_path_segments() {
        assert!(validate_package_name("effect").is_ok());
        assert!(matches!(
            validate_package_name("../effect"),
            Err(DocsError::InvalidPackageName(_))
        ));
        assert!(matches!(
            validate_package_name("scope/effect"),
            Err(DocsError::InvalidPackageName(_))
        ));
        assert!(matches!(validate_package_name("."), Err(DocsError::InvalidPackageName(_))));
    }

    #[test]
    fn missing_package_folders_are_rejected() {
        let root = temporary_directory();
        let mut compiler = Compiler::default();

        assert!(matches!(
            load_package_from_folder(&mut compiler, &root, Path::new("missing"), None, None, None),
            Err(DocsError::PackageError(package::PackageError::MissingFolder(_)))
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn documentation_generation_clears_stale_output_files() {
        let root = temporary_directory();
        let package = root.join("effect");
        let source = package.join("src/Main.purs");
        let output = root.join("generated");

        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(&output).unwrap();
        fs::write(package.join("purs.json"), r#"{ "name": "effect", "version": "1.0.0" }"#)
            .unwrap();
        fs::write(source, "module Main where\n").unwrap();
        fs::write(output.join("stale.json"), "{}").unwrap();

        generate_documentation(DocsConfig {
            output: output.clone(),
            spago_project: None,
            packages: vec![package],
            quiet: true,
        })
        .unwrap();

        assert!(!output.join("stale.json").exists());
        assert!(output.join("effect/manifest.json").is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_module_names_are_rejected_before_populating_the_engine() {
        let root = temporary_directory();
        let first = root.join("src/First.purs");
        let second = root.join("src/Second.purs");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::write(&first, "module Main where\n").unwrap();
        fs::write(&second, "module Main where\n").unwrap();

        let mut compiler = Compiler::default();
        load_modules(&mut compiler, vec![first, second]).unwrap();

        assert!(matches!(
            populate_module_file(&mut compiler),
            Err(DocsError::DuplicateModuleName(_))
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn analysis_propagates_query_failures() {
        let mut compiler = Compiler::default();
        let file_id = compiler.files.insert("file:///Main.purs", "module Main where\n");

        assert!(matches!(
            analyse_module(&compiler.engine, file_id),
            Err(DocsError::QueryError(building::QueryError::MissingContent {
                file_id: missing_file_id
            })) if missing_file_id == file_id
        ));
    }
}
