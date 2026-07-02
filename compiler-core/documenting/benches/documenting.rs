use std::fmt::Write;
use std::hint::black_box;
use std::sync::Arc;

use building_types::{QueryProxy, QueryResult};
use criterion::{Criterion, criterion_group, criterion_main};
use files::{FileId, Files};

struct Fixture {
    file_id: FileId,
    queries: SyntheticQueries,
}

#[derive(Clone)]
struct SyntheticQueries {
    parsed: parsing::FullParsedModule,
    stabilized: Arc<stabilizing::StabilizedModule>,
    indexed: Arc<indexing::IndexedModule>,
}

impl QueryProxy for SyntheticQueries {
    type Parsed = parsing::FullParsedModule;
    type Stabilized = Arc<stabilizing::StabilizedModule>;
    type Indexed = Arc<indexing::IndexedModule>;
    type Lowered = ();
    type Grouped = ();
    type Resolved = ();
    type Bracketed = ();
    type Sectioned = ();
    type Checked = ();
    type Documented = Arc<documenting::DocumentedModule>;

    fn parsed(&self, _id: FileId) -> QueryResult<Self::Parsed> {
        Ok(self.parsed.clone())
    }

    fn stabilized(&self, _id: FileId) -> QueryResult<Self::Stabilized> {
        Ok(Arc::clone(&self.stabilized))
    }

    fn indexed(&self, _id: FileId) -> QueryResult<Self::Indexed> {
        Ok(Arc::clone(&self.indexed))
    }

    fn lowered(&self, _id: FileId) -> QueryResult<Self::Lowered> {
        unreachable!("documenting does not read lowered modules")
    }

    fn grouped(&self, _id: FileId) -> QueryResult<Self::Grouped> {
        unreachable!("documenting does not read grouped modules")
    }

    fn resolved(&self, _id: FileId) -> QueryResult<Self::Resolved> {
        unreachable!("documenting does not read resolved modules")
    }

    fn bracketed(&self, _id: FileId) -> QueryResult<Self::Bracketed> {
        unreachable!("documenting does not read bracketed modules")
    }

    fn sectioned(&self, _id: FileId) -> QueryResult<Self::Sectioned> {
        unreachable!("documenting does not read sectioned modules")
    }

    fn checked(&self, _id: FileId) -> QueryResult<Self::Checked> {
        unreachable!("documenting does not read checked modules")
    }

    fn documented(&self, _id: FileId) -> QueryResult<Self::Documented> {
        unreachable!("documenting does not recursively read documented modules")
    }

    fn prim_id(&self) -> FileId {
        unreachable!("documenting does not read Prim")
    }

    fn module_file(&self, _name: &str) -> Option<FileId> {
        unreachable!("documenting does not resolve modules")
    }
}

fn fixture(item_count: usize) -> Fixture {
    let source = synthetic_module(item_count);
    let lexed = lexing::lex(&source);
    let tokens = lexing::layout(&lexed);
    let parsed = parsing::parse(&lexed, &tokens);
    assert!(parsed.1.is_empty(), "synthetic benchmark module should parse cleanly");

    let root = parsed.0.syntax_node();
    let stabilized = Arc::new(stabilizing::stabilize_module(&root));
    let indexed = Arc::new(indexing::index_module(&parsed.0.cst(), &stabilized));
    assert!(indexed.errors.is_empty(), "synthetic benchmark module should index cleanly");

    let mut files = Files::default();
    let file_id = files.insert("benchmark:///Bench/Documenting.purs", source);
    let queries = SyntheticQueries { parsed, stabilized, indexed };

    Fixture { file_id, queries }
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
                black_box(&fixture.queries),
                black_box(fixture.file_id),
            )
            .unwrap();
            black_box(documented);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
