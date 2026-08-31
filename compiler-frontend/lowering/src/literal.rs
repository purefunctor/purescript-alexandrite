use std::fmt;
use std::sync::Arc;

use smol_str::SmolStr;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringLiteral(Arc<[u16]>);

impl StringLiteral {
    pub fn from_utf16(value: impl Into<Arc<[u16]>>) -> StringLiteral {
        StringLiteral(value.into())
    }

    pub fn as_utf16(&self) -> &[u16] {
        &self.0
    }

    pub fn to_utf8(&self) -> Result<String, std::string::FromUtf16Error> {
        String::from_utf16(&self.0)
    }

    pub fn append(&self, suffix: &StringLiteral) -> StringLiteral {
        let mut value = Vec::with_capacity(self.0.len() + suffix.0.len());
        value.extend_from_slice(&self.0);
        value.extend_from_slice(&suffix.0);
        StringLiteral::from_utf16(value)
    }
}

impl fmt::Debug for StringLiteral {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&encode_normal_string(self))
    }
}

impl From<&str> for StringLiteral {
    fn from(value: &str) -> StringLiteral {
        let value = value.encode_utf16().collect::<Vec<_>>();
        StringLiteral::from_utf16(value)
    }
}

impl From<String> for StringLiteral {
    fn from(value: String) -> StringLiteral {
        StringLiteral::from(value.as_str())
    }
}

impl From<SmolStr> for StringLiteral {
    fn from(value: SmolStr) -> StringLiteral {
        StringLiteral::from(value.as_str())
    }
}

fn is_string_gap_character(character: char) -> bool {
    matches!(character, ' ' | '\r' | '\n')
}

