use std::cell::RefCell;

use files::FileId;
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use ssa::tree::{
    Block, BlockId, BlockTarget, CallingConvention, DeclarationKind, Function, FunctionId, Global,
    GlobalIdentity, InstanceIdentity, Instruction, InstructionValue, PatternTest, Projection,
    RecordUpdate, Terminator, ValueId,
};

use super::Generator;
use super::names::NameAllocator;
use crate::error::UnsupportedState;
use crate::tree::{ExpressionId, Tree};

pub(super) struct FunctionContext {
    pub(super) names: FxHashMap<ValueId, String>,
    closure_names: FxHashMap<FunctionId, String>,
    block_names: FxHashMap<BlockId, String>,
    inlineable_values: FxHashMap<BlockId, FxHashSet<ValueId>>,
    pub(super) dispatch_block: String,
    pub(super) dispatch_arguments: String,
}

pub(super) trait ValueExpressionContext {
    fn expression(&self, tree: &mut Tree, value: ValueId) -> ExpressionId;

    fn closure(&self, function: FunctionId) -> &str;
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

    fn closure(&self, _function: FunctionId) -> &str {
        panic!("invariant violated: inline JavaScript expression contains a closure")
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

    fn closure(&self, function: FunctionId) -> &str {
        self.function.closure(function)
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
        let mut closure_names = FxHashMap::default();
        for function_id in direct_closures(generator.module, function) {
            let function_name = &generator.module.storage[function_id].name;
            let preferred_name = locally_scoped_function_name(function_name);
            let name = allocator.allocate(preferred_name);
            closure_names.insert(function_id, name);
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
            closure_names,
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

    pub(super) fn closure(&self, function: FunctionId) -> &str {
        self.closure_names
            .get(&function)
            .map(String::as_str)
            .expect("invariant violated: JavaScript closure function has no allocated name")
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

fn direct_closures(module: &ssa::tree::Module, function: &Function) -> Vec<FunctionId> {
    let mut closures = vec![];
    for block in function.blocks.iter().copied() {
        for instruction in &module.storage[block].instructions {
            match instruction {
                Instruction::Assign {
                    value: InstructionValue::Closure { function, .. }, ..
                } => closures.push(*function),
                Instruction::RecursiveClosures { bindings } => {
                    closures.extend(bindings.iter().map(|binding| binding.function));
                }
                Instruction::RecursiveLazyInitializers { bindings } => {
                    closures.extend(bindings.iter().map(|binding| binding.initializer));
                }
                Instruction::Assign { .. } => {}
            }
        }
    }
    closures
}

fn locally_scoped_function_name(name: &str) -> &str {
    let Some((preferred, suffix)) = name.rsplit_once('$') else {
        return name;
    };
    if !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit()) {
        preferred
    } else {
        name
    }
}

impl ValueExpressionContext for FunctionContext {
    fn expression(&self, tree: &mut Tree, value: ValueId) -> ExpressionId {
        tree.identifier(self.value(value))
    }

    fn closure(&self, function: FunctionId) -> &str {
        FunctionContext::closure(self, function)
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
                Instruction::RecursiveLazyInitializers { bindings } => {
                    for binding in bindings.iter() {
                        uses.extend(binding.captures.iter().copied());
                        definitions.insert(binding.accessor);
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
                Instruction::RecursiveClosures { .. }
                | Instruction::RecursiveLazyInitializers { .. } => {}
            }
        }
    }
    external_globals
}

pub(super) fn has_local_lazy_initializers(module: &ssa::tree::Module) -> bool {
    module.storage.blocks().any(|(_, block)| {
        block
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::RecursiveLazyInitializers { .. }))
    })
}

pub(super) fn function_globals(
    module: &ssa::tree::Module,
    function: FunctionId,
) -> FxHashSet<GlobalIdentity> {
    function_globals_eager(module, function, &mut FxHashSet::default())
}

fn function_globals_eager(
    module: &ssa::tree::Module,
    function: FunctionId,
    visited: &mut FxHashSet<FunctionId>,
) -> FxHashSet<GlobalIdentity> {
    if !visited.insert(function) {
        return FxHashSet::default();
    }
    let mut globals = FxHashSet::default();
    for block in module.storage[function].blocks.iter().copied() {
        for instruction in &module.storage[block].instructions {
            match instruction {
                Instruction::Assign { value: InstructionValue::Global { global }, .. } => {
                    globals.insert(global.identity);
                }
                Instruction::RecursiveLazyInitializers { bindings } => {
                    for binding in bindings.iter() {
                        globals.extend(function_globals_eager(
                            module,
                            binding.initializer,
                            visited,
                        ));
                    }
                }
                Instruction::Assign { .. } | Instruction::RecursiveClosures { .. } => {}
            }
        }
    }
    globals
}

pub(super) fn cyclic_instance_initializers(
    module: &ssa::tree::Module,
) -> FxHashSet<GlobalIdentity> {
    let (identities, dependencies) = initializer_dependencies(module);

    let mut cyclic = FxHashSet::default();
    for position in 0..identities.len() {
        let mut visited = FxHashSet::default();
        if reaches_initializer(position, position, &dependencies, &mut visited) {
            cyclic.insert(position);
        }
    }

    if cyclic
        .iter()
        .any(|position| !matches!(identities[*position], GlobalIdentity::Instance { .. }))
    {
        // The ordinary initializer sorter retains its deterministic rejection for non-dictionary
        // cycles instead of exposing partially initialized values through runtime laziness.
        return FxHashSet::default();
    }

    cyclic.into_iter().map(|position| identities[position]).collect()
}

fn initializer_dependencies(module: &ssa::tree::Module) -> (Vec<GlobalIdentity>, Vec<Vec<usize>>) {
    let values = module.declarations.iter().filter_map(|declaration| match declaration.kind {
        DeclarationKind::Value { initializer } => Some((declaration.global.identity, initializer)),
        DeclarationKind::Function { .. }
        | DeclarationKind::Constructor { .. }
        | DeclarationKind::Foreign => None,
    });

    let values = values.collect_vec();

    let positions = values
        .iter()
        .enumerate()
        .map(|(position, (identity, _))| (*identity, position))
        .collect::<FxHashMap<_, _>>();

    let mut dependencies = vec![Vec::new(); values.len()];
    for (position, (_, initializer)) in values.iter().enumerate() {
        // Delayed superclass accessors still belong to the dictionary knot even though creating
        // their closures does not eagerly initialize the dictionaries they reference.
        let globals = function_globals_recursive(module, *initializer, &mut FxHashSet::default());
        for global in globals {
            if let Some(dependency) = positions.get(&global) {
                dependencies[position].push(*dependency);
            }
        }
    }

    let identities = values.into_iter().map(|(identity, _)| identity);
    let identities = identities.collect();

    (identities, dependencies)
}

fn function_globals_recursive(
    module: &ssa::tree::Module,
    function: FunctionId,
    visited: &mut FxHashSet<FunctionId>,
) -> FxHashSet<GlobalIdentity> {
    if !visited.insert(function) {
        return FxHashSet::default();
    }
    let mut globals = FxHashSet::default();
    for block in module.storage[function].blocks.iter().copied() {
        for instruction in &module.storage[block].instructions {
            match instruction {
                Instruction::Assign { value, .. } => match value {
                    InstructionValue::Global { global } => {
                        globals.insert(global.identity);
                    }
                    InstructionValue::Closure { function, .. } => {
                        globals.extend(function_globals_recursive(module, *function, visited));
                    }
                    _ => {}
                },
                Instruction::RecursiveClosures { bindings } => {
                    for binding in bindings.iter() {
                        globals.extend(function_globals_recursive(
                            module,
                            binding.function,
                            visited,
                        ));
                    }
                }
                Instruction::RecursiveLazyInitializers { bindings } => {
                    for binding in bindings.iter() {
                        globals.extend(function_globals_recursive(
                            module,
                            binding.initializer,
                            visited,
                        ));
                    }
                }
            }
        }
    }
    globals
}

fn reaches_initializer(
    start: usize,
    current: usize,
    dependencies: &[Vec<usize>],
    visited: &mut FxHashSet<usize>,
) -> bool {
    for dependency in dependencies[current].iter().copied() {
        if dependency == start {
            return true;
        }
        if visited.insert(dependency)
            && reaches_initializer(start, dependency, dependencies, visited)
        {
            return true;
        }
    }
    false
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
                Instruction::RecursiveLazyInitializers { bindings } => {
                    values.extend(bindings.iter().map(|binding| binding.accessor));
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
        InstructionValue::Force { accessor } => vec![*accessor],
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
                Instruction::RecursiveLazyInitializers { bindings } => {
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

        let candidates = block.instructions.iter().filter_map(|instruction| {
            let Instruction::Assign { result, .. } = instruction else {
                return None;
            };
            (function_uses.get(result) == Some(&1) && local_uses.contains(result))
                .then_some(*result)
        });
        let candidates = candidates.collect_vec();
        // A substitution is valid exactly when JavaScript evaluates the same observable operations
        // in the same order. Trying candidates from consumers toward definitions lets complete
        // expression trees form without making calls, allocations, or accesses globally movable.
        let baseline = block_evaluation_trace(block, &FxHashSet::default())
            .expect("invariant violated: materialized JavaScript block has an invalid trace");
        let mut values = FxHashSet::default();
        // Every changing pass accepts at least one candidate permanently, so one pass per
        // candidate plus a final convergence check is sufficient.
        for _ in 0..=candidates.len() {
            let mut changed = false;
            for candidate in candidates.iter().rev().copied() {
                if values.contains(&candidate) {
                    continue;
                }
                let mut proposed = values.clone();
                proposed.insert(candidate);
                if block_evaluation_trace(block, &proposed).as_ref() == Some(&baseline) {
                    values = proposed;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        inlineable.insert(block_id, values);
    }
    inlineable
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EvaluationOperation {
    value: ValueId,
    stage: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvaluationContext {
    Eager,
    ConditionalOrLazy,
}

struct EvaluationTracer<'b> {
    definitions: FxHashMap<ValueId, &'b InstructionValue>,
    inlineable: &'b FxHashSet<ValueId>,
    operations: Vec<EvaluationOperation>,
    valid: bool,
}

impl<'b> EvaluationTracer<'b> {
    fn new(block: &'b Block, inlineable: &'b FxHashSet<ValueId>) -> EvaluationTracer<'b> {
        let definitions = block.instructions.iter().filter_map(|instruction| {
            let Instruction::Assign { result, value } = instruction else {
                return None;
            };
            Some((*result, value))
        });
        let definitions = definitions.collect();
        EvaluationTracer { definitions, inlineable, operations: Vec::new(), valid: true }
    }

    fn expression(&mut self, value: ValueId, context: EvaluationContext) {
        if !self.inlineable.contains(&value) {
            return;
        }
        let Some(definition) = self.definitions.get(&value).copied() else {
            return;
        };
        let operation_count = self.operations.len();
        self.instruction(value, definition);
        if matches!(context, EvaluationContext::ConditionalOrLazy)
            && self.operations.len() != operation_count
        {
            self.valid = false;
        }
    }

    fn instruction(&mut self, result: ValueId, value: &InstructionValue) {
        // Operand visits follow the order used by instruction_expression. Operations mark the
        // points at which the generated JavaScript can call, allocate, access, or throw.
        match value {
            InstructionValue::Literal { .. }
            | InstructionValue::Constructor { .. }
            | InstructionValue::Global { .. } => {}
            InstructionValue::Array { elements } => {
                for element in elements.iter().copied() {
                    self.expression(element, EvaluationContext::Eager);
                }
                self.operation(result, 0);
            }
            InstructionValue::Record { fields } => {
                for field in fields.iter() {
                    self.expression(field.value, EvaluationContext::Eager);
                }
                self.operation(result, 0);
            }
            InstructionValue::RecordUpdate { record, updates } => {
                if record_updates_have_branches(updates) {
                    self.expression(*record, EvaluationContext::ConditionalOrLazy);
                    self.record_updates(updates, EvaluationContext::ConditionalOrLazy);
                    self.operation(result, 0);
                } else {
                    self.expression(*record, EvaluationContext::Eager);
                    self.operation(result, 0);
                    self.record_updates(updates, EvaluationContext::Eager);
                    self.operation(result, 1);
                }
            }
            InstructionValue::Project { record, .. } => {
                self.expression(*record, EvaluationContext::Eager);
                self.operation(result, 0);
            }
            InstructionValue::Force { accessor } => {
                self.expression(*accessor, EvaluationContext::Eager);
                self.operation(result, 0);
            }
            InstructionValue::Closure { captures, .. } => {
                for capture in captures.iter().copied() {
                    self.expression(capture, EvaluationContext::Eager);
                }
                if !captures.is_empty() {
                    self.operation(result, 0);
                }
            }
            InstructionValue::Call { calling_convention, function, arguments } => {
                self.expression(*function, EvaluationContext::Eager);
                match calling_convention {
                    CallingConvention::Initializer => {
                        for argument in arguments.iter().copied() {
                            self.expression(argument, EvaluationContext::Eager);
                        }
                        self.operation(result, 0);
                    }
                    CallingConvention::Source | CallingConvention::Effect => {
                        for (stage, argument) in arguments.iter().copied().enumerate() {
                            self.expression(argument, EvaluationContext::Eager);
                            self.operation(result, stage);
                        }
                    }
                }
            }
            InstructionValue::Test { value, test } => {
                let context = match test {
                    PatternTest::Literal { .. } => EvaluationContext::Eager,
                    PatternTest::ArrayLength { .. } | PatternTest::Constructor { .. } => {
                        EvaluationContext::ConditionalOrLazy
                    }
                };
                self.expression(*value, context);
                self.operation(result, 0);
            }
            InstructionValue::Extract { value, .. } => {
                self.expression(*value, EvaluationContext::Eager);
                self.operation(result, 0);
            }
            InstructionValue::EffectPure { value } => {
                self.expression(*value, EvaluationContext::ConditionalOrLazy);
                self.operation(result, 0);
            }
            InstructionValue::EffectBind { action, continuation } => {
                self.expression(*action, EvaluationContext::ConditionalOrLazy);
                self.expression(*continuation, EvaluationContext::ConditionalOrLazy);
                self.operation(result, 0);
            }
            InstructionValue::SynthesizedEvidence { .. } | InstructionValue::TrivialEvidence => {
                self.operation(result, 0)
            }
        }
    }

    fn record_updates(&mut self, updates: &[RecordUpdate], context: EvaluationContext) {
        for update in updates {
            match update {
                RecordUpdate::Leaf { value, .. } => {
                    self.expression(*value, context);
                }
                RecordUpdate::Branch { updates, .. } => self.record_updates(updates, context),
            }
        }
    }

    fn operation(&mut self, value: ValueId, stage: usize) {
        self.operations.push(EvaluationOperation { value, stage });
    }
}

fn block_evaluation_trace(
    block: &Block,
    inlineable: &FxHashSet<ValueId>,
) -> Option<Vec<EvaluationOperation>> {
    let mut tracer = EvaluationTracer::new(block, inlineable);
    for instruction in &block.instructions {
        match instruction {
            Instruction::Assign { result, value } if !inlineable.contains(result) => {
                tracer.instruction(*result, value);
            }
            Instruction::Assign { .. } => {}
            Instruction::RecursiveClosures { bindings } => {
                for binding in bindings.iter() {
                    tracer.operation(binding.result, 0);
                }
            }
            Instruction::RecursiveLazyInitializers { bindings } => {
                for binding in bindings.iter() {
                    tracer.operation(binding.accessor, 0);
                }
            }
        }
    }
    match &block.terminator {
        Terminator::Return { value } => tracer.expression(*value, EvaluationContext::Eager),
        Terminator::Jump { target } => {
            for argument in target.arguments.iter().copied() {
                tracer.expression(argument, EvaluationContext::Eager);
            }
        }
        Terminator::Branch { condition, then_target, else_target } => {
            tracer.expression(*condition, EvaluationContext::Eager);
            for argument in then_target.arguments.iter().chain(else_target.arguments.iter()) {
                tracer.expression(*argument, EvaluationContext::ConditionalOrLazy);
            }
        }
        Terminator::Fail { .. } | Terminator::Unreachable => {}
    }
    tracer.valid.then_some(tracer.operations)
}

fn record_updates_have_branches(updates: &[RecordUpdate]) -> bool {
    updates.iter().any(|update| match update {
        RecordUpdate::Leaf { .. } => false,
        RecordUpdate::Branch { .. } => true,
    })
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
            | InstructionValue::Closure { .. }
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
