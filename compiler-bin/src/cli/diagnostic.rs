use std::ffi::OsStr;

use diagnostics::{Diagnostic, Span};
use itertools::Itertools;
use line_index::LineIndex;
use serde::de::IgnoredAny;
use usage::diagnostic::ArgvSpan;

use super::{ColorChoice, ConfigurationError, Program};
use crate::compile::use_color;

impl Program {
    pub fn parse_with_diagnostics() -> Program {
        let arguments = std::env::args_os().collect_vec();
        let arguments = arguments.iter().map(|argument| argument.as_os_str()).collect_vec();
        let mut warnings = vec![];
        let error = match Program::try_parse_from_with_warnings(&arguments, &mut warnings) {
            Ok(program) => {
                eprint!("{}", usage::render_warnings(&warnings));
                return program;
            }
            Err(error) => error,
        };

        match error {
            usage::Error::Help { cmd, long } => {
                let page = usage::help::render_styled(
                    Program::spec(),
                    cmd,
                    long,
                    usage::help::Style::auto(),
                );
                print!("{}", page.unwrap_or_default());
                std::process::exit(0);
            }
            usage::Error::HelpAll { cmd } => {
                let page = usage::help::render_all_styled(
                    Program::spec(),
                    cmd,
                    usage::help::Style::auto(),
                );
                print!("{}", page.unwrap_or_default());
                std::process::exit(0);
            }
            usage::Error::Version { .. } => {
                println!("iris {}", crate::VERSION);
                std::process::exit(0);
            }
            usage::Error::MissingArgsHelp { cmd } => {
                let page = usage::help::render_styled(
                    Program::spec(),
                    cmd,
                    false,
                    usage::help::Style::auto_stderr(),
                );
                eprint!("{}", page.unwrap_or_default());
            }
            error => {
                let report = usage::diagnostic::report(Program::spec(), &arguments[1..], &error);
                let (content, span) = command_line(&arguments[1..], report.location);
                let message = report.rendered.strip_prefix("error: ").unwrap_or(&report.rendered);
                eprint!(
                    "{}",
                    render(
                        report.code.as_str(),
                        message.trim_end(),
                        &content,
                        "command line",
                        span
                    )
                );
            }
        }
        std::process::exit(2);
    }
}

impl ConfigurationError {
    pub(crate) fn render(&self) -> String {
        match self {
            ConfigurationError::ReadFile { path, error } => {
                let content = path.display().to_string();
                render(
                    "ConfigurationFileRead",
                    &format!("Cannot read the configuration file: {error}"),
                    &content,
                    "--config-file",
                    Span::new(0, content.len() as u32),
                )
            }
            ConfigurationError::InvalidJson { input, content, error } => {
                let message = error.to_string();
                let location = format!(" at line {} column {}", error.line(), error.column());
                let message = message.strip_suffix(&location).unwrap_or(&message);
                render("InvalidConfiguration", message, content, input, json_span(content, error))
            }
        }
    }
}

fn render(code: &'static str, message: &str, content: &str, input: &str, span: Span) -> String {
    let diagnostic = Diagnostic::error(code, message, span, "cli");
    diagnostics::format_rich_with_path(
        &[diagnostic],
        content,
        &LineIndex::new(content),
        input,
        use_color(ColorChoice::Auto),
    )
}

fn command_line(arguments: &[&OsStr], location: Option<ArgvSpan>) -> (String, Span) {
    let mut content = "iris".to_string();
    let mut span = None;
    for (index, argument) in arguments.iter().enumerate() {
        content.push(' ');
        let text = argument.to_string_lossy();
        let quoted = text
            .chars()
            .any(|character| character.is_whitespace() || matches!(character, '"' | '\\' | '\''));
        if quoted {
            content.push('"');
        }
        if let Some(location) = location.filter(|location| location.index == index) {
            let bytes = argument.as_encoded_bytes();
            let start =
                String::from_utf8_lossy(&bytes[..location.start]).escape_debug().to_string().len();
            let end =
                String::from_utf8_lossy(&bytes[..location.end]).escape_debug().to_string().len();
            span = Some(Span::new((content.len() + start) as u32, (content.len() + end) as u32));
        }
        content.extend(text.escape_debug());
        if quoted {
            content.push('"');
        }
    }
    let span = span.unwrap_or_else(|| Span::new(0, content.len() as u32));
    (content, span)
}

fn json_span(content: &str, error: &serde_json::Error) -> Span {
    if error.is_eof() {
        // Keep incomplete input visible rather than displaying an empty trailing line.
        let end = content.trim_end_matches([' ', '\t', '\r', '\n']).len() as u32;
        return Span::new(end, end);
    }
    let line_start: usize =
        content.split_inclusive('\n').take(error.line().saturating_sub(1)).map(str::len).sum();
    let end = (line_start + error.column()).min(content.len());

    // Data errors point just past the consumed token. Let Serde find token boundaries as
    // well, so escaped strings and numbers use the same grammar as configuration parsing.
    if error.is_data() {
        let mut offset = 0;
        while offset < end {
            let character = content[offset..].chars().next().unwrap();
            if character.is_ascii_whitespace()
                || matches!(character, '{' | '}' | '[' | ']' | ':' | ',')
            {
                offset += character.len_utf8();
                continue;
            }
            let mut tokens =
                serde_json::Deserializer::from_str(&content[offset..]).into_iter::<IgnoredAny>();
            if !matches!(tokens.next(), Some(Ok(_))) {
                break;
            }
            let token_end = offset + tokens.byte_offset();
            if end <= token_end {
                return Span::new(offset as u32, token_end as u32);
            }
            offset = token_end;
        }
    }

    let start = content.floor_char_boundary(end.saturating_sub(1));
    let end = start + content[start..].chars().next().map_or(0, char::len_utf8);
    Span::new(start as u32, end as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_spans_follow_escaped_byte_offsets() {
        let arguments = [OsStr::new("--option"), OsStr::new("λ\n\"bad\"")];
        let location = ArgvSpan { index: 1, start: 3, end: 8 };
        let (content, span) = command_line(&arguments, Some(location));
        assert_eq!(content, r#"iris --option "λ\n\"bad\"""#);
        assert_eq!(&content[span.start as usize..span.end as usize], r#"\"bad\""#);
    }

    #[cfg(unix)]
    #[test]
    fn command_line_spans_follow_lossy_arguments() {
        use std::os::unix::ffi::OsStrExt;

        let arguments = [OsStr::from_bytes(b"--option=\xffbad")];
        let location = ArgvSpan { index: 0, start: 10, end: 13 };
        let (content, span) = command_line(&arguments, Some(location));
        assert_eq!(content, "iris --option=�bad");
        assert_eq!(&content[span.start as usize..span.end as usize], "bad");
    }
}
