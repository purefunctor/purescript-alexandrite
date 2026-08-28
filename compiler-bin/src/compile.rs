use std::collections::{BTreeSet, HashSet};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use std::{fs, io, process};

use building::{DiskObservation, QueryError, SourceUnitKey};
use diagnostics::Severity;
use files::{FileId, ForeignSourceKind};
use indicatif::MultiProgress;
use rayon::prelude::*;
use thiserror::Error;
use url::Url;

use crate::cli::ColorChoice;
use crate::compilation::CompilationState;
use crate::{package, progress, walk};

pub struct CompileConfig {
    pub output: PathBuf,
    pub inputs: Vec<PathBuf>,
    pub packages: Vec<PathBuf>,
    pub json_errors: bool,
    pub quiet: bool,
    pub color: ColorChoice,
}

pub(crate) struct BuildConfig<'a> {
    pub output: &'a Path,
    pub current_directory: &'a Path,
    pub color: bool,
    pub progress: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BuildOutcome {
    Succeeded(BTreeSet<PathBuf>),
    Diagnostics,
}

#[derive(Debug, Error)]
pub(crate) enum CompileError {
    #[error("compilation failed")]
    Diagnostics,
    #[error("failed to convert path to a file URL: {0}")]
    InvalidPath(PathBuf),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Package(#[from] package::PackageError),
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
    let started = Instant::now();
    let preparation_progress = progress::bar(1, "Preparing", !config.quiet);
    let current_directory = std::env::current_dir()?;
    let walked = walk::walk(&current_directory, &config.inputs)?;

    let mut source_paths = walked.files.into_iter().collect::<BTreeSet<_>>();
    for package in &config.packages {
        source_paths.extend(package::source_files(&current_directory, package)?);
    }

    let mut compilation = CompilationState::new();

    for path in source_paths {
        load_source(&mut compilation, &path)?;
    }
    let source_ids = compilation.input_source_ids();
    preparation_progress.inc(1);
    progress::finish(&preparation_progress);

    if source_ids.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no input files found").into());
    }

    let build_config = BuildConfig {
        output: &config.output,
        current_directory: &current_directory,
        color: use_color(config.color),
        progress: !config.quiet,
    };
    if matches!(build(&compilation, &build_config)?, BuildOutcome::Diagnostics) {
        return Err(CompileError::Diagnostics);
    }

    if !config.quiet {
        progress::report_completion(started.elapsed());
    }

    Ok(())
}

pub(crate) fn build(
    compilation: &CompilationState,
    config: &BuildConfig<'_>,
) -> Result<BuildOutcome, CompileError> {
    let source_ids = compilation.input_source_ids();
    let has_errors = report_diagnostics(compilation, &source_ids, config)?;
    if has_errors {
        return Ok(BuildOutcome::Diagnostics);
    }

    let modules = generate_modules(compilation, &source_ids, config.progress)?;
    let outputs = write_modules(compilation, &modules, config.output, config.progress)?;
    Ok(BuildOutcome::Succeeded(outputs))
}

pub(crate) fn load_source(
    compilation: &mut CompilationState,
    path: &Path,
) -> Result<(), CompileError> {
    let source_url =
        Url::from_file_path(path).map_err(|()| CompileError::InvalidPath(path.to_path_buf()))?;
    let foreign_path = path.with_extension("js");
    let foreign_url = Url::from_file_path(&foreign_path)
        .map_err(|()| CompileError::InvalidPath(foreign_path.clone()))?;

    let unit = SourceUnitKey::new(source_url.as_str(), foreign_url.as_str());

    let content = fs::read_to_string(path)?;
    compilation.observe_source(SourceUnitKey::clone(&unit), DiskObservation::Found(content.into()));

    let disk = match fs::read_to_string(&foreign_path) {
        Ok(content) => DiskObservation::Found(content.into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => DiskObservation::NotFound,
        Err(error) => return Err(error.into()),
    };

    compilation.observe_foreign(unit, ForeignSourceKind::JavaScript, disk);
    Ok(())
}

pub(crate) fn use_color(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Auto => {
            let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
            io::stderr().is_terminal() && !no_color
        }
        ColorChoice::Always => true,
        ColorChoice::Never => false,
    }
}

fn display_source_path(source_path: &str, current_directory: &Path) -> String {
    if let Some(file_path) = Url::parse(source_path).ok().and_then(|url| url.to_file_path().ok()) {
        file_path.strip_prefix(current_directory).unwrap_or(&file_path).display().to_string()
    } else {
        source_path.to_owned()
    }
}

