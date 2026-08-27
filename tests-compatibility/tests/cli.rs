use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::Value;
use tar::Builder;
use tempfile::TempDir;

const PACKAGE: &str = "prelude";
const VERSION: &str = "1.0.0";

struct VerifierFixture {
    temporary: TempDir,
    registry: PathBuf,
    index: PathBuf,
    cache: PathBuf,
}

impl VerifierFixture {
    fn new() -> VerifierFixture {
        let temporary = tempfile::tempdir().unwrap();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/cli");
        let registry = fixtures.join("registry");
        let index = fixtures.join("registry-index");
        let cache = temporary.path().join("cache");
        fs::create_dir_all(cache.join("downloads")).unwrap();

        VerifierFixture { temporary, registry, index, cache }
    }

    fn install_tarball(&self, source_directory: &Path) {
        let tarball = self.cache.join("downloads").join(format!("{PACKAGE}-{VERSION}.tar.gz"));
        let file = fs::File::create(tarball).unwrap();
        let encoder = GzEncoder::new(file, Compression::default());
        let mut archive = Builder::new(encoder);
        archive.append_dir_all("package/src", source_directory).unwrap();
        archive.finish().unwrap();
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tests-compatibility"));
        command.current_dir(self.temporary.path());
        command
    }

    fn registry_arguments(&self) -> [String; 6] {
        [
            "--registry-dir".to_string(),
            self.registry.display().to_string(),
            "--index-dir".to_string(),
            self.index.display().to_string(),
            "--cache-dir".to_string(),
            self.cache.display().to_string(),
        ]
    }

    fn report(&self, name: &str) -> PathBuf {
        self.temporary.path().join(name)
    }
}

fn run(mut command: Command) -> Output {
    command.output().unwrap()
}

fn output_text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn prepare_verify_and_compare_reports_a_compiler_regression() {
    let fixture = VerifierFixture::new();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/cli");
    fixture.install_tarball(&fixtures.join("valid"));

    let mut prepare = fixture.command();
    prepare.arg("prepare").args(fixture.registry_arguments()).args([
        "--package-set",
        "1.0.0",
        "--preset",
        "core",
    ]);
    let prepared = run(prepare);
    assert!(prepared.status.success(), "{}", output_text(&prepared));
    assert!(String::from_utf8_lossy(&prepared.stdout).contains("Prepared 1 packages"));
    let extracted_source = fixture.cache.join("sources/prelude/1.0.0/src/Prelude.purs");
    assert_eq!(
        fs::read_to_string(&extracted_source).unwrap(),
        fs::read_to_string(fixtures.join("valid/Prelude.purs")).unwrap()
    );
    assert!(fixture.cache.join("sources/prelude/1.0.0/.tarball-size").is_file());

    let base_report = fixture.report("base.json");
    let mut verify_base = fixture.command();
    verify_base
        .arg("verify")
        .args(fixture.registry_arguments())
        .args(["--package-set", "1.0.0", "--package", PACKAGE, "--json-output"])
        .arg(&base_report);
    let base = run(verify_base);
    assert!(base.status.success(), "{}", output_text(&base));
    let base_json = read_json(&base_report);
    assert_eq!(base_json["summary"]["sourceFiles"], 1);
    assert_eq!(base_json["summary"]["compilerErrors"], 0);

    fs::copy(fixtures.join("regression/Prelude.purs"), extracted_source).unwrap();

    let candidate_report = fixture.report("candidate.json");
    let mut verify_candidate = fixture.command();
    verify_candidate
        .arg("verify")
        .args(fixture.registry_arguments())
        .args(["--package-set", "1.0.0", "--package", PACKAGE, "--json-output"])
        .arg(&candidate_report);
    let candidate = run(verify_candidate);
    assert_eq!(candidate.status.code(), Some(1), "{}", output_text(&candidate));
    let candidate_json = read_json(&candidate_report);
    assert!(candidate_json["summary"]["compilerErrors"].as_u64().unwrap() > 0);
    let diagnostics = candidate_json["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["package"] == PACKAGE && diagnostic["code"] == "NotInScope"
    }));

    let comparison_report = fixture.report("comparison.json");
    let summary = fixture.report("summary.md");
    let mut compare = fixture.command();
    compare
        .arg("compare")
        .args(["--base-report", base_report.to_str().unwrap()])
        .args(["--candidate-report", candidate_report.to_str().unwrap()])
        .args(["--json-output", comparison_report.to_str().unwrap()])
        .args(["--summary-output", summary.to_str().unwrap()])
        .arg("--github-annotations");
    let comparison = run(compare);
    assert_eq!(comparison.status.code(), Some(1), "{}", output_text(&comparison));

    let comparison_json = read_json(&comparison_report);
    assert!(comparison_json["summary"]["introducedCompilerErrors"].as_u64().unwrap() > 0);
    let markdown = fs::read_to_string(summary).unwrap();
    assert!(markdown.contains("The candidate introduces compatibility errors"));
    assert!(markdown.contains("NotInScope"));
    assert!(String::from_utf8_lossy(&comparison.stdout).contains("::error title="));
}
