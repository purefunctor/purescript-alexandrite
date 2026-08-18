//! Conversion from owned functional trees into A-normal control-flow graphs.

use std::sync::Arc;

use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use smol_str::{SmolStr, format_smolstr};

use crate::error::{ConversionError, ConversionResult, UnsupportedState};
use crate::tree::{
    Block, BlockId, BlockTarget, CallingConvention, Declaration, DeclarationKind, Failure, Field,
    FieldIdentity, Function, FunctionId, Global, GlobalIdentity, InstanceIdentity, Instruction,
    InstructionValue, Literal, Module, PatternTest, Projection, RecordField, RecordUpdate,
    RecursiveClosure, RecursiveGroupId, ReflectableEvidence, ReflectableOrdering, Storage,
    SuperclassIdentity, SynthesizedEvidence, Terminator, Value, ValueId,
};

pub fn convert_module(functional: &nbe::tree::Module) -> ConversionResult<Module> {
    let mut state = State::default();
    let context = Context { functional };
    for declaration in functional.declarations.iter() {
        convert_declaration(&mut state, &context, declaration)?;
    }
    Ok(Module {
        file_id: functional.file_id,
        declarations: state.output.declarations.into(),
        storage: state.output.storage,
    })
}

struct Context<'m> {
    functional: &'m nbe::tree::Module,
}

struct Traversal {
    function_name: SmolStr,
    current_block: Option<BlockId>,
    blocks: Vec<BlockId>,
    scope: FxHashMap<nbe::tree::LocalId, ValueId>,
    value_names: FxHashMap<SmolStr, u32>,
    block_names: FxHashMap<SmolStr, u32>,
    unterminated: FxHashSet<BlockId>,
}

impl Traversal {
    fn new(function_name: SmolStr) -> Traversal {
        Traversal {
            function_name,
            current_block: None,
            blocks: vec![],
            scope: FxHashMap::default(),
            value_names: FxHashMap::default(),
            block_names: FxHashMap::default(),
            unterminated: FxHashSet::default(),
        }
    }
}

#[derive(Default)]
struct PersistedOutput {
    declarations: Vec<Declaration>,
    storage: Storage,
    function_names: FxHashMap<SmolStr, u32>,
}

#[derive(Default)]
struct State {
    traversal: Option<Traversal>,
    output: PersistedOutput,
}

#[derive(Clone)]
enum FunctionParameter {
    Pattern { pattern: nbe::tree::PatternId },
    Local { parameter: nbe::tree::Parameter },
}

struct BuiltFunction {
    function: FunctionId,
    captures: Arc<[ValueId]>,
}

fn convert_declaration(
    state: &mut State,
    context: &Context<'_>,
    declaration: &nbe::tree::Declaration,
) -> ConversionResult<()> {
    let global = convert_global(&declaration.global);
    let recursive_group =
        declaration.recursive_group.map(|group| RecursiveGroupId { index: group.0 });
    let kind = match declaration.kind {
        nbe::tree::DeclarationKind::Value(expression) => {
            let expression = &context.functional.storage[expression];
            if let nbe::tree::ExpressionKind::Abstraction { parameters, body } = &expression.kind {
                let parameters =
                    parameters.iter().map(|&pattern| FunctionParameter::Pattern { pattern });
                let parameters = parameters.collect_vec();
                let built = build_function(
                    state,
                    context,
                    global.name.clone(),
                    CallingConvention::Source,
                    parameters,
                    *body,
                )?;
                DeclarationKind::Function { function: built.function }
            } else {
                let name = format_smolstr!("{}$initialize", global.name);
                let built = build_function(
                    state,
                    context,
                    name,
                    CallingConvention::Initializer,
                    vec![],
                    expression_id(declaration),
                )?;
                DeclarationKind::Value { initializer: built.function }
            }
        }
        nbe::tree::DeclarationKind::Foreign => DeclarationKind::Foreign,
    };
    state.output.declarations.push(Declaration { global, recursive_group, kind });
    Ok(())
}

fn expression_id(declaration: &nbe::tree::Declaration) -> nbe::tree::ExpressionId {
    let nbe::tree::DeclarationKind::Value(expression) = declaration.kind else {
        unreachable!("invariant violated: requested expression from foreign declaration")
    };
    expression
}

fn build_function(
    state: &mut State,
    context: &Context<'_>,
    preferred_name: SmolStr,
    calling_convention: CallingConvention,
    parameters: Vec<FunctionParameter>,
    body: nbe::tree::ExpressionId,
) -> ConversionResult<BuiltFunction> {
    let function_name = state.fresh_function_name(preferred_name);
    let free_parameters = free_parameters(context, &parameters, body);
    let capture_sources =
        free_parameters.iter().map(|parameter| state.lookup_local(context, parameter));
    let capture_sources = capture_sources.collect::<ConversionResult<Vec<_>>>()?;

    let parent = state.traversal.take();
    state.traversal = Some(Traversal::new(function_name.clone()));
    let result = build_function_body(state, context, &free_parameters, &parameters, body);
    let traversal =
        state.traversal.take().expect("invariant violated: function traversal is missing");
    state.traversal = parent;

    let (captures, formal_parameters, entry) = result?;
    if !traversal.unterminated.is_empty() {
        return Err(context.unsupported(UnsupportedState::UnterminatedBlock {
            function_name: traversal.function_name.to_string(),
        }));
    }
    let function = Function {
        name: function_name,
        calling_convention,
        captures: captures.into(),
        parameters: formal_parameters.into(),
        entry,
        blocks: traversal.blocks.into(),
    };
    let function = state.output.storage.allocate_function(function);
    Ok(BuiltFunction { function, captures: capture_sources.into() })
}

