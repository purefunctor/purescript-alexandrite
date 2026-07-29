use std::sync::Arc;

use building_types::QueryResult;
use itertools::izip;
use smol_str::format_smolstr;

use crate::context::CheckContext;
use crate::core::substitute::{NameToType, SubstituteName};
use crate::core::{KindOrType, RowType, Type, TypeId, normalise, signature, toolkit};
use crate::evidence::Evidence;
use crate::source::derive::builder::DerivedTreeBuilder;
use crate::source::derive::{field, tools};
use crate::source::term_items::{
    emit_instance_superclass_constraints, freshen_instance_rigids, instantiate_class_member_type,
};
use crate::source::terms::ElaboratedExpression;
use crate::state::CheckState;
use crate::{ExternalQueries, tree};

use super::variance::{
    RecordFieldRecipe, TraversalOperation, TraversalParameter, VarianceRecipe,
};
use super::{DeriveDispatch, DeriveHeadResult, DeriveStrategy, derive_dispatch};

pub(crate) fn generate_instance<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    variance_recipe: Option<&VarianceRecipe>,
) -> QueryResult<Option<tree::TermDeclaration>>
where
    Q: ExternalQueries,
{
    if let DeriveStrategy::NewtypeDeriveConstraint { delegate_constraint } = result.strategy {
        return generate_delegate_instance(state, context, result, delegate_constraint);
    }

    let dispatch = derive_dispatch(context, result.class_file, result.class_id);
    match dispatch {
        DeriveDispatch::Eq
        | DeriveDispatch::Eq1
        | DeriveDispatch::Ord
        | DeriveDispatch::Ord1
        | DeriveDispatch::Functor => {
            generate_known_instance(state, context, result, dispatch, variance_recipe)
        }
        _ => Ok(None),
    }
}

fn generate_delegate_instance<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    delegate_constraint: TypeId,
) -> QueryResult<Option<tree::TermDeclaration>>
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
        return Ok(None);
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

        let delegate_constraint =
            SubstituteName::many(state, context, &freshened.substitution, delegate_constraint)?;
        let evidence = state.push_wanted(delegate_constraint);

        let instance = tree::InstanceDeclaration {
            class: (result.class_file, result.class_id),
            rigid_parameters: Arc::from(freshened.rigids),
            evidences: Arc::from(evidences),
            superclasses: Arc::from(superclasses),
            implementation: tree::InstanceImplementation::Delegate {
                constraint: delegate_constraint,
                evidence,
            },
        };
        let declaration = tree::TermDeclaration {
            type_id: result.signature,
            kind: tree::TermDeclarationKind::Instance(instance),
        };
        Ok(Some(declaration))
    })
}

fn generate_known_instance<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    dispatch: DeriveDispatch,
    variance_recipe: Option<&VarianceRecipe>,
) -> QueryResult<Option<tree::TermDeclaration>>
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
        return Ok(None);
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
            DeriveDispatch::Eq1 => generate_delegated_member(
                state,
                context,
                result,
                &freshened.arguments,
                context.known_terms.eq,
            )?,
            DeriveDispatch::Ord => {
                generate_ord_member(state, context, result, &freshened.arguments)?
            }
            DeriveDispatch::Ord1 => generate_delegated_member(
                state,
                context,
                result,
                &freshened.arguments,
                context.known_terms.compare,
            )?,
            DeriveDispatch::Functor => {
                let Some(recipe) = variance_recipe else { return Ok(None) };
                generate_functor_member(state, context, result, &freshened.arguments, recipe)?
            }
            _ => None,
        };

        let Some(member) = member else {
            return Ok(None);
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
        Ok(Some(declaration))
    })
}

struct ComparisonMember {
    member: ResolvedMember,
    constructors: Vec<AppliedConstructor>,
}

struct ResolvedMember {
    file_id: files::FileId,
    item_id: indexing::TermItemId,
    implementation_type: TypeId,
}

struct AppliedConstructor {
    file_id: files::FileId,
    item_id: indexing::TermItemId,
    fields: Vec<AppliedField>,
}

