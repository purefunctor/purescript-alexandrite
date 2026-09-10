use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::Child;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use url::Url;

use super::support::TestWorkspace;

struct LanguageServer {
    child: Child,
    messages: Receiver<Value>,
    notifications: Vec<Value>,
    next_request: u32,
}

impl LanguageServer {
    fn start(
        workspace: &TestWorkspace,
        directory: &str,
        arguments: &[&str],
        root: &Path,
    ) -> LanguageServer {
        let mut arguments = arguments.to_vec();
        arguments.extend(["--stdio", "--lsp-log", "off"]);
        let mut child = workspace.spawn_in(directory, &arguments);
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let (sender, messages) = mpsc::channel();
        thread::spawn(move || {
            loop {
                let mut length = None;
                loop {
                    let mut header = String::new();
                    if stdout.read_line(&mut header).unwrap() == 0 {
                        return;
                    }
                    if header == "\r\n" {
                        break;
                    }
                    if let Some(value) = header.strip_prefix("Content-Length: ") {
                        length = Some(value.trim().parse::<usize>().unwrap());
                    }
                }
                let mut content = vec![0; length.expect("missing Content-Length")];
                stdout.read_exact(&mut content).unwrap();
                let message = serde_json::from_slice(&content).unwrap();
                if sender.send(message).is_err() {
                    return;
                }
            }
        });
        let mut server = LanguageServer { child, messages, notifications: vec![], next_request: 1 };
        let result = server.request("initialize", json!({
            "processId": null,
            "capabilities": {},
            "workspaceFolders": [{"uri": Url::from_directory_path(root).unwrap(), "name": "project"}]
        }));
        assert!(result["capabilities"].is_object(), "{result}");
        server.notify("initialized", json!({}));
        server
    }

    fn send(&mut self, message: Value) {
        let content = message.to_string();
        let stdin = self.child.stdin.as_mut().unwrap();
        write!(stdin, "Content-Length: {}\r\n\r\n{content}", content.len()).unwrap();
        stdin.flush().unwrap();
    }

    fn notify(&mut self, method: &str, parameters: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": parameters}));
    }

    fn request(&mut self, method: &str, parameters: Value) -> Value {
        let request = self.next_request;
        self.next_request += 1;
        self.send(json!({"jsonrpc": "2.0", "id": request, "method": method, "params": parameters}));
        loop {
            let message = self.messages.recv_timeout(Duration::from_secs(10)).unwrap();
            if message.get("id") == Some(&json!(request)) {
                assert!(message.get("error").is_none(), "{message}");
                return message["result"].clone();
            }
            self.notifications.push(message);
        }
    }

    fn assert_diagnostics(&mut self, uri: &Url, version: i32, enabled: bool) {
        // A response proves the preceding document notification reached the server before
        // the bounded wait for asynchronous diagnostics, including when none are expected.
        self.request("workspace/symbol", json!({"query": "noSuchSymbol"}));
        if enabled {
            let message = if self.notifications.is_empty() {
                self.messages.recv_timeout(Duration::from_secs(10)).unwrap()
            } else {
                self.notifications.remove(0)
            };
            assert_eq!(message["method"], "textDocument/publishDiagnostics");
            assert_eq!(message["params"]["uri"], uri.as_str());
            assert_eq!(message["params"]["version"], version);
            let diagnostics = message["params"]["diagnostics"].as_array().unwrap();
            assert!(diagnostics.iter().any(|diagnostic| diagnostic["severity"] == 1), "{message}");
        } else {
            assert!(self.notifications.is_empty(), "{:?}", self.notifications);
            assert_eq!(
                self.messages.recv_timeout(Duration::from_millis(500)),
                Err(RecvTimeoutError::Timeout)
            );
        }
    }

    fn shutdown(&mut self) {
        assert_eq!(self.request("shutdown", Value::Null), Value::Null);
        self.notify("exit", Value::Null);
        assert_eq!(
            self.messages.recv_timeout(Duration::from_secs(10)),
            Err(RecvTimeoutError::Disconnected)
        );
        assert!(self.child.wait().unwrap().success());
    }
}

