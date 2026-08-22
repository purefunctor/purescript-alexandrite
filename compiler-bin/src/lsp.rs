pub mod capabilities;
pub mod error;
pub mod event;
pub mod extension;

use std::borrow::BorrowMut;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::{env, fs, mem, process};

use analyzer::completion::SuggestionsCache;
use analyzer::position::PositionEncoding;
use analyzer::symbols::WorkspaceSymbolsCache;
use analyzer::{AnalyzerCapabilities, AnalyzerContext, AnalyzerHost};
use async_lsp::client_monitor::ClientProcessMonitorLayer;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::{ClientSocket, LanguageClient, ResponseError};
use building::QueryEngine;
use files::{FileId, Files, ForeignFiles};
use itertools::Itertools;
use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::*;
use parking_lot::{RwLock, RwLockReadGuard};
use prim_constants::MODULE_MAP;
use rustc_hash::FxHashSet;
use tempfile::TempDir;
use tokio::task;
use tower::ServiceBuilder;

use crate::lsp::capabilities::{negotiate_analyzer_capabilities, negotiate_position_encoding};
use crate::lsp::error::{AnalyzerResultExt, LspError};
use crate::walk;

static PRIM_DIRECTORY: LazyLock<TempDir> =
    LazyLock::new(|| TempDir::new().expect("invariant violated: failed to create PRIM_DIRECTORY"));

fn configure_materialized_prim(engine: &QueryEngine, files: &mut Files) {
    for (name, content) in MODULE_MAP {
        let path = PRIM_DIRECTORY.path().join(format!("{name}.purs"));
        fs::write(&path, content).expect("invariant violated: failed to materialize Prim module");

        let uri = Url::from_file_path(path)
            .expect("invariant violated: failed to create Prim module file URL");
        let id = files.insert(uri.as_str(), *content);

        engine.set_content(id, *content);
        engine.set_module_file(name, id);
    }
}

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
    pub files: Arc<RwLock<LspFiles>>,
    pub foreign_files: Arc<RwLock<ForeignFiles>>,

    pub workspace_symbols_cache: Arc<RwLock<WorkspaceSymbolsCache>>,
    pub suggestions_cache: Arc<RwLock<SuggestionsCache>>,

    pub root: Option<PathBuf>,
    pub position_encoding: PositionEncoding,
    pub analyzer_capabilities: AnalyzerCapabilities,
    pub watched_files_dynamic_registration: bool,
}

