use analyzer::position::PositionEncoding;
use async_lsp::lsp_types::{InitializeParams, PositionEncodingKind};

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
    use async_lsp::lsp_types::{ClientCapabilities, GeneralClientCapabilities};

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