fn build_function_body(
    state: &mut State,
    context: &Context<'_>,
    free_parameters: &[nbe::tree::Parameter],
    parameters: &[FunctionParameter],
    body: nbe::tree::ExpressionId,
) -> ConversionResult<(Vec<ValueId>, Vec<ValueId>, BlockId)> {
    let entry = state.create_block("entry", vec![]);
    state.switch_to(entry);

    let mut captures = vec![];
    for parameter in free_parameters {
        let value = state.fresh_value(parameter.name.clone());
        state.bind_local(parameter, value);
        captures.push(value);
    }

    let mut formal_parameters = vec![];
    for (position, parameter) in parameters.iter().enumerate() {
        let name = function_parameter_name(context, parameter, position);
        formal_parameters.push(state.fresh_value(name));
    }

    let has_refutable_parameter = parameters.iter().any(|parameter| match parameter {
        FunctionParameter::Pattern { pattern } => pattern_is_refutable(context, *pattern),
        FunctionParameter::Local { .. } => false,
    });
    let failure = has_refutable_parameter.then(|| state.create_block("parameter$failure", vec![]));
    for (parameter, value) in parameters.iter().zip(formal_parameters.iter().copied()) {
        match parameter {
            FunctionParameter::Pattern { pattern } => {
                compile_pattern(state, context, *pattern, value, failure)?;
            }
            FunctionParameter::Local { parameter } => state.bind_local(parameter, value),
        }
    }

    let result = lower_expression(state, context, body)?;
    state.terminate(Terminator::Return { value: result });
    if let Some(failure) = failure {
        state.switch_to(failure);
        state.terminate(Terminator::Fail { failure: Failure::PatternMatch });
    }
    Ok((captures, formal_parameters, entry))
}

fn lower_expression(
    state: &mut State,
    context: &Context<'_>,
    expression_id: nbe::tree::ExpressionId,
) -> ConversionResult<ValueId> {
    let expression = &context.functional.storage[expression_id];
    match &expression.kind {
        nbe::tree::ExpressionKind::Literal { literal } => {
            let literal = convert_literal(literal);
            Ok(state.emit("literal", InstructionValue::Literal { literal }))
        }
        nbe::tree::ExpressionKind::Array { elements } => {
            let elements = lower_expressions(state, context, elements)?;
            Ok(state.emit("array", InstructionValue::Array { elements: elements.into() }))
        }
        nbe::tree::ExpressionKind::Record { fields } => {
            let mut converted = vec![];
            for field in fields.iter() {
                let value = lower_expression(state, context, field.expression)?;
                converted.push(RecordField { field: convert_field(&field.field), value });
            }
            Ok(state.emit("record", InstructionValue::Record { fields: converted.into() }))
        }
        nbe::tree::ExpressionKind::RecordUpdate { record, updates } => {
            let record = lower_expression(state, context, *record)?;
            let updates = lower_record_updates(state, context, updates)?;
            let value = InstructionValue::RecordUpdate { record, updates: updates.into() };
            Ok(state.emit("updated", value))
        }
        nbe::tree::ExpressionKind::Project { record, field } => {
            let record = lower_expression(state, context, *record)?;
            let field = convert_field(field);
            let name = field.name.clone();
            Ok(state.emit(name, InstructionValue::Project { record, field }))
        }
        nbe::tree::ExpressionKind::Constructor { global } => {
            let global = convert_global(global);
            let name = global.name.clone();
            Ok(state.emit(name, InstructionValue::Constructor { global }))
        }
        nbe::tree::ExpressionKind::Global { global } => {
            let global = convert_global(global);
            let name = global.name.clone();
            Ok(state.emit(name, InstructionValue::Global { global }))
        }
        nbe::tree::ExpressionKind::Local { parameter } => state.lookup_local(context, parameter),
        nbe::tree::ExpressionKind::Abstraction { parameters, body } => {
            let parameters =
                parameters.iter().map(|&pattern| FunctionParameter::Pattern { pattern });
            let parameters = parameters.collect_vec();
            let preferred_name = format_smolstr!("{}$closure", state.function_name());
            let built = build_function(
                state,
                context,
                preferred_name,
                CallingConvention::Source,
                parameters,
                *body,
            )?;
            let value =
                InstructionValue::Closure { function: built.function, captures: built.captures };
            Ok(state.emit("closure", value))
        }
        nbe::tree::ExpressionKind::Application { function, arguments } => {
            let function = lower_expression(state, context, *function)?;
            let arguments = lower_expressions(state, context, arguments)?;
            let value = InstructionValue::Call {
                calling_convention: CallingConvention::Source,
                function,
                arguments: arguments.into(),
            };
            Ok(state.emit("call", value))
        }
        nbe::tree::ExpressionKind::IfThenElse { condition, then, else_ } => {
            lower_if_then_else(state, context, *condition, *then, *else_)
        }
        nbe::tree::ExpressionKind::Case { scrutinees, alternatives } => {
            lower_case(state, context, scrutinees, alternatives)
        }
        nbe::tree::ExpressionKind::Guarded { alternatives } => {
            lower_guarded(state, context, alternatives)
        }
        nbe::tree::ExpressionKind::Let { recursive, bindings, body } => {
            lower_let(state, context, *recursive, bindings, *body)
        }
        nbe::tree::ExpressionKind::LetPattern { pattern, value, body } => {
            lower_pattern_let(state, context, *pattern, *value, *body)
        }
        nbe::tree::ExpressionKind::Effect { effect } => lower_effect(state, context, effect),
        nbe::tree::ExpressionKind::SynthesizedEvidence { evidence } => {
            let evidence = convert_synthesized_evidence(evidence);
            Ok(state.emit("evidence", InstructionValue::SynthesizedEvidence { evidence }))
        }
        nbe::tree::ExpressionKind::TrivialEvidence => {
            Ok(state.emit("evidence", InstructionValue::TrivialEvidence))
        }
    }
}

