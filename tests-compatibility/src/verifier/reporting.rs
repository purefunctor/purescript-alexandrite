use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use super::comparison::{
    ComparisonReport, DiagnosticChange, DiagnosticKey, VerifierChange, diagnostic_groups,
};
use super::error::VerifierError;
use super::report::{CompilerDiagnostic, DiagnosticSeverity, Report, VerifierIssue};

#[derive(Clone, Copy)]
enum ChangeKind {
    Introduced,
    Fixed,
}

#[derive(Clone, Copy)]
enum AnnotationLevel {
    Error,
    Warning,
}

impl AnnotationLevel {
    fn as_str(self) -> &'static str {
        match self {
            AnnotationLevel::Error => "error",
            AnnotationLevel::Warning => "warning",
        }
    }
}

pub fn render_markdown(
    comparison: &ComparisonReport,
    candidate: &Report,
    detail_limit: usize,
) -> String {
    let mut output = String::new();
    output.push_str("# Compatibility regression report\n\n");
    output.push_str(&format!(
        "Package set {} for PureScript {}.\n\n",
        html_code(&comparison.package_set.version),
        html_code(&comparison.package_set.compiler)
    ));
    if comparison.has_regressions() {
        output.push_str("❌ The candidate introduces compatibility errors.\n\n");
    } else {
        output.push_str("✅ The candidate introduces no compatibility errors.\n\n");
    }
    render_summary_table(&mut output, comparison);
    render_introduced_errors(&mut output, comparison, detail_limit);
    render_fixed_errors(&mut output, comparison, detail_limit);
    render_warning_changes(&mut output, comparison, detail_limit);
    let candidate_groups = diagnostic_groups(candidate);
    render_candidate_diagnostics(
        &mut output,
        &candidate_groups,
        DiagnosticSeverity::Error,
        "Candidate errors",
        detail_limit,
    );
    render_candidate_diagnostics(
        &mut output,
        &candidate_groups,
        DiagnosticSeverity::Warning,
        "Candidate warnings",
        detail_limit,
    );
    output
}

pub fn append_markdown(path: &Path, markdown: &str) -> Result<(), VerifierError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(markdown.as_bytes())?;
    Ok(())
}

pub fn github_annotations(
    comparison: &ComparisonReport,
    error_limit: usize,
    warning_limit: usize,
) -> String {
    let mut annotations = String::new();
    let introduced_errors = comparison.diagnostic_changes.iter().filter(|change| {
        change.key.severity == DiagnosticSeverity::Error && change.introduced_count() > 0
    });
    let introduced_errors = introduced_errors.take(error_limit);
    let mut emitted_errors = 0;
    for change in introduced_errors {
        let diagnostic = change
            .candidate_diagnostics
            .first()
            .expect("introduced diagnostic has candidate evidence");
        annotations.push_str(&github_command(
            AnnotationLevel::Error,
            &diagnostic_title(&change.key),
            &annotation_diagnostic_message(diagnostic, change.introduced_count()),
        ));
        emitted_errors += 1;
    }

    let verifier_limit = error_limit.saturating_sub(emitted_errors);
    let introduced_verifier_errors = comparison
        .verifier_changes
        .iter()
        .filter(|change| change.introduced_count() > 0)
        .take(verifier_limit);
    for change in introduced_verifier_errors {
        let issue = change
            .candidate_issues
            .first()
            .expect("introduced verifier issue has candidate evidence");
        annotations.push_str(&github_command(
            AnnotationLevel::Error,
            &format!("Compatibility verifier {}", change.key.kind),
            &annotation_verifier_message(issue, change.introduced_count()),
        ));
    }

    let introduced_warnings = comparison
        .diagnostic_changes
        .iter()
        .filter(|change| {
            change.key.severity == DiagnosticSeverity::Warning && change.introduced_count() > 0
        })
        .take(warning_limit);
    for change in introduced_warnings {
        let diagnostic = change
            .candidate_diagnostics
            .first()
            .expect("introduced diagnostic has candidate evidence");
        annotations.push_str(&github_command(
            AnnotationLevel::Warning,
            &diagnostic_title(&change.key),
            &annotation_diagnostic_message(diagnostic, change.introduced_count()),
        ));
    }
    annotations
}

