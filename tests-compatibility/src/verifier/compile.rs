use std::collections::HashMap;
use std::fs;

use building::{QueryEngine, QueryError, prim};
use diagnostics::{
    Diagnostic, DiagnosticsContext, Severity, Span, ToDiagnostics, format_rustc_with_path,
};
use files::{FileId, Files};
use rayon::prelude::*;
use url::Url;

use super::error::VerifierError;
use super::report::{
    CompilationStage, CompilerDiagnostic, DiagnosticSeverity, IssueLocation, SourcePosition,
    SpanReport, VerifierIssue,
};
use super::sources::SourceFile;

#[derive(Debug, Default)]
pub struct CompileReport {
    pub diagnostics: Vec<CompilerDiagnostic>,
    pub verifier_errors: Vec<VerifierIssue>,
}

#[derive(Debug, Clone)]
struct FileMetadata {
    package: String,
    version: String,
    relative_path: String,
    content: String,
}

pub fn compile_sources(source_files: &[SourceFile]) -> Result<CompileReport, VerifierError> {
    let mut engine = QueryEngine::default();
    let mut files = Files::default();
    prim::configure(&mut engine, &mut files);

    let mut report = CompileReport::default();
    let mut file_ids = Vec::new();
    let mut file_metadata = HashMap::new();

    for source in source_files {
        let content = fs::read_to_string(&source.path)?;
        let absolute_path = fs::canonicalize(&source.path)?;
        let uri = Url::from_file_path(&absolute_path)
            .map_err(|_| VerifierError::FileUrl(absolute_path.clone()))?
            .to_string();
        let file_id = files.insert(uri, content.clone());
        engine.set_content(file_id, content.clone());
        file_ids.push(file_id);
        file_metadata.insert(
            file_id,
            FileMetadata {
                package: source.package.clone(),
                version: source.version.clone(),
                relative_path: source.relative_path.to_string_lossy().replace('\\', "/"),
                content,
            },
        );
    }

    register_modules(&engine, &file_ids, &file_metadata, &mut report);

    let file_reports = file_ids.par_iter().map(|&file_id| {
        let engine = engine.snapshot();
        let mut file_report = CompileReport::default();
        collect_file(
            &engine,
            file_id,
            file_metadata.get(&file_id).expect("file metadata exists"),
            &mut file_report,
        );
        file_report
    });
    let file_reports = file_reports.collect::<Vec<_>>();

    for file_report in file_reports {
        report.diagnostics.extend(file_report.diagnostics);
        report.verifier_errors.extend(file_report.verifier_errors);
    }

    Ok(report)
}

fn register_modules(
    engine: &QueryEngine,
    file_ids: &[FileId],
    file_metadata: &HashMap<FileId, FileMetadata>,
    report: &mut CompileReport,
) {
    let mut modules: HashMap<String, FileId> = HashMap::new();

    for &file_id in file_ids {
        let metadata = file_metadata.get(&file_id).expect("file metadata exists");
        match engine.parsed(file_id) {
            Ok((parsed, errors)) => {
                report.diagnostics.extend(parse_diagnostics(metadata, &errors));

                if let Some(module_name) = parsed.module_name(&metadata.content) {
                    let module_name = module_name.to_string();
                    if let Some(existing) = modules.get(&module_name) {
                        let first = file_metadata.get(existing).expect("file metadata exists");
                        report.verifier_errors.push(VerifierIssue::duplicate_module(
                            &module_name,
                            IssueLocation {
                                package: first.package.clone(),
                                version: first.version.clone(),
                                file: first.relative_path.clone(),
                            },
                            IssueLocation {
                                package: metadata.package.clone(),
                                version: metadata.version.clone(),
                                file: metadata.relative_path.clone(),
                            },
                        ));
                    } else {
                        engine.set_module_file(&module_name, file_id);
                        modules.insert(module_name, file_id);
                    }
                }
            }
            Err(error) => {
                push_query_error(report, metadata, CompilationStage::Parse, error);
            }
        }
    }
}

