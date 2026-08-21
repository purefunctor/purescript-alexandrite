use core::str;
use std::sync::Arc;

use indexmap::IndexMap;
use la_arena::{Idx, RawIdx};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rustc_hash::{FxBuildHasher, FxHashMap};

#[derive(Debug, Default)]
pub struct Files {
    files: IndexMap<Arc<str>, Arc<str>, FxBuildHasher>,
}

pub struct File;

pub type FileId = Idx<File>;

#[derive(Debug, Default)]
pub struct ForeignFiles {
    paths: FxHashMap<Arc<str>, ForeignFileId>,
    files: Vec<ForeignFileSlot>,
    free: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ForeignFileId {
    index: u32,
    generation: u32,
}

#[derive(Debug)]
struct ForeignFileSlot {
    generation: u32,
    file: Option<ForeignFileData>,
}

#[derive(Debug)]
struct ForeignFileData {
    path: Arc<str>,
    content: Arc<str>,
}

impl Files {
    pub fn insert(&mut self, k: impl Into<Arc<str>>, v: impl Into<Arc<str>>) -> FileId {
        let k = k.into();
        let v = v.into();
        let (index, _) = self.files.insert_full(k, v);
        Idx::from_raw(RawIdx::from_u32(index as u32))
    }

    pub fn id(&self, k: &str) -> Option<FileId> {
        self.files.get_full(k).map(|(index, _, _)| Idx::from_raw(RawIdx::from_u32(index as u32)))
    }

    pub fn path(&self, id: FileId) -> Arc<str> {
        let index = id.into_raw().into_u32() as usize;
        let (path, _) =
            self.files.get_index(index).expect("invariant violated: expected valid FileId");
        Arc::clone(path)
    }

    pub fn content(&self, id: FileId) -> Arc<str> {
        let index = id.into_raw().into_u32() as usize;
        let (_, contents) =
            self.files.get_index(index).expect("invariant violated: expected valid FileId");
        Arc::clone(contents)
    }

    pub fn iter_id(&self) -> impl Iterator<Item = FileId> + use<> {
        let length = self.files.len();
        (0..length).map(|index| {
            let index = RawIdx::from_u32(index as u32);
            Idx::from_raw(index)
        })
    }

    pub fn par_iter_id(&self) -> impl ParallelIterator<Item = FileId> {
        let length = self.files.len();
        (0..length).into_par_iter().map(|index| {
            let index = RawIdx::from_u32(index as u32);
            Idx::from_raw(index)
        })
    }
}

impl ForeignFiles {
    pub fn insert(
        &mut self,
        path: impl Into<Arc<str>>,
        content: impl Into<Arc<str>>,
    ) -> ForeignFileId {
        let path = path.into();
        let content = content.into();
        if let Some(id) = self.paths.get(path.as_ref()).copied() {
            let file = self.file_mut(id);
            file.content = content;
            return id;
        }

        let file = ForeignFileData { path: Arc::clone(&path), content };
        let id = if let Some(index) = self.free.pop() {
            let slot = &mut self.files[index as usize];
            slot.file = Some(file);
            ForeignFileId { index, generation: slot.generation }
        } else {
            let index = u32::try_from(self.files.len())
                .expect("invariant violated: too many foreign files");
            let slot = ForeignFileSlot { generation: 0, file: Some(file) };
            self.files.push(slot);
            ForeignFileId { index, generation: 0 }
        };
        self.paths.insert(path, id);
        id
    }

    pub fn id(&self, path: &str) -> Option<ForeignFileId> {
        self.paths.get(path).copied()
    }

    pub fn path(&self, id: ForeignFileId) -> Arc<str> {
        let file = self.file(id);
        Arc::clone(&file.path)
    }

    pub fn content(&self, id: ForeignFileId) -> Arc<str> {
        let file = self.file(id);
        Arc::clone(&file.content)
    }

    pub fn remove(&mut self, path: &str) -> Option<ForeignFileId> {
        let id = self.paths.remove(path)?;
        let slot = &mut self.files[id.index as usize];
        assert_eq!(
            slot.generation, id.generation,
            "invariant violated: expected valid ForeignFileId"
        );
        slot.file.take().expect("invariant violated: expected valid ForeignFileId");
        slot.generation = slot
            .generation
            .checked_add(1)
            .expect("invariant violated: exhausted ForeignFileId generations");
        self.free.push(id.index);
        Some(id)
    }

    fn file(&self, id: ForeignFileId) -> &ForeignFileData {
        let slot = &self.files[id.index as usize];
        assert_eq!(
            slot.generation, id.generation,
            "invariant violated: expected valid ForeignFileId"
        );
        slot.file.as_ref().expect("invariant violated: expected valid ForeignFileId")
    }

    fn file_mut(&mut self, id: ForeignFileId) -> &mut ForeignFileData {
        let slot = &mut self.files[id.index as usize];
        assert_eq!(
            slot.generation, id.generation,
            "invariant violated: expected valid ForeignFileId"
        );
        slot.file.as_mut().expect("invariant violated: expected valid ForeignFileId")
    }
}

#[cfg(test)]
mod tests {
    use super::{Files, ForeignFiles};

    #[test]
    fn test_basic() {
        let mut files = Files::default();

        let k = "src/Main.purs";
        let v = "module Main where\n\n";

        let id = files.insert(k, v);

        assert_eq!(files.id(k), Some(id));
        assert_eq!(files.path(id).as_ref(), k);
        assert_eq!(files.content(id).as_ref(), v);
    }

    #[test]
    fn test_foreign_files() {
        let mut files = ForeignFiles::default();

        let path = "src/Main.js";
        let content = "export const life = 42;\n";
        let id = files.insert(path, content);

        assert_eq!(files.id(path), Some(id));
        assert_eq!(files.path(id).as_ref(), path);
        assert_eq!(files.content(id).as_ref(), content);

        let retained_path = "src/Retained.js";
        let retained_id = files.insert(retained_path, "export const retained = 1;");

        assert_eq!(files.remove(path), Some(id));
        assert_eq!(files.id(path), None);
        assert!(files.files[id.index as usize].file.is_none());
        assert_eq!(files.path(retained_id).as_ref(), retained_path);

        let replacement_id = files.insert(path, content);
        assert_ne!(replacement_id, id);
        assert_eq!(files.remove(path), Some(replacement_id));

        let slots = files.files.len();
        for number in 0..32 {
            let path = format!("src/Temporary-{number}.js");
            let temporary_id = files.insert(path.as_str(), content);
            assert_eq!(files.remove(&path), Some(temporary_id));
        }
        assert_eq!(files.files.len(), slots);
    }
}
