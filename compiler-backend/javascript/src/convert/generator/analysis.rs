use std::cell::RefCell;

use files::FileId;
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use ssa::tree::{
    Block, BlockId, BlockTarget, Function, FunctionId, Global, GlobalIdentity, InstanceIdentity,
    Instruction, InstructionValue, PatternTest, Projection, RecordUpdate, Terminator, ValueId,
};

use super::Generator;
use super::names::NameAllocator;
use crate::error::UnsupportedState;
use crate::tree::{ExpressionId, Tree};

pub(super) struct FunctionContext {
    pub(super) names: FxHashMap<ValueId, String>,
    block_names: FxHashMap<BlockId, String>,
    inlineable_values: FxHashMap<BlockId, FxHashSet<ValueId>>,
    pub(super) dispatch_block: String,
    pub(super) dispatch_arguments: String,
}

pub(super) trait ValueExpressionContext {
    fn expression(&self, tree: &mut Tree, value: ValueId) -> ExpressionId;
}

pub(super) struct InlineExpressionContext {
    expressions: RefCell<FxHashMap<ValueId, ExpressionId>>,
}

impl InlineExpressionContext {
    pub(super) fn new(expressions: Vec<(ValueId, ExpressionId)>) -> InlineExpressionContext {
        let expressions = expressions.into_iter();
        let expressions = expressions.collect();
        InlineExpressionContext { expressions: RefCell::new(expressions) }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.expressions.borrow().is_empty()
    }
}

impl ValueExpressionContext for InlineExpressionContext {
    fn expression(&self, _tree: &mut Tree, value: ValueId) -> ExpressionId {
        self.expressions
            .borrow_mut()
            .remove(&value)
            .expect("invariant violated: inline JavaScript expression has no SSA operand")
    }
}

pub(super) struct BlockExpressionContext<'c> {
    function: &'c FunctionContext,
    expressions: RefCell<FxHashMap<ValueId, ExpressionId>>,
}

impl<'c> BlockExpressionContext<'c> {
    pub(super) fn new(function: &'c FunctionContext) -> BlockExpressionContext<'c> {
        BlockExpressionContext { function, expressions: RefCell::new(FxHashMap::default()) }
    }

    pub(super) fn insert(&self, value: ValueId, expression: ExpressionId) {
        self.expressions.borrow_mut().insert(value, expression);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.expressions.borrow().is_empty()
    }
}

impl ValueExpressionContext for BlockExpressionContext<'_> {
    fn expression(&self, tree: &mut Tree, value: ValueId) -> ExpressionId {
        self.expressions
            .borrow_mut()
            .remove(&value)
            .unwrap_or_else(|| self.function.expression(tree, value))
    }
}

impl FunctionContext {
    pub(super) fn new(generator: &Generator<'_>, function: &Function) -> FunctionContext {
        let mut allocator =
            NameAllocator::with_reserved(generator.reserved_module_names.iter().cloned());
        let mut names = FxHashMap::default();
        for value in function_values(generator.module, function) {
            names
                .entry(value)
                .or_insert_with(|| allocator.allocate(&generator.module.storage[value].name));
        }
        let mut block_names = FxHashMap::default();
        for block in function.blocks.iter().copied() {
            let name = allocator.allocate(&generator.module.storage[block].name);
            block_names.insert(block, name);
        }
        let inlineable_values = locally_inlineable_values(generator.module, function);
        let dispatch_block = allocator.allocate("$block");
        let dispatch_arguments = allocator.allocate("$arguments");
        FunctionContext {
            names,
            block_names,
            inlineable_values,
            dispatch_block,
            dispatch_arguments,
        }
    }

    pub(super) fn value(&self, value: ValueId) -> &str {
        self.names
            .get(&value)
            .map(String::as_str)
            .expect("invariant violated: JavaScript SSA value has no allocated name")
    }

    pub(super) fn block(&self, block: BlockId) -> &str {
        self.block_names
            .get(&block)
            .map(String::as_str)
            .expect("invariant violated: JavaScript SSA block has no allocated name")
    }

    pub(super) fn inlineable_values(&self, block: BlockId) -> &FxHashSet<ValueId> {
        self.inlineable_values
            .get(&block)
            .expect("invariant violated: JavaScript SSA block has no inlining analysis")
    }

