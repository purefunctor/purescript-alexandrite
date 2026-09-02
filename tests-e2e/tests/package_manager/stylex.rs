use std::path::Path;
use std::process::Command;

use super::support::{TestWorkspace, assert_success};

#[test]
fn emits_javascript_consumable_by_stylex_babel() {
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
        "src/Tokens.purs",
        r#"module Tokens (rowMarker, variables) where

import Alexandrite.StyleX as StyleX

variables = StyleX.defineVars { accent: "blue" }

rowMarker :: StyleX.Marker
rowMarker = StyleX.defineMarker
"#,
    );
    workspace.write(
        "src/Main.purs",
        r#"module Main where

import Alexandrite.StyleX as StyleX
import Alexandrite.StyleX.When as When
import Tokens (rowMarker, variables)

theme = StyleX.createTheme variables { accent: "white" }

styles = StyleX.create
  { root:
      { color: StyleX.conditionalValue "blue"
          [ When.ancestorMarker ":hover" rowMarker "red" ]
      }
  }
"#,
    );

    let output = workspace.command(&["build", "--quiet"]);
    assert_success(&output);
    let generated = workspace.read("output/Main/index.js");
    assert!(
        generated
            .contains("import { rowMarker as Tokens_rowMarker, variables as Tokens_variables }"),
        "unexpected generated JavaScript:\n{generated}"
    );

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = manifest.join("tools/verify-stylex.mjs");
    let verification =
        Command::new("node").arg(script).arg(workspace.path().join("output")).output().unwrap();
    assert_success(&verification);
    workspace.assert_spago_calls(
        "",
        &[&["fetch", "-p", "application"], &["sources", "--json", "-p", "application"]],
    );
}