impl State {
    fn new(config: Arc<LspConfig>, client: ClientSocket) -> State {
        let engine = QueryEngine::default();
        let mut files = Files::default();
        configure_materialized_prim(&engine, &mut files);

        let files = LspFiles::new(files);
        let files = Arc::new(RwLock::new(files));

        let foreign_files = ForeignFiles::default();
        let foreign_files = Arc::new(RwLock::new(foreign_files));

        let workspace_symbols_cache = WorkspaceSymbolsCache::default();
        let workspace_symbols_cache = Arc::new(RwLock::new(workspace_symbols_cache));

        let suggestions_cache = SuggestionsCache::default();
        let suggestions_cache = Arc::new(RwLock::new(suggestions_cache));

        let root = None;
        let position_encoding = PositionEncoding::Utf16;
        let analyzer_capabilities = AnalyzerCapabilities::default();
        let watched_files_dynamic_registration = false;

        State {
            config,
            client,
            engine,
            files,
            foreign_files,
            workspace_symbols_cache,
            suggestions_cache,
            root,
            position_encoding,
            analyzer_capabilities,
            watched_files_dynamic_registration,
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
            analyzer_capabilities: self.analyzer_capabilities,
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
    files: Arc<RwLock<LspFiles>>,
    workspace_symbols_cache: Arc<RwLock<WorkspaceSymbolsCache>>,
    suggestions_cache: Arc<RwLock<SuggestionsCache>>,
    position_encoding: PositionEncoding,
    analyzer_capabilities: AnalyzerCapabilities,
}

impl StateSnapshot {
    fn with_analyzer_context<T>(
        &self,
        f: impl FnOnce(&AnalyzerContext<LspAnalyzerHost<'_>>) -> T,
    ) -> T {
        let files = self.files.read();
        let host = LspAnalyzerHost { queries: &self.engine, files };
        let context =
            AnalyzerContext::new(&host, self.position_encoding, self.analyzer_capabilities);
        f(&context)
    }
}

pub struct LspFiles {
    files: Files,
    editable: FxHashSet<FileId>,
    open: FxHashSet<FileId>,
}

impl LspFiles {
    fn new(files: Files) -> LspFiles {
        LspFiles { files, editable: FxHashSet::default(), open: FxHashSet::default() }
    }

    fn id(&self, uri: &str) -> Option<FileId> {
        self.files.id(uri)
    }

    fn contains(&self, file_id: FileId) -> bool {
        self.files.contains(file_id)
    }

    fn path(&self, file_id: FileId) -> Option<Arc<str>> {
        self.files.contains(file_id).then(|| self.files.path(file_id))
    }

    fn iter_id(&self) -> impl Iterator<Item = FileId> + '_ {
        self.files.iter_id()
    }

    fn is_editable(&self, file_id: FileId) -> bool {
        self.editable.contains(&file_id)
    }

    fn is_open(&self, uri: &str) -> bool {
        self.id(uri).is_some_and(|file_id| self.open.contains(&file_id))
    }

    fn insert(&mut self, uri: &str, content: &str, editable: Option<bool>) -> FileId {
        let file_id = self.files.insert(uri, content);
        if let Some(editable) = editable {
            if editable {
                self.editable.insert(file_id);
            } else {
                self.editable.remove(&file_id);
            }
        }
        file_id
    }

    fn open(&mut self, file_id: FileId) {
        self.open.insert(file_id);
    }

    fn close(&mut self, file_id: FileId) {
        self.open.remove(&file_id);
    }

    fn remove(&mut self, uri: &str) -> Option<FileId> {
        let file_id = self.files.remove(uri)?;
        self.editable.remove(&file_id);
        self.open.remove(&file_id);
        Some(file_id)
    }
}

struct LspAnalyzerHost<'a> {
    queries: &'a QueryEngine,
    files: RwLockReadGuard<'a, LspFiles>,
}

impl AnalyzerHost for LspAnalyzerHost<'_> {
    type Queries = QueryEngine;

    fn queries(&self) -> &QueryEngine {
        self.queries
    }

    fn file_id(&self, uri: &str) -> Option<FileId> {
        self.files.id(uri)
    }

    fn file_uri(&self, file_id: FileId) -> Result<Option<Url>, url::ParseError> {
        let Some(uri) = self.files.path(file_id) else {
            return Ok(None);
        };
        Url::parse(&uri).map(Some)
    }

    fn active_files(&self) -> impl Iterator<Item = FileId> {
        self.files.iter_id()
    }

    fn is_editable(&self, file_id: FileId) -> bool {
        self.files.is_editable(file_id)
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
    state.analyzer_capabilities = negotiate_analyzer_capabilities(&p.initialize_params);
    state.watched_files_dynamic_registration =
        watched_files_dynamic_registration(&p.initialize_params.capabilities);

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
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                })),
                document_highlight_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions {
                                work_done_progress: None,
                            },
                            legend: SemanticTokensLegend {
                                token_types: analyzer::semantic_tokens::TOKEN_TYPES.to_vec(),
                                token_modifiers: analyzer::semantic_tokens::TOKEN_MODIFIERS
                                    .to_vec(),
                            },
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
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

fn watched_files_dynamic_registration(capabilities: &ClientCapabilities) -> bool {
    capabilities
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.did_change_watched_files.as_ref())
        .and_then(|watched_files| watched_files.dynamic_registration)
        .unwrap_or(false)
}

fn shutdown(_state: &mut State, (): ()) -> impl Future<Output = Result<(), ResponseError>> + use<> {
    async { Ok(()) }
}

fn initialized(state: &mut State, _: InitializedParams) -> Result<(), LspError> {
    let _span = tracing::info_span!("initialization").entered();
    register_file_watcher(state);

    let config = Arc::clone(&state.config);
    if let Some(command) = config.source_command.as_deref() {
        initialized_manual(state, command)
    } else {
        initialized_spago(state)
    }
}

