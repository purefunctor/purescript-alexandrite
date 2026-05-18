use std::fmt;
use std::time::Duration;

use super::command;

const FORMATTER_TIMEOUT: Duration = Duration::from_secs(10);

pub fn run(format_command: &str, input: &[u8]) -> Result<Vec<u8>, FormattingError> {
    let parts = command::split(format_command)
        .ok_or_else(|| FormattingError::parse(format_command.to_string()))?;
    let mut parts = parts.into_iter();
    let program = parts.next().ok_or_else(FormattingError::empty_command)?;

    let mut child = command::piped(&program, parts).spawn().map_err(|source| FormattingError {
        kind: FormattingErrorKind::Spawn { command: format_command.to_string(), source },
    })?;

    command::write_stdin(&mut child, input)
        .map_err(|source| FormattingError { kind: FormattingErrorKind::WriteStdin { source } })?;

    let output =
        command::wait_with_output_timeout(child, FORMATTER_TIMEOUT).map_err(
            |error| match error {
                command::WaitWithOutputTimeoutError::Wait(source) => FormattingError {
                    kind: FormattingErrorKind::Wait { command: format_command.to_string(), source },
                },
                command::WaitWithOutputTimeoutError::TimedOut(cleanup) => FormattingError {
                    kind: FormattingErrorKind::TimedOut {
                        command: format_command.to_string(),
                        input_len: input.len(),
                        cleanup,
                    },
                },
                command::WaitWithOutputTimeoutError::Poll { source, cleanup } => FormattingError {
                    kind: FormattingErrorKind::Poll {
                        command: format_command.to_string(),
                        input_len: input.len(),
                        source,
                        cleanup,
                    },
                },
            },
        )?;

    if !output.status.success() {
        return Err(FormattingError {
            kind: FormattingErrorKind::Exited {
                status: output.status,
                stderr: output.stderr,
                stdout: output.stdout,
            },
        });
    }

    Ok(output.stdout)
}

#[derive(Debug)]
pub struct FormattingError {
    kind: FormattingErrorKind,
}

impl FormattingError {
    fn parse(command: String) -> FormattingError {
        FormattingError { kind: FormattingErrorKind::Parse { command } }
    }

    fn empty_command() -> FormattingError {
        FormattingError { kind: FormattingErrorKind::EmptyCommand }
    }
}

#[derive(Debug)]
enum FormattingErrorKind {
    Parse {
        command: String,
    },
    EmptyCommand,
    Spawn {
        command: String,
        source: std::io::Error,
    },
    WriteStdin {
        source: std::io::Error,
    },
    Wait {
        command: String,
        source: std::io::Error,
    },
    TimedOut {
        command: String,
        input_len: usize,
        cleanup: command::ChildCleanup,
    },
    Poll {
        command: String,
        input_len: usize,
        source: std::io::Error,
        cleanup: command::ChildCleanup,
    },
    Exited {
        status: std::process::ExitStatus,
        stderr: Vec<u8>,
        stdout: Vec<u8>,
    },
}

impl fmt::Display for FormattingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            FormattingErrorKind::Parse { command } => {
                write!(f, "failed to parse --format-command: {command}")
            }
            FormattingErrorKind::EmptyCommand => write!(f, "--format-command is empty"),
            FormattingErrorKind::Spawn { command, source } => {
                write!(f, "failed to spawn formatter '{command}': {source}")
            }
            FormattingErrorKind::WriteStdin { source } => {
                write!(f, "failed writing to formatter stdin: {source}")
            }
            FormattingErrorKind::Wait { command, source } => {
                write!(f, "failed to wait for formatter '{command}': {source}")
            }
            FormattingErrorKind::TimedOut { command, input_len, cleanup } => {
                write!(
                    f,
                    "{}",
                    format_cleanup(
                        &format!(
                            "formatter timed out after {:?} (input {} bytes): {command}",
                            cleanup.timeout, input_len
                        ),
                        cleanup,
                    )
                )
            }
            FormattingErrorKind::Poll { command, input_len, source, cleanup } => {
                write!(
                    f,
                    "{}",
                    format_cleanup(
                        &format!(
                            "failed waiting for formatter '{command}' (input {} bytes): {source}",
                            input_len
                        ),
                        cleanup,
                    )
                )
            }
            FormattingErrorKind::Exited { status, stderr, stdout } => {
                write!(f, "formatter exited with {status}: {}", format_output(stderr, stdout))
            }
        }
    }
}

impl std::error::Error for FormattingError {}

fn format_cleanup(message: &str, cleanup: &command::ChildCleanup) -> String {
    let mut message = message.to_string();
    if let Some(error) = &cleanup.kill_error {
        message.push_str(&format!("; kill failed: {error}"));
    }
    if let Some(error) = &cleanup.wait_error {
        message.push_str(&format!("; wait failed: {error}"));
    }
    message
}

fn format_output(stderr: &[u8], stdout: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let stdout = String::from_utf8_lossy(stdout);

    let stderr = stderr.trim();
    let stdout = stdout.trim();

    let mut details = String::new();
    if !stderr.is_empty() {
        details.push_str(stderr);
    }
    if !stdout.is_empty() {
        if !details.is_empty() {
            details.push('\n');
        }
        details.push_str(stdout);
    }
    if details.is_empty() {
        details.push_str("<no output>");
    }
    details
}
