use std::collections::BTreeMap;

use building::QueryEngine;
use corefn::Module;
use files::FileId;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use serde::Deserialize;
use tests_compatibility::{all_source_files, build_registered_engine, load_sources};

fn generate_and_round_trip(engine: &QueryEngine, file_id: FileId) -> Result<(), String> {
    let module = engine.corefn(file_id).map_err(|error| format!("query failed: {error}"))?;
    let encoded =
        serde_json::to_vec(module.as_ref()).map_err(|error| format!("encoding failed: {error}"))?;

    let mut deserializer = serde_json::Deserializer::from_slice(&encoded);
    deserializer.disable_recursion_limit();
    let decoded = Module::deserialize(&mut deserializer)
        .map_err(|error| format!("decoding failed: {error}"))?;
    if &decoded == module.as_ref() {
        return Ok(());
    }

    let reencoded = serde_json::to_vec(&decoded).unwrap();
    let difference = encoded
        .iter()
        .zip(&reencoded)
        .position(|(left, right)| left != right)
        .unwrap_or(encoded.len().min(reencoded.len()));
    let start = difference.saturating_sub(80);
    let original_end = (difference + 80).min(encoded.len());
    let decoded_end = (difference + 80).min(reencoded.len());
    let original = String::from_utf8_lossy(&encoded[start..original_end]);
    let decoded = String::from_utf8_lossy(&reencoded[start..decoded_end]);
    Err(format!("round trip changed the module at byte {difference}: {original:?} != {decoded:?}"))
}

#[test]
fn generates_corefn_for_package_corpus() {
    let sources = load_sources(all_source_files());
    if sources.is_empty() {
        return;
    }
    let registered = build_registered_engine(&sources);
    let pool = ThreadPoolBuilder::new().stack_size(64 * 1024 * 1024).build().unwrap();
    let failures = pool.install(|| {
        registered
            .candidates
            .par_iter()
            .zip(registered.paths.par_iter())
            .filter_map(|(&file_id, path)| {
                let snapshot = registered.engine.snapshot();
                generate_and_round_trip(&snapshot, file_id).err().map(|error| (path.clone(), error))
            })
            .collect::<BTreeMap<_, _>>()
    });

    assert!(failures.is_empty(), "{failures:#?}");
}
