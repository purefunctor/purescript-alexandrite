//! A simplified, single-threaded query engine for WASM.

use std::cell::RefCell;
use std::sync::Arc;

use analyzer::AnalyzerHost;
use building_types::{ModuleNameId, ModuleNameInterner, QueryError, QueryProxy, QueryResult};
use documenting::DocumentedModule;
use files::{FileId, Files};
use indexing::IndexedModule;
use lowering::{GroupedModule, LoweredModule};
use parsing::FullParsedModule;
use prim_constants::MODULE_MAP;
use resolving::{ExportedModule, ResolvedModule};
use rustc_hash::FxHashMap;
use stabilizing::StabilizedModule;
use url::Url;

#[derive(Default)]
struct InputStorage {
    content: FxHashMap<FileId, Arc<str>>,
    module: FxHashMap<ModuleNameId, FileId>,
}

#[derive(Default)]
struct DerivedStorage {
    parsed: FxHashMap<FileId, FullParsedModule>,
    stabilized: FxHashMap<FileId, Arc<StabilizedModule>>,
    indexed: FxHashMap<FileId, Arc<IndexedModule>>,
    lowered: FxHashMap<FileId, Arc<LoweredModule>>,
    grouped: FxHashMap<FileId, Arc<GroupedModule>>,
    resolved: FxHashMap<FileId, Arc<ResolvedModule>>,
    exported: FxHashMap<FileId, Arc<ExportedModule>>,
    bracketed: FxHashMap<FileId, Arc<sugar::Bracketed>>,
    sectioned: FxHashMap<FileId, Arc<sugar::Sectioned>>,
    checked: FxHashMap<FileId, Arc<checking::CheckedModule>>,
    documented: FxHashMap<FileId, Arc<DocumentedModule>>,
}

#[derive(Default)]
struct InternedStorage {
    module: ModuleNameInterner,
    checking: checking::CoreInterners,
}

/// Single-threaded query engine for WASM
pub struct WasmQueryEngine {
    files: Files,
    input: InputStorage,
    derived: RefCell<DerivedStorage>,
    interned: InternedStorage,

    prim_id: FileId,
    user_id: Option<FileId>,
    /// FileIds of external (package) modules, for cleanup
    external_ids: Vec<FileId>,
}

impl WasmQueryEngine {
    pub fn new() -> Self {
        let mut files = Files::default();
        let mut input = InputStorage::default();
        let interned = InternedStorage::default();

        // Load Prim modules
        let mut prim_id = None;
        for (name, module_content) in MODULE_MAP {
            let path = format!("prim://localhost/{name}.purs");
            let id = files.insert(path.as_str(), *module_content);
            input.content.insert(id, Arc::from(*module_content));

            let name_id = interned.module.intern(name);
            input.module.insert(name_id, id);

            if *name == "Prim" {
                prim_id = Some(id);
            }
        }

        Self {
            files,
            input,
            derived: RefCell::new(DerivedStorage::default()),
            interned,
            prim_id: prim_id.expect("invariant violated: Prim must exist"),
            user_id: None,
            external_ids: Vec::new(),
        }
    }

    /// Register an external module source, parsing the module name from source.
    /// Returns the parsed module name on success, or None if parsing fails.
    pub fn register_external_source(&mut self, path: &str, source: &str) -> Option<String> {
        let virtual_path = format!("pkg://registry/{path}");
        let id = self.files.insert(virtual_path.as_str(), source);

        self.input.content.insert(id, Arc::from(source));

        let (parsed, _) = self.parsed(id).ok()?;
        let module_name = parsed.module_name(source)?;

        let name_id = self.interned.module.intern(&module_name);
        self.input.module.insert(name_id, id);

        self.external_ids.push(id);

        Some(module_name.to_string())
    }

