use std::collections::BTreeSet;
use std::fs;
use std::hint::black_box;

use analyzer::position::{
    PositionConverter, PositionEncoding, offset_to_utf8_position, utf8_position_to_protocol,
};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use syntax::TextRange;
use syntax::ast::AstNode;
use tests_compatibility::{core_source_files, default_cache_dir};

struct ModuleWorkload {
    content: String,
    ranges: Vec<TextRange>,
}

fn workload() -> (usize, usize, usize, Vec<ModuleWorkload>) {
    let paths = core_source_files();
    assert!(!paths.is_empty(), "prepare the core compatibility corpus before benchmarking");

    let sources_dir = default_cache_dir().join("sources");
    let mut packages = BTreeSet::new();
    let mut bytes = 0;
    let modules = paths.into_iter().map(|path| {
        let relative_path = path.strip_prefix(&sources_dir).unwrap();
        let package = relative_path.components().next().unwrap();
        packages.insert(package.as_os_str().to_owned());

        let content = fs::read_to_string(&path).unwrap();
        bytes += content.len();

        let lexed = lexing::lex(&content);
        let tokens = lexing::layout(&lexed);
        let (parsed, errors) = parsing::parse(&lexed, &tokens);
        assert!(errors.is_empty(), "{} has parse errors", path.display());

        let module = parsed.cst();
        let ranges = module.statements().map(|statements| {
            let ranges = statements
                .children()
                .map(|declaration| declaration.syntax().text_range())
                .filter(|range| !range.is_empty());
            ranges.collect::<Vec<_>>()
        });
        let ranges = ranges.unwrap_or_default();

        // The corpus compiles without diagnostics, so declaration ranges provide stable,
        // source-derived proxies at a bounded density for a broken-workspace workload.
        let range_count = content.len().div_ceil(4_096).clamp(1, 8).min(ranges.len());
        let ranges = if range_count <= 1 {
            vec![ranges.get(ranges.len() / 2).copied().unwrap_or(module.syntax().text_range())]
        } else {
            let selected = (0..range_count).map(|index| {
                let range_index = index * (ranges.len() - 1) / (range_count - 1);
                ranges[range_index]
            });
            selected.collect()
        };

        ModuleWorkload { content, ranges }
    });
    let modules = modules.collect::<Vec<_>>();

    (packages.len(), modules.len(), bytes, modules)
}

fn convert_rebuilding_indexes(modules: &[ModuleWorkload]) -> u64 {
    let mut checksum = 0;
    for module in modules {
        for range in &module.ranges {
            for offset in [range.start(), range.end()] {
                let position = offset_to_utf8_position(&module.content, offset).unwrap();
                let position =
                    utf8_position_to_protocol(&module.content, position, PositionEncoding::Utf16)
                        .unwrap();
                checksum += u64::from(position.line) + u64::from(position.character);
            }
        }
    }
    checksum
}

fn convert_reusing_indexes(modules: &[ModuleWorkload]) -> u64 {
    let mut checksum = 0;
    for module in modules {
        let converter = PositionConverter::new(&module.content);
        for range in &module.ranges {
            for offset in [range.start(), range.end()] {
                let position =
                    converter.offset_to_protocol(offset, PositionEncoding::Utf16).unwrap();
                checksum += u64::from(position.line) + u64::from(position.character);
            }
        }
    }
    checksum
}

fn criterion_benchmark(criterion: &mut Criterion) {
    let (packages, module_count, bytes, modules) = workload();
    let ranges = modules.iter().map(|module| module.ranges.len()).sum::<usize>();
    assert!(bytes > 0);
    assert!(ranges > 0);
    eprintln!(
        "diagnostics corpus: {packages} packages, {module_count} modules, {bytes} bytes, {ranges} diagnostic ranges"
    );

    let mut group = criterion.benchmark_group("diagnostics_positions/compatibility-core");
    group.sample_size(20);
    group.throughput(Throughput::Elements(ranges as u64));
    group.bench_function("rebuild", |bencher| {
        bencher.iter(|| black_box(convert_rebuilding_indexes(&modules)));
    });
    group.bench_function("reuse", |bencher| {
        bencher.iter(|| black_box(convert_reusing_indexes(&modules)));
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
