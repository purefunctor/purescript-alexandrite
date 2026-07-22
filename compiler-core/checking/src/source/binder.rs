//! Implements syntax-driven checking rules for binders.

use std::sync::Arc;

use building_types::QueryResult;
use itertools::{EitherOrBoth, Itertools};
use smol_str::SmolStr;

use crate::context::CheckContext;
use crate::core::{RowField, RowType, Type, TypeId, normalise, toolkit, unification};
use crate::error::{ErrorCrumb, ErrorKind};
use crate::source::terms::application;
use crate::source::{operator, types};
use crate::state::CheckState;
use crate::{ExternalQueries, safe_loop, tree};

#[derive(Copy, Clone, Debug)]
pub struct ElaboratedBinder {
    pub type_id: TypeId,
    pub binder: tree::BinderId,
}

#[derive(Copy, Clone, Debug)]
enum BinderMode {
    Infer,
    Check { expected_type: TypeId, elaborating: bool },
}

pub fn infer_binder<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder_id: lowering::BinderId,
) -> QueryResult<ElaboratedBinder>
where
    Q: ExternalQueries,
{
    state.with_error_crumb(ErrorCrumb::InferringBinder(binder_id), |state| {
        binder_core(state, context, binder_id, BinderMode::Infer)
    })
}

pub fn check_binder<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder_id: lowering::BinderId,
    expected_type: TypeId,
) -> QueryResult<ElaboratedBinder>
where
    Q: ExternalQueries,
{
    state.with_error_crumb(ErrorCrumb::CheckingBinder(binder_id), |state| {
        binder_core(
            state,
            context,
            binder_id,
            BinderMode::Check { expected_type, elaborating: true },
        )
    })
}

pub fn check_argument_binder<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder_id: lowering::BinderId,
    expected_type: TypeId,
) -> QueryResult<ElaboratedBinder>
where
    Q: ExternalQueries,
{
    state.with_error_crumb(ErrorCrumb::CheckingBinder(binder_id), |state| {
        binder_core(
            state,
            context,
            binder_id,
            BinderMode::Check { expected_type, elaborating: false },
        )
    })
}

pub fn requires_instantiation<Q>(context: &CheckContext<Q>, binder_id: lowering::BinderId) -> bool
where
    Q: ExternalQueries,
{
    let Some(kind) = context.lowered.info.get_binder_kind(binder_id) else {
        return false;
    };
    match kind {
        lowering::BinderKind::Variable { .. } | lowering::BinderKind::Wildcard => false,
        lowering::BinderKind::Named { binder, .. } => {
            binder.is_some_and(|id| requires_instantiation(context, id))
        }
        lowering::BinderKind::Parenthesized { parenthesized } => {
            parenthesized.is_some_and(|id| requires_instantiation(context, id))
        }
        lowering::BinderKind::Typed { binder, type_ } => {
            type_.is_some_and(|id| type_annotation_requires_instantiation(context, id))
                || binder.is_some_and(|id| requires_instantiation(context, id))
        }
        _ => true,
    }
}

fn type_annotation_requires_instantiation<Q>(
    context: &CheckContext<Q>,
    type_id: lowering::TypeId,
) -> bool
where
    Q: ExternalQueries,
{
    let Some(kind) = context.lowered.info.get_type_kind(type_id) else {
        return false;
    };
    match kind {
        lowering::TypeKind::Forall { .. } => false,
        lowering::TypeKind::Kinded { type_, .. } => {
            type_.is_some_and(|id| type_annotation_requires_instantiation(context, id))
        }
        lowering::TypeKind::Parenthesized { parenthesized } => {
            parenthesized.is_some_and(|id| type_annotation_requires_instantiation(context, id))
        }
        _ => true,
    }
}

/// Instantiates pattern types for binders that require instantiation.
///
/// For equations, the `types` are usually the types of the arguments while
/// the `binders` are the syntactic arguments themselves. For example:
///
/// ```purescript
/// unbox :: (forall a. Box a) -> forall a. a
/// unbox (Box a) = a
/// ```
///
/// The argument `forall a. Box a` will be instantiated if at least one of its
/// syntactic arguments demands it. For case expressions, the `types` are the
/// types of the scrutinees. For example:
///
/// ```purescript
/// box :: forall a. Box a
/// box = ...
///
/// case box of
///   Box a -> a
/// ```
pub fn instantiate_pattern_column_types<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    types: &mut [TypeId],
    binders: impl IntoIterator<Item = (usize, lowering::BinderId)>,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let columns = binders.into_iter().filter_map(|(position, binder_id)| {
        requires_instantiation(context, binder_id).then_some(position)
    });

    for column in columns {
        if let Some(column_type) = types.get_mut(column) {
            *column_type = toolkit::instantiate_unifications(state, context, *column_type)?;
        }
    }

    Ok(())
}

