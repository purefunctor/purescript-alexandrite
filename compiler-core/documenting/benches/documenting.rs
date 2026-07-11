use std::fmt::Write;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};

struct Fixture {
    source: String,
    parsed: parsing::ParsedModule,
    stabilized: Arc<stabilizing::StabilizedModule>,
    indexed: Arc<indexing::IndexedModule>,
}

fn fixture(item_count: usize) -> Fixture {
    let source = synthetic_module(item_count);
    let lexed = lexing::lex(&source);
    let tokens = lexing::layout(&lexed);
    let (parsed, _) = parsing::parse(&lexed, &tokens);

    let root = parsed.syntax_node();
    let cst = parsed.cst();

    let stabilized = Arc::new(stabilizing::stabilize_module(&root));
    let indexed = Arc::new(indexing::index_module(&source, &cst, &stabilized));

    Fixture { source, parsed, stabilized, indexed }
}

fn synthetic_module(item_count: usize) -> String {
    let mut source = String::from("-- | Module documentation.\nmodule Bench.Documenting where\n\n");

    for index in 0..item_count {
        writeln!(source, "-- | Documentation for value {index}.").unwrap();
        writeln!(source, "value{index} :: Int").unwrap();
        writeln!(source, "value{index} = {index}").unwrap();
        writeln!(source).unwrap();

        writeln!(source, "undocumented{index} :: Int").unwrap();
        writeln!(source, "undocumented{index} = {index}").unwrap();
        writeln!(source).unwrap();

        writeln!(source, "-- | Documentation for Type{index}.").unwrap();
        writeln!(source, "data Type{index}").unwrap();
        writeln!(source, "  -- | Documentation for First{index}.").unwrap();
        writeln!(source, "  = First{index} Int").unwrap();
        writeln!(source, "  -- | Documentation for Second{index}.").unwrap();
        writeln!(source, "  | Second{index} Int").unwrap();
        writeln!(source).unwrap();
    }

    source
}

fn criterion_benchmark(c: &mut Criterion) {
    let fixture = fixture(400);

    c.bench_function("document-module-synthetic", |b| {
        b.iter(|| {
            let documented = documenting::document_module(
                black_box(&fixture.source),
                black_box(&fixture.parsed),
                black_box(&fixture.stabilized),
                black_box(&fixture.indexed),
            );
            black_box(documented);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
