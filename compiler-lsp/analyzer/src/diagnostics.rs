use building_types::QueryProxy;
use diagnostics::{DiagnosticsContext, ToDiagnostics};
use files::FileId;
use lsp_types::{Diagnostic, Url};

use crate::{AnalyzerContext, AnalyzerError, AnalyzerHost, common};

pub struct CollectedDiagnostics {
    pub uri: Url,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn implementation<Host>(
    context: &AnalyzerContext<Host>,
    file_id: FileId,
) -> Result<CollectedDiagnostics, AnalyzerError>
where
    Host: AnalyzerHost,
    Host::Queries: diagnostics::ExternalQueries,
{
    let queries = context.queries();
    let content = queries.content(file_id);

    let (parsed, _) = queries.parsed(file_id)?;
    let root = parsed.syntax_node();

    let stabilized = queries.stabilized(file_id)?;
    let indexed = queries.indexed(file_id)?;
    let resolved = queries.resolved(file_id)?;
    let lowered = queries.lowered(file_id)?;
    let checked = queries.checked(file_id)?;

    let uri = common::file_uri(context, file_id)?;
    let diagnostics_context = DiagnosticsContext::new(
        queries,
        &content,
        &root,
        &stabilized,
        &indexed,
        &lowered,
        &checked,
    );

    let mut all_diagnostics = vec![];

    for error in &lowered.errors {
        all_diagnostics.extend(error.to_diagnostics(&diagnostics_context));
    }

    for error in &resolved.errors {
        all_diagnostics.extend(error.to_diagnostics(&diagnostics_context));
    }

    for error in &checked.errors {
        all_diagnostics.extend(error.to_diagnostics(&diagnostics_context));
    }

    let position_encoding = context.position_encoding().into();
    let diagnostics = all_diagnostics.iter().filter_map(|diagnostic| {
        diagnostics::to_lsp_diagnostic(diagnostic, &content, &uri, &position_encoding)
    });

    let diagnostics = diagnostics.collect();
    Ok(CollectedDiagnostics { uri, diagnostics })
}
