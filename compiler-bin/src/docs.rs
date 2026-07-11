pub mod error;
mod location;

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::{env, fs, process};

use analyzer::{QueryEngine, prim};
use documentation::schema::Location;
use files::{FileId, Files};
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

use crate::docs::error::DocsError;
use crate::docs::location::{manifest_location, package_reference_location};
use crate::walk;

macro_rules! warm_modules {
    ($engine:expr, $modules:expr, $query:ident) => {
        $modules.par_iter().try_for_each(|&id| {
            let engine = $engine.snapshot();
            let _ = engine.$query(id)?;
            Ok::<(), DocsError>(())
        })
    };
}

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
    let mut compiler = Compiler::default();
    prim::configure(&mut compiler.engine, &mut compiler.files);

    let packages = load_packages(&config, &mut compiler)?;
    let modules = package_modules(&packages);

    warm_documentation_queries(&compiler.engine, &modules)?;

    if config.output.exists() {
        fs::remove_dir_all(&config.output)?;
    }

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

fn warm_documentation_queries(engine: &QueryEngine, modules: &[FileId]) -> Result<(), DocsError> {
    warm_modules!(engine, modules, indexed)?;
    warm_modules!(engine, modules, resolved)?;
    warm_modules!(engine, modules, lowered)?;
    warm_modules!(engine, modules, grouped)?;
    warm_modules!(engine, modules, bracketed)?;
    warm_modules!(engine, modules, sectioned)?;
    warm_modules!(engine, modules, checked)?;
    warm_modules!(engine, modules, documented)?;

    Ok(())
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
    if !package_root.is_dir() {
        return Err(DocsError::MissingPackageFolder(package_root));
    }

    let metadata = load_package_metadata(&package_root)?;
    let includes = package_include_globs(path, &metadata)?;
    let excludes = package_exclude_globs(path, &metadata)?;

    let walk::Walk { files, .. } = walk::walk_filtered(root, includes, excludes)?;

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

fn package_include_globs(
    package_root: &Path,
    metadata: &PackageMetadata,
) -> Result<Vec<PathBuf>, DocsError> {
    let mut includes =
        vec![package_root.join("src/**/*.purs"), package_root.join("test/**/*.purs")];

    for path in &metadata.include_files {
        validate_package_glob(path)?;
        includes.push(package_root.join(path));
    }

    Ok(includes)
}

fn package_exclude_globs(
    package_root: &Path,
    metadata: &PackageMetadata,
) -> Result<Vec<PathBuf>, DocsError> {
    let mut excludes = vec![];

    for path in &metadata.exclude_files {
        validate_package_glob(path)?;
        excludes.push(package_root.join(path));
    }

    Ok(excludes)
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

fn validate_package_glob(path: &str) -> Result<(), DocsError> {
    if !is_package_relative_path(path) {
        return Err(DocsError::InvalidPackageGlob(path.to_owned()));
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

fn is_package_relative_path(path: &str) -> bool {
    !path.is_empty() && Path::new(path).components().all(is_package_relative_component)
}

fn is_package_relative_component(component: Component<'_>) -> bool {
    match component {
        Component::Normal(_) | Component::CurDir => true,
        Component::Prefix(_) | Component::RootDir | Component::ParentDir => false,
    }
}

fn package_version(reference: &spago::PackageReference) -> String {
    match reference {
        spago::PackageReference::Workspace | spago::PackageReference::Local => "0.0.0".to_owned(),
        spago::PackageReference::Git { version, .. } => version.to_string(),
        spago::PackageReference::Registry { version } => version.to_string(),
    }
}

fn write_packages_manifest(
    engine: &QueryEngine,
    config: &DocsConfig,
    packages: &[Package],
) -> Result<(), DocsError> {
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
        let package = documentation::render_package_manifest(engine, &package_input)?;

        let package_folder = config.output.join(&package.name);
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
    let modules_folder = config.output.join(&package.name).join("modules");
    fs::create_dir_all(&modules_folder)?;

    for &id in &package.modules {
        let Some(module) = documentation::render_module(&compiler.engine, id, package_by_file)?
        else {
            continue;
        };

        let module_file = modules_folder.join(format!("{}.json", module.name));
        let module = serde_json::to_string_pretty(&module)?;

        fs::write(module_file, module)?;
    }

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
        let content = compiler.engine.content(id);
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
    fn package_globs_are_relative_to_the_package_root() {
        let metadata = PackageMetadata {
            include_files: vec!["examples/**/*.purs".to_owned()],
            exclude_files: vec!["test/Excluded.purs".to_owned()],
            ..PackageMetadata::default()
        };

        let includes = package_include_globs(Path::new("packages/effect"), &metadata).unwrap();
        let excludes = package_exclude_globs(Path::new("packages/effect"), &metadata).unwrap();

        insta::assert_debug_snapshot!(
            (includes, excludes),
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

    #[test]
    fn package_globs_reject_paths_outside_the_package_root() {
        let metadata = PackageMetadata {
            include_files: vec!["../Outside.purs".to_owned()],
            ..PackageMetadata::default()
        };

        assert!(matches!(
            package_include_globs(Path::new("packages/effect"), &metadata),
            Err(DocsError::InvalidPackageGlob(_))
        ));

        let metadata = PackageMetadata {
            exclude_files: vec!["/Outside.purs".to_owned()],
            ..PackageMetadata::default()
        };

        assert!(matches!(
            package_exclude_globs(Path::new("packages/effect"), &metadata),
            Err(DocsError::InvalidPackageGlob(_))
        ));
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
            Err(DocsError::MissingPackageFolder(_))
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
}
