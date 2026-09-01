use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use std::{env, io};

use building::QueryEngine;
use files::{FileId, Files};
use itertools::Itertools;
use process_control::{ChildExt, Control};
use url::Url;

pub type FixtureResult<T = ()> = Result<T, Box<dyn Error>>;

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn fixture_folder(path: &Path) -> Result<&Path, io::Error> {
    path.parent().ok_or_else(|| {
        invalid_data(format!("invariant violated: fixture path has no parent: {}", path.display()))
    })
}

fn module_name(path: &Path) -> Result<String, io::Error> {
    path.file_stem().and_then(|name| name.to_str()).map(ToOwned::to_owned).ok_or_else(|| {
        invalid_data(format!(
            "invariant violated: fixture path has no valid module name: {}",
            path.display()
        ))
    })
}

fn snapshot_path(folder: &Path) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(folder)
}

fn missing_module(path: &Path, module: &str) -> io::Error {
    invalid_data(format!(
        "invariant violated: fixture module {module} not found for path {}",
        path.display()
    ))
}

const UPDATE_JAVASCRIPT_OUTPUT: &str = "ALEXANDRITE_UPDATE_JAVASCRIPT_OUTPUT";
const JAVASCRIPT_VERIFICATION_TIMEOUT: Duration = Duration::from_secs(30);

struct JavaScriptModules {
    modules: Vec<Arc<javascript::Module>>,
}

impl JavaScriptModules {
    fn write_to(&self, files: &Files, output: &Path) -> FixtureResult {
        if self.modules.iter().any(|module| module.requires_runtime()) {
            let runtime = output.join(javascript::runtime_filename());
            std::fs::write(runtime, javascript::runtime_source())?;
        }
        for module in &self.modules {
            let output_path = output.join(module.filename());
            let output_parent = output_path.parent().expect("module filename has no parent");
            std::fs::create_dir_all(output_parent)?;
            std::fs::write(output_path, module.source())?;
            if module.requires_foreign() {
                let source_url = Url::parse(&files.path(module.file_id()))?;
                let source_path = source_url.to_file_path().map_err(|()| {
                    invalid_data(format!(
                        "invariant violated: source URL is not a file: {source_url}"
                    ))
                })?;
                let kind = module
                    .foreign_kind()
                    .expect("invariant violated: required foreign module has no source kind");
                let foreign_path = source_path.with_extension(kind.extension());
                if !foreign_path.exists() {
                    continue;
                }
                let output_path = output.join(module.foreign_filename());
                let output_parent = output_path.parent().expect("foreign filename has no parent");
                std::fs::create_dir_all(output_parent)?;
                std::fs::copy(&foreign_path, &output_path).map_err(|error| {
                    invalid_data(format!(
                        "failed to copy foreign module {} to {}: {error}",
                        foreign_path.display(),
                        output_path.display()
                    ))
                })?;
            }
        }
        Ok(())
    }
}

fn javascript_modules(
    engine: &QueryEngine,
    entry: FileId,
) -> FixtureResult<javascript::ModuleResult<JavaScriptModules>> {
    let mut pending = vec![entry];
    let mut visited = HashSet::new();
    let mut modules = Vec::new();
    while let Some(file_id) = pending.pop() {
        if !visited.insert(file_id) {
            continue;
        }
        let module = match engine.javascript(file_id)? {
            Ok(module) => module,
            Err(error) => return Ok(Err(error)),
        };
        pending.extend(module.dependencies().iter().copied());
        modules.push(module);
    }
    Ok(Ok(JavaScriptModules { modules }))
}

fn collect_output_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<PathBuf, String>,
) -> io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_output_files(root, &path, files)?;
        } else {
            let relative = path.strip_prefix(root).map_err(|_| {
                invalid_data(format!("output path is outside its root: {}", path.display()))
            })?;
            let contents = std::fs::read_to_string(&path)?;
            let contents = contents.replace("\r\n", "\n");
            files.insert(relative.to_owned(), contents);
        }
    }
    Ok(())
}

fn output_files(root: &Path) -> io::Result<BTreeMap<PathBuf, String>> {
    let mut files = BTreeMap::new();
    if root.exists() {
        collect_output_files(root, root, &mut files)?;
    }
    Ok(files)
}

fn copy_output(source: &Path, destination: &Path) -> io::Result<()> {
    if destination.exists() {
        std::fs::remove_dir_all(destination)?;
    }
    for (path, contents) in output_files(source)? {
        let destination = destination.join(path);
        let parent = destination
            .parent()
            .ok_or_else(|| invalid_data("output file has no parent directory"))?;
        std::fs::create_dir_all(parent)?;
        std::fs::write(destination, contents)?;
    }
    Ok(())
}

