use analyzer::AnalyzerCapabilities;
use analyzer::position::PositionEncoding;
use lsp_types::{InitializeParams, PositionEncodingKind};

pub fn negotiate_analyzer_capabilities(params: &InitializeParams) -> AnalyzerCapabilities {
    let workspace_edit = params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.workspace_edit.as_ref());
    let honors_rename_annotations = params
        .capabilities
        .text_document
        .as_ref()
        .and_then(|text_document| text_document.rename.as_ref())
        .is_some_and(|rename| rename.honors_change_annotations == Some(true));
    let change_annotations = workspace_edit.is_some_and(|workspace_edit| {
        workspace_edit.document_changes == Some(true)
            && workspace_edit.change_annotation_support.is_some()
            && honors_rename_annotations
    });

    if change_annotations {
        AnalyzerCapabilities::default().with_change_annotations()
    } else {
        AnalyzerCapabilities::default()
    }
}

pub fn negotiate_position_encoding(params: &InitializeParams) -> PositionEncoding {
    let Some(encodings) = params
        .capabilities
        .general
        .as_ref()
        .and_then(|general| general.position_encodings.as_ref())
    else {
        return PositionEncoding::Utf16;
    };

    if encodings.contains(&PositionEncodingKind::UTF8) {
        PositionEncoding::Utf8
    } else if encodings.contains(&PositionEncodingKind::UTF16) {
        PositionEncoding::Utf16
    } else if encodings.contains(&PositionEncodingKind::UTF32) {
        PositionEncoding::Utf32
    } else {
        PositionEncoding::Utf16
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::{ClientCapabilities, GeneralClientCapabilities};

    use super::*;

    fn params(position_encodings: Option<Vec<PositionEncodingKind>>) -> InitializeParams {
        InitializeParams {
            capabilities: ClientCapabilities {
                general: Some(GeneralClientCapabilities {
                    position_encodings,
                    ..GeneralClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        }
    }

    #[test]
    fn defaults_to_utf16_without_client_preference() {
        let params = InitializeParams::default();

        let encoding = negotiate_position_encoding(&params);
        assert_eq!(encoding, PositionEncoding::Utf16);
    }

    #[test]
    fn prefers_utf8_when_available() {
        let params = params(Some(vec![
            PositionEncodingKind::UTF32,
            PositionEncodingKind::UTF16,
            PositionEncodingKind::UTF8,
        ]));

        let encoding = negotiate_position_encoding(&params);
        assert_eq!(encoding, PositionEncoding::Utf8);
    }

    #[test]
    fn falls_back_to_utf16_before_utf32() {
        let params = params(Some(vec![PositionEncodingKind::UTF32, PositionEncodingKind::UTF16]));

        let encoding = negotiate_position_encoding(&params);
        assert_eq!(encoding, PositionEncoding::Utf16);
    }

    #[test]
    fn supports_utf32_when_it_is_the_only_known_option() {
        let params = params(Some(vec![PositionEncodingKind::UTF32]));

        let encoding = negotiate_position_encoding(&params);
        assert_eq!(encoding, PositionEncoding::Utf32);
    }
}
