pub mod render;

use std::fmt::Write;

use analyzer::completion::SuggestionsCache;
use analyzer::position::PositionEncoding;
use analyzer::{QueryEngine, prim};
use async_lsp::lsp_types::{
    CodeActionContext, CodeActionKind, CodeActionOrCommand, CodeActionResponse,
    CodeActionTriggerKind, CompletionItemKind, CompletionList, CompletionResponse,
    DocumentHighlight, DocumentSymbolResponse, GotoDefinitionResponse, HoverContents,
    LanguageString, Location, MarkedString, Position, Range, SymbolInformation, TextEdit, Url,
    WorkspaceEdit, WorkspaceSymbolResponse,
};
use files::{FileId, Files};
use itertools::Itertools;
use line_index::{LineIndex, TextSize};
use render::{TabledCompletionItem, TabledDetailedCompletionItem};
use tabled::Table;
use tabled::settings::{Padding, Style};

#[derive(Debug, Clone, Copy)]
enum CursorKind {
    GotoDefinition,
    Hover,
    Completion,
    CompletionCached,
    References,
    DocumentHighlight,
    DocumentSymbols,
    CodeAction,
}

impl CursorKind {
    const CHARACTERS: &[char] = &['@', '$', '^', '~', '%', '!', '&', '.'];

    fn parse(text: &str) -> Option<CursorKind> {
        match text {
            "@" => Some(CursorKind::GotoDefinition),
            "$" => Some(CursorKind::Hover),
            "^" => Some(CursorKind::Completion),
            "~" => Some(CursorKind::CompletionCached),
            "%" => Some(CursorKind::References),
            "&" => Some(CursorKind::DocumentHighlight),
            "!" => Some(CursorKind::DocumentSymbols),
            "." => Some(CursorKind::CodeAction),
            _ => None,
        }
    }

    fn valid(c: char) -> bool {
        CursorKind::CHARACTERS.contains(&c)
    }
}

fn cursor_marker_line(line: &str) -> bool {
    let Some(markers) = line.strip_prefix("--") else { return false };
    markers.chars().all(|character| character.is_whitespace() || CursorKind::valid(character))
}

enum Request {
    Cursor(Position, CursorKind),
    WorkspaceSymbols(String),
}

const WORKSPACE_SYMBOLS_DIRECTIVE: &str = "-- #";

fn extract_cursors(content: &str) -> Vec<(usize, Request)> {
    let line_index = LineIndex::new(content);
    let mut cursors = vec![];

    for (index, text) in content.match_indices(CursorKind::valid) {
        let line_col = line_index.line_col(TextSize::new(index as u32));
        let line_range = line_index.line(line_col.line).unwrap();
        if !cursor_marker_line(&content[line_range]) {
            continue;
        }

        let line = line_col.line - 1;
        let character = line_col.col;
        let position = Position::new(line, character);
        let Some(kind) = CursorKind::parse(text) else { continue };

        cursors.push((index, Request::Cursor(position, kind)));
    }

    cursors
}

fn extract_workspace_symbol_queries(content: &str) -> Vec<(usize, Request)> {
    let line_index = LineIndex::new(content);
    let mut queries = vec![];

    for (index, _) in content.match_indices(WORKSPACE_SYMBOLS_DIRECTIVE) {
        let line_col = line_index.line_col(TextSize::new(index as u32));
        let line_range = line_index.line(line_col.line).unwrap();
        let line = &content[line_range];
        if !line.starts_with(WORKSPACE_SYMBOLS_DIRECTIVE) {
            continue;
        }

        let query = line
            .strip_prefix(WORKSPACE_SYMBOLS_DIRECTIVE)
            .expect("line starts with workspace symbols directive")
            .trim()
            .to_string();

        queries.push((index, Request::WorkspaceSymbols(query)));
    }

    queries
}

fn extract_requests(content: &str) -> Vec<Request> {
    let mut requests = extract_cursors(content);
    requests.extend(extract_workspace_symbol_queries(content));
    requests.sort_by_key(|(index, _)| *index);
    requests.into_iter().map(|(_, request)| request).collect()
}

pub fn report(engine: &QueryEngine, files: &Files, id: FileId) -> String {
    let uri = {
        let path = files.path(id);
        let content = files.content(id);
        let uri = Url::parse(&path).unwrap();
        prim::handle_generated(uri, &content).unwrap()
    };

    let content = engine.content(id);
    let line_index = LineIndex::new(&content);
    let requests = extract_requests(&content);

    let mut suggestions_cache = SuggestionsCache::default();
    let mut symbols_cache = analyzer::symbols::WorkspaceSymbolsCache::default();
    let mut result = String::new();
    for (index, request) in requests.iter().enumerate() {
        let uri = uri.clone();

        if index > 0 {
            writeln!(result, "\n").unwrap();
        }

        match request {
            Request::Cursor(position, cursor) => {
                writeln!(result, "{cursor:#?} at {position:?}\n").unwrap();

                let line_0 = line_index.line(position.line);
                let line_1 = line_index.line(position.line + 1);
                if let Some((line_0, line_1)) = line_0.zip(line_1) {
                    let line_0 = &content[line_0];
                    let line_1 = &content[line_1];
                    writeln!(result, "```").unwrap();
                    write!(result, "{line_0}").unwrap();
                    write!(result, "{line_1}").unwrap();
                    writeln!(result, "```").unwrap();
                }
                writeln!(result).unwrap();

                if matches!(cursor, CursorKind::Completion) {
                    suggestions_cache = SuggestionsCache::default();
                }

                dispatch_cursor(
                    &mut result,
                    engine,
                    files,
                    &mut suggestions_cache,
                    *position,
                    *cursor,
                    uri,
                );
            }
            Request::WorkspaceSymbols(query) => {
                writeln!(result, "WorkspaceSymbols query {query:?}\n").unwrap();
                dispatch_workspace_symbols(&mut result, engine, files, &mut symbols_cache, query);
            }
        }
    }

    redact_paths(result)
}

