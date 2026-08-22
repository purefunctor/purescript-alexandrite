use std::sync::Arc;

use files::{FileId, Files, ForeignFileId, ForeignFiles};
use rustc_hash::FxHashMap;

use crate::QueryEngine;

mod change;
mod event;
mod foreign;
mod source;

#[cfg(test)]
mod tests;

pub use change::*;
pub use event::*;

#[derive(Debug)]
pub struct FileLifecycle<Version, Metadata> {
    units: FxHashMap<SourceUnitKey, SourceUnit<Version, Metadata>>,
    source_units: FxHashMap<FileId, SourceUnitKey>,
    source_files: Files,
    foreign_files: ForeignFiles,
}

impl<Version, Metadata> Default for FileLifecycle<Version, Metadata> {
    fn default() -> FileLifecycle<Version, Metadata> {
        FileLifecycle {
            units: FxHashMap::default(),
            source_units: FxHashMap::default(),
            source_files: Files::default(),
            foreign_files: ForeignFiles::default(),
        }
    }
}

#[derive(Debug)]
struct SourceUnit<Version, Metadata> {
    source: Member<SourceDocument<Version, Metadata>>,
    foreign: Member<ForeignDocument<Version>>,
}

impl<Version, Metadata> Default for SourceUnit<Version, Metadata> {
    fn default() -> SourceUnit<Version, Metadata> {
        SourceUnit { source: Member::Missing, foreign: Member::Missing }
    }
}

#[derive(Debug, Default)]
enum Member<Document> {
    #[default]
    Missing,
    Present(Document),
}

#[derive(Debug)]
struct SourceDocument<Version, Metadata> {
    id: FileId,
    metadata: Metadata,
    content: EffectiveContent<Version>,
}

#[derive(Debug)]
struct ForeignDocument<Version> {
    id: ForeignFileId,
    content: EffectiveContent<Version>,
}

#[derive(Debug)]
enum EffectiveContent<Version> {
    Open { text: Arc<str>, version: Version },
    Disk { text: Arc<str> },
    Retained { text: Arc<str>, failure: ReloadFailure },
}

impl<Version> EffectiveContent<Version> {
    fn authority(&self) -> ContentAuthority {
        match self {
            EffectiveContent::Open { .. } => ContentAuthority::Open,
            EffectiveContent::Disk { .. } => ContentAuthority::Disk,
            EffectiveContent::Retained { .. } => ContentAuthority::Retained,
        }
    }

    fn text(&self) -> &Arc<str> {
        match self {
            EffectiveContent::Open { text, .. }
            | EffectiveContent::Disk { text }
            | EffectiveContent::Retained { text, .. } => text,
        }
    }

    fn reload_failure(&self) -> Option<&ReloadFailure> {
        let EffectiveContent::Retained { failure, .. } = self else {
            return None;
        };
        Some(failure)
    }
}

impl<Version, Metadata> FileLifecycle<Version, Metadata>
where
    Version: Clone + Ord,
{
    pub fn apply(
        &mut self,
        engine: &QueryEngine,
        event: LifecycleEvent<Version, Metadata>,
    ) -> LifecycleChange {
        match event {
            LifecycleEvent::Source { unit, event } => self.apply_source(engine, unit, event),
            LifecycleEvent::Foreign { unit, event } => self.apply_foreign(engine, unit, event),
        }
    }

    pub fn is_open(&self, document: &DocumentKey) -> bool {
        let (unit, kind) = match document {
            DocumentKey::Source(unit) => (unit, DocumentKind::Source),
            DocumentKey::Foreign(unit) => (unit, DocumentKind::Foreign),
        };
        let Some(source_unit) = self.units.get(unit) else {
            return false;
        };
        match kind {
            DocumentKind::Source => match &source_unit.source {
                Member::Missing => false,
                Member::Present(document) => document.content.authority() == ContentAuthority::Open,
            },
            DocumentKind::Foreign => match &source_unit.foreign {
                Member::Missing => false,
                Member::Present(document) => document.content.authority() == ContentAuthority::Open,
            },
        }
    }

    pub fn source_id(&self, locator: &str) -> Option<FileId> {
        self.source_files.id(locator)
    }

    pub fn contains_source(&self, file_id: FileId) -> bool {
        self.source_files.contains(file_id)
    }

    pub fn source_path(&self, file_id: FileId) -> Option<Arc<str>> {
        self.source_files.contains(file_id).then(|| self.source_files.path(file_id))
    }

    pub fn source_ids(&self) -> impl Iterator<Item = FileId> + '_ {
        self.source_files.iter_id()
    }

    pub fn source_metadata(&self, file_id: FileId) -> Option<&Metadata> {
        let unit = self.source_units.get(&file_id)?;
        let source_unit = self.units.get(unit)?;
        let Member::Present(source) = &source_unit.source else {
            return None;
        };
        Some(&source.metadata)
    }

    pub fn source_version(&self, file_id: FileId) -> Option<Version> {
        let unit = self.source_units.get(&file_id)?;
        let source_unit = self.units.get(unit)?;
        let Member::Present(source) = &source_unit.source else {
            return None;
        };
        let EffectiveContent::Open { version, .. } = &source.content else {
            return None;
        };
        Some(Version::clone(version))
    }

    pub fn source_authority(&self, unit: &SourceUnitKey) -> Option<ContentAuthority> {
        let source_unit = self.units.get(unit)?;
        let Member::Present(source) = &source_unit.source else {
            return None;
        };
        Some(source.content.authority())
    }

    pub fn source_reload_failure(&self, unit: &SourceUnitKey) -> Option<&ReloadFailure> {
        let source_unit = self.units.get(unit)?;
        let Member::Present(source) = &source_unit.source else {
            return None;
        };
        source.content.reload_failure()
    }

    pub fn foreign_authority(&self, unit: &SourceUnitKey) -> Option<ContentAuthority> {
        let source_unit = self.units.get(unit)?;
        let Member::Present(foreign) = &source_unit.foreign else {
            return None;
        };
        Some(foreign.content.authority())
    }

    pub fn foreign_reload_failure(&self, unit: &SourceUnitKey) -> Option<&ReloadFailure> {
        let source_unit = self.units.get(unit)?;
        let Member::Present(foreign) = &source_unit.foreign else {
            return None;
        };
        foreign.content.reload_failure()
    }

    pub fn foreign_id(&self, locator: &str) -> Option<ForeignFileId> {
        self.foreign_files.id(locator)
    }
}

impl<Version, Metadata> SourceUnit<Version, Metadata> {
    fn is_missing(&self) -> bool {
        matches!(self.source, Member::Missing) && matches!(self.foreign, Member::Missing)
    }

    fn source_id(&self) -> Option<FileId> {
        let Member::Present(source) = &self.source else {
            return None;
        };
        Some(source.id)
    }

    fn foreign_id(&self) -> Option<ForeignFileId> {
        let Member::Present(foreign) = &self.foreign else {
            return None;
        };
        Some(foreign.id)
    }
}