    pub(super) fn mutable_values(&self, function: &Function) -> Vec<&str> {
        let excluded = function.captures.iter().chain(function.parameters.iter()).copied();
        let excluded = excluded.collect::<FxHashSet<_>>();
        let values = self
            .names
            .iter()
            .filter(|(value, _)| !excluded.contains(value))
            .map(|(_, name)| name.as_str());
        let mut values = values.collect_vec();
        values.sort_unstable();
        values
    }
}

impl ValueExpressionContext for FunctionContext {
    fn expression(&self, tree: &mut Tree, value: ValueId) -> ExpressionId {
        tree.identifier(self.value(value))
    }
}

pub(super) struct ControlFlow {
    predecessors: FxHashMap<BlockId, usize>,
    indices: FxHashMap<BlockId, usize>,
    pub(super) cyclic: bool,
}

impl ControlFlow {
    pub(super) fn new(module: &ssa::tree::Module, function: &Function) -> ControlFlow {
        let mut predecessors = FxHashMap::default();
        let mut edges = FxHashMap::default();
        let mut indices = FxHashMap::default();
        for (position, block) in function.blocks.iter().copied().enumerate() {
            predecessors.insert(block, 0);
            indices.insert(block, position);
        }
        for block in function.blocks.iter().copied() {
            let successors = block_successors(&module.storage[block].terminator);
            for successor in &successors {
                *predecessors.entry(*successor).or_default() += 1;
            }
            edges.insert(block, successors);
        }
        let cyclic = graph_is_cyclic(function, &edges);
        ControlFlow { predecessors, indices, cyclic }
    }

    pub(super) fn needs_helper(&self, entry: BlockId, block: BlockId) -> bool {
        block != entry && self.predecessors.get(&block).copied().unwrap_or_default() != 1
    }

    pub(super) fn index(&self, block: BlockId) -> usize {
        self.indices[&block]
    }
}

pub(super) fn helper_captures(
    module: &ssa::tree::Module,
    function: &Function,
    helpers: &FxHashSet<BlockId>,
) -> FxHashMap<BlockId, Vec<ValueId>> {
    // Shared blocks become function-scope helpers. A value defined in a branch that dominates a
    // join is not lexically visible there, so pass every free SSA value as an explicit parameter.
    let mut regions = HelperRegions {
        module,
        helpers,
        memo: FxHashMap::default(),
        visiting: FxHashSet::default(),
    };
    for helper in helpers.iter().copied() {
        regions.capture_set(helper);
    }

    let value_order = function_values(module, function);
    let mut captures = FxHashMap::default();
    for helper in helpers.iter().copied() {
        let capture_set = &regions.memo[&helper];
        let mut seen = FxHashSet::default();
        let ordered = value_order
            .iter()
            .copied()
            .filter(|value| capture_set.contains(value) && seen.insert(*value));
        captures.insert(helper, ordered.collect_vec());
    }
    captures
}

struct HelperRegions<'m, 'h> {
    module: &'m ssa::tree::Module,
    helpers: &'h FxHashSet<BlockId>,
    memo: FxHashMap<BlockId, FxHashSet<ValueId>>,
    visiting: FxHashSet<BlockId>,
}

impl HelperRegions<'_, '_> {
    fn capture_set(&mut self, helper: BlockId) -> FxHashSet<ValueId> {
        if let Some(captures) = self.memo.get(&helper) {
            return captures.clone();
        }
        assert!(
            self.visiting.insert(helper),
            "invariant violated: acyclic JavaScript helpers contain a cycle"
        );
        let mut definitions = FxHashSet::default();
        let mut uses = FxHashSet::default();
        let mut visited_blocks = FxHashSet::default();
        self.collect_region(helper, &mut visited_blocks, &mut definitions, &mut uses);
        self.visiting.remove(&helper);
        uses.retain(|value| !definitions.contains(value));
        self.memo.insert(helper, uses.clone());
        uses
    }

