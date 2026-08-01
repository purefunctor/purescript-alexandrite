use building_types::QueryResult;
use files::FileId;
use indexing::TypeItemId;

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::toolkit;
use crate::error::ErrorKind;
use crate::state::CheckState;

use super::DeriveStrategy;
use super::variance::{FunctionPolicy, ParameterConfig, Variance, VarianceConfig};

pub fn check_derive_contravariant<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    class_file: FileId,
    class_id: TypeItemId,
    arguments: &[crate::core::TypeId],
) -> QueryResult<Option<DeriveStrategy>>
where
    Q: ExternalQueries,
{
    let [derived_type] = arguments else {
        state.insert_error(ErrorKind::DeriveInvalidArity {
            class_file,
            class_id,
            expected: 1,
            actual: arguments.len(),
        });
        return Ok(None);
    };

    let Some((data_file, data_id)) =
        toolkit::extract_type_constructor(state, context, *derived_type)?
    else {
        state.insert_error(ErrorKind::CannotDeriveForType { type_id: *derived_type });
        return Ok(None);
    };

    let parameter = ParameterConfig {
        variance: Variance::Contravariant,
        unary_class: Some((class_file, class_id)),
        function_policy: FunctionPolicy::Allow,
    };
    let config = VarianceConfig::Single { parameter, binary_class: None };

    Ok(Some(DeriveStrategy::VarianceConstraints {
        data_file,
        data_id,
        derived_type: *derived_type,
        config,
    }))
}

pub fn check_derive_profunctor<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    class_file: FileId,
    class_id: TypeItemId,
    arguments: &[crate::core::TypeId],
) -> QueryResult<Option<DeriveStrategy>>
where
    Q: ExternalQueries,
{
    let [derived_type] = arguments else {
        state.insert_error(ErrorKind::DeriveInvalidArity {
            class_file,
            class_id,
            expected: 1,
            actual: arguments.len(),
        });
        return Ok(None);
    };

    let Some((data_file, data_id)) =
        toolkit::extract_type_constructor(state, context, *derived_type)?
    else {
        state.insert_error(ErrorKind::CannotDeriveForType { type_id: *derived_type });
        return Ok(None);
    };

    let contravariant = context.known_types.contravariant;
    let functor = context.known_types.functor;
    let config = VarianceConfig::Pair {
        first: ParameterConfig {
            variance: Variance::Contravariant,
            unary_class: contravariant,
            function_policy: FunctionPolicy::Allow,
        },
        second: ParameterConfig {
            variance: Variance::Covariant,
            unary_class: functor,
            function_policy: FunctionPolicy::Allow,
        },
        binary_class: None,
    };

    Ok(Some(DeriveStrategy::VarianceConstraints {
        data_file,
        data_id,
        derived_type: *derived_type,
        config,
    }))
}
