use std::io;
use std::process::{self, Child, Output};
use std::thread;
use std::time::{Duration, Instant};

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
            Err(source) => {
                return Err(WaitWithOutputTimeoutError::Poll {
                    source,
                    cleanup: cleanup_child(&mut child, timeout),
                });
            }
        }
    }
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
        match child.try_wait() {
            Ok(Some(_)) => return None,
            Ok(None) => {
                if start.elapsed() >= CHILD_CLEANUP_TIMEOUT {
                    return Some(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!(
                            "timed out waiting {:?} for killed child process to exit",
                            CHILD_CLEANUP_TIMEOUT
                        ),
                    ));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn split_handles_shell_quoted_args() {
        assert_eq!(split("tr 'a-z' 'A-Z'"), Some(vec!["tr".into(), "a-z".into(), "A-Z".into()]));
    }

    #[test]
    fn split_strips_outer_quotes_before_parsing() {
        assert_eq!(split("\"tr a-z A-Z\""), Some(vec!["tr".into(), "a-z".into(), "A-Z".into()]));
    }
}