    fn collect_region(
        &mut self,
        block_id: BlockId,
        visited_blocks: &mut FxHashSet<BlockId>,
        definitions: &mut FxHashSet<ValueId>,
        uses: &mut FxHashSet<ValueId>,
    ) {
        if !visited_blocks.insert(block_id) {
            return;
        }
        let block = &self.module.storage[block_id];
        definitions.extend(block.parameters.iter().copied());
        for instruction in &block.instructions {
            match instruction {
                Instruction::Assign { result, value } => {
                    uses.extend(instruction_value_uses(value));
                    definitions.insert(*result);
                }
                Instruction::RecursiveClosures { bindings } => {
                    for binding in bindings.iter() {
                        uses.extend(binding.captures.iter().copied());
                        definitions.insert(binding.result);
                    }
                }
            }
        }

        match &block.terminator {
            Terminator::Return { value } => {
                uses.insert(*value);
            }
            Terminator::Jump { target } => {
                self.collect_target(target, visited_blocks, definitions, uses);
            }
            Terminator::Branch { condition, then_target, else_target } => {
                uses.insert(*condition);
                self.collect_target(then_target, visited_blocks, definitions, uses);
                self.collect_target(else_target, visited_blocks, definitions, uses);
            }
            Terminator::Fail { .. } | Terminator::Unreachable => {}
        }
    }

    fn collect_target(
        &mut self,
        target: &BlockTarget,
        visited_blocks: &mut FxHashSet<BlockId>,
        definitions: &mut FxHashSet<ValueId>,
        uses: &mut FxHashSet<ValueId>,
    ) {
        uses.extend(target.arguments.iter().copied());
        if self.helpers.contains(&target.block) {
            let captures = self.capture_set(target.block);
            uses.extend(captures);
        } else {
            self.collect_region(target.block, visited_blocks, definitions, uses);
        }
    }
}

pub(super) fn collect_module_references(module: &ssa::tree::Module) -> Vec<&Global> {
    let mut external_globals = Vec::new();
    let mut external_identities = FxHashSet::default();
    for (_, block) in module.storage.blocks() {
        for instruction in &block.instructions {
            match instruction {
                Instruction::Assign { value, .. } => match value {
                    InstructionValue::Constructor { global }
                    | InstructionValue::Global { global }
                        if identity_file(global.identity) != module.file_id
                            && external_identities.insert(global.identity) =>
                    {
                        external_globals.push(global);
                    }
                    InstructionValue::Test {
                        test: PatternTest::Constructor { global }, ..
                    }
                    | InstructionValue::Extract {
                        projection: Projection::ConstructorArgument { constructor: global, .. },
                        ..
                    } if identity_file(global.identity) != module.file_id
                        && external_identities.insert(global.identity) =>
                    {
                        external_globals.push(global);
                    }
                    _ => {}
                },
                Instruction::RecursiveClosures { .. } => {}
            }
        }
    }
    external_globals
}

pub(super) fn function_globals(
    module: &ssa::tree::Module,
    function: FunctionId,
) -> FxHashSet<GlobalIdentity> {
    let mut globals = FxHashSet::default();
    for block in module.storage[function].blocks.iter().copied() {
        for instruction in &module.storage[block].instructions {
            if let Instruction::Assign { value: InstructionValue::Global { global }, .. } =
                instruction
            {
                globals.insert(global.identity);
            }
        }
    }
    globals
}

#[derive(Debug, Clone, Copy)]
pub(super) enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

pub(super) fn visit_initializer(
    position: usize,
    dependencies: &[Vec<usize>],
    states: &mut [VisitState],
    ordered: &mut Vec<usize>,
) -> Result<(), UnsupportedState> {
    match states[position] {
        VisitState::Visited => return Ok(()),
        VisitState::Visiting => return Err(UnsupportedState::CyclicInitializers),
        VisitState::Unvisited => states[position] = VisitState::Visiting,
    }
    for dependency in dependencies[position].iter().copied() {
        visit_initializer(dependency, dependencies, states, ordered)?;
    }
    states[position] = VisitState::Visited;
    ordered.push(position);
    Ok(())
}

pub(super) fn substitute_block_parameter(
    block: &Block,
    target: &BlockTarget,
    value: ValueId,
) -> Option<ValueId> {
    let position = block.parameters.iter().position(|parameter| *parameter == value)?;
    target.arguments.get(position).copied()
}