fn binder_core<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder_id: lowering::BinderId,
    mode: BinderMode,
) -> QueryResult<ElaboratedBinder>
where
    Q: ExternalQueries,
{
    let unknown = context.unknown("missing binder");

    let Some(kind) = context.lowered.info.get_binder_kind(binder_id) else {
        return Ok(allocate_checked_binder(state, binder_id, unknown, tree::BinderKind::Error));
    };

    let (binder_type, binder_kind) = match kind {
        lowering::BinderKind::Typed { binder, type_ } => {
            let Some(nested_binder) = binder else {
                return Ok(allocate_checked_binder(
                    state,
                    binder_id,
                    unknown,
                    tree::BinderKind::Error,
                ));
            };
            let Some(type_id) = type_ else {
                return Ok(allocate_checked_binder(
                    state,
                    binder_id,
                    unknown,
                    tree::BinderKind::Error,
                ));
            };

            let (type_id, _) = types::infer_kind(state, context, *type_id)?;
            match mode {
                BinderMode::Check { elaborating: false, .. } => {
                    check_argument_binder(state, context, *nested_binder, type_id)?;
                }
                _ => {
                    check_binder(state, context, *nested_binder, type_id)?;
                }
            }

            if let BinderMode::Check { expected_type, elaborating } = mode {
                subtype_for_mode(state, context, type_id, expected_type, elaborating)?;
            }

            (type_id, tree::BinderKind::Error)
        }

        lowering::BinderKind::OperatorChain { .. } => {
            let (_, inferred_type) = operator::infer_operator_chain(state, context, binder_id)?;

            if let BinderMode::Check { expected_type, elaborating } = mode {
                subtype_for_mode(state, context, inferred_type, expected_type, elaborating)?;
            }

            (inferred_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::Integer { .. } => {
            let inferred_type = context.prim.int;

            if let BinderMode::Check { expected_type, .. } = mode {
                unification::unify(state, context, inferred_type, expected_type)?;
            }

            (inferred_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::Number { .. } => {
            let inferred_type = context.prim.number;

            if let BinderMode::Check { expected_type, .. } = mode {
                unification::unify(state, context, inferred_type, expected_type)?;
            }

            (inferred_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::Constructor { resolution, arguments } => {
            let Some((file_id, term_id)) = resolution else {
                return Ok(allocate_checked_binder(
                    state,
                    binder_id,
                    unknown,
                    tree::BinderKind::Error,
                ));
            };

            let mut constructor_t = toolkit::lookup_file_term(state, context, *file_id, *term_id)?;
            let mut checked_arguments = vec![];

            let inferred_type = if arguments.is_empty() {
                constructor_t = toolkit::instantiate_unifications(state, context, constructor_t)?;
                toolkit::without_constraints(state, context, constructor_t)?
            } else {
                for &argument in arguments.iter() {
                    let (result, checked_argument) = check_constructor_binder_application(
                        state,
                        context,
                        constructor_t,
                        argument,
                    )?;
                    constructor_t = result;
                    checked_arguments.push(checked_argument.binder);
                }
                constructor_t
            };

            let binder_type = if let BinderMode::Check { expected_type, elaborating } = mode {
                subtype_for_mode(state, context, inferred_type, expected_type, elaborating)?;
                expected_type
            } else {
                inferred_type
            };
            let binder_kind = tree::BinderKind::Constructor {
                resolution: (*file_id, *term_id),
                arguments: Arc::from(checked_arguments),
            };
            (binder_type, binder_kind)
        }

        lowering::BinderKind::Variable { variable } => {
            let binder_type = match mode {
                BinderMode::Infer => state.fresh_unification(context.queries, context.prim.t),
                BinderMode::Check { expected_type, .. } => expected_type,
            };
            let binder_kind = if variable.is_some() {
                tree::BinderKind::Variable
            } else {
                tree::BinderKind::Error
            };
            (binder_type, binder_kind)
        }

        lowering::BinderKind::Named { binder, .. } => {
            let Some(binder) = binder else {
                return Ok(allocate_checked_binder(
                    state,
                    binder_id,
                    unknown,
                    tree::BinderKind::Error,
                ));
            };

            let type_id = match mode {
                BinderMode::Infer => infer_binder(state, context, *binder)?.type_id,
                BinderMode::Check { expected_type, elaborating } => {
                    if elaborating {
                        check_binder(state, context, *binder, expected_type)?.type_id
                    } else {
                        check_argument_binder(state, context, *binder, expected_type)?.type_id
                    }
                }
            };

            (type_id, tree::BinderKind::Error)
        }

        lowering::BinderKind::Wildcard => {
            let binder_type = match mode {
                BinderMode::Infer => state.fresh_unification(context.queries, context.prim.t),
                BinderMode::Check { expected_type, .. } => expected_type,
            };
            (binder_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::String { .. } => {
            let inferred_type = context.prim.string;

            if let BinderMode::Check { expected_type, .. } = mode {
                unification::unify(state, context, inferred_type, expected_type)?;
            }

            (inferred_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::Char { .. } => {
            let inferred_type = context.prim.char;

            if let BinderMode::Check { expected_type, .. } = mode {
                unification::unify(state, context, inferred_type, expected_type)?;
            }

            (inferred_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::Boolean { .. } => {
            let inferred_type = context.prim.boolean;

            if let BinderMode::Check { expected_type, .. } = mode {
                unification::unify(state, context, inferred_type, expected_type)?;
            }

            (inferred_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::Array { array } => {
            let element_type = state.fresh_unification(context.queries, context.prim.t);

            for binder in array.iter() {
                let binder_type = infer_binder(state, context, *binder)?.type_id;
                unification::subtype_with::<unification::NonElaborating, Q>(
                    state,
                    context,
                    binder_type,
                    element_type,
                )?;
            }

            let array_type = context.intern_application(context.prim.array, element_type);

            if let BinderMode::Check { expected_type, elaborating } = mode {
                subtype_for_mode(state, context, array_type, expected_type, elaborating)?;
            }

            (array_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::Record { record } => {
            let binder_type = if let BinderMode::Check { expected_type, elaborating } = mode {
                check_record_binder(state, context, binder_id, record, expected_type, elaborating)?
            } else {
                infer_record_binder(state, context, binder_id, record)?
            };
            (binder_type, tree::BinderKind::Error)
        }

        lowering::BinderKind::Parenthesized { parenthesized } => {
            let Some(parenthesized) = parenthesized else {
                return Ok(allocate_checked_binder(
                    state,
                    binder_id,
                    unknown,
                    tree::BinderKind::Error,
                ));
            };
            let elaborated = binder_core(state, context, *parenthesized, mode)?;
            state.checked.nodes.binders.insert(binder_id, elaborated.type_id);
            return Ok(elaborated);
        }
    };

    Ok(allocate_checked_binder(state, binder_id, binder_type, binder_kind))
}

fn allocate_checked_binder(
    state: &mut CheckState,
    source: lowering::BinderId,
    type_id: TypeId,
    kind: tree::BinderKind,
) -> ElaboratedBinder {
    state.checked.nodes.binders.insert(source, type_id);
    let binder = match kind {
        tree::BinderKind::Error => state.allocate_error_binder(source, type_id),
        kind => state.allocate_binder(source, type_id, kind),
    };
    ElaboratedBinder { type_id, binder }
}

fn subtype_for_mode<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    t1: TypeId,
    t2: TypeId,
    elaborating: bool,
) -> QueryResult<bool>
where
    Q: ExternalQueries,
{
    if elaborating {
        unification::subtype(state, context, t1, t2)
    } else {
        unification::subtype_with::<unification::NonElaborating, Q>(state, context, t1, t2)
    }
}

fn check_constructor_binder_application<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    mut constructor: TypeId,
    binder_id: lowering::BinderId,
) -> QueryResult<(TypeId, ElaboratedBinder)>
where
    Q: ExternalQueries,
{
    safe_loop! {
        match application::analyse_callable_head(state, context, constructor)? {
            application::CallableAnalysis::Forall { binder, body } => {
                let (_, result) =
                    application::instantiate_callable_forall(state, context, binder, body)?;
                constructor = result;
            }
            application::CallableAnalysis::Constraint { result, .. } => constructor = result,
            application::CallableAnalysis::Function { argument, result } => {
                let binder = check_binder(state, context, binder_id, argument)?;
                break Ok((result, binder));
            }
            application::CallableAnalysis::NotCallable => {
                let unknown = context.unknown("invalid function application");
                let binder = allocate_checked_binder(
                    state,
                    binder_id,
                    unknown,
                    tree::BinderKind::Error,
                );
                break Ok((unknown, binder));
            }
        }
    }
}

enum PatternItem {
    Field(lowering::BinderId),
    Pun(lowering::RecordPunId),
}

fn collect_pattern_items(record: &[lowering::BinderRecordItem]) -> Vec<(SmolStr, PatternItem)> {
    let mut items = vec![];
    for field in record {
        match field {
            lowering::BinderRecordItem::RecordField { name, value } => {
                let Some(name) = name else { continue };
                let Some(value) = value else { continue };
                let name = SmolStr::clone(name);
                items.push((name, PatternItem::Field(*value)));
            }
            lowering::BinderRecordItem::RecordPun { id, name } => {
                let Some(name) = name else { continue };
                let name = SmolStr::clone(name);
                items.push((name, PatternItem::Pun(*id)));
            }
        }
    }
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items
}

fn check_pattern_item<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    item: &PatternItem,
    expected_type: TypeId,
    elaborating: bool,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    match *item {
        PatternItem::Field(binder_id) => {
            if elaborating {
                check_binder(state, context, binder_id, expected_type)?;
            } else {
                check_argument_binder(state, context, binder_id, expected_type)?;
            }
        }
        PatternItem::Pun(pun_id) => {
            state.checked.nodes.puns.insert(pun_id, expected_type);
        }
    }
    Ok(())
}

fn infer_record_binder<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder_id: lowering::BinderId,
    record: &[lowering::BinderRecordItem],
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let mut fields = vec![];

    for field in record.iter() {
        match field {
            lowering::BinderRecordItem::RecordField { name, value } => {
                let Some(name) = name else { continue };
                let Some(value) = value else { continue };

                let label = SmolStr::clone(name);
                let id = infer_binder(state, context, *value)?.type_id;
                fields.push(RowField { label, id });
            }
            lowering::BinderRecordItem::RecordPun { id, name } => {
                let Some(name) = name else { continue };

                let label = SmolStr::clone(name);
                let field_type = state.fresh_unification(context.queries, context.prim.t);

                state.checked.nodes.puns.insert(*id, field_type);
                fields.push(RowField { label, id: field_type });
            }
        }
    }

    let row_tail = state.fresh_unification(context.queries, context.prim.row_type);
    let row_type = context.intern_row(fields, Some(row_tail));
    let record_type = context.intern_application(context.prim.record, row_type);

    state.checked.nodes.binders.insert(binder_id, record_type);
    Ok(record_type)
}

fn extract_expected_row<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    expected_type: TypeId,
) -> QueryResult<Option<RowType>>
where
    Q: ExternalQueries,
{
    let expected_type = normalise::expand(state, context, expected_type)?;
    let Type::Application(function, argument) = context.lookup_type(expected_type) else {
        return Ok(None);
    };
    let function = normalise::expand(state, context, function)?;
    if function != context.prim.record {
        return Ok(None);
    }
    let row = normalise::expand(state, context, argument)?;
    let Type::Row(row_id) = context.lookup_type(row) else {
        return Ok(None);
    };
    Ok(Some(context.lookup_row_type(row_id)))
}

fn check_record_binder<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    binder_id: lowering::BinderId,
    record: &[lowering::BinderRecordItem],
    expected_type: TypeId,
    elaborating: bool,
) -> QueryResult<TypeId>
where
    Q: ExternalQueries,
{
    let pattern_items = collect_pattern_items(record);

    let expected_type = normalise::expand(state, context, expected_type)?;

    let expected_row = if let Type::Application(function, _) = context.lookup_type(expected_type) {
        let function = normalise::expand(state, context, function)?;
        if function == context.prim.record {
            extract_expected_row(state, context, expected_type)?
        } else {
            None
        }
    } else {
        None
    };

    let Some(expected_row) = expected_row else {
        let result = infer_record_binder(state, context, binder_id, record)?;
        unification::unify(state, context, result, expected_type)?;
        return Ok(expected_type);
    };

    let mut extra_fields = vec![];

    let patterns = pattern_items.iter();
    let expected = expected_row.fields.iter();

    for pair in patterns.merge_join_by(expected, |pattern, expected| pattern.0.cmp(&expected.label))
    {
        match pair {
            EitherOrBoth::Both((_, item), expected) => {
                check_pattern_item(state, context, item, expected.id, elaborating)?;
            }
            EitherOrBoth::Left((label, item)) => {
                let id = state.fresh_unification(context.queries, context.prim.t);
                check_pattern_item(state, context, item, id, elaborating)?;

                let label = SmolStr::clone(label);
                extra_fields.push(RowField { label, id });
            }
            EitherOrBoth::Right(_) => (),
        }
    }

    if !extra_fields.is_empty() {
        if let Some(tail) = expected_row.tail {
            let row_tail = state.fresh_unification(context.queries, context.prim.row_type);

            let row_type = context.intern_row(extra_fields, Some(row_tail));

            unification::unify(state, context, tail, row_type)?;
        } else {
            let labels = extra_fields.into_iter().map(|field| field.label);
            state.insert_error(ErrorKind::AdditionalProperty { labels: Arc::from_iter(labels) });
        }
    }

    state.checked.nodes.binders.insert(binder_id, expected_type);
    Ok(expected_type)
}
