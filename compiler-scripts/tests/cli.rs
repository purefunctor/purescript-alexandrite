use std::path::Path;
use std::process::{Command, Output};

fn scripts() -> Command {
    Command::new(env!("CARGO_BIN_EXE_compiler-scripts"))
}

fn run(current_directory: &Path, arguments: &[&str]) -> Output {
    scripts()
        .current_dir(current_directory)
        .args(arguments)
        .output()
        .expect("compiler scripts should run")
}

fn build_failing_snapshot_cargo(path: &Path) {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/failing_snapshot_cargo.rs");
    let executable = path.with_extension(std::env::consts::EXE_EXTENSION);
    let status = Command::new("rustc")
        .arg(source)
        .arg("-o")
        .arg(executable)
        .status()
        .expect("rustc should run");
    assert!(status.success(), "fake cargo should compile");
}

#[test]
fn creates_previews_and_deletes_a_fixture_through_the_cli() {
    let temporary = tempfile::tempdir().unwrap();
    let fixtures = temporary.path().join("tests-integration/fixtures/backend");
    std::fs::create_dir_all(&fixtures).unwrap();

    let created = run(temporary.path(), &["backend", "--create", "CLI lifecycle"]);
    assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));

    let fixture_entries = std::fs::read_dir(&fixtures).unwrap();
    let fixture_entries = fixture_entries.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(fixture_entries.len(), 1);
    let fixture = fixture_entries[0].path();
    assert!(fixture.file_name().unwrap().to_string_lossy().ends_with("_cli_lifecycle"));
    let main = fixture.join("Main.purs");
    assert_eq!(std::fs::read_to_string(&main).unwrap(), "module Main where\n\n");

    let preview = run(temporary.path(), &["backend", "--delete", "CLI lifecycle"]);
    assert!(preview.status.success(), "{}", String::from_utf8_lossy(&preview.stderr));
    assert!(String::from_utf8_lossy(&preview.stdout).contains("1 pending deletion(s) in backend"));
    assert_eq!(std::fs::read_to_string(&main).unwrap(), "module Main where\n\n");

    let deleted = run(temporary.path(), &["backend", "--delete", "CLI lifecycle", "--confirm"]);
    assert!(deleted.status.success(), "{}", String::from_utf8_lossy(&deleted.stderr));
    assert_eq!(std::fs::read_dir(&fixtures).unwrap().count(), 0);
}

#[test]
fn runs_a_fixture_category_end_to_end() {
    let temporary = tempfile::tempdir().unwrap();
    let package = temporary.path().join("tests-integration");
    std::fs::create_dir_all(package.join("tests")).unwrap();
    std::fs::write(
        temporary.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"tests-integration\"]\nresolver = \"3\"\n",
    )
    .unwrap();
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"tests-integration\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(package.join("tests/docs.rs"), "#[test]\nfn documentation_baseline() {}\n")
        .unwrap();

    let result = run(temporary.path(), &["docs", "documentation_baseline"]);

    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("All tests passed, no pending snapshots.")
    );
}

#[test]
fn reports_failed_pending_snapshot_inspection() {
    let temporary = tempfile::tempdir().unwrap();
    let binaries = temporary.path().join("bin");
    std::fs::create_dir(&binaries).unwrap();
    build_failing_snapshot_cargo(&binaries.join("cargo"));

    let result = scripts()
        .current_dir(temporary.path())
        .env("PATH", binaries)
        .args(["docs", "snapshot_failure"])
        .output()
        .expect("compiler scripts should run");

    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("cargo insta pending-snapshots failed: snapshot inspection failed")
    );
}