pub(super) fn block_is_transparent_terminal(block: &Block) -> bool {
    if !block.instructions.is_empty() {
        return false;
    }
    match block.terminator {
        Terminator::Return { .. } => true,
        Terminator::Fail { .. } => block.parameters.is_empty(),
        _ => false,
    }
}

pub(super) fn identity_file(identity: GlobalIdentity) -> FileId {
    match identity {
        GlobalIdentity::Term { file_id, .. } => file_id,
        GlobalIdentity::Instance { identity } => match identity {
            InstanceIdentity::Declared { file_id, .. }
            | InstanceIdentity::Derived { file_id, .. } => file_id,
        },
    }
}

fn function_values(module: &ssa::tree::Module, function: &Function) -> Vec<ValueId> {
    let mut values = Vec::new();
    values.extend(function.captures.iter().copied());
    values.extend(function.parameters.iter().copied());
    for block in function.blocks.iter().copied() {
        let block = &module.storage[block];
        values.extend(block.parameters.iter().copied());
        for instruction in &block.instructions {
            match instruction {
                Instruction::Assign { result, .. } => values.push(*result),
                Instruction::RecursiveClosures { bindings } => {
                    values.extend(bindings.iter().map(|binding| binding.result));
                }
            }
        }
    }
    values
}

fn block_successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Jump { target } => vec![target.block],
        Terminator::Branch { then_target, else_target, .. } => {
            vec![then_target.block, else_target.block]
        }
        Terminator::Return { .. } | Terminator::Fail { .. } | Terminator::Unreachable => vec![],
    }
}

fn graph_is_cyclic(function: &Function, edges: &FxHashMap<BlockId, Vec<BlockId>>) -> bool {
    fn visit(
        block: BlockId,
        edges: &FxHashMap<BlockId, Vec<BlockId>>,
        visiting: &mut FxHashSet<BlockId>,
        visited: &mut FxHashSet<BlockId>,
    ) -> bool {
        if visited.contains(&block) {
            return false;
        }
        if !visiting.insert(block) {
            return true;
        }
        let cyclic = edges
            .get(&block)
            .into_iter()
            .flatten()
            .copied()
            .any(|successor| visit(successor, edges, visiting, visited));
        visiting.remove(&block);
        visited.insert(block);
        cyclic
    }

    let mut visiting = FxHashSet::default();
    let mut visited = FxHashSet::default();
    function.blocks.iter().copied().any(|block| visit(block, edges, &mut visiting, &mut visited))
}

pub(super) fn instruction_value_uses(value: &InstructionValue) -> Vec<ValueId> {
    match value {
        InstructionValue::Array { elements } => elements.to_vec(),
        InstructionValue::Record { fields } => {
            let values = fields.iter().map(|field| field.value);
            values.collect_vec()
        }
        InstructionValue::RecordUpdate { record, updates } => {
            let mut values = vec![*record];
            collect_record_update_values(updates, &mut values);
            values
        }
        InstructionValue::Project { record, .. } => vec![*record],
        InstructionValue::Closure { captures, .. } => captures.to_vec(),
        InstructionValue::Call { function, arguments, .. } => {
            let mut values = vec![*function];
            values.extend(arguments.iter().copied());
            values
        }
        InstructionValue::Test { value, .. } | InstructionValue::Extract { value, .. } => {
            vec![*value]
        }
        InstructionValue::EffectPure { value } => vec![*value],
        InstructionValue::EffectBind { action, continuation } => vec![*action, *continuation],
        InstructionValue::Literal { .. }
        | InstructionValue::Constructor { .. }
        | InstructionValue::Global { .. }
        | InstructionValue::SynthesizedEvidence { .. }
        | InstructionValue::TrivialEvidence => vec![],
    }
}