fn register_file_watcher(state: &State) {
    if !state.watched_files_dynamic_registration {
        return;
    }

    let parameters = file_watcher_registration();
    let mut client = state.client.clone();
    task::spawn(async move {
        if let Err(error) = client.register_capability(parameters).await {
            tracing::warn!("Failed to register source file watcher: {error}");
        }
    });
}

fn file_watcher_registration() -> RegistrationParams {
    let options = DidChangeWatchedFilesRegistrationOptions {
        watchers: vec![
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.purs".to_string()),
                kind: None,
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.js".to_string()),
                kind: None,
            },
        ],
    };
    let register_options = serde_json::to_value(options)
        .expect("invariant violated: watched file registration options must serialize");
    let registration = Registration {
        id: "purescript-source-files".to_string(),
        method: notification::DidChangeWatchedFiles::METHOD.to_string(),
        register_options: Some(register_options),
    };
    RegistrationParams { registrations: vec![registration] }
}

fn exit(_state: &mut State, (): ()) -> Result<(), LspError> {
    Ok(())
}

fn initialized_manual(state: &mut State, command: &str) -> Result<(), LspError> {
    let root = state.root.clone().ok_or(LspError::MissingRoot)?;

    tracing::info!("Using '{}'", command);

    let mut parts = command.split(" ");
    let program = parts.next().ok_or(LspError::InvalidSourceCommand)?;

    let mut command = process::Command::new(program);
    command.args(parts);

    let output = command.output()?;
    let output = str::from_utf8(&output.stdout)?;

    let walk::Walk { files, .. } = walk::walk(&root, output.lines())?;
    let files = files.into_iter().map(|file| {
        let editable = file.starts_with(&root);
        (file, editable)
    });
    load_files(state, files)?;

    Ok(())
}

fn initialized_spago(state: &mut State) -> Result<(), LspError> {
    let root = state.root.as_ref().ok_or(LspError::MissingRoot)?;

    tracing::info!("Using 'spago.lock'");

    let packages = spago::source_files_by_package(root).map_err(LspError::SpagoLock)?;
    let files = packages.into_values().flat_map(|package| {
        let editable = package.reference == spago::PackageReference::Workspace;
        package.sources.into_iter().map(move |file| (file, editable))
    });

    let files = files.sorted().collect_vec();
    load_files(state, files)?;

    Ok(())
}

