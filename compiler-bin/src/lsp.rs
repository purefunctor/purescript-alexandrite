pub mod capabilities;
mod command;
pub mod error;
pub mod event;
pub mod extension;
pub mod formatting;

use std::borrow::BorrowMut;
use std::ops::{ControlFlow, Deref};
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs, mem, process};

use analyzer::completion::SuggestionsCache;
use analyzer::position::{self, PositionEncoding};
use analyzer::symbols::WorkspaceSymbolsCache;
use analyzer::{Files, LanguageContext, QueryEngine, prim};
use async_lsp::client_monitor::ClientProcessMonitorLayer;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::lsp_types::notification::Notification;
use async_lsp::lsp_types::request::Request;
use async_lsp::lsp_types::*;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::{ClientSocket, ResponseError};
use globset::{Glob, GlobSetBuilder};
use parking_lot::RwLock;
use path_absolutize::Absolutize;
use rowan::TextSize;
use tokio::task;
use tower::ServiceBuilder;
use walkdir::WalkDir;

use crate::cli;
use crate::lsp::capabilities::negotiate_position_encoding;
use crate::lsp::error::{AnalyzerResultExt, LspError};

pub struct State {
    pub config: Arc<cli::Config>,
    pub client: ClientSocket,

    pub engine: QueryEngine,
    pub files: Arc<RwLock<Files>>,

    pub workspace_symbols_cache: Arc<RwLock<WorkspaceSymbolsCache>>,
    pub suggestions_cache: Arc<RwLock<SuggestionsCache>>,

    pub root: Option<PathBuf>,
    pub position_encoding: PositionEncoding,
}

impl State {
    fn new(config: Arc<cli::Config>, client: ClientSocket) -> State {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let files = Arc::new(RwLock::new(files));

        let workspace_symbols_cache = WorkspaceSymbolsCache::default();
        let workspace_symbols_cache = Arc::new(RwLock::new(workspace_symbols_cache));

        let suggestions_cache = SuggestionsCache::default();
        let suggestions_cache = Arc::new(RwLock::new(suggestions_cache));

        let root = None;
        let position_encoding = PositionEncoding::Utf16;

        State {
            config,
            client,
            engine,
            files,
            workspace_symbols_cache,
            suggestions_cache,
            root,
            position_encoding,
        }
    }

    fn spawn<T>(&self, f: impl FnOnce(StateSnapshot) -> T + Send + 'static) -> task::JoinHandle<T>
    where
        T: Send + 'static,
    {
        let snapshot = StateSnapshot {
            client: self.client.clone(),
            config: Arc::clone(&self.config),
            engine: self.engine.snapshot(),
            files: Arc::clone(&self.files),
            workspace_symbols_cache: Arc::clone(&self.workspace_symbols_cache),
            suggestions_cache: Arc::clone(&self.suggestions_cache),
            position_encoding: self.position_encoding,
        };
        task::spawn_blocking(move || f(snapshot))
    }

    fn invalidate_workspace_symbols(&self) {
        let mut cache = self.workspace_symbols_cache.write();
        mem::take(&mut *cache);
    }

    fn invalidate_suggestions_cache(&self) {
        let mut cache = self.suggestions_cache.write();
        mem::take(&mut *cache);
    }
}

struct StateSnapshot {
    client: ClientSocket,
    config: Arc<cli::Config>,
    engine: QueryEngine,
    files: Arc<RwLock<Files>>,
    workspace_symbols_cache: Arc<RwLock<WorkspaceSymbolsCache>>,
    suggestions_cache: Arc<RwLock<SuggestionsCache>>,
    position_encoding: PositionEncoding,
}

impl StateSnapshot {
    fn files(&self) -> impl Deref<Target = Files> {
        self.files.read()
    }

