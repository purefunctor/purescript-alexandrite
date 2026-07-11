use std::fs;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use stabilizing::StabilizedModule;
use syntax::cst;
use tests_compatibility::all_source_files;
use walkdir::WalkDir;

struct PreparedModule {
    cst: cst::Module,
    stabilized: StabilizedModule,
}

fn prepared_modules() -> (&'static str, u64, Arc<[PreparedModule]>) {
    let mut paths = all_source_files();
    let corpus = if paths.is_empty() {
        let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests-integration/fixtures");
        paths.extend(WalkDir::new(fixtures).into_iter().filter_map(|entry| {
            let path = entry.expect("failed to walk integration fixture corpus").into_path();
            (path.extension().is_some_and(|extension| extension == "purs")).then_some(path)
        }));
        "integration-fixtures"
    } else {
        "compatibility-cache"
    };

    assert!(!paths.is_empty(), "benchmark corpus is empty");

    let mut bytes = 0;
    let modules = paths
        .iter()
        .map(|path| {
            let source = fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            bytes += source.len() as u64;

            let lexed = lexing::lex(&source);
            let tokens = lexing::layout(&lexed);
            let (parsed, _) = parsing::parse(&lexed, &tokens);
            let stabilized = stabilizing::stabilize_module(&parsed.syntax_node());
            let cst = parsed.cst();

            PreparedModule { cst, stabilized }
        })
        .collect();

    assert!(bytes > 0, "benchmark corpus contains no source text");

    (corpus, bytes, modules)
}

fn criterion_benchmark(c: &mut Criterion) {
    let (corpus, bytes, modules) = prepared_modules();

    let mut group = c.benchmark_group(format!("indexing/{corpus}"));
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Bytes(bytes));

    group.bench_function("index-prepared-single-core", |b| {
        b.iter(|| {
            modules.iter().for_each(|module| {
                black_box(indexing::index_module(
                    black_box(&module.cst),
                    black_box(&module.stabilized),
                ));
            });
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
