use std::fmt;
use std::time::Duration;

use super::command;

const FORMATTER_TIMEOUT: Duration = Duration::from_secs(10);

pub fn run(format_command: &str, input: &[u8]) -> Result<Vec<u8>, FormattingError> {
    run_with_timeout(format_command, input, FORMATTER_TIMEOUT)
}

fn run_with_timeout(
    format_command: &str,
    input: &[u8],
    timeout: Duration,
) -> Result<Vec<u8>, FormattingError> {
    let parts = command::split(format_command)
        .ok_or_else(|| FormattingError::parse(format_command.to_string()))?;
    let mut parts = parts.into_iter();
    let program = parts.next().ok_or_else(FormattingError::empty_command)?;

    let mut child = command::piped(&program, parts).spawn().map_err(|source| FormattingError {
        kind: FormattingErrorKind::Spawn { command: format_command.to_string(), source },
    })?;

    command::write_stdin(&mut child, input).map_err(|source| {
        cleanup_write_failure_child(&mut child);
        FormattingError { kind: FormattingErrorKind::WriteStdin { source } }
    })?;

    let output = command::wait_with_output_timeout(child, timeout)
        .map_err(|error| map_wait_with_output_error(format_command, input.len(), error))?;

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

fn cleanup_write_failure_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn map_wait_with_output_error(
    format_command: &str,
    input_len: usize,
    error: command::WaitWithOutputTimeoutError,
) -> FormattingError {
    match error {
        command::WaitWithOutputTimeoutError::Wait(source) => FormattingError {
            kind: FormattingErrorKind::Wait { command: format_command.to_string(), source },
        },
        command::WaitWithOutputTimeoutError::TimedOut(cleanup) => FormattingError {
            kind: FormattingErrorKind::TimedOut {
                command: format_command.to_string(),
                input_len,
                cleanup,
            },
        },
        command::WaitWithOutputTimeoutError::Poll { source, cleanup } => FormattingError {
            kind: FormattingErrorKind::Poll {
                command: format_command.to_string(),
                input_len,
                source,
                cleanup,
            },
        },
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[cfg(unix)]
    #[test]
    fn run_reports_parse_failure() {
        let error = run("\"", b"input").unwrap_err();

        assert_eq!(error.to_string(), "failed to parse --format-command: \"");
    }

    #[cfg(unix)]
    #[test]
    fn run_reports_empty_command() {
        let error = run("\"\"", b"input").unwrap_err();

        assert_eq!(error.to_string(), "--format-command is empty");
    }

    #[cfg(unix)]
    #[test]
    fn run_reports_spawn_failure() {
        let error = run("command-that-does-not-exist", b"input").unwrap_err();

        assert!(
            error
                .to_string()
                .starts_with("failed to spawn formatter 'command-that-does-not-exist':")
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_reports_exit_output() {
        let error = run("sh -c 'printf err >&2; printf out; exit 7'", b"input").unwrap_err();

        assert_eq!(error.to_string(), "formatter exited with exit status: 7: err\nout");
    }

    #[cfg(unix)]
    #[test]
    fn run_reports_stdin_write_failure() {
        let input = vec![b'x'; 1_000_000];
        let error = run("sh -c 'exec 0<&-; sleep 1'", &input).unwrap_err();

        assert!(error.to_string().starts_with("failed writing to formatter stdin:"));
    }

    #[cfg(unix)]
    #[test]
    fn run_reports_timeout() {
        let error =
            run_with_timeout("sh -c 'sleep 1'", b"", Duration::from_millis(10)).unwrap_err();

        assert_eq!(
            error.to_string(),
            "formatter timed out after 10ms (input 0 bytes): sh -c 'sleep 1'"
        );
    }

    #[test]
    fn map_wait_with_output_error_formats_wait_error() {
        let error = map_wait_with_output_error(
            "formatter",
            3,
            command::WaitWithOutputTimeoutError::Wait(io::Error::other("wait failed")),
        );

        assert_eq!(error.to_string(), "failed to wait for formatter 'formatter': wait failed");
    }

    #[test]
    fn map_wait_with_output_error_formats_poll_error() {
        let error = map_wait_with_output_error(
            "formatter",
            3,
            command::WaitWithOutputTimeoutError::Poll {
                source: io::Error::other("poll failed"),
                cleanup: command::ChildCleanup {
                    timeout: Duration::from_secs(10),
                    kill_error: None,
                    wait_error: None,
                },
            },
        );

        assert_eq!(
            error.to_string(),
            "failed waiting for formatter 'formatter' (input 3 bytes): poll failed"
        );
    }

    #[test]
    fn timed_out_error_includes_cleanup_failures() {
        let error = FormattingError {
            kind: FormattingErrorKind::TimedOut {
                command: "formatter".to_string(),
                input_len: 3,
                cleanup: command::ChildCleanup {
                    timeout: Duration::from_secs(10),
                    kill_error: Some(io::Error::other("kill failed")),
                    wait_error: Some(io::Error::other("wait failed")),
                },
            },
        };

        assert_eq!(
            error.to_string(),
            "formatter timed out after 10s (input 3 bytes): formatter; kill failed: kill failed; wait failed: wait failed"
        );
    }

    #[test]
    fn poll_error_includes_cleanup_failures() {
        let error = FormattingError {
            kind: FormattingErrorKind::Poll {
                command: "formatter".to_string(),
                input_len: 3,
                source: io::Error::other("poll failed"),
                cleanup: command::ChildCleanup {
                    timeout: Duration::from_secs(10),
                    kill_error: Some(io::Error::other("kill failed")),
                    wait_error: Some(io::Error::other("wait failed")),
                },
            },
        };

        assert_eq!(
            error.to_string(),
            "failed waiting for formatter 'formatter' (input 3 bytes): poll failed; kill failed: kill failed; wait failed: wait failed"
        );
    }

    #[cfg(unix)]
    #[test]
    fn exited_error_without_output_reports_placeholder() {
        let error = FormattingError {
            kind: FormattingErrorKind::Exited {
                status: std::process::Command::new("sh").args(["-c", "exit 1"]).status().unwrap(),
                stderr: vec![],
                stdout: vec![],
            },
        };

        assert_eq!(error.to_string(), "formatter exited with exit status: 1: <no output>");
    }

    #[test]
    fn write_stdin_error_formats_message() {
        let error = FormattingError {
            kind: FormattingErrorKind::WriteStdin { source: io::Error::other("broken pipe") },
        };

        assert_eq!(error.to_string(), "failed writing to formatter stdin: broken pipe");
    }

    #[test]
    fn wait_error_formats_message() {
        let error = FormattingError {
            kind: FormattingErrorKind::Wait {
                command: "formatter".to_string(),
                source: io::Error::other("wait failed"),
            },
        };

        assert_eq!(error.to_string(), "failed to wait for formatter 'formatter': wait failed");
    }

    #[test]
    fn timed_out_error_without_cleanup_failures_stays_compact() {
        let error = FormattingError {
            kind: FormattingErrorKind::TimedOut {
                command: "formatter".to_string(),
                input_len: 3,
                cleanup: command::ChildCleanup {
                    timeout: Duration::from_secs(10),
                    kill_error: None,
                    wait_error: None,
                },
            },
        };

        assert_eq!(error.to_string(), "formatter timed out after 10s (input 3 bytes): formatter");
    }

    #[test]
    fn exited_error_with_stdout_only_omits_newline() {
        let error = FormattingError {
            kind: FormattingErrorKind::Exited {
                status: std::process::Command::new("sh").args(["-c", "exit 1"]).status().unwrap(),
                stderr: vec![],
                stdout: b"out\n".to_vec(),
            },
        };

        assert_eq!(error.to_string(), "formatter exited with exit status: 1: out");
    }
}
