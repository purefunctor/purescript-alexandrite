use std::sync::Arc;

use building_types::QueryResult;
use smol_str::format_smolstr;

use crate::context::CheckContext;
use crate::core::{KindOrType, TypeId, toolkit};
use crate::evidence::Evidence;
use crate::source::derive::builder::DerivedTreeBuilder;
use crate::source::derive::{field, tools};
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
            DeriveDispatch::Eq => generate_eq_member(state, context, result, &freshened.arguments)?,
            DeriveDispatch::Ord => {
                generate_ord_member(state, context, result, &freshened.arguments)?
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

struct ComparisonMember {
    resolution: (files::FileId, indexing::TermItemId),
    implementation_type: TypeId,
    constructors: Vec<AppliedConstructor>,
}

struct AppliedConstructor {
    file_id: files::FileId,
    item_id: indexing::TermItemId,
    fields: Vec<TypeId>,
}

fn resolve_comparison_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
) -> QueryResult<Option<ComparisonMember>>
where
    Q: ExternalQueries,
{
    let DeriveStrategy::FieldConstraints { data_file, data_id, .. } = result.strategy else {
        return Ok(None);
    };
    let Some(KindOrType::Type(derived_type)) = instance_arguments.last() else {
        return Ok(None);
    };

    let (_, type_arguments) = toolkit::extract_all_applications(state, context, *derived_type)?;

    let constructor_ids = tools::lookup_data_constructors(context, data_file, data_id)?;
    let mut constructors = Vec::with_capacity(constructor_ids.len());

    for constructor_id in constructor_ids {
        let constructor_type =
            toolkit::lookup_file_term(state, context, data_file, constructor_id)?;

        let fields = field::instantiate_constructor_fields(
            state,
            context,
            constructor_type,
            &type_arguments,
        )?;
        for &field in &fields {
            if field::requires_lifted_comparison(state, context, field)? {
                return Ok(None);
            }
        }

        constructors.push(AppliedConstructor {
            file_id: data_file,
            item_id: constructor_id,
            fields,
        });
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

    Ok(Some(ComparisonMember { resolution, implementation_type, constructors }))
}

fn generate_eq_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
) -> QueryResult<Option<tree::InstanceMember>>
where
    Q: ExternalQueries,
{
    let Some(member) = resolve_comparison_member(state, context, result, instance_arguments)?
    else {
        return Ok(None);
    };
    let Some(operation) = context.known_terms.eq else {
        return Ok(None);
    };

    let Some(body) = emit_eq(state, context, result.derive_id, &member, operation)? else {
        return Ok(None);
    };

    let member = generated_member(
        result.derive_id,
        member.resolution,
        member.implementation_type,
        vec![],
        body,
    );
    Ok(Some(member))
}

