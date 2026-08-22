use core::fmt;
use std::io;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceUnitKey {
    pub(super) source: Arc<str>,
    pub(super) foreign: Arc<str>,
}

impl SourceUnitKey {
    pub fn new(source: impl Into<Arc<str>>, foreign: impl Into<Arc<str>>) -> SourceUnitKey {
        SourceUnitKey { source: source.into(), foreign: foreign.into() }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn foreign(&self) -> &str {
        &self.foreign
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReloadFailure {
    kind: io::ErrorKind,
    message: Arc<str>,
}

impl ReloadFailure {
    pub fn new(kind: io::ErrorKind, message: impl Into<Arc<str>>) -> ReloadFailure {
        ReloadFailure { kind, message: message.into() }
    }

    pub fn kind(&self) -> io::ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ReloadFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.kind, self.message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiskObservation {
    Found(Arc<str>),
    NotFound,
    Failed(ReloadFailure),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleEvent<Version, Metadata> {
    Source { unit: SourceUnitKey, event: SourceEvent<Version, Metadata> },
    Foreign { unit: SourceUnitKey, event: ForeignEvent<Version> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceEvent<Version, Metadata> {
    Opened { text: Arc<str>, version: Version, metadata: Metadata },
    Changed { text: Arc<str>, version: Version },
    Closed { disk: DiskObservation },
    DiskObserved { disk: DiskObservation, metadata: Metadata },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForeignEvent<Version> {
    Opened { text: Arc<str>, version: Version },
    Changed { text: Arc<str>, version: Version },
    Closed { disk: DiskObservation },
    DiskObserved { disk: DiskObservation },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentKey {
    Source(SourceUnitKey),
    Foreign(SourceUnitKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentKind {
    Source,
    Foreign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentAuthority {
    Open,
    Disk,
    Retained,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleWarning {
    ChangedNonOpen { unit: SourceUnitKey, document: DocumentKind },
    ClosedNonOpen { unit: SourceUnitKey, document: DocumentKind },
    StaleChange { unit: SourceUnitKey, document: DocumentKind },
    DiskObservedWhileOpen { unit: SourceUnitKey, document: DocumentKind },
    ReloadFailed { unit: SourceUnitKey, document: DocumentKind, failure: ReloadFailure },
}

impl fmt::Display for LifecycleWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleWarning::ChangedNonOpen { unit, document } => {
                write!(formatter, "ignored change for non-open {document:?} {}", unit.source())
            }
            LifecycleWarning::ClosedNonOpen { unit, document } => {
                write!(formatter, "ignored close for non-open {document:?} {}", unit.source())
            }
            LifecycleWarning::StaleChange { unit, document } => {
                write!(formatter, "ignored stale change for {document:?} {}", unit.source())
            }
            LifecycleWarning::DiskObservedWhileOpen { unit, document } => {
                write!(formatter, "ignored disk event for open {document:?} {}", unit.source())
            }
            LifecycleWarning::ReloadFailed { unit, document, failure } => {
                write!(formatter, "failed to reload {document:?} {}: {failure}", unit.source())
            }
        }
    }
}
