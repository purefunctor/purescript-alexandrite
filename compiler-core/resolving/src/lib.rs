mod algorithm;
mod error;

pub use error::*;

use building_types::{QueryProxy, QueryResult};
use files::FileId;
use hashbrown::HashTable;
use indexing::{ImportId, ImportKind, IndexedModule, TermItemId, TypeItemId};
use rustc_hash::{FxBuildHasher, FxHashMap};
use smol_str::SmolStr;
use std::fmt;
use std::hash::BuildHasher;
use std::sync::Arc;

pub trait ExternalQueries:
    QueryProxy<Indexed = Arc<IndexedModule>, Resolved = Arc<ResolvedModule>>
{
}

pub struct ResolvedClassMembers {
    index: HashTable<usize>,
    members: Vec<(TypeItemId, SmolStr, FileId, TermItemId)>,
}

impl fmt::Debug for ResolvedClassMembers {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("ResolvedClassMembers").field("members", &self.members).finish()
    }
}

impl Default for ResolvedClassMembers {
    fn default() -> Self {
        ResolvedClassMembers { index: HashTable::default(), members: Vec::default() }
    }
}

impl PartialEq for ResolvedClassMembers {
    fn eq(&self, other: &Self) -> bool {
        self.members.len() == other.members.len()
            && self.members.iter().all(|(class_id, name, file, term_id)| {
                other.lookup(*class_id, name) == Some((*file, *term_id))
            })
    }
}

impl Eq for ResolvedClassMembers {}

impl ResolvedClassMembers {
    pub fn insert(
        &mut self,
        class_id: TypeItemId,
        name: SmolStr,
        file: FileId,
        term_id: TermItemId,
    ) {
        let hash = FxBuildHasher.hash_one((class_id, name.as_str()));
        if let Some(&index) = self.index.find(hash, |&index| {
            let (existing_class_id, existing_name, _, _) = &self.members[index];
            *existing_class_id == class_id && existing_name == &name
        }) {
            self.members[index] = (class_id, name, file, term_id);
            return;
        }

        let index = self.members.len();
        self.members.push((class_id, name, file, term_id));
        self.index.insert_unique(hash, index, |&index| {
            let (class_id, name, _, _) = &self.members[index];
            FxBuildHasher.hash_one((class_id, name.as_str()))
        });
    }

    pub fn lookup(&self, class_id: TypeItemId, name: &str) -> Option<(FileId, TermItemId)> {
        let hash = FxBuildHasher.hash_one((class_id, name));
        let &index = self.index.find(hash, |&index| {
            let (existing_class_id, existing_name, _, _) = &self.members[index];
            *existing_class_id == class_id && existing_name == name
        })?;
        let (_, _, file, term_id) = self.members[index];
        Some((file, term_id))
    }

