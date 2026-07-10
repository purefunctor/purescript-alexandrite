use std::fs;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

use building::QueryEngine;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use files::FileId;
use indexing::IndexedModule;
use resolving::ResolvedModule;
use stabilizing::StabilizedModule;
use syntax::cst;
use tests_compatibility::{all_source_files, build_warmed_engine};

struct PreparedLoweringModule {
    file_id: FileId,
    cst: cst::Module,
    stabilized: Arc<StabilizedModule>,
    indexed: Arc<IndexedModule>,
    resolved: Arc<ResolvedModule>,
}

struct PreparedCorpus {
    bytes: u64,
    engine: QueryEngine,
    modules: Arc<[PreparedLoweringModule]>,
    prim: Arc<ResolvedModule>,
}

fn prepared_corpus() -> PreparedCorpus {
    let paths = all_source_files();
    assert!(
        !paths.is_empty(),
        "lowering and resolving benchmarks require the prepared compatibility corpus"
    );

    let mut bytes = 0;
    let sources: Arc<[(String, String)]> = paths
        .iter()
        .map(|path| {
            let source = fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            bytes += source.len() as u64;
            (path.to_string_lossy().into_owned(), source)
        })
        .collect();
    assert!(bytes > 0, "benchmark corpus contains no source text");

    let warmed = build_warmed_engine(&sources);
    let prim = warmed.engine.resolved(warmed.engine.prim_id()).expect("failed to resolve Prim");
    let modules = warmed
        .candidates
        .iter()
        .map(|&file_id| {
            let (parsed, _) = warmed.engine.parsed(file_id).expect("failed to parse corpus module");
            let stabilized =
                warmed.engine.stabilized(file_id).expect("failed to stabilize corpus module");
            let indexed = warmed.engine.indexed(file_id).expect("failed to index corpus module");
            let resolved =
                warmed.engine.resolved(file_id).expect("failed to resolve corpus module");
            let cst = parsed.cst();
            PreparedLoweringModule { file_id, cst, stabilized, indexed, resolved }
        })
        .collect();

    PreparedCorpus { bytes, engine: warmed.engine, modules, prim }
}

fn criterion_benchmark(c: &mut Criterion) {
    let corpus = prepared_corpus();

    let mut group = c.benchmark_group("lowering-resolving/compatibility-cache");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Bytes(corpus.bytes));

    group.bench_function("resolve-prepared-single-core", |b| {
        b.iter(|| {
            corpus.modules.iter().for_each(|module| {
                black_box(
                    resolving::resolve_module(black_box(&corpus.engine), black_box(module.file_id))
                        .expect("failed to resolve corpus module"),
                );
            });
        })
    });

    group.bench_function("lower-prepared-single-core", |b| {
        b.iter(|| {
            corpus.modules.iter().for_each(|module| {
                black_box(lowering::lower_module(
                    black_box(module.file_id),
                    black_box(&module.cst),
                    black_box(&corpus.prim),
                    black_box(&module.stabilized),
                    black_box(&module.indexed),
                    black_box(&module.resolved),
                ));
            });
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
