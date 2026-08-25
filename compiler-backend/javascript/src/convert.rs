//! Structured JavaScript generation from functional modules.

mod generator;

use building_types::QueryResult;
use files::FileId;

use crate::error::ModuleResult;
use crate::module::Module;

pub fn convert_module(
    queries: &impl functional::ExternalQueries,
    file_id: FileId,
) -> QueryResult<ModuleResult<Module>> {
    let functional = match queries.functional(file_id)? {
        Ok(functional) => functional,
        Err(error) => return Ok(Err(error.into())),
    };
    Ok(generator::Generator::new(&functional).generate())
}