struct AppliedField {
    type_id: TypeId,
    comparison: field::ComparisonStyle,
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

        let field_types = field::instantiate_constructor_fields(
            state,
            context,
            constructor_type,
            &type_arguments,
        )?;

        let mut fields = Vec::with_capacity(field_types.len());
        for type_id in field_types {
            let comparison = field::comparison_style(state, context, type_id)?;
            fields.push(AppliedField { type_id, comparison });
        }

        constructors.push(AppliedConstructor {
            file_id: data_file,
            item_id: constructor_id,
            fields,
        });
    }

    let Some(member) = resolve_member(state, context, result, instance_arguments)? else {
        return Ok(None);
    };

    Ok(Some(ComparisonMember { member, constructors }))
}

fn resolve_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
) -> QueryResult<Option<ResolvedMember>>
where
    Q: ExternalQueries,
{
    let Some(class) =
        toolkit::lookup_file_class(state, context, result.class_file, result.class_id)?
    else {
        return Ok(None);
    };

    let [member] = class.members.as_slice() else {
        return Ok(None);
    };

    let file_id = result.class_file;
    let item_id = member.item_id;
    let Some(implementation_type) = instantiate_class_member_type(
        state,
        context,
        (file_id, item_id),
        (result.class_file, result.class_id),
        instance_arguments,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(ResolvedMember { file_id, item_id, implementation_type }))
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
    let Some(direct) = context.known_terms.eq else {
        return Ok(None);
    };
    let operations = ComparisonOperations { direct, lifted: context.known_terms.eq1 };

    let Some(body) = emit_eq(state, context, result.derive_id, &member, operations)? else {
        return Ok(None);
    };

    let member = generated_member(
        result.derive_id,
        (member.member.file_id, member.member.item_id),
        member.member.implementation_type,
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
    let Some(direct) = context.known_terms.compare else {
        return Ok(None);
    };
    let operations = ComparisonOperations { direct, lifted: context.known_terms.compare1 };
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
        operations,
        equal,
        constructor_ordering,
    )?
    else {
        return Ok(None);
    };

    let member = generated_member(
        result.derive_id,
        (member.member.file_id, member.member.item_id),
        member.member.implementation_type,
        vec![],
        body,
    );
    Ok(Some(member))
}

fn generate_delegated_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
    operation: Option<(files::FileId, indexing::TermItemId)>,
) -> QueryResult<Option<tree::InstanceMember>>
where
    Q: ExternalQueries,
{
    state.with_implication(|state| {
        let DeriveStrategy::DelegateConstraint { .. } = result.strategy else {
            return Ok(None);
        };
        let Some(operation) = operation else {
            return Ok(None);
        };
        let Some(member) = resolve_member(state, context, result, instance_arguments)? else {
            return Ok(None);
        };

        let signature::SkolemisedSignature { substitution, constraints, result: body_type, .. } =
            signature::expect_term_signature(state, context, member.implementation_type, 0)?;

        let mut evidences = Vec::with_capacity(constraints.len());
        for constraint in constraints {
            let evidence = Evidence::Given(state.push_given(constraint));
            evidences.push(evidence);
        }

        let body = state.with_implicit(context, &substitution, |state| {
            let mut builder = DerivedTreeBuilder::new(state, context, result.derive_id);
            let body = builder.term_reference(operation)?;
            builder.subtype(body, body_type)
        })?;

        let member = generated_member(
            result.derive_id,
            (member.file_id, member.item_id),
            member.implementation_type,
            evidences,
            body,
        );
        Ok(Some(member))
    })
}

struct InstantiatedDataType {
    type_id: TypeId,
    constructor_arguments: Vec<KindOrType>,
}

struct DecodedFunctorMember {
    member: ResolvedMember,
    substitution: NameToType,
    constraints: Vec<TypeId>,
    implementation_type: TypeId,
    function_type: TypeId,
    mapping_type: TypeId,
    data_file: files::FileId,
    source: InstantiatedDataType,
    target: InstantiatedDataType,
}