fn lower_expressions(
    state: &mut State,
    context: &Context<'_>,
    expressions: &[nbe::tree::ExpressionId],
) -> ConversionResult<Vec<ValueId>> {
    let values = expressions.iter().map(|&expression| lower_expression(state, context, expression));
    values.collect::<ConversionResult<Vec<_>>>()
}

fn lower_record_updates(
    state: &mut State,
    context: &Context<'_>,
    updates: &[nbe::tree::RecordUpdate],
) -> ConversionResult<Vec<RecordUpdate>> {
    let mut converted = vec![];
    for update in updates {
        let update = match update {
            nbe::tree::RecordUpdate::Leaf { field, expression } => {
                let value = lower_expression(state, context, *expression)?;
                RecordUpdate::Leaf { field: convert_field(field), value }
            }
            nbe::tree::RecordUpdate::Branch { field, updates } => {
                let updates = lower_record_updates(state, context, updates)?;
                RecordUpdate::Branch { field: convert_field(field), updates: updates.into() }
            }
        };
        converted.push(update);
    }
    Ok(converted)
}

fn lower_if_then_else(
    state: &mut State,
    context: &Context<'_>,
    condition: nbe::tree::ExpressionId,
    then: nbe::tree::ExpressionId,
    else_: nbe::tree::ExpressionId,
) -> ConversionResult<ValueId> {
    let condition = lower_expression(state, context, condition)?;
    let outer_scope = state.scope().clone();
    let then_block = state.create_block("then", vec![]);
    let else_block = state.create_block("else", vec![]);
    let result = state.fresh_value("result".into());
    let join = state.create_block("if$join", vec![result]);
    let then_target = state.target(then_block, vec![]);
    let else_target = state.target(else_block, vec![]);
    state.terminate(Terminator::Branch { condition, then_target, else_target });

    state.switch_to(then_block);
    state.set_scope(outer_scope.clone());
    let then = lower_expression(state, context, then)?;
    let target = state.target(join, vec![then]);
    state.terminate(Terminator::Jump { target });

    state.switch_to(else_block);
    state.set_scope(outer_scope.clone());
    let else_ = lower_expression(state, context, else_)?;
    let target = state.target(join, vec![else_]);
    state.terminate(Terminator::Jump { target });

    state.switch_to(join);
    state.set_scope(outer_scope);
    Ok(result)
}

fn lower_case(
    state: &mut State,
    context: &Context<'_>,
    scrutinees: &[nbe::tree::ExpressionId],
    alternatives: &[nbe::tree::CaseAlternative],
) -> ConversionResult<ValueId> {
    if alternatives.is_empty() {
        return Err(context.unsupported(UnsupportedState::MissingCaseAlternative));
    }
    let scrutinees = lower_expressions(state, context, scrutinees)?;
    let outer_scope = state.scope().clone();
    let alternative_blocks = (0..alternatives.len())
        .map(|position| state.create_block(format_smolstr!("case${position}"), vec![]));
    let alternative_blocks = alternative_blocks.collect_vec();
    let first_alternative = alternative_blocks
        .first()
        .copied()
        .expect("invariant violated: case expression has no alternative block");
    let failure = state.create_block("case$failure", vec![]);
    let result = state.fresh_value("result".into());
    let join = state.create_block("case$join", vec![result]);
    let target = state.target(first_alternative, vec![]);
    state.terminate(Terminator::Jump { target });

    let mut blocks = alternative_blocks.iter().copied().peekable();
    for alternative in alternatives {
        if alternative.patterns.len() != scrutinees.len() {
            return Err(context.unsupported(UnsupportedState::CaseArity {
                patterns: alternative.patterns.len(),
                scrutinees: scrutinees.len(),
            }));
        }
        let block = blocks.next().expect("invariant violated: case alternative has no basic block");
        let next = blocks.peek().copied().unwrap_or(failure);
        state.switch_to(block);
        state.set_scope(outer_scope.clone());
        for (&pattern, value) in alternative.patterns.iter().zip(scrutinees.iter().copied()) {
            compile_pattern(state, context, pattern, value, Some(next))?;
        }
        let value = lower_expression(state, context, alternative.expression)?;
        let target = state.target(join, vec![value]);
        state.terminate(Terminator::Jump { target });
    }

    state.switch_to(failure);
    state.terminate(Terminator::Fail { failure: Failure::PatternMatch });
    state.switch_to(join);
    state.set_scope(outer_scope);
    Ok(result)
}

