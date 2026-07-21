use analyzer::QueryEngine;
use checking::tree::pretty;
use files::FileId;

pub fn report(engine: &QueryEngine, id: FileId) -> String {
    let checked = engine.checked(id).unwrap();
    pretty::Pretty::new(engine, &checked).render(id).unwrap().to_string()
}
