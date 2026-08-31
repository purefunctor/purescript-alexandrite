use std::path::PathBuf;
use std::process::{Child, Output};
use std::thread;
use std::time::{Duration, Instant};

use itertools::Itertools;

use super::support::TestWorkspace;

#[test]
fn watches_a_single_package_with_real_spago() {
    let workspace = TestWorkspace::empty();
    workspace.write(
        "spago.yaml",
        r#"workspace: {}
package:
  name: application
  dependencies: []
"#,
    );
    workspace.write(
        "src/Main.purs",
        r#"module Main where

value = 42
"#,
    );

    let child = workspace.spawn(&["watch"]);
    let output = workspace.path().join("output/Main/index.js");
    let result = watch_until_outputs(child, &[output]);
    insta::assert_snapshot!(normalized_watch_log(&result), @r#"
    [TIME] 1 file changed: Main
    "#);

    workspace.assert_spago_calls(
        "",
        &[&["fetch", "-p", "application"], &["sources", "--json", "-p", "application"]],
    );
}

#[test]
fn watches_the_whole_workspace_from_a_root_package_subdirectory() {
    let workspace = TestWorkspace::empty();
    workspace.write(
        "spago.yaml",
        r#"workspace: {}
package:
  name: application
  dependencies: []
"#,
    );
    workspace.write("src/Application.purs", "module Application where\n");
    workspace.write(
        "packages/library/spago.yaml",
        r#"package:
  name: library
  dependencies: []
"#,
    );
    workspace.write("packages/library/src/Library.purs", "module Library where\n");

    let child = workspace.spawn_in("src", &["watch"]);
    let outputs = [
        workspace.path().join("output/Application/index.js"),
        workspace.path().join("output/Library/index.js"),
    ];
    let result = watch_until_outputs(child, &outputs);
    insta::assert_snapshot!(normalized_watch_log(&result), @r#"
    [TIME] 2 files changed: Application, Library
    "#);

    workspace.assert_spago_calls("src", &[&["fetch"], &["sources", "--json"]]);
}

fn watch_until_outputs(mut child: Child, outputs: &[PathBuf]) -> Output {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !outputs.iter().all(|output| output.is_file()) {
        if let Some(status) = child.try_wait().unwrap() {
            let output = child.wait_with_output().unwrap();
            panic!(
                "watch exited before compiling with status {status}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "watch did not compile within 30 seconds\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        thread::sleep(Duration::from_millis(50));
    }
    child.kill().unwrap();
    child.wait_with_output().unwrap()
}

fn normalized_watch_log(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines().map(|line| {
        let Some((timestamp, message)) = line.split_once("] ") else {
            return line.to_owned();
        };
        let timestamp = timestamp.as_bytes();
        let is_timestamp = timestamp.len() == 9
            && timestamp[0] == b'['
            && timestamp[3] == b':'
            && timestamp[6] == b':'
            && timestamp[1..3].iter().all(u8::is_ascii_digit)
            && timestamp[4..6].iter().all(u8::is_ascii_digit)
            && timestamp[7..9].iter().all(u8::is_ascii_digit);
        if is_timestamp { format!("[TIME] {message}") } else { line.to_owned() }
    });
    lines.join("\n")
}
