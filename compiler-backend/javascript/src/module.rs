use std::sync::Arc;

use files::FileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    file_id: FileId,
    name: Arc<str>,
    source: Arc<str>,
    dependencies: Arc<[FileId]>,
    requires_foreign: bool,
}

impl Module {
    pub(crate) fn new(
        file_id: FileId,
        name: String,
        source: String,
        dependencies: Vec<FileId>,
        requires_foreign: bool,
    ) -> Module {
        Module {
            file_id,
            name: name.into(),
            source: source.into(),
            dependencies: dependencies.into(),
            requires_foreign,
        }
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn filename(&self) -> String {
        module_filename(&self.name)
    }

    pub fn foreign_filename(&self) -> String {
        foreign_module_filename(&self.name)
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn dependencies(&self) -> &[FileId] {
        &self.dependencies
    }

    pub fn requires_foreign(&self) -> bool {
        self.requires_foreign
    }
}

pub fn module_filename(module_name: &str) -> String {
    format!("{module_name}/index.js")
}

pub fn foreign_module_filename(module_name: &str) -> String {
    format!("{module_name}/foreign.js")
}