impl DecodedFunctorMember {
    fn decode<Q>(
        state: &mut CheckState,
        context: &CheckContext<Q>,
        member: ResolvedMember,
        data_file: files::FileId,
    ) -> QueryResult<Option<DecodedFunctorMember>>
    where
        Q: ExternalQueries,
    {
        let signature::SkolemisedSignature { substitution, constraints, arguments, result } =
            signature::expect_term_signature(state, context, member.implementation_type, 2)?;
        let [mapping_type, source_type] = arguments.as_slice() else {
            return Ok(None);
        };

        let (_, source_arguments) =
            toolkit::extract_all_applications(state, context, *source_type)?;
        let (_, target_arguments) = toolkit::extract_all_applications(state, context, result)?;
        let function_type = context.intern_function_iter([*mapping_type, *source_type], result);

        Ok(Some(DecodedFunctorMember {
            implementation_type: member.implementation_type,
            member,
            substitution,
            constraints,
            function_type,
            mapping_type: *mapping_type,
            data_file,
            source: InstantiatedDataType {
                type_id: *source_type,
                constructor_arguments: source_arguments,
            },
            target: InstantiatedDataType {
                type_id: result,
                constructor_arguments: target_arguments,
            },
        }))
    }
}

fn generate_functor_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
    recipe: &VarianceRecipe,
) -> QueryResult<Option<tree::InstanceMember>>
where
    Q: ExternalQueries,
{
    state.with_implication(|state| {
        let DeriveStrategy::VarianceConstraints { data_file, .. } = result.strategy else {
            return Ok(None);
        };

        let Some(member) = resolve_member(state, context, result, instance_arguments)? else {
            return Ok(None);
        };
        let Some(member) = DecodedFunctorMember::decode(state, context, member, data_file)? else {
            return Ok(None);
        };

        let mut evidences = Vec::with_capacity(member.constraints.len());
        for &constraint in &member.constraints {
            evidences.push(Evidence::Given(state.push_given(constraint)));
        }

        let body = state.with_implicit(context, &member.substitution, |state| {
            emit_functor(state, context, result.derive_id, &member, recipe)
        })?;
        let Some(body) = body else { return Ok(None) };

        Ok(Some(generated_member(
            result.derive_id,
            (member.member.file_id, member.member.item_id),
            member.implementation_type,
            evidences,
            body,
        )))
    })
}

fn emit_functor<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    derive_id: indexing::DeriveId,
    member: &DecodedFunctorMember,
    recipe: &VarianceRecipe,
) -> QueryResult<Option<ElaboratedExpression>>
where
    Q: ExternalQueries,
{
    let mut builder = DerivedTreeBuilder::new(state, context, derive_id);

    let mapping = builder.variable_binder("function", member.mapping_type);
    let value = builder.variable_binder("value", member.source.type_id);

    let mapping_expression = builder.variable(mapping);
    let value_expression = builder.variable(value);

    let mut alternatives = Vec::with_capacity(recipe.constructors.len());

    for constructor in &recipe.constructors {
        let Some(alternative) =
            emit_functor_alternative(&mut builder, member, constructor, mapping_expression)?
        else {
            return Ok(None);
        };
        alternatives.push(alternative);
    }

    let body = builder.case(member.target.type_id, vec![value_expression], alternatives);
    Ok(Some(builder.lambda(member.function_type, vec![mapping, value], body)))
}

