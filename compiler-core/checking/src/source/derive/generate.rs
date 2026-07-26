use std::sync::Arc;

use building_types::QueryResult;

use crate::context::CheckContext;
use crate::core::{KindOrType, TypeId, toolkit};
use crate::evidence::Evidence;
use crate::source::derive::builder::DerivedTreeBuilder;
use crate::source::derive::tools;
use crate::source::term_items::{
    emit_instance_superclass_constraints, freshen_instance_rigids, instantiate_class_member_type,
};
use crate::source::terms::ElaboratedExpression;
use crate::state::CheckState;
use crate::{ExternalQueries, tree};

use super::{DeriveDispatch, DeriveHeadResult, DeriveStrategy, derive_dispatch};

pub(crate) fn generate_instance<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let dispatch = derive_dispatch(context, result.class_file, result.class_id);
    match dispatch {
        DeriveDispatch::Eq | DeriveDispatch::Ord => {
            generate_known_instance(state, context, result, dispatch)
        }
        _ => Ok(()),
    }
}

fn generate_known_instance<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    dispatch: DeriveDispatch,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let Some(instance) = toolkit::instance_info(
        state,
        context,
        result.signature,
        (result.class_file, result.class_id),
    )?
    else {
        return Ok(());
    };

    let freshened = freshen_instance_rigids(state, context, &instance)?;
    state.with_implicit(context, &freshened.substitution, |state| {
        let mut evidences = Vec::with_capacity(freshened.constraints.len());
        for (&constraint, &signature_constraint) in
            std::iter::zip(&freshened.constraints, &instance.constraints)
        {
            let evidence = Evidence::Given(state.push_given(constraint));
            evidences.push(tree::InstanceEvidence { constraint: signature_constraint, evidence });
        }

        let superclasses = emit_instance_superclass_constraints(
            state,
            context,
            result.class_file,
            result.class_id,
            &freshened.arguments,
        )?;

        let member = match dispatch {
            DeriveDispatch::Eq => {
                generate_nullary_eq_member(state, context, result, &freshened.arguments)?
            }
            DeriveDispatch::Ord => {
                generate_nullary_ord_member(state, context, result, &freshened.arguments)?
            }
            _ => None,
        };
        let Some(member) = member else {
            return Ok(());
        };

        let instance = tree::InstanceDeclaration {
            class: (result.class_file, result.class_id),
            rigid_parameters: Arc::from(freshened.rigids),
            evidences: Arc::from(evidences),
            superclasses: Arc::from(superclasses),
            implementation: tree::InstanceImplementation::Members(Arc::from([member])),
        };
        let declaration = tree::TermDeclaration {
            type_id: result.signature,
            kind: tree::TermDeclarationKind::Instance(instance),
        };
        state.checked.tree.insert_term(result.item_id, declaration);

        Ok(())
    })
}

struct NullaryMember {
    resolution: (files::FileId, indexing::TermItemId),
    implementation_type: TypeId,
    constructor: (files::FileId, indexing::TermItemId),
}

fn resolve_nullary_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
) -> QueryResult<Option<NullaryMember>>
where
    Q: ExternalQueries,
{
    let DeriveStrategy::FieldConstraints { data_file, data_id, .. } = result.strategy else {
        return Ok(None);
    };

    let constructors = tools::lookup_data_constructors(context, data_file, data_id)?;
    let [constructor_id] = constructors.as_slice() else {
        return Ok(None);
    };

    let constructor_type = toolkit::lookup_file_term(state, context, data_file, *constructor_id)?;
    let constructor = toolkit::inspect_function(state, context, constructor_type)?;
    if !constructor.arguments.is_empty() {
        return Ok(None);
    }

    let Some(class) =
        toolkit::lookup_file_class(state, context, result.class_file, result.class_id)?
    else {
        return Ok(None);
    };

    let [member] = class.members.as_slice() else {
        return Ok(None);
    };

    let resolution = (result.class_file, member.item_id);
    let Some(implementation_type) = instantiate_class_member_type(
        state,
        context,
        resolution,
        (result.class_file, result.class_id),
        instance_arguments,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(NullaryMember {
        resolution,
        implementation_type,
        constructor: (data_file, *constructor_id),
    }))
}

