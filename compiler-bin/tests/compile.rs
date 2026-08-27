use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn compiler() -> Command {
    Command::new(env!("CARGO_BIN_EXE_purescript-alexandrite"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn compile(fixture: &Path, output: &Path, extra_arguments: &[&str]) -> Output {
    let sources = fixture.join("src/**/*.purs");
    compiler()
        .arg("compile")
        .arg("--output")
        .arg(output)
        .args(extra_arguments)
        .arg(sources)
        .output()
        .expect("compiler should run")
}

#[test]
fn compiles_a_project_with_dependencies_and_foreign_javascript() {
    let fixture = fixture("compile-project");
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("output");

    let result = compile(&fixture, &output, &["--json-errors", "--color", "never"]);

    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["warnings"], serde_json::json!([]));
    assert_eq!(report["errors"], serde_json::json!([]));
    assert!(output.join("Library/index.js").is_file());
    assert!(output.join("Main/foreign.js").is_file());

    std::fs::write(output.join("package.json"), r#"{"type":"module"}"#).unwrap();
    let module = url::Url::from_file_path(output.join("Main/index.js")).unwrap();
    let javascript =
        format!("const module = await import({:?}); console.log(module.result);", module.as_str());
    let execution = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(javascript)
        .output()
        .expect("generated JavaScript should run");

    assert!(execution.status.success(), "{}", String::from_utf8_lossy(&execution.stderr));
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "42");
}

#[test]
fn reports_compilation_errors_through_the_json_cli_contract() {
    let fixture = fixture("compile-error");
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("output");

    let result = compile(
        &fixture,
        &output,
        &["--json-errors", "--diagnostic-limit", "0", "--color", "never"],
    );

    assert!(!result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["warnings"], serde_json::json!([]));
    assert_eq!(report["errors"][0]["message"], "compilation failed");
    assert!(!output.exists());
}