fn lower_guarded(
    state: &mut State,
    context: &Context<'_>,
    alternatives: &[nbe::tree::GuardedAlternative],
) -> ConversionResult<ValueId> {
    if alternatives.is_empty() {
        return Err(context.unsupported(UnsupportedState::MissingGuardedAlternative));
    }
    let outer_scope = state.scope().clone();
    let alternative_blocks = (0..alternatives.len())
        .map(|position| state.create_block(format_smolstr!("guard${position}"), vec![]));
    let alternative_blocks = alternative_blocks.collect_vec();
    let first_alternative = alternative_blocks
        .first()
        .copied()
        .expect("invariant violated: guarded expression has no alternative block");
    let failure = state.create_block("guard$failure", vec![]);
    let result = state.fresh_value("result".into());
    let join = state.create_block("guard$join", vec![result]);
    let target = state.target(first_alternative, vec![]);
    state.terminate(Terminator::Jump { target });

    let mut blocks = alternative_blocks.iter().copied().peekable();
    for alternative in alternatives {
        let block =
            blocks.next().expect("invariant violated: guarded alternative has no basic block");
        let next = blocks.peek().copied().unwrap_or(failure);
        state.switch_to(block);
        state.set_scope(outer_scope.clone());
        for guard in alternative.guards.iter() {
            match guard {
                nbe::tree::Guard::Boolean(expression) => {
                    let condition = lower_expression(state, context, *expression)?;
                    let success = state.create_block("guard$success", vec![]);
                    let then_target = state.target(success, vec![]);
                    let else_target = state.target(next, vec![]);
                    state.terminate(Terminator::Branch { condition, then_target, else_target });
                    state.switch_to(success);
                }
                nbe::tree::Guard::Pattern { expression, pattern } => {
                    let value = lower_expression(state, context, *expression)?;
                    compile_pattern(state, context, *pattern, value, Some(next))?;
                }
            }
        }
        let value = lower_expression(state, context, alternative.expression)?;
        let target = state.target(join, vec![value]);
        state.terminate(Terminator::Jump { target });
    }

    state.switch_to(failure);
    state.terminate(Terminator::Fail { failure: Failure::PatternMatch });
    state.switch_to(join);
    state.set_scope(outer_scope);
    Ok(result)
}

fn lower_let(
    state: &mut State,
    context: &Context<'_>,
    recursive: bool,
    bindings: &[nbe::tree::Binding],
    body: nbe::tree::ExpressionId,
) -> ConversionResult<ValueId> {
    let outer_scope = state.scope().clone();
    if recursive {
        lower_recursive_bindings(state, context, bindings)?;
    } else {
        for binding in bindings {
            let value = lower_expression(state, context, binding.expression)?;
            state.bind_local(&binding.parameter, value);
        }
    }
    let result = lower_expression(state, context, body);
    state.set_scope(outer_scope);
    result
}

fn lower_recursive_bindings(
    state: &mut State,
    context: &Context<'_>,
    bindings: &[nbe::tree::Binding],
) -> ConversionResult<()> {
    let mut results = vec![];
    for binding in bindings {
        let result = state.fresh_value(binding.parameter.name.clone());
        state.bind_local(&binding.parameter, result);
        results.push(result);
    }

    let mut closures = vec![];
    for (binding, result) in bindings.iter().zip(results) {
        let expression = &context.functional.storage[binding.expression];
        let nbe::tree::ExpressionKind::Abstraction { parameters, body } = &expression.kind else {
            return Err(context.unsupported(UnsupportedState::RecursiveValue {
                local_name: binding.parameter.name.to_string(),
            }));
        };
        let parameters = parameters.iter().map(|&pattern| FunctionParameter::Pattern { pattern });
        let parameters = parameters.collect_vec();
        let preferred_name = format_smolstr!("{}$function", binding.parameter.name);
        let built = build_function(
            state,
            context,
            preferred_name,
            CallingConvention::Source,
            parameters,
            *body,
        )?;
        closures.push(RecursiveClosure {
            result,
            function: built.function,
            captures: built.captures,
        });
    }
    state.append(Instruction::RecursiveClosures { bindings: closures.into() });
    Ok(())
}

fn lower_pattern_let(
    state: &mut State,
    context: &Context<'_>,
    pattern: nbe::tree::PatternId,
    value: nbe::tree::ExpressionId,
    body: nbe::tree::ExpressionId,
) -> ConversionResult<ValueId> {
    let value = lower_expression(state, context, value)?;
    let outer_scope = state.scope().clone();
    let failure =
        pattern_is_refutable(context, pattern).then(|| state.create_block("let$failure", vec![]));
    compile_pattern(state, context, pattern, value, failure)?;
    let result = lower_expression(state, context, body)?;
    let continuation = state.current_block();
    if let Some(failure) = failure {
        state.switch_to(failure);
        state.terminate(Terminator::Fail { failure: Failure::PatternMatch });
        state.switch_to(continuation);
    }
    state.set_scope(outer_scope);
    Ok(result)
}

