use std::process::{self, Child, ExitStatus, Output};
use std::time::{Duration, Instant};
use std::{io, thread};

const CHILD_CLEANUP_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) fn split(command: &str) -> Option<Vec<String>> {
    let mut command = command.trim();

    // Users sometimes include extra quoting in editor configs, e.g.
    // `--format-command "\"purs-tidy format\""`.
    if command.len() >= 2 {
        let bytes = command.as_bytes();
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        let is_wrapped = (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'');
        if is_wrapped {
            command = &command[1..command.len() - 1];
        }
    }

    shlex::split(command)
}

pub(super) fn piped(program: &str, args: impl IntoIterator<Item = String>) -> process::Command {
    let mut command = process::Command::new(program);
    command.args(args);
    command.stdin(process::Stdio::piped());
    command.stdout(process::Stdio::piped());
    command.stderr(process::Stdio::piped());
    command
}

pub(super) fn write_stdin(child: &mut Child, input: &[u8]) -> io::Result<()> {
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(input)?;
    }
    Ok(())
}

pub(super) fn wait_with_output_timeout(
    mut child: Child,
    timeout: Duration,
) -> Result<Output, WaitWithOutputTimeoutError> {
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child.wait_with_output().map_err(WaitWithOutputTimeoutError::Wait);
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    return Err(WaitWithOutputTimeoutError::TimedOut(cleanup_child(
                        &mut child, timeout,
                    )));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(source) => return Err(poll_wait_error(source, &mut child, timeout)),
        }
    }
}

fn poll_wait_error(
    source: io::Error,
    child: &mut Child,
    timeout: Duration,
) -> WaitWithOutputTimeoutError {
    WaitWithOutputTimeoutError::Poll { source, cleanup: cleanup_child(child, timeout) }
}

#[derive(Debug)]
pub(super) enum WaitWithOutputTimeoutError {
    Wait(io::Error),
    TimedOut(ChildCleanup),
    Poll { source: io::Error, cleanup: ChildCleanup },
}

#[derive(Debug)]
pub(super) struct ChildCleanup {
    pub timeout: Duration,
    pub kill_error: Option<io::Error>,
    pub wait_error: Option<io::Error>,
}

fn cleanup_child(child: &mut Child, timeout: Duration) -> ChildCleanup {
    let kill_error = child.kill().err();
    let wait_error = wait_for_cleanup(child);

    ChildCleanup { timeout, kill_error, wait_error }
}

fn wait_for_cleanup(child: &mut Child) -> Option<io::Error> {
    let start = Instant::now();

    loop {
        match cleanup_wait_state(child.try_wait(), start.elapsed()) {
            CleanupWaitState::Exited => return None,
            CleanupWaitState::Pending => thread::sleep(Duration::from_millis(10)),
            CleanupWaitState::Failed(error) => return Some(error),
        }
    }
}

enum CleanupWaitState {
    Exited,
    Pending,
    Failed(io::Error),
}

fn cleanup_wait_state(
    result: io::Result<Option<ExitStatus>>,
    elapsed: Duration,
) -> CleanupWaitState {
    match result {
        Ok(Some(_)) => CleanupWaitState::Exited,
        Ok(None) => {
            if elapsed >= CHILD_CLEANUP_TIMEOUT {
                CleanupWaitState::Failed(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "timed out waiting {:?} for killed child process to exit",
                        CHILD_CLEANUP_TIMEOUT
                    ),
                ))
            } else {
                CleanupWaitState::Pending
            }
        }
        Err(error) => CleanupWaitState::Failed(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn write_stdin_and_wait_with_output_timeout_capture_output() {
        let mut child = piped("cat", std::iter::empty::<String>()).spawn().unwrap();
        write_stdin(&mut child, b"hello").unwrap();

        let output = wait_with_output_timeout(child, Duration::from_secs(1)).unwrap();

        assert!(output.status.success());
        assert_eq!(output.stdout, b"hello");
    }

    #[cfg(unix)]
    #[test]
    fn wait_with_output_timeout_cleans_up_timed_out_process() {
        let child = piped("sh", ["-c".to_string(), "sleep 2".to_string()]).spawn().unwrap();

        let error = wait_with_output_timeout(child, Duration::from_millis(10)).unwrap_err();

        match error {
            WaitWithOutputTimeoutError::TimedOut(cleanup) => {
                assert_eq!(cleanup.timeout, Duration::from_millis(10));
                assert!(cleanup.kill_error.is_none());
                assert!(cleanup.wait_error.is_none());
            }
            other => panic!("expected timeout, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn write_stdin_succeeds_without_pipe() {
        let mut child = process::Command::new("cat")
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .spawn()
            .unwrap();

        write_stdin(&mut child, b"hello").unwrap();
        child.wait().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn poll_wait_error_cleans_up_child() {
        let mut child = piped("sh", ["-c".to_string(), "sleep 2".to_string()]).spawn().unwrap();

        let error =
            poll_wait_error(io::Error::other("poll failed"), &mut child, Duration::from_millis(10));

        match error {
            WaitWithOutputTimeoutError::Poll { source, cleanup } => {
                assert_eq!(source.kind(), io::ErrorKind::Other);
                assert_eq!(source.to_string(), "poll failed");
                assert_eq!(cleanup.timeout, Duration::from_millis(10));
            }
            other => panic!("expected poll error, got {other:?}"),
        }
    }

    #[test]
    fn cleanup_wait_state_times_out() {
        let state = cleanup_wait_state(Ok(None), CHILD_CLEANUP_TIMEOUT);

        match state {
            CleanupWaitState::Failed(error) => assert_eq!(error.kind(), io::ErrorKind::TimedOut),
            CleanupWaitState::Exited => panic!("expected timeout, got exited"),
            CleanupWaitState::Pending => panic!("expected timeout, got pending"),
        }
    }

    #[test]
    fn cleanup_wait_state_propagates_errors() {
        let state = cleanup_wait_state(Err(io::Error::other("wait failed")), Duration::ZERO);

        match state {
            CleanupWaitState::Failed(error) => assert_eq!(error.to_string(), "wait failed"),
            CleanupWaitState::Exited => panic!("expected error, got exited"),
            CleanupWaitState::Pending => panic!("expected error, got pending"),
        }
    }

    #[test]
    fn split_handles_shell_quoted_args() {
        assert_eq!(split("tr 'a-z' 'A-Z'"), Some(vec!["tr".into(), "a-z".into(), "A-Z".into()]));
    }

    #[test]
    fn split_strips_outer_quotes_before_parsing() {
        assert_eq!(split("\"tr a-z A-Z\""), Some(vec!["tr".into(), "a-z".into(), "A-Z".into()]));
    }
}
