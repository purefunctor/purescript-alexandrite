use std::sync::Arc;

use files::FileId;

use super::*;
use crate::QueryEngine;

impl<Version, Metadata> FileLifecycle<Version, Metadata>
where
    Version: Clone + Ord,
{
    pub(super) fn apply_source(
        &mut self,
        engine: &QueryEngine,
        unit: SourceUnitKey,
        event: SourceEvent<Version, Metadata>,
    ) -> LifecycleChange {
        let mut source_unit = self.units.remove(&unit).unwrap_or_default();
        let result = self.apply_source_event(engine, &unit, &mut source_unit, event);
        if !source_unit.is_missing() {
            self.units.insert(unit, source_unit);
        }
        result
    }

    fn apply_source_event(
        &mut self,
        engine: &QueryEngine,
        unit: &SourceUnitKey,
        source_unit: &mut SourceUnit<Version, Metadata>,
        event: SourceEvent<Version, Metadata>,
    ) -> LifecycleChange {
        let mut change = LifecycleChange::default();
        let current = std::mem::take(&mut source_unit.source);
        let next = match (current, event) {
            (Member::Missing, SourceEvent::Opened { text, version, metadata }) => {
                let document = self.insert_source(engine, unit, source_unit, text, metadata);
                change.source_changed(document.id, true);
                Member::Present(SourceDocument {
                    content: EffectiveContent::Open {
                        text: self.source_files.content(document.id),
                        version,
                    },
                    ..document
                })
            }
            (Member::Missing, SourceEvent::DiskObserved { disk, metadata }) => match disk {
                DiskObservation::Found(text) => {
                    let document = self.insert_source(engine, unit, source_unit, text, metadata);
                    change.source_changed(document.id, true);
                    Member::Present(document)
                }
                DiskObservation::NotFound => Member::Missing,
                DiskObservation::Failed(failure) => {
                    change.warnings.push(LifecycleWarning::ReloadFailed {
                        unit: SourceUnitKey::clone(unit),
                        document: DocumentKind::Source,
                        failure,
                    });
                    Member::Missing
                }
            },
            (Member::Missing, SourceEvent::Changed { .. }) => {
                change.warnings.push(LifecycleWarning::ChangedNonOpen {
                    unit: SourceUnitKey::clone(unit),
                    document: DocumentKind::Source,
                });
                Member::Missing
            }
            (Member::Missing, SourceEvent::Closed { .. }) => {
                change.warnings.push(LifecycleWarning::ClosedNonOpen {
                    unit: SourceUnitKey::clone(unit),
                    document: DocumentKind::Source,
                });
                Member::Missing
            }
            (Member::Present(mut document), SourceEvent::Opened { text, version, metadata }) => {
                let content_changed = self.set_source_content(engine, document.id, &text);
                document.metadata = metadata;
                document.content = EffectiveContent::Open { text, version };
                change.source_changed(document.id, content_changed);
                Member::Present(document)
            }
            (Member::Present(mut document), SourceEvent::Changed { text, version }) => {
                match &document.content {
                    EffectiveContent::Open { version: current_version, .. }
                        if version > *current_version =>
                    {
                        let content_changed = self.set_source_content(engine, document.id, &text);
                        document.content = EffectiveContent::Open { text, version };
                        change.source_changed(document.id, content_changed);
                    }
                    EffectiveContent::Open { .. } => {
                        change.warnings.push(LifecycleWarning::StaleChange {
                            unit: SourceUnitKey::clone(unit),
                            document: DocumentKind::Source,
                        });
                    }
                    EffectiveContent::Disk { .. } | EffectiveContent::Retained { .. } => {
                        change.warnings.push(LifecycleWarning::ChangedNonOpen {
                            unit: SourceUnitKey::clone(unit),
                            document: DocumentKind::Source,
                        });
                    }
                }
                Member::Present(document)
            }
            (Member::Present(document), SourceEvent::Closed { disk }) => {
                if !matches!(document.content, EffectiveContent::Open { .. }) {
                    change.warnings.push(LifecycleWarning::ClosedNonOpen {
                        unit: SourceUnitKey::clone(unit),
                        document: DocumentKind::Source,
                    });
                    Member::Present(document)
                } else {
                    self.reconcile_closed_source(engine, unit, document, disk, &mut change)
                }
            }
            (Member::Present(document), SourceEvent::DiskObserved { disk, metadata }) => {
                if matches!(document.content, EffectiveContent::Open { .. }) {
                    change.warnings.push(LifecycleWarning::DiskObservedWhileOpen {
                        unit: SourceUnitKey::clone(unit),
                        document: DocumentKind::Source,
                    });
                    Member::Present(document)
                } else {
                    self.reconcile_disk_source(engine, unit, document, disk, metadata, &mut change)
                }
            }
        };
        source_unit.source = next;
        change
    }

    fn reconcile_closed_source(
        &mut self,
        engine: &QueryEngine,
        unit: &SourceUnitKey,
        mut document: SourceDocument<Version, Metadata>,
        disk: DiskObservation,
        change: &mut LifecycleChange,
    ) -> Member<SourceDocument<Version, Metadata>> {
        match disk {
            DiskObservation::Found(text) => {
                let content_changed = self.set_source_content(engine, document.id, &text);
                document.content = EffectiveContent::Disk { text };
                change.source_changed(document.id, content_changed);
                Member::Present(document)
            }
            DiskObservation::NotFound => {
                self.remove_source(engine, unit, document.id);
                change.analysis = AnalysisInvalidation::Workspace;
                change.removed_sources.push(RemovedSource {
                    file_id: document.id,
                    locator: Arc::clone(&unit.source),
                });
                Member::Missing
            }
            DiskObservation::Failed(failure) => {
                let text = Arc::clone(document.content.text());
                let retained_failure = ReloadFailure::clone(&failure);
                document.content = EffectiveContent::Retained { text, failure: retained_failure };
                change.source_changed(document.id, false);
                change.warnings.push(LifecycleWarning::ReloadFailed {
                    unit: SourceUnitKey::clone(unit),
                    document: DocumentKind::Source,
                    failure,
                });
                Member::Present(document)
            }
        }
    }

    fn reconcile_disk_source(
        &mut self,
        engine: &QueryEngine,
        unit: &SourceUnitKey,
        mut document: SourceDocument<Version, Metadata>,
        disk: DiskObservation,
        metadata: Metadata,
        change: &mut LifecycleChange,
    ) -> Member<SourceDocument<Version, Metadata>> {
        match disk {
            DiskObservation::Found(text) => {
                let content_changed = self.set_source_content(engine, document.id, &text);
                document.metadata = metadata;
                document.content = EffectiveContent::Disk { text };
                change.source_changed(document.id, content_changed);
                Member::Present(document)
            }
            DiskObservation::NotFound => {
                self.remove_source(engine, unit, document.id);
                change.analysis = AnalysisInvalidation::Workspace;
                change.removed_sources.push(RemovedSource {
                    file_id: document.id,
                    locator: Arc::clone(&unit.source),
                });
                Member::Missing
            }
            DiskObservation::Failed(failure) => {
                let text = Arc::clone(document.content.text());
                let retained_failure = ReloadFailure::clone(&failure);
                document.content = EffectiveContent::Retained { text, failure: retained_failure };
                change.source_changed(document.id, false);
                change.warnings.push(LifecycleWarning::ReloadFailed {
                    unit: SourceUnitKey::clone(unit),
                    document: DocumentKind::Source,
                    failure,
                });
                Member::Present(document)
            }
        }
    }

    fn insert_source(
        &mut self,
        engine: &QueryEngine,
        unit: &SourceUnitKey,
        source_unit: &SourceUnit<Version, Metadata>,
        text: Arc<str>,
        metadata: Metadata,
    ) -> SourceDocument<Version, Metadata> {
        let id = self.source_files.insert(Arc::clone(&unit.source), Arc::clone(&text));
        self.source_units.insert(id, SourceUnitKey::clone(unit));
        engine.set_content(id, Arc::clone(&text));
        let lexed = lexing::lex(&text);
        let tokens = lexing::layout(&lexed);
        let (parsed, _) = parsing::parse(&lexed, &tokens);
        if let Some(name) = parsed.module_name(&text) {
            engine.set_module_file(&name, id);
        }
        if let Some(foreign_id) = source_unit.foreign_id() {
            engine.set_foreign_file(id, foreign_id);
        }
        SourceDocument { id, metadata, content: EffectiveContent::Disk { text } }
    }

    fn set_source_content(&mut self, engine: &QueryEngine, id: FileId, text: &Arc<str>) -> bool {
        let previous_content = self.source_files.content(id);
        if previous_content == *text {
            return false;
        }
        let previous_lexed = lexing::lex(&previous_content);
        let previous_tokens = lexing::layout(&previous_lexed);
        let (previous_parsed, _) = parsing::parse(&previous_lexed, &previous_tokens);
        let previous_name = previous_parsed.module_name(&previous_content);

        let path = self.source_files.path(id);
        let inserted_id = self.source_files.insert(path, Arc::clone(text));
        debug_assert_eq!(inserted_id, id);
        engine.set_content(id, Arc::clone(text));

        let current_lexed = lexing::lex(text);
        let current_tokens = lexing::layout(&current_lexed);
        let (current_parsed, _) = parsing::parse(&current_lexed, &current_tokens);
        let current_name = current_parsed.module_name(text);
        if previous_name != current_name
            && let Some(previous_name) = previous_name
        {
            engine.remove_module_file(&previous_name, id);
        }
        if let Some(current_name) = current_name {
            engine.set_module_file(&current_name, id);
        }
        true
    }

    fn remove_source(&mut self, engine: &QueryEngine, unit: &SourceUnitKey, id: FileId) {
        engine.remove_file(id);
        let removed_id = self.source_files.remove(unit.source());
        debug_assert_eq!(removed_id, Some(id));
        self.source_units.remove(&id);
    }
}
