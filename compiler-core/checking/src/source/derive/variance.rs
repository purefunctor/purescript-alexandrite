use building_types::QueryResult;
use files::FileId;
use indexing::{TermItemId, TypeItemId};
use smol_str::SmolStr;

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::substitute::SubstituteName;
use crate::core::{KindOrType, Name, Type, TypeId, normalise, toolkit};
use crate::error::ErrorKind;
use crate::state::CheckState;

use super::tools;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::source) enum Variance {
    Covariant,
    Contravariant,
}

impl Variance {
    fn flip(self) -> Variance {
        match self {
            Variance::Covariant => Variance::Contravariant,
            Variance::Contravariant => Variance::Covariant,
        }
    }
}

type ParameterConfig = (Variance, Option<(FileId, TypeItemId)>);

#[derive(Clone, Copy)]
pub(in crate::source) enum VarianceConfig {
    Single(ParameterConfig),
    Pair(ParameterConfig, ParameterConfig),
}

pub struct VarianceRecipe {
    pub constructors: Vec<ConstructorRecipe>,
    pub valid: bool,
}

pub struct ConstructorRecipe {
    pub constructor_id: TermItemId,
    pub fields: Vec<Option<TraversalOperation>>,
}

#[derive(Clone, Copy)]
pub enum TraversalParameter {
    First,
    Second,
}

pub enum TraversalOperation {
    Parameter { parameter: TraversalParameter },
    Function { argument: Option<Box<TraversalOperation>>, result: Option<Box<TraversalOperation>> },
    Map { argument: Box<TraversalOperation> },
    Record { fields: Vec<RecordFieldRecipe> },
}

pub struct RecordFieldRecipe {
    pub label: SmolStr,
    pub operation: TraversalOperation,
}

struct DerivedParameter {
    name: Name,
    traversal_parameter: TraversalParameter,
    expected: Variance,
    class: Option<(FileId, TypeItemId)>,
}

enum DerivedRigids {
    Invalid,
    Single(DerivedParameter),
    Pair(DerivedParameter, DerivedParameter),
}

impl DerivedRigids {
    fn get(&self, name: Name) -> Option<&DerivedParameter> {
        self.iter().find(|parameter| parameter.name == name)
    }

    fn iter(&self) -> impl Iterator<Item = &DerivedParameter> {
        let (first, second) = match self {
            DerivedRigids::Invalid => (None, None),
            DerivedRigids::Single(first) => (Some(first), None),
            DerivedRigids::Pair(first, second) => (Some(first), Some(second)),
        };
        first.into_iter().chain(second)
    }
}

pub fn generate_variance_constraints<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    data_file: FileId,
    data_id: TypeItemId,
    derived_type: TypeId,
    config: VarianceConfig,
) -> QueryResult<VarianceRecipe>
where
    Q: ExternalQueries,
{
    let constructor_ids = tools::lookup_data_constructors(context, data_file, data_id)?;
    let mut constructors = Vec::with_capacity(constructor_ids.len());
    let mut valid = true;
    for constructor_id in constructor_ids {
        let constructor_t = toolkit::lookup_file_term(state, context, data_file, constructor_id)?;
        let (fields, rigids) =
            extract_fields_with_rigids(state, context, constructor_t, derived_type, config)?;
        valid &= !matches!(rigids, DerivedRigids::Invalid);

        let mut field_recipes = Vec::with_capacity(fields.len());
        for field in fields {
            let operation = check_variance_field(
                state,
                context,
                field,
                Variance::Covariant,
                &rigids,
                &mut valid,
            )?;
            field_recipes.push(operation);
        }
        constructors.push(ConstructorRecipe { constructor_id, fields: field_recipes });
    }

    Ok(VarianceRecipe { constructors, valid })
}