fn lower_effect(
    state: &mut State,
    context: &Context<'_>,
    effect: &nbe::tree::EffectExpression,
) -> ConversionResult<ValueId> {
    match effect {
        nbe::tree::EffectExpression::Pure(expression) => {
            let value = lower_expression(state, context, *expression)?;
            Ok(state.emit("effect", InstructionValue::EffectPure { value }))
        }
        nbe::tree::EffectExpression::Bind { action, parameter, body } => {
            let action = lower_expression(state, context, *action)?;
            let parameters = vec![FunctionParameter::Local { parameter: parameter.clone() }];
            let preferred_name = format_smolstr!("{}$effect", state.function_name());
            let built = build_function(
                state,
                context,
                preferred_name,
                CallingConvention::Effect,
                parameters,
                *body,
            )?;
            let continuation =
                InstructionValue::Closure { function: built.function, captures: built.captures };
            let continuation = state.emit("continuation", continuation);
            let value = InstructionValue::EffectBind { action, continuation };
            Ok(state.emit("effect", value))
        }
    }
}

fn compile_pattern(
    state: &mut State,
    context: &Context<'_>,
    pattern_id: nbe::tree::PatternId,
    value: ValueId,
    failure: Option<BlockId>,
) -> ConversionResult<()> {
    let pattern = &context.functional.storage[pattern_id];
    match &pattern.kind {
        nbe::tree::PatternKind::Variable(parameter) => state.bind_local(parameter, value),
        nbe::tree::PatternKind::Named { parameter, pattern } => {
            state.bind_local(parameter, value);
            compile_pattern(state, context, *pattern, value, failure)?;
        }
        nbe::tree::PatternKind::Literal(literal) => {
            let test = PatternTest::Literal { literal: convert_literal(literal) };
            compile_refutable_test(state, value, test, failure)?;
        }
        nbe::tree::PatternKind::Array(elements) => {
            let test = PatternTest::ArrayLength { length: elements.len() };
            compile_refutable_test(state, value, test, failure)?;
            for (index, &element) in elements.iter().enumerate() {
                let name = pattern_value_name(context, element, "element");
                let projection = Projection::ArrayElement { index };
                let element_value =
                    state.emit(name, InstructionValue::Extract { value, projection });
                compile_pattern(state, context, element, element_value, failure)?;
            }
        }
        nbe::tree::PatternKind::Record(fields) => {
            for field in fields.iter() {
                let pattern = field.pattern;
                let name = pattern_value_name(context, pattern, &field.field.name);
                let field = convert_field(&field.field);
                let field_value =
                    state.emit(name, InstructionValue::Project { record: value, field });
                compile_pattern(state, context, pattern, field_value, failure)?;
            }
        }
        nbe::tree::PatternKind::Constructor { global, arguments } => {
            let global = convert_global(global);
            let test = PatternTest::Constructor { global: global.clone() };
            compile_refutable_test(state, value, test, failure)?;
            for (index, &argument) in arguments.iter().enumerate() {
                let name = pattern_value_name(context, argument, "argument");
                let projection =
                    Projection::ConstructorArgument { constructor: global.clone(), index };
                let argument_value =
                    state.emit(name, InstructionValue::Extract { value, projection });
                compile_pattern(state, context, argument, argument_value, failure)?;
            }
        }
        nbe::tree::PatternKind::Wildcard => {}
    }
    Ok(())
}

fn compile_refutable_test(
    state: &mut State,
    value: ValueId,
    test: PatternTest,
    failure: Option<BlockId>,
) -> ConversionResult<()> {
    let Some(failure) = failure else {
        unreachable!("invariant violated: refutable pattern has no failure block")
    };
    let condition = state.emit("matches", InstructionValue::Test { value, test });
    let success = state.create_block("match$success", vec![]);
    let then_target = state.target(success, vec![]);
    let else_target = state.target(failure, vec![]);
    state.terminate(Terminator::Branch { condition, then_target, else_target });
    state.switch_to(success);
    Ok(())
}

fn pattern_is_refutable(context: &Context<'_>, pattern_id: nbe::tree::PatternId) -> bool {
    let pattern = &context.functional.storage[pattern_id];
    match &pattern.kind {
        nbe::tree::PatternKind::Variable(_) | nbe::tree::PatternKind::Wildcard => false,
        nbe::tree::PatternKind::Named { pattern, .. } => pattern_is_refutable(context, *pattern),
        nbe::tree::PatternKind::Record(fields) => {
            fields.iter().any(|field| pattern_is_refutable(context, field.pattern))
        }
        nbe::tree::PatternKind::Literal(_)
        | nbe::tree::PatternKind::Array(_)
        | nbe::tree::PatternKind::Constructor { .. } => true,
    }
}

fn function_parameter_name(
    context: &Context<'_>,
    parameter: &FunctionParameter,
    position: usize,
) -> SmolStr {
    match parameter {
        FunctionParameter::Pattern { pattern } => {
            pattern_value_name(context, *pattern, &format!("argument{position}"))
        }
        FunctionParameter::Local { parameter } => parameter.name.clone(),
    }
}

fn pattern_value_name(
    context: &Context<'_>,
    pattern_id: nbe::tree::PatternId,
    fallback: &str,
) -> SmolStr {
    let pattern = &context.functional.storage[pattern_id];
    match &pattern.kind {
        nbe::tree::PatternKind::Variable(parameter)
        | nbe::tree::PatternKind::Named { parameter, .. } => parameter.name.clone(),
        _ => SmolStr::new(fallback),
    }
}

