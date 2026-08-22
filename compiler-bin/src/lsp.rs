pub mod capabilities;
pub mod error;
pub mod event;
pub mod extension;

#[cfg(test)]
mod tests;

use std::borrow::BorrowMut;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::{env, fs, io, mem, process};

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
use building::lifecycle::{
    AnalysisInvalidation, DiskObservation, DocumentKey, FileLifecycle, ForeignEvent,
    LifecycleChange, LifecycleEvent, ReloadFailure, SourceEvent, SourceUnitKey,
};
use files::FileId;
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

fn configure_materialized_prim(engine: &QueryEngine, files: &mut FileLifecycle<i32, bool>) {
    for (name, content) in MODULE_MAP {
        let path = PRIM_DIRECTORY.path().join(format!("{name}.purs"));
        fs::write(&path, content).expect("invariant violated: failed to materialize Prim module");

        let uri = Url::from_file_path(path)
            .expect("invariant violated: failed to create Prim module file URL");
        let unit = source_unit_from_source_uri(&uri)
            .expect("invariant violated: failed to create Prim source unit");
        let event = LifecycleEvent::Source {
            unit,
            event: SourceEvent::DiskObserved {
                disk: DiskObservation::Found(Arc::from(*content)),
                metadata: false,
            },
        };
        let change = files.apply(engine, event);
        let id = change
            .changed_sources()
            .next()
            .expect("invariant violated: Prim source lifecycle did not insert a source");
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
    pub files: Arc<RwLock<FileLifecycle<i32, bool>>>,
    pub diagnostics: event::DiagnosticScheduler,

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
        let mut files = FileLifecycle::default();
        configure_materialized_prim(&engine, &mut files);

        let files = Arc::new(RwLock::new(files));
        let diagnostics = event::DiagnosticScheduler::default();

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
            diagnostics,
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
            client: ClientSocket::clone(&self.client),
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
    files: Arc<RwLock<FileLifecycle<i32, bool>>>,
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

struct LspAnalyzerHost<'a> {
    queries: &'a QueryEngine,
    files: RwLockReadGuard<'a, FileLifecycle<i32, bool>>,
}

impl AnalyzerHost for LspAnalyzerHost<'_> {
    type Queries = QueryEngine;

    fn queries(&self) -> &QueryEngine {
        self.queries
    }

    fn file_id(&self, uri: &str) -> Option<FileId> {
        self.files.source_id(uri)
    }

    fn file_uri(&self, file_id: FileId) -> Result<Option<Url>, url::ParseError> {
        let Some(uri) = self.files.source_path(file_id) else {
            return Ok(None);
        };
        Url::parse(&uri).map(Some)
    }

    fn active_files(&self) -> impl Iterator<Item = FileId> {
        self.files.source_ids()
    }

    fn is_editable(&self, file_id: FileId) -> bool {
        self.files.source_metadata(file_id).copied().unwrap_or(false)
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
    let mut client = ClientSocket::clone(&state.client);
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
    let root = Option::clone(&state.root).ok_or(LspError::MissingRoot)?;

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

    let mut lifecycle_change = LifecycleChange::default();
    for (file, editable) in &files {
        let url = url::Url::from_file_path(file).map_err(|_| {
            let file = PathBuf::clone(file);
            LspError::PathParseFail(file)
        })?;

        let text = fs::read_to_string(file)?;
        let unit = source_unit_from_source_uri(&url)?;
        let event = LifecycleEvent::Source {
            unit: SourceUnitKey::clone(&unit),
            event: SourceEvent::DiskObserved {
                disk: DiskObservation::Found(Arc::from(text)),
                metadata: *editable,
            },
        };
        lifecycle_change.combine(apply_lifecycle_event(state, event));
        lifecycle_change.combine(observe_sibling_foreign(state, &unit)?);
    }
    finish_lifecycle_change(state, &lifecycle_change)?;

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
    let Some(content_change) = p.content_changes.last() else {
        return Ok(());
    };
    let unit = source_unit_from_document_uri(uri)?;
    let event = if is_javascript_uri(uri) {
        LifecycleEvent::Foreign {
            unit,
            event: ForeignEvent::Changed {
                text: Arc::from(content_change.text.as_str()),
                version: p.text_document.version,
            },
        }
    } else {
        LifecycleEvent::Source {
            unit,
            event: SourceEvent::Changed {
                text: Arc::from(content_change.text.as_str()),
                version: p.text_document.version,
            },
        }
    };
    let change = apply_lifecycle_event(state, event);
    finish_lifecycle_change(state, &change)?;

    if state.config.diagnostics_on_change {
        emit_associated_diagnostics(state, Url::clone(&p.text_document.uri))?;
    }

    Ok(())
}

fn did_open(state: &mut State, p: DidOpenTextDocumentParams) -> Result<(), LspError> {
    let uri = &p.text_document.uri;
    let unit = source_unit_from_document_uri(uri)?;

    let change = if is_javascript_uri(uri) {
        let event = LifecycleEvent::Foreign {
            unit,
            event: ForeignEvent::Opened {
                text: Arc::from(p.text_document.text.as_str()),
                version: p.text_document.version,
            },
        };
        apply_lifecycle_event(state, event)
    } else {
        let editable = source_editable(state, &unit, uri);
        let event = LifecycleEvent::Source {
            unit: SourceUnitKey::clone(&unit),
            event: SourceEvent::Opened {
                text: Arc::from(p.text_document.text.as_str()),
                version: p.text_document.version,
                metadata: editable,
            },
        };
        let mut change = apply_lifecycle_event(state, event);
        change.combine(observe_sibling_foreign(state, &unit)?);
        change
    };
    finish_lifecycle_change(state, &change)?;

    if state.config.diagnostics_on_open {
        emit_associated_diagnostics(state, p.text_document.uri)?;
    }

    Ok(())
}

fn did_close(state: &mut State, p: DidCloseTextDocumentParams) -> Result<(), LspError> {
    let uri = p.text_document.uri;
    let unit = source_unit_from_document_uri(&uri)?;
    let disk = observe_disk(&uri);
    let change = if is_javascript_uri(&uri) {
        let event = LifecycleEvent::Foreign { unit, event: ForeignEvent::Closed { disk } };
        apply_lifecycle_event(state, event)
    } else {
        let document = DocumentKey::Source(SourceUnitKey::clone(&unit));
        let was_open = state.files.read().is_open(&document);
        let event = LifecycleEvent::Source {
            unit: SourceUnitKey::clone(&unit),
            event: SourceEvent::Closed { disk },
        };
        let mut change = apply_lifecycle_event(state, event);
        if was_open {
            change.combine(observe_sibling_foreign(state, &unit)?);
        }
        change
    };
    finish_lifecycle_change(state, &change)?;
    emit_diagnostics_for_change(state, &change)?;
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
    let mut source_units = FxHashSet::default();
    let mut foreign_units = FxHashSet::default();
    for change in p.changes {
        if is_javascript_uri(&change.uri) {
            foreign_units.insert(source_unit_from_foreign_uri(&change.uri)?);
        } else if is_purescript_uri(&change.uri) {
            source_units.insert(source_unit_from_source_uri(&change.uri)?);
        }
    }

    let mut lifecycle_change = LifecycleChange::default();
    let mut observed_foreign = FxHashSet::default();
    for unit in source_units {
        let document = DocumentKey::Source(SourceUnitKey::clone(&unit));
        if state.files.read().is_open(&document) {
            continue;
        }
        let uri = Url::parse(unit.source())?;
        let disk = observe_disk(&uri);
        let source_found = matches!(disk, DiskObservation::Found(_));
        let metadata = source_editable(state, &unit, &uri);
        let event = LifecycleEvent::Source {
            unit: SourceUnitKey::clone(&unit),
            event: SourceEvent::DiskObserved { disk, metadata },
        };
        lifecycle_change.combine(apply_lifecycle_event(state, event));
        if source_found {
            lifecycle_change.combine(observe_sibling_foreign(state, &unit)?);
            observed_foreign.insert(unit);
        }
    }

    for unit in foreign_units {
        if observed_foreign.contains(&unit) {
            continue;
        }
        let document = DocumentKey::Foreign(SourceUnitKey::clone(&unit));
        if state.files.read().is_open(&document) {
            continue;
        }
        let tracked = {
            let files = state.files.read();
            files.source_id(unit.source()).is_some() || files.foreign_id(unit.foreign()).is_some()
        };
        if !tracked {
            continue;
        }
        let uri = Url::parse(unit.foreign())?;
        let event = LifecycleEvent::Foreign {
            unit,
            event: ForeignEvent::DiskObserved { disk: observe_disk(&uri) },
        };
        lifecycle_change.combine(apply_lifecycle_event(state, event));
    }

    finish_lifecycle_change(state, &lifecycle_change)?;
    emit_diagnostics_for_change(state, &lifecycle_change)?;
    Ok(())
}

fn is_javascript_uri(uri: &Url) -> bool {
    uri.path().ends_with(".js")
}

fn is_purescript_uri(uri: &Url) -> bool {
    uri.path().ends_with(".purs")
}

fn source_unit_from_document_uri(uri: &Url) -> Result<SourceUnitKey, LspError> {
    if is_javascript_uri(uri) {
        source_unit_from_foreign_uri(uri)
    } else {
        source_unit_from_source_uri(uri)
    }
}

fn file_uri_with_extension(uri: &Url, extension: &str) -> Result<Url, LspError> {
    if uri.scheme() != "file" || uri.to_file_path().is_err() {
        return Err(LspError::InvalidFileUri(Url::clone(uri)));
    }
    let uri_path = uri.path();
    let file_name_start = uri_path.rfind('/').map_or(0, |index| index + 1);
    let extension_start = uri_path[file_name_start..]
        .rfind('.')
        .filter(|index| *index > 0)
        .map_or(uri_path.len(), |index| file_name_start + index);
    let mut sibling_path = String::from(&uri_path[..extension_start]);
    sibling_path.push('.');
    sibling_path.push_str(extension);

    let mut sibling_uri = Url::clone(uri);
    sibling_uri.set_path(&sibling_path);
    Ok(sibling_uri)
}

fn source_unit_from_source_uri(source_uri: &Url) -> Result<SourceUnitKey, LspError> {
    let foreign_uri = file_uri_with_extension(source_uri, "js")?;
    Ok(SourceUnitKey::new(source_uri.as_str(), foreign_uri.as_str()))
}

fn source_unit_from_foreign_uri(foreign_uri: &Url) -> Result<SourceUnitKey, LspError> {
    let source_uri = file_uri_with_extension(foreign_uri, "purs")?;
    Ok(SourceUnitKey::new(source_uri.as_str(), foreign_uri.as_str()))
}

fn emit_associated_diagnostics(state: &mut State, uri: Url) -> Result<(), LspError> {
    let unit = source_unit_from_document_uri(&uri)?;
    event::emit_collect_diagnostics(state, Url::parse(unit.source())?)
}

fn apply_lifecycle_event(state: &mut State, event: LifecycleEvent<i32, bool>) -> LifecycleChange {
    // Cancel in-flight queries so that threads holding a read lock over the
    // lifecycle finish before this write waits for expensive LSP requests.
    state.engine.request_cancel();
    state.files.write().apply(&state.engine, event)
}

fn finish_lifecycle_change(state: &mut State, change: &LifecycleChange) -> Result<(), LspError> {
    state.diagnostics.invalidate(change, &state.files.read());
    if !matches!(change.analysis(), AnalysisInvalidation::None) {
        state.invalidate_workspace_symbols();
        state.invalidate_suggestions_cache();
    }
    for warning in change.warnings() {
        tracing::warn!("{warning}");
    }
    for removed in change.removed_sources() {
        state.client.publish_diagnostics(PublishDiagnosticsParams {
            uri: Url::parse(&removed.locator)?,
            diagnostics: Vec::new(),
            version: None,
        })?;
    }
    Ok(())
}

fn observe_sibling_foreign(
    state: &mut State,
    unit: &SourceUnitKey,
) -> Result<LifecycleChange, LspError> {
    let document = DocumentKey::Foreign(SourceUnitKey::clone(unit));
    if state.files.read().is_open(&document) {
        return Ok(LifecycleChange::default());
    }
    let uri = Url::parse(unit.foreign())?;
    let event = LifecycleEvent::Foreign {
        unit: SourceUnitKey::clone(unit),
        event: ForeignEvent::DiskObserved { disk: observe_disk(&uri) },
    };
    Ok(apply_lifecycle_event(state, event))
}

fn observe_disk(uri: &Url) -> DiskObservation {
    let path = uri.to_file_path().expect("invariant violated: expected a valid file URI");
    match fs::read_to_string(path) {
        Ok(content) => DiskObservation::Found(Arc::from(content)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => DiskObservation::NotFound,
        Err(error) => {
            let kind = error.kind();
            DiskObservation::Failed(ReloadFailure::new(kind, error.to_string()))
        }
    }
}

fn source_editable(state: &State, unit: &SourceUnitKey, uri: &Url) -> bool {
    let previous = {
        let files = state.files.read();
        let file_id = files.source_id(unit.source());
        file_id.and_then(|file_id| files.source_metadata(file_id)).copied()
    };
    previous.unwrap_or_else(|| match (&state.root, uri.to_file_path()) {
        (Some(root), Ok(path)) => path.starts_with(root),
        (Some(_), Err(())) => false,
        (None, _) => true,
    })
}

fn emit_diagnostics_for_change(
    state: &mut State,
    change: &LifecycleChange,
) -> Result<(), LspError> {
    match change.analysis() {
        AnalysisInvalidation::None => Ok(()),
        AnalysisInvalidation::Sources(sources) => {
            for file_id in sources {
                event::emit_collect_diagnostics_id(state, *file_id)?;
            }
            Ok(())
        }
        AnalysisInvalidation::Workspace => event::emit_collect_all_diagnostics(state),
    }
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
            .event_ext::<event::CollectDiagnostics>(event::collect_diagnostics)
            .event_ext::<event::DiagnosticsFinished>(event::finish_diagnostics);

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
