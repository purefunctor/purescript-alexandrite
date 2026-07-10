mod algorithm;
mod error;
mod items;
mod source;

pub use error::*;
pub use items::*;
pub use source::*;

use std::hash::BuildHasher;
use std::{fmt, ops};

use hashbrown::HashTable;
use la_arena::Arena;
use rustc_hash::{FxBuildHasher, FxHashMap};
use smol_str::SmolStr;
use stabilizing::StabilizedModule;
use syntax::{SyntaxNodePtr, cst};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IndexedModule {
    pub kind: ExportKind,
    pub names: IndexedNames,
    pub exports: IndexedExports,
    pub items: IndexedItems,
    pub imports: IndexedImports,
    pub pairs: IndexedPairs,
    pub errors: Vec<IndexingError>,
}

impl IndexedModule {
    pub fn term_item_ptr(
        &self,
        stabilized: &StabilizedModule,
        id: TermItemId,
    ) -> impl Iterator<Item = SyntaxNodePtr> {
        const fn aux<T: Copy>(expected_id: TermItemId) -> impl Fn(&(T, TermItemId)) -> Option<T> {
            move |(id, item_id)| if *item_id == expected_id { Some(*id) } else { None }
        }

        let declaration = self.pairs.declaration_to_term.iter().filter_map(aux(id));
        let constructor = self.pairs.constructor_to_term.iter().filter_map(aux(id));
        let class_member = self.pairs.class_member_to_term.iter().filter_map(aux(id));

        let declaration = declaration.filter_map(|id| stabilized.syntax_ptr(id));
        let constructor = constructor.filter_map(|id| stabilized.syntax_ptr(id));
        let class_member = class_member.filter_map(|id| stabilized.syntax_ptr(id));

        declaration.chain(constructor).chain(class_member)
    }

    pub fn type_item_ptr(
        &self,
        stabilized: &StabilizedModule,
        id: TypeItemId,
    ) -> impl Iterator<Item = SyntaxNodePtr> {
        const fn aux<T: Copy>(expected_id: TypeItemId) -> impl Fn(&(T, TypeItemId)) -> Option<T> {
            move |(id, item_id)| if *item_id == expected_id { Some(*id) } else { None }
        }

        let declaration = self.pairs.declaration_to_type.iter().filter_map(aux(id));
        declaration.filter_map(|id| stabilized.syntax_ptr(id))
    }

    pub fn data_constructors(&self, id: TypeItemId) -> impl Iterator<Item = TermItemId> + '_ {
        let constructors = match &self.items[id].kind {
            TypeItemKind::Data { constructors, .. }
            | TypeItemKind::Newtype { constructors, .. } => constructors.as_slice(),
            _ => &[],
        };

        constructors.iter().copied()
    }

    pub fn constructor_type(&self, id: TermItemId) -> Option<TypeItemId> {
        self.items.iter_types().find_map(|(type_id, _)| {
            if self.data_constructors(type_id).any(|term_id| term_id == id) {
                Some(type_id)
            } else {
                None
            }
        })
    }

    pub fn class_members(&self, id: TypeItemId) -> impl Iterator<Item = TermItemId> + '_ {
        let members = match &self.items[id].kind {
            TypeItemKind::Class { members, .. } => members.as_slice(),
            _ => &[],
        };

        members.iter().copied()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IndexedNames {
    pub terms: NameIndex<TermItemId>,
    pub types: NameIndex<TypeItemId>,
}

pub struct NameIndex<ItemId> {
    first: HashTable<usize>,
    entries: Vec<(SmolStr, ItemId)>,
}

impl<ItemId: fmt::Debug> fmt::Debug for NameIndex<ItemId> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("NameIndex").field("entries", &self.entries).finish()
    }
}