fn generate_ord_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
) -> QueryResult<Option<tree::InstanceMember>>
where
    Q: ExternalQueries,
{
    let Some(member) = resolve_comparison_member(state, context, result, instance_arguments)?
    else {
        return Ok(None);
    };
    let Some(operation) = context.known_terms.compare else {
        return Ok(None);
    };
    let Some(equal) = context.known_terms.ordering_eq else {
        return Ok(None);
    };
    let constructor_ordering = if member.constructors.len() >= 2 {
        let Some(less) = context.known_terms.ordering_lt else {
            return Ok(None);
        };
        let Some(greater) = context.known_terms.ordering_gt else {
            return Ok(None);
        };
        Some(ConstructorOrdering { less, greater })
    } else {
        None
    };

    let Some(body) = emit_ord(
        state,
        context,
        result.derive_id,
        &member,
        operation,
        equal,
        constructor_ordering,
    )?
    else {
        return Ok(None);
    };

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

#[derive(Clone, Copy)]
struct ConstructorOrdering {
    less: (files::FileId, indexing::TermItemId),
    greater: (files::FileId, indexing::TermItemId),
}

struct BinaryComparisonMember {
    implementation_type: TypeId,
    left_type: TypeId,
    right_type: TypeId,
    result_type: TypeId,
}

impl BinaryComparisonMember {
    fn decode<Q>(
        state: &mut CheckState,
        context: &CheckContext<Q>,
        implementation_type: TypeId,
    ) -> QueryResult<Option<BinaryComparisonMember>>
    where
        Q: ExternalQueries,
    {
        let toolkit::InspectFunction { arguments, result } =
            toolkit::inspect_function(state, context, implementation_type)?;
        let [left_type, right_type] = arguments.as_slice() else {
            return Ok(None);
        };
        Ok(Some(BinaryComparisonMember {
            implementation_type,
            left_type: *left_type,
            right_type: *right_type,
            result_type: result,
        }))
    }
}

struct ComparisonParameters {
    left_binder: tree::BinderId,
    right_binder: tree::BinderId,
    left_expression: ElaboratedExpression,
    right_expression: ElaboratedExpression,
}

struct ConstructorComparison {
    left_pattern: tree::BinderId,
    right_pattern: tree::BinderId,
    comparisons: Vec<ElaboratedExpression>,
}

struct GeneratedFieldPair {
    left: tree::BinderId,
    right: tree::BinderId,
    comparison: ElaboratedExpression,
}

fn emit_eq<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    derive_id: indexing::DeriveId,
    comparison_member: &ComparisonMember,
    operation: (files::FileId, indexing::TermItemId),
) -> QueryResult<Option<ElaboratedExpression>>
where
    Q: ExternalQueries,
{
    let Some(member) =
        BinaryComparisonMember::decode(state, context, comparison_member.implementation_type)?
    else {
        return Ok(None);
    };

    let mut builder = DerivedTreeBuilder::new(state, context, derive_id);
    let parameters = emit_parameters(&mut builder, &member);

    let mut alternatives = vec![];
    for constructor in &comparison_member.constructors {
        let Some(comparison) =
            emit_constructor_comparison(&mut builder, constructor, &member, operation)?
        else {
            return Ok(None);
        };

        let body = emit_conjunction(&mut builder, member.result_type, comparison.comparisons);
        let patterns = vec![comparison.left_pattern, comparison.right_pattern];

        alternatives.push(builder.alternative(patterns, body));
    }

    if let Some(alternative) = emit_mismatched_constructor_alternative(
        &mut builder,
        &comparison_member.constructors,
        &member,
    ) {
        alternatives.push(alternative);
    }

    let body = emit_lambda_case(&mut builder, member, parameters, alternatives);
    Ok(Some(body))
}

fn emit_mismatched_constructor_alternative<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    constructors: &[AppliedConstructor],
    member: &BinaryComparisonMember,
) -> Option<tree::CaseAlternative>
where
    Q: ExternalQueries,
{
    let ([] | [_, _, ..]) = constructors else {
        return None;
    };
    let left = builder.wildcard_pattern("left", member.left_type);
    let right = builder.wildcard_pattern("right", member.right_type);
    let body = builder.boolean(member.result_type, false);
    Some(builder.alternative(vec![left, right], body))
}

fn emit_ord<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    derive_id: indexing::DeriveId,
    comparison_member: &ComparisonMember,
    operation: (files::FileId, indexing::TermItemId),
    equal: (files::FileId, indexing::TermItemId),
    constructor_ordering: Option<ConstructorOrdering>,
) -> QueryResult<Option<ElaboratedExpression>>
where
    Q: ExternalQueries,
{
    let Some(member) =
        BinaryComparisonMember::decode(state, context, comparison_member.implementation_type)?
    else {
        return Ok(None);
    };

    let mut builder = DerivedTreeBuilder::new(state, context, derive_id);
    let parameters = emit_parameters(&mut builder, &member);
    if comparison_member.constructors.is_empty() {
        let body = emit_ordering_constant(&mut builder, equal, member.result_type)?;
        let body = builder.lambda(
            member.implementation_type,
            vec![parameters.left_binder, parameters.right_binder],
            body,
        );
        return Ok(Some(body));
    }

    let Some(alternatives) = emit_ord_alternatives(
        &mut builder,
        &comparison_member.constructors,
        &member,
        operation,
        equal,
        constructor_ordering,
    )?
    else {
        return Ok(None);
    };

    let body = emit_lambda_case(&mut builder, member, parameters, alternatives);
    Ok(Some(body))
}