fn extract_fields_with_rigids<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    constructor_t: TypeId,
    derived_type: TypeId,
    config: VarianceConfig,
) -> QueryResult<(Vec<TypeId>, DerivedRigids)>
where
    Q: ExternalQueries,
{
    let (_, arguments) = toolkit::extract_all_applications(state, context, derived_type)?;
    let mut arguments = arguments.iter().copied();
    let mut current = constructor_t;
    let mut names = vec![];

    loop {
        current = normalise::expand(state, context, current)?;
        let Type::Forall(binder_id, inner) = context.lookup_type(current) else {
            break;
        };

        let binder = context.lookup_forall_binder(binder_id);
        let replacement = arguments
            .next()
            .map(|argument| match argument {
                KindOrType::Kind(argument) | KindOrType::Type(argument) => argument,
            })
            .unwrap_or_else(|| {
                let rigid = state.fresh_rigid(context.queries, binder.kind);
                let Type::Rigid(name, _, _) = context.lookup_type(rigid) else {
                    unreachable!("fresh_rigid must create Type::Rigid")
                };
                names.push(name);
                rigid
            });

        current = SubstituteName::one(state, context, binder.name, replacement, inner)?;
    }

    let rigids = match (config, &names[..]) {
        (VarianceConfig::Single((expected, class)), [.., a]) => {
            DerivedRigids::Single(DerivedParameter {
                name: *a,
                traversal_parameter: TraversalParameter::First,
                expected,
                class,
            })
        }
        (
            VarianceConfig::Pair((first_expected, first_class), (second_expected, second_class)),
            [.., a, b],
        ) => DerivedRigids::Pair(
            DerivedParameter {
                name: *a,
                traversal_parameter: TraversalParameter::First,
                expected: first_expected,
                class: first_class,
            },
            DerivedParameter {
                name: *b,
                traversal_parameter: TraversalParameter::Second,
                expected: second_expected,
                class: second_class,
            },
        ),
        _ => {
            state.insert_error(ErrorKind::CannotDeriveForType { type_id: derived_type });
            DerivedRigids::Invalid
        }
    };

    let toolkit::InspectFunction { arguments: fields, .. } =
        toolkit::inspect_function(state, context, current)?;

    Ok((fields, rigids))
}

fn check_variance_field<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    type_id: TypeId,
    variance: Variance,
    rigids: &DerivedRigids,
    valid: &mut bool,
) -> QueryResult<Option<TraversalOperation>>
where
    Q: ExternalQueries,
{
    let type_id = normalise::expand(state, context, type_id)?;

    match context.lookup_type(type_id) {
        Type::Rigid(name, _, _) => {
            if let Some(parameter) = rigids.get(name) {
                *valid &= emit_variance_error(state, type_id, variance, parameter.expected);
                return Ok(Some(TraversalOperation::Parameter {
                    parameter: parameter.traversal_parameter,
                }));
            }
        }
        Type::Function(argument, result) => {
            let argument =
                check_variance_field(state, context, argument, variance.flip(), rigids, valid)?;
            let result = check_variance_field(state, context, result, variance, rigids, valid)?;
            if argument.is_some() || result.is_some() {
                return Ok(Some(TraversalOperation::Function {
                    argument: argument.map(Box::new),
                    result: result.map(Box::new),
                }));
            }
        }
        Type::Application(function, argument) => {
            let function = normalise::expand(state, context, function)?;
            if function == context.prim.record {
                return check_variance_field(state, context, argument, variance, rigids, valid);
            } else {
                for parameter in rigids.iter() {
                    if toolkit::contains_rigid(state, context, argument, parameter.name)? {
                        *valid &= emit_variance_error(state, type_id, variance, parameter.expected);
                        if variance == parameter.expected {
                            if let Some(class) = parameter.class {
                                tools::emit_constraint(context, state, class, function);
                            } else {
                                state.insert_error(ErrorKind::DeriveMissingFunctor);
                                *valid = false;
                            }
                        }
                    }
                }
                let argument =
                    check_variance_field(state, context, argument, variance, rigids, valid)?;
                if let Some(argument) = argument {
                    return Ok(Some(TraversalOperation::Map { argument: Box::new(argument) }));
                }
            }
        }
        Type::KindApplication(_, argument) => {
            return check_variance_field(state, context, argument, variance, rigids, valid);
        }
        Type::Row(row_id) => {
            let row = context.lookup_row_type(row_id);
            let mut fields = Vec::new();
            for field in row.fields.iter() {
                let operation =
                    check_variance_field(state, context, field.id, variance, rigids, valid)?;
                if let Some(operation) = operation {
                    fields.push(RecordFieldRecipe { label: field.label.clone(), operation });
                }
            }
            if let Some(tail) = row.tail {
                check_variance_field(state, context, tail, variance, rigids, valid)?;
            }
            if !fields.is_empty() {
                return Ok(Some(TraversalOperation::Record { fields }));
            }
        }
        _ => {}
    }

    Ok(None)
}

fn emit_variance_error(
    state: &mut CheckState,
    type_id: TypeId,
    actual: Variance,
    expected: Variance,
) -> bool {
    if actual == expected {
        return true;
    }

    match actual {
        Variance::Covariant => state.insert_error(ErrorKind::CovariantOccurrence { type_id }),
        Variance::Contravariant => {
            state.insert_error(ErrorKind::ContravariantOccurrence { type_id })
        }
    }
    false
}