pub fn decode_normal_string(original: &str) -> Option<StringLiteral> {
    let text = original.strip_prefix('"')?.strip_suffix('"')?;
    let mut characters = text.chars().peekable();
    let mut decoded = Vec::with_capacity(text.len());

    while let Some(character) = characters.next() {
        if matches!(character, '\r' | '\n') {
            return None;
        }
        if character != '\\' {
            decoded.extend(character.encode_utf16(&mut [0; 2]).iter().copied());
            continue;
        }

        let escaped = characters.next()?;
        match escaped {
            't' => decoded.push('\t' as u16),
            'r' => decoded.push('\r' as u16),
            'n' => decoded.push('\n' as u16),
            '"' => decoded.push('"' as u16),
            '\'' => decoded.push('\'' as u16),
            '\\' => decoded.push('\\' as u16),
            'x' => {
                let mut hexadecimal = String::new();
                for _ in 0..6 {
                    let Some(character) =
                        characters.next_if(|character| character.is_ascii_hexdigit())
                    else {
                        break;
                    };
                    hexadecimal.push(character);
                }
                let value = u32::from_str_radix(&hexadecimal, 16).unwrap_or(0);
                if value <= u16::MAX as u32 {
                    decoded.push(value as u16);
                } else {
                    let character = char::from_u32(value)?;
                    decoded.extend(character.encode_utf16(&mut [0; 2]).iter().copied());
                }
            }
            gap if is_string_gap_character(gap) => {
                while characters.peek().is_some_and(|character| is_string_gap_character(*character))
                {
                    characters.next();
                }
                if characters.next_if_eq(&'\\').is_none() && characters.peek().is_some() {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(StringLiteral::from_utf16(decoded))
}

pub fn decode_raw_string(original: &str) -> Option<StringLiteral> {
    let text = original.strip_prefix("\"\"\"")?.strip_suffix("\"\"\"")?;
    Some(StringLiteral::from(text))
}

pub fn encode_normal_string(value: &StringLiteral) -> String {
    let content = encode_normal_string_content(value);
    format!("\"{content}\"")
}

pub fn encode_normal_string_content(value: &StringLiteral) -> String {
    let mut literal = String::with_capacity(value.as_utf16().len());
    for character in char::decode_utf16(value.as_utf16().iter().copied()) {
        let character = match character {
            Ok(character) => character,
            Err(error) => {
                literal.push_str(&format!("\\x{:06x}", error.unpaired_surrogate()));
                continue;
            }
        };
        match character {
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '\\' => literal.push_str("\\\\"),
            '"' => literal.push_str("\\\""),
            character if character.is_control() => {
                literal.push_str(&format!("\\x{:06x}", character as u32));
            }
            character => literal.push(character),
        }
    }
    literal
}

#[cfg(test)]
mod tests {
    use super::{StringLiteral, decode_normal_string, decode_raw_string, encode_normal_string};

    #[test]
    fn normal_strings_round_trip_every_unicode_scalar_value() {
        let value = (0..=0x10ffff).filter_map(char::from_u32).collect::<String>();
        let value = StringLiteral::from(value);
        let encoded = encode_normal_string(&value);

        assert_eq!(decode_normal_string(&encoded), Some(value));
    }

    #[test]
    fn normal_strings_round_trip_every_utf16_code_unit() {
        let value = (u16::MIN..=u16::MAX).collect::<Vec<_>>();
        let value = StringLiteral::from_utf16(value);
        let encoded = encode_normal_string(&value);

        assert_eq!(decode_normal_string(&encoded), Some(value));
    }

    #[test]
    fn normal_strings_round_trip_escape_boundaries() {
        let cases = ["", "\"\\'", "\0abcdef", "\u{1f}0123456789", "\n\r\t"];

        for value in cases {
            let value = StringLiteral::from(value);
            let encoded = encode_normal_string(&value);
            assert_eq!(decode_normal_string(&encoded), Some(value), "{encoded}");
        }
    }

    #[test]
    fn normal_strings_decode_escape_boundaries() {
        let cases = [
            (r#""\xZ""#, "\0Z"),
            (r#""\x1G""#, "\u{1}G"),
            (r#""\x12345G""#, "\u{12345}G"),
            (r#""\x000041B""#, "AB"),
            (r#""\x10FFFF0""#, "\u{10ffff}0"),
            (r#""\t\r\n\"\'\\""#, "\t\r\n\"'\\"),
        ];

        for (source, expected) in cases {
            assert_eq!(
                decode_normal_string(source),
                Some(StringLiteral::from(expected)),
                "{source}"
            );
        }
    }

    #[test]
    fn normal_strings_decode_gaps() {
        let cases = [("\"a\\ \r\n\\b\"", "ab"), ("\"a\\ \r\n\"", "a")];

        for (source, expected) in cases {
            assert_eq!(
                decode_normal_string(source),
                Some(StringLiteral::from(expected)),
                "{source:?}"
            );
        }
    }

    #[test]
    fn normal_strings_reject_invalid_escapes_and_gaps() {
        let cases = [
            r#""\q""#,
            r#""trailing\"#,
            "\"a\\\t\\b\"",
            "\"a\\\u{a0}\\b\"",
            "\"a\\ q\"",
            "\"line\nfeed\"",
        ];

        for source in cases {
            assert_eq!(decode_normal_string(source), None, "{source:?}");
        }
    }

    #[test]
    fn normal_strings_preserve_lone_surrogates_as_utf16() {
        let value = decode_normal_string(r#""\xD800x\xDFFF""#).unwrap();
        assert_eq!(value.as_utf16(), &[0xd800, b'x' as u16, 0xdfff]);
        assert_eq!(encode_normal_string(&value), r#""\x00d800x\x00dfff""#);
    }

    #[test]
    fn normal_strings_reject_values_outside_the_code_point_domain() {
        assert_eq!(decode_normal_string(r#""\x110000""#), None);
    }

    #[test]
    fn raw_strings_preserve_escape_spelling() {
        let source = "\"\"\"\\n\\\"quoted\\\"\"\"\"";
        let expected = StringLiteral::from(r#"\n\"quoted\""#);
        assert_eq!(decode_raw_string(source), Some(expected));
    }
}