fn locally_inlineable_values(
    module: &ssa::tree::Module,
    function: &Function,
) -> FxHashMap<BlockId, FxHashSet<ValueId>> {
    let mut function_uses = FxHashMap::<ValueId, usize>::default();
    for block in function.blocks.iter().copied() {
        let block = &module.storage[block];
        for instruction in &block.instructions {
            match instruction {
                Instruction::Assign { value, .. } => {
                    for value in instruction_value_uses(value) {
                        *function_uses.entry(value).or_default() += 1;
                    }
                }
                Instruction::RecursiveClosures { bindings } => {
                    for binding in bindings.iter() {
                        for capture in binding.captures.iter().copied() {
                            *function_uses.entry(capture).or_default() += 1;
                        }
                    }
                }
            }
        }
        for value in terminator_value_uses(&block.terminator) {
            *function_uses.entry(value).or_default() += 1;
        }
    }

    let mut inlineable = FxHashMap::default();
    for block_id in function.blocks.iter().copied() {
        let block = &module.storage[block_id];
        let mut local_uses = FxHashSet::default();
        for instruction in &block.instructions {
            if let Instruction::Assign { value, .. } = instruction {
                local_uses.extend(instruction_value_uses(value));
            }
        }
        local_uses.extend(terminator_value_uses(&block.terminator));

        let values = block.instructions.iter().filter_map(|instruction| {
            let Instruction::Assign { result, value } = instruction else {
                return None;
            };
            // Keep substitutions in their defining block and do not move them past an operation
            // whose evaluation must remain ordered. This also keeps helper capture scopes intact.
            (function_uses.get(result) == Some(&1)
                && local_uses.contains(result)
                && instruction_value_is_locally_inlineable(value)
                && local_use_precedes_evaluation_barrier(block, *result))
            .then_some(*result)
        });
        inlineable.insert(block_id, values.collect());
    }
    inlineable
}

fn instruction_value_is_locally_inlineable(value: &InstructionValue) -> bool {
    match value {
        InstructionValue::Literal { .. }
        | InstructionValue::Constructor { .. }
        | InstructionValue::Global { .. }
        | InstructionValue::Project { .. } => true,
        InstructionValue::Closure { captures, .. } => captures.is_empty(),
        InstructionValue::Array { .. }
        | InstructionValue::Record { .. }
        | InstructionValue::RecordUpdate { .. }
        | InstructionValue::Call { .. }
        | InstructionValue::Test { .. }
        | InstructionValue::Extract { .. }
        | InstructionValue::EffectPure { .. }
        | InstructionValue::EffectBind { .. }
        | InstructionValue::SynthesizedEvidence { .. }
        | InstructionValue::TrivialEvidence => false,
    }
}

fn local_use_precedes_evaluation_barrier(block: &Block, result: ValueId) -> bool {
    let Some(position) = block.instructions.iter().position(|instruction| {
        matches!(instruction, Instruction::Assign { result: assigned, .. } if *assigned == result)
    }) else {
        return false;
    };
    for instruction in &block.instructions[position + 1..] {
        let Instruction::Assign { value, .. } = instruction else {
            return false;
        };
        if instruction_value_uses(value).contains(&result) {
            return true;
        }
        if !instruction_value_is_locally_inlineable(value) {
            return false;
        }
    }
    terminator_value_uses(&block.terminator).contains(&result)
}

fn terminator_value_uses(terminator: &Terminator) -> Vec<ValueId> {
    match terminator {
        Terminator::Return { value } => vec![*value],
        Terminator::Jump { target } => target.arguments.to_vec(),
        Terminator::Branch { condition, then_target, else_target } => {
            let mut values = vec![*condition];
            values.extend(then_target.arguments.iter().copied());
            values.extend(else_target.arguments.iter().copied());
            values
        }
        Terminator::Fail { .. } | Terminator::Unreachable => vec![],
    }
}

pub(super) fn initializer_value_is_inlineable(value: &InstructionValue) -> bool {
    !matches!(
        value,
        InstructionValue::RecordUpdate { .. }
            | InstructionValue::Test { .. }
            | InstructionValue::EffectPure { .. }
            | InstructionValue::EffectBind { .. }
    )
}

fn collect_record_update_values(updates: &[RecordUpdate], values: &mut Vec<ValueId>) {
    for update in updates {
        match update {
            RecordUpdate::Leaf { value, .. } => values.push(*value),
            RecordUpdate::Branch { updates, .. } => {
                collect_record_update_values(updates, values);
            }
        }
    }
}
