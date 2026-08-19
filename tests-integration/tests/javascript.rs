use std::fs;
use std::process::Command;
use std::sync::Arc;

use building_types::QueryResult;
use files::FileId;
use indexing::TermItemId;
use la_arena::{Idx, RawIdx};
use ssa::tree::{
    Block, BlockTarget, CallingConvention, Declaration, DeclarationKind, Function, Global,
    GlobalIdentity, Instruction, InstructionValue, Literal, Module, ModuleSurface, Storage,
    Terminator, Value,
};
use tempfile::TempDir;

const EXECUTION_FIXTURE: &str = "fixtures/backend/1787081220_javascript_execution";
const RE_EXPORT_FIXTURE: &str = "fixtures/backend/1787130720_module_re_exports";

#[test]
fn executes_compiled_fixture_with_node() {
    let (engine, files) = tests_integration::load_compiler(EXECUTION_FIXTURE.as_ref());
    let main = engine.module_file("Main").expect("invariant violated: missing Main module");
    let library =
        engine.module_file("Library").expect("invariant violated: missing Library module");
    let directory = TempDir::new().expect("invariant violated: failed to create output directory");

    let modules = tests_integration::fixtures::javascript_modules(&engine, main).unwrap().unwrap();
    modules.write_to(&files, directory.path()).unwrap();
    let main_module = modules.entry();
    assert!(!main_module.source().contains("while (true)"));
    assert!(!main_module.source().contains("switch ("));
    assert!(main_module.source().contains("export const integer = 42 | 0;"));
    assert!(main_module.source().contains("export const updated = (() => {"));
    assert!(!main_module.source().contains("function integer$initialize"));
    let library_module =
        modules.get(library).expect("invariant violated: Library is not a dependency of Main");
    let runner = fixture_runner(&main_module.filename(), &library_module.filename());
    fs::write(directory.path().join("run.mjs"), runner).unwrap();

    run_node(directory.path(), "run.mjs");
}

fn fixture_runner(main_module: &str, library_module: &str) -> String {
    format!(
        r#"import * as Main from "./{main_module}";
import * as Library from "./{library_module}";

let patternFailure = false;
try {{
  Main.partialPattern(Main.None);
}} catch (error) {{
  patternFailure = error.message === "Pattern match failure";
}}

const actual = {{
  integer: Main.integer,
  number: Main.number,
  string: Main.string,
  array: Main.array,
  recordCount: Main.model.count,
  updatedCount: Main.updated.count,
  updatedNested: Main.updated.nested.enabled,
  originalNested: Main.model.nested.enabled,
  hostileProperty: Main.readHostile(Main.model),
  hostileExport: Main["await"],
  protoProperty: Main.readProto(Main.model),
  recordPrototype: Object.getPrototypeOf(Main.model) === Object.prototype,
  closureCapture: Main.capture(42)(0),
  curriedApplication: Main.curried,
  sharedJoinCapture: Main.nestedJoin(true)(true),
  nestedBranch: Main.nestedJoin(false)(true),
  recursion: Main.countdown(5),
  mutualRecursion: Main.isEven(6) && Main.isOdd(5),
  recursivePeerAndFreeCapture: Main.capturedMutual(42)(false),
  constructorTag: Main.pair[0],
  constructorArguments: Main.pair.slice(1),
  constructorPattern: Main.first(Main.pair),
  importedConstructorPattern: Main.unwrapWrapped(Library.Wrapped(34)),
  zeroArgumentConstructorPattern: Main.first(Main.None),
  curriedConstructorPattern: Main.first(Main.Pair(11)(12)),
  crossModuleConstructor: Main.crossModule,
  forwardReference: Main.forwardReference,
  foreignValue: Main.foreignValue,
  effectThunk: Main.effectValue(),
  evidence: Main.evidenceValue,
  patternFailure,
}};

const expected = {{
  integer: 42,
  number: 1.5,
  string: "alexandrite",
  array: [1, 2, 3],
  recordCount: 0,
  updatedCount: 1,
  updatedNested: false,
  originalNested: true,
  hostileProperty: 17,
  hostileExport: 17,
  protoProperty: "data, not a prototype",
  recordPrototype: true,
  closureCapture: 42,
  curriedApplication: 42,
  sharedJoinCapture: 10,
  nestedBranch: 0,
  recursion: 5,
  mutualRecursion: true,
  recursivePeerAndFreeCapture: 42,
  constructorTag: "Pair",
  constructorArguments: [7, 8],
  constructorPattern: 7,
  importedConstructorPattern: 34,
  zeroArgumentConstructorPattern: 0,
  curriedConstructorPattern: 11,
  crossModuleConstructor: 21,
  forwardReference: 13,
  foreignValue: 9,
  effectThunk: 41,
  evidence: 42,
  patternFailure: true,
}};

if (JSON.stringify(actual) !== JSON.stringify(expected)) {{
  throw new Error(`unexpected output\nactual: ${{JSON.stringify(actual)}}\nexpected: ${{JSON.stringify(expected)}}`);
}}
"#
    )
}

