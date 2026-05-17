fn checking(path: &std::path::Path) -> datatest_stable::Result<()> {
    tests_integration::fixtures::checking(path)
}

datatest_stable::harness! {
    { test = checking, root = "fixtures/checking", pattern = r".*/Main\.purs$" },
}
