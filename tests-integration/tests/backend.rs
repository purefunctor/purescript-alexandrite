fn backend(path: &std::path::Path) -> datatest_stable::Result<()> {
    tests_integration::fixtures::backend(path)
}

datatest_stable::harness! {
    { test = backend, root = "fixtures/backend", pattern = r".*/Main\.purs$" },
}