fn emit_functor_alternative<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    member: &DecodedFunctorMember,
    constructor: &super::variance::ConstructorRecipe,
    mapping_expression: ElaboratedExpression,
) -> QueryResult<Option<tree::CaseAlternative>>
where
    Q: ExternalQueries,
{
    let constructor_type = toolkit::lookup_file_term(
        builder.state,
        builder.context,
        member.data_file,
        constructor.constructor_id,
    )?;
    let source_fields = field::instantiate_constructor_fields(
        builder.state,
        builder.context,
        constructor_type,
        &member.source.constructor_arguments,
    )?;
    let target_fields = field::instantiate_constructor_fields(
        builder.state,
        builder.context,
        constructor_type,
        &member.target.constructor_arguments,
    )?;

    if source_fields.len() != target_fields.len() || source_fields.len() != constructor.fields.len()
    {
        return Ok(None);
    }

    let mut emitter = FunctorTraversalEmitter { builder, mapping_expression };
    let mut binders = Vec::with_capacity(source_fields.len());
    let mut values = Vec::with_capacity(source_fields.len());

    for (index, (source, target, operation)) in
        izip!(&source_fields, &target_fields, &constructor.fields).enumerate()
    {
        let binder = emitter.builder.variable_binder(&format_smolstr!("field{index}"), *source);
        let value = emitter.builder.variable(binder);
        let value = if let Some(operation) = operation {
            let traversal =
                TraversalContext { source_type: *source, target_type: *target, function_depth: 0 };
            let Some(value) = emitter.emit_traversal(operation, value, traversal)? else {
                return Ok(None);
            };
            value
        } else {
            value
        };
        binders.push(binder);
        values.push(value);
    }

    let pattern = emitter.builder.constructor_pattern(
        "constructor",
        member.source.type_id,
        (member.data_file, constructor.constructor_id),
        binders,
    );

    let mut reconstructed =
        emitter.builder.term_reference((member.data_file, constructor.constructor_id))?;
    for value in values {
        let Some(applied) = emitter.builder.apply(reconstructed, value)? else { return Ok(None) };
        reconstructed = applied;
    }

    let reconstructed = emitter.builder.subtype(reconstructed, member.target.type_id)?;
    Ok(Some(emitter.builder.alternative(vec![pattern], reconstructed)))
}

#[derive(Clone, Copy)]
struct TraversalContext {
    source_type: TypeId,
    target_type: TypeId,
    function_depth: usize,
}

struct FunctorTraversalEmitter<'builder, 'state, 'context, 'queries, Q: ExternalQueries> {
    builder: &'builder mut DerivedTreeBuilder<'state, 'context, 'queries, Q>,
    mapping_expression: ElaboratedExpression,
}

