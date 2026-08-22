use std::iter;

use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use ssa::tree::{
    BlockId, BlockTarget, Failure, Function, Instruction, InstructionValue, PatternTest,
    RecordUpdate, RecursiveClosure, ReflectableEvidence, ReflectableOrdering, SynthesizedEvidence,
    Terminator, ValueId,
};

use super::Generator;
use super::analysis::{
    BlockExpressionContext, ControlFlow, FunctionContext, ValueExpressionContext,
    block_is_transparent_terminal, helper_captures, substitute_block_parameter,
};
use super::names::NameAllocator;
use super::syntax::{
    call_expression, integer_expression, literal_expression, project_field, projection_expression,
};
use crate::error::ModuleResult;
use crate::pretty::Writer;
use crate::tree::{BinaryOperator, ExpressionId, ObjectProperty, Tree};

struct RenderingOutput<'o, 'd> {
    tree: &'o mut Tree,
    writer: &'o mut Writer<'d>,
}

struct AcyclicHelpers<'a> {
    blocks: &'a FxHashSet<BlockId>,
    captures: &'a FxHashMap<BlockId, Vec<ValueId>>,
}

impl Generator<'_> {
    pub(super) fn render_function(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        name: &str,
        function: &Function,
        exported: bool,
    ) -> ModuleResult<()> {
        let context = FunctionContext::new(self, function);
        let captures = function.captures.iter().map(|capture| context.value(*capture).to_owned());
        let captures = captures.collect_vec();
        let parameters =
            function.parameters.iter().map(|parameter| context.value(*parameter).to_owned());
        let parameters = parameters.collect_vec();
        let (function_parameters, arrow_parameters) = if captures.is_empty() {
            match parameters.split_first() {
                Some((first, rest)) => (vec![first.clone()], rest.to_vec()),
                None => (vec![], vec![]),
            }
        } else {
            (captures, parameters)
        };

        let export = if exported { "export " } else { "" };
        let header = format!("{export}function {name}({}) {{", function_parameters.join(", "));
        writer.block(header, "}", |writer| {
            self.render_curried_function_body(tree, writer, function, &context, &arrow_parameters)
        })
    }

    fn render_curried_function_body(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        function: &Function,
        context: &FunctionContext,
        parameters: &[String],
    ) -> ModuleResult<()> {
        let Some((parameter, parameters)) = parameters.split_first() else {
            return self.render_function_body(tree, writer, function, context);
        };
        writer.block(format!("return {parameter} => {{"), "};", |writer| {
            self.render_curried_function_body(tree, writer, function, context, parameters)
        })
    }

    pub(super) fn render_function_body(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        function: &Function,
        context: &FunctionContext,
    ) -> ModuleResult<()> {
        let control_flow = ControlFlow::new(self.module, function);
        if control_flow.cyclic {
            // Expanding a cyclic CFG cannot terminate. Keep its dispatcher inside this function;
            // source recursion is represented by ordinary calls and never reaches this path.
            self.render_cyclic_function(tree, writer, function, context, &control_flow)
        } else {
            self.render_acyclic_function(tree, writer, function, context, &control_flow)
        }
    }

    fn render_acyclic_function(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        function: &Function,
        context: &FunctionContext,
        control_flow: &ControlFlow,
    ) -> ModuleResult<()> {
        let helpers = function.blocks.iter().copied().filter(|block| {
            let block_value = &self.module.storage[*block];
            control_flow.needs_helper(function.entry, *block)
                && !block_is_transparent_terminal(block_value)
        });
        let helpers = helpers.collect::<FxHashSet<_>>();
        let helper_captures = helper_captures(self.module, function, &helpers);
        let helpers = AcyclicHelpers { blocks: &helpers, captures: &helper_captures };
        for block_id in function.blocks.iter().copied() {
            if !helpers.blocks.contains(&block_id) {
                continue;
            }
            let block = &self.module.storage[block_id];
            let parameters = block.parameters.iter().map(|parameter| context.value(*parameter));
            let parameters = parameters.collect_vec();
            let captures =
                helpers.captures[&block_id].iter().map(|capture| context.value(*capture));
            let parameters = parameters.into_iter().chain(captures);
            let parameters = parameters.collect_vec();
            let header =
                format!("function {}({}) {{", context.block(block_id), parameters.join(", "));
            writer.block(header, "}", |writer| {
                let mut output = RenderingOutput { tree, writer };
                self.render_acyclic_block(&mut output, block_id, context, control_flow, &helpers)
            })?;
            writer.blank();
        }
        let mut output = RenderingOutput { tree, writer };
        self.render_acyclic_block(&mut output, function.entry, context, control_flow, &helpers)
    }

    fn render_acyclic_block(
        &self,
        output: &mut RenderingOutput<'_, '_>,
        block_id: BlockId,
        context: &FunctionContext,
        control_flow: &ControlFlow,
        helpers: &AcyclicHelpers<'_>,
    ) -> ModuleResult<()> {
        let block = &self.module.storage[block_id];
        let inlineable = context.inlineable_values(block_id);
        let expressions = BlockExpressionContext::new(context);
        for instruction in &block.instructions {
            if let Instruction::Assign { result, value } = instruction
                && inlineable.contains(result)
            {
                self.render_closure_function(output.tree, output.writer, value, context)?;
                let expression = self.instruction_expression(output.tree, value, &expressions)?;
                expressions.insert(*result, expression);
            } else {
                self.render_instruction(
                    output.tree,
                    output.writer,
                    instruction,
                    context,
                    &expressions,
                    true,
                )?;
            }
        }
        match &block.terminator {
            Terminator::Return { value } => {
                let value = expressions.expression(output.tree, *value);
                output.writer.expression_line("return ", output.tree, value, ";");
            }
            Terminator::Jump { target } => {
                self.render_acyclic_target(
                    output,
                    target,
                    context,
                    &expressions,
                    control_flow,
                    helpers,
                )?;
            }
            Terminator::Branch { condition, then_target, else_target } => {
                let condition = expressions.expression(output.tree, *condition);
                output.writer.if_else(
                    output.tree,
                    condition,
                    |tree, writer| {
                        let mut output = RenderingOutput { tree, writer };
                        self.render_acyclic_target(
                            &mut output,
                            then_target,
                            context,
                            &expressions,
                            control_flow,
                            helpers,
                        )
                    },
                    |tree, writer| {
                        let mut output = RenderingOutput { tree, writer };
                        self.render_acyclic_target(
                            &mut output,
                            else_target,
                            context,
                            &expressions,
                            control_flow,
                            helpers,
                        )
                    },
                )?;
            }
            Terminator::Fail { failure } => self.render_failure(output.writer, *failure),
            Terminator::Unreachable => {
                output.writer.line("throw new Error(\"unreachable SSA block\");");
            }
        }
        assert!(
            expressions.is_empty(),
            "invariant violated: inline JavaScript block expression was not consumed"
        );
        Ok(())
    }

    fn render_acyclic_target(
        &self,
        output: &mut RenderingOutput<'_, '_>,
        target: &BlockTarget,
        context: &FunctionContext,
        expressions: &dyn ValueExpressionContext,
        control_flow: &ControlFlow,
        helpers: &AcyclicHelpers<'_>,
    ) -> ModuleResult<()> {
        let block = &self.module.storage[target.block];
        debug_assert_eq!(
            block.parameters.len(),
            target.arguments.len(),
            "invariant violated: SSA block target arity does not match block parameters"
        );
        if block.instructions.is_empty() {
            match &block.terminator {
                Terminator::Return { value } => {
                    let value = substitute_block_parameter(block, target, *value).unwrap_or(*value);
                    let value = expressions.expression(output.tree, value);
                    output.writer.expression_line("return ", output.tree, value, ";");
                    return Ok(());
                }
                Terminator::Fail { failure } if block.parameters.is_empty() => {
                    self.render_failure(output.writer, *failure);
                    return Ok(());
                }
                _ => {}
            }
        }
        if helpers.blocks.contains(&target.block) {
            let arguments = target
                .arguments
                .iter()
                .map(|argument| expressions.expression(output.tree, *argument));
            let mut arguments = arguments.collect_vec();
            let captures = helpers.captures[&target.block]
                .iter()
                .map(|capture| expressions.expression(output.tree, *capture));
            arguments.extend(captures);
            let function = output.tree.identifier(context.block(target.block));
            let call = output.tree.call(function, arguments);
            output.writer.expression_line("return ", output.tree, call, ";");
            return Ok(());
        }
        for (&parameter, &argument) in block.parameters.iter().zip(target.arguments.iter()) {
            let argument = expressions.expression(output.tree, argument);
            output.writer.expression_line(
                format!("const {} = ", context.value(parameter)),
                output.tree,
                argument,
                ";",
            );
        }
        self.render_acyclic_block(output, target.block, context, control_flow, helpers)
    }

    fn render_cyclic_function(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        function: &Function,
        context: &FunctionContext,
        control_flow: &ControlFlow,
    ) -> ModuleResult<()> {
        for value in context.mutable_values(function) {
            writer.line(format!("let {value};"));
        }
        writer.line(format!(
            "let {} = {};",
            context.dispatch_block,
            control_flow.index(function.entry)
        ));
        writer.line(format!("let {} = [];", context.dispatch_arguments));
        writer.block("while (true) {", "}", |writer| {
            writer.block(format!("switch ({}) {{", context.dispatch_block), "}", |writer| {
                for block_id in function.blocks.iter().copied() {
                    let block = &self.module.storage[block_id];
                    writer.block(
                        format!("case {}: {{", control_flow.index(block_id)),
                        "}",
                        |writer| -> ModuleResult<()> {
                            for (position, parameter) in block.parameters.iter().enumerate() {
                                writer.line(format!(
                                    "{} = {}[{}];",
                                    context.value(*parameter),
                                    context.dispatch_arguments,
                                    position
                                ));
                            }
                            for instruction in &block.instructions {
                                self.render_instruction(
                                    tree,
                                    writer,
                                    instruction,
                                    context,
                                    context,
                                    false,
                                )?;
                            }
                            self.render_cyclic_terminator(
                                tree,
                                writer,
                                &block.terminator,
                                context,
                                control_flow,
                            )?;
                            Ok(())
                        },
                    )?;
                }
                Ok(())
            })
        })
    }

    fn render_cyclic_terminator(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        terminator: &Terminator,
        context: &FunctionContext,
        control_flow: &ControlFlow,
    ) -> ModuleResult<()> {
        match terminator {
            Terminator::Return { value } => {
                let value = context.expression(tree, *value);
                writer.expression_line("return ", tree, value, ";");
            }
            Terminator::Jump { target } => {
                self.render_cyclic_target(tree, writer, target, context, control_flow);
            }
            Terminator::Branch { condition, then_target, else_target } => {
                let condition = context.expression(tree, *condition);
                writer.if_else(
                    tree,
                    condition,
                    |tree, writer| -> ModuleResult<()> {
                        self.render_cyclic_target(tree, writer, then_target, context, control_flow);
                        Ok(())
                    },
                    |tree, writer| -> ModuleResult<()> {
                        self.render_cyclic_target(tree, writer, else_target, context, control_flow);
                        Ok(())
                    },
                )?;
            }
            Terminator::Fail { failure } => self.render_failure(writer, *failure),
            Terminator::Unreachable => {
                writer.line("throw new Error(\"unreachable SSA block\");");
            }
        }
        Ok(())
    }

    fn render_cyclic_target(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        target: &BlockTarget,
        context: &FunctionContext,
        control_flow: &ControlFlow,
    ) {
        let arguments = target.arguments.iter().map(|argument| context.expression(tree, *argument));
        let arguments = arguments.collect_vec();
        let arguments = tree.array(arguments);
        writer.expression_line(format!("{} = ", context.dispatch_arguments), tree, arguments, ";");
        writer.line(format!("{} = {};", context.dispatch_block, control_flow.index(target.block)));
        writer.line("continue;");
    }

    fn render_instruction(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        instruction: &Instruction,
        context: &FunctionContext,
        expressions: &dyn ValueExpressionContext,
        declare: bool,
    ) -> ModuleResult<()> {
        match instruction {
            Instruction::Assign { result, value } => {
                self.render_closure_function(tree, writer, value, context)?;
                let expression = self.instruction_expression(tree, value, expressions)?;
                let binding = if declare { "const " } else { "" };
                writer.expression_line(
                    format!("{binding}{} = ", context.value(*result)),
                    tree,
                    expression,
                    ";",
                );
            }
            Instruction::RecursiveClosures { bindings } => {
                for binding in bindings.iter() {
                    let name = context.closure(binding.function);
                    let function = &self.module.storage[binding.function];
                    self.render_function(tree, writer, name, function, false)?;
                }
                for binding in bindings.iter() {
                    let expression = self.recursive_closure_expression(tree, binding, context);
                    let declaration = if declare { "const " } else { "" };
                    writer.expression_line(
                        format!("{declaration}{} = ", context.value(binding.result)),
                        tree,
                        expression,
                        ";",
                    );
                }
            }
        }
        Ok(())
    }

    fn render_closure_function(
        &self,
        tree: &mut Tree,
        writer: &mut Writer<'_>,
        value: &InstructionValue,
        context: &FunctionContext,
    ) -> ModuleResult<()> {
        let InstructionValue::Closure { function, .. } = value else {
            return Ok(());
        };
        let name = context.closure(*function);
        let function = &self.module.storage[*function];
        self.render_function(tree, writer, name, function, false)
    }

    pub(super) fn instruction_expression(
        &self,
        tree: &mut Tree,
        value: &InstructionValue,
        context: &dyn ValueExpressionContext,
    ) -> ModuleResult<ExpressionId> {
        match value {
            InstructionValue::Literal { literal } => {
                literal_expression(tree, literal, self.module.file_id)
            }
            InstructionValue::Array { elements } => {
                let elements = elements.iter().map(|element| context.expression(tree, *element));
                let elements = elements.collect_vec();
                Ok(tree.array(elements))
            }
            InstructionValue::Record { fields } => {
                let fields = fields.iter().map(|field| ObjectProperty::Field {
                    name: field.field.name.to_string(),
                    value: context.expression(tree, field.value),
                });
                let fields = fields.collect_vec();
                Ok(tree.object(fields))
            }
            InstructionValue::RecordUpdate { record, updates } => {
                let record = context.expression(tree, *record);
                Ok(self.record_update_expression(tree, record, updates, context))
            }
            InstructionValue::Project { record, field } => {
                let record = context.expression(tree, *record);
                Ok(project_field(tree, record, field))
            }
            InstructionValue::Constructor { global } => self.global_expression(tree, global),
            InstructionValue::Global { global } => self.global_expression(tree, global),
            InstructionValue::Closure { function, captures } => {
                let name = tree.identifier(context.closure(*function));
                if captures.is_empty() {
                    Ok(name)
                } else {
                    let captures =
                        captures.iter().map(|capture| context.expression(tree, *capture));
                    let captures = captures.collect_vec();
                    Ok(tree.call(name, captures))
                }
            }
            InstructionValue::Call { calling_convention, function, arguments } => {
                let function = context.expression(tree, *function);
                let arguments =
                    arguments.iter().map(|argument| context.expression(tree, *argument));
                let arguments = arguments.collect_vec();
                Ok(call_expression(tree, *calling_convention, function, arguments))
            }
            InstructionValue::Test { value, test } => {
                let value = context.expression(tree, *value);
                self.pattern_test_expression(tree, value, test)
            }
            InstructionValue::Extract { value, projection } => {
                let value = context.expression(tree, *value);
                Ok(projection_expression(tree, value, projection))
            }
            InstructionValue::EffectPure { value } => {
                let value = context.expression(tree, *value);
                Ok(tree.arrow(vec![], value))
            }
            InstructionValue::EffectBind { action, continuation } => {
                let action = context.expression(tree, *action);
                let action = tree.call(action, vec![]);
                let continuation = context.expression(tree, *continuation);
                let continuation = tree.call(continuation, vec![action]);
                let result = tree.call(continuation, vec![]);
                Ok(tree.arrow(vec![], result))
            }
            InstructionValue::SynthesizedEvidence { evidence } => {
                self.synthesized_evidence_expression(tree, evidence)
            }
            InstructionValue::TrivialEvidence => Ok(tree.object(vec![])),
        }
    }

    fn recursive_closure_expression(
        &self,
        tree: &mut Tree,
        closure: &RecursiveClosure,
        context: &FunctionContext,
    ) -> ExpressionId {
        let function = &self.module.storage[closure.function];
        let parameter = function
            .parameters
            .first()
            .expect("invariant violated: recursive SSA closure has no source parameter");
        let reserved = iter::chain(context.names.values(), &self.reserved_module_names).cloned();
        let mut allocator = NameAllocator::with_reserved(reserved);
        let parameter = allocator.allocate(&self.module.storage[*parameter].name);
        let captures = closure.captures.iter().map(|capture| context.expression(tree, *capture));
        let captures = captures.collect_vec();
        let function = tree.identifier(context.closure(closure.function));
        // Defer capture evaluation until every binding in the recursive group has been initialized.
        let function = tree.call(function, captures);
        let argument = tree.identifier(&parameter);
        let body = tree.call(function, vec![argument]);
        tree.arrow(vec![parameter], body)
    }

    fn record_update_expression(
        &self,
        tree: &mut Tree,
        record: ExpressionId,
        updates: &[RecordUpdate],
        context: &dyn ValueExpressionContext,
    ) -> ExpressionId {
        let mut properties = vec![ObjectProperty::Spread(record)];
        for update in updates {
            match update {
                RecordUpdate::Leaf { field, value } => {
                    properties.push(ObjectProperty::Field {
                        name: field.name.to_string(),
                        value: context.expression(tree, *value),
                    });
                }
                RecordUpdate::Branch { field, updates } => {
                    let nested = project_field(tree, record, field);
                    let nested = self.record_update_expression(tree, nested, updates, context);
                    properties.push(ObjectProperty::Field {
                        name: field.name.to_string(),
                        value: nested,
                    });
                }
            }
        }
        tree.object(properties)
    }

    fn pattern_test_expression(
        &self,
        tree: &mut Tree,
        value: ExpressionId,
        test: &PatternTest,
    ) -> ModuleResult<ExpressionId> {
        match test {
            PatternTest::Literal { literal } => {
                let literal = literal_expression(tree, literal, self.module.file_id)?;
                Ok(tree.binary(BinaryOperator::StrictEqual, value, literal))
            }
            PatternTest::ArrayLength { length } => {
                let array = tree.identifier("Array");
                let is_array = tree.member(array, "isArray");
                let is_array = tree.call(is_array, vec![value]);
                let actual_length = tree.member(value, "length");
                let expected_length = tree.number(length.to_string());
                let length =
                    tree.binary(BinaryOperator::StrictEqual, actual_length, expected_length);
                Ok(tree.binary(BinaryOperator::LogicalAnd, is_array, length))
            }
            PatternTest::Constructor { global } => {
                let array = tree.identifier("Array");
                let is_array = tree.member(array, "isArray");
                let is_array = tree.call(is_array, vec![value]);
                let zero = tree.number("0");
                let actual_tag = tree.index(value, zero);
                let expected_tag = tree.string(global.item_name.as_str());
                let tagged = tree.binary(BinaryOperator::StrictEqual, actual_tag, expected_tag);
                Ok(tree.binary(BinaryOperator::LogicalAnd, is_array, tagged))
            }
        }
    }

    fn synthesized_evidence_expression(
        &self,
        tree: &mut Tree,
        evidence: &SynthesizedEvidence,
    ) -> ModuleResult<ExpressionId> {
        let (field, value) = match evidence {
            SynthesizedEvidence::IsSymbol { symbol } => {
                ("reflectSymbol", tree.string(symbol.as_str()))
            }
            SynthesizedEvidence::Reflectable { evidence } => {
                let value = match evidence {
                    ReflectableEvidence::Integer { value } => integer_expression(tree, *value),
                    ReflectableEvidence::String { value } => tree.string(value.as_str()),
                    ReflectableEvidence::Boolean { value } => tree.boolean(*value),
                    ReflectableEvidence::Ordering { ordering } => {
                        let tag = match ordering {
                            ReflectableOrdering::Less => "LT",
                            ReflectableOrdering::Equal => "EQ",
                            ReflectableOrdering::Greater => "GT",
                        };
                        let tag = tree.string(tag);
                        tree.array(vec![tag])
                    }
                };
                ("reflectType", value)
            }
        };
        let reflection = tree.arrow(vec!["$proxy".into()], value);
        Ok(tree.object(vec![ObjectProperty::Field { name: field.into(), value: reflection }]))
    }

    fn render_failure(&self, writer: &mut Writer<'_>, failure: Failure) {
        match failure {
            Failure::PatternMatch => {
                writer.line("throw new Error(\"Pattern match failure\");");
            }
        }
    }
}