fn render_summary_table(output: &mut String, comparison: &ComparisonReport) {
    output.push_str("| Diagnostic class | Base | Candidate | Introduced | Fixed |\n");
    output.push_str("| --- | ---: | ---: | ---: | ---: |\n");
    output.push_str(&format!(
        "| Compiler errors | {} | {} | {} | {} |\n",
        comparison.base_summary.compiler_errors,
        comparison.candidate_summary.compiler_errors,
        comparison.summary.introduced_compiler_errors,
        comparison.summary.fixed_compiler_errors
    ));
    output.push_str(&format!(
        "| Compiler warnings | {} | {} | {} | {} |\n",
        comparison.base_summary.compiler_warnings,
        comparison.candidate_summary.compiler_warnings,
        comparison.summary.introduced_compiler_warnings,
        comparison.summary.fixed_compiler_warnings
    ));
    output.push_str(&format!(
        "| Verifier errors | {} | {} | {} | {} |\n\n",
        comparison.base_summary.verifier_errors,
        comparison.candidate_summary.verifier_errors,
        comparison.summary.introduced_verifier_errors,
        comparison.summary.fixed_verifier_errors
    ));
}

fn render_introduced_errors(
    output: &mut String,
    comparison: &ComparisonReport,
    detail_limit: usize,
) {
    let diagnostics =
        diagnostic_changes(comparison, DiagnosticSeverity::Error, ChangeKind::Introduced);
    let verifier = verifier_changes(comparison, ChangeKind::Introduced);
    output.push_str("## Introduced errors\n\n");
    if diagnostics.is_empty() && verifier.is_empty() {
        output.push_str("None.\n\n");
        return;
    }
    if !diagnostics.is_empty() {
        render_diagnostic_changes(output, &diagnostics, ChangeKind::Introduced, detail_limit);
    }
    if !verifier.is_empty() {
        render_verifier_changes(output, &verifier, ChangeKind::Introduced, detail_limit);
    }
}

fn render_fixed_errors(output: &mut String, comparison: &ComparisonReport, detail_limit: usize) {
    let diagnostics = diagnostic_changes(comparison, DiagnosticSeverity::Error, ChangeKind::Fixed);
    let verifier = verifier_changes(comparison, ChangeKind::Fixed);
    let total = comparison.summary.fixed_compiler_errors + comparison.summary.fixed_verifier_errors;
    output.push_str(&format!("<details>\n<summary>Fixed errors ({total})</summary>\n\n"));
    if diagnostics.is_empty() && verifier.is_empty() {
        output.push_str("None.\n\n");
    } else {
        if !diagnostics.is_empty() {
            render_diagnostic_changes(output, &diagnostics, ChangeKind::Fixed, detail_limit);
        }
        if !verifier.is_empty() {
            render_verifier_changes(output, &verifier, ChangeKind::Fixed, detail_limit);
        }
    }
    output.push_str("</details>\n\n");
}

fn render_warning_changes(output: &mut String, comparison: &ComparisonReport, detail_limit: usize) {
    let introduced =
        diagnostic_changes(comparison, DiagnosticSeverity::Warning, ChangeKind::Introduced);
    let fixed = diagnostic_changes(comparison, DiagnosticSeverity::Warning, ChangeKind::Fixed);
    output.push_str(&format!(
        "<details>\n<summary>Warning changes ({} introduced, {} fixed)</summary>\n\n",
        comparison.summary.introduced_compiler_warnings, comparison.summary.fixed_compiler_warnings
    ));
    output.push_str("**Introduced**\n\n");
    render_diagnostic_changes(output, &introduced, ChangeKind::Introduced, detail_limit);
    output.push_str("**Fixed**\n\n");
    render_diagnostic_changes(output, &fixed, ChangeKind::Fixed, detail_limit);
    output.push_str("</details>\n\n");
}

fn diagnostic_changes(
    comparison: &ComparisonReport,
    severity: DiagnosticSeverity,
    change_kind: ChangeKind,
) -> Vec<&DiagnosticChange> {
    let changes = comparison.diagnostic_changes.iter().filter(|change| {
        change.key.severity == severity && diagnostic_change_count(change, change_kind) > 0
    });
    changes.collect()
}

