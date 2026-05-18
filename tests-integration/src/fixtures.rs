use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};

pub type FixtureResult = Result<(), Box<dyn Error>>;

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