fn free_parameters(
    context: &Context<'_>,
    parameters: &[FunctionParameter],
    body: nbe::tree::ExpressionId,
) -> Vec<nbe::tree::Parameter> {
    let mut bound = FxHashSet::default();
    for parameter in parameters {
        match parameter {
            FunctionParameter::Pattern { pattern } => {
                collect_pattern_bindings(context, *pattern, &mut bound);
            }
            FunctionParameter::Local { parameter } => {
                bound.insert(parameter.id);
            }
        }
    }
    let mut free = FreeParameters::default();
    collect_free_expression(context, body, &bound, &mut free);
    free.parameters
}

#[derive(Default)]
struct FreeParameters {
    seen: FxHashSet<nbe::tree::LocalId>,
    parameters: Vec<nbe::tree::Parameter>,
}

impl FreeParameters {
    fn insert(&mut self, parameter: &nbe::tree::Parameter) {
        if self.seen.insert(parameter.id) {
            self.parameters.push(parameter.clone());
        }
    }
}

fn collect_free_expression(
    context: &Context<'_>,
    expression_id: nbe::tree::ExpressionId,
    bound: &FxHashSet<nbe::tree::LocalId>,
    free: &mut FreeParameters,
) {
    let expression = &context.functional.storage[expression_id];
    match &expression.kind {
        nbe::tree::ExpressionKind::Array { elements } => {
            for &element in elements.iter() {
                collect_free_expression(context, element, bound, free);
            }
        }
        nbe::tree::ExpressionKind::Record { fields } => {
            for field in fields.iter() {
                collect_free_expression(context, field.expression, bound, free);
            }
        }
        nbe::tree::ExpressionKind::RecordUpdate { record, updates } => {
            collect_free_expression(context, *record, bound, free);
            collect_free_record_updates(context, updates, bound, free);
        }
        nbe::tree::ExpressionKind::Project { record, .. } => {
            collect_free_expression(context, *record, bound, free);
        }
        nbe::tree::ExpressionKind::Local { parameter } => {
            if !bound.contains(&parameter.id) {
                free.insert(parameter);
            }
        }
        nbe::tree::ExpressionKind::Abstraction { parameters, body } => {
            let mut nested_bound = bound.clone();
            for &pattern in parameters.iter() {
                collect_pattern_bindings(context, pattern, &mut nested_bound);
            }
            collect_free_expression(context, *body, &nested_bound, free);
        }
        nbe::tree::ExpressionKind::Application { function, arguments } => {
            collect_free_expression(context, *function, bound, free);
            for &argument in arguments.iter() {
                collect_free_expression(context, argument, bound, free);
            }
        }
        nbe::tree::ExpressionKind::IfThenElse { condition, then, else_ } => {
            collect_free_expression(context, *condition, bound, free);
            collect_free_expression(context, *then, bound, free);
            collect_free_expression(context, *else_, bound, free);
        }
        nbe::tree::ExpressionKind::Case { scrutinees, alternatives } => {
            for &scrutinee in scrutinees.iter() {
                collect_free_expression(context, scrutinee, bound, free);
            }
            for alternative in alternatives.iter() {
                let mut alternative_bound = bound.clone();
                for &pattern in alternative.patterns.iter() {
                    collect_pattern_bindings(context, pattern, &mut alternative_bound);
                }
                collect_free_expression(context, alternative.expression, &alternative_bound, free);
            }
        }
        nbe::tree::ExpressionKind::Guarded { alternatives } => {
            for alternative in alternatives.iter() {
                let mut alternative_bound = bound.clone();
                for guard in alternative.guards.iter() {
                    match guard {
                        nbe::tree::Guard::Boolean(expression) => {
                            collect_free_expression(context, *expression, &alternative_bound, free);
                        }
                        nbe::tree::Guard::Pattern { expression, pattern } => {
                            collect_free_expression(context, *expression, &alternative_bound, free);
                            collect_pattern_bindings(context, *pattern, &mut alternative_bound);
                        }
                    }
                }
                collect_free_expression(context, alternative.expression, &alternative_bound, free);
            }
        }
        nbe::tree::ExpressionKind::Let { recursive, bindings, body } => {
            collect_free_let(context, *recursive, bindings, *body, bound, free);
        }
        nbe::tree::ExpressionKind::LetPattern { pattern, value, body } => {
            collect_free_expression(context, *value, bound, free);
            let mut body_bound = bound.clone();
            collect_pattern_bindings(context, *pattern, &mut body_bound);
            collect_free_expression(context, *body, &body_bound, free);
        }
        nbe::tree::ExpressionKind::Effect { effect } => match effect {
            nbe::tree::EffectExpression::Pure(expression) => {
                collect_free_expression(context, *expression, bound, free);
            }
            nbe::tree::EffectExpression::Bind { action, parameter, body } => {
                collect_free_expression(context, *action, bound, free);
                let mut body_bound = bound.clone();
                body_bound.insert(parameter.id);
                collect_free_expression(context, *body, &body_bound, free);
            }
        },
        nbe::tree::ExpressionKind::Literal { .. }
        | nbe::tree::ExpressionKind::Constructor { .. }
        | nbe::tree::ExpressionKind::Global { .. }
        | nbe::tree::ExpressionKind::SynthesizedEvidence { .. }
        | nbe::tree::ExpressionKind::TrivialEvidence => {}
    }
}