fn verifier_changes(
    comparison: &ComparisonReport,
    change_kind: ChangeKind,
) -> Vec<&VerifierChange> {
    let changes = comparison
        .verifier_changes
        .iter()
        .filter(|change| verifier_change_count(change, change_kind) > 0);
    changes.collect()
}

fn render_diagnostic_changes(
    output: &mut String,
    changes: &[&DiagnosticChange],
    change_kind: ChangeKind,
    detail_limit: usize,
) {
    if changes.is_empty() {
        output.push_str("None.\n\n");
        return;
    }
    for change in changes.iter().take(detail_limit) {
        let count = diagnostic_change_count(change, change_kind);
        let evidence = diagnostic_evidence(change, change_kind);
        output.push_str(&format_diagnostic_item(&change.key, evidence, count));
    }
    render_omitted(output, changes.len(), detail_limit);
    output.push('\n');
}

fn render_verifier_changes(
    output: &mut String,
    changes: &[&VerifierChange],
    change_kind: ChangeKind,
    detail_limit: usize,
) {
    for change in changes.iter().take(detail_limit) {
        let count = verifier_change_count(change, change_kind);
        let evidence = verifier_evidence(change, change_kind);
        output.push_str(&format!(
            "- <strong>Verifier {}</strong>{}: {}\n",
            html_code(change.key.kind.as_str()),
            count_suffix(count),
            escape_markdown_text(&evidence.message)
        ));
    }
    render_omitted(output, changes.len(), detail_limit);
    if !changes.is_empty() {
        output.push('\n');
    }
}

fn render_candidate_diagnostics(
    output: &mut String,
    groups: &BTreeMap<DiagnosticKey, Vec<CompilerDiagnostic>>,
    severity: DiagnosticSeverity,
    title: &str,
    detail_limit: usize,
) {
    let matching_groups = groups.iter().filter(|(key, _)| key.severity == severity);
    let matching_groups = matching_groups.collect::<Vec<_>>();
    let diagnostic_counts = matching_groups.iter().map(|(_, diagnostics)| diagnostics.len());
    let total = diagnostic_counts.sum::<usize>();
    output.push_str(&format!("<details>\n<summary>{title} ({total})</summary>\n\n"));
    for (key, diagnostics) in matching_groups.iter().take(detail_limit) {
        let evidence = diagnostics.first().expect("diagnostic group is non-empty");
        output.push_str(&format_diagnostic_item(key, evidence, diagnostics.len()));
    }
    if matching_groups.is_empty() {
        output.push_str("None.\n");
    }
    render_omitted(output, matching_groups.len(), detail_limit);
    output.push_str("\n</details>\n\n");
}

fn diagnostic_change_count(change: &DiagnosticChange, change_kind: ChangeKind) -> usize {
    match change_kind {
        ChangeKind::Introduced => change.introduced_count(),
        ChangeKind::Fixed => change.fixed_count(),
    }
}

fn verifier_change_count(change: &VerifierChange, change_kind: ChangeKind) -> usize {
    match change_kind {
        ChangeKind::Introduced => change.introduced_count(),
        ChangeKind::Fixed => change.fixed_count(),
    }
}

fn diagnostic_evidence(change: &DiagnosticChange, change_kind: ChangeKind) -> &CompilerDiagnostic {
    let evidence = match change_kind {
        ChangeKind::Introduced => change.candidate_diagnostics.first(),
        ChangeKind::Fixed => change.base_diagnostics.first(),
    };
    evidence.expect("changed diagnostic has evidence")
}

fn verifier_evidence(change: &VerifierChange, change_kind: ChangeKind) -> &VerifierIssue {
    let evidence = match change_kind {
        ChangeKind::Introduced => change.candidate_issues.first(),
        ChangeKind::Fixed => change.base_issues.first(),
    };
    evidence.expect("changed verifier issue has evidence")
}

fn format_diagnostic_item(
    key: &DiagnosticKey,
    diagnostic: &CompilerDiagnostic,
    count: usize,
) -> String {
    let location = diagnostic
        .span
        .start_position
        .as_ref()
        .map(|position| format!(":{}:{}", position.line, position.column))
        .unwrap_or_default();
    let path = format!("{}@{}/{}{}", key.package, key.version, key.file, location);
    format!(
        "- {} — <strong>{}</strong> ({}){}: {}\n",
        html_code(&path),
        escape_html(&key.code),
        html_code(key.stage.as_str()),
        count_suffix(count),
        escape_markdown_text(&diagnostic.message)
    )
}

