use std::sync::Arc;

use super::{
    AnalysisInvalidation, ContentAuthority, DiskObservation, DocumentKey, FileLifecycle,
    ForeignEvent, LifecycleEvent, ReloadFailure, SourceEvent, SourceUnitKey,
};
use crate::QueryEngine;

fn unit() -> SourceUnitKey {
    SourceUnitKey::new("file:///src/Main.purs", "file:///src/Main.js")
}

fn text(content: &str) -> Arc<str> {
    Arc::from(content)
}

fn source_opened(version: i32, content: &str) -> LifecycleEvent<i32, bool> {
    LifecycleEvent::Source {
        unit: unit(),
        event: SourceEvent::Opened { text: text(content), version, metadata: true },
    }
}

fn source_disk(content: &str) -> LifecycleEvent<i32, bool> {
    LifecycleEvent::Source {
        unit: unit(),
        event: SourceEvent::DiskObserved {
            disk: DiskObservation::Found(text(content)),
            metadata: true,
        },
    }
}

fn foreign_opened(version: i32, content: &str) -> LifecycleEvent<i32, bool> {
    LifecycleEvent::Foreign {
        unit: unit(),
        event: ForeignEvent::Opened { text: text(content), version },
    }
}

#[test]
fn open_source_ignores_disk_events_and_rejects_stale_changes() {
    let engine = QueryEngine::default();
    let mut files = FileLifecycle::default();
    let content = "module Main where\n";
    files.apply(&engine, source_opened(2, content)).unwrap();
    let file_id = files.source_id(unit().source()).unwrap();

    let event = LifecycleEvent::Source {
        unit: unit(),
        event: SourceEvent::DiskObserved { disk: DiskObservation::NotFound, metadata: true },
    };
    files.apply(&engine, event).unwrap();
    assert_eq!(files.source_id(unit().source()), Some(file_id));
    assert_eq!(files.source_authority(&unit()), Some(ContentAuthority::Open));

    let event = LifecycleEvent::Source {
        unit: unit(),
        event: SourceEvent::Changed { text: text("module Newer where\n"), version: 1 },
    };
    files.apply(&engine, event).unwrap();
    assert_eq!(engine.content(file_id).unwrap().as_ref(), content);
}

#[test]
fn source_close_reloads_retains_or_removes() {
    let engine = QueryEngine::default();
    let mut files = FileLifecycle::default();
    files.apply(&engine, source_opened(1, "module Main where\n")).unwrap();
    let file_id = files.source_id(unit().source()).unwrap();

    let event = LifecycleEvent::Source {
        unit: unit(),
        event: SourceEvent::Closed {
            disk: DiskObservation::Failed(ReloadFailure::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            )),
        },
    };
    files.apply(&engine, event).unwrap();
    assert_eq!(files.source_authority(&unit()), Some(ContentAuthority::Retained));
    assert_eq!(engine.content(file_id).unwrap().as_ref(), "module Main where\n");

    files.apply(&engine, source_opened(2, "module Main where\n")).unwrap();
    let event = LifecycleEvent::Source {
        unit: unit(),
        event: SourceEvent::Closed { disk: DiskObservation::NotFound },
    };
    let change = files.apply(&engine, event).unwrap();
    assert_eq!(files.source_id(unit().source()), None);
    assert_eq!(change.removed_sources()[0].file_id, file_id);
    assert!(matches!(change.analysis(), AnalysisInvalidation::Workspace));
}

#[test]
fn removed_and_recreated_source_gets_new_identity() {
    let engine = QueryEngine::default();
    let mut files = FileLifecycle::default();
    files.apply(&engine, source_disk("module Main where\n")).unwrap();
    let removed_id = files.source_id(unit().source()).unwrap();

    let event = LifecycleEvent::Source {
        unit: unit(),
        event: SourceEvent::DiskObserved { disk: DiskObservation::NotFound, metadata: true },
    };
    files.apply(&engine, event).unwrap();
    files.apply(&engine, source_disk("module Main where\n")).unwrap();
    let recreated_id = files.source_id(unit().source()).unwrap();
    assert!(recreated_id > removed_id);
}

#[test]
fn foreign_only_unit_associates_when_source_appears() {
    let engine = QueryEngine::default();
    let mut files = FileLifecycle::default();
    files.apply(&engine, foreign_opened(1, "export const life = 42;\n")).unwrap();
    let foreign_id = files.foreign_id(unit().foreign()).unwrap();
    assert_eq!(files.foreign_authority(&unit()), Some(ContentAuthority::Open));

    files.apply(&engine, source_disk("module Main where\n")).unwrap();
    let source_id = files.source_id(unit().source()).unwrap();
    assert_eq!(engine.foreign_file(source_id), Some(foreign_id));
}

#[test]
fn open_foreign_ignores_disk_deletion_until_close() {
    let engine = QueryEngine::default();
    let mut files = FileLifecycle::default();
    files.apply(&engine, source_disk("module Main where\n")).unwrap();
    files.apply(&engine, foreign_opened(1, "export const life = 42;\n")).unwrap();
    let foreign_id = files.foreign_id(unit().foreign()).unwrap();

    let event = LifecycleEvent::Foreign {
        unit: unit(),
        event: ForeignEvent::DiskObserved { disk: DiskObservation::NotFound },
    };
    files.apply(&engine, event).unwrap();
    assert_eq!(files.foreign_id(unit().foreign()), Some(foreign_id));
    assert!(files.is_open(&DocumentKey::Foreign(unit())));

    let event = LifecycleEvent::Foreign {
        unit: unit(),
        event: ForeignEvent::Closed { disk: DiskObservation::NotFound },
    };
    files.apply(&engine, event).unwrap();
    assert_eq!(files.foreign_id(unit().foreign()), None);
}
