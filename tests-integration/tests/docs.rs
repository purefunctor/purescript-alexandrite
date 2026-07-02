fn docs(path: &std::path::Path) -> datatest_stable::Result<()> {
    tests_integration::fixtures::docs(path)
}

datatest_stable::harness! {
    { test = docs, root = "fixtures/docs", pattern = r".*/Main\.purs$" },
}