fn render_omitted(output: &mut String, count: usize, limit: usize) {
    if count > limit {
        output.push_str(&format!(
            "- … {} additional groups are available in the JSON artifact.\n",
            count - limit
        ));
    }
}

fn count_suffix(count: usize) -> String {
    if count == 1 { String::new() } else { format!(" × {count}") }
}

fn diagnostic_title(key: &DiagnosticKey) -> String {
    format!("{}@{}/{} [{}:{}]", key.package, key.version, key.file, key.stage, key.code)
}

fn annotation_diagnostic_message(diagnostic: &CompilerDiagnostic, count: usize) -> String {
    let location = diagnostic
        .span
        .start_position
        .as_ref()
        .map(|position| format!("{}:{}:{}: ", diagnostic.file, position.line, position.column))
        .unwrap_or_else(|| format!("{}: ", diagnostic.file));
    format!("{location}{}{}", diagnostic.message, count_suffix(count))
}

fn annotation_verifier_message(issue: &VerifierIssue, count: usize) -> String {
    format!("{}{}", issue.message, count_suffix(count))
}

fn github_command(level: AnnotationLevel, title: &str, message: &str) -> String {
    let level = level.as_str();
    format!("::{level} title={}::{}\n", github_property(title), github_data(message))
}

fn github_property(value: &str) -> String {
    github_data(value).replace(':', "%3A").replace(',', "%2C")
}

fn github_data(value: &str) -> String {
    value.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A")
}

fn html_code(value: &str) -> String {
    format!("<code>{}</code>", escape_html(value))
}

fn escape_html(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_markdown_text(value: &str) -> String {
    let value = value.replace('\r', "");
    let lines = value.split('\n').map(|line| {
        let mut escaped = String::new();
        for character in line.chars() {
            match character {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-'
                | '.' | '!' | '|' => {
                    escaped.push('\\');
                    escaped.push(character);
                }
                _ => escaped.push(character),
            }
        }
        escaped
    });
    let lines = lines.collect::<Vec<_>>();
    lines.join("<br>")
}

#[cfg(test)]
mod tests {
    use super::super::comparison::compare_reports;
    use super::super::report::{
        CompilationStage, CompilerDiagnostic, DiagnosticSeverity, PackageSetReport, Report,
        SelectionReport, SourcePosition, SpanReport,
    };
    use super::super::selection::{SelectedPackage, SelectionMode};

    use super::{github_annotations, render_markdown};

    #[test]
    fn renders_warnings_safely_in_markdown_and_annotations() {
        let base = report();
        let mut candidate = report();
        candidate.diagnostics.push(warning("first line%\r\nsecond *line* _[x]_ `tick` \\ <tag> &"));
        candidate.recompute_summary();
        let comparison = compare_reports(&base, &candidate).unwrap();

        let markdown = render_markdown(&comparison, &candidate, 20);
        let annotations = github_annotations(&comparison, 10, 10);

        assert!(markdown.contains("<summary>Warning changes (1 introduced, 0 fixed)</summary>"));
        assert!(
            markdown.contains(
                r"first line%<br>second \*line\* \_\[x\]\_ \`tick\` \\ &lt;tag&gt; &amp;"
            )
        );
        assert!(annotations.starts_with("::warning title="));
        assert!(annotations.contains("first line%25%0D%0Asecond *line*"));
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

    fn warning(message: &str) -> CompilerDiagnostic {
        CompilerDiagnostic {
            package: "prelude".to_string(),
            version: "6.0.2".to_string(),
            file: "src/Prelude.purs".to_string(),
            stage: CompilationStage::Checking,
            severity: DiagnosticSeverity::Warning,
            code: "Warning".to_string(),
            message: message.to_string(),
            span: SpanReport {
                start: 1,
                end: 2,
                start_position: Some(SourcePosition { line: 1, column: 2 }),
                end_position: Some(SourcePosition { line: 1, column: 3 }),
            },
            human: String::new(),
        }
    }
}