fn generate_nullary_eq_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
) -> QueryResult<Option<tree::InstanceMember>>
where
    Q: ExternalQueries,
{
    let Some(member) = resolve_nullary_member(state, context, result, instance_arguments)? else {
        return Ok(None);
    };

    let body = generate_nullary_body(
        state,
        context,
        result.derive_id,
        member.implementation_type,
        member.constructor,
        NullaryResult::Boolean(true),
    )?;

    let member = generated_member(
        result.derive_id,
        member.resolution,
        member.implementation_type,
        vec![],
        body,
    );
    Ok(Some(member))
}

fn generate_nullary_ord_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
) -> QueryResult<Option<tree::InstanceMember>>
where
    Q: ExternalQueries,
{
    let Some(member) = resolve_nullary_member(state, context, result, instance_arguments)? else {
        return Ok(None);
    };
    let Some(equal) = context.known_terms.ordering_eq else {
        return Ok(None);
    };

    let body = generate_nullary_body(
        state,
        context,
        result.derive_id,
        member.implementation_type,
        member.constructor,
        NullaryResult::Reference(equal),
    )?;

    let member = generated_member(
        result.derive_id,
        member.resolution,
        member.implementation_type,
        vec![],
        body,
    );
    Ok(Some(member))
}

fn generated_member(
    derive_id: indexing::DeriveId,
    resolution: (files::FileId, indexing::TermItemId),
    implementation_type: TypeId,
    evidences: Vec<Evidence>,
    body: ElaboratedExpression,
) -> tree::InstanceMember {
    let where_expression = tree::WhereExpression::new(body.expression);
    let guarded_expression = tree::GuardedExpression::unconditional(where_expression);
    let equation =
        tree::Equation::generated(derive_id, resolution.1, Arc::from([]), guarded_expression);
    tree::InstanceMember {
        resolution,
        implementation_type,
        evidences: Arc::from(evidences),
        equations: Arc::from([equation]),
    }
}

enum NullaryResult {
    Boolean(bool),
    Reference((files::FileId, indexing::TermItemId)),
}

fn generate_nullary_body<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    derive_id: indexing::DeriveId,
    implementation_type: TypeId,
    constructor: (files::FileId, indexing::TermItemId),
    nullary_result: NullaryResult,
) -> QueryResult<ElaboratedExpression>
where
    Q: ExternalQueries,
{
    let toolkit::InspectFunction { arguments, result: result_type } =
        toolkit::inspect_function(state, context, implementation_type)?;
    let [left_type, right_type] = arguments.as_slice() else {
        panic!("comparison member must have exactly two value arguments")
    };

    let mut builder = DerivedTreeBuilder::new(state, context, derive_id);
    let left = builder.variable_binder("left", *left_type);
    let right = builder.variable_binder("right", *right_type);

    let left_expression = builder.variable(left);
    let right_expression = builder.variable(right);

    let left_pattern = builder.constructor_pattern("constructor", *left_type, constructor, vec![]);
    let right_pattern =
        builder.constructor_pattern("constructor", *right_type, constructor, vec![]);

    let equal = match nullary_result {
        NullaryResult::Boolean(value) => builder.boolean(result_type, value),
        NullaryResult::Reference(resolution) => {
            let reference = builder.term_reference(resolution)?;
            builder.subtype(reference, result_type)?
        }
    };
    let alternative = builder.alternative(vec![left_pattern, right_pattern], equal);
    let case =
        builder.case(result_type, vec![left_expression, right_expression], vec![alternative]);

    Ok(builder.lambda(implementation_type, vec![left, right], case))
}