    /// Clear all external modules (packages), keeping Prim and user modules.
    pub fn clear_external_modules(&mut self) {
        let derived = self.derived.get_mut();
        let input = &mut self.input;

        for id in self.external_ids.drain(..) {
            input.content.remove(&id);
            input.module.retain(|_, file_id| *file_id != id);
            derived.parsed.remove(&id);
            derived.stabilized.remove(&id);
            derived.indexed.remove(&id);
            derived.lowered.remove(&id);
            derived.grouped.remove(&id);
            derived.resolved.remove(&id);
            derived.exported.remove(&id);
            derived.bracketed.remove(&id);
            derived.sectioned.remove(&id);
            derived.checked.remove(&id);
            derived.documented.remove(&id);
        }

        if let Some(user_id) = self.user_id {
            derived.parsed.remove(&user_id);
            derived.stabilized.remove(&user_id);
            derived.indexed.remove(&user_id);
            derived.lowered.remove(&user_id);
            derived.grouped.remove(&user_id);
            derived.resolved.remove(&user_id);
            derived.exported.remove(&user_id);
            derived.bracketed.remove(&user_id);
            derived.sectioned.remove(&user_id);
            derived.checked.remove(&user_id);
            derived.documented.remove(&user_id);
        }
    }

    /// Set the user's source code and return its FileId.
    /// Clears caches for the user file only.
    pub fn set_user_source(&mut self, source: &str) -> FileId {
        let id = if let Some(existing_id) = self.user_id {
            let derived = self.derived.get_mut();
            derived.parsed.remove(&existing_id);
            derived.stabilized.remove(&existing_id);
            derived.indexed.remove(&existing_id);
            derived.lowered.remove(&existing_id);
            derived.grouped.remove(&existing_id);
            derived.resolved.remove(&existing_id);
            derived.exported.remove(&existing_id);
            derived.bracketed.remove(&existing_id);
            derived.sectioned.remove(&existing_id);
            derived.checked.remove(&existing_id);
            derived.documented.remove(&existing_id);
            existing_id
        } else {
            let id = self.files.insert("user://localhost/Main.purs", source);
            self.user_id = Some(id);

            let name_id = self.interned.module.intern("Main");
            self.input.module.insert(name_id, id);

            id
        };

        self.input.content.insert(id, Arc::from(source));
        id
    }

    pub fn checked(&self, id: FileId) -> QueryResult<Arc<checking::CheckedModule>> {
        if let Some(cached) = self.derived.borrow().checked.get(&id) {
            return Ok(cached.clone());
        }

        let checked = Arc::new(checking::check_module(self, id)?);

        self.derived.borrow_mut().checked.insert(id, checked.clone());
        Ok(checked)
    }

    pub fn documented(&self, id: FileId) -> QueryResult<Arc<DocumentedModule>> {
        if let Some(cached) = self.derived.borrow().documented.get(&id) {
            return Ok(cached.clone());
        }

        let content = self.content(id)?;
        let (parsed, _) = self.parsed(id)?;
        let stabilized = self.stabilized(id)?;
        let indexed = self.indexed(id)?;
        let documented = documenting::document_module(&content, &parsed, &stabilized, &indexed);

        self.derived.borrow_mut().documented.insert(id, documented.clone());
        Ok(documented)
    }

    fn content(&self, id: FileId) -> QueryResult<Arc<str>> {
        self.input.content.get(&id).cloned().ok_or(QueryError::MissingContent { file_id: id })
    }
}

impl QueryProxy for WasmQueryEngine {
    type Parsed = FullParsedModule;
    type Stabilized = Arc<StabilizedModule>;
    type Indexed = Arc<IndexedModule>;
    type Lowered = Arc<LoweredModule>;
    type Grouped = Arc<GroupedModule>;
    type Resolved = Arc<ResolvedModule>;
    type Exported = Arc<ExportedModule>;
    type Bracketed = Arc<sugar::Bracketed>;
    type Sectioned = Arc<sugar::Sectioned>;
    type Checked = Arc<checking::CheckedModule>;
    type Documented = Arc<DocumentedModule>;

    fn content(&self, id: FileId) -> QueryResult<Arc<str>> {
        WasmQueryEngine::content(self, id)
    }

