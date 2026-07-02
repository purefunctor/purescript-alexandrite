use building::QueryEngine;
use files::FileId;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::Error;

macro_rules! warm_modules {
    ($engine:expr, $modules:expr, $query:ident) => {
        $modules.par_iter().try_for_each(|&id| {
            let engine = $engine.snapshot();
            let _ = engine.$query(id)?;
            Ok::<(), Error>(())
        })
    };
}

pub fn warm_documentation_queries(engine: &QueryEngine, modules: &[FileId]) -> Result<(), Error> {
    warm_modules!(engine, modules, indexed)?;
    warm_modules!(engine, modules, resolved)?;
    warm_modules!(engine, modules, lowered)?;
    warm_modules!(engine, modules, grouped)?;
    warm_modules!(engine, modules, bracketed)?;
    warm_modules!(engine, modules, sectioned)?;
    warm_modules!(engine, modules, checked)?;
    warm_modules!(engine, modules, documented)?;

    Ok(())
}
