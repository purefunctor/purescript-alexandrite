use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io, process};

use building::{
    DiskObservation, FileLifecycle, ForeignEvent, LifecycleEvent, QueryEngine, QueryError,
    SourceEvent, SourceUnitKey,
};
use diagnostics::{DiagnosticsContext, Severity, ToDiagnostics};
use files::FileId;
use prim_constants::MODULE_MAP;
use thiserror::Error;
use url::Url;

use crate::walk;

pub struct CompileConfig {
    pub output: PathBuf,
    pub inputs: Vec<PathBuf>,
    pub json_errors: bool,
}

#[derive(Debug, Error)]
enum CompileError {
    #[error("compilation failed")]
    Diagnostics,
    #[error("failed to convert path to a file URL: {0}")]
    InvalidPath(PathBuf),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    Walk(#[from] walk::Error),
    #[error("failed to generate JavaScript for {path}: {source}")]
    JavaScript {
        path: Arc<str>,
        #[source]
        source: javascript::ModuleError,
    },
}

pub fn start(config: CompileConfig) {
    let json_errors = config.json_errors;
    if let Err(error) = compile(config) {
        if !matches!(error, CompileError::Diagnostics) {
            eprintln!("Compilation exited: {error}");
        }
        tracing::error!(?error, "Compilation exited");
        if json_errors {
            println!(
                r#"{{"warnings":[],"errors":[{{"message":{message:?}}}]}}"#,
                message = error.to_string()
            );
        }
        process::exit(1);
    }

    if json_errors {
        println!(r#"{{"warnings":[],"errors":[]}}"#);
    }
}

fn compile(config: CompileConfig) -> Result<(), CompileError> {
    let current_directory = std::env::current_dir()?;
    let walked = walk::walk(&current_directory, &config.inputs)?;

    let engine = QueryEngine::default();
    let mut files = FileLifecycle::<(), ()>::default();
    configure_prim(&engine, &mut files);

    let mut source_ids = vec![];
    for path in walked.files {
        let source_id = load_source(&engine, &mut files, &path)?;
        source_ids.push(source_id);
    }

    if source_ids.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no input files found").into());
    }

    let has_errors = report_diagnostics(&engine, &files, &source_ids)?;
    if has_errors {
        return Err(CompileError::Diagnostics);
    }

    let modules = generate_modules(&engine, &files, &source_ids)?;
    write_modules(&files, &modules, &config.output)?;

    Ok(())
}

fn configure_prim(engine: &QueryEngine, files: &mut FileLifecycle<(), ()>) {
    for (name, content) in MODULE_MAP {
        let source = format!("prim://localhost/{name}.purs");
        let foreign = format!("prim://localhost/{name}.js");

        let event = LifecycleEvent::Source {
            unit: SourceUnitKey::new(source, foreign),
            event: SourceEvent::DiskObserved {
                disk: DiskObservation::Found(Arc::from(*content)),
                metadata: (),
            },
        };

        let change = files.apply(engine, event);
        let id = change
            .changed_sources()
            .next()
            .expect("invariant violated: Prim source lifecycle did not insert a source");

        engine.set_module_file(name, id);
    }
}

fn load_source(
    engine: &QueryEngine,
    files: &mut FileLifecycle<(), ()>,
    path: &Path,
) -> Result<FileId, CompileError> {
    let source_url =
        Url::from_file_path(path).map_err(|()| CompileError::InvalidPath(path.to_path_buf()))?;
    let foreign_path = path.with_extension("js");
    let foreign_url = Url::from_file_path(&foreign_path)
        .map_err(|()| CompileError::InvalidPath(foreign_path.clone()))?;

    let unit = SourceUnitKey::new(source_url.as_str(), foreign_url.as_str());

    let content = fs::read_to_string(path)?;
    let event = LifecycleEvent::Source {
        unit: SourceUnitKey::clone(&unit),
        event: SourceEvent::DiskObserved {
            disk: DiskObservation::Found(content.into()),
            metadata: (),
        },
    };

    let change = files.apply(engine, event);
    let source_id = change
        .changed_sources()
        .next()
        .expect("invariant violated: source lifecycle did not insert an input source");

    let disk = match fs::read_to_string(&foreign_path) {
        Ok(content) => DiskObservation::Found(content.into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => DiskObservation::NotFound,
        Err(error) => return Err(error.into()),
    };

    let event = LifecycleEvent::Foreign { unit, event: ForeignEvent::DiskObserved { disk } };
    files.apply(engine, event);
    Ok(source_id)
}

fn report_diagnostics(
    engine: &QueryEngine,
    files: &FileLifecycle<(), ()>,
    source_ids: &[FileId],
) -> Result<bool, CompileError> {
    let mut has_errors = false;

    for &file_id in source_ids {
        let content = engine.content(file_id)?;
        let (parsed, _) = engine.parsed(file_id)?;
        let root = parsed.syntax_node();

        let stabilized = engine.stabilized(file_id)?;
        let indexed = engine.indexed(file_id)?;
        let resolved = engine.resolved(file_id)?;
        let lowered = engine.lowered(file_id)?;
        let checked = engine.checked(file_id)?;
        let foreign = engine.foreign_validation(file_id)?;

        let context = DiagnosticsContext::new(
            engine,
            &content,
            &root,
            &stabilized,
            &indexed,
            &lowered,
            &checked,
        );

        let mut all = vec![];
        for error in &lowered.errors {
            all.extend(error.to_diagnostics(&context));
        }
        for error in &resolved.errors {
            all.extend(error.to_diagnostics(&context));
        }
        for error in &checked.errors {
            all.extend(error.to_diagnostics(&context));
        }
        for error in foreign.errors.iter() {
            all.extend(error.to_diagnostics(&context));
        }

        has_errors |= all.iter().any(|diagnostic| diagnostic.severity == Severity::Error);
        if !all.is_empty() {
            let path = files.source_path(file_id).expect("input source has no lifecycle path");
            let display_path = Url::parse(&path)
                .ok()
                .and_then(|url| url.to_file_path().ok())
                .map_or_else(|| path.to_string(), |path| path.display().to_string());

            let rendered = diagnostics::format_rustc_with_path(&all, &content, &display_path);
            eprint!("{rendered}");
        }
    }

    Ok(has_errors)
}

fn generate_modules(
    engine: &QueryEngine,
    files: &FileLifecycle<(), ()>,
    source_ids: &[FileId],
) -> Result<Vec<Arc<javascript::Module>>, CompileError> {
    let mut pending = source_ids.to_vec();
    let mut visited = HashSet::new();

    let mut modules = vec![];
    while let Some(file_id) = pending.pop() {
        if !visited.insert(file_id) {
            continue;
        }
        let module = engine.javascript(file_id)?.map_err(|source| {
            let path = files
                .source_path(file_id)
                .expect("invariant violated: generated module has no lifecycle path");
            CompileError::JavaScript { path, source }
        })?;
        pending.extend(module.dependencies().iter().copied());
        modules.push(module);
    }

    Ok(modules)
}

fn write_modules(
    files: &FileLifecycle<(), ()>,
    modules: &[Arc<javascript::Module>],
    output: &Path,
) -> Result<(), CompileError> {
    if modules.iter().any(|module| module.requires_runtime()) {
        let runtime = output.join(javascript::runtime_filename());
        fs::create_dir_all(output)?;
        fs::write(runtime, javascript::runtime_source())?;
    }

    for module in modules {
        let output_path = output.join(module.filename());
        let output_parent =
            output_path.parent().expect("invariant violated: module filename has no parent");

        fs::create_dir_all(output_parent)?;
        fs::write(output_path, module.source())?;

        if !module.requires_foreign() {
            continue;
        }

        let source_locator = files
            .source_path(module.file_id())
            .expect("invariant violated: generated module has no lifecycle path");
        let source_url = Url::parse(&source_locator)
            .map_err(|_| CompileError::InvalidPath(PathBuf::from(source_locator.as_ref())))?;
        let source_path = source_url
            .to_file_path()
            .map_err(|()| CompileError::InvalidPath(PathBuf::from(source_locator.as_ref())))?;

        let foreign_path = source_path.with_extension("js");
        let output_path = output.join(module.foreign_filename());
        fs::copy(foreign_path, output_path)?;
    }
    Ok(())
}