#[test]
fn executes_module_re_exports_with_node() {
    let (engine, files) = tests_integration::load_compiler(RE_EXPORT_FIXTURE.as_ref());
    let main = engine.module_file("Main").expect("invariant violated: missing Main module");
    let origin = engine.module_file("Origin").expect("invariant violated: missing Origin module");
    let direct = engine.module_file("Direct").expect("invariant violated: missing Direct module");
    let transitive =
        engine.module_file("Transitive").expect("invariant violated: missing Transitive module");
    let directory = TempDir::new().expect("invariant violated: failed to create output directory");

    let modules = tests_integration::fixtures::javascript_modules(&engine, main).unwrap().unwrap();
    modules.write_to(&files, directory.path()).unwrap();
    let main_module = modules.entry();
    let origin_module =
        modules.get(origin).expect("invariant violated: Origin is not a dependency of Main");
    let direct_module =
        modules.get(direct).expect("invariant violated: Direct is not a dependency of Main");
    let transitive_module = modules
        .get(transitive)
        .expect("invariant violated: Transitive is not a dependency of Main");
    assert!(!direct_module.source().contains("import * as"));
    assert!(!transitive_module.source().contains("import * as"));
    assert!(direct_module.source().contains(
        "export { Just, \"await\", foreignValue, visible } from \"../Origin/index.js\";"
    ));
    assert!(transitive_module.source().contains("export { append } from \"../Direct/index.js\";"));
    assert!(transitive_module.source().contains(
        "export { Just, \"await\", foreignValue, visible } from \"../Origin/index.js\";"
    ));
    assert!(!origin_module.source().contains("\"<>\""));
    assert!(!direct_module.source().contains("\"<>\""));
    assert!(!transitive_module.source().contains("\"<>\""));

    let runner = re_export_runner(
        &main_module.filename(),
        &origin_module.filename(),
        &direct_module.filename(),
        &transitive_module.filename(),
    );
    fs::write(directory.path().join("run.mjs"), runner).unwrap();

    run_node(directory.path(), "run.mjs");
}

