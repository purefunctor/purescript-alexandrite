mod context;
mod convert;
mod model;
mod render;

pub use context::{DiagnosticsContext, ExternalQueries};
pub use convert::ToDiagnostics;
pub use model::{Diagnostic, DiagnosticCode, RelatedSpan, Severity, Span};
pub use render::{
    format_rich_with_path, format_text, to_lsp_diagnostic, to_lsp_diagnostic_with_line_index,
};