fn collect_free_let(
    context: &Context<'_>,
    recursive: bool,
    bindings: &[nbe::tree::Binding],
    body: nbe::tree::ExpressionId,
    bound: &FxHashSet<nbe::tree::LocalId>,
    free: &mut FreeParameters,
) {
    let mut body_bound = bound.clone();
    if recursive {
        for binding in bindings {
            body_bound.insert(binding.parameter.id);
        }
        for binding in bindings {
            collect_free_expression(context, binding.expression, &body_bound, free);
        }
    } else {
        for binding in bindings {
            collect_free_expression(context, binding.expression, &body_bound, free);
            body_bound.insert(binding.parameter.id);
        }
    }
    collect_free_expression(context, body, &body_bound, free);
}

fn collect_free_record_updates(
    context: &Context<'_>,
    updates: &[nbe::tree::RecordUpdate],
    bound: &FxHashSet<nbe::tree::LocalId>,
    free: &mut FreeParameters,
) {
    for update in updates {
        match update {
            nbe::tree::RecordUpdate::Leaf { expression, .. } => {
                collect_free_expression(context, *expression, bound, free);
            }
            nbe::tree::RecordUpdate::Branch { updates, .. } => {
                collect_free_record_updates(context, updates, bound, free);
            }
        }
    }
}

fn collect_pattern_bindings(
    context: &Context<'_>,
    pattern_id: nbe::tree::PatternId,
    bound: &mut FxHashSet<nbe::tree::LocalId>,
) {
    let pattern = &context.functional.storage[pattern_id];
    match &pattern.kind {
        nbe::tree::PatternKind::Variable(parameter) => {
            bound.insert(parameter.id);
        }
        nbe::tree::PatternKind::Named { parameter, pattern } => {
            bound.insert(parameter.id);
            collect_pattern_bindings(context, *pattern, bound);
        }
        nbe::tree::PatternKind::Array(elements) => {
            for &element in elements.iter() {
                collect_pattern_bindings(context, element, bound);
            }
        }
        nbe::tree::PatternKind::Record(fields) => {
            for field in fields.iter() {
                collect_pattern_bindings(context, field.pattern, bound);
            }
        }
        nbe::tree::PatternKind::Constructor { arguments, .. } => {
            for &argument in arguments.iter() {
                collect_pattern_bindings(context, argument, bound);
            }
        }
        nbe::tree::PatternKind::Wildcard | nbe::tree::PatternKind::Literal(_) => {}
    }
}

fn convert_global(global: &nbe::tree::Global) -> Global {
    let identity = match global.id {
        nbe::tree::GlobalId::Term(file_id, term_id) => GlobalIdentity::Term { file_id, term_id },
        nbe::tree::GlobalId::Instance(identity) => {
            GlobalIdentity::Instance { identity: convert_instance_identity(identity) }
        }
    };
    Global { identity, name: global.name.clone() }
}

fn convert_instance_identity(identity: nbe::tree::InstanceIdentity) -> InstanceIdentity {
    match identity {
        nbe::tree::InstanceIdentity::Declared(file_id, instance_id) => {
            InstanceIdentity::Declared { file_id, instance_id }
        }
        nbe::tree::InstanceIdentity::Derived(file_id, derive_id) => {
            InstanceIdentity::Derived { file_id, derive_id }
        }
    }
}

fn convert_field(field: &nbe::tree::Field) -> Field {
    let identity = match &field.identity {
        nbe::tree::FieldIdentity::Label(label) => FieldIdentity::Label { label: label.clone() },
        nbe::tree::FieldIdentity::Member(file_id, term_id) => {
            FieldIdentity::Member { file_id: *file_id, term_id: *term_id }
        }
        nbe::tree::FieldIdentity::Superclass(identity) => {
            let identity = SuperclassIdentity {
                file_id: identity.file_id,
                class: identity.class,
                source: identity.source,
            };
            FieldIdentity::Superclass { identity }
        }
    };
    Field { identity, name: field.name.clone() }
}

fn convert_literal(literal: &nbe::tree::Literal) -> Literal {
    match literal {
        nbe::tree::Literal::String(value) => Literal::String { value: value.clone() },
        nbe::tree::Literal::Char(value) => Literal::Char { value: *value },
        nbe::tree::Literal::Boolean(value) => Literal::Boolean { value: *value },
        nbe::tree::Literal::Integer(value) => Literal::Integer { value: *value },
        nbe::tree::Literal::Number(value) => Literal::Number { value: value.clone() },
    }
}

fn convert_synthesized_evidence(evidence: &nbe::tree::SynthesizedEvidence) -> SynthesizedEvidence {
    match evidence {
        nbe::tree::SynthesizedEvidence::IsSymbol(symbol) => {
            SynthesizedEvidence::IsSymbol { symbol: symbol.clone() }
        }
        nbe::tree::SynthesizedEvidence::Reflectable(evidence) => {
            let evidence = match evidence {
                nbe::tree::ReflectableEvidence::Integer(value) => {
                    ReflectableEvidence::Integer { value: *value }
                }
                nbe::tree::ReflectableEvidence::String(value) => {
                    ReflectableEvidence::String { value: value.clone() }
                }
                nbe::tree::ReflectableEvidence::Boolean(value) => {
                    ReflectableEvidence::Boolean { value: *value }
                }
                nbe::tree::ReflectableEvidence::Ordering(ordering) => {
                    let ordering = match ordering {
                        nbe::tree::ReflectableOrdering::Less => ReflectableOrdering::Less,
                        nbe::tree::ReflectableOrdering::Equal => ReflectableOrdering::Equal,
                        nbe::tree::ReflectableOrdering::Greater => ReflectableOrdering::Greater,
                    };
                    ReflectableEvidence::Ordering { ordering }
                }
            };
            SynthesizedEvidence::Reflectable { evidence }
        }
    }
}

