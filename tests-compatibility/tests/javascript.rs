use std::collections::BTreeMap;

use building::QueryEngine;
use files::FileId;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use tests_compatibility::{all_source_files, build_registered_engine, load_sources};

fn generate_and_serialize(engine: &QueryEngine, file_id: FileId) -> Result<usize, String> {
    let module = engine.javascript(file_id).map_err(|error| format!("query failed: {error}"))?;
    let mut output = Vec::new();
    module.serialize(&mut output).map_err(|error| format!("serialization failed: {error}"))?;
    Ok(output.len())
}

#[test]
fn generates_javascript_for_package_corpus() {
    let sources = load_sources(all_source_files());
    if sources.is_empty() {
        return;
    }
    let registered = build_registered_engine(&sources);
    let pool = ThreadPoolBuilder::new().stack_size(64 * 1024 * 1024).build().unwrap();
    let results = pool.install(|| {
        registered
            .candidates
            .par_iter()
            .zip(registered.paths.par_iter())
            .map(|(&file_id, path)| {
                let snapshot = registered.engine.snapshot();
                (path.clone(), generate_and_serialize(&snapshot, file_id))
            })
            .collect::<Vec<_>>()
    });
    let failures = results
        .iter()
        .filter_map(|(path, result)| {
            result.as_ref().err().map(|error| (path.clone(), error.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let output_size = results.iter().filter_map(|(_, result)| result.as_ref().ok()).sum::<usize>();

    assert!(failures.is_empty(), "{failures:#?}");
    assert!(output_size > 0);
}