fn verify_output(expected: &Path, generated: &Path) -> FixtureResult {
    if env::var_os(UPDATE_JAVASCRIPT_OUTPUT).is_some() {
        copy_output(generated, expected)?;
        return Ok(());
    }

    let expected_files = output_files(expected)?;
    let generated_files = output_files(generated)?;
    if expected_files == generated_files {
        return Ok(());
    }

    let expected_paths = expected_files.keys().cloned();
    let expected_paths = expected_paths.collect::<BTreeSet<_>>();
    let generated_paths = generated_files.keys().cloned();
    let generated_paths = generated_paths.collect::<BTreeSet<_>>();
    let created =
        generated_paths.difference(&expected_paths).map(|path| path.display().to_string());
    let removed =
        expected_paths.difference(&generated_paths).map(|path| path.display().to_string());
    let changed = expected_paths
        .intersection(&generated_paths)
        .filter(|path| expected_files[*path] != generated_files[*path]);
    let changed = changed.map(|path| path.display().to_string());
    let changes = created
        .map(|path| format!("created {path}"))
        .chain(removed.map(|path| format!("removed {path}")))
        .chain(changed.map(|path| format!("changed {path}")));
    let changes = changes.collect_vec();
    let changes = changes.join("\n  ");
    let fixture_name = expected
        .parent()
        .and_then(|fixture| fixture.file_name())
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            invalid_data(format!("fixture output has no valid parent: {}", expected.display()))
        })?;
    let message = format!(
        "generated JavaScript differs from {}:\n  {changes}\nrun \
         `just t backend {fixture_name} --update-output` to update fixture output",
        expected.display()
    );
    Err(invalid_data(message).into())
}

fn run_javascript_verification(folder: &Path) -> FixtureResult {
    let script = folder.join("verify.mjs");
    if !script.exists() {
        return Ok(());
    }

    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    let mut child = Command::new("node")
        .arg("verify.mjs")
        .current_dir(folder)
        .stdout(Stdio::from(stdout.try_clone()?))
        .stderr(Stdio::from(stderr.try_clone()?))
        .spawn()?;
    let status = child
        .controlled()
        .time_limit(JAVASCRIPT_VERIFICATION_TIMEOUT)
        .terminate_for_timeout()
        .wait()?;
    let Some(status) = status else {
        let message = format!(
            "Node verification timed out after {} seconds for {}",
            JAVASCRIPT_VERIFICATION_TIMEOUT.as_secs(),
            script.display()
        );
        return Err(invalid_data(message).into());
    };
    if status.success() {
        return Ok(());
    }
    stdout.rewind()?;
    stderr.rewind()?;
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    stdout.read_to_end(&mut stdout_bytes)?;
    stderr.read_to_end(&mut stderr_bytes)?;
    let message = format!(
        "Node verification failed for {}\nstdout:\n{}\nstderr:\n{}",
        script.display(),
        String::from_utf8_lossy(&stdout_bytes),
        String::from_utf8_lossy(&stderr_bytes)
    );
    Err(invalid_data(message).into())
}

pub fn backend(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let fixture = snapshot_path(folder);
    let file = module_name(path)?;
    let display_path = path.file_name().and_then(|name| name.to_str()).ok_or_else(|| {
        invalid_data(format!("invariant violated: invalid fixture file name: {}", path.display()))
    })?;
    let (engine, files) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let checking_report = crate::generated::basic::report_checked(&engine, id, display_path);
    let foreign_report = crate::generated::basic::report_foreign(&engine, id, display_path);
    let backend_report = crate::generated::basic::report_backend(&engine, id, display_path);
    let diagnostics_report = format!("{checking_report}{foreign_report}{backend_report}");
    let generated = tempfile::tempdir()?;
    match javascript_modules(&engine, id)? {
        Ok(modules) => modules.write_to(&files, generated.path())?,
        Err(error) if backend_report.is_empty() => return Err(error.into()),
        Err(_) => {}
    }
    verify_output(&fixture.join("output"), generated.path())?;
    run_javascript_verification(&fixture)?;

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(fixture);
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(file, diagnostics_report));

    Ok(())
}

pub fn checking(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let file = module_name(path)?;
    let display_path = path.file_name().and_then(|name| name.to_str()).ok_or_else(|| {
        invalid_data(format!("invariant violated: invalid fixture file name: {}", path.display()))
    })?;
    let (engine, _) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let report = crate::generated::basic::report_checked(&engine, id, display_path);

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_path(folder));
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(file, report));

    Ok(())
}

pub fn semantic(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let file = module_name(path)?;
    let (engine, _) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let report = crate::generated::semantic::report(&engine, id);

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_path(folder));
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(file, report));

    Ok(())
}

pub fn lowering(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let file = module_name(path)?;
    let (engine, _) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let report = crate::generated::basic::report_lowered(&engine, id, &file);
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_path(folder));
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(file, report));

    Ok(())
}

pub fn resolving(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let file = module_name(path)?;
    let display_path = path.file_name().and_then(|name| name.to_str()).ok_or_else(|| {
        invalid_data(format!("invariant violated: invalid fixture file name: {}", path.display()))
    })?;
    let (engine, _) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let report = crate::generated::basic::report_resolved(&engine, id, &file, display_path);
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_path(folder));
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(file, report));

    Ok(())
}

pub fn docs(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let file = module_name(path)?;
    let (engine, files) = crate::load_compiler(folder);
    let snapshot_path = snapshot_path(folder);

    let report = crate::generated::docs::report(&engine, &files, &snapshot_path)?;
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_path);
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(file, report));

    Ok(())
}

pub fn lsp(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let file = module_name(path)?;
    let (engine, files) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let report = crate::generated::lsp::report(&engine, &files, id);
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_path(folder));
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(file, report));

    Ok(())
}