fn collect_file(
    engine: &QueryEngine,
    file_id: FileId,
    metadata: &FileMetadata,
    report: &mut CompileReport,
) {
    let Ok((parsed, _)) = engine.parsed(file_id) else {
        return;
    };
    let root = parsed.syntax_node();

    if let Err(error) = engine.stabilized(file_id) {
        push_query_error(report, metadata, CompilationStage::Stabilizing, error);
        return;
    }

    match engine.indexed(file_id) {
        Ok(indexed) => {
            with_diagnostics_context(engine, file_id, &root, metadata, |ctx| {
                for error in &indexed.errors {
                    let diagnostics = error.to_diagnostics(&ctx);
                    if diagnostics.is_empty() {
                        report.diagnostics.push(debug_diagnostic(
                            metadata,
                            CompilationStage::Indexing,
                            "IndexingError",
                            error,
                        ));
                    } else {
                        report.diagnostics.extend(convert_diagnostics(
                            metadata,
                            CompilationStage::Indexing,
                            diagnostics,
                        ));
                    }
                }
            });
        }
        Err(error) => {
            push_query_error(report, metadata, CompilationStage::Indexing, error);
            return;
        }
    }

    match engine.resolved(file_id) {
        Ok(resolved) => {
            with_diagnostics_context(engine, file_id, &root, metadata, |ctx| {
                for error in &resolved.errors {
                    report.diagnostics.extend(convert_diagnostics(
                        metadata,
                        CompilationStage::Resolving,
                        error.to_diagnostics(&ctx),
                    ));
                }
            });
        }
        Err(error) => {
            push_query_error(report, metadata, CompilationStage::Resolving, error);
        }
    }

    match engine.lowered(file_id) {
        Ok(lowered) => {
            with_diagnostics_context(engine, file_id, &root, metadata, |ctx| {
                for error in &lowered.errors {
                    report.diagnostics.extend(convert_diagnostics(
                        metadata,
                        CompilationStage::Lowering,
                        error.to_diagnostics(&ctx),
                    ));
                }
            });
        }
        Err(error) => push_query_error(report, metadata, CompilationStage::Lowering, error),
    }

    match engine.grouped(file_id) {
        Ok(grouped) => {
            with_diagnostics_context(engine, file_id, &root, metadata, |ctx| {
                for error in &grouped.cycle_errors {
                    report.diagnostics.extend(convert_diagnostics(
                        metadata,
                        CompilationStage::Lowering,
                        error.to_diagnostics(&ctx),
                    ));
                }
            });
        }
        Err(error) => push_query_error(report, metadata, CompilationStage::Grouping, error),
    }

    if let Err(error) = engine.bracketed(file_id) {
        push_query_error(report, metadata, CompilationStage::Bracketing, error);
    }

    if let Err(error) = engine.sectioned(file_id) {
        push_query_error(report, metadata, CompilationStage::Sectioning, error);
    }

    match engine.checked(file_id) {
        Ok(checked) => {
            with_diagnostics_context(engine, file_id, &root, metadata, |ctx| {
                for error in &checked.errors {
                    report.diagnostics.extend(convert_diagnostics(
                        metadata,
                        CompilationStage::Checking,
                        error.to_diagnostics(&ctx),
                    ));
                }
            });
        }
        Err(error) => push_query_error(report, metadata, CompilationStage::Checking, error),
    }

    if let Err(error) = engine.corefn(file_id) {
        push_query_error(report, metadata, CompilationStage::CoreFn, error);
        return;
    }

    if let Err(error) = engine.javascript(file_id) {
        push_query_error(report, metadata, CompilationStage::JavaScript, error);
    }
}