impl Drop for LanguageServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn assert_diagnostic_triggers(
    server: &mut LanguageServer,
    root: &Path,
    on_open: bool,
    on_save: bool,
    on_change: bool,
) {
    let uri = Url::from_file_path(root.join("Main.purs")).unwrap();
    let text = "module Main where\nvalue :: Int\nvalue = \"invalid\"\n";
    server.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {"uri": uri, "languageId": "purescript", "version": 1, "text": text}
        }),
    );
    server.assert_diagnostics(&uri, 1, on_open);
    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": format!("{text}\n")}]
        }),
    );
    server.assert_diagnostics(&uri, 2, on_change);
    server.notify("textDocument/didSave", json!({"textDocument": {"uri": uri}}));
    server.assert_diagnostics(&uri, 2, on_save);
}

#[test]
fn empty_configuration_preserves_spago_and_default_diagnostics() {
    let workspace = TestWorkspace::empty();
    workspace.write(
        "spago.lock",
        r#"{"workspace":{"packages":{"application":{"path":"."}}},"packages":{}}"#,
    );
    workspace.write("src/Library.purs", "module Library where\nfromSpago = 42\n");
    workspace.write("config/empty.json", "{}");

    let cases: &[&[&str]] = &[
        &["lsp"],
        &["lsp", "--config", "null"],
        &["lsp", "--config-file", "config/empty.json"],
        &[
            "lsp",
            "--config",
            r#"{"sources":{"kind":"spago"},"diagnostics":{"onOpen":null,"onSave":null,"onChange":null}}"#,
        ],
    ];
    for arguments in cases {
        let mut server = LanguageServer::start(&workspace, "", arguments, workspace.path());
        let symbols = server.request("workspace/symbol", json!({"query": "fromSpago"}));
        assert_eq!(symbols.as_array().unwrap().len(), 1, "{symbols}");
        assert_eq!(symbols[0]["name"], "fromSpago");
        assert_diagnostic_triggers(&mut server, workspace.path(), true, true, false);
        server.shutdown();
    }
}

#[test]
fn json_inputs_configure_source_commands_and_diagnostic_triggers() {
    let workspace = TestWorkspace::empty();
    workspace.write("project/selected/Library.purs", "module Library where\nfromCommand = 42\n");
    workspace.write(
        "launcher/source command.mjs",
        r#"
import { writeFileSync } from "node:fs";
writeFileSync("arguments.json", JSON.stringify(process.argv.slice(2)));
console.log("selected/*.purs");
"#,
    );
    let configuration = json!({
        "sources": {
            "kind": "command",
            "program": "node",
            "arguments": ["source command.mjs", "", "path with spaces", "--flag", "λ", "$(not-a-shell)"]
        },
        "diagnostics": {"onOpen": false, "onSave": false, "onChange": true}
    }).to_string();
    let absolute_path = workspace.path().join("settings/server config.json");
    let root = workspace.path().join("project");
    let cases: &[&[&str]] = &[
        &["lsp", "--config", &configuration],
        &["lsp", "--config-file", "../settings/server config.json"],
        &["lsp", "--config-file", absolute_path.to_str().unwrap()],
    ];
    for arguments in cases {
        workspace.write("settings/server config.json", &configuration);
        let mut server = LanguageServer::start(&workspace, "launcher", arguments, &root);
        workspace.write("settings/server config.json", "invalid after startup");
        let symbols = server.request("workspace/symbol", json!({"query": "fromCommand"}));
        assert_eq!(symbols.as_array().unwrap().len(), 1, "{symbols}");
        assert_eq!(symbols[0]["name"], "fromCommand");
        let expected_uri = Url::from_file_path(root.join("selected/Library.purs")).unwrap();
        assert_eq!(symbols[0]["location"]["uri"], expected_uri.as_str());
        let arguments: Value =
            serde_json::from_str(&workspace.read("launcher/arguments.json")).unwrap();
        assert_eq!(arguments, json!(["", "path with spaces", "--flag", "λ", "$(not-a-shell)"]));
        assert_diagnostic_triggers(&mut server, &root, false, false, true);
        server.shutdown();
    }
}

#[test]
fn partial_diagnostic_configuration_preserves_omitted_triggers() {
    let workspace = TestWorkspace::empty();
    workspace.write("spago.lock", r#"{"workspace":{"packages":{}},"packages":{}}"#);
    let mut server = LanguageServer::start(
        &workspace,
        "",
        &["lsp", "--config", r#"{"diagnostics":{"onOpen":false}}"#],
        workspace.path(),
    );
    assert_diagnostic_triggers(&mut server, workspace.path(), false, true, false);
    server.shutdown();
}