fn render_location(location: Location) -> String {
    format!(
        "{} @ {}:{}..{}:{}",
        location.uri,
        location.range.start.line,
        location.range.start.character,
        location.range.end.line,
        location.range.end.character,
    )
}

fn render_text_edit(edit: TextEdit) -> String {
    format!(
        "{}:{}..{}:{} => {:?}",
        edit.range.start.line,
        edit.range.start.character,
        edit.range.end.line,
        edit.range.end.character,
        edit.new_text,
    )
}

fn render_workspace_edit(edit: WorkspaceEdit) -> Vec<String> {
    let mut result = vec![];

    if let Some(changes) = edit.changes {
        for edits in changes.into_values() {
            result.extend(edits.into_iter().map(render_text_edit));
        }
    }

    result.sort();
    result
}

fn render_code_action_response(response: CodeActionResponse) -> String {
    let mut result = vec![];

    for action in response {
        match action {
            CodeActionOrCommand::CodeAction(action) => {
                let kind = action.kind.as_ref().map(CodeActionKind::as_str).unwrap_or("<none>");

                if let Some(edit) = action.edit {
                    let edits = render_workspace_edit(edit);
                    if edits.is_empty() {
                        result.push(format!("{} [{kind}] <no edit>", action.title));
                    } else {
                        for edit in edits {
                            result.push(format!("{} [{kind}] {edit}", action.title));
                        }
                    }
                } else {
                    result.push(format!("{} [{kind}] <no edit>", action.title));
                }
            }
            CodeActionOrCommand::Command(command) => {
                result.push(format!("{} [command:{}]", command.title, command.command));
            }
        }
    }

    if result.is_empty() { "<empty>".to_string() } else { result.join("\n") }
}

