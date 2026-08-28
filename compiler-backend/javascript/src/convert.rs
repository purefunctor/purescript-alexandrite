//! Structured JavaScript generation from functional modules.

mod generator;

use building_types::QueryResult;
use files::FileId;

pub(crate) use generator::identifier_is_binding;

use crate::error::ModuleResult;
use crate::module::Module;

pub fn convert_module(
    queries: &impl crate::ExternalQueries,
    file_id: FileId,
) -> QueryResult<ModuleResult<Module>> {
    let functional = match queries.functional(file_id)? {
        Ok(functional) => functional,
        Err(error) => return Ok(Err(error.into())),
    };
    let foreign_file = queries.foreign_file(file_id)?;
    Ok(generator::Generator::new(&functional, foreign_file.map(|file| file.kind())).generate())
}
