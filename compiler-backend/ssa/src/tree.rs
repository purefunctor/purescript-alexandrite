//! Arena-allocated A-normal instructions and control-flow graphs.

use std::ops::Index;
use std::sync::Arc;

use files::FileId;
use indexing::{DeriveId, InstanceId, TermItemId, TypeItemId};
use la_arena::{Arena, Idx};
use lowering::TypeId as SourceTypeId;
use smol_str::SmolStr;

pub type FunctionId = Idx<Function>;
pub type BlockId = Idx<Block>;
pub type ValueId = Idx<Value>;

#[derive(Debug, PartialEq, Eq)]
pub struct Module {
    pub file_id: FileId,
    pub name: SmolStr,
    pub dependencies: Arc<[ModuleDependency]>,
    pub surface: ModuleSurface,
    pub declarations: Arc<[Declaration]>,
    pub storage: Storage,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ModuleSurface {
    pub indirect: Arc<[IndirectModuleExports]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IndirectModuleExports {
    pub file_id: FileId,
    pub globals: Arc<[Global]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ModuleDependency {
    pub file_id: FileId,
    pub module_name: SmolStr,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Storage {
    functions: Arena<Function>,
    blocks: Arena<Block>,
    values: Arena<Value>,
}

impl Storage {
    pub fn allocate_function(&mut self, function: Function) -> FunctionId {
        self.functions.alloc(function)
    }

    pub fn allocate_block(&mut self, block: Block) -> BlockId {
        self.blocks.alloc(block)
    }

    pub fn allocate_value(&mut self, value: Value) -> ValueId {
        self.values.alloc(value)
    }

    pub fn functions(&self) -> impl Iterator<Item = (FunctionId, &Function)> {
        self.functions.iter()
    }

    pub fn blocks(&self) -> impl Iterator<Item = (BlockId, &Block)> {
        self.blocks.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = (ValueId, &Value)> {
        self.values.iter()
    }

    pub(crate) fn append_instruction(&mut self, block: BlockId, instruction: Instruction) {
        self.blocks[block].instructions.push(instruction);
    }

    pub(crate) fn set_terminator(&mut self, block: BlockId, terminator: Terminator) {
        self.blocks[block].terminator = terminator;
    }
}

impl Index<FunctionId> for Storage {
    type Output = Function;

    fn index(&self, index: FunctionId) -> &Function {
        &self.functions[index]
    }
}

impl Index<BlockId> for Storage {
    type Output = Block;

    fn index(&self, index: BlockId) -> &Block {
        &self.blocks[index]
    }
}

impl Index<ValueId> for Storage {
    type Output = Value;

    fn index(&self, index: ValueId) -> &Value {
        &self.values[index]
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Declaration {
    pub global: Global,
    pub exported: bool,
    pub recursive_group: Option<RecursiveGroupId>,
    pub kind: DeclarationKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DeclarationKind {
    Function { function: FunctionId },
    Value { initializer: FunctionId },
    Constructor { arity: usize },
    Foreign,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Global {
    pub identity: GlobalIdentity,
    pub item_name: SmolStr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlobalIdentity {
    Term { file_id: FileId, term_id: TermItemId },
    Instance { identity: InstanceIdentity },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstanceIdentity {
    Declared { file_id: FileId, instance_id: InstanceId },
    Derived { file_id: FileId, derive_id: DeriveId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecursiveGroupId {
    pub index: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    pub name: SmolStr,
    pub calling_convention: CallingConvention,
    pub captures: Arc<[ValueId]>,
    pub parameters: Arc<[ValueId]>,
    pub entry: BlockId,
    pub blocks: Arc<[BlockId]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    Initializer,
    Source,
    Effect,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    pub name: SmolStr,
    pub parameters: Arc<[ValueId]>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub name: SmolStr,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    Assign { result: ValueId, value: InstructionValue },
    RecursiveClosures { bindings: Arc<[RecursiveClosure]> },
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecursiveClosure {
    pub result: ValueId,
    pub function: FunctionId,
    pub captures: Arc<[ValueId]>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InstructionValue {
    Literal { literal: Literal },
    Array { elements: Arc<[ValueId]> },
    Record { fields: Arc<[RecordField]> },
    RecordUpdate { record: ValueId, updates: Arc<[RecordUpdate]> },
    Project { record: ValueId, field: Field },
    Constructor { global: Global },
    Global { global: Global },
    Closure { function: FunctionId, captures: Arc<[ValueId]> },
    Call { calling_convention: CallingConvention, function: ValueId, arguments: Arc<[ValueId]> },
    Test { value: ValueId, test: PatternTest },
    Extract { value: ValueId, projection: Projection },
    EffectPure { value: ValueId },
    EffectBind { action: ValueId, continuation: ValueId },
    SynthesizedEvidence { evidence: SynthesizedEvidence },
    TrivialEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    String { value: SmolStr },
    Char { value: char },
    Boolean { value: bool },
    Integer { value: i32 },
    Number { value: SmolStr },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub identity: FieldIdentity,
    pub name: SmolStr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldIdentity {
    Label { label: SmolStr },
    Member { file_id: FileId, term_id: TermItemId },
    Superclass { identity: SuperclassIdentity },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuperclassIdentity {
    pub file_id: FileId,
    pub class: TypeItemId,
    pub source: SourceTypeId,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecordField {
    pub field: Field,
    pub value: ValueId,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RecordUpdate {
    Leaf { field: Field, value: ValueId },
    Branch { field: Field, updates: Arc<[RecordUpdate]> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternTest {
    Literal { literal: Literal },
    ArrayLength { length: usize },
    Constructor { global: Global },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Projection {
    ArrayElement { index: usize },
    ConstructorArgument { constructor: Global, index: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Terminator {
    Return { value: ValueId },
    Jump { target: BlockTarget },
    Branch { condition: ValueId, then_target: BlockTarget, else_target: BlockTarget },
    Fail { failure: Failure },
    Unreachable,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlockTarget {
    pub block: BlockId,
    pub arguments: Arc<[ValueId]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    PatternMatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynthesizedEvidence {
    IsSymbol { symbol: SmolStr },
    Reflectable { evidence: ReflectableEvidence },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectableEvidence {
    Integer { value: i32 },
    String { value: SmolStr },
    Boolean { value: bool },
    Ordering { ordering: ReflectableOrdering },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectableOrdering {
    Less,
    Equal,
    Greater,
}
