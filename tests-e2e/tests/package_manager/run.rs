use super::support::{TestWorkspace, assert_success};

#[test]
fn runs_the_configured_module_with_node() {
    let workspace = TestWorkspace::empty();
    workspace.write(
        "spago.yaml",
        r#"workspace: {}
package:
  name: application
  dependencies: []
  run:
    main: Configured
"#,
    );
    workspace.write(
        "src/Configured.purs",
        r#"module Configured where

data Unit = Unit
foreign import data Effect :: Type -> Type
foreign import main :: Effect Unit
"#,
    );
    workspace.write(
        "src/Configured.js",
        "export const main = () => console.log(process.argv.slice(2).join(\",\"));\n",
    );

    let output =
        workspace.command(&["run", "--output", "generated", "--quiet", "--", "first", "second"]);
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "first,second\n");
    assert!(workspace.path().join("generated/Configured/index.js").is_file());
    workspace.assert_spago_calls(
        "",
        &[&["fetch", "-p", "application"], &["sources", "--json", "-p", "application"]],
    );
}
