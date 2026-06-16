fn resolving(path: &std::path::Path) -> datatest_stable::Result<()> {
    tests_integration::fixtures::resolving(path)
}

datatest_stable::harness! {
    { test = resolving, root = "fixtures/resolving", pattern = r".*\.purs$" },
}
