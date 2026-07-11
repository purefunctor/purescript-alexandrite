pub mod capabilities;
pub mod error;
pub mod event;
pub mod extension;

use std::borrow::BorrowMut;
use std::ops::{ControlFlow, Deref};
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs, mem, process};

use analyzer::completion::SuggestionsCache;
use analyzer::position::PositionEncoding;
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
use itertools::Itertools;
use parking_lot::RwLock;
use tokio::task;
use tower::ServiceBuilder;

use crate::lsp::capabilities::negotiate_position_encoding;
use crate::lsp::error::{AnalyzerResultExt, LspError};
use crate::walk;

#[derive(Debug)]
pub struct LspConfig {
    pub source_command: Option<String>,
    pub diagnostics_on_open: bool,
    pub diagnostics_on_save: bool,
    pub diagnostics_on_change: bool,
}

pub struct State {
    pub config: Arc<LspConfig>,
    pub client: ClientSocket,

    pub engine: QueryEngine,
    pub files: Arc<RwLock<Files>>,

    pub workspace_symbols_cache: Arc<RwLock<WorkspaceSymbolsCache>>,
    pub suggestions_cache: Arc<RwLock<SuggestionsCache>>,

    pub root: Option<PathBuf>,
    pub position_encoding: PositionEncoding,
}

impl State {
    fn new(config: Arc<LspConfig>, client: ClientSocket) -> State {
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
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                        ..CodeActionOptions::default()
                    },
                )),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
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

    let mut parts = command.split(" ");
    let program = parts.next().ok_or(LspError::InvalidSourceCommand)?;

    let mut command = process::Command::new(program);
    command.args(parts);

    let output = command.output()?;
    let output = str::from_utf8(&output.stdout)?;

    let walk::Walk { files, .. } = walk::walk(root, output.lines())?;
    load_files(state, &files)?;

    Ok(())
}

fn initialized_spago(state: &mut State) -> Result<(), LspError> {
    let root = state.root.as_ref().ok_or(LspError::MissingRoot)?;

    tracing::info!("Using 'spago.lock'");

    let packages = spago::source_files_by_package(root).map_err(LspError::SpagoLock)?;
    let files = packages.into_values().flat_map(|package| package.sources);

    let files = files.sorted().collect_vec();
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

fn code_action(
    snapshot: StateSnapshot,
    p: CodeActionParams,
) -> Result<Option<CodeActionResponse>, LspError> {
    let _span = tracing::info_span!("code_action").entered();
    let uri = p.text_document.uri;
    let range = p.range;
    let action_context = p.context;

    let result = snapshot.with_language_context(|context| {
        analyzer::code_action::implementation(context, uri, range, action_context)
    });

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

fn document_highlight(
    snapshot: StateSnapshot,
    p: DocumentHighlightParams,
) -> Result<Option<Vec<DocumentHighlight>>, LspError> {
    let _span = tracing::info_span!("document_highlight").entered();
    let uri = p.text_document_position_params.text_document.uri;
    let position = p.text_document_position_params.position;
    let result = snapshot.with_language_context(|context| {
        analyzer::document_highlight::implementation(context, uri, position)
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

fn document_symbols(
    snapshot: StateSnapshot,
    p: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>, LspError> {
    let _span = tracing::info_span!("document_symbols").entered();
    let uri = p.text_document.uri;
    let result =
        snapshot.with_language_context(|context| analyzer::symbols::document(context, uri));

    result.on_non_fatal(None)
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

    if let Some(name) = parsed.module_name(content) {
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

pub async fn async_start(config: Arc<LspConfig>) {
    let (server, _) = async_lsp::MainLoop::new_server(move |client| {
        let mut router: Router<State, ResponseError> =
            Router::new(State::new(config, client.clone()));

        router
            .request::<extension::CustomInitialize, _>(initialize)
            .request_snapshot::<request::GotoDefinition>(definition)
            .request_snapshot::<request::HoverRequest>(hover)
            .request_snapshot::<request::CodeActionRequest>(code_action)
            .request_snapshot::<request::Completion>(completion)
            .request_snapshot::<request::ResolveCompletionItem>(resolve_completion_item)
            .request_snapshot::<request::References>(references)
            .request_snapshot::<request::DocumentHighlightRequest>(document_highlight)
            .request_snapshot::<request::WorkspaceSymbolRequest>(workspace_symbols)
            .request_snapshot::<request::DocumentSymbolRequest>(document_symbols)
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

#[tokio::main(flavor = "current_thread")]
pub async fn start(config: LspConfig) {
    let config = Arc::new(config);
    async_start(Arc::clone(&config)).await
}
