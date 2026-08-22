use std::sync::Arc;

use files::FileId;
use rustc_hash::FxHashSet;

use super::LifecycleWarning;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AnalysisInvalidation {
    #[default]
    None,
    Sources(FxHashSet<FileId>),
    Workspace,
}

impl AnalysisInvalidation {
    fn source(file_id: FileId) -> AnalysisInvalidation {
        let mut sources = FxHashSet::default();
        sources.insert(file_id);
        AnalysisInvalidation::Sources(sources)
    }

    pub(super) fn include_source(&mut self, file_id: FileId) {
        match self {
            AnalysisInvalidation::None => {
                *self = AnalysisInvalidation::source(file_id);
            }
            AnalysisInvalidation::Sources(sources) => {
                sources.insert(file_id);
            }
            AnalysisInvalidation::Workspace => {}
        }
    }

    fn combine(&mut self, other: AnalysisInvalidation) {
        match other {
            AnalysisInvalidation::None => {}
            AnalysisInvalidation::Sources(other_sources) => {
                for file_id in other_sources {
                    self.include_source(file_id);
                }
            }
            AnalysisInvalidation::Workspace => *self = AnalysisInvalidation::Workspace,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovedSource {
    pub file_id: FileId,
    pub locator: Arc<str>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifecycleChange {
    pub(super) analysis: AnalysisInvalidation,
    changed_sources: FxHashSet<FileId>,
    pub(super) removed_sources: Vec<RemovedSource>,
    pub(super) warnings: Vec<LifecycleWarning>,
}

impl LifecycleChange {
    pub fn analysis(&self) -> &AnalysisInvalidation {
        &self.analysis
    }

    pub fn changed_sources(&self) -> impl Iterator<Item = FileId> + '_ {
        self.changed_sources.iter().copied()
    }

    pub fn removed_sources(&self) -> &[RemovedSource] {
        &self.removed_sources
    }

    pub fn warnings(&self) -> &[LifecycleWarning] {
        &self.warnings
    }

    pub fn combine(&mut self, other: LifecycleChange) {
        self.analysis.combine(other.analysis);
        self.changed_sources.extend(other.changed_sources);
        self.removed_sources.extend(other.removed_sources);
        self.warnings.extend(other.warnings);
    }

    pub(super) fn source_changed(&mut self, file_id: FileId, content_changed: bool) {
        if content_changed {
            self.analysis = AnalysisInvalidation::Workspace;
        } else {
            self.analysis.include_source(file_id);
        }
        self.changed_sources.insert(file_id);
    }

    pub(super) fn foreign_changed(&mut self, source_id: Option<FileId>) {
        if let Some(source_id) = source_id {
            self.analysis.include_source(source_id);
            self.changed_sources.insert(source_id);
        }
    }
}