fn dispatch_cursor(
    result: &mut String,
    engine: &QueryEngine,
    files: &Files,
    cache: &mut SuggestionsCache,
    position: Position,
    cursor: CursorKind,
    uri: Url,
) {
    let encoding = PositionEncoding::Utf16;
    let context = analyzer::LanguageContext::new(engine, files, encoding);

    match cursor {
        CursorKind::GotoDefinition => {
            if let Ok(Some(response)) =
                analyzer::definition::implementation(&context, uri, position)
            {
                match response {
                    GotoDefinitionResponse::Scalar(location) => {
                        let location = render_location(location);
                        writeln!(result, "{location}").unwrap();
                    }
                    GotoDefinitionResponse::Array(location) => {
                        let location = location.into_iter().map(render_location).join("\n");
                        writeln!(result, "{location}").unwrap();
                    }
                    GotoDefinitionResponse::Link(_) => (),
                }
            } else {
                writeln!(result, "<empty>").unwrap();
            }
        }
        CursorKind::Hover => {
            if let Ok(Some(response)) = analyzer::hover::implementation(&context, uri, position) {
                let convert = |marked: MarkedString| -> String {
                    match marked {
                        MarkedString::String(string) => string,
                        MarkedString::LanguageString(LanguageString {
                            language, value, ..
                        }) => format!("```{language}\n{value}\n```"),
                    }
                };

                match response.contents {
                    HoverContents::Scalar(marked) => {
                        let marked = convert(marked);
                        if marked.is_empty() {
                            writeln!(result, "<empty>").unwrap();
                        } else {
                            writeln!(result, "{marked}").unwrap();
                        }
                    }
                    HoverContents::Array(marked) => {
                        let marked = marked.into_iter().map(convert).join("\n");
                        if marked.is_empty() {
                            writeln!(result, "<empty>").unwrap();
                        } else {
                            writeln!(result, "{marked}").unwrap();
                        }
                    }
                    HoverContents::Markup(markup) => {
                        if markup.value.is_empty() {
                            writeln!(result, "<empty>").unwrap();
                        } else {
                            writeln!(result, "{}", markup.value).unwrap();
                        }
                    }
                }
            } else {
                writeln!(result, "<empty>").unwrap();
            }
        }
        CursorKind::CodeAction => {
            let range = Range::new(position, position);
            let action_context = CodeActionContext {
                diagnostics: vec![],
                only: Some(vec![CodeActionKind::QUICKFIX]),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            };

            if let Ok(Some(response)) =
                analyzer::code_action::implementation(&context, uri, range, action_context)
            {
                writeln!(result, "{}", render_code_action_response(response)).unwrap();
            } else {
                writeln!(result, "<empty>").unwrap();
            }
        }
        CursorKind::Completion | CursorKind::CompletionCached => {
            if let Ok(Some(response)) =
                analyzer::completion::implementation(&context, cache, uri, position)
            {
                match response {
                    CompletionResponse::Array(items)
                    | CompletionResponse::List(CompletionList { items, .. }) => {
                        let items: Vec<_> = items
                            .into_iter()
                            .filter_map(|item| {
                                analyzer::completion::resolve::implementation(engine, item).ok()
                            })
                            .collect();

                        let has_values =
                            items.iter().any(|item| item.kind == Some(CompletionItemKind::VALUE));

                        let mut table = if has_values {
                            let items: Vec<TabledDetailedCompletionItem> =
                                items.into_iter().map(TabledDetailedCompletionItem::from).collect();
                            Table::new(items)
                        } else {
                            let items: Vec<TabledCompletionItem> =
                                items.into_iter().map(TabledCompletionItem::from).collect();
                            Table::new(items)
                        };
                        table.with(Style::modern_rounded());
                        table.with(Padding::new(2, 2, 0, 0));

                        writeln!(result, "{table}").unwrap();
                    }
                }
            } else {
                writeln!(result, "<empty>").unwrap();
            }
        }
        CursorKind::DocumentSymbols => {
            if let Ok(Some(response)) = analyzer::symbols::document(&context, uri) {
                writeln!(result, "{}", render_document_symbols_response(response)).unwrap();
            } else {
                writeln!(result, "<empty>").unwrap();
            }
        }
        CursorKind::References => {
            if let Ok(Some(location)) =
                analyzer::references::implementation(&context, uri, position)
            {
                let location = location.into_iter().map(render_location).join("\n");
                writeln!(result, "{location}").unwrap();
            } else {
                writeln!(result, "<empty>").unwrap();
            }
        }
        CursorKind::DocumentHighlight => {
            let render_highlight = |h: DocumentHighlight| -> String {
                format!(
                    "{}:{}..{}:{}",
                    h.range.start.line,
                    h.range.start.character,
                    h.range.end.line,
                    h.range.end.character
                )
            };

            if let Ok(Some(highlights)) =
                analyzer::document_highlight::implementation(&context, uri, position)
            {
                let highlights = highlights.into_iter().map(render_highlight).join("\n");
                writeln!(result, "{highlights}").unwrap();
            } else {
                writeln!(result, "<empty>").unwrap();
            }
        }
    }
}

fn dispatch_workspace_symbols(
    result: &mut String,
    engine: &QueryEngine,
    files: &Files,
    cache: &mut analyzer::symbols::WorkspaceSymbolsCache,
    query: &str,
) {
    let encoding = PositionEncoding::Utf16;
    let context = analyzer::LanguageContext::new(engine, files, encoding);

    match analyzer::symbols::workspace(&context, cache, query) {
        Ok(Some(WorkspaceSymbolResponse::Flat(symbols))) => {
            let mut lines = symbols
                .into_iter()
                .map(|symbol| {
                    let location = render_location(symbol.location);
                    format!("{} {:?} {location}", symbol.name, symbol.kind)
                })
                .collect_vec();

            if lines.is_empty() {
                writeln!(result, "<empty>").unwrap();
            } else {
                lines.sort();
                writeln!(result, "{}", lines.join("\n")).unwrap();
            }
        }
        Ok(Some(_)) => {
            writeln!(result, "<unsupported>").unwrap();
        }
        Ok(None) => {
            writeln!(result, "<none>").unwrap();
        }
        Err(_) => {
            writeln!(result, "<empty>").unwrap();
        }
    }
}

fn render_document_symbols_response(response: DocumentSymbolResponse) -> String {
    match response {
        DocumentSymbolResponse::Flat(symbols) => {
            if symbols.is_empty() {
                "<empty>".into()
            } else {
                symbols.into_iter().map(render_symbol_information).join("\n")
            }
        }
        DocumentSymbolResponse::Nested(_) => "<nested>".into(),
    }
}

fn render_symbol_information(symbol: SymbolInformation) -> String {
    let SymbolInformation { name, kind, location, .. } = symbol;
    format!(
        "{name} :: {kind:?} @ {}:{}..{}:{}",
        location.range.start.line,
        location.range.start.character,
        location.range.end.line,
        location.range.end.character,
    )
}

fn redact_paths(mut result: String) -> String {
    let manifest_directory = env!("CARGO_MANIFEST_DIR");
    let temporary_directory = prim::TEMPORARY_DIRECTORY.path();

    let manifest_directory_url = url::Url::from_file_path(manifest_directory).unwrap();
    let temporary_directory_url = url::Url::from_file_path(temporary_directory).unwrap();

    for (url, redacted) in [
        (manifest_directory_url, "file:///tests-integration"),
        (temporary_directory_url, "file:///temporary-directory"),
    ] {
        let uri = url.to_string();
        result = result.replace(&uri, redacted);
    }

    result
}