fn load_files(
    state: &mut State,
    files: impl IntoIterator<Item = (PathBuf, bool)>,
) -> Result<(), LspError> {
    let files = files.into_iter().collect_vec();
    tracing::info!("Loading {} files.", files.len());

    for (file, editable) in &files {
        let url = url::Url::from_file_path(file).map_err(|_| {
            let file = PathBuf::clone(file);
            LspError::PathParseFail(file)
        })?;

        let uri = url.to_string();

        let text = fs::read_to_string(file)?;
        on_change(state, &uri, &text, Some(*editable))?;
        load_sibling_foreign(state, &url)?;
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

    let result = snapshot.with_analyzer_context(|context| {
        analyzer::definition::implementation(context, uri, position)
    });

    result.on_non_fatal(None)
}

fn hover(snapshot: StateSnapshot, p: HoverParams) -> Result<Option<Hover>, LspError> {
    let _span = tracing::info_span!("hover").entered();
    let uri = p.text_document_position_params.text_document.uri;
    let position = p.text_document_position_params.position;

    let result = snapshot
        .with_analyzer_context(|context| analyzer::hover::implementation(context, uri, position));

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

    let result = snapshot.with_analyzer_context(|context| {
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

    let result = snapshot.with_analyzer_context(|context| {
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

    let result = snapshot.with_analyzer_context(|context| {
        analyzer::references::implementation(context, uri, position)
    });

    result.on_non_fatal(None)
}

fn rename(snapshot: StateSnapshot, p: RenameParams) -> Result<Option<WorkspaceEdit>, LspError> {
    let _span = tracing::info_span!("rename").entered();
    let uri = p.text_document_position.text_document.uri;
    let position = p.text_document_position.position;
    let new_name = p.new_name;

    let result = snapshot.with_analyzer_context(|context| {
        analyzer::rename::implementation(context, uri, position, new_name)
    });

    result.on_non_fatal(None)
}

fn prepare_rename(
    snapshot: StateSnapshot,
    p: TextDocumentPositionParams,
) -> Result<Option<PrepareRenameResponse>, LspError> {
    let _span = tracing::info_span!("prepare_rename").entered();
    let uri = p.text_document.uri;
    let position = p.position;

    let result =
        snapshot.with_analyzer_context(|context| analyzer::rename::prepare(context, uri, position));

    result.on_non_fatal(None)
}

fn document_highlight(
    snapshot: StateSnapshot,
    p: DocumentHighlightParams,
) -> Result<Option<Vec<DocumentHighlight>>, LspError> {
    let _span = tracing::info_span!("document_highlight").entered();
    let uri = p.text_document_position_params.text_document.uri;
    let position = p.text_document_position_params.position;
    let result = snapshot.with_analyzer_context(|context| {
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

    let result = snapshot.with_analyzer_context(|context| {
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
        snapshot.with_analyzer_context(|context| analyzer::symbols::document(context, uri));

    result.on_non_fatal(None)
}

fn semantic_tokens(
    snapshot: StateSnapshot,
    p: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>, LspError> {
    let _span = tracing::info_span!("semantic_tokens").entered();
    let uri = p.text_document.uri;
    let result = snapshot.with_analyzer_context(|context| {
        analyzer::semantic_tokens::implementation(context, uri)
            .map(|tokens| tokens.map(SemanticTokensResult::Tokens))
    });

    result.on_non_fatal(None)
}

fn did_change(state: &mut State, p: DidChangeTextDocumentParams) -> Result<(), LspError> {
    let uri = &p.text_document.uri;

    for content_change in &p.content_changes {
        let text = content_change.text.as_str();
        if is_javascript_uri(uri) {
            on_foreign_change(state, uri, text)?;
        } else {
            on_change(state, uri.as_str(), text, None)?;
        }
    }

    state.invalidate_workspace_symbols();
    state.invalidate_suggestions_cache();

    if state.config.diagnostics_on_change {
        emit_associated_diagnostics(state, p.text_document.uri)?;
    }

    Ok(())
}

fn did_open(state: &mut State, p: DidOpenTextDocumentParams) -> Result<(), LspError> {
    let uri = &p.text_document.uri;
    let text = p.text_document.text.as_str();

    if is_javascript_uri(uri) {
        on_foreign_change(state, uri, text)?;
        if state.config.diagnostics_on_open {
            emit_associated_diagnostics(state, uri.clone())?;
        }
        return Ok(());
    }

    let editable = {
        let files = state.files.read();
        let previous_id = files.id(uri.as_str());
        previous_id.map(|file_id| files.is_editable(file_id))
    }
    .unwrap_or_else(|| {
        let Some(root) = state.root.as_ref() else {
            return true;
        };
        p.text_document.uri.to_file_path().is_ok_and(|path| path.starts_with(root))
    });
    let file_id = on_change(state, uri.as_str(), text, Some(editable))?;
    state.files.write().open(file_id);
    load_sibling_foreign(state, uri)?;

    state.invalidate_workspace_symbols();
    state.invalidate_suggestions_cache();

    if state.config.diagnostics_on_open {
        event::emit_collect_diagnostics(state, p.text_document.uri)?;
    }

    Ok(())
}

fn did_close(state: &mut State, p: DidCloseTextDocumentParams) -> Result<(), LspError> {
    let uri = p.text_document.uri;
    if is_javascript_uri(&uri) {
        return Ok(());
    }

    let file_id = state.files.read().id(uri.as_str());
    let Some(file_id) = file_id else {
        return Ok(());
    };
    state.files.write().close(file_id);

    let reloaded = reload_source_file(state, &uri, None)?;
    state.invalidate_workspace_symbols();
    state.invalidate_suggestions_cache();
    if reloaded {
        event::emit_collect_all_diagnostics(state)?;
    }
    Ok(())
}

fn did_save(state: &mut State, p: DidSaveTextDocumentParams) -> Result<(), LspError> {
    state.invalidate_suggestions_cache();

    if state.config.diagnostics_on_save {
        emit_associated_diagnostics(state, p.text_document.uri)?;
    }
    Ok(())
}

fn did_change_watched_files(
    state: &mut State,
    p: DidChangeWatchedFilesParams,
) -> Result<(), LspError> {
    let mut source_changed = false;
    for change in p.changes {
        if is_javascript_uri(&change.uri) {
            on_watched_foreign_change(state, change)?;
        } else if is_purescript_uri(&change.uri) {
            source_changed |= on_watched_source_change(state, change)?;
        }
    }

    if source_changed {
        state.invalidate_workspace_symbols();
        state.invalidate_suggestions_cache();
        event::emit_collect_all_diagnostics(state)?;
    }

    Ok(())
}

fn on_watched_source_change(state: &mut State, change: FileEvent) -> Result<bool, LspError> {
    if state.files.read().is_open(change.uri.as_str()) {
        return Ok(false);
    }

    if change.typ == FileChangeType::DELETED {
        remove_source_file(state, &change.uri)
    } else {
        let source_path = change.uri.to_file_path().ok();
        let editable = match (&state.root, source_path) {
            (Some(root), Some(path)) => path.starts_with(root),
            (Some(_), None) => false,
            (None, _) => true,
        };
        reload_source_file(state, &change.uri, Some(editable))
    }
}

fn reload_source_file(
    state: &mut State,
    uri: &Url,
    editable: Option<bool>,
) -> Result<bool, LspError> {
    let Ok(path) = uri.to_file_path() else {
        return Ok(false);
    };
    match fs::read_to_string(path) {
        Ok(content) => {
            on_change(state, uri.as_str(), &content, editable)?;
            if let Err(error) = load_sibling_foreign(state, uri) {
                tracing::warn!("Failed to reload sibling foreign file for {uri}: {error}");
            }
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            remove_source_file(state, uri)
        }
        Err(error) => {
            tracing::warn!("Failed to reload {uri}: {error}");
            Ok(false)
        }
    }
}

fn on_watched_foreign_change(state: &mut State, change: FileEvent) -> Result<(), LspError> {
    let Some(source_uri) = source_uri_from_foreign(&change.uri) else {
        return Ok(());
    };
    let source_tracked = state.files.read().id(source_uri.as_str()).is_some();
    if !source_tracked && state.foreign_files.read().id(change.uri.as_str()).is_none() {
        return Ok(());
    }

    if change.typ == FileChangeType::DELETED {
        remove_foreign_file(state, &change.uri);
    } else if let Ok(path) = change.uri.to_file_path() {
        match fs::read_to_string(path) {
            Ok(content) => on_foreign_change(state, &change.uri, &content)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                remove_foreign_file(state, &change.uri);
            }
            Err(error) => return Err(error.into()),
        }
    }

    if source_tracked {
        event::emit_collect_diagnostics(state, source_uri)?;
    }
    Ok(())
}

fn is_javascript_uri(uri: &Url) -> bool {
    uri.path().ends_with(".js")
}

fn is_purescript_uri(uri: &Url) -> bool {
    uri.path().ends_with(".purs")
}

fn source_uri_from_foreign(uri: &Url) -> Option<Url> {
    let path = uri.to_file_path().ok()?;
    Url::from_file_path(path.with_extension("purs")).ok()
}

fn load_sibling_foreign(state: &mut State, source_uri: &Url) -> Result<(), LspError> {
    let Ok(source_path) = source_uri.to_file_path() else {
        return Ok(());
    };
    let foreign_path = source_path.with_extension("js");
    let foreign_uri = Url::from_file_path(&foreign_path)
        .map_err(|()| LspError::PathParseFail(foreign_path.clone()))?;

    match fs::read_to_string(foreign_path) {
        Ok(content) => on_foreign_change(state, &foreign_uri, &content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            remove_foreign_file(state, &foreign_uri);
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn emit_associated_diagnostics(state: &mut State, uri: Url) -> Result<(), LspError> {
    let uri = if is_javascript_uri(&uri) {
        let Some(source_uri) = source_uri_from_foreign(&uri) else {
            return Ok(());
        };
        source_uri
    } else {
        uri
    };
    event::emit_collect_diagnostics(state, uri)
}

fn on_foreign_change(state: &mut State, uri: &Url, content: &str) -> Result<(), LspError> {
    state.engine.request_cancel();

    let foreign_id = state.foreign_files.write().insert(uri.as_str(), content);
    state.engine.set_foreign_content(foreign_id, content);

    let Some(source_uri) = source_uri_from_foreign(uri) else {
        return Ok(());
    };
    let Some(source_id) = state.files.read().id(source_uri.as_str()) else {
        return Ok(());
    };

    state.engine.set_foreign_file(source_id, foreign_id);

    Ok(())
}

fn remove_foreign_file(state: &mut State, uri: &Url) {
    let foreign_id = state.foreign_files.write().remove(uri.as_str());
    let Some(foreign_id) = foreign_id else {
        return;
    };

    state.engine.remove_foreign_file(foreign_id);
}

fn remove_source_file(state: &mut State, uri: &Url) -> Result<bool, LspError> {
    let file_id = state.files.read().id(uri.as_str());
    let Some(file_id) = file_id else {
        return Ok(false);
    };

    state.engine.remove_file(file_id);
    let removed_id = state.files.write().remove(uri.as_str());
    debug_assert_eq!(removed_id, Some(file_id));
    state.client.publish_diagnostics(PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: Vec::new(),
        version: None,
    })?;
    Ok(true)
}

fn on_change(
    state: &mut State,
    uri: &str,
    content: &str,
    editable: Option<bool>,
) -> Result<FileId, LspError> {
    let previous_id = state.files.read().id(uri);
    let previous_name = if let Some(id) = previous_id {
        let previous_content = state.engine.content(id)?;
        let (parsed, _) = state.engine.parsed(id)?;
        parsed.module_name(&previous_content)
    } else {
        None
    };

    // Cancel in-flight queries so that threads holding a read lock
    // over `files` are terminated quickly, compared to having to
    // wait for expensive LSP requests to complete successfully.
    state.engine.request_cancel();

    let mut files = state.files.write();
    let id = files.insert(uri, content, editable);

    state.engine.set_content(id, content);

    let (parsed, _) = state.engine.parsed(id)?;
    let current_name = parsed.module_name(content);

    if previous_name != current_name
        && let Some(previous_name) = previous_name
    {
        state.engine.remove_module_file(&previous_name, id);
    }

    if let Some(name) = current_name {
        state.engine.set_module_file(&name, id);
    }

    Ok(id)
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
        let client_socket = ClientSocket::clone(&client);
        let mut router: Router<State, ResponseError> =
            Router::new(State::new(config, client_socket));

        router
            .request::<extension::CustomInitialize, _>(initialize)
            .request::<request::Shutdown, _>(shutdown)
            .request_snapshot::<request::GotoDefinition>(definition)
            .request_snapshot::<request::HoverRequest>(hover)
            .request_snapshot::<request::CodeActionRequest>(code_action)
            .request_snapshot::<request::Completion>(completion)
            .request_snapshot::<request::ResolveCompletionItem>(resolve_completion_item)
            .request_snapshot::<request::References>(references)
            .request_snapshot::<request::PrepareRenameRequest>(prepare_rename)
            .request_snapshot::<request::Rename>(rename)
            .request_snapshot::<request::DocumentHighlightRequest>(document_highlight)
            .request_snapshot::<request::WorkspaceSymbolRequest>(workspace_symbols)
            .request_snapshot::<request::DocumentSymbolRequest>(document_symbols)
            .request_snapshot::<request::SemanticTokensFullRequest>(semantic_tokens)
            .notification_ext::<notification::Initialized>(initialized)
            .notification_ext::<notification::Exit>(exit)
            .notification_ext::<notification::DidOpenTextDocument>(did_open)
            .notification_ext::<notification::DidSaveTextDocument>(did_save)
            .notification_ext::<notification::DidCloseTextDocument>(did_close)
            .notification_ext::<notification::DidChangeConfiguration>(|_, _| Ok(()))
            .notification_ext::<notification::DidChangeTextDocument>(did_change)
            .notification_ext::<notification::DidChangeWatchedFiles>(did_change_watched_files)
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
