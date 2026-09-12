use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::Child;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use url::Url;

use super::support::TestWorkspace;

struct LanguageServer {
    child: Child,
    messages: Receiver<Value>,
    notifications: Vec<Value>,
    next_request: u32,
    configuration: Option<Value>,
    configuration_requests: usize,
    registrations: Vec<Value>,
}

impl LanguageServer {
    fn start(
        workspace: &TestWorkspace,
        directory: &str,
        arguments: &[&str],
        root: &Path,
    ) -> LanguageServer {
        LanguageServer::start_with_capabilities(
            workspace,
            directory,
            arguments,
            root,
            json!({}),
            None,
        )
    }

    fn start_with_capabilities(
        workspace: &TestWorkspace,
        directory: &str,
        arguments: &[&str],
        root: &Path,
        capabilities: Value,
        configuration: Option<Value>,
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
        let mut server = LanguageServer {
            child,
            messages,
            notifications: vec![],
            next_request: 1,
            configuration,
            configuration_requests: 0,
            registrations: vec![],
        };
        let result = server.request("initialize", json!({
            "processId": null,
            "capabilities": capabilities,
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

    #[track_caller]
    fn request(&mut self, method: &str, parameters: Value) -> Value {
        for _ in 0..20 {
            let request = self.next_request;
            self.next_request += 1;
            let deadline = Instant::now() + Duration::from_secs(10);
            let waiting_for = format!("response to {method} request");
            self.send(json!({
                "jsonrpc": "2.0",
                "id": request,
                "method": method,
                "params": parameters
            }));
            loop {
                let message = self.receive_before(deadline, &waiting_for);
                if message.get("method").is_some() {
                    self.handle_server_message(message);
                    continue;
                }
                if message.get("id") == Some(&json!(request)) {
                    if message["error"]["code"] == -32800 {
                        break;
                    }
                    assert!(message.get("error").is_none(), "{message}");
                    return message["result"].clone();
                }
                panic!("unexpected response while waiting for {waiting_for}: {message}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("request {method} was repeatedly cancelled");
    }

    fn handle_server_message(&mut self, message: Value) {
        let Some(request) = message.get("id").cloned() else {
            self.notifications.push(message);
            return;
        };
        match message["method"].as_str().unwrap() {
            "workspace/configuration" => {
                let items = message["params"]["items"].as_array().unwrap();
                assert_eq!(items.len(), 1, "{message}");
                assert_eq!(items[0]["section"], "iris.server");
                assert!(items[0]["scopeUri"].is_string(), "{message}");
                self.configuration_requests += 1;
                let configuration = self.configuration.clone().unwrap_or(Value::Null);
                self.send(json!({"jsonrpc": "2.0", "id": request, "result": [configuration]}));
            }
            "client/registerCapability" => {
                self.registrations.push(message["params"].clone());
                self.send(json!({"jsonrpc": "2.0", "id": request, "result": null}));
            }
            method => panic!("unexpected server request {method}: {message}"),
        }
    }

    fn set_configuration(&mut self, configuration: Value) {
        self.configuration = Some(configuration);
        self.notify("workspace/didChangeConfiguration", json!({"settings": null}));
    }

    fn wait_for_symbol(&mut self, name: &str, present: bool) {
        for _ in 0..20 {
            let symbols = self.request("workspace/symbol", json!({"query": name}));
            let found = symbols.as_array().unwrap().iter().any(|symbol| symbol["name"] == name);
            if found == present {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("symbol {name:?} presence did not become {present}");
    }

    #[track_caller]
    fn wait_for_notification(&mut self, method: &str) -> Value {
        self.wait_for_notification_matching(method, |_| true)
    }

    #[track_caller]
    fn wait_for_notification_matching(
        &mut self,
        method: &str,
        predicate: impl Fn(&Value) -> bool,
    ) -> Value {
        let deadline = Instant::now() + Duration::from_secs(10);
        let waiting_for = format!("notification {method}");
        loop {
            if let Some(index) = self.notifications.iter().position(|notification| {
                notification["method"] == method && predicate(notification)
            }) {
                return self.notifications.remove(index);
            }
            let message = self.receive_before(deadline, &waiting_for);
            if message.get("method").is_some() {
                self.handle_server_message(message);
            } else {
                panic!("unexpected response while waiting for notification {method}: {message}")
            }
        }
    }

    #[track_caller]
    fn receive_before(&self, deadline: Instant, waiting_for: &str) -> Value {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_else(|| panic!("timed out waiting for {waiting_for}"));
        match self.messages.recv_timeout(remaining) {
            Ok(message) => message,
            Err(RecvTimeoutError::Timeout) => panic!("timed out waiting for {waiting_for}"),
            Err(RecvTimeoutError::Disconnected) => {
                panic!("language server disconnected while waiting for {waiting_for}")
            }
        }
    }

    fn assert_diagnostics(&mut self, uri: &Url, version: i32, enabled: bool) {
        // A response proves the preceding document notification reached the server before
        // the bounded wait for asynchronous diagnostics, including when none are expected.
        self.request("workspace/symbol", json!({"query": "noSuchSymbol"}));
        if enabled {
            let message = self.wait_for_notification_matching(
                "textDocument/publishDiagnostics",
                |notification| notification["params"]["uri"] == uri.as_str(),
            );
            assert_eq!(message["params"]["uri"], uri.as_str());
            assert_eq!(message["params"]["version"], version);
            let diagnostics = message["params"]["diagnostics"].as_array().unwrap();
            assert!(diagnostics.iter().any(|diagnostic| diagnostic["severity"] == 1), "{message}");
        } else {
            assert!(!self.notifications.iter().any(|notification| {
                notification["method"] == "textDocument/publishDiagnostics"
                    && notification["params"]["uri"] == uri.as_str()
            }));
            match self.messages.recv_timeout(Duration::from_millis(500)) {
                Ok(message) if message.get("method").is_some() => {
                    assert_ne!(message["params"]["uri"], uri.as_str(), "{message}");
                    self.handle_server_message(message);
                }
                Ok(message) => {
                    panic!("unexpected response while checking diagnostics are disabled: {message}")
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(error) => panic!("language server disconnected: {error}"),
            }
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
    assert_diagnostic_triggers_for(server, root, "Main.purs", on_open, on_save, on_change);
}

fn assert_diagnostic_triggers_for(
    server: &mut LanguageServer,
    root: &Path,
    file: &str,
    on_open: bool,
    on_save: bool,
    on_change: bool,
) {
    let uri = Url::from_file_path(root.join(file)).unwrap();
    let module = file.strip_suffix(".purs").unwrap();
    let text = format!("module {module} where\nvalue :: Int\nvalue = \"invalid\"\n");
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

#[test]
fn workspace_configuration_applies_initial_and_runtime_snapshots() {
    let workspace = TestWorkspace::empty();
    workspace.write("project/startup/Library.purs", "module Library where\nfromStartup = 1\n");
    workspace.write("project/runtime/Library.purs", "module Library where\nfromRuntime = 2\n");
    workspace.write("launcher/startup.mjs", "console.log('../project/startup/*.purs');\n");
    workspace.write("launcher/runtime.mjs", "console.log('../project/runtime/*.purs');\n");
    let startup = r#"{"sources":{"kind":"command","program":"node","arguments":["startup.mjs"]}}"#;
    let runtime = json!({
        "sources": {"kind": "command", "program": "node", "arguments": ["runtime.mjs"]},
        "diagnostics": {"onOpen": false, "onSave": false, "onChange": true}
    });
    let root = workspace.path().join("project");
    let mut server = LanguageServer::start_with_capabilities(
        &workspace,
        "launcher",
        &["lsp", "--config", startup],
        &root,
        json!({"workspace": {"configuration": true}}),
        Some(runtime),
    );

    server.wait_for_symbol("fromRuntime", true);
    server.wait_for_symbol("fromStartup", false);
    assert_diagnostic_triggers(&mut server, &root, false, false, true);
    let runtime_uri = Url::from_file_path(root.join("runtime/Library.purs")).unwrap();
    server.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": runtime_uri,
                "languageId": "purescript",
                "version": 1,
                "text": "module Library where\nunsavedRuntime = 3\n"
            }
        }),
    );
    server.wait_for_symbol("unsavedRuntime", true);

    server.set_configuration(json!({"diagnostics": {"onOpen": true}}));
    server.wait_for_symbol("fromStartup", true);
    server.wait_for_symbol("fromRuntime", false);
    server.wait_for_symbol("unsavedRuntime", true);
    server.notify("textDocument/didClose", json!({"textDocument": {"uri": runtime_uri}}));
    server.wait_for_symbol("unsavedRuntime", false);
    assert_diagnostic_triggers_for(&mut server, &root, "AfterUpdate.purs", true, true, false);
    assert!(server.configuration_requests >= 2);
    server.shutdown();
}

#[test]
fn invalid_runtime_configuration_preserves_the_previous_workspace() {
    let workspace = TestWorkspace::empty();
    workspace.write(
        "spago.lock",
        r#"{"workspace":{"packages":{"application":{"path":"."}}},"packages":{}}"#,
    );
    workspace.write("src/Library.purs", "module Library where\nstillLoaded = 42\n");
    workspace.write(
        "slow failure.mjs",
        r#"
setTimeout(() => {
  process.stderr.write("slow failure\n");
  process.exit(1);
}, 1000);
"#,
    );
    let mut server = LanguageServer::start_with_capabilities(
        &workspace,
        "",
        &["lsp"],
        workspace.path(),
        json!({
            "workspace": {
                "configuration": true,
                "didChangeConfiguration": {"dynamicRegistration": true}
            }
        }),
        Some(json!({})),
    );
    server.wait_for_symbol("stillLoaded", true);

    server.set_configuration(json!({"diagnostics": {"onOpen": "invalid"}}));
    let message = server.wait_for_notification("window/showMessage");
    assert_eq!(message["params"]["type"], 1);
    assert!(
        message["params"]["message"]
            .as_str()
            .unwrap()
            .contains("previous Iris settings remain active")
    );
    server.wait_for_symbol("stillLoaded", true);

    server.set_configuration(json!({
        "sources": {
            "kind": "command",
            "program": "node",
            "arguments": ["slow failure.mjs"]
        }
    }));
    let message = server.wait_for_notification("window/showMessage");
    assert!(
        message["params"]["message"].as_str().unwrap().contains("Failed to apply Iris settings")
    );
    server.wait_for_symbol("stillLoaded", true);
    assert!(server.registrations.iter().any(|parameters| {
        parameters["registrations"][0]["method"] == "workspace/didChangeConfiguration"
    }));
    server.shutdown();
}

#[test]
fn clients_without_workspace_configuration_keep_startup_settings() {
    let workspace = TestWorkspace::empty();
    workspace.write(
        "spago.lock",
        r#"{"workspace":{"packages":{"application":{"path":"."}}},"packages":{}}"#,
    );
    workspace.write("src/Library.purs", "module Library where\nstartupOnly = 42\n");
    let mut server = LanguageServer::start(&workspace, "", &["lsp"], workspace.path());
    server.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"sources": {"kind": "command", "program": "missing"}}}),
    );
    server.wait_for_symbol("startupOnly", true);
    assert_eq!(server.configuration_requests, 0);
    server.shutdown();
}