impl<Q> FunctorTraversalEmitter<'_, '_, '_, '_, Q>
where
    Q: ExternalQueries,
{
    fn emit_traversal(
        &mut self,
        operation: &TraversalOperation,
        value: ElaboratedExpression,
        traversal: TraversalContext,
    ) -> QueryResult<Option<ElaboratedExpression>> {
        let TraversalContext { source_type, target_type, function_depth } = traversal;
        match operation {
            TraversalOperation::Parameter { parameter: TraversalParameter::First } => {
                let Some(mapped) = self.builder.apply(self.mapping_expression, value)? else {
                    return Ok(None);
                };
                Ok(Some(self.builder.subtype(mapped, target_type)?))
            }
            TraversalOperation::Parameter { parameter: TraversalParameter::Second } => Ok(None),
            TraversalOperation::Map { argument } => {
                let Some((_, source_argument)) = toolkit::decompose_type_application(
                    self.builder.state,
                    self.builder.context,
                    source_type,
                )?
                else {
                    return Ok(None);
                };
                let Some((_, target_argument)) = toolkit::decompose_type_application(
                    self.builder.state,
                    self.builder.context,
                    target_type,
                )?
                else {
                    return Ok(None);
                };
                let argument_context = TraversalContext {
                    source_type: source_argument,
                    target_type: target_argument,
                    function_depth,
                };
                let Some(transformer) = self.emit_transformer(argument, argument_context)? else {
                    return Ok(None);
                };
                let Some(map) = self.builder.context.known_terms.map else {
                    return Ok(None);
                };
                let map = self.builder.term_reference(map)?;
                let Some(map) = self.builder.apply(map, transformer)? else {
                    return Ok(None);
                };
                let Some(mapped) = self.builder.apply(map, value)? else {
                    return Ok(None);
                };
                Ok(Some(self.builder.subtype(mapped, target_type)?))
            }
            TraversalOperation::Function { argument, result } => {
                let Some((source_argument, source_result)) = toolkit::decompose_function(
                    self.builder.state,
                    self.builder.context,
                    source_type,
                )?
                else {
                    return Ok(None);
                };
                let Some((target_argument, target_result)) = toolkit::decompose_function(
                    self.builder.state,
                    self.builder.context,
                    target_type,
                )?
                else {
                    return Ok(None);
                };
                let input_name = match function_depth {
                    0 => "argument".into(),
                    _ => format_smolstr!("argument{function_depth}"),
                };
                let input = self.builder.variable_binder(&input_name, target_argument);
                let mut input_value = self.builder.variable(input);
                if let Some(operation) = argument {
                    let argument_context = TraversalContext {
                        source_type: target_argument,
                        target_type: source_argument,
                        function_depth: function_depth + 1,
                    };
                    let Some(transformed) =
                        self.emit_traversal(operation, input_value, argument_context)?
                    else {
                        return Ok(None);
                    };
                    input_value = transformed;
                }
                let Some(output) = self.builder.apply(value, input_value)? else {
                    return Ok(None);
                };
                let output = if let Some(operation) = result {
                    let result_context = TraversalContext {
                        source_type: source_result,
                        target_type: target_result,
                        function_depth: function_depth + 1,
                    };
                    let Some(output) = self.emit_traversal(operation, output, result_context)?
                    else {
                        return Ok(None);
                    };
                    output
                } else {
                    output
                };
                let output = self.builder.subtype(output, target_result)?;
                Ok(Some(self.builder.lambda(target_type, vec![input], output)))
            }
            TraversalOperation::Record { fields } => {
                self.emit_record_traversal(fields, value, traversal)
            }
        }
    }

    fn emit_transformer(
        &mut self,
        operation: &TraversalOperation,
        traversal: TraversalContext,
    ) -> QueryResult<Option<ElaboratedExpression>> {
        let input = self.builder.variable_binder("mapped", traversal.source_type);
        let value = self.builder.variable(input);
        let Some(body) = self.emit_traversal(operation, value, traversal)? else {
            return Ok(None);
        };

        let function =
            self.builder.context.intern_function(traversal.source_type, traversal.target_type);
        Ok(Some(self.builder.lambda(function, vec![input], body)))
    }

    fn emit_record_traversal(
        &mut self,
        fields: &[RecordFieldRecipe],
        value: ElaboratedExpression,
        traversal: TraversalContext,
    ) -> QueryResult<Option<ElaboratedExpression>> {
        let Some(source_row) =
            extract_record_row(self.builder.state, self.builder.context, traversal.source_type)?
        else {
            return Ok(None);
        };
        let Some(target_row) =
            extract_record_row(self.builder.state, self.builder.context, traversal.target_type)?
        else {
            return Ok(None);
        };

        let mut updates = Vec::with_capacity(fields.len());
        for field in fields {
            let Some(source_field) = source_row.fields.iter().find(|row| row.label == field.label)
            else {
                return Ok(None);
            };
            let Some(target_field) = target_row.fields.iter().find(|row| row.label == field.label)
            else {
                return Ok(None);
            };
            let accessed = self.builder.record_access(value, field.label.clone(), source_field.id);
            let field_context = TraversalContext {
                source_type: source_field.id,
                target_type: target_field.id,
                function_depth: traversal.function_depth,
            };
            let Some(updated) = self.emit_traversal(&field.operation, accessed, field_context)?
            else {
                return Ok(None);
            };
            updates.push(tree::RecordExpressionUpdate::Leaf {
                label: field.label.clone(),
                expression: updated.expression,
            });
        }

        Ok(Some(self.builder.record_update(value, updates, traversal.target_type)))
    }
}

