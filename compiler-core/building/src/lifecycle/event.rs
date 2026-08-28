use core::fmt;
use std::io;
use std::sync::Arc;

use files::ForeignSourceKind;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceUnitKey {
    pub(super) source: Arc<str>,
    pub(super) javascript: Arc<str>,
    pub(super) jsx: Arc<str>,
}

impl SourceUnitKey {
    pub fn new(source: impl Into<Arc<str>>, javascript: impl Into<Arc<str>>) -> SourceUnitKey {
        let javascript = javascript.into();
        let jsx = if let Some(stem) = javascript.strip_suffix(".js") {
            format!("{stem}.jsx").into()
        } else {
            panic!("invariant violated: JavaScript foreign locator must end in .js")
        };
        SourceUnitKey { source: source.into(), javascript, jsx }
    }

    pub fn with_foreign_sources(
        source: impl Into<Arc<str>>,
        javascript: impl Into<Arc<str>>,
        jsx: impl Into<Arc<str>>,
    ) -> SourceUnitKey {
        SourceUnitKey { source: source.into(), javascript: javascript.into(), jsx: jsx.into() }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn foreign(&self) -> &str {
        &self.javascript
    }

    pub fn foreign_for(&self, kind: ForeignSourceKind) -> &str {
        match kind {
            ForeignSourceKind::JavaScript => &self.javascript,
            ForeignSourceKind::Jsx => &self.jsx,
        }
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
    Foreign { unit: SourceUnitKey, kind: ForeignSourceKind, event: ForeignEvent<Version> },
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
    Foreign(SourceUnitKey, ForeignSourceKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentKind {
    Source,
    Foreign(ForeignSourceKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentAuthority {
    Open,
    Disk,
    Retained,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleWarning {
    LocatorAlreadyOwned { locator: Arc<str>, owner: SourceUnitKey, requested: SourceUnitKey },
    ChangedNonOpen { unit: SourceUnitKey, document: DocumentKind },
    ClosedNonOpen { unit: SourceUnitKey, document: DocumentKind },
    StaleChange { unit: SourceUnitKey, document: DocumentKind },
    DiskObservedWhileOpen { unit: SourceUnitKey, document: DocumentKind },
    ReloadFailed { unit: SourceUnitKey, document: DocumentKind, failure: ReloadFailure },
}

impl fmt::Display for LifecycleWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleWarning::LocatorAlreadyOwned { locator, owner, requested } => {
                write!(
                    formatter,
                    "ignored lifecycle event for {} because {locator} already belongs to {}",
                    requested.source(),
                    owner.source(),
                )
            }
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