    fn parsed(&self, id: FileId) -> QueryResult<Self::Parsed> {
        if let Some(cached) = self.derived.borrow().parsed.get(&id) {
            return Ok(cached.clone());
        }

        let content = self.content(id)?;
        let lexed = lexing::lex(&content);
        let tokens = lexing::layout(&lexed);
        let parsed = parsing::parse(&lexed, &tokens);

        self.derived.borrow_mut().parsed.insert(id, parsed.clone());
        Ok(parsed)
    }

    fn stabilized(&self, id: FileId) -> QueryResult<Self::Stabilized> {
        if let Some(cached) = self.derived.borrow().stabilized.get(&id) {
            return Ok(cached.clone());
        }

        let (parsed, _) = self.parsed(id)?;
        let node = parsed.syntax_node();
        let stabilized = Arc::new(stabilizing::stabilize_module(&node));

        self.derived.borrow_mut().stabilized.insert(id, stabilized.clone());
        Ok(stabilized)
    }

    fn indexed(&self, id: FileId) -> QueryResult<Self::Indexed> {
        if let Some(cached) = self.derived.borrow().indexed.get(&id) {
            return Ok(cached.clone());
        }

        let content = self.content(id)?;
        let (parsed, _) = self.parsed(id)?;
        let stabilized = self.stabilized(id)?;

        let module = parsed.cst();
        let indexed = Arc::new(indexing::index_module(&content, &module, &stabilized));

        self.derived.borrow_mut().indexed.insert(id, indexed.clone());
        Ok(indexed)
    }

    fn lowered(&self, id: FileId) -> QueryResult<Self::Lowered> {
        if let Some(cached) = self.derived.borrow().lowered.get(&id) {
            return Ok(cached.clone());
        }

        let content = self.content(id)?;
        let (parsed, _) = self.parsed(id)?;
        let prim = self.resolved(self.prim_id)?;
        let stabilized = self.stabilized(id)?;
        let indexed = self.indexed(id)?;
        let resolved = self.resolved(id)?;

        let module = parsed.cst();
        let lowered = Arc::new(lowering::lower_module(
            id,
            &content,
            &module,
            &prim,
            &stabilized,
            &indexed,
            &resolved,
        ));

        self.derived.borrow_mut().lowered.insert(id, lowered.clone());
        Ok(lowered)
    }

    fn grouped(&self, id: FileId) -> QueryResult<Self::Grouped> {
        if let Some(cached) = self.derived.borrow().grouped.get(&id) {
            return Ok(cached.clone());
        }

        let lowered = self.lowered(id)?;
        let indexed = self.indexed(id)?;
        let grouped = Arc::new(lowering::group_module(&indexed, &lowered));

        self.derived.borrow_mut().grouped.insert(id, grouped.clone());
        Ok(grouped)
    }

    fn resolved(&self, id: FileId) -> QueryResult<Self::Resolved> {
        if let Some(cached) = self.derived.borrow().resolved.get(&id) {
            return Ok(cached.clone());
        }

        let resolved = Arc::new(resolving::resolve_module(self, id)?);

        self.derived.borrow_mut().resolved.insert(id, resolved.clone());
        Ok(resolved)
    }

    fn exported(&self, id: FileId) -> QueryResult<Self::Exported> {
        if let Some(cached) = self.derived.borrow().exported.get(&id) {
            return Ok(Arc::clone(cached));
        }

        let resolved = self.resolved(id)?;
        let exported = Arc::new(resolving::export_module(&resolved));

        self.derived.borrow_mut().exported.insert(id, Arc::clone(&exported));
        Ok(exported)
    }

    fn bracketed(&self, id: FileId) -> QueryResult<Self::Bracketed> {
        if let Some(cached) = self.derived.borrow().bracketed.get(&id) {
            return Ok(cached.clone());
        }

        let lowered = self.lowered(id)?;
        let bracketed = Arc::new(sugar::bracketed(self, &lowered)?);

        self.derived.borrow_mut().bracketed.insert(id, bracketed.clone());
        Ok(bracketed)
    }