    fn with_language_context<T>(&self, f: impl FnOnce(&LanguageContext) -> T) -> T {
        let files = self.files();
        let context = LanguageContext::new(&self.engine, &files, self.position_encoding);
        f(&context)
    }
}

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn initialize(
    state: &mut State,
    p: extension::CustomInitializeParams,
) -> impl Future<Output = Result<InitializeResult, ResponseError>> + use<> {
    let position_encoding = negotiate_position_encoding(&p.initialize_params);
    state.position_encoding = position_encoding;

    state.root = p
        .initialize_params
        .workspace_folders
        .and_then(|folders| {
            let folder = folders.first()?;
            folder.uri.to_file_path().ok()
        })
        .or_else(|| env::current_dir().ok());
    let formatting_enabled =
        state.config.format_command.as_deref().is_some_and(|s| !s.trim().is_empty());
    async move {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: PACKAGE_NAME.to_string(),
                version: Some(PACKAGE_VERSION.to_string()),
            }),
            capabilities: ServerCapabilities {
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec![".".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                    completion_item: Some(CompletionOptionsCompletionItem {
                        label_details_support: Some(true),
                    }),
                }),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                document_formatting_provider: formatting_enabled.then_some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                position_encoding: Some(PositionEncodingKind::from(position_encoding)),
                ..ServerCapabilities::default()
            },
        })
    }
}

fn initialized(state: &mut State, _: InitializedParams) -> Result<(), LspError> {
    let _span = tracing::info_span!("initialization").entered();
    let config = Arc::clone(&state.config);
    if let Some(command) = config.source_command.as_deref() {
        initialized_manual(state, command)
    } else {
        initialized_spago(state)
    }
}

fn initialized_manual(state: &mut State, command: &str) -> Result<(), LspError> {
    let root = state.root.as_ref().ok_or(LspError::MissingRoot)?;

    tracing::info!("Using '{}'", command);

    let mut parts = command::split(command).ok_or(LspError::InvalidSourceCommand)?.into_iter();
    let program = parts.next().ok_or(LspError::InvalidSourceCommand)?;

    let mut command = process::Command::new(program);
    command.args(parts);

    let output = command.output()?;
    let output = str::from_utf8(&output.stdout)?;

    let mut files = vec![];
    let mut globs = GlobSetBuilder::new();

    for line in output.lines() {
        let path = root.join(line);
        if let Ok(path) = path.absolutize()
            && let Some(path) = path.to_str()
            && let Ok(glob) = Glob::new(path)
        {
            globs.add(glob);
        } else {
            files.push(path);
        }
    }

    let globs = globs.build()?;

    tracing::info!("Found {} file literals", files.len());
    tracing::info!("Found {} glob patterns", globs.len());

    let files_from_glob = WalkDir::new(root).into_iter().filter_map(move |entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if globs.matches(path).is_empty() { None } else { Some(path.to_path_buf()) }
    });

    files.extend(files_from_glob);
    load_files(state, &files)?;

    Ok(())
}

fn initialized_spago(state: &mut State) -> Result<(), LspError> {
    let root = state.root.as_ref().ok_or(LspError::MissingRoot)?;

    tracing::info!("Using 'spago.lock'");

    let files = spago::source_files(root).map_err(LspError::SpagoLock)?;
    load_files(state, &files)?;

    Ok(())
}

fn load_files(state: &mut State, files: &[PathBuf]) -> Result<(), LspError> {
    tracing::info!("Loading {} files.", files.len());

    for file in files {
        let url = url::Url::from_file_path(file).map_err(|_| {
            let file = PathBuf::clone(file);
            LspError::PathParseFail(file)
        })?;

        let uri = url.to_string();

        let text = fs::read_to_string(file)?;
        on_change(state, &uri, &text)?
    }

    tracing::info!("Loaded {} files.", files.len());

    Ok(())
}