fn report_diagnostics(
    compilation: &CompilationState,
    source_ids: &[FileId],
    config: &BuildConfig<'_>,
) -> Result<bool, CompileError> {
    let progress = MultiProgress::new();
    progress.set_move_cursor(true);
    let analysing_progress =
        progress::phase(&progress, source_ids.len(), "Analyse", config.progress);
    let checking_progress =
        progress::phase(&progress, source_ids.len(), "Elaborate", config.progress);

    let module_names = source_ids.par_iter().map(|&file_id| {
        let engine = compilation.snapshot();
        let content = engine.content(file_id)?;
        let (parsed, _) = engine.parsed(file_id)?;
        let module_name = parsed.module_name(&content);
        if let Some(module_name) = &module_name {
            progress::set_message(&analysing_progress, module_name);
        }
        engine.stabilized(file_id)?;
        engine.indexed(file_id)?;
        engine.resolved(file_id)?;
        engine.lowered(file_id)?;
        analysing_progress.inc(1);
        Ok::<_, CompileError>(module_name)
    });
    let module_names = module_names.collect::<Result<Vec<_>, _>>()?;
    progress::finish(&analysing_progress);

    let elaborated = source_ids.par_iter().zip(&module_names).map(|(&file_id, module_name)| {
        let engine = compilation.snapshot();
        if let Some(module_name) = module_name {
            progress::set_message(&checking_progress, module_name);
        }
        engine.checked(file_id)?;
        engine.foreign_validation(file_id)?;
        checking_progress.inc(1);
        Ok::<_, CompileError>(())
    });
    elaborated.collect::<Result<Vec<_>, _>>()?;
    progress::finish(&checking_progress);

    let engine = compilation.snapshot();
    let diagnostics = diagnostics::collect_diagnostics(&engine, source_ids)?;
    let has_errors = diagnostics
        .iter()
        .flat_map(diagnostics::DiagnosticCollection::diagnostics)
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    let has_diagnostics = diagnostics.iter().any(|collected| !collected.diagnostics().is_empty());

    if has_diagnostics {
        if config.progress && io::stderr().is_terminal() {
            eprint!("\n\n");
        } else if !config.progress {
            eprintln!();
        }
    }
    for collected in diagnostics {
        if collected.diagnostics().is_empty() {
            continue;
        }
        let source_path =
            compilation.source_path(collected.file_id).expect("input source has no lifecycle path");
        let display_path = display_source_path(&source_path, config.current_directory);
        let rendered = diagnostics::format_rich_with_path(
            collected.diagnostics(),
            &collected.content,
            &display_path,
            config.color,
        );
        eprint!("{rendered}");
    }
    Ok(has_errors)
}

fn generate_modules(
    compilation: &CompilationState,
    source_ids: &[FileId],
    show_progress: bool,
) -> Result<Vec<Arc<javascript::Module>>, CompileError> {
    let mut pending = source_ids.to_vec();
    let mut visited = HashSet::new();

    let initial_total = source_ids.iter().copied().collect::<HashSet<_>>().len();
    let progress = progress::bar(initial_total, "Codegen", show_progress);
    let mut first_frontier = true;
    let mut modules = vec![];
    while !pending.is_empty() {
        let frontier = pending.drain(..).filter(|file_id| visited.insert(*file_id));
        let frontier = frontier.collect::<Vec<_>>();
        if first_frontier {
            first_frontier = false;
        } else {
            let frontier_len = frontier.len() as u64;
            progress.inc_length(frontier_len);
        }

        let generated = frontier.par_iter().map(|&file_id| {
            let engine = compilation.snapshot();
            let content = engine.content(file_id)?;
            let (parsed, _) = engine.parsed(file_id)?;
            if let Some(module_name) = parsed.module_name(&content) {
                progress::set_message(&progress, &module_name);
            }
            let module = engine.javascript(file_id)?.map_err(|source| {
                let path = compilation
                    .source_path(file_id)
                    .expect("invariant violated: generated module has no lifecycle path");
                CompileError::JavaScript { path, source }
            })?;
            progress.inc(1);
            Ok::<_, CompileError>(module)
        });
        let generated = generated.collect::<Result<Vec<_>, _>>()?;

        for module in generated {
            pending.extend(module.dependencies().iter().copied());
            modules.push(module);
        }
    }
    progress::finish(&progress);

    Ok(modules)
}

fn write_modules(
    compilation: &CompilationState,
    modules: &[Arc<javascript::Module>],
    output: &Path,
    show_progress: bool,
) -> Result<BTreeSet<PathBuf>, CompileError> {
    let mut outputs = BTreeSet::new();
    if modules.iter().any(|module| module.requires_runtime()) {
        let runtime = output.join(javascript::runtime_filename());
        fs::create_dir_all(output)?;
        write_if_changed(&runtime, javascript::runtime_source().as_bytes())?;
        outputs.insert(runtime);
    }

    let progress = progress::bar(modules.len(), "Output", show_progress);
    let module_outputs = modules.par_iter().map(|module| -> Result<Vec<PathBuf>, CompileError> {
        progress::set_message(&progress, module.name());
        let output_path = output.join(module.filename());
        let output_parent =
            output_path.parent().expect("invariant violated: module filename has no parent");

        fs::create_dir_all(output_parent)?;
        write_if_changed(&output_path, module.source().as_bytes())?;
        let mut outputs = vec![output_path];

        if !module.requires_foreign() {
            progress.inc(1);
            return Ok(outputs);
        }

        let output_path = output.join(module.foreign_filename());
        let foreign = compilation
            .source_foreign_content(module.file_id())
            .expect("invariant violated: generated module requires missing foreign content");
        write_if_changed(&output_path, foreign.as_bytes())?;
        outputs.push(output_path);
        progress.inc(1);
        Ok(outputs)
    });
    let module_outputs = module_outputs.collect::<Result<Vec<_>, _>>()?;
    outputs.extend(module_outputs.into_iter().flatten());
    progress::finish(&progress);
    Ok(outputs)
}

fn write_if_changed(path: &Path, content: &[u8]) -> io::Result<()> {
    match fs::read(path) {
        Ok(previous) if previous == content => Ok(()),
        Ok(_) => fs::write(path, content),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::write(path, content),
        Err(error) => Err(error),
    }
}