    fn sectioned(&self, id: FileId) -> QueryResult<Self::Sectioned> {
        if let Some(cached) = self.derived.borrow().sectioned.get(&id) {
            return Ok(cached.clone());
        }

        let lowered = self.lowered(id)?;
        let sectioned = Arc::new(sugar::sectioned(&lowered));

        self.derived.borrow_mut().sectioned.insert(id, sectioned.clone());
        Ok(sectioned)
    }

    fn checked(&self, id: FileId) -> QueryResult<Arc<checking::CheckedModule>> {
        WasmQueryEngine::checked(self, id)
    }

    fn documented(&self, id: FileId) -> QueryResult<Arc<DocumentedModule>> {
        WasmQueryEngine::documented(self, id)
    }

    fn prim_id(&self) -> FileId {
        self.prim_id
    }

    fn module_file(&self, name: &str) -> Option<FileId> {
        let name_id = self.interned.module.lookup(name)?;
        self.input.module.get(&name_id).copied()
    }
}

impl AnalyzerHost for WasmQueryEngine {
    type Queries = WasmQueryEngine;

    fn queries(&self) -> &WasmQueryEngine {
        self
    }

    fn file_id(&self, uri: &str) -> Option<FileId> {
        let id = self.files.id(uri)?;
        self.input.content.contains_key(&id).then_some(id)
    }

    fn file_uri(&self, file_id: FileId) -> Result<Option<Url>, url::ParseError> {
        if !self.input.content.contains_key(&file_id) {
            return Ok(None);
        }
        let path = self.files.path(file_id);
        Url::parse(&path).map(Some)
    }

    fn active_files(&self) -> impl Iterator<Item = FileId> {
        let files = self.files.iter_id();
        files.filter(|id| self.input.content.contains_key(id))
    }

    fn is_editable(&self, file_id: FileId) -> bool {
        self.user_id == Some(file_id)
    }
}

impl checking::PrettyQueries for WasmQueryEngine {
    fn lookup_type(&self, id: checking::core::TypeId) -> checking::core::Type {
        self.interned.checking.lookup_type(id)
    }

    fn lookup_forall_binder(
        &self,
        id: checking::core::ForallBinderId,
    ) -> checking::core::ForallBinder {
        self.interned.checking.lookup_forall_binder(id)
    }

    fn lookup_row_type(&self, id: checking::core::RowTypeId) -> checking::core::RowType {
        self.interned.checking.lookup_row_type(id)
    }

    fn lookup_smol_str(&self, id: checking::core::SmolStrId) -> smol_str::SmolStr {
        self.interned.checking.lookup_smol_str(id)
    }
}

impl checking::ExternalQueries for WasmQueryEngine {
    fn intern_type(&self, t: checking::core::Type) -> checking::core::TypeId {
        self.interned.checking.intern_type(t)
    }

    fn intern_forall_binder(
        &self,
        binder: checking::core::ForallBinder,
    ) -> checking::core::ForallBinderId {
        self.interned.checking.intern_forall_binder(binder)
    }

    fn intern_row_type(&self, row: checking::core::RowType) -> checking::core::RowTypeId {
        self.interned.checking.intern_row_type(row)
    }

    fn intern_smol_str(&self, s: smol_str::SmolStr) -> checking::core::SmolStrId {
        self.interned.checking.intern_smol_str(s)
    }
}

impl resolving::ExternalQueries for WasmQueryEngine {}
impl sugar::ExternalQueries for WasmQueryEngine {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_user_source_invalidates_exported_module() {
        let mut engine = WasmQueryEngine::new();
        let id = engine.set_user_source("module Main (x) where\n\nx = 1\ny = 2");

        let exported = engine.exported(id).unwrap();
        assert_eq!(exported.local.len(), 1);

        let replacement_id = engine.set_user_source("module Main (x, y) where\n\nx = 1\ny = 2");
        assert_eq!(replacement_id, id);

        let exported = engine.exported(replacement_id).unwrap();
        assert_eq!(exported.local.len(), 2);
    }
}
