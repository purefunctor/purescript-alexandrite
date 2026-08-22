use std::collections::BTreeSet;
use std::path::Path;
use std::{fmt, fs};

use serde::{Deserialize, Serialize};

use super::error::VerifierError;
use super::selection::{SelectedPackage, SelectionMode};

pub const REPORT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageSetReport {
    pub version: String,
    pub compiler: String,
    pub published: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectionReport {
    pub mode: SelectionMode,
    pub requested_packages: Vec<String>,
    pub resolved_packages: Vec<SelectedPackage>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub packages: usize,
    pub source_files: usize,
    pub verifier_errors: usize,
    pub compiler_errors: usize,
    pub compiler_warnings: usize,
    pub packages_with_errors: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpanReport {
    pub start: u32,
    pub end: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_position: Option<SourcePosition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_position: Option<SourcePosition>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum CompilationStage {
    Parse,
    Stabilizing,
    Indexing,
    Resolving,
    Lowering,
    Grouping,
    Bracketing,
    Sectioning,
    Checking,
    JavaScript,
}

impl CompilationStage {
    pub fn as_str(self) -> &'static str {
        match self {
            CompilationStage::Parse => "parse",
            CompilationStage::Stabilizing => "stabilizing",
            CompilationStage::Indexing => "indexing",
            CompilationStage::Resolving => "resolving",
            CompilationStage::Lowering => "lowering",
            CompilationStage::Grouping => "grouping",
            CompilationStage::Bracketing => "bracketing",
            CompilationStage::Sectioning => "sectioning",
            CompilationStage::Checking => "checking",
            CompilationStage::JavaScript => "javascript",
        }
    }
}

impl fmt::Display for CompilationStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CompilerDiagnostic {
    pub package: String,
    pub version: String,
    pub file: String,
    pub stage: CompilationStage,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub span: SpanReport,
    #[serde(skip)]
    pub human: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct IssueLocation {
    pub package: String,
    pub version: String,
    pub file: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub enum VerifierIssueKind {
    MissingPackageSetPackage,
    MissingManifest,
    MissingPackageSetDependency,
    DuplicateModule,
    QueryError,
}

impl VerifierIssueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VerifierIssueKind::MissingPackageSetPackage => "MissingPackageSetPackage",
            VerifierIssueKind::MissingManifest => "MissingManifest",
            VerifierIssueKind::MissingPackageSetDependency => "MissingPackageSetDependency",
            VerifierIssueKind::DuplicateModule => "DuplicateModule",
            VerifierIssueKind::QueryError => "QueryError",
        }
    }
}

impl fmt::Display for VerifierIssueKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct VerifierIssue {
    pub kind: VerifierIssueKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<IssueLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<CompilationStage>,
    pub message: String,
}

impl VerifierIssue {
    pub fn missing_package(package: &str) -> VerifierIssue {
        VerifierIssue {
            kind: VerifierIssueKind::MissingPackageSetPackage,
            package: Some(package.to_string()),
            version: None,
            dependency: None,
            locations: Vec::new(),
            stage: None,
            message: format!("package '{package}' is not present in the selected package set"),
        }
    }

    pub fn missing_manifest(package: &str, version: &str) -> VerifierIssue {
        VerifierIssue {
            kind: VerifierIssueKind::MissingManifest,
            package: Some(package.to_string()),
            version: Some(version.to_string()),
            dependency: None,
            locations: Vec::new(),
            stage: None,
            message: format!(
                "package '{package}' has package-set version '{version}' but no matching registry-index manifest"
            ),
        }
    }

    pub fn missing_dependency(package: &str, version: &str, dependency: &str) -> VerifierIssue {
        VerifierIssue {
            kind: VerifierIssueKind::MissingPackageSetDependency,
            package: Some(package.to_string()),
            version: Some(version.to_string()),
            dependency: Some(dependency.to_string()),
            locations: Vec::new(),
            stage: None,
            message: format!(
                "package '{package}' depends on '{dependency}', which is absent from the selected package set"
            ),
        }
    }

    pub fn duplicate_module(
        module: &str,
        first: IssueLocation,
        second: IssueLocation,
    ) -> VerifierIssue {
        let mut locations = vec![first, second];
        locations.sort();
        let [first, second] = locations.as_slice() else {
            unreachable!("duplicate modules have two locations");
        };
        let first_display = format!("{}@{}/{}", first.package, first.version, first.file);
        let second_display = format!("{}@{}/{}", second.package, second.version, second.file);

        VerifierIssue {
            kind: VerifierIssueKind::DuplicateModule,
            package: None,
            version: None,
            dependency: None,
            locations,
            stage: Some(CompilationStage::Parse),
            message: format!(
                "module '{module}' is provided by both '{first_display}' and '{second_display}'"
            ),
        }
    }

    pub fn query_error(
        package: &str,
        version: &str,
        file: &str,
        stage: CompilationStage,
        message: String,
    ) -> VerifierIssue {
        VerifierIssue {
            kind: VerifierIssueKind::QueryError,
            package: Some(package.to_string()),
            version: Some(version.to_string()),
            dependency: None,
            locations: vec![IssueLocation {
                package: package.to_string(),
                version: version.to_string(),
                file: file.to_string(),
            }],
            stage: Some(stage),
            message: format!("{stage} failed for {file}: {message}"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    #[serde(default)]
    pub schema_version: u32,
    pub package_set: PackageSetReport,
    pub selection: SelectionReport,
    pub summary: Summary,
    pub diagnostics: Vec<CompilerDiagnostic>,
    pub verifier_errors: Vec<VerifierIssue>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportSchema {
    #[serde(default)]
    schema_version: u32,
}

impl Report {
    pub fn new(package_set: PackageSetReport, selection: SelectionReport) -> Report {
        let packages = selection.resolved_packages.len();
        Report {
            schema_version: REPORT_SCHEMA_VERSION,
            package_set,
            selection,
            summary: Summary { packages, ..Summary::default() },
            diagnostics: Vec::new(),
            verifier_errors: Vec::new(),
        }
    }

    pub fn recompute_summary(&mut self) {
        self.summary.packages = self.selection.resolved_packages.len();
        self.summary.verifier_errors = self.verifier_errors.len();
        let compiler_errors = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
        self.summary.compiler_errors = compiler_errors.count();
        let compiler_warnings = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning);
        self.summary.compiler_warnings = compiler_warnings.count();
        let packages_with_errors = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .map(|diagnostic| diagnostic.package.as_str());
        let packages_with_errors = packages_with_errors.collect::<BTreeSet<_>>();
        self.summary.packages_with_errors = packages_with_errors.len();
    }

    pub fn has_errors(&self) -> bool {
        self.summary.verifier_errors > 0 || self.summary.compiler_errors > 0
    }

    pub fn print_human(&self) {
        println!("Package set: {}", self.package_set.version);
        println!("Compiler: {}", self.package_set.compiler);
        println!("Selected packages: {}", self.summary.packages);
        println!("Source files: {}", self.summary.source_files);
        println!();

        for issue in &self.verifier_errors {
            println!("verifier[{}]: {}", issue.kind, issue.message);
        }
        if !self.verifier_errors.is_empty() {
            println!();
        }

        for diagnostic in &self.diagnostics {
            print!("{}", diagnostic.human);
            if !diagnostic.human.ends_with('\n') {
                println!();
            }
        }

        println!("Summary:");
        println!("  verifier errors: {}", self.summary.verifier_errors);
        println!("  parse errors: {}", self.stage_error_count(CompilationStage::Parse));
        println!("  indexing errors: {}", self.stage_error_count(CompilationStage::Indexing));
        println!("  resolving errors: {}", self.stage_error_count(CompilationStage::Resolving));
        println!("  lowering errors: {}", self.stage_error_count(CompilationStage::Lowering));
        println!("  checking errors: {}", self.stage_error_count(CompilationStage::Checking));
        println!("  JavaScript errors: {}", self.stage_error_count(CompilationStage::JavaScript));
        println!("  compiler errors: {}", self.summary.compiler_errors);
        println!("  compiler warnings: {}", self.summary.compiler_warnings);
        println!("  packages with errors: {}", self.summary.packages_with_errors);
    }

    pub fn write_json(&self, path: &Path) -> Result<(), VerifierError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut report = self.clone();
        report.sort_details();
        let content = serde_json::to_string_pretty(&report)?;
        fs::write(path, content).map_err(VerifierError::from)
    }

    pub fn read_json(path: &Path) -> Result<Report, VerifierError> {
        let content = fs::read_to_string(path)?;
        let schema: ReportSchema = serde_json::from_str(&content)?;
        if schema.schema_version > REPORT_SCHEMA_VERSION {
            return Err(VerifierError::UnsupportedReportSchemaVersion {
                found: schema.schema_version,
                latest_supported: REPORT_SCHEMA_VERSION,
            });
        }
        let report: Report = serde_json::from_str(&content)?;
        Ok(report)
    }

    fn sort_details(&mut self) {
        self.diagnostics.sort_by(|left, right| {
            (
                &left.package,
                &left.version,
                &left.file,
                &left.stage,
                &left.severity,
                &left.code,
                left.span.start,
                left.span.end,
                &left.message,
            )
                .cmp(&(
                    &right.package,
                    &right.version,
                    &right.file,
                    &right.stage,
                    &right.severity,
                    &right.code,
                    right.span.start,
                    right.span.end,
                    &right.message,
                ))
        });
        for issue in &mut self.verifier_errors {
            issue.locations.sort();
        }
        self.verifier_errors.sort_by(|left, right| {
            (
                &left.kind,
                &left.package,
                &left.version,
                &left.dependency,
                &left.locations,
                &left.stage,
                &left.message,
            )
                .cmp(&(
                    &right.kind,
                    &right.package,
                    &right.version,
                    &right.dependency,
                    &right.locations,
                    &right.stage,
                    &right.message,
                ))
        });
    }

    fn stage_error_count(&self, stage: CompilationStage) -> usize {
        let errors = self.diagnostics.iter().filter(|diagnostic| {
            diagnostic.severity == DiagnosticSeverity::Error && diagnostic.stage == stage
        });
        errors.count()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::super::selection::{SelectedPackage, SelectionMode};

    use super::{
        CompilationStage, CompilerDiagnostic, DiagnosticSeverity, PackageSetReport,
        REPORT_SCHEMA_VERSION, Report, SelectionReport, SourcePosition, SpanReport, VerifierIssue,
    };

    #[test]
    fn json_round_trip_preserves_report_details() {
        let mut report = Report::new(
            PackageSetReport {
                version: "64.7.1".to_string(),
                compiler: "0.15.15".to_string(),
                published: "2025-05-06".to_string(),
            },
            SelectionReport {
                mode: SelectionMode::Packages,
                requested_packages: vec!["effect".to_string()],
                resolved_packages: vec![SelectedPackage {
                    name: "effect".to_string(),
                    version: "4.0.0".to_string(),
                }],
            },
        );
        report.summary.source_files = 1;
        report.diagnostics.push(CompilerDiagnostic {
            package: "effect".to_string(),
            version: "4.0.0".to_string(),
            file: "src/Effect.purs".to_string(),
            stage: CompilationStage::Checking,
            severity: DiagnosticSeverity::Error,
            code: "UnknownType".to_string(),
            message: "unknown type".to_string(),
            span: SpanReport {
                start: 0,
                end: 10,
                start_position: Some(SourcePosition { line: 1, column: 1 }),
                end_position: Some(SourcePosition { line: 1, column: 11 }),
            },
            human: "hidden from JSON".to_string(),
        });
        report
            .verifier_errors
            .push(VerifierIssue::missing_dependency("effect", "4.0.0", "prelude"));
        report.recompute_summary();

        let dir = tempdir().unwrap();
        let path = dir.path().join("report.json");
        report.write_json(&path).unwrap();
        let json = std::fs::read_to_string(&path).unwrap();

        let deserialized = Report::read_json(&path).unwrap();
        let mut expected = report.clone();
        expected.sort_details();
        for diagnostic in &mut expected.diagnostics {
            diagnostic.human.clear();
        }

        assert_eq!(deserialized.schema_version, REPORT_SCHEMA_VERSION);
        assert_eq!(deserialized.package_set, expected.package_set);
        assert_eq!(deserialized.selection, expected.selection);
        assert_eq!(deserialized.summary, expected.summary);
        assert_eq!(deserialized.diagnostics, expected.diagnostics);
        assert_eq!(deserialized.verifier_errors, expected.verifier_errors);
        assert!(!json.contains("hidden from JSON"));
    }

    #[test]
    fn reads_legacy_reports() {
        let dir = tempdir().unwrap();
        let legacy_path = dir.path().join("legacy.json");
        let report = Report::new(
            PackageSetReport {
                version: "64.7.1".to_string(),
                compiler: "0.15.15".to_string(),
                published: "2025-05-06".to_string(),
            },
            SelectionReport {
                mode: SelectionMode::Core,
                requested_packages: Vec::new(),
                resolved_packages: Vec::new(),
            },
        );
        let mut legacy = serde_json::to_value(&report).unwrap();
        legacy.as_object_mut().unwrap().remove("schemaVersion");
        std::fs::write(&legacy_path, serde_json::to_vec(&legacy).unwrap()).unwrap();

        let legacy = Report::read_json(&legacy_path).unwrap();
        assert_eq!(legacy.schema_version, 0);
    }

    #[test]
    fn rejects_future_schema_before_deserializing_report_fields() {
        let dir = tempdir().unwrap();
        let future_path = dir.path().join("future.json");
        let future = serde_json::json!({ "schemaVersion": REPORT_SCHEMA_VERSION + 1 });
        std::fs::write(&future_path, serde_json::to_vec(&future).unwrap()).unwrap();

        let error = Report::read_json(&future_path).unwrap_err();
        assert!(error.to_string().contains("unsupported report schema version"));
    }
}
