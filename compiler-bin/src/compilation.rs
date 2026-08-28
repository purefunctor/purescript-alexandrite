use std::sync::Arc;

use building::{
    DiskObservation, FileLifecycle, ForeignEvent, LifecycleChange, LifecycleEvent, QueryEngine,
    SourceEvent, SourceUnitKey,
};
use files::FileId;
use prim_constants::MODULE_MAP;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceRole {
    Prim,
    Input,
}

pub struct CompilationState {
    engine: QueryEngine,
    files: FileLifecycle<(), SourceRole>,
}

impl CompilationState {
    pub fn new() -> CompilationState {
        let engine = QueryEngine::default();
        let mut files = FileLifecycle::default();

        for (name, content) in MODULE_MAP {
            let source = format!("prim://localhost/{name}.purs");
            let foreign = format!("prim://localhost/{name}.js");
            let event = LifecycleEvent::Source {
                unit: SourceUnitKey::new(source, foreign),
                event: SourceEvent::DiskObserved {
                    disk: DiskObservation::Found(Arc::from(*content)),
                    metadata: SourceRole::Prim,
                },
            };

            let change = files.apply(&engine, event);
            let id = change
                .changed_sources()
                .next()
                .expect("invariant violated: Prim source lifecycle did not insert a source");
            engine.set_module_file(name, id);
        }

        CompilationState { engine, files }
    }

    pub fn observe_source(
        &mut self,
        unit: SourceUnitKey,
        disk: DiskObservation,
    ) -> LifecycleChange {
        let event = LifecycleEvent::Source {
            unit,
            event: SourceEvent::DiskObserved { disk, metadata: SourceRole::Input },
        };
        self.files.apply(&self.engine, event)
    }

    pub fn observe_foreign(
        &mut self,
        unit: SourceUnitKey,
        disk: DiskObservation,
    ) -> LifecycleChange {
        let event = LifecycleEvent::Foreign { unit, event: ForeignEvent::DiskObserved { disk } };
        self.files.apply(&self.engine, event)
    }

    pub fn input_source_ids(&self) -> Vec<FileId> {
        self.files
            .source_ids()
            .filter(|&file_id| self.files.source_metadata(file_id) == Some(&SourceRole::Input))
            .collect()
    }

    pub fn snapshot(&self) -> QueryEngine {
        self.engine.snapshot()
    }

    pub fn source_path(&self, file_id: FileId) -> Option<Arc<str>> {
        self.files.source_path(file_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(name: &str) -> SourceUnitKey {
        SourceUnitKey::new(format!("file:///src/{name}.purs"), format!("file:///src/{name}.js"))
    }

    #[test]
    fn prim_modules_are_not_input_sources() {
        let compilation = CompilationState::new();

        assert!(compilation.input_source_ids().is_empty());
        assert!(compilation.snapshot().module_file("Prim").is_some());
    }

    #[test]
    fn source_observations_preserve_identity_until_deletion() {
        let mut compilation = CompilationState::new();
        let unit = unit("Main");
        let content = DiskObservation::Found(Arc::from("module Main where\n"));
        compilation.observe_source(SourceUnitKey::clone(&unit), content);
        let original = compilation.input_source_ids()[0];

        let content = DiskObservation::Found(Arc::from("module Main where\n\nvalue = 1\n"));
        compilation.observe_source(SourceUnitKey::clone(&unit), content);
        assert_eq!(compilation.input_source_ids(), vec![original]);

        compilation.observe_source(SourceUnitKey::clone(&unit), DiskObservation::NotFound);
        assert!(compilation.input_source_ids().is_empty());

        let content = DiskObservation::Found(Arc::from("module Main where\n"));
        compilation.observe_source(unit, content);
        assert_ne!(compilation.input_source_ids(), vec![original]);
    }

    #[test]
    fn foreign_observations_are_associated_with_the_input_source() {
        let mut compilation = CompilationState::new();
        let unit = unit("Main");
        let content = DiskObservation::Found(Arc::from("module Main where\n"));
        compilation.observe_source(SourceUnitKey::clone(&unit), content);
        let source_id = compilation.input_source_ids()[0];

        let content = DiskObservation::Found(Arc::from("export const value = 1;\n"));
        let change = compilation.observe_foreign(unit, content);

        assert_eq!(change.changed_sources().collect::<Vec<_>>(), vec![source_id]);
        assert!(compilation.snapshot().foreign_file(source_id).is_some());
    }
}