fn re_export_runner(
    main_module: &str,
    origin_module: &str,
    direct_module: &str,
    transitive_module: &str,
) -> String {
    format!(
        r#"import * as Main from "./{main_module}";
import * as Origin from "./{origin_module}";
import * as Direct from "./{direct_module}";
import * as Transitive from "./{transitive_module}";

const assertKeys = (namespace, expected, name) => {{
  const actual = Object.keys(namespace).sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {{
    throw new Error(`${{name}} exports ${{JSON.stringify(actual)}}`);
  }}
}};

assertKeys(
  Origin,
  ["Just", "append", "await", "eqOption", "foreignValue", "measureInt", "visible"],
  "Origin",
);
assertKeys(Direct, ["Just", "append", "await", "foreignValue", "visible"], "Direct");
assertKeys(
  Transitive,
  ["Just", "append", "await", "foreignValue", "marker", "visible"],
  "Transitive",
);
assertKeys(
  Main,
  [
    "Just",
    "append",
    "await",
    "constructorValue",
    "foreignResult",
    "foreignValue",
    "hostileResult",
    "localCollision",
    "marker",
    "measured",
    "operatorValue",
    "transitiveMarker",
    "visible",
  ],
  "Main",
);

if (Direct.Just !== Origin.Just || Transitive.Just !== Origin.Just || Main.Just !== Origin.Just) {{
  throw new Error("constructor re-export identity");
}}
if (JSON.stringify(Transitive.Just(42)) !== JSON.stringify(["Just", 42])) {{
  throw new Error("constructor representation");
}}
if (
  Direct.visible !== Origin.visible ||
  Transitive.visible !== Origin.visible ||
  Main.visible !== Origin.visible
) {{
  throw new Error("function re-export identity");
}}
if (
  Direct.append === Origin.append ||
  Transitive.append !== Direct.append ||
  Main.append !== Direct.append
) {{
  throw new Error("local collision");
}}
if (
  Direct.foreignValue !== Origin.foreignValue ||
  Transitive.await !== Origin.await ||
  Main.foreignValue !== Origin.foreignValue ||
  Main.await !== Origin.await
) {{
  throw new Error("foreign or hostile re-export identity");
}}
if (Main.marker !== Transitive.marker) throw new Error("transitive re-export identity");

const actual = {{
  constructorValue: Main.constructorValue,
  operatorValue: Main.operatorValue,
  localCollision: Main.localCollision,
  foreignResult: Main.foreignResult,
  hostileResult: Main.hostileResult,
  measured: Main.measured,
  transitiveMarker: Main.transitiveMarker,
}};
const expected = {{
  constructorValue: ["Just", 42],
  operatorValue: 23,
  localCollision: 99,
  foreignResult: 7,
  hostileResult: 17,
  measured: 41,
  transitiveMarker: 1,
}};
if (JSON.stringify(actual) !== JSON.stringify(expected)) {{
  throw new Error(`unexpected output ${{JSON.stringify(actual)}}`);
}}
"#
    )
}

#[test]
fn executes_effect_instructions_and_cyclic_control_flow_with_node() {
    let file_id = file_id(0);
    let module = Arc::new(manual_ssa_module(file_id));
    let queries = StaticSsa { module };
    let javascript = javascript::convert_module(&queries, file_id).unwrap().unwrap();
    assert!(javascript.source().contains("while (true)"));
    assert!(javascript.source().contains("switch ("));
    assert!(javascript.source().contains("export const effectProgram = (() => {"));

    let directory = TempDir::new().expect("invariant violated: failed to create output directory");
    let output_path = directory.path().join(javascript.filename());
    fs::create_dir_all(output_path.parent().unwrap()).unwrap();
    fs::write(output_path, javascript.source()).unwrap();
    let runner = format!(
        "import * as Main from \"./{}\";\n\
         if (Main.effectProgram() !== 41) throw new Error(\"effect instructions\");\n\
         if (Main.cycle(true) !== 42) throw new Error(\"cyclic CFG\");\n",
        javascript.filename()
    );
    fs::write(directory.path().join("run.mjs"), runner).unwrap();

    run_node(directory.path(), "run.mjs");
}

