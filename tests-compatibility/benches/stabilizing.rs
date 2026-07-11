use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use syntax::SyntaxNode;
use tests_compatibility::benchmark_sources;

fn criterion_benchmark(c: &mut Criterion) {
    let (corpus, sources) = benchmark_sources();
    let bytes = sources.iter().map(String::len).sum::<usize>() as u64;
    assert!(bytes > 0, "benchmark corpus contains no source text");

    let roots: Arc<[SyntaxNode]> = sources
        .iter()
        .map(|source| {
            let lexed = lexing::lex(source);
            let tokens = lexing::layout(&lexed);
            let (parsed, _) = parsing::parse(&lexed, &tokens);
            parsed.syntax_node()
        })
        .collect();

    let mut group = c.benchmark_group(format!("stabilizing/{corpus}"));
    group.sample_size(20);
    group.throughput(Throughput::Bytes(bytes));

    group.bench_function("stabilize-single-core", |b| {
        b.iter(|| {
            roots.iter().for_each(|root| {
                black_box(stabilizing::stabilize_module(black_box(root)));
            });
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
