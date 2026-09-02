use super::support::{TestWorkspace, assert_success};

#[test]
fn builds_a_single_package_with_real_spago() {
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

    let output = workspace.command(&["build", "--quiet"]);
    assert_success(&output);
    assert!(workspace.path().join("spago.lock").is_file());
    assert!(workspace.path().join("output/Main/index.js").is_file());
    workspace.assert_spago_calls(
        "",
        &[&["fetch", "-p", "application"], &["sources", "--json", "-p", "application"]],
    );
}

#[test]
fn builds_resilient_output_despite_diagnostics() {
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

usable = 42

broken = missing
"#,
    );

    let output = workspace.command(&["build", "--quiet", "--resilient"]);
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("'missing' is not in scope"), "unexpected stderr:\n{stderr}");

    let generated = workspace.read("output/Main/index.js");
    assert!(generated.contains("Generated code reached a source error"));
    assert!(generated.contains("export const broken"));
    assert!(generated.contains("export const usable = 42 | 0;"));
    workspace.assert_spago_calls(
        "",
        &[&["fetch", "-p", "application"], &["sources", "--json", "-p", "application"]],
    );
}

#[test]
fn gates_initializer_cycle_output_on_resilience() {
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

first :: Int
first = second

second :: Int
second = first
"#,
    );

    let strict = workspace.command(&["build", "--quiet"]);
    assert!(!strict.status.success());
    let stderr = String::from_utf8_lossy(&strict.stderr);
    assert!(stderr.contains("JavaScriptInitializerCycle"), "unexpected stderr:\n{stderr}");
    assert!(!workspace.path().join("output/Main/index.js").exists());

    let resilient = workspace.command(&["build", "--quiet", "--resilient"]);
    assert!(!resilient.status.success());
    let stderr = String::from_utf8_lossy(&resilient.stderr);
    assert!(stderr.contains("JavaScriptInitializerCycle"), "unexpected stderr:\n{stderr}");
    let generated = workspace.read("output/Main/index.js");
    assert!(generated.contains("Top-level value initializer cycle"));
    assert!(!generated.contains("@__PURE__"));
    workspace.assert_spago_calls(
        "",
        &[
            &["fetch", "-p", "application"],
            &["sources", "--json", "-p", "application"],
            &["fetch", "-p", "application"],
            &["sources", "--json", "-p", "application"],
        ],
    );
}

#[test]
fn reports_backend_failures_as_diagnostics() {
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

import Alexandrite.StyleX (Props, Style, props)

partialProps :: Style -> Props
partialProps = props
"#,
    );

    let output = workspace.command(&["build", "--quiet"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("FunctionalCodegen"), "unexpected stderr:\n{stderr}");
    assert!(
        stderr.contains(
            "Alexandrite.StyleX.props must be used as a direct, saturated intrinsic call"
        ),
        "unexpected stderr:\n{stderr}"
    );
    assert!(!stderr.contains("failed to generate JavaScript"), "unexpected stderr:\n{stderr}");
    assert!(!workspace.path().join("output/Main/index.js").exists());
    workspace.assert_spago_calls(
        "",
        &[&["fetch", "-p", "application"], &["sources", "--json", "-p", "application"]],
    );
}

#[test]
fn builds_the_whole_workspace_from_a_root_package_subdirectory() {
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

    let output = workspace.command_in("src", &["build", "--quiet"]);
    assert_success(&output);
    assert!(workspace.path().join("output/Application/index.js").is_file());
    assert!(workspace.path().join("output/Library/index.js").is_file());
    workspace.assert_spago_calls("src", &[&["fetch"], &["sources", "--json"]]);
}
