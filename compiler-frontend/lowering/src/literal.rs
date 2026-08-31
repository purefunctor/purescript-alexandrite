use smol_str::SmolStr;

fn is_string_gap_character(character: char) -> bool {
    matches!(character, ' ' | '\r' | '\n')
}

pub fn decode_normal_string(original: &str) -> Option<SmolStr> {
    let text = original.strip_prefix('"')?.strip_suffix('"')?;
    let mut characters = text.chars().peekable();
    let mut decoded = String::with_capacity(text.len());

    while let Some(character) = characters.next() {
        if matches!(character, '\r' | '\n') {
            return None;
        }
        if character != '\\' {
            decoded.push(character);
            continue;
        }

        let escaped = characters.next()?;
        match escaped {
            't' => decoded.push('\t'),
            'r' => decoded.push('\r'),
            'n' => decoded.push('\n'),
            '"' => decoded.push('"'),
            '\'' => decoded.push('\''),
            '\\' => decoded.push('\\'),
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
                decoded.push(char::from_u32(value)?);
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

    Some(SmolStr::from(decoded))
}

pub fn decode_raw_string(original: &str) -> Option<SmolStr> {
    let text = original.strip_prefix("\"\"\"")?.strip_suffix("\"\"\"")?;
    Some(SmolStr::from(text))
}

pub fn encode_normal_string(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');
    for character in value.chars() {
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
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::{decode_normal_string, decode_raw_string, encode_normal_string};

    #[test]
    fn normal_strings_round_trip_every_unicode_scalar_value() {
        let value = (0..=0x10ffff).filter_map(char::from_u32).collect::<String>();
        let encoded = encode_normal_string(&value);

        assert_eq!(decode_normal_string(&encoded).as_deref(), Some(value.as_str()));
    }

    #[test]
    fn normal_strings_round_trip_escape_boundaries() {
        let cases = ["", "\"\\'", "\0abcdef", "\u{1f}0123456789", "\n\r\t"];

        for value in cases {
            let encoded = encode_normal_string(value);
            assert_eq!(decode_normal_string(&encoded).as_deref(), Some(value), "{encoded}");
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
            assert_eq!(decode_normal_string(source).as_deref(), Some(expected), "{source}");
        }
    }

    #[test]
    fn normal_strings_decode_gaps() {
        let cases = [("\"a\\ \r\n\\b\"", "ab"), ("\"a\\ \r\n\"", "a")];

        for (source, expected) in cases {
            assert_eq!(decode_normal_string(source).as_deref(), Some(expected), "{source:?}");
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
    fn normal_strings_reject_values_outside_the_unicode_scalar_domain() {
        for source in [r#""\xD800""#, r#""\xDFFF""#, r#""\x110000""#] {
            assert_eq!(decode_normal_string(source), None, "{source}");
        }
    }

    #[test]
    fn raw_strings_preserve_escape_spelling() {
        let source = "\"\"\"\\n\\\"quoted\\\"\"\"\"";
        assert_eq!(decode_raw_string(source).as_deref(), Some(r#"\n\"quoted\""#));
    }
}
