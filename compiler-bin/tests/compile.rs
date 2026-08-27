use std::fs;
use std::process::Command;

use purescript_alexandrite::cli::ColorChoice;
use purescript_alexandrite::compile::{self, CompileConfig};

#[test]
fn compiles_module_graph_with_foreign_dependency() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join("tests/fixtures/compile_module_graph");
    let output = tempfile::tempdir().expect("failed to create compiler output directory");

    compile::start(CompileConfig {
        output: output.path().to_path_buf(),
        inputs: vec![fixture.join("*.purs")],
        json_errors: false,
        diagnostic_limit: None,
        color: ColorChoice::Never,
    });

    let main = output.path().join("Main/index.js");
    let foreign = output.path().join("Main/foreign.js");
    let library = output.path().join("Library/index.js");
    assert!(main.is_file(), "compiler did not emit Main");
    assert!(foreign.is_file(), "compiler did not copy Main's foreign module");
    assert!(library.is_file(), "compiler did not emit Library");

    fs::write(output.path().join("package.json"), r#"{"type":"module"}"#)
        .expect("failed to configure generated JavaScript as ES modules");
    let verification = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg("import('./Main/index.js').then(({ result }) => { if (result !== 43) process.exit(1); })")
        .current_dir(output.path())
        .status()
        .expect("failed to run generated JavaScript");
    assert!(verification.success(), "generated JavaScript returned the wrong result");
}