fn emit_ord_alternatives<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    constructors: &[AppliedConstructor],
    member: &BinaryComparisonMember,
    operation: (files::FileId, indexing::TermItemId),
    equal: (files::FileId, indexing::TermItemId),
    constructor_ordering: Option<ConstructorOrdering>,
) -> QueryResult<Option<Vec<tree::CaseAlternative>>>
where
    Q: ExternalQueries,
{
    let mut alternatives = vec![];
    let mut remaining = constructors;
    while let Some((constructor, later_constructors)) = remaining.split_first() {
        let Some(comparison) =
            emit_constructor_comparison(builder, constructor, member, operation)?
        else {
            return Ok(None);
        };
        let body = emit_lexicographic(builder, comparison.comparisons, member.result_type, equal)?;
        let patterns = vec![comparison.left_pattern, comparison.right_pattern];
        alternatives.push(builder.alternative(patterns, body));

        if let Some(ordering) = constructor_ordering {
            emit_constructor_ordering_alternatives(
                builder,
                constructor,
                later_constructors,
                member,
                ordering,
                &mut alternatives,
            )?;
        }

        remaining = later_constructors;
    }
    Ok(Some(alternatives))
}

fn emit_constructor_ordering_alternatives<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    constructor: &AppliedConstructor,
    later_constructors: &[AppliedConstructor],
    member: &BinaryComparisonMember,
    ordering: ConstructorOrdering,
    alternatives: &mut Vec<tree::CaseAlternative>,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    for later in later_constructors {
        let left = emit_constructor_wildcard(builder, constructor, member.left_type);
        let right = emit_constructor_wildcard(builder, later, member.right_type);
        let body = emit_ordering_constant(builder, ordering.less, member.result_type)?;
        alternatives.push(builder.alternative(vec![left, right], body));

        let left = emit_constructor_wildcard(builder, later, member.left_type);
        let right = emit_constructor_wildcard(builder, constructor, member.right_type);
        let body = emit_ordering_constant(builder, ordering.greater, member.result_type)?;
        alternatives.push(builder.alternative(vec![left, right], body));
    }

    Ok(())
}

fn emit_parameters<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    member: &BinaryComparisonMember,
) -> ComparisonParameters
where
    Q: ExternalQueries,
{
    let left_binder = builder.variable_binder("left", member.left_type);
    let right_binder = builder.variable_binder("right", member.right_type);
    let left_expression = builder.variable(left_binder);
    let right_expression = builder.variable(right_binder);
    ComparisonParameters { left_binder, right_binder, left_expression, right_expression }
}

fn emit_constructor_comparison<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    constructor: &AppliedConstructor,
    member: &BinaryComparisonMember,
    operation: (files::FileId, indexing::TermItemId),
) -> QueryResult<Option<ConstructorComparison>>
where
    Q: ExternalQueries,
{
    let mut fields = Vec::with_capacity(constructor.fields.len());
    for (index, &field_type) in constructor.fields.iter().enumerate() {
        let Some(field) = emit_field_comparison(builder, operation, index, field_type)? else {
            return Ok(None);
        };
        fields.push(field);
    }

    let left_fields = fields.iter().map(|field| field.left);
    let left_fields = left_fields.collect();
    let left_pattern = builder.constructor_pattern(
        "constructor",
        member.left_type,
        (constructor.file_id, constructor.item_id),
        left_fields,
    );

    let right_fields = fields.iter().map(|field| field.right);
    let right_fields = right_fields.collect();
    let right_pattern = builder.constructor_pattern(
        "constructor",
        member.right_type,
        (constructor.file_id, constructor.item_id),
        right_fields,
    );

    let comparisons = fields.into_iter().map(|field| field.comparison);
    let comparisons = comparisons.collect();
    Ok(Some(ConstructorComparison { left_pattern, right_pattern, comparisons }))
}

