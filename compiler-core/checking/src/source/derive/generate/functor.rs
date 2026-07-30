use building_types::QueryResult;
use itertools::izip;
use smol_str::format_smolstr;

use crate::context::CheckContext;
use crate::core::substitute::NameToType;
use crate::core::{KindOrType, RowType, Type, TypeId, normalise, signature, toolkit};
use crate::evidence::Evidence;
use crate::source::derive::builder::DerivedTreeBuilder;
use crate::source::derive::field;
use crate::source::derive::variance::{
    ConstructorRecipe, RecordFieldRecipe, TraversalOperation, TraversalParameter, VarianceRecipe,
};
use crate::source::terms::ElaboratedExpression;
use crate::state::CheckState;
use crate::{ExternalQueries, tree};

use super::{DeriveHeadResult, DeriveStrategy, ResolvedMember, generated_member, resolve_member};

struct InstantiatedDataType {
    type_id: TypeId,
    constructor_arguments: Vec<KindOrType>,
}

#[derive(Clone, Copy)]
pub(super) enum TraversalKind {
    Functor,
    Bifunctor,
}

#[derive(Clone, Copy)]
enum Mappings<T> {
    Functor(T),
    Bifunctor { first: T, second: T },
}

impl Mappings<ElaboratedExpression> {
    fn mapping_for(self, parameter: TraversalParameter) -> Option<ElaboratedExpression> {
        match (self, parameter) {
            (Mappings::Functor(function), TraversalParameter::First) => Some(function),
            (Mappings::Bifunctor { first, .. }, TraversalParameter::First) => Some(first),
            (Mappings::Bifunctor { second, .. }, TraversalParameter::Second) => Some(second),
            (Mappings::Functor(_), TraversalParameter::Second) => None,
        }
    }
}

struct DecodedTraversalMember {
    member: ResolvedMember,
    substitution: NameToType,
    constraints: Vec<TypeId>,
    implementation_type: TypeId,
    function_type: TypeId,
    mappings: Mappings<TypeId>,
    data_file: files::FileId,
    source: InstantiatedDataType,
    target: InstantiatedDataType,
}

impl DecodedTraversalMember {
    fn decode<Q>(
        state: &mut CheckState,
        context: &CheckContext<Q>,
        member: ResolvedMember,
        data_file: files::FileId,
        traversal: TraversalKind,
    ) -> QueryResult<Option<DecodedTraversalMember>>
    where
        Q: ExternalQueries,
    {
        let argument_count = match traversal {
            TraversalKind::Functor => 2,
            TraversalKind::Bifunctor => 3,
        };
        let signature::SkolemisedSignature { substitution, constraints, arguments, result } =
            signature::expect_term_signature(
                state,
                context,
                member.implementation_type,
                argument_count,
            )?;

        let (source_type, mappings) = match (traversal, arguments.as_slice()) {
            (TraversalKind::Functor, [mapping, source]) => (*source, Mappings::Functor(*mapping)),
            (TraversalKind::Bifunctor, [first, second, source]) => {
                (*source, Mappings::Bifunctor { first: *first, second: *second })
            }
            _ => return Ok(None),
        };

        let (_, source_arguments) = toolkit::extract_all_applications(state, context, source_type)?;
        let (_, target_arguments) = toolkit::extract_all_applications(state, context, result)?;
        let function_arguments = arguments.iter().copied();
        let function_type = context.intern_function_iter(function_arguments, result);

        Ok(Some(DecodedTraversalMember {
            implementation_type: member.implementation_type,
            member,
            substitution,
            constraints,
            function_type,
            mappings,
            data_file,
            source: InstantiatedDataType {
                type_id: source_type,
                constructor_arguments: source_arguments,
            },
            target: InstantiatedDataType {
                type_id: result,
                constructor_arguments: target_arguments,
            },
        }))
    }
}

