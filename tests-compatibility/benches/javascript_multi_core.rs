use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use tests_compatibility::{
    acme_source_files, build_registered_engine, core_source_files, load_sources,
    run_javascript_multi_core, run_javascript_single_core,
};

fn probe(sources: &[(String, String)]) -> Duration {
    let registered = build_registered_engine(sources);
    let start = Instant::now();
    black_box(run_javascript_multi_core(&registered));
    start.elapsed()
}

fn criterion_benchmark(c: &mut Criterion) {
    let core_sources = load_sources(core_source_files());
    let acme_sources = load_sources(acme_source_files());
    assert!(!core_sources.is_empty(), "Core benchmark corpus is empty");
    assert!(!acme_sources.is_empty(), "Acme benchmark corpus is empty");

    let core_probe = probe(&core_sources);
    let acme_probe = probe(&acme_sources);
    let measurement = (core_probe.max(acme_probe) * 12).max(Duration::from_secs(60));

    let mut group = c.benchmark_group("javascript-full-pipeline");
    group.sample_size(10);
    group.measurement_time(measurement);

    for (label, sources) in [("core", &core_sources), ("acme", &acme_sources)] {
        group.throughput(Throughput::Elements(sources.len() as u64));
        let single_core_sources = Arc::clone(sources);
        group.bench_function(format!("generate-{label}-single-core"), move |bencher| {
            bencher.iter_batched(
                || build_registered_engine(&single_core_sources),
                |registered| black_box(run_javascript_single_core(&registered)),
                BatchSize::PerIteration,
            )
        });

        let multi_core_sources = Arc::clone(sources);
        group.bench_function(format!("generate-{label}-multi-core"), move |bencher| {
            bencher.iter_batched(
                || build_registered_engine(&multi_core_sources),
                |registered| black_box(run_javascript_multi_core(&registered)),
                BatchSize::PerIteration,
            )
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