fn definition(
    snapshot: StateSnapshot,
    p: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>, LspError> {
    let _span = tracing::info_span!("definition").entered();
    let uri = p.text_document_position_params.text_document.uri;
    let position = p.text_document_position_params.position;

    let result = snapshot.with_language_context(|context| {
        analyzer::definition::implementation(context, uri, position)
    });

    result.on_non_fatal(None)
}

fn hover(snapshot: StateSnapshot, p: HoverParams) -> Result<Option<Hover>, LspError> {
    let _span = tracing::info_span!("hover").entered();
    let uri = p.text_document_position_params.text_document.uri;
    let position = p.text_document_position_params.position;

    let result = snapshot
        .with_language_context(|context| analyzer::hover::implementation(context, uri, position));

    result.on_non_fatal(None)
}

fn completion(
    snapshot: StateSnapshot,
    p: CompletionParams,
) -> Result<Option<CompletionResponse>, LspError> {
    let _span = tracing::info_span!("completion").entered();
    let uri = p.text_document_position.text_document.uri;
    let position = p.text_document_position.position;

    let mut cache = snapshot.suggestions_cache.write();

    let result = snapshot.with_language_context(|context| {
        analyzer::completion::implementation(context, &mut cache, uri, position)
    });

    result.on_non_fatal(None)
}

fn resolve_completion_item(
    snapshot: StateSnapshot,
    item: CompletionItem,
) -> Result<CompletionItem, LspError> {
    let _span = tracing::info_span!("resolve_completion_item").entered();
    analyzer::completion::resolve::implementation(&snapshot.engine, item)
        .or_else(|(error, item)| Err(error).on_non_fatal(item))
}

fn references(
    snapshot: StateSnapshot,
    p: ReferenceParams,
) -> Result<Option<Vec<Location>>, LspError> {
    let _span = tracing::info_span!("references").entered();
    let uri = p.text_document_position.text_document.uri;
    let position = p.text_document_position.position;

    let result = snapshot.with_language_context(|context| {
        analyzer::references::implementation(context, uri, position)
    });

    result.on_non_fatal(None)
}

fn workspace_symbols(
    snapshot: StateSnapshot,
    p: WorkspaceSymbolParams,
) -> Result<Option<WorkspaceSymbolResponse>, LspError> {
    let _span = tracing::info_span!("workspace_symbols").entered();

    let mut cache = snapshot.workspace_symbols_cache.write();

    let result = snapshot.with_language_context(|context| {
        analyzer::symbols::workspace(context, &mut cache, &p.query)
    });

    result.on_non_fatal(None)
}

fn formatting(
    snapshot: StateSnapshot,
    p: DocumentFormattingParams,
) -> Result<Option<Vec<TextEdit>>, LspError> {
    let _span = tracing::info_span!("formatting").entered();

    // Formatting support is advertised conditionally in `initialize`.
    let Some(format_command) = snapshot.config.format_command.as_deref() else {
        return Ok(None);
    };
    if format_command.trim().is_empty() {
        return Ok(None);
    }

    let uri = p.text_document.uri;
    let current_file = {
        let files = snapshot.files();
        let uri = uri.as_str();
        let Some(id) = files.id(uri) else { return Ok(None) };
        id
    };

    let input = snapshot.engine.content(current_file);

    let output = formatting::run(format_command, input.as_bytes())
        .map_err(|e| LspError::FormattingFailed(e.to_string()))?;

    let formatted = String::from_utf8(output)
        .map_err(|e| LspError::FormattingFailed(format!("formatter output was not utf-8: {e}")))?;

    if formatted.as_str() == input.as_ref() {
        return Ok(Some(vec![]));
    }

    let end = position::offset_to_utf8_position(input.as_ref(), TextSize::from(input.len() as u32))
        .and_then(|position| {
            position::utf8_position_to_protocol(
                input.as_ref(),
                position,
                snapshot.position_encoding,
            )
        })
        .unwrap_or(Position { line: 0, character: 0 });
    let range = Range { start: Position { line: 0, character: 0 }, end };

    Ok(Some(vec![TextEdit { range, new_text: formatted }]))
}

fn did_change(state: &mut State, p: DidChangeTextDocumentParams) -> Result<(), LspError> {
    let uri = p.text_document.uri.as_str();

    for content_change in &p.content_changes {
        let text = content_change.text.as_str();
        on_change(state, uri, text)?
    }

    state.invalidate_workspace_symbols();
    state.invalidate_suggestions_cache();

    if state.config.diagnostics_on_change {
        event::emit_collect_diagnostics(state, p.text_document.uri)?;
    }

    Ok(())
}

fn did_open(state: &mut State, p: DidOpenTextDocumentParams) -> Result<(), LspError> {
    let uri = p.text_document.uri.as_str();
    let text = p.text_document.text.as_str();

    on_change(state, uri, text)?;

    state.invalidate_workspace_symbols();
    state.invalidate_suggestions_cache();

    if state.config.diagnostics_on_open {
        event::emit_collect_diagnostics(state, p.text_document.uri)?;
    }

    Ok(())
}

fn did_save(state: &mut State, p: DidSaveTextDocumentParams) -> Result<(), LspError> {
    state.invalidate_suggestions_cache();

    if state.config.diagnostics_on_save {
        event::emit_collect_diagnostics(state, p.text_document.uri)?;
    }
    Ok(())
}

fn on_change(state: &mut State, uri: &str, content: &str) -> Result<(), LspError> {
    // Cancel in-flight queries so that threads holding a read lock
    // over `files` are terminated quickly, compared to having to
    // wait for expensive LSP requests to complete successfully.
    state.engine.request_cancel();

    let mut files = state.files.write();
    let id = files.insert(uri, content);

    state.engine.set_content(id, content);

    let (parsed, _) = state.engine.parsed(id)?;

    if let Some(name) = parsed.module_name() {
        state.engine.set_module_file(&name, id);
    }

    Ok(())
}

trait RequestExtension: BorrowMut<Router<State>> {
    fn request_snapshot<R: Request>(
        &mut self,
        action: impl Fn(StateSnapshot, R::Params) -> Result<R::Result, LspError> + Send + Copy + 'static,
    ) -> &mut Self {
        self.borrow_mut().request::<R, _>(move |state, parameters| {
            let task = state.spawn(move |snapshot| action(snapshot, parameters));
            async move {
                task.await.map_err(LspError::JoinError).flatten().map_err(|error| {
                    error.emit_trace();
                    let code = error.code();
                    let message = error.message();
                    ResponseError::new(code, message)
                })
            }
        });
        self
    }

    fn notification_ext<N: Notification>(
        &mut self,
        action: impl Fn(&mut State, N::Params) -> Result<(), LspError> + Send + Copy + 'static,
    ) -> &mut Self {
        let this: &mut Router<State> = self.borrow_mut();
        this.notification::<N>(move |state, parameters| {
            let _ = action(state, parameters).inspect_err(|error| error.emit_trace());
            ControlFlow::Continue(())
        });
        self
    }
    fn event_ext<E>(
        &mut self,
        action: impl Fn(&mut State, E) -> Result<(), LspError> + Send + Copy + 'static,
    ) -> &mut Self
    where
        E: Send + 'static,
    {
        let this: &mut Router<State> = self.borrow_mut();
        this.event::<E>(move |state, event| {
            let _ = action(state, event).inspect_err(|error| error.emit_trace());
            ControlFlow::Continue(())
        });
        self
    }
}

impl RequestExtension for Router<State> {}

pub async fn start(config: Arc<cli::Config>) {
    let (server, _) = async_lsp::MainLoop::new_server(move |client| {
        let mut router: Router<State, ResponseError> =
            Router::new(State::new(config, client.clone()));

        router
            .request::<extension::CustomInitialize, _>(initialize)
            .request_snapshot::<request::GotoDefinition>(definition)
            .request_snapshot::<request::HoverRequest>(hover)
            .request_snapshot::<request::Completion>(completion)
            .request_snapshot::<request::ResolveCompletionItem>(resolve_completion_item)
            .request_snapshot::<request::References>(references)
            .request_snapshot::<request::WorkspaceSymbolRequest>(workspace_symbols)
            .request_snapshot::<request::Formatting>(formatting)
            .notification_ext::<notification::Initialized>(initialized)
            .notification_ext::<notification::DidOpenTextDocument>(did_open)
            .notification_ext::<notification::DidSaveTextDocument>(did_save)
            .notification_ext::<notification::DidCloseTextDocument>(|_, _| Ok(()))
            .notification_ext::<notification::DidChangeConfiguration>(|_, _| Ok(()))
            .notification_ext::<notification::DidChangeTextDocument>(did_change)
            .notification_ext::<notification::DidChangeWatchedFiles>(|_, _| Ok(()))
            .event_ext::<event::CollectDiagnostics>(event::collect_diagnostics);

        ServiceBuilder::new()
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .layer(ClientProcessMonitorLayer::new(client))
            .service(router)
    });

    #[cfg(unix)]
    let (stdin, stdout) = (
        async_lsp::stdio::PipeStdin::lock_tokio().unwrap(),
        async_lsp::stdio::PipeStdout::lock_tokio().unwrap(),
    );

    #[cfg(not(unix))]
    let (stdin, stdout) = (
        tokio_util::compat::TokioAsyncReadCompatExt::compat(tokio::io::stdin()),
        tokio_util::compat::TokioAsyncWriteCompatExt::compat_write(tokio::io::stdout()),
    );

    if let Err(error) = server.run_buffered(stdin, stdout).await {
        tracing::error!(?error, "LSP main loop exited");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_lsp::lsp_types::{
        ClientCapabilities, DocumentFormattingParams, InitializeParams, Position,
        TextDocumentIdentifier, Url, WorkspaceFolder,
    };

    fn mk_state_with(config: cli::Config) -> State {
        // ClientSocket isn't used by initialize/formatting logic in tests.
        let client = ClientSocket::new_closed();
        State::new(Arc::new(config), client)
    }

    fn mk_init_params(root: &std::path::Path) -> InitializeParams {
        InitializeParams {
            capabilities: ClientCapabilities::default(),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::from_file_path(root).unwrap(),
                name: "workspace".to_string(),
            }]),
            ..InitializeParams::default()
        }
    }

    fn base_config(format_command: Option<String>) -> cli::Config {
        cli::Config {
            stdio: true,
            log_file: false,
            query_log: tracing::level_filters::LevelFilter::OFF,
            lsp_log: tracing::level_filters::LevelFilter::INFO,
            checking_log: tracing::level_filters::LevelFilter::OFF,
            source_command: None,
            format_command,
            diagnostics_on_open: true,
            diagnostics_on_save: true,
            diagnostics_on_change: false,
        }
    }

    fn snapshot(state: &State) -> StateSnapshot {
        StateSnapshot {
            client: state.client.clone(),
            config: Arc::clone(&state.config),
            engine: state.engine.snapshot(),
            files: Arc::clone(&state.files),
            workspace_symbols_cache: Arc::clone(&state.workspace_symbols_cache),
            suggestions_cache: Arc::clone(&state.suggestions_cache),
            position_encoding: state.position_encoding,
        }
    }

    fn formatting_params(uri: Url) -> DocumentFormattingParams {
        DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri },
            options: FormattingOptions::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    #[tokio::test]
    async fn formatting_capability_not_advertised_without_flag() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut state = mk_state_with(base_config(None));

        let initialize_params = mk_init_params(root);
        let res = initialize(
            &mut state,
            extension::CustomInitializeParams { initialize_params, work_done_token: None },
        )
        .await
        .unwrap();

        assert!(res.capabilities.document_formatting_provider.is_none());
    }

    #[tokio::test]
    async fn formatting_capability_advertised_with_flag() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // Capability advertising doesn't execute the formatter; any non-empty value should enable it.
        let mut state = mk_state_with(base_config(Some("purs-tidy".to_string())));

        let initialize_params = mk_init_params(root);
        let res = initialize(
            &mut state,
            extension::CustomInitializeParams { initialize_params, work_done_token: None },
        )
        .await
        .unwrap();

        assert!(res.capabilities.document_formatting_provider.is_some());
    }

    #[tokio::test]
    async fn state_spawn_captures_snapshot_config() {
        let state = mk_state_with(base_config(Some("formatter".to_string())));

        let format_command =
            state.spawn(|snapshot| snapshot.config.format_command.clone()).await.unwrap();

        assert_eq!(format_command.as_deref(), Some("formatter"));
    }

    #[cfg(unix)]
    #[test]
    fn initialized_manual_parses_shell_command() {
        let root =
            std::env::temp_dir().join(format!("alexandrite-lsp-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();

        let mut state = mk_state_with(base_config(None));
        state.root = Some(root.clone());

        initialized_manual(&mut state, "sh -c 'exit 0'").unwrap();

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn formatting_returns_none_without_command() {
        let state = mk_state_with(base_config(None));
        let uri = Url::parse("file:///test/Main.purs").unwrap();

        let edits = formatting(snapshot(&state), formatting_params(uri)).unwrap();

        assert!(edits.is_none());
    }

    #[test]
    fn formatting_returns_none_for_blank_command() {
        let state = mk_state_with(base_config(Some("   ".to_string())));
        let uri = Url::parse("file:///test/Main.purs").unwrap();

        let edits = formatting(snapshot(&state), formatting_params(uri)).unwrap();

        assert!(edits.is_none());
    }

    #[test]
    fn formatting_returns_none_for_unknown_document() {
        let state = mk_state_with(base_config(Some("cat".to_string())));
        let uri = Url::parse("file:///test/Missing.purs").unwrap();

        let edits = formatting(snapshot(&state), formatting_params(uri)).unwrap();

        assert!(edits.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn formatting_returns_empty_edits_for_noop_formatter() {
        let mut state = mk_state_with(base_config(Some("cat".to_string())));
        let uri = Url::parse("file:///test/Main.purs").unwrap();
        on_change(&mut state, uri.as_str(), "module Main where\nfoo = bar\n").unwrap();

        let edits = formatting(snapshot(&state), formatting_params(uri)).unwrap().unwrap();

        assert!(edits.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn formatting_returns_full_document_edit() {
        let mut state = mk_state_with(base_config(Some("tr a-z A-Z".to_string())));
        let uri = Url::parse("file:///test/Main.purs").unwrap();
        on_change(&mut state, uri.as_str(), "module Main where\nfoo = bar\n").unwrap();

        let edits = formatting(snapshot(&state), formatting_params(uri)).unwrap().unwrap();

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "MODULE MAIN WHERE\nFOO = BAR\n");
        assert_eq!(edits[0].range.start, Position::new(0, 0));
    }

    #[cfg(unix)]
    #[test]
    fn formatting_reports_non_utf8_output() {
        let mut state = mk_state_with(base_config(Some("perl -e 'print chr 255'".to_string())));
        let uri = Url::parse("file:///test/Main.purs").unwrap();
        on_change(&mut state, uri.as_str(), "module Main where\nfoo = bar\n").unwrap();

        let error = formatting(snapshot(&state), formatting_params(uri)).unwrap_err();

        assert!(matches!(error, LspError::FormattingFailed(_)));
        assert!(error.message().starts_with("formatter output was not utf-8:"));
    }

    #[cfg(unix)]
    #[test]
    fn formatting_reports_formatter_failure() {
        let mut state = mk_state_with(base_config(Some(
            "sh -c 'cat >/dev/null; printf err >&2; exit 7'".to_string(),
        )));
        let uri = Url::parse("file:///test/Main.purs").unwrap();
        on_change(&mut state, uri.as_str(), "module Main where\nfoo = bar\n").unwrap();

        let error = formatting(snapshot(&state), formatting_params(uri)).unwrap_err();

        assert!(matches!(error, LspError::FormattingFailed(_)));
        assert_eq!(error.message(), "formatter exited with exit status: 7: err");
    }
}
