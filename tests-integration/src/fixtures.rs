use std::collections::HashSet;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use building::QueryEngine;
use files::{FileId, Files};
use itertools::Itertools;
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

pub struct JavaScriptModules {
    modules: Vec<Arc<javascript::Module>>,
}

impl JavaScriptModules {
    pub fn entry(&self) -> &javascript::Module {
        &self.modules[0]
    }

    pub fn get(&self, file_id: FileId) -> Option<&javascript::Module> {
        self.modules.iter().find(|module| module.file_id() == file_id).map(Arc::as_ref)
    }

    pub fn render(&self) -> String {
        let modules = self
            .modules
            .iter()
            .map(|module| format!("// {}\n{}", module.filename(), module.source()));
        let modules = modules.collect_vec();
        modules.join("\n")
    }

    pub fn write_to(&self, files: &Files, output: &Path) -> FixtureResult {
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
                let foreign_path = source_path.with_extension("js");
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

pub fn javascript_modules(
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

pub fn backend(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let file = module_name(path)?;
    let (engine, _) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let checking_report = crate::generated::basic::report_checked(&engine, id);
    let ssa_report = match engine.ssa(id)? {
        Ok(module) => ssa::pretty::render(&module),
        Err(error) => error.to_string(),
    };
    let javascript_report = match javascript_modules(&engine, id)? {
        Ok(modules) => modules.render(),
        Err(error) => error.to_string(),
    };
    let ssa_snapshot = format!("{file}.ssa");
    let javascript_snapshot = format!("{file}.javascript");

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_path(folder));
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| {
        insta::assert_snapshot!(file, checking_report);
        insta::assert_snapshot!(ssa_snapshot, ssa_report);
        insta::assert_snapshot!(javascript_snapshot, javascript_report);
    });

    Ok(())
}

pub fn checking(path: &Path) -> FixtureResult {
    let folder = fixture_folder(path)?;
    let file = module_name(path)?;
    let (engine, _) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let report = crate::generated::basic::report_checked(&engine, id);

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
    let (engine, _) = crate::load_compiler(folder);
    let Some(id) = engine.module_file(&file) else {
        return Err(missing_module(path, &file).into());
    };

    let report = crate::generated::basic::report_resolved(&engine, id, &file);
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
