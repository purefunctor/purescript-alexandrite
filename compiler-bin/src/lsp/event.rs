use async_lsp::LanguageClient;
use files::FileId;
use lsp_types::{PublishDiagnosticsParams, Url};

use crate::lsp::error::LspError;
use crate::lsp::{State, StateSnapshot};

pub fn emit_collect_diagnostics(state: &mut State, uri: Url) -> Result<(), LspError> {
    let files = state.files.read();
    let uri = uri.as_str();

    if let Some(file_id) = files.id(uri) {
        state.client.emit(CollectDiagnostics(file_id))?;
    }

    Ok(())
}

pub struct CollectDiagnostics(FileId);

pub fn collect_diagnostics(state: &mut State, id: CollectDiagnostics) -> Result<(), LspError> {
    state.spawn(move |snapshot| {
        let _span = tracing::info_span!("collect_diagnostics").entered();
        collect_diagnostics_core(snapshot, id).inspect_err(|error| error.emit_trace())
    });
    Ok(())
}

fn collect_diagnostics_core(
    mut snapshot: StateSnapshot,
    CollectDiagnostics(id): CollectDiagnostics,
) -> Result<(), LspError> {
    let collected = snapshot
        .with_analyzer_context(|context| analyzer::diagnostics::implementation(context, id))?;

    snapshot.client.publish_diagnostics(PublishDiagnosticsParams {
        uri: collected.uri,
        diagnostics: collected.diagnostics,
        version: None,
    })?;

    Ok(())
}
