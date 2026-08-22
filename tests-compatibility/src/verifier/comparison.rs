use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Serialize;

use super::error::VerifierError;
use super::report::{
    CompilationStage, CompilerDiagnostic, DiagnosticSeverity, IssueLocation, PackageSetReport,
    Report, Summary, VerifierIssue, VerifierIssueKind,
};

#[derive(Clone, Copy)]
enum VerifierIssueIdentity {
    LegacySchema,
    StructuredSchema,
}

#[derive(Clone, Copy)]
enum ReportPathFormat {
    LegacySchema,
    PackageRelative,
}

impl ReportPathFormat {
    fn from_schema_version(schema_version: u32) -> ReportPathFormat {
        if schema_version == 0 {
            ReportPathFormat::LegacySchema
        } else {
            ReportPathFormat::PackageRelative
        }
    }
}

const COMPARISON_REPORT_SCHEMA_VERSION: u32 = 1;
const JAVASCRIPT_STAGE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticKey {
    pub(super) package: String,
    pub(super) version: String,
    pub(super) file: String,
    pub(super) stage: CompilationStage,
    pub(super) severity: DiagnosticSeverity,
    pub(super) code: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct VerifierKey {
    pub(super) kind: VerifierIssueKind,
    pub(super) package: Option<String>,
    pub(super) version: Option<String>,
    pub(super) dependency: Option<String>,
    pub(super) locations: Vec<IssueLocation>,
    pub(super) stage: Option<CompilationStage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticChange {
    pub(super) key: DiagnosticKey,
    pub(super) base_diagnostics: Vec<CompilerDiagnostic>,
    pub(super) candidate_diagnostics: Vec<CompilerDiagnostic>,
}

impl DiagnosticChange {
    pub(super) fn introduced_count(&self) -> usize {
        self.candidate_diagnostics.len().saturating_sub(self.base_diagnostics.len())
    }

    pub(super) fn fixed_count(&self) -> usize {
        self.base_diagnostics.len().saturating_sub(self.candidate_diagnostics.len())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifierChange {
    pub(super) key: VerifierKey,
    pub(super) base_issues: Vec<VerifierIssue>,
    pub(super) candidate_issues: Vec<VerifierIssue>,
}

impl VerifierChange {
    pub(super) fn introduced_count(&self) -> usize {
        self.candidate_issues.len().saturating_sub(self.base_issues.len())
    }

    pub(super) fn fixed_count(&self) -> usize {
        self.base_issues.len().saturating_sub(self.candidate_issues.len())
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonSummary {
    pub(super) introduced_compiler_errors: usize,
    pub(super) fixed_compiler_errors: usize,
    pub(super) introduced_compiler_warnings: usize,
    pub(super) fixed_compiler_warnings: usize,
    pub(super) introduced_verifier_errors: usize,
    pub(super) fixed_verifier_errors: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonReport {
    schema_version: u32,
    pub(super) package_set: PackageSetReport,
    pub(super) base_summary: Summary,
    pub(super) candidate_summary: Summary,
    pub(super) summary: ComparisonSummary,
    pub(super) diagnostic_changes: Vec<DiagnosticChange>,
    pub(super) verifier_changes: Vec<VerifierChange>,
}

impl ComparisonReport {
    pub fn has_regressions(&self) -> bool {
        self.summary.introduced_compiler_errors > 0 || self.summary.introduced_verifier_errors > 0
    }

    pub fn write_json(&self, path: &Path) -> Result<(), VerifierError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content).map_err(VerifierError::from)
    }
}

pub fn compare_reports(
    base: &Report,
    candidate: &Report,
) -> Result<ComparisonReport, VerifierError> {
    validate_comparable(base, candidate)?;

    let mut base = base.clone();
    let mut candidate = candidate.clone();
    normalize_report_paths(&mut base);
    normalize_report_paths(&mut candidate);
    base.recompute_summary();
    candidate.recompute_summary();

    let mut base_diagnostics = base.diagnostics.clone();
    let mut candidate_diagnostics = candidate.diagnostics.clone();
    let mut base_verifier_errors = base.verifier_errors.clone();
    let mut candidate_verifier_errors = candidate.verifier_errors.clone();
    if base.schema_version < JAVASCRIPT_STAGE_SCHEMA_VERSION
        || candidate.schema_version < JAVASCRIPT_STAGE_SCHEMA_VERSION
    {
        base_diagnostics.retain(|diagnostic| diagnostic.stage != CompilationStage::JavaScript);
        candidate_diagnostics.retain(|diagnostic| diagnostic.stage != CompilationStage::JavaScript);
        base_verifier_errors.retain(|issue| issue.stage != Some(CompilationStage::JavaScript));
        candidate_verifier_errors.retain(|issue| issue.stage != Some(CompilationStage::JavaScript));
    }

    let diagnostic_changes = compare_diagnostics(&base_diagnostics, &candidate_diagnostics);
    let verifier_issue_identity = if base.schema_version == 0 || candidate.schema_version == 0 {
        VerifierIssueIdentity::LegacySchema
    } else {
        VerifierIssueIdentity::StructuredSchema
    };
    let verifier_changes = compare_verifier_issues(
        &base_verifier_errors,
        &candidate_verifier_errors,
        verifier_issue_identity,
    );
    let summary = comparison_summary(&diagnostic_changes, &verifier_changes);

    Ok(ComparisonReport {
        schema_version: COMPARISON_REPORT_SCHEMA_VERSION,
        package_set: candidate.package_set,
        base_summary: base.summary,
        candidate_summary: candidate.summary,
        summary,
        diagnostic_changes,
        verifier_changes,
    })
}

fn validate_comparable(base: &Report, candidate: &Report) -> Result<(), VerifierError> {
    if base.package_set != candidate.package_set {
        return Err(VerifierError::IncompatibleReports(format!(
            "package sets differ (base {}, candidate {})",
            base.package_set.version, candidate.package_set.version
        )));
    }
    if base.selection.resolved_packages != candidate.selection.resolved_packages {
        return Err(VerifierError::IncompatibleReports(
            "resolved package selections differ".to_string(),
        ));
    }
    if base.summary.source_files != candidate.summary.source_files {
        return Err(VerifierError::IncompatibleReports(format!(
            "source file counts differ (base {}, candidate {})",
            base.summary.source_files, candidate.summary.source_files
        )));
    }
    Ok(())
}

fn normalize_report_paths(report: &mut Report) {
    let path_format = ReportPathFormat::from_schema_version(report.schema_version);
    for diagnostic in &mut report.diagnostics {
        normalize_diagnostic_path(diagnostic, path_format);
    }
    for issue in &mut report.verifier_errors {
        for location in &mut issue.locations {
            location.file =
                normalize_file(&location.file, &location.package, &location.version, path_format);
        }
        issue.locations.sort();
    }
}

fn normalize_diagnostic_path(diagnostic: &mut CompilerDiagnostic, path_format: ReportPathFormat) {
    diagnostic.file =
        normalize_file(&diagnostic.file, &diagnostic.package, &diagnostic.version, path_format);
}

fn normalize_file(
    file: &str,
    package: &str,
    version: &str,
    path_format: ReportPathFormat,
) -> String {
    let file = file.replace('\\', "/");
    match path_format {
        ReportPathFormat::PackageRelative => file,
        ReportPathFormat::LegacySchema => {
            let cache_marker = format!("/sources/{package}/{version}/");
            file.rsplit_once(&cache_marker)
                .map_or(file.clone(), |(_, relative)| relative.to_string())
        }
    }
}

pub(super) fn diagnostic_groups(
    report: &Report,
) -> BTreeMap<DiagnosticKey, Vec<CompilerDiagnostic>> {
    let mut diagnostics = report.diagnostics.clone();
    let path_format = ReportPathFormat::from_schema_version(report.schema_version);
    for diagnostic in &mut diagnostics {
        normalize_diagnostic_path(diagnostic, path_format);
    }
    group_diagnostics(&diagnostics)
}

fn compare_diagnostics(
    base: &[CompilerDiagnostic],
    candidate: &[CompilerDiagnostic],
) -> Vec<DiagnosticChange> {
    let base = group_diagnostics(base);
    let candidate = group_diagnostics(candidate);
    let keys = base.keys().chain(candidate.keys()).cloned();
    let keys = keys.collect::<BTreeSet<_>>();
    let changes = keys.into_iter().filter_map(|key| {
        let base_diagnostics = base.get(&key).cloned().unwrap_or_default();
        let candidate_diagnostics = candidate.get(&key).cloned().unwrap_or_default();
        let base_count = base_diagnostics.len();
        let candidate_count = candidate_diagnostics.len();
        (base_count != candidate_count).then_some(DiagnosticChange {
            key,
            base_diagnostics,
            candidate_diagnostics,
        })
    });
    changes.collect()
}

fn group_diagnostics(
    diagnostics: &[CompilerDiagnostic],
) -> BTreeMap<DiagnosticKey, Vec<CompilerDiagnostic>> {
    let mut groups: BTreeMap<DiagnosticKey, Vec<CompilerDiagnostic>> = BTreeMap::new();
    for diagnostic in diagnostics {
        let key = DiagnosticKey {
            package: diagnostic.package.clone(),
            version: diagnostic.version.clone(),
            file: diagnostic.file.clone(),
            stage: diagnostic.stage,
            severity: diagnostic.severity,
            code: diagnostic.code.clone(),
        };
        groups.entry(key).or_default().push(diagnostic.clone());
    }
    for diagnostics in groups.values_mut() {
        diagnostics.sort_by(|left, right| {
            (left.span.start, left.span.end, &left.message).cmp(&(
                right.span.start,
                right.span.end,
                &right.message,
            ))
        });
    }
    groups
}

fn compare_verifier_issues(
    base: &[VerifierIssue],
    candidate: &[VerifierIssue],
    identity: VerifierIssueIdentity,
) -> Vec<VerifierChange> {
    let base = group_verifier_issues(base, identity);
    let candidate = group_verifier_issues(candidate, identity);
    let keys = base.keys().chain(candidate.keys()).cloned();
    let keys = keys.collect::<BTreeSet<_>>();
    let changes = keys.into_iter().filter_map(|key| {
        let base_issues = base.get(&key).cloned().unwrap_or_default();
        let candidate_issues = candidate.get(&key).cloned().unwrap_or_default();
        let base_count = base_issues.len();
        let candidate_count = candidate_issues.len();
        (base_count != candidate_count).then_some(VerifierChange {
            key,
            base_issues,
            candidate_issues,
        })
    });
    changes.collect()
}

fn group_verifier_issues(
    issues: &[VerifierIssue],
    identity: VerifierIssueIdentity,
) -> BTreeMap<VerifierKey, Vec<VerifierIssue>> {
    let mut groups: BTreeMap<VerifierKey, Vec<VerifierIssue>> = BTreeMap::new();
    for issue in issues {
        let key = VerifierKey {
            kind: issue.kind,
            package: issue.package.clone(),
            version: match identity {
                VerifierIssueIdentity::LegacySchema => None,
                VerifierIssueIdentity::StructuredSchema => issue.version.clone(),
            },
            dependency: issue.dependency.clone(),
            locations: match identity {
                VerifierIssueIdentity::LegacySchema => Vec::new(),
                VerifierIssueIdentity::StructuredSchema => issue.locations.clone(),
            },
            stage: match identity {
                VerifierIssueIdentity::LegacySchema => None,
                VerifierIssueIdentity::StructuredSchema => issue.stage,
            },
        };
        groups.entry(key).or_default().push(issue.clone());
    }
    for issues in groups.values_mut() {
        issues.sort_by(|left, right| left.message.cmp(&right.message));
    }
    groups
}

fn comparison_summary(
    diagnostic_changes: &[DiagnosticChange],
    verifier_changes: &[VerifierChange],
) -> ComparisonSummary {
    let mut summary = ComparisonSummary::default();
    for change in diagnostic_changes {
        match change.key.severity {
            DiagnosticSeverity::Error => {
                summary.introduced_compiler_errors += change.introduced_count();
                summary.fixed_compiler_errors += change.fixed_count();
            }
            DiagnosticSeverity::Warning => {
                summary.introduced_compiler_warnings += change.introduced_count();
                summary.fixed_compiler_warnings += change.fixed_count();
            }
        }
    }
    for change in verifier_changes {
        summary.introduced_verifier_errors += change.introduced_count();
        summary.fixed_verifier_errors += change.fixed_count();
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::super::report::{
        CompilationStage, CompilerDiagnostic, DiagnosticSeverity, PackageSetReport, Report,
        SelectionReport, SourcePosition, SpanReport, VerifierIssue,
    };
    use super::super::selection::{SelectedPackage, SelectionMode};

    use super::compare_reports;

    #[test]
    fn ignores_diagnostic_message_span_and_legacy_path_changes() {
        let mut base = report();
        base.schema_version = 0;
        base.diagnostics.push(diagnostic(
            DiagnosticSeverity::Error,
            "CannotUnify",
            "base message",
            "/tmp/target/compatibility/sources/prelude/6.0.2/src/Prelude.purs",
            1,
        ));
        base.recompute_summary();
        let mut candidate = report();
        candidate.diagnostics.push(diagnostic(
            DiagnosticSeverity::Error,
            "CannotUnify",
            "candidate message",
            "src/Prelude.purs",
            9,
        ));
        candidate.recompute_summary();

        let comparison = compare_reports(&base, &candidate).unwrap();

        assert!(comparison.diagnostic_changes.is_empty());
        assert!(!comparison.has_regressions());
    }

    #[test]
    fn distinguishes_introduced_errors_warnings_and_fixes() {
        let mut base = report();
        base.diagnostics.push(diagnostic(
            DiagnosticSeverity::Error,
            "FixedError",
            "fixed",
            "src/Prelude.purs",
            1,
        ));
        base.recompute_summary();
        let mut candidate = report();
        candidate.diagnostics.extend([
            diagnostic(DiagnosticSeverity::Error, "NewError", "new", "src/Prelude.purs", 1),
            diagnostic(DiagnosticSeverity::Warning, "NewWarning", "warning", "src/Prelude.purs", 1),
        ]);
        candidate.verifier_errors.push(VerifierIssue::query_error(
            "prelude",
            "6.0.2",
            "src/Prelude.purs",
            CompilationStage::Checking,
            "query failed".to_string(),
        ));
        candidate.recompute_summary();

        let comparison = compare_reports(&base, &candidate).unwrap();

        assert_eq!(comparison.summary.introduced_compiler_errors, 1);
        assert_eq!(comparison.summary.fixed_compiler_errors, 1);
        assert_eq!(comparison.summary.introduced_compiler_warnings, 1);
        assert_eq!(comparison.summary.fixed_compiler_warnings, 0);
        assert_eq!(comparison.summary.introduced_verifier_errors, 1);
        assert!(comparison.has_regressions());
    }

    #[test]
    fn establishes_javascript_baseline_for_previous_report_schema() {
        let mut base = report();
        base.schema_version = 1;
        let mut candidate = report();
        let mut javascript_diagnostic = diagnostic(
            DiagnosticSeverity::Error,
            "JavaScriptError",
            "unsupported JavaScript",
            "src/Prelude.purs",
            1,
        );
        javascript_diagnostic.stage = CompilationStage::JavaScript;
        candidate.diagnostics.push(javascript_diagnostic);
        candidate.verifier_errors.push(VerifierIssue::query_error(
            "prelude",
            "6.0.2",
            "src/Prelude.purs",
            CompilationStage::JavaScript,
            "query failed".to_string(),
        ));
        candidate.recompute_summary();

        let comparison = compare_reports(&base, &candidate).unwrap();

        assert!(comparison.diagnostic_changes.is_empty());
        assert!(comparison.verifier_changes.is_empty());
        assert!(!comparison.has_regressions());
        assert_eq!(comparison.candidate_summary.compiler_errors, 1);
        assert_eq!(comparison.candidate_summary.verifier_errors, 1);
    }

    #[test]
    fn rejects_different_corpora() {
        let base = report();
        let mut candidate = report();
        candidate.summary.source_files += 1;

        let error = compare_reports(&base, &candidate).unwrap_err();

        assert!(error.to_string().contains("source file counts differ"));
    }

    fn report() -> Report {
        let mut report = Report::new(
            PackageSetReport {
                version: "80.3.1".to_string(),
                compiler: "0.15.15".to_string(),
                published: "2026-08-07".to_string(),
            },
            SelectionReport {
                mode: SelectionMode::Combined,
                requested_packages: Vec::new(),
                resolved_packages: vec![SelectedPackage {
                    name: "prelude".to_string(),
                    version: "6.0.2".to_string(),
                }],
            },
        );
        report.summary.source_files = 1;
        report
    }

    fn diagnostic(
        severity: DiagnosticSeverity,
        code: &str,
        message: &str,
        file: &str,
        start: u32,
    ) -> CompilerDiagnostic {
        CompilerDiagnostic {
            package: "prelude".to_string(),
            version: "6.0.2".to_string(),
            file: file.to_string(),
            stage: CompilationStage::Checking,
            severity,
            code: code.to_string(),
            message: message.to_string(),
            span: SpanReport {
                start,
                end: start + 1,
                start_position: Some(SourcePosition { line: 1, column: start + 1 }),
                end_position: Some(SourcePosition { line: 1, column: start + 2 }),
            },
            human: String::new(),
        }
    }
}
