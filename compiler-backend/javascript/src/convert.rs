//! Structured JavaScript generation from SSA modules.

mod generator;

use building_types::QueryResult;
use files::FileId;

use self::generator::Generator;
use crate::error::ModuleResult;
use crate::module::Module;

pub fn convert_module(
    queries: &impl ssa::ExternalQueries,
    file_id: FileId,
) -> QueryResult<ModuleResult<Module>> {
    let control_flow = match queries.ssa(file_id)? {
        Ok(control_flow) => control_flow,
        Err(error) => return Ok(Err(error.into())),
    };
    Ok(Generator::new(&control_flow).generate())
}