fn with_diagnostics_context(
    engine: &QueryEngine,
    file_id: FileId,
    root: &syntax::SyntaxNode,
    metadata: &FileMetadata,
    f: impl FnOnce(DiagnosticsContext<'_, QueryEngine>),
) {
    let Ok(stabilized) = engine.stabilized(file_id) else {
        return;
    };
    let Ok(indexed) = engine.indexed(file_id) else {
        return;
    };
    let Ok(lowered) = engine.lowered(file_id) else {
        return;
    };
    let Ok(checked) = engine.checked(file_id) else {
        return;
    };
    f(DiagnosticsContext::new(
        engine,
        &metadata.content,
        root,
        &stabilized,
        &indexed,
        &lowered,
        &checked,
    ));
}

fn parse_diagnostics(
    metadata: &FileMetadata,
    errors: &[parsing::ParseError],
) -> Vec<CompilerDiagnostic> {
    let diagnostics = errors.iter().map(|error| {
        let start = error.offset as u32;
        let content_end = metadata.content.len() as u32;
        let end = start.saturating_add(1).min(content_end);
        let diagnostic = Diagnostic::error(
            "ParseError",
            error.message.to_string(),
            Span::new(start, end),
            "parse",
        );
        compiler_diagnostic(metadata, CompilationStage::Parse, diagnostic)
    });
    diagnostics.collect()
}

fn debug_diagnostic(
    metadata: &FileMetadata,
    stage: CompilationStage,
    code: &'static str,
    error: &impl std::fmt::Debug,
) -> CompilerDiagnostic {
    let diagnostic = Diagnostic::error(
        code,
        format!("{error:?}"),
        Span::new(0, metadata.content.len() as u32),
        stage.as_str(),
    );
    compiler_diagnostic(metadata, stage, diagnostic)
}

fn convert_diagnostics(
    metadata: &FileMetadata,
    stage: CompilationStage,
    diagnostics: Vec<Diagnostic>,
) -> Vec<CompilerDiagnostic> {
    let diagnostics =
        diagnostics.into_iter().map(|diagnostic| compiler_diagnostic(metadata, stage, diagnostic));
    diagnostics.collect()
}

fn compiler_diagnostic(
    metadata: &FileMetadata,
    stage: CompilationStage,
    diagnostic: Diagnostic,
) -> CompilerDiagnostic {
    let severity = match diagnostic.severity {
        Severity::Error => DiagnosticSeverity::Error,
        Severity::Warning => DiagnosticSeverity::Warning,
    };
    let human = format_rustc_with_path(
        std::slice::from_ref(&diagnostic),
        &metadata.content,
        &metadata.relative_path,
    );

    CompilerDiagnostic {
        package: metadata.package.clone(),
        version: metadata.version.clone(),
        file: metadata.relative_path.clone(),
        stage,
        severity,
        code: diagnostic.code.to_string(),
        message: diagnostic.message,
        span: SpanReport {
            start: diagnostic.span.start,
            end: diagnostic.span.end,
            start_position: Some(source_position(&metadata.content, diagnostic.span.start)),
            end_position: Some(source_position(&metadata.content, diagnostic.span.end)),
        },
        human,
    }
}

fn push_query_error(
    report: &mut CompileReport,
    metadata: &FileMetadata,
    stage: CompilationStage,
    error: QueryError,
) {
    report.verifier_errors.push(VerifierIssue::query_error(
        &metadata.package,
        &metadata.version,
        &metadata.relative_path,
        stage,
        format!("{error:?}"),
    ));
}

fn source_position(content: &str, offset: u32) -> SourcePosition {
    let offset = offset as usize;
    let prefix = content.get(..offset).expect("diagnostic span is a source text boundary");
    let preceding_lines = prefix.bytes().filter(|byte| *byte == b'\n');
    let line = preceding_lines.count() as u32 + 1;
    let current_line = prefix.rsplit_once('\n').map_or(prefix, |(_, current_line)| current_line);
    let characters = current_line.chars();
    let column = characters.count() as u32 + 1;
    SourcePosition { line, column }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::super::report::{CompilationStage, IssueLocation, VerifierIssueKind};
    use super::super::sources::SourceFile;

    use super::compile_sources;

    #[test]
    fn smoke_compiles_two_modules() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        let lib = dir.path().join("src/Lib.purs");
        let main = dir.path().join("src/Main.purs");
        fs::write(&lib, "module Lib where\n\nx :: Int\nx = 1\n").unwrap();
        fs::write(&main, "module Main where\n\nimport Lib\n\ny :: Int\ny = x\n").unwrap();

        let sources = [source("fixture", "1.0.0", &lib), source("fixture", "1.0.0", &main)];
        let report = compile_sources(&sources).unwrap();

        assert!(report.verifier_errors.is_empty(), "{:#?}", report.verifier_errors);
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    }

    #[test]
    fn duplicate_module_reports_verifier_issue() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        let a = dir.path().join("src/A.purs");
        let b = dir.path().join("src/B.purs");
        fs::write(&a, "module Main where\n").unwrap();
        fs::write(&b, "module Main where\n").unwrap();

        let report =
            compile_sources(&[source("fixture-a", "1.0.0", &a), source("fixture-b", "2.0.0", &b)])
                .unwrap();

        let duplicate = report
            .verifier_errors
            .iter()
            .find(|issue| issue.kind == VerifierIssueKind::DuplicateModule)
            .expect("duplicate module issue");
        assert_eq!(
            duplicate.locations,
            [
                IssueLocation {
                    package: "fixture-a".to_string(),
                    version: "1.0.0".to_string(),
                    file: "src/A.purs".to_string(),
                },
                IssueLocation {
                    package: "fixture-b".to_string(),
                    version: "2.0.0".to_string(),
                    file: "src/B.purs".to_string(),
                },
            ]
        );
    }

    #[test]
    fn reports_parse_errors_at_end_of_file() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        let path = dir.path().join("src/Broken.purs");
        let content = "module Broken where\n\nimport Prelude (";
        fs::write(&path, content).unwrap();

        let report = compile_sources(&[source("fixture", "1.0.0", &path)]).unwrap();
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.stage == CompilationStage::Parse
                    && diagnostic.span.start == content.len() as u32
            })
            .expect("parse diagnostic at end of file");

        assert_eq!(diagnostic.file, "src/Broken.purs");
        assert_eq!(diagnostic.span.end, content.len() as u32);
        assert_eq!(diagnostic.span.end_position.as_ref().unwrap().line, 3);
        assert_eq!(diagnostic.span.end_position.as_ref().unwrap().column, 17);
    }

    fn source(package: &str, version: &str, path: &std::path::Path) -> SourceFile {
        SourceFile {
            package: package.to_string(),
            version: version.to_string(),
            path: path.to_path_buf(),
            relative_path: std::path::Path::new("src").join(path.file_name().unwrap()),
        }
    }
}
