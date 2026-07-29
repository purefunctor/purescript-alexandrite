use building::QueryEngine;
use checking::tree::pretty;
use files::FileId;

pub fn report(engine: &QueryEngine, id: FileId) -> String {
    let checked = engine.checked(id).unwrap();
    let config = pretty::PrettyConfig::new().fully_qualified_names();
    pretty::Pretty::with_config(engine, &checked, config).render(id).unwrap().to_string()
}