impl<ItemId: PartialEq> PartialEq for NameIndex<ItemId> {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl<ItemId: Eq> Eq for NameIndex<ItemId> {}

impl<ItemId> Default for NameIndex<ItemId> {
    fn default() -> Self {
        NameIndex { first: HashTable::default(), entries: Vec::default() }
    }
}

impl<ItemId> NameIndex<ItemId>
where
    ItemId: Copy + Eq,
{
    pub fn lookup(&self, name: &str) -> Option<ItemId> {
        let hash = FxBuildHasher.hash_one(name);
        self.first.find(hash, |&index| self.entries[index].0 == name).map(|&index| {
            let (_, id) = &self.entries[index];
            *id
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&SmolStr, ItemId)> {
        self.entries.iter().map(|(name, id)| (name, *id))
    }

    pub(crate) fn insert(&mut self, name: SmolStr, id: ItemId) -> Option<ItemId> {
        let hash = FxBuildHasher.hash_one(name.as_str());
        let existing =
            self.first.find(hash, |&index| self.entries[index].0 == name).map(|&index| {
                let (_, id) = &self.entries[index];
                *id
            });

        self.entries.push((name, id));

        if existing.is_none() {
            let index = self.entries.len() - 1;
            self.first.insert_unique(hash, index, |&index| {
                FxBuildHasher.hash_one(&self.entries[index].0)
            });
        }

        existing.filter(|existing| *existing != id)
    }
}

#[cfg(test)]
mod tests {
    use super::NameIndex;

    #[test]
    fn name_index_preserves_first_and_insertion_order() {
        let mut index = NameIndex::default();

        assert_eq!(index.insert("first".into(), 0), None);
        assert_eq!(index.insert("second".into(), 1), None);
        assert_eq!(index.insert("first".into(), 2), Some(0));
        assert_eq!(index.insert("first".into(), 0), None);

        assert_eq!(index.lookup("first"), Some(0));
        assert_eq!(index.lookup("second"), Some(1));
        assert_eq!(index.lookup("missing"), None);

        let entries: Vec<_> = index.iter().map(|(name, id)| (name.as_str(), id)).collect();
        assert_eq!(entries, [("first", 0), ("second", 1), ("first", 2), ("first", 0)]);
    }

    #[test]
    fn name_index_preserves_lookups_when_growing() {
        let mut index = NameIndex::default();

        for id in 0..1024 {
            let name = format!("name_{id}");
            assert_eq!(index.insert(name.into(), id), None);
        }

        for id in 0..1024 {
            let name = format!("name_{id}");
            assert_eq!(index.lookup(&name), Some(id));
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IndexedExports {
    pub terms: Vec<IndexedExport<TermItemId>>,
    pub types: Vec<IndexedTypeExport>,
    pub modules: Vec<IndexedModuleExport>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IndexedExport<ItemId> {
    pub id: ExportItemId,
    pub name: SmolStr,
    pub item: Option<ItemId>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IndexedTypeExport {
    pub id: ExportItemId,
    pub name: SmolStr,
    pub item: Option<TypeItemId>,
    pub selection: Option<TypeSelection>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IndexedModuleExport {
    pub id: ExportItemId,
    pub name: SmolStr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeSelection {
    Everything,
    Enumerated(Box<[SmolStr]>),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IndexedItems {
    terms: Arena<TermItem>,
    types: Arena<TypeItem>,
}

impl IndexedItems {
    pub fn iter_terms(&self) -> impl Iterator<Item = (TermItemId, &TermItem)> {
        self.terms.iter()
    }

    pub fn iter_types(&self) -> impl Iterator<Item = (TypeItemId, &TypeItem)> {
        self.types.iter()
    }
}

impl ops::Index<TermItemId> for IndexedItems {
    type Output = TermItem;

    fn index(&self, index: TermItemId) -> &TermItem {
        &self.terms[index]
    }
}

impl ops::Index<TypeItemId> for IndexedItems {
    type Output = TypeItem;

    fn index(&self, index: TypeItemId) -> &TypeItem {
        &self.types[index]
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum ExportKind {
    #[default]
    /// module Main where
    Implicit,
    /// module Main (value, Type, ...) where
    Explicit,
    /// module Main (module Main, ...) where
    ExplicitSelf,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    #[default]
    /// import Lib
    Implicit,
    /// import Lib (value, Type, ...)
    Explicit,
    /// import Lib hiding (value, Type, ...)
    Hidden,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImplicitItems {
    Everything,
    Enumerated(Box<[SmolStr]>),
}

pub type ImportedTerms = FxHashMap<SmolStr, ImportItemId>;
pub type ImportedTypes = FxHashMap<SmolStr, (ImportItemId, Option<ImplicitItems>)>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IndexedImport {
    pub name: Option<SmolStr>,
    pub alias: Option<SmolStr>,
    pub kind: ImportKind,
    pub terms: ImportedTerms,
    pub types: ImportedTypes,
    pub exported: bool,
}

pub type IndexedImports = FxHashMap<ImportId, IndexedImport>;

impl IndexedImport {
    pub(crate) fn new(name: Option<SmolStr>, alias: Option<SmolStr>) -> IndexedImport {
        IndexedImport { name, alias, ..Default::default() }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IndexedPairs {
    instance_chain: Vec<(InstanceChainId, InstanceId)>,
    instance_members: Vec<(InstanceId, InstanceMemberId)>,

    declaration_to_term: Vec<(DeclarationId, TermItemId)>,
    declaration_to_type: Vec<(DeclarationId, TypeItemId)>,
    constructor_to_term: Vec<(DataConstructorId, TermItemId)>,
    class_member_to_term: Vec<(ClassMemberId, TermItemId)>,
}

impl IndexedPairs {
    pub fn declaration_to_term(&self, id: DeclarationId) -> Option<TermItemId> {
        self.declaration_to_term.iter().find_map(move |(declaration_id, term_id)| {
            if *declaration_id == id { Some(*term_id) } else { None }
        })
    }

    pub fn declaration_to_type(&self, id: DeclarationId) -> Option<TypeItemId> {
        self.declaration_to_type.iter().find_map(move |(declaration_id, type_id)| {
            if *declaration_id == id { Some(*type_id) } else { None }
        })
    }

    pub fn constructor_to_term(&self, id: DataConstructorId) -> Option<TermItemId> {
        self.constructor_to_term.iter().find_map(move |(constructor_id, term_id)| {
            if *constructor_id == id { Some(*term_id) } else { None }
        })
    }

    pub fn class_member_to_term(&self, id: ClassMemberId) -> Option<TermItemId> {
        self.class_member_to_term.iter().find_map(move |(class_member_id, term_id)| {
            if *class_member_id == id { Some(*term_id) } else { None }
        })
    }

    pub fn instance_chain_id(&self, id: InstanceId) -> Option<InstanceChainId> {
        self.instance_chain.iter().find_map(
            |(chain_id, instance_id)| {
                if *instance_id == id { Some(*chain_id) } else { None }
            },
        )
    }

    pub fn instance_chain_position(&self, id: InstanceId) -> Option<u32> {
        let chain_of_id = self.instance_chain_id(id)?;
        self.instance_chain
            .iter()
            .filter(|(chain_id, _)| *chain_id == chain_of_id)
            .position(|(_, instance_id)| *instance_id == id)
            .map(|position| position as u32)
    }
}

pub fn index_module(cst: &cst::Module, stabilized: &StabilizedModule) -> IndexedModule {
    let algorithm::State { kind, names, exports, items, imports, pairs, errors, .. } =
        algorithm::index_module(cst, stabilized);
    IndexedModule { kind, names, exports, items, imports, pairs, errors }
}