impl State {
    fn traversal(&self) -> &Traversal {
        self.traversal.as_ref().expect("invariant violated: no active function traversal")
    }

    fn traversal_mut(&mut self) -> &mut Traversal {
        self.traversal.as_mut().expect("invariant violated: no active function traversal")
    }

    fn function_name(&self) -> &str {
        &self.traversal().function_name
    }

    fn fresh_function_name(&mut self, preferred: SmolStr) -> SmolStr {
        let preferred = normalize_local_name(preferred);
        deconflict(&mut self.output.function_names, preferred)
    }

    fn fresh_value(&mut self, preferred: SmolStr) -> ValueId {
        let preferred = normalize_local_name(preferred);
        let name = deconflict(&mut self.traversal_mut().value_names, preferred);
        self.output.storage.allocate_value(Value { name })
    }

    fn create_block(&mut self, preferred: impl Into<SmolStr>, parameters: Vec<ValueId>) -> BlockId {
        let preferred = normalize_local_name(preferred.into());
        let name = deconflict(&mut self.traversal_mut().block_names, preferred);
        let block = Block {
            name,
            parameters: parameters.into(),
            instructions: vec![],
            terminator: Terminator::Unreachable,
        };
        let block = self.output.storage.allocate_block(block);
        let traversal = self.traversal_mut();
        traversal.blocks.push(block);
        traversal.unterminated.insert(block);
        block
    }

    fn current_block(&self) -> BlockId {
        self.traversal().current_block.expect("invariant violated: no active basic block")
    }

    fn switch_to(&mut self, block: BlockId) {
        self.traversal_mut().current_block = Some(block);
    }

    fn append(&mut self, instruction: Instruction) {
        let block = self.current_block();
        assert!(
            self.traversal().unterminated.contains(&block),
            "invariant violated: appending to a terminated basic block"
        );
        self.output.storage.append_instruction(block, instruction);
    }

    fn emit(&mut self, preferred: impl Into<SmolStr>, value: InstructionValue) -> ValueId {
        let result = self.fresh_value(preferred.into());
        self.append(Instruction::Assign { result, value });
        result
    }

    fn terminate(&mut self, terminator: Terminator) {
        let block = self.current_block();
        let removed = self.traversal_mut().unterminated.remove(&block);
        assert!(removed, "invariant violated: basic block terminated more than once");
        self.output.storage.set_terminator(block, terminator);
    }

    fn target(&self, block: BlockId, arguments: Vec<ValueId>) -> BlockTarget {
        BlockTarget { block, arguments: arguments.into() }
    }

    fn scope(&self) -> &FxHashMap<nbe::tree::LocalId, ValueId> {
        &self.traversal().scope
    }

    fn set_scope(&mut self, scope: FxHashMap<nbe::tree::LocalId, ValueId>) {
        self.traversal_mut().scope = scope;
    }

    fn bind_local(&mut self, parameter: &nbe::tree::Parameter, value: ValueId) {
        self.traversal_mut().scope.insert(parameter.id, value);
    }

    fn lookup_local(
        &self,
        context: &Context<'_>,
        parameter: &nbe::tree::Parameter,
    ) -> ConversionResult<ValueId> {
        let Some(traversal) = &self.traversal else {
            return Err(context.unsupported(UnsupportedState::MissingLocal {
                local_name: parameter.name.to_string(),
            }));
        };
        traversal.scope.get(&parameter.id).copied().ok_or_else(|| {
            context.unsupported(UnsupportedState::MissingLocal {
                local_name: parameter.name.to_string(),
            })
        })
    }
}

impl Context<'_> {
    fn unsupported(&self, state: UnsupportedState) -> ConversionError {
        ConversionError::Unsupported { file_id: self.functional.file_id, state }
    }
}

fn normalize_local_name(preferred: SmolStr) -> SmolStr {
    let mut normalized = String::new();
    for (position, character) in preferred.chars().enumerate() {
        let valid_initial = character.is_ascii_alphabetic() || character == '_' || character == '$';
        let valid_subsequent = valid_initial || character.is_ascii_digit();
        if position == 0 && !valid_initial {
            normalized.push_str("value_");
        }
        if valid_subsequent {
            normalized.push(character);
        } else {
            normalized.push('_');
        }
    }
    if normalized.is_empty() {
        normalized.push_str("value");
    }
    SmolStr::new(normalized)
}

fn deconflict(names: &mut FxHashMap<SmolStr, u32>, preferred: SmolStr) -> SmolStr {
    let next = names.entry(preferred.clone()).or_default();
    let name = if *next == 0 { preferred } else { format_smolstr!("{preferred}${next}") };
    *next += 1;
    name
}
