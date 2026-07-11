use std::hint::black_box;
use std::sync::Arc;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use rayon::prelude::*;
use tests_compatibility::benchmark_sources;

fn criterion_benchmark(c: &mut Criterion) {
    let (corpus, files) = benchmark_sources();
    let bytes = files.iter().map(String::len).sum::<usize>() as u64;
    assert!(bytes > 0, "benchmark corpus contains no source text");
    let lexed: Arc<[_]> = files.iter().map(|file| lexing::lex(file)).collect();
    let tokens: Arc<[_]> = lexed.iter().map(lexing::layout).collect();

    let mut g = c.benchmark_group(format!("parsing/{corpus}"));
    g.sample_size(10);
    g.throughput(Throughput::Bytes(bytes));

    g.bench_function("lex-single-core", |b| {
        b.iter(|| {
            files.iter().for_each(|file| {
                black_box(lexing::lex(black_box(file)));
            });
        })
    });

    g.bench_function("layout-single-core", |b| {
        b.iter(|| {
            lexed.iter().for_each(|lexed| {
                black_box(lexing::layout(black_box(lexed)));
            });
        })
    });

    g.bench_function("parse-prelexed-single-core", |b| {
        b.iter(|| {
            lexed.iter().zip(tokens.iter()).for_each(|(lexed, tokens)| {
                black_box(parsing::parse(black_box(lexed), black_box(tokens)));
            });
        })
    });

    g.bench_function("parse-prelexed-retained-single-core", |b| {
        b.iter_batched(
            || (),
            |()| {
                lexed
                    .iter()
                    .zip(tokens.iter())
                    .map(|(lexed, tokens)| parsing::parse(black_box(lexed), black_box(tokens)))
                    .collect::<Vec<_>>()
            },
            BatchSize::PerIteration,
        )
    });

    let files = Arc::clone(&files);
    g.bench_function("parse-single-core", |b| {
        b.iter(|| {
            files.iter().for_each(|file| {
                let lexed = lexing::lex(black_box(file));
                let tokens = lexing::layout(black_box(&lexed));
                let parsed = parsing::parse(black_box(&lexed), black_box(&tokens));
                black_box(parsed);
            });
        })
    });

    let files = Arc::clone(&files);
    g.bench_function("parse-multi-core", |b| {
        b.iter(|| {
            files.par_iter().for_each(|file| {
                let lexed = lexing::lex(black_box(file));
                let tokens = lexing::layout(black_box(&lexed));
                let parsed = parsing::parse(black_box(&lexed), black_box(&tokens));
                black_box(parsed);
            });
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