fn run_node(directory: &std::path::Path, runner: &str) {
    fs::write(directory.join("package.json"), "{\"type\":\"module\"}\n").unwrap();
    let output = Command::new("node").arg(runner).current_dir(directory).output().unwrap();
    assert!(
        output.status.success(),
        "Node failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct StaticSsa {
    module: Arc<Module>,
}

impl ssa::ExternalQueries for StaticSsa {
    fn ssa(&self, _file_id: FileId) -> QueryResult<ssa::ModuleResult<Arc<Module>>> {
        Ok(Ok(Arc::clone(&self.module)))
    }
}

fn manual_ssa_module(file_id: FileId) -> Module {
    let mut storage = Storage::default();

    let continuation_parameter = storage.allocate_value(Value { name: "value".into() });
    let continuation_effect = storage.allocate_value(Value { name: "effect".into() });
    let continuation_block = storage.allocate_block(Block {
        name: "entry".into(),
        parameters: Arc::new([]),
        instructions: vec![Instruction::Assign {
            result: continuation_effect,
            value: InstructionValue::EffectPure { value: continuation_parameter },
        }],
        terminator: Terminator::Return { value: continuation_effect },
    });
    let continuation = storage.allocate_function(Function {
        name: "continuation".into(),
        calling_convention: CallingConvention::Effect,
        captures: Arc::new([]),
        parameters: Arc::new([continuation_parameter]),
        entry: continuation_block,
        blocks: Arc::new([continuation_block]),
    });

    let literal = storage.allocate_value(Value { name: "literal".into() });
    let action = storage.allocate_value(Value { name: "action".into() });
    let continuation_value = storage.allocate_value(Value { name: "continuation".into() });
    let program = storage.allocate_value(Value { name: "program".into() });
    let effect_block = storage.allocate_block(Block {
        name: "entry".into(),
        parameters: Arc::new([]),
        instructions: vec![
            Instruction::Assign {
                result: literal,
                value: InstructionValue::Literal { literal: Literal::Integer { value: 41 } },
            },
            Instruction::Assign {
                result: action,
                value: InstructionValue::EffectPure { value: literal },
            },
            Instruction::Assign {
                result: continuation_value,
                value: InstructionValue::Closure { function: continuation, captures: Arc::new([]) },
            },
            Instruction::Assign {
                result: program,
                value: InstructionValue::EffectBind { action, continuation: continuation_value },
            },
        ],
        terminator: Terminator::Return { value: program },
    });
    let effect_function = storage.allocate_function(Function {
        name: "effectProgram".into(),
        calling_convention: CallingConvention::Initializer,
        captures: Arc::new([]),
        parameters: Arc::new([]),
        entry: effect_block,
        blocks: Arc::new([effect_block]),
    });

    let condition = storage.allocate_value(Value { name: "condition".into() });
    let loop_condition = storage.allocate_value(Value { name: "loopCondition".into() });
    let false_value = storage.allocate_value(Value { name: "falseValue".into() });
    let answer = storage.allocate_value(Value { name: "answer".into() });
    let return_block = storage.allocate_block(Block {
        name: "return".into(),
        parameters: Arc::new([]),
        instructions: vec![],
        terminator: Terminator::Return { value: answer },
    });
    let expected_loop_block = block_id(storage.blocks().count() as u32);
    let loop_block = storage.allocate_block(Block {
        name: "loop".into(),
        parameters: Arc::new([loop_condition]),
        instructions: vec![],
        terminator: Terminator::Branch {
            condition: loop_condition,
            then_target: BlockTarget {
                block: expected_loop_block,
                arguments: Arc::new([false_value]),
            },
            else_target: BlockTarget { block: return_block, arguments: Arc::new([]) },
        },
    });
    assert_eq!(loop_block, expected_loop_block);
    let cycle_entry = storage.allocate_block(Block {
        name: "entry".into(),
        parameters: Arc::new([]),
        instructions: vec![
            Instruction::Assign {
                result: false_value,
                value: InstructionValue::Literal { literal: Literal::Boolean { value: false } },
            },
            Instruction::Assign {
                result: answer,
                value: InstructionValue::Literal { literal: Literal::Integer { value: 42 } },
            },
        ],
        terminator: Terminator::Jump {
            target: BlockTarget { block: loop_block, arguments: Arc::new([condition]) },
        },
    });
    let cycle_function = storage.allocate_function(Function {
        name: "cycle".into(),
        calling_convention: CallingConvention::Source,
        captures: Arc::new([]),
        parameters: Arc::new([condition]),
        entry: cycle_entry,
        blocks: Arc::new([cycle_entry, loop_block, return_block]),
    });
    let effect_global = Global {
        identity: GlobalIdentity::Term { file_id, term_id: term_id(0) },
        item_name: "effectProgram".into(),
    };
    let cycle_global = Global {
        identity: GlobalIdentity::Term { file_id, term_id: term_id(1) },
        item_name: "cycle".into(),
    };

    Module {
        file_id,
        name: "Main".into(),
        dependencies: Arc::new([]),
        surface: ModuleSurface { indirect: Arc::new([]) },
        declarations: Arc::new([
            Declaration {
                global: effect_global,
                exported: true,
                recursive_group: None,
                kind: DeclarationKind::Value { initializer: effect_function },
            },
            Declaration {
                global: cycle_global,
                exported: true,
                recursive_group: None,
                kind: DeclarationKind::Function { function: cycle_function },
            },
        ]),
        storage,
    }
}

fn file_id(index: u32) -> FileId {
    Idx::from_raw(RawIdx::from_u32(index))
}

fn term_id(index: u32) -> TermItemId {
    Idx::from_raw(RawIdx::from_u32(index))
}

fn block_id(index: u32) -> ssa::tree::BlockId {
    Idx::from_raw(RawIdx::from_u32(index))
}
