use std::path::PathBuf;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

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

    let mut child = workspace.spawn(&["watch", "--quiet"]);
    let output = workspace.path().join("output/Main/index.js");
    wait_for_outputs(&mut child, &[output]);
    child.kill().unwrap();
    child.wait().unwrap();

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

    let mut child = workspace.spawn_in("src", &["watch", "--quiet"]);
    let outputs = [
        workspace.path().join("output/Application/index.js"),
        workspace.path().join("output/Library/index.js"),
    ];
    wait_for_outputs(&mut child, &outputs);
    child.kill().unwrap();
    child.wait().unwrap();

    workspace.assert_spago_calls("src", &[&["fetch"], &["sources", "--json"]]);
}

fn wait_for_outputs(child: &mut Child, outputs: &[PathBuf]) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !outputs.iter().all(|output| output.is_file()) {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("watch exited before compiling with status {status}");
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("watch did not compile within 30 seconds");
        }
        thread::sleep(Duration::from_millis(50));
    }
}
