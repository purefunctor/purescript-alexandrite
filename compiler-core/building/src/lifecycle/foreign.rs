use std::sync::Arc;

use files::{ForeignFileId, ForeignSourceKind};

use super::*;
use crate::QueryEngine;

impl<Version, Metadata> FileLifecycle<Version, Metadata>
where
    Version: Clone + Ord,
{
    pub(super) fn apply_foreign(
        &mut self,
        engine: &QueryEngine,
        unit: SourceUnitKey,
        kind: ForeignSourceKind,
        event: ForeignEvent<Version>,
    ) -> LifecycleChange {
        let mut source_unit = self.units.remove(&unit).unwrap_or_default();
        let result = self.apply_foreign_event(engine, &unit, kind, &mut source_unit, event);
        self.store_unit(unit, source_unit);
        result
    }

    fn apply_foreign_event(
        &mut self,
        engine: &QueryEngine,
        unit: &SourceUnitKey,
        kind: ForeignSourceKind,
        source_unit: &mut SourceUnit<Version, Metadata>,
        event: ForeignEvent<Version>,
    ) -> LifecycleChange {
        let mut change = LifecycleChange::default();
        let current = std::mem::take(source_unit.foreign.get_mut(kind));
        let next = match (current, event) {
            (Member::Missing, ForeignEvent::Opened { text, version }) => {
                let document = self.insert_foreign(engine, unit, kind, source_unit, &text);
                change.foreign_changed(source_unit.source_id());
                Member::Present(ForeignDocument {
                    id: document.id,
                    content: EffectiveContent::Open { text, version },
                })
            }
            (Member::Missing, ForeignEvent::DiskObserved { disk }) => match disk {
                DiskObservation::Found(text) => {
                    let document = self.insert_foreign(engine, unit, kind, source_unit, &text);
                    change.foreign_changed(source_unit.source_id());
                    Member::Present(document)
                }
                DiskObservation::NotFound => Member::Missing,
                DiskObservation::Failed(failure) => {
                    change.warnings.push(LifecycleWarning::ReloadFailed {
                        unit: SourceUnitKey::clone(unit),
                        document: DocumentKind::Foreign(kind),
                        failure,
                    });
                    Member::Missing
                }
            },
            (Member::Missing, ForeignEvent::Changed { .. }) => {
                change.warnings.push(LifecycleWarning::ChangedNonOpen {
                    unit: SourceUnitKey::clone(unit),
                    document: DocumentKind::Foreign(kind),
                });
                Member::Missing
            }
            (Member::Missing, ForeignEvent::Closed { .. }) => {
                change.warnings.push(LifecycleWarning::ClosedNonOpen {
                    unit: SourceUnitKey::clone(unit),
                    document: DocumentKind::Foreign(kind),
                });
                Member::Missing
            }
            (Member::Present(mut document), ForeignEvent::Opened { text, version }) => {
                self.set_foreign_content(engine, document.id, &text);
                document.content = EffectiveContent::Open { text, version };
                change.foreign_changed(source_unit.source_id());
                Member::Present(document)
            }
            (Member::Present(mut document), ForeignEvent::Changed { text, version }) => {
                match &document.content {
                    EffectiveContent::Open { version: current_version, .. }
                        if version > *current_version =>
                    {
                        self.set_foreign_content(engine, document.id, &text);
                        document.content = EffectiveContent::Open { text, version };
                        change.foreign_changed(source_unit.source_id());
                    }
                    EffectiveContent::Open { .. } => {
                        change.warnings.push(LifecycleWarning::StaleChange {
                            unit: SourceUnitKey::clone(unit),
                            document: DocumentKind::Foreign(kind),
                        });
                    }
                    EffectiveContent::Disk { .. } | EffectiveContent::Retained { .. } => {
                        change.warnings.push(LifecycleWarning::ChangedNonOpen {
                            unit: SourceUnitKey::clone(unit),
                            document: DocumentKind::Foreign(kind),
                        });
                    }
                }
                Member::Present(document)
            }
            (Member::Present(document), ForeignEvent::Closed { disk }) => {
                if !matches!(document.content, EffectiveContent::Open { .. }) {
                    change.warnings.push(LifecycleWarning::ClosedNonOpen {
                        unit: SourceUnitKey::clone(unit),
                        document: DocumentKind::Foreign(kind),
                    });
                    Member::Present(document)
                } else {
                    self.reconcile_foreign(
                        engine,
                        unit,
                        kind,
                        source_unit,
                        document,
                        disk,
                        &mut change,
                    )
                }
            }
            (Member::Present(document), ForeignEvent::DiskObserved { disk }) => {
                if matches!(document.content, EffectiveContent::Open { .. }) {
                    change.warnings.push(LifecycleWarning::DiskObservedWhileOpen {
                        unit: SourceUnitKey::clone(unit),
                        document: DocumentKind::Foreign(kind),
                    });
                    Member::Present(document)
                } else {
                    self.reconcile_foreign(
                        engine,
                        unit,
                        kind,
                        source_unit,
                        document,
                        disk,
                        &mut change,
                    )
                }
            }
        };
        *source_unit.foreign.get_mut(kind) = next;
        change
    }

    fn reconcile_foreign(
        &mut self,
        engine: &QueryEngine,
        unit: &SourceUnitKey,
        kind: ForeignSourceKind,
        source_unit: &SourceUnit<Version, Metadata>,
        mut document: ForeignDocument<Version>,
        disk: DiskObservation,
        change: &mut LifecycleChange,
    ) -> Member<ForeignDocument<Version>> {
        match disk {
            DiskObservation::Found(text) => {
                self.set_foreign_content(engine, document.id, &text);
                document.content = EffectiveContent::Disk { text };
                change.foreign_changed(source_unit.source_id());
                Member::Present(document)
            }
            DiskObservation::NotFound => {
                self.remove_foreign(engine, unit, kind, document.id);
                change.foreign_changed(source_unit.source_id());
                Member::Missing
            }
            DiskObservation::Failed(failure) => {
                let text = Arc::clone(document.content.text());
                let retained_failure = ReloadFailure::clone(&failure);
                document.content = EffectiveContent::Retained { text, failure: retained_failure };
                change.foreign_changed(source_unit.source_id());
                change.warnings.push(LifecycleWarning::ReloadFailed {
                    unit: SourceUnitKey::clone(unit),
                    document: DocumentKind::Foreign(kind),
                    failure,
                });
                Member::Present(document)
            }
        }
    }

    fn insert_foreign(
        &mut self,
        engine: &QueryEngine,
        unit: &SourceUnitKey,
        kind: ForeignSourceKind,
        source_unit: &SourceUnit<Version, Metadata>,
        text: &Arc<str>,
    ) -> ForeignDocument<Version> {
        let id =
            self.foreign_files.insert(kind, Arc::from(unit.foreign_for(kind)), Arc::clone(text));
        engine.set_foreign_content(id, Arc::clone(text));
        if let Some(source_id) = source_unit.source_id() {
            engine.set_foreign_file(source_id, id);
        }
        ForeignDocument { id, content: EffectiveContent::Disk { text: Arc::clone(text) } }
    }

    fn set_foreign_content(&mut self, engine: &QueryEngine, id: ForeignFileId, text: &Arc<str>) {
        let previous_content = self.foreign_files.content(id);
        if previous_content == *text {
            return;
        }
        let path = self.foreign_files.path(id);
        let inserted_id = self.foreign_files.insert(id.kind(), path, Arc::clone(text));
        debug_assert_eq!(inserted_id, id);
        engine.set_foreign_content(id, Arc::clone(text));
    }

    fn remove_foreign(
        &mut self,
        engine: &QueryEngine,
        unit: &SourceUnitKey,
        kind: ForeignSourceKind,
        id: ForeignFileId,
    ) {
        engine.remove_foreign_file(id);
        let removed_id = self.foreign_files.remove(unit.foreign_for(kind));
        debug_assert_eq!(removed_id, Some(id));
    }
}
