fn functional(path: &std::path::Path) -> datatest_stable::Result<()> {
    let folder = path.parent().ok_or("fixture path has no parent")?;
    let (engine, _) = tests_integration::load_compiler(folder);
    let id = engine.module_file("Main").ok_or("fixture has no Main module")?;
    let report = match engine.functional(id)? {
        Ok(module) => functional::pretty::render(&module),
        Err(error) => error.to_string(),
    };

    let snapshot_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(folder);
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_path);
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!("Main.functional", report));
    Ok(())
}

datatest_stable::harness! {
    { test = functional, root = "fixtures/backend", pattern = r".*/Main\.purs$" },
}
