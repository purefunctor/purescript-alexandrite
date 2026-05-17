fn lowering(path: &std::path::Path) -> datatest_stable::Result<()> {
    tests_integration::fixtures::lowering(path)
}

datatest_stable::harness! {
    { test = lowering, root = "fixtures/lowering", pattern = r".*\.purs$" },
}
