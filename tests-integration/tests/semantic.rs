fn semantic(path: &std::path::Path) -> datatest_stable::Result<()> {
    tests_integration::fixtures::semantic(path)
}

datatest_stable::harness! {
    { test = semantic, root = "fixtures/semantic", pattern = r".*/Main\.purs$" },
}
