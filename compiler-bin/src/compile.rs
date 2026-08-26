use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{fmt, fs, io, process};

use building::{
    DiskObservation, FileLifecycle, ForeignEvent, LifecycleEvent, QueryEngine, QueryError,
    SourceEvent, SourceUnitKey,
};
use console::{Color, Style};
use diagnostics::{DiagnosticsContext, Severity, ToDiagnostics};
use files::FileId;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use prim_constants::MODULE_MAP;
use rayon::prelude::*;
use thiserror::Error;
use url::Url;

use crate::cli::ColorChoice;
use crate::walk;

const CARGO_PROGRESS_REGION_WIDTH: usize = 50;
const CARGO_PROGRESS_FIXED_OVERHEAD: usize = 17;
const SHIMMER_FRAME_INTERVAL: Duration = Duration::from_millis(80);

pub struct CompileConfig {
    pub output: PathBuf,
    pub inputs: Vec<PathBuf>,
    pub json_errors: bool,
    pub diagnostic_limit: Option<usize>,
    pub color: ColorChoice,
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
    let started = Instant::now();
    let preparation_progress = compilation_progress(1, "Preparing");
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
    preparation_progress.inc(1);
    finish_progress(&preparation_progress);

    if source_ids.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no input files found").into());
    }

    let color = match config.color {
        ColorChoice::Auto => {
            let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
            io::stderr().is_terminal() && !no_color
        }
        ColorChoice::Always => true,
        ColorChoice::Never => false,
    };
    let has_errors = report_diagnostics(
        &engine,
        &files,
        &source_ids,
        &current_directory,
        config.diagnostic_limit,
        color,
    )?;
    if has_errors {
        return Err(CompileError::Diagnostics);
    }

    let modules = generate_modules(&engine, &files, &source_ids)?;
    write_modules(&files, &modules, &config.output)?;
    report_completion(started.elapsed());

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

fn display_source_path(source_path: &str, current_directory: &Path) -> String {
    if let Some(file_path) = Url::parse(source_path).ok().and_then(|url| url.to_file_path().ok()) {
        file_path.strip_prefix(current_directory).unwrap_or(&file_path).display().to_string()
    } else {
        source_path.to_owned()
    }
}

