fn elaborating(path: &std::path::Path) -> datatest_stable::Result<()> {
    tests_integration::fixtures::elaborating(path)
}

datatest_stable::harness! {
    { test = elaborating, root = "fixtures/elaborating", pattern = r".*/Main\.purs$" },
}