pub(super) fn generate_traversal_member<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    result: &DeriveHeadResult,
    instance_arguments: &[KindOrType],
    recipe: &VarianceRecipe,
    traversal: TraversalKind,
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
        let Some(member) =
            DecodedTraversalMember::decode(state, context, member, data_file, traversal)?
        else {
            return Ok(None);
        };

        let mut evidences = Vec::with_capacity(member.constraints.len());
        for &constraint in &member.constraints {
            evidences.push(Evidence::Given(state.push_given(constraint)));
        }

        let body = state.with_implicit(context, &member.substitution, |state| {
            emit_variance_traversal(state, context, result.derive_id, &member, recipe)
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

fn emit_variance_traversal<Q>(
    state: &mut CheckState,
    context: &CheckContext<Q>,
    derive_id: indexing::DeriveId,
    member: &DecodedTraversalMember,
    recipe: &VarianceRecipe,
) -> QueryResult<Option<ElaboratedExpression>>
where
    Q: ExternalQueries,
{
    let mut builder = DerivedTreeBuilder::new(state, context, derive_id);

    let (mapping_binders, mapping_expressions) = match member.mappings {
        Mappings::Functor(mapping) => {
            let function = builder.variable_binder("function", mapping);
            let expression = builder.variable(function);
            (vec![function], Mappings::Functor(expression))
        }
        Mappings::Bifunctor { first, second } => {
            let first = builder.variable_binder("firstFunction", first);
            let second = builder.variable_binder("secondFunction", second);
            let expressions = Mappings::Bifunctor {
                first: builder.variable(first),
                second: builder.variable(second),
            };
            (vec![first, second], expressions)
        }
    };
    let value = builder.variable_binder("value", member.source.type_id);

    let value_expression = builder.variable(value);

    let mut alternatives = Vec::with_capacity(recipe.constructors.len());

    for constructor in &recipe.constructors {
        let Some(alternative) =
            emit_traversal_alternative(&mut builder, member, constructor, mapping_expressions)?
        else {
            return Ok(None);
        };
        alternatives.push(alternative);
    }

    let body = builder.case(member.target.type_id, vec![value_expression], alternatives);
    let mut binders = mapping_binders;
    binders.push(value);
    Ok(Some(builder.lambda(member.function_type, binders, body)))
}

fn emit_traversal_alternative<Q>(
    builder: &mut DerivedTreeBuilder<'_, '_, '_, Q>,
    member: &DecodedTraversalMember,
    constructor: &ConstructorRecipe,
    mapping_expressions: Mappings<ElaboratedExpression>,
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

    let mut emitter = VarianceTraversalEmitter { builder, mapping_expressions };
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

struct VarianceTraversalEmitter<'builder, 'state, 'context, 'queries, Q: ExternalQueries> {
    builder: &'builder mut DerivedTreeBuilder<'state, 'context, 'queries, Q>,
    mapping_expressions: Mappings<ElaboratedExpression>,
}

impl<Q> VarianceTraversalEmitter<'_, '_, '_, '_, Q>
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
            TraversalOperation::Parameter { parameter } => {
                // A parameter is a leaf in the traversal. Apply the corresponding mapping
                // expression directly to the source expression.
                //
                //   source   :: a
                //   target   :: b
                //   function :: a -> b
                //
                //   function source :: b
                let Some(mapping_expression) = self.mapping_expressions.mapping_for(*parameter)
                else {
                    return Ok(None);
                };
                let Some(mapped) = self.builder.apply(mapping_expression, value)? else {
                    return Ok(None);
                };
                Ok(Some(self.builder.subtype(mapped, target_type)?))
            }
            TraversalOperation::Map { argument } => {
                // Map delegates traversal of a unary type constructor to its existing
                // Functor instance. The generator only needs to produce the transformation
                // that its map implementation applies to each contained value.
                //
                // NonEmpty
                //
                //   data NonEmpty a = NonEmpty a (Array a)
                //
                //   function :: a -> b
                //
                //   source :: Array a
                //   target :: Array b
                //
                // Inventory
                //
                //   newtype Inventory a = Inventory (Array { item :: a })
                //
                //   function :: a -> b
                //
                //   source :: Array { item :: a }
                //   target :: Array { item :: b }
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

                // First generate the transformation between the applied arguments.
                // `emit_transformer` wraps the argument operation in the function that
                // `map` calls for each contained value. A Parameter operation is
                // semantically just `function`; a Record operation produces a more
                // involved transformation using record field updates.
                //
                // NonEmpty
                //
                //   sourceArgument :: a
                //   targetArgument :: b
                //
                //   transformer :: a -> b
                //   transformer = function
                //
                // Inventory
                //
                //   sourceArgument :: { item :: a }
                //   targetArgument :: { item :: b }
                //
                //   transformer :: { item :: a } -> { item :: b }
                //   transformer = \element ->
                //     element { item = function element.item }
                let argument_context = TraversalContext {
                    source_type: source_argument,
                    target_type: target_argument,
                    function_depth,
                };
                let Some(transformer) = self.emit_transformer(argument, argument_context)? else {
                    return Ok(None);
                };

                // Resolve the polymorphic, constrained map declaration and specialize it
                // through application. Applying `transformer` solves the element types;
                // applying `source` solves the fresh constructor variable, specializing
                // its wanted Functor evidence.
                //
                //   map :: forall f a b. Functor f => (a -> b) -> f a -> f b
                //
                // NonEmpty
                //
                //   ?f := Array
                //
                //   map transformer source :: Array b
                //
                // Inventory
                //
                //   ?f := Array
                //
                //   map transformer source :: Array { item :: b }
                //
                // The generated NonEmpty member therefore delegates its tail to Array's
                // map:
                //
                //   map function (NonEmpty head tail) =
                //     NonEmpty (function head) (map (\element -> function element) tail)
                //
                // The generated Inventory member also delegates to Array's map, using the
                // record transformer for each element:
                //
                //   map function (Inventory items) =
                //     Inventory
                //       (map (\element -> element { item = function element.item }) items)
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

                // Check the specialized result against the target established above.
                Ok(Some(self.builder.subtype(mapped, target_type)?))
            }
            TraversalOperation::Bimap { first, second } => {
                // Bimap lifts transformations through the first and second arguments of a
                // binary type constructor. See `emit_bimap` for the staged construction.
                self.emit_bimap(first, second.as_deref(), value, traversal)
            }
            // Function types require both covariant and contravariant transformations.
            // Given:
            //
            //   f :: sourceArgument -> sourceResult
            //
            // and the goal:
            //
            //   targetArgument -> targetResult
            //
            // derive the two transformations:
            //
            //   transformArgument :: targetArgument -> sourceArgument
            //   transformResult   :: sourceResult -> targetResult
            //
            // and combine them as:
            //
            //   \targetArgument ->
            //     transformResult (f (transformArgument targetArgument))
            TraversalOperation::Function { argument, result } => {
                // Decompose both sides of the transformation. For example:
                //
                // Reader
                //
                //   source :: r -> a
                //   target :: r -> b
                //
                // CPS
                //
                //   source :: (a -> r) -> r
                //   target :: (b -> r) -> r
                //
                // CPS-i (CPS with its intermediate value)
                //
                //   source :: (a -> r) -> Tuple r a
                //   target :: (b -> r) -> Tuple r b
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

                // Bind the target argument.
                //
                // Reader
                //
                //   argument :: r
                //
                // CPS and CPS-i
                //
                //   returnB :: b -> r
                let input_name = match function_depth {
                    0 => "argument".into(),
                    _ => format_smolstr!("argument{function_depth}"),
                };
                let input = self.builder.variable_binder(&input_name, target_argument);
                let mut input_value = self.builder.variable(input);

                // Transform the target argument contravariantly. Reader has no argument
                // operation, so its argument remains unchanged. CPS and CPS-i
                // recursively transform `returnB` into the function accepted by `program`.
                //
                // Reader
                //
                //   argument :: r
                //
                // CPS and CPS-i
                //
                //   \source -> returnB (function source)
                //     :: a -> r
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

                // Apply the source function to the transformed argument:
                //
                // Reader
                //
                //   program argument :: a
                //
                // CPS
                //
                //   program (\source -> ...) :: r
                //
                // CPS-i
                //
                //   program (\source -> ...) :: Tuple r a
                let Some(output) = self.builder.apply(value, input_value)? else {
                    return Ok(None);
                };

                // Transform the source result covariantly.
                //
                // Reader
                //
                //   transformResult :: a -> b
                //   transformResult = function
                //
                // CPS
                //
                //   transformResult :: r -> r
                //   transformResult = identity
                //
                // CPS-i
                //
                //   transformResult :: Tuple r a -> Tuple r b
                //   transformResult = map function
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

                // Close the target function.
                //
                // Reader
                //
                //   \argument -> function (program argument)
                //
                // CPS
                //
                //   \returnB ->
                //     program \source ->
                //       returnB (function source)
                //
                // CPS-i
                //
                //   \returnB ->
                //     map function (program \source -> returnB (function source))
                Ok(Some(self.builder.lambda(target_type, vec![input], output)))
            }
            TraversalOperation::Record { fields } => {
                // A record operation recursively transforms only the entries containing a
                // traversed parameter. See `emit_record_traversal` for reconstruction.
                self.emit_record_traversal(fields, value, traversal)
            }
        }
    }

    fn emit_transformer(
        &mut self,
        operation: &TraversalOperation,
        traversal: TraversalContext,
    ) -> QueryResult<Option<ElaboratedExpression>> {
        // Map and bimap require a function that transforms the inside of their source type
        // into the inside of their target type. Bind one source value, emit its traversal,
        // and return the resulting lambda to the map or bimap call site. A Parameter
        // operation eta-expands its mapping expression; nested operations produce a more
        // involved body.
        //
        //   source :: a
        //   target :: b
        //   element :: a
        //
        //   body :: b
        //   body = emit_traversal operation element
        //
        //   transformer :: a -> b
        //   transformer = \element -> body
        let input = self.builder.variable_binder("element", traversal.source_type);
        let value = self.builder.variable(input);
        let Some(body) = self.emit_traversal(operation, value, traversal)? else {
            return Ok(None);
        };

        let function =
            self.builder.context.intern_function(traversal.source_type, traversal.target_type);
        Ok(Some(self.builder.lambda(function, vec![input], body)))
    }

    fn emit_bimap(
        &mut self,
        first: &TraversalOperation,
        second: Option<&TraversalOperation>,
        value: ElaboratedExpression,
        traversal: TraversalContext,
    ) -> QueryResult<Option<ElaboratedExpression>> {
        // Decompose both arguments of the binary type application.
        //
        //   firstFunction :: a -> c
        //   secondFunction :: b -> d
        //
        // Pair
        //
        //   newtype Pair a b = Pair (Tuple a b)
        //
        //   source :: Tuple a b
        //   target :: Tuple c d
        //
        // LeftPair
        //
        //   data LeftPair a b = LeftPair (Tuple a Int) b
        //
        //   source :: Tuple a Int
        //   target :: Tuple c Int
        //
        // InventoryPair
        //
        //   newtype InventoryPair a b =
        //     InventoryPair (Tuple (Array a) { item :: b })
        //
        //   source :: Tuple (Array a) { item :: b }
        //   target :: Tuple (Array c) { item :: d }
        let Some((source_function, source_second)) = toolkit::decompose_type_application(
            self.builder.state,
            self.builder.context,
            traversal.source_type,
        )?
        else {
            return Ok(None);
        };
        let Some((_, source_first)) = toolkit::decompose_type_application(
            self.builder.state,
            self.builder.context,
            source_function,
        )?
        else {
            return Ok(None);
        };
        let Some((target_function, target_second)) = toolkit::decompose_type_application(
            self.builder.state,
            self.builder.context,
            traversal.target_type,
        )?
        else {
            return Ok(None);
        };
        let Some((_, target_first)) = toolkit::decompose_type_application(
            self.builder.state,
            self.builder.context,
            target_function,
        )?
        else {
            return Ok(None);
        };

        // Generate the transformation that bimap calls for values in its first argument.
        //
        //   bimap :: (a -> b) -> (c -> d) -> f a c -> f b d
        //   firstTransformer :: sourceFirst -> targetFirst
        //
        // Pair and LeftPair
        //
        //   firstTransformer :: a -> c
        //   firstTransformer = firstFunction
        //
        // InventoryPair
        //
        //   firstTransformer :: Array a -> Array c
        //   firstTransformer = map firstFunction
        let first_context = TraversalContext {
            source_type: source_first,
            target_type: target_first,
            function_depth: traversal.function_depth,
        };
        let Some(first_transformer) = self.emit_transformer(first, first_context)? else {
            return Ok(None);
        };

        // Generate the transformation that bimap calls for values in its second argument.
        // When the traversed parameter does not occur there, bimap still requires an
        // identity function.
        //
        //   secondTransformer :: sourceSecond -> targetSecond
        //
        // Pair
        //
        //   secondTransformer :: b -> d
        //   secondTransformer = secondFunction
        //
        // LeftPair
        //
        //   secondTransformer :: Int -> Int
        //   secondTransformer = identity
        //
        // InventoryPair
        //
        //   secondTransformer :: { item :: b } -> { item :: d }
        //   secondTransformer = \record ->
        //     record { item = secondFunction record.item }
        let second_context = TraversalContext {
            source_type: source_second,
            target_type: target_second,
            function_depth: traversal.function_depth,
        };
        let second_transformer = match second {
            Some(second) => {
                let Some(transformer) = self.emit_transformer(second, second_context)? else {
                    return Ok(None);
                };
                transformer
            }
            None => self.emit_identity(second_context)?,
        };

        // Resolve the polymorphic, constrained bimap declaration and specialize it through
        // application. Applying the source solves the fresh constructor variable and
        // specializes its wanted Bifunctor evidence.
        //
        //   bimap :: forall f a b c d.
        //     Bifunctor f => (a -> b) -> (c -> d) -> f a c -> f b d
        //
        //   ?f := Tuple
        //
        // Pair
        //
        //   bimap firstFunction secondFunction (Pair pair) =
        //     Pair (bimap firstFunction secondFunction pair)
        //
        // LeftPair
        //
        //   bimap firstFunction secondFunction (LeftPair pair second) =
        //     LeftPair (bimap firstFunction identity pair) (secondFunction second)
        //
        // InventoryPair
        //
        //   bimap firstFunction secondFunction (InventoryPair pair) =
        //     InventoryPair
        //       (bimap
        //         (map firstFunction)
        //         (\record -> record { item = secondFunction record.item })
        //         pair)
        let Some(bimap) = self.builder.context.known_terms.bimap else {
            return Ok(None);
        };
        let bimap = self.builder.term_reference(bimap)?;
        let Some(bimap) = self.builder.apply(bimap, first_transformer)? else {
            return Ok(None);
        };
        let Some(bimap) = self.builder.apply(bimap, second_transformer)? else {
            return Ok(None);
        };
        let Some(mapped) = self.builder.apply(bimap, value)? else { return Ok(None) };
        Ok(Some(self.builder.subtype(mapped, traversal.target_type)?))
    }

    fn emit_identity(&mut self, traversal: TraversalContext) -> QueryResult<ElaboratedExpression> {
        // A Bimap operation omits its second operation when the traversed parameter does
        // not occur in that argument. Bimap still requires a second transformer, so supply
        // identity rather than treating the missing operation as a missing expression.
        //
        // LeftPair
        //
        //   sourceSecond :: Int
        //   targetSecond :: Int
        //
        //   identity :: Int -> Int
        //   identity = \a -> a
        let input = self.builder.variable_binder("unchanged", traversal.source_type);
        let value = self.builder.variable(input);
        let body = self.builder.subtype(value, traversal.target_type)?;
        let function =
            self.builder.context.intern_function(traversal.source_type, traversal.target_type);
        Ok(self.builder.lambda(function, vec![input], body))
    }

    fn emit_record_traversal(
        &mut self,
        fields: &[RecordFieldRecipe],
        value: ElaboratedExpression,
        traversal: TraversalContext,
    ) -> QueryResult<Option<ElaboratedExpression>> {
        // Recover the source and target rows used to type each access and update.
        //
        // Profile
        //
        //   newtype Profile a = Profile { name :: String, value :: a }
        //
        //   function :: a -> b
        //
        //   source :: { name :: String, value :: a }
        //   target :: { name :: String, value :: b }
        //
        // Catalog
        //
        //   newtype Catalog a b =
        //     Catalog { items :: Array a, name :: String, selected :: b }
        //
        //   firstFunction :: a -> c
        //   secondFunction :: b -> d
        //
        //   source :: { items :: Array a, name :: String, selected :: b }
        //   target :: { items :: Array c, name :: String, selected :: d }
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

        // Each entry in `fields` identifies a record entry containing a traversed
        // parameter; entries such as `name` are absent and remain unchanged. Access each
        // selected entry and recursively transform it according to its operation.
        //
        // Profile
        //
        //   valueOperation = Parameter First
        //
        //   value :: a
        //   updatedValue :: b
        //   updatedValue = function source.value
        //
        // Catalog
        //
        //   itemsOperation = Map (Parameter First)
        //
        //   items :: Array a
        //   updatedItems :: Array c
        //   updatedItems = map firstFunction source.items
        //
        //   selectedOperation = Parameter Second
        //
        //   selected :: b
        //   updatedSelected :: d
        //   updatedSelected = secondFunction source.selected
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

        // Reconstruct the target with the transformed entries.
        //
        // Profile
        //
        //   map function (Profile source) =
        //     Profile (source { value = function source.value })
        //
        // Catalog
        //
        //   bimap firstFunction secondFunction (Catalog source) =
        //     Catalog
        //       (source
        //         { items = map firstFunction source.items
        //         , selected = secondFunction source.selected
        //         })
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