fn report_diagnostics(
    engine: &QueryEngine,
    files: &FileLifecycle<(), ()>,
    source_ids: &[FileId],
    current_directory: &Path,
    diagnostic_limit: Option<usize>,
    color: bool,
) -> Result<bool, CompileError> {
    let progress = MultiProgress::new();
    progress.set_move_cursor(true);
    let analysing_progress = phase_progress(&progress, source_ids.len(), "Analyse");
    let checking_progress = phase_progress(&progress, source_ids.len(), "Elaborate");

    let diagnostics = source_ids.par_iter().map(|&file_id| {
        let engine = engine.snapshot();
        let content = engine.content(file_id)?;
        let (parsed, _) = engine.parsed(file_id)?;
        let module_name = parsed.module_name(&content);
        if let Some(module_name) = &module_name {
            set_progress_module(&analysing_progress, module_name);
        }
        let root = parsed.syntax_node();

        let stabilized = engine.stabilized(file_id)?;
        let indexed = engine.indexed(file_id)?;
        let resolved = engine.resolved(file_id)?;
        let lowered = engine.lowered(file_id)?;
        analysing_progress.inc(1);
        if let Some(module_name) = &module_name {
            set_progress_module(&checking_progress, module_name);
        }
        let checked = engine.checked(file_id)?;
        let foreign = engine.foreign_validation(file_id)?;
        checking_progress.inc(1);

        let context = DiagnosticsContext::new(
            &engine,
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

        let has_errors = all.iter().any(|diagnostic| diagnostic.severity == Severity::Error);
        let display_path = if all.is_empty() {
            None
        } else {
            let source_path =
                files.source_path(file_id).expect("input source has no lifecycle path");
            Some(display_source_path(&source_path, current_directory))
        };
        Ok::<_, CompileError>((has_errors, all, content, display_path))
    });
    let diagnostics = diagnostics.collect::<Result<Vec<_>, _>>()?;
    finish_progress(&analysing_progress);
    finish_progress(&checking_progress);

    let mut has_errors = false;
    let mut remaining = diagnostic_limit.unwrap_or(usize::MAX);
    let mut omitted = 0;
    let mut rendered_any = false;
    let separate_from_progress = io::stderr().is_terminal();
    for (file_has_errors, all, content, display_path) in diagnostics {
        has_errors |= file_has_errors;
        let rendered_count = remaining.min(all.len());
        omitted += all.len() - rendered_count;
        remaining -= rendered_count;

        if rendered_count > 0 {
            if !rendered_any && separate_from_progress {
                eprint!("\n\n");
            }
            rendered_any = true;
            let display_path =
                display_path.expect("non-empty diagnostics must have a display path");
            let rendered = diagnostics::format_rich_with_path(
                &all[..rendered_count],
                &content,
                &display_path,
                color,
            );
            eprint!("{rendered}");
        }
    }
    if omitted > 0 && diagnostic_limit != Some(0) {
        eprintln!("note: {omitted} additional diagnostics omitted by --diagnostic-limit");
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

    let initial_total = source_ids.iter().copied().collect::<HashSet<_>>().len();
    let progress = compilation_progress(initial_total, "Codegen");
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
            let engine = engine.snapshot();
            let content = engine.content(file_id)?;
            let (parsed, _) = engine.parsed(file_id)?;
            if let Some(module_name) = parsed.module_name(&content) {
                set_progress_module(&progress, &module_name);
            }
            let module = engine.javascript(file_id)?.map_err(|source| {
                let path = files
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
    finish_progress(&progress);

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

    let progress = compilation_progress(modules.len(), "Output");
    modules.par_iter().try_for_each(|module| -> Result<(), CompileError> {
        set_progress_module(&progress, module.name());
        let output_path = output.join(module.filename());
        let output_parent =
            output_path.parent().expect("invariant violated: module filename has no parent");

        fs::create_dir_all(output_parent)?;
        fs::write(output_path, module.source())?;

        if !module.requires_foreign() {
            progress.inc(1);
            return Ok(());
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
        progress.inc(1);
        Ok(())
    })?;
    finish_progress(&progress);
    Ok(())
}

fn compilation_progress(total: usize, phase: &'static str) -> ProgressBar {
    let progress = ProgressBar::new(total as u64);
    configure_progress(&progress, phase);
    progress
}

fn configure_progress(progress: &ProgressBar, phase: &'static str) {
    let started = Instant::now();
    let characters = phase.chars().collect::<Vec<_>>();
    let cycle = characters.len() + 4;
    let total = progress.length().unwrap_or_default();
    let count_width = total.to_string().len().max(4);
    let statistics_width = count_width * 2 + 2;
    let bar_width = CARGO_PROGRESS_REGION_WIDTH
        .saturating_sub(CARGO_PROGRESS_FIXED_OVERHEAD + statistics_width)
        .max(1);
    let phase_text = move |state: &ProgressState, writer: &mut dyn fmt::Write| {
        if state.is_finished() {
            let style = Style::new().fg(Color::TrueColor(255, 255, 255));
            write!(writer, "{}", style.apply_to(phase))
                .expect("writing to a formatter cannot fail");
            return;
        }

        let frame =
            (started.elapsed().as_millis() / SHIMMER_FRAME_INTERVAL.as_millis()) as usize % cycle;
        let highlight = frame as isize - 2;
        for (index, character) in characters.iter().enumerate() {
            let distance = (index as isize - highlight).unsigned_abs();
            let intensity = match distance {
                0 => 255,
                1 => 210,
                2 => 155,
                3 => 115,
                _ => 90,
            };
            let style = Style::new().fg(Color::TrueColor(intensity, intensity, intensity));
            write!(writer, "{}", style.apply_to(character))
                .expect("writing to a formatter cannot fail");
        }
    };
    let template =
        format!("{{phase:>12}} [{{bar:{bar_width}.cyan/blue}}] {{pos:>4}}/{{len:4}} {{msg}}");
    let style = ProgressStyle::with_template(&template)
        .expect("progress bar template is valid")
        .with_key("phase", phase_text)
        .progress_chars("=> ");
    progress.set_style(style);
    progress.enable_steady_tick(SHIMMER_FRAME_INTERVAL);
}

fn phase_progress(progress: &MultiProgress, total: usize, phase: &'static str) -> ProgressBar {
    let phase_progress = progress.add(ProgressBar::new(total as u64));
    configure_progress(&phase_progress, phase);
    phase_progress
}

fn set_progress_module(progress: &ProgressBar, module_name: &str) {
    progress.set_message(module_name.to_string());
}

fn finish_progress(progress: &ProgressBar) {
    progress.set_message("");
    progress.finish();
}

fn report_completion(elapsed: Duration) {
    if !io::stderr().is_terminal() {
        return;
    }

    let label = format!("{:>12}", "Finished");
    let style = Style::new().green().bold();
    let jobs = rayon::current_num_threads();
    let job_label = if jobs == 1 { "job" } else { "jobs" };
    eprintln!("{} in {elapsed:.2?} via {jobs} {job_label}", style.apply_to(label));
}