    pub fn class_members(
        &self,
        class_id: TypeItemId,
    ) -> impl Iterator<Item = (&SmolStr, FileId, TermItemId)> + '_ {
        self.members.iter().filter_map(move |(type_id, name, file, id)| {
            (*type_id == class_id).then_some((name, *file, *id))
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (TypeItemId, &SmolStr, FileId, TermItemId)> + '_ {
        self.members.iter().map(|(class_id, name, file, id)| (*class_id, name, *file, *id))
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ResolvedModule {
    pub unqualified: ResolvedImportsUnqualified,
    pub qualified: ResolvedImportsQualified,
    pub exports: ResolvedExports,
    pub locals: ResolvedLocals,
    pub class: ResolvedClassMembers,
    pub errors: Vec<ResolvingError>,
}

impl ResolvedModule {
    fn visible_import_priority(kind: ImportKind) -> Option<u8> {
        match kind {
            ImportKind::Explicit => Some(0),
            ImportKind::Implicit => Some(1),
            ImportKind::Hidden => None,
        }
    }

    fn lookup_qualified<ItemId, LookupFn, DefaultFn>(
        &self,
        qualifier: &str,
        lookup: LookupFn,
        default: DefaultFn,
    ) -> Option<(FileId, ItemId)>
    where
        LookupFn: Fn(&ResolvedImport) -> Option<(FileId, ItemId, ImportKind)>,
        DefaultFn: FnOnce() -> Option<(FileId, ItemId)>,
    {
        if let Some(imports) = self.qualified.get(qualifier) {
            let (_, file_id, item_id) = imports
                .iter()
                .filter_map(|import| {
                    let (file_id, item_id, kind) = lookup(import)?;
                    let priority = ResolvedModule::visible_import_priority(kind)?;
                    Some((priority, file_id, item_id))
                })
                .min_by_key(|(priority, _, _)| *priority)?;
            Some((file_id, item_id))
        } else if qualifier == "Prim" {
            default()
        } else {
            None
        }
    }

    fn lookup_unqualified<ItemId, LookupFn>(&self, lookup: LookupFn) -> Option<(FileId, ItemId)>
    where
        LookupFn: Fn(&ResolvedImport) -> Option<(FileId, ItemId, ImportKind)>,
    {
        let (_, file_id, item_id) = self
            .unqualified
            .values()
            .flatten()
            .filter_map(|import| {
                let (file_id, item_id, kind) = lookup(import)?;
                let priority = ResolvedModule::visible_import_priority(kind)?;
                Some((priority, file_id, item_id))
            })
            .min_by_key(|(priority, _, _)| *priority)?;
        Some((file_id, item_id))
    }

    fn lookup_prim_import<ItemId, LookupFn, DefaultFn>(
        &self,
        lookup: LookupFn,
        default: DefaultFn,
    ) -> Option<(FileId, ItemId)>
    where
        LookupFn: Fn(&ResolvedImport) -> Option<(FileId, ItemId, ImportKind)>,
        DefaultFn: FnOnce() -> Option<(FileId, ItemId)>,
    {
        if let Some(prim) = self.unqualified.get("Prim") {
            let (_, file_id, item_id) = prim
                .iter()
                .filter_map(|import| {
                    let (file_id, item_id, kind) = lookup(import)?;
                    let priority = ResolvedModule::visible_import_priority(kind)?;
                    Some((priority, file_id, item_id))
                })
                .min_by_key(|(priority, _, _)| *priority)?;
            Some((file_id, item_id))
        } else {
            default()
        }
    }

    pub fn lookup_term(
        &self,
        prim: &ResolvedModule,
        qualifier: Option<&str>,
        name: &str,
    ) -> Option<(FileId, TermItemId)> {
        if let Some(qualifier) = qualifier {
            let lookup_item = |import: &ResolvedImport| import.lookup_term(name);
            let lookup_prim = || prim.exports.lookup_term(name);
            self.lookup_qualified(qualifier, lookup_item, lookup_prim)
        } else {
            let lookup_item = |import: &ResolvedImport| import.lookup_term(name);
            let lookup_prim = || prim.exports.lookup_term(name);
            None.or_else(|| self.locals.lookup_term(name))
                .or_else(|| self.lookup_unqualified(lookup_item))
                .or_else(|| self.lookup_prim_import(lookup_item, lookup_prim))
        }
    }

    pub fn lookup_type(
        &self,
        prim: &ResolvedModule,
        qualifier: Option<&str>,
        name: &str,
    ) -> Option<(FileId, TypeItemId)> {
        if let Some(qualifier) = qualifier {
            let lookup_item = |import: &ResolvedImport| import.lookup_type(name);
            let lookup_prim = || prim.exports.lookup_type(name);
            self.lookup_qualified(qualifier, lookup_item, lookup_prim)
        } else {
            let lookup_item = |import: &ResolvedImport| import.lookup_type(name);
            let lookup_prim = || prim.exports.lookup_type(name);
            None.or_else(|| self.locals.lookup_type(name))
                .or_else(|| self.lookup_unqualified(lookup_item))
                .or_else(|| self.lookup_prim_import(lookup_item, lookup_prim))
        }
    }

    pub fn lookup_class(
        &self,
        prim: &ResolvedModule,
        qualifier: Option<&str>,
        name: &str,
    ) -> Option<(FileId, TypeItemId)> {
        if let Some(qualifier) = qualifier {
            let lookup_item = |import: &ResolvedImport| import.lookup_class(name);
            let lookup_prim = || prim.exports.lookup_class(name);
            self.lookup_qualified(qualifier, lookup_item, lookup_prim)
        } else {
            let lookup_item = |import: &ResolvedImport| import.lookup_class(name);
            let lookup_prim = || prim.exports.lookup_class(name);
            None.or_else(|| self.locals.lookup_class(name))
                .or_else(|| self.lookup_unqualified(lookup_item))
                .or_else(|| self.lookup_prim_import(lookup_item, lookup_prim))
        }
    }

    pub fn lookup_class_member(
        &self,
        class_id: TypeItemId,
        name: &str,
    ) -> Option<(FileId, TermItemId)> {
        self.class.lookup(class_id, name)
    }

    pub fn is_term_in_scope(
        &self,
        prim: &ResolvedModule,
        file_id: FileId,
        item_id: TermItemId,
    ) -> bool {
        if self.locals.contains_term(file_id, item_id) {
            return true;
        }

        for imports in self.unqualified.values() {
            for import in imports {
                if import.contains_term(file_id, item_id) {
                    return true;
                }
            }
        }

        for imports in self.qualified.values() {
            for import in imports {
                if import.contains_term(file_id, item_id) {
                    return true;
                }
            }
        }

        // If an unqualified Prim import exists, use its import list;
        if let Some(prim_imports) = self.unqualified.get("Prim") {
            for prim_import in prim_imports {
                if prim_import.contains_term(file_id, item_id) {
                    return true;
                }
            }
        }

        // if a qualified Prim import exists, use its import list;
        if let Some(prim_imports) = self.qualified.get("Prim") {
            for prim_import in prim_imports {
                if prim_import.contains_term(file_id, item_id) {
                    return true;
                }
            }
        }

        // if there are no Prim imports, use the export list.
        if prim.exports.contains_term(file_id, item_id) {
            return true;
        }

        false
    }
}

type ResolvedImportsUnqualified = FxHashMap<SmolStr, Vec<ResolvedImport>>;
type ResolvedImportsQualified = FxHashMap<SmolStr, Vec<ResolvedImport>>;

/// Precomputes unqualified lookups with the same precedence as [`ResolvedModule`].
#[derive(Debug, Default)]
pub struct ResolvedVisibleImports {
    terms: FxHashMap<SmolStr, (u8, FileId, TermItemId)>,
    types: FxHashMap<SmolStr, (u8, FileId, TypeItemId)>,
    classes: FxHashMap<SmolStr, (u8, FileId, TypeItemId)>,
}

impl ResolvedVisibleImports {
    pub fn new(module: &ResolvedModule) -> ResolvedVisibleImports {
        let mut visible = ResolvedVisibleImports::default();
        for import in module.unqualified.values().flatten() {
            for (name, file, id, kind) in import.iter_terms() {
                insert_visible(&mut visible.terms, name, file, id, kind);
            }
            for (name, file, id, kind) in import.iter_types() {
                insert_visible(&mut visible.types, name, file, id, kind);
            }
            for (name, file, id, kind) in import.iter_classes() {
                insert_visible(&mut visible.classes, name, file, id, kind);
            }
        }
        visible
    }

    pub fn lookup_term(
        &self,
        module: &ResolvedModule,
        prim: &ResolvedModule,
        qualifier: Option<&str>,
        name: &str,
    ) -> Option<(FileId, TermItemId)> {
        if qualifier.is_some() {
            return module.lookup_term(prim, qualifier, name);
        }
        module
            .locals
            .lookup_term(name)
            .or_else(|| self.terms.get(name).map(|&(_, file, id)| (file, id)))
            .or_else(|| {
                let lookup_item = |import: &ResolvedImport| import.lookup_term(name);
                let lookup_prim = || prim.exports.lookup_term(name);
                module.lookup_prim_import(lookup_item, lookup_prim)
            })
    }

    pub fn lookup_type(
        &self,
        module: &ResolvedModule,
        prim: &ResolvedModule,
        qualifier: Option<&str>,
        name: &str,
    ) -> Option<(FileId, TypeItemId)> {
        if qualifier.is_some() {
            return module.lookup_type(prim, qualifier, name);
        }
        module
            .locals
            .lookup_type(name)
            .or_else(|| self.types.get(name).map(|&(_, file, id)| (file, id)))
            .or_else(|| {
                let lookup_item = |import: &ResolvedImport| import.lookup_type(name);
                let lookup_prim = || prim.exports.lookup_type(name);
                module.lookup_prim_import(lookup_item, lookup_prim)
            })
    }

    pub fn lookup_class(
        &self,
        module: &ResolvedModule,
        prim: &ResolvedModule,
        qualifier: Option<&str>,
        name: &str,
    ) -> Option<(FileId, TypeItemId)> {
        if qualifier.is_some() {
            return module.lookup_class(prim, qualifier, name);
        }
        module
            .locals
            .lookup_class(name)
            .or_else(|| self.classes.get(name).map(|&(_, file, id)| (file, id)))
            .or_else(|| {
                let lookup_item = |import: &ResolvedImport| import.lookup_class(name);
                let lookup_prim = || prim.exports.lookup_class(name);
                module.lookup_prim_import(lookup_item, lookup_prim)
            })
    }
}

fn insert_visible<ItemId: Copy>(
    visible: &mut FxHashMap<SmolStr, (u8, FileId, ItemId)>,
    name: &SmolStr,
    file: FileId,
    id: ItemId,
    kind: ImportKind,
) {
    let Some(priority) = ResolvedModule::visible_import_priority(kind) else { return };
    if visible.get(name).is_some_and(|&(current, _, _)| current <= priority) {
        return;
    }
    visible.insert(SmolStr::clone(name), (priority, file, id));
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ResolvedLocals {
    terms: FxHashMap<SmolStr, (FileId, TermItemId)>,
    types: FxHashMap<SmolStr, (FileId, TypeItemId)>,
    classes: FxHashMap<SmolStr, (FileId, TypeItemId)>,
}

impl ResolvedLocals {
    pub fn lookup_term(&self, name: &str) -> Option<(FileId, TermItemId)> {
        self.terms.get(name).copied()
    }

    pub fn lookup_type(&self, name: &str) -> Option<(FileId, TypeItemId)> {
        self.types.get(name).copied()
    }

    pub fn contains_term(&self, file: FileId, term: TermItemId) -> bool {
        self.terms.values().any(|&(f, t)| f == file && t == term)
    }

    pub fn iter_terms(&self) -> impl Iterator<Item = (&SmolStr, FileId, TermItemId)> {
        self.terms.iter().map(|(k, (f, i))| (k, *f, *i))
    }

    pub fn iter_types(&self) -> impl Iterator<Item = (&SmolStr, FileId, TypeItemId)> {
        self.types.iter().map(|(k, (f, i))| (k, *f, *i))
    }

    pub fn lookup_class(&self, name: &str) -> Option<(FileId, TypeItemId)> {
        self.classes.get(name).copied()
    }

    pub fn iter_classes(&self) -> impl Iterator<Item = (&SmolStr, FileId, TypeItemId)> {
        self.classes.iter().map(|(k, (f, i))| (k, *f, *i))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportSource {
    Local,
    Import(ImportId),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ResolvedExports {
    terms: FxHashMap<SmolStr, (FileId, TermItemId, ExportSource)>,
    types: FxHashMap<SmolStr, (FileId, TypeItemId, ExportSource)>,
    classes: FxHashMap<SmolStr, (FileId, TypeItemId, ExportSource)>,
}

impl ResolvedExports {
    pub fn lookup_term(&self, name: &str) -> Option<(FileId, TermItemId)> {
        self.terms.get(name).copied().map(|(f, i, _)| (f, i))
    }

    pub fn lookup_type(&self, name: &str) -> Option<(FileId, TypeItemId)> {
        self.types.get(name).copied().map(|(f, i, _)| (f, i))
    }

    pub fn contains_term(&self, file: FileId, term: TermItemId) -> bool {
        self.terms.values().any(|&(f, t, _)| f == file && t == term)
    }

    pub fn iter_terms(&self) -> impl Iterator<Item = (&SmolStr, FileId, TermItemId)> {
        self.terms.iter().map(|(k, (f, i, _))| (k, *f, *i))
    }

    pub fn iter_types(&self) -> impl Iterator<Item = (&SmolStr, FileId, TypeItemId)> {
        self.types.iter().map(|(k, (f, i, _))| (k, *f, *i))
    }

    pub fn lookup_class(&self, name: &str) -> Option<(FileId, TypeItemId)> {
        self.classes.get(name).copied().map(|(f, i, _)| (f, i))
    }

    pub fn iter_classes(&self) -> impl Iterator<Item = (&SmolStr, FileId, TypeItemId)> {
        self.classes.iter().map(|(k, (f, i, _))| (k, *f, *i))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedImport {
    pub id: ImportId,
    pub file: FileId,
    pub kind: ImportKind,
    pub exported: bool,
    terms: FxHashMap<SmolStr, (FileId, TermItemId, ImportKind)>,
    types: FxHashMap<SmolStr, (FileId, TypeItemId, ImportKind)>,
    classes: FxHashMap<SmolStr, (FileId, TypeItemId, ImportKind)>,
}

impl ResolvedImport {
    fn new(id: ImportId, file: FileId, kind: ImportKind, exported: bool) -> ResolvedImport {
        let terms = FxHashMap::default();
        let types = FxHashMap::default();
        let classes = FxHashMap::default();
        ResolvedImport { id, file, kind, exported, terms, types, classes }
    }

    pub fn lookup_term(&self, name: &str) -> Option<(FileId, TermItemId, ImportKind)> {
        self.terms.get(name).copied()
    }

    pub fn lookup_type(&self, name: &str) -> Option<(FileId, TypeItemId, ImportKind)> {
        self.types.get(name).copied()
    }

    pub fn contains_term(&self, file: FileId, term: TermItemId) -> bool {
        self.terms
            .values()
            .any(|&(f, t, kind)| f == file && t == term && !matches!(kind, ImportKind::Hidden))
    }

    pub fn iter_terms(&self) -> impl Iterator<Item = (&SmolStr, FileId, TermItemId, ImportKind)> {
        self.terms.iter().map(|(k, (f, i, d))| (k, *f, *i, *d))
    }

    pub fn iter_types(&self) -> impl Iterator<Item = (&SmolStr, FileId, TypeItemId, ImportKind)> {
        self.types.iter().map(|(k, (f, i, d))| (k, *f, *i, *d))
    }

    pub fn lookup_class(&self, name: &str) -> Option<(FileId, TypeItemId, ImportKind)> {
        self.classes.get(name).copied()
    }

    pub fn iter_classes(&self) -> impl Iterator<Item = (&SmolStr, FileId, TypeItemId, ImportKind)> {
        self.classes.iter().map(|(k, (f, i, d))| (k, *f, *i, *d))
    }
}

pub fn resolve_module(queries: &impl ExternalQueries, file: FileId) -> QueryResult<ResolvedModule> {
    let algorithm::State { unqualified, qualified, exports, locals, class, errors } =
        algorithm::resolve_module(queries, file)?;
    Ok(ResolvedModule { unqualified, qualified, exports, locals, class, errors })
}