fn emit_field_comparison<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    operation: (files::FileId, indexing::TermItemId),
    index: usize,
    field_type: TypeId,
) -> QueryResult<Option<GeneratedFieldPair>>
where
    Q: ExternalQueries,
{
    let left = builder.variable_binder(&format_smolstr!("left{index}"), field_type);
    let right = builder.variable_binder(&format_smolstr!("right{index}"), field_type);
    let left_value = builder.variable(left);
    let right_value = builder.variable(right);
    let Some(comparison) = emit_comparison(builder, operation, left_value, right_value)? else {
        return Ok(None);
    };
    Ok(Some(GeneratedFieldPair { left, right, comparison }))
}

fn emit_comparison<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    operation: (files::FileId, indexing::TermItemId),
    left: ElaboratedExpression,
    right: ElaboratedExpression,
) -> QueryResult<Option<ElaboratedExpression>>
where
    Q: ExternalQueries,
{
    let comparison = builder.term_reference(operation)?;
    let Some(comparison) = builder.apply(comparison, left)? else {
        return Ok(None);
    };
    builder.apply(comparison, right)
}

fn emit_constructor_wildcard<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    constructor: &AppliedConstructor,
    type_id: TypeId,
) -> tree::BinderId
where
    Q: ExternalQueries,
{
    let fields = constructor
        .fields
        .iter()
        .map(|&field_type| builder.wildcard_pattern("field", field_type))
        .collect();
    builder.constructor_pattern(
        "constructor",
        type_id,
        (constructor.file_id, constructor.item_id),
        fields,
    )
}

fn emit_conjunction<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    result_type: TypeId,
    comparisons: Vec<ElaboratedExpression>,
) -> ElaboratedExpression
where
    Q: ExternalQueries,
{
    let mut result = builder.boolean(result_type, true);
    for comparison in comparisons.into_iter().rev() {
        let false_ = builder.boolean(result_type, false);
        result = builder.if_then_else(result_type, comparison, result, false_);
    }
    result
}

fn emit_lexicographic<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    comparisons: Vec<ElaboratedExpression>,
    result_type: TypeId,
    equal: (files::FileId, indexing::TermItemId),
) -> QueryResult<ElaboratedExpression>
where
    Q: ExternalQueries,
{
    let mut result = emit_ordering_constant(builder, equal, result_type)?;
    for comparison in comparisons.into_iter().rev() {
        let equal_pattern = builder.constructor_pattern("EQ", result_type, equal, vec![]);
        let other = builder.variable_binder("ordering", result_type);
        let other_expression = builder.variable(other);
        let alternatives = vec![
            builder.alternative(vec![equal_pattern], result),
            builder.alternative(vec![other], other_expression),
        ];
        result = builder.case(result_type, vec![comparison], alternatives);
    }
    Ok(result)
}

fn emit_ordering_constant<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    resolution: (files::FileId, indexing::TermItemId),
    expected: TypeId,
) -> QueryResult<ElaboratedExpression>
where
    Q: ExternalQueries,
{
    let expression = builder.term_reference(resolution)?;
    builder.subtype(expression, expected)
}

fn emit_lambda_case<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    member: BinaryComparisonMember,
    parameters: ComparisonParameters,
    alternatives: Vec<tree::CaseAlternative>,
) -> ElaboratedExpression
where
    Q: ExternalQueries,
{
    let body = builder.case(
        member.result_type,
        vec![parameters.left_expression, parameters.right_expression],
        alternatives,
    );

    builder.lambda(
        member.implementation_type,
        vec![parameters.left_binder, parameters.right_binder],
        body,
    )
}