fn extract_record_row<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    type_id: TypeId,
) -> QueryResult<Option<RowType>>
where
    Q: ExternalQueries,
{
    let Some((_, row)) = toolkit::decompose_type_application(state, context, type_id)? else {
        return Ok(None);
    };
    let row = normalise::expand(state, context, row)?;
    let Type::Row(row) = context.lookup_type(row) else { return Ok(None) };
    Ok(Some(context.lookup_row_type(row)))
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

#[derive(Clone, Copy)]
struct ComparisonOperations {
    direct: (files::FileId, indexing::TermItemId),
    lifted: Option<(files::FileId, indexing::TermItemId)>,
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
    operations: ComparisonOperations,
) -> QueryResult<Option<ElaboratedExpression>>
where
    Q: ExternalQueries,
{
    let Some(member) = BinaryComparisonMember::decode(
        state,
        context,
        comparison_member.member.implementation_type,
    )?
    else {
        return Ok(None);
    };

    let mut builder = DerivedTreeBuilder::new(state, context, derive_id);
    let parameters = emit_parameters(&mut builder, &member);

    let mut alternatives = vec![];
    for constructor in &comparison_member.constructors {
        let Some(comparison) =
            emit_constructor_comparison(&mut builder, constructor, &member, operations)?
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
    operations: ComparisonOperations,
    equal: (files::FileId, indexing::TermItemId),
    constructor_ordering: Option<ConstructorOrdering>,
) -> QueryResult<Option<ElaboratedExpression>>
where
    Q: ExternalQueries,
{
    let Some(member) = BinaryComparisonMember::decode(
        state,
        context,
        comparison_member.member.implementation_type,
    )?
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
        operations,
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
    operations: ComparisonOperations,
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
            emit_constructor_comparison(builder, constructor, member, operations)?
        else {
            return Ok(None);
        };
        let body = emit_lexicographic(builder, comparison.comparisons, member.result_type, equal)?;
        let patterns = vec![comparison.left_pattern, comparison.right_pattern];
        alternatives.push(builder.alternative(patterns, body));

        if let Some(ordering) = constructor_ordering
            && !later_constructors.is_empty()
        {
            emit_constructor_ordering_alternatives(
                builder,
                constructor,
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
    member: &BinaryComparisonMember,
    ordering: ConstructorOrdering,
    alternatives: &mut Vec<tree::CaseAlternative>,
) -> QueryResult<()>
where
    Q: ExternalQueries,
{
    let left = emit_constructor_wildcard(builder, constructor, member.left_type);
    let right = builder.wildcard_pattern("right", member.right_type);
    let body = emit_ordering_constant(builder, ordering.less, member.result_type)?;
    alternatives.push(builder.alternative(vec![left, right], body));

    let left = builder.wildcard_pattern("left", member.left_type);
    let right = emit_constructor_wildcard(builder, constructor, member.right_type);
    let body = emit_ordering_constant(builder, ordering.greater, member.result_type)?;
    alternatives.push(builder.alternative(vec![left, right], body));

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
    operations: ComparisonOperations,
) -> QueryResult<Option<ConstructorComparison>>
where
    Q: ExternalQueries,
{
    let mut fields = Vec::with_capacity(constructor.fields.len());
    for (index, field) in constructor.fields.iter().enumerate() {
        let Some(field) = emit_field_comparison(builder, operations, index, field)? else {
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
    operations: ComparisonOperations,
    index: usize,
    field: &AppliedField,
) -> QueryResult<Option<GeneratedFieldPair>>
where
    Q: ExternalQueries,
{
    let operation = match field.comparison {
        field::ComparisonStyle::Direct => operations.direct,
        field::ComparisonStyle::Lifted => {
            let Some(operation) = operations.lifted else {
                return Ok(None);
            };
            operation
        }
    };

    let left = builder.variable_binder(&format_smolstr!("left{index}"), field.type_id);
    let right = builder.variable_binder(&format_smolstr!("right{index}"), field.type_id);
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
    let fields =
        constructor.fields.iter().map(|field| builder.wildcard_pattern("field", field.type_id));
    let fields = fields.collect();
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
