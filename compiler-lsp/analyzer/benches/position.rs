use std::hint::black_box;

use analyzer::position::{
    PositionConverter, PositionEncoding, offset_to_utf8_position, utf8_position_to_protocol,
};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use syntax::TextSize;

fn content(bytes: usize) -> String {
    let line = "value😀 = value\n";
    line.repeat(bytes.div_ceil(line.len()))
}

fn offsets(content: &str, count: usize) -> Vec<TextSize> {
    let stride = content.len() / count;
    let offsets = (0..count).map(|index| {
        let mut offset = index * stride;
        while !content.is_char_boundary(offset) {
            offset += 1;
        }
        TextSize::new(offset as u32)
    });

    offsets.collect()
}

fn convert_positions(content: &str, offsets: &[TextSize]) -> u64 {
    let mut checksum = 0;
    for &offset in offsets {
        let position = offset_to_utf8_position(content, offset).unwrap();
        let position =
            utf8_position_to_protocol(content, position, PositionEncoding::Utf16).unwrap();
        checksum += u64::from(position.line) + u64::from(position.character);
    }
    checksum
}

fn convert_positions_reusing_index(content: &str, offsets: &[TextSize]) -> u64 {
    let converter = PositionConverter::new(content);
    let mut checksum = 0;
    for &offset in offsets {
        let position = converter.offset_to_protocol(offset, PositionEncoding::Utf16).unwrap();
        checksum += u64::from(position.line) + u64::from(position.character);
    }
    checksum
}

fn source_bytes(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("position/source_bytes");
    group.sample_size(20);

    for bytes in [1_024, 16_384, 262_144] {
        let content = content(bytes);
        let offsets = offsets(&content, 32);
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::new("rebuild", bytes), &bytes, |bencher, _| {
            bencher.iter(|| black_box(convert_positions(&content, &offsets)));
        });
        group.bench_with_input(BenchmarkId::new("reuse", bytes), &bytes, |bencher, _| {
            bencher.iter(|| black_box(convert_positions_reusing_index(&content, &offsets)));
        });
    }

    group.finish();
}

fn span_count(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("position/span_count");
    group.sample_size(20);
    let content = content(65_536);

    for count in [1, 16, 256] {
        let offsets = offsets(&content, count * 2);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("rebuild", count), &count, |bencher, _| {
            bencher.iter(|| black_box(convert_positions(&content, &offsets)));
        });
        group.bench_with_input(BenchmarkId::new("reuse", count), &count, |bencher, _| {
            bencher.iter(|| black_box(convert_positions_reusing_index(&content, &offsets)));
        });
    }

    group.finish();
}

criterion_group!(benches, source_bytes, span_count);
criterion_main!(benches);
