//! Stable JS-like control-flow rendering for SSA snapshots.

use itertools::Itertools;
use pretty::{Arena, DocAllocator, DocBuilder};
use rustc_hash::FxHashSet;

use crate::tree::{
    Block, BlockTarget, CallingConvention, Declaration, DeclarationKind, Failure, Field, Function,
    Global, GlobalIdentity, IndirectModuleExports, Instruction, InstructionValue, LazyInitializer,
    Literal, Module, PatternTest, Projection, RecordUpdate, RecursiveClosure, ReflectableEvidence,
    ReflectableOrdering, SynthesizedEvidence, Terminator, ValueId,
};

type Doc<'a> = DocBuilder<'a, Arena<'a>, ()>;

const DEFAULT_WIDTH: usize = 100;

pub fn render(module: &Module) -> String {
    let arena = Arena::new();
    let printer = Printer { arena: &arena, module };
    let root_functions =
        module.declarations.iter().filter_map(|declaration| match declaration.kind {
            DeclarationKind::Function { function } => Some(function),
            DeclarationKind::Value { initializer } => Some(initializer),
            DeclarationKind::Constructor { .. } | DeclarationKind::Foreign => None,
        });
    let root_functions = root_functions.collect::<FxHashSet<_>>();

    let declarations =
        module.declarations.iter().map(|declaration| printer.declaration(declaration));
    let nested_functions = module
        .storage
        .functions()
        .filter(|(function, _)| !root_functions.contains(function))
        .map(|(_, function)| printer.nested_function(function));
    let indirect = module.surface.indirect.iter().map(|exports| printer.indirect_exports(exports));
    let documents = indirect.chain(declarations).chain(nested_functions);
    let separator = arena.hardline().append(arena.hardline());
    let document = arena.intersperse(documents, separator);

    let mut output = String::new();
    document
        .render_fmt(DEFAULT_WIDTH, &mut output)
        .expect("critical failure: failed to render SSA control-flow graph");
    output
}

struct Printer<'a, 'm> {
    arena: &'a Arena<'a>,
    module: &'m Module,
}

impl<'a> Printer<'a, '_> {
    fn indirect_exports(&self, exports: &IndirectModuleExports) -> Doc<'a> {
        let dependency = self
            .module
            .dependencies
            .iter()
            .find(|dependency| dependency.file_id == exports.file_id)
            .expect("invariant violated: indirect exports have no module dependency");
        let globals =
            exports.globals.iter().map(|global| self.arena.text(global.item_name.to_string()));
        let globals = self.arena.intersperse(globals, ", ");
        self.arena
            .text(format!("@export {} {{ ", dependency.module_name))
            .append(globals)
            .append(" }")
    }

    fn declaration(&self, declaration: &Declaration) -> Doc<'a> {
        let export = if declaration.exported { "@export " } else { "" };
        let instance = if matches!(declaration.global.identity, GlobalIdentity::Instance { .. }) {
            "instance "
        } else {
            ""
        };
        let recursion = declaration
            .recursive_group
            .map(|group| format!("recursive[{}] ", group.index))
            .unwrap_or_default();
        match declaration.kind {
            DeclarationKind::Function { function } => {
                let function = &self.module.storage[function];
                let parameters = self.values(&function.parameters);
                let header = self
                    .arena
                    .text(format!(
                        "{export}{instance}{recursion}function {}",
                        declaration.global.item_name
                    ))
                    .append(self.parenthesized(parameters));
                self.function_body(header, function)
            }
            DeclarationKind::Value { initializer } => {
                let function = &self.module.storage[initializer];
                let header = self.arena.text(format!(
                    "{export}{instance}{recursion}const {} = initialize",
                    declaration.global.item_name
                ));
                self.function_body(header, function)
            }
            DeclarationKind::Constructor { arity } => self.arena.text(format!(
                "{export}{instance}{recursion}constructor {}/{arity};",
                declaration.global.item_name
            )),
            DeclarationKind::Foreign => self.arena.text(format!(
                "{export}{instance}{recursion}foreign const {};",
                declaration.global.item_name
            )),
        }
    }

    fn nested_function(&self, function: &Function) -> Doc<'a> {
        let convention = match function.calling_convention {
            CallingConvention::Initializer => "initializer",
            CallingConvention::Source => "closure",
            CallingConvention::Effect => "effect closure",
        };
        let captures = self.values(&function.captures);
        let captures = self.bracketed(captures);
        let parameters = self.values(&function.parameters);
        let parameters = self.parenthesized(parameters);
        let header = self
            .arena
            .text(format!("{convention} {}", function.name))
            .append(captures)
            .append(parameters);
        self.function_body(header, function)
    }

    fn function_body(&self, header: Doc<'a>, function: &Function) -> Doc<'a> {
        let blocks = function.blocks.iter().map(|block| self.block(&self.module.storage[*block]));
        let blocks = self.arena.intersperse(blocks, self.arena.hardline());
        let blocks = self.arena.hardline().append(blocks).nest(2);
        header.append(" {").append(blocks).append(self.arena.hardline()).append("}")
    }

    fn block(&self, block: &Block) -> Doc<'a> {
        let parameters = self.values(&block.parameters);
        let header = self.arena.text(block.name.to_string()).append(self.parenthesized(parameters));
        let instructions =
            block.instructions.iter().map(|instruction| self.instruction(instruction));
        let instructions = instructions.chain(std::iter::once(self.terminator(&block.terminator)));
        let instructions = self.arena.intersperse(instructions, self.arena.hardline());
        let instructions = self.arena.hardline().append(instructions).nest(2);
        header.append(" {").append(instructions).append(self.arena.hardline()).append("}")
    }

    fn instruction(&self, instruction: &Instruction) -> Doc<'a> {
        match instruction {
            Instruction::Assign { result, value } => {
                let result = self.value(*result);
                let value = self.instruction_value(value);
                self.arena.text("const ").append(result).append(" = ").append(value).append(";")
            }
            Instruction::RecursiveClosures { bindings } => {
                let results = bindings.iter().map(|binding| self.value(binding.result));
                let results = self.bracketed(results);
                let bindings = bindings.iter().map(|binding| self.recursive_closure(binding));
                let bindings = self.braced(bindings);
                self.arena
                    .text("const ")
                    .append(results)
                    .append(" = closures.recursive(")
                    .append(bindings)
                    .append(");")
            }
            Instruction::RecursiveLazyInitializers { bindings } => {
                let accessors = bindings.iter().map(|binding| self.value(binding.accessor));
                let accessors = self.bracketed(accessors);
                let bindings = bindings.iter().map(|binding| self.lazy_initializer(binding));
                let bindings = self.braced(bindings);
                self.arena
                    .text("const ")
                    .append(accessors)
                    .append(" = lazy.recursive(")
                    .append(bindings)
                    .append(");")
            }
        }
    }

    fn recursive_closure(&self, closure: &RecursiveClosure) -> Doc<'a> {
        let name = self.value(closure.result);
        let function = &self.module.storage[closure.function];
        let captures = closure.captures.iter().map(|capture| self.value(*capture));
        let captures = self.arguments(captures);
        name.append(": closure(")
            .append(self.arena.text(function.name.to_string()))
            .append(captures)
            .append(")")
    }

    fn lazy_initializer(&self, binding: &LazyInitializer) -> Doc<'a> {
        let function = &self.module.storage[binding.initializer];
        let captures = binding.captures.iter().map(|capture| self.value(*capture));
        let captures = self.arguments(captures);
        self.arena
            .text(format!("{:?}: initializer(", binding.name))
            .append(self.arena.text(function.name.to_string()))
            .append(captures)
            .append(")")
    }

    fn instruction_value(&self, value: &InstructionValue) -> Doc<'a> {
        match value {
            InstructionValue::Literal { literal } => self.literal(literal),
            InstructionValue::Array { elements } => {
                let elements = elements.iter().map(|element| self.value(*element));
                self.bracketed(elements)
            }
            InstructionValue::Record { fields } => {
                let fields = fields.iter().map(|field| {
                    let field_name = self.field(&field.field);
                    let value = self.value(field.value);
                    self.arena.text(format!("{field_name}: ")).append(value)
                });
                self.braced(fields)
            }
            InstructionValue::RecordUpdate { record, updates } => {
                let record = self.value(*record);
                let updates = updates.iter().map(|update| self.record_update(update));
                let updates = self.braced(updates);
                self.arena
                    .text("record.update(")
                    .append(record)
                    .append(", ")
                    .append(updates)
                    .append(")")
            }
            InstructionValue::Project { record, field } => {
                let record = self.value(*record);
                self.project_field(record, field)
            }
            InstructionValue::Constructor { global } => {
                self.arena.text("constructor(").append(self.global(global)).append(")")
            }
            InstructionValue::Global { global } => {
                self.arena.text("global(").append(self.global(global)).append(")")
            }
            InstructionValue::Force { accessor } => {
                self.arena.text("force(").append(self.value(*accessor)).append(")")
            }
            InstructionValue::Closure { function, captures } => {
                let function = &self.module.storage[*function];
                let captures = captures.iter().map(|capture| self.value(*capture));
                let captures = self.arguments(captures);
                self.arena
                    .text("closure(")
                    .append(self.arena.text(function.name.to_string()))
                    .append(captures)
                    .append(")")
            }
            InstructionValue::Call { calling_convention, function, arguments } => {
                let convention = match calling_convention {
                    CallingConvention::Initializer => "initializer.call",
                    CallingConvention::Source => "source.call",
                    CallingConvention::Effect => "effect.call",
                };
                let function = self.value(*function);
                let arguments = arguments.iter().map(|argument| self.value(*argument));
                let arguments = self.arguments(arguments);
                self.arena
                    .text(format!("{convention}("))
                    .append(function)
                    .append(arguments)
                    .append(")")
            }
            InstructionValue::Test { value, test } => self.pattern_test(*value, test),
            InstructionValue::Extract { value, projection } => self.projection(*value, projection),
            InstructionValue::EffectPure { value } => {
                self.arena.text("effect.pure(").append(self.value(*value)).append(")")
            }
            InstructionValue::EffectBind { action, continuation } => self
                .arena
                .text("effect.bind(")
                .append(self.value(*action))
                .append(", ")
                .append(self.value(*continuation))
                .append(")"),
            InstructionValue::SynthesizedEvidence { evidence } => {
                self.synthesized_evidence(evidence)
            }
            InstructionValue::TrivialEvidence => self.arena.text("evidence.trivial"),
        }
    }

    fn pattern_test(&self, value: ValueId, test: &PatternTest) -> Doc<'a> {
        let value = self.value(value);
        match test {
            PatternTest::Literal { literal } => self
                .arena
                .text("pattern.literal(")
                .append(value)
                .append(", ")
                .append(self.literal(literal))
                .append(")"),
            PatternTest::ArrayLength { length } => {
                self.arena.text("pattern.array(").append(value).append(format!(", {length})"))
            }
            PatternTest::Constructor { global } => self
                .arena
                .text("pattern.constructor(")
                .append(value)
                .append(", ")
                .append(self.global(global))
                .append(")"),
        }
    }

    fn projection(&self, value: ValueId, projection: &Projection) -> Doc<'a> {
        let value = self.value(value);
        match projection {
            Projection::ArrayElement { index } => value.append(format!("[{index}]")),
            Projection::ConstructorArgument { constructor, index } => self
                .arena
                .text("pattern.argument(")
                .append(value)
                .append(", ")
                .append(self.global(constructor))
                .append(format!(", {index})")),
        }
    }

    fn record_update(&self, update: &RecordUpdate) -> Doc<'a> {
        match update {
            RecordUpdate::Leaf { field, value } => {
                let field = self.field(field);
                self.arena.text(format!("{field}: ")).append(self.value(*value))
            }
            RecordUpdate::Branch { field, updates } => {
                let field = self.field(field);
                let updates = updates.iter().map(|update| self.record_update(update));
                self.arena.text(format!("{field}: ")).append(self.braced(updates))
            }
        }
    }

    fn terminator(&self, terminator: &Terminator) -> Doc<'a> {
        match terminator {
            Terminator::Return { value } => {
                self.arena.text("return ").append(self.value(*value)).append(";")
            }
            Terminator::Jump { target } => self.block_target(target),
            Terminator::Branch { condition, then_target, else_target } => {
                let then_target = self.block_target(then_target);
                let then_target = self.arena.hardline().append(then_target).nest(2);
                let else_target = self.block_target(else_target);
                let else_target = self.arena.hardline().append(else_target).nest(2);
                self.arena
                    .text("if (")
                    .append(self.value(*condition))
                    .append(") {")
                    .append(then_target)
                    .append(self.arena.hardline())
                    .append("} else {")
                    .append(else_target)
                    .append(self.arena.hardline())
                    .append("}")
            }
            Terminator::Fail { failure } => match failure {
                Failure::PatternMatch => self.arena.text("throw patternMatchFailure();"),
            },
            Terminator::Unreachable => self.arena.text("throw unreachable();"),
        }
    }

    fn block_target(&self, target: &BlockTarget) -> Doc<'a> {
        let block = &self.module.storage[target.block];
        let arguments = target.arguments.iter().map(|argument| self.value(*argument));
        self.arena.text(block.name.to_string()).append(self.parenthesized(arguments)).append(";")
    }

    fn synthesized_evidence(&self, evidence: &SynthesizedEvidence) -> Doc<'a> {
        match evidence {
            SynthesizedEvidence::IsSymbol { symbol } => {
                self.arena.text(format!("evidence.symbol({symbol:?})"))
            }
            SynthesizedEvidence::Reflectable { evidence } => {
                let evidence = match evidence {
                    ReflectableEvidence::Integer { value } => value.to_string(),
                    ReflectableEvidence::String { value } => format!("{value:?}"),
                    ReflectableEvidence::Boolean { value } => value.to_string(),
                    ReflectableEvidence::Ordering { ordering } => match ordering {
                        ReflectableOrdering::Less => "LT".into(),
                        ReflectableOrdering::Equal => "EQ".into(),
                        ReflectableOrdering::Greater => "GT".into(),
                    },
                };
                self.arena.text(format!("evidence.reflectable({evidence})"))
            }
        }
    }

    fn literal(&self, literal: &Literal) -> Doc<'a> {
        let literal = match literal {
            Literal::String { value } => format!("{value:?}"),
            Literal::Char { value } => format!("{value:?}"),
            Literal::Boolean { value } => value.to_string(),
            Literal::Integer { value } => value.to_string(),
            Literal::Number { value } => value.to_string(),
        };
        self.arena.text(literal)
    }

    fn values(&self, values: &[ValueId]) -> Vec<Doc<'a>> {
        let values = values.iter().map(|value| self.value(*value));
        values.collect_vec()
    }

    fn value(&self, value: ValueId) -> Doc<'a> {
        self.arena.text(self.module.storage[value].name.to_string())
    }

    fn global(&self, global: &Global) -> Doc<'a> {
        self.arena.text(global.item_name.to_string())
    }

    fn field(&self, field: &Field) -> String {
        let name = field.name.as_str();
        let is_valid_identifier = field_is_valid_identifier(name);
        if is_valid_identifier { name.to_string() } else { format!("{name:?}") }
    }

    fn project_field(&self, record: Doc<'a>, field: &Field) -> Doc<'a> {
        let name = field.name.as_str();
        if field_is_valid_identifier(name) {
            record.append(self.arena.text(format!(".{name}")))
        } else {
            record.append(self.arena.text(format!("[{name:?}]")))
        }
    }

    fn arguments<I>(&self, arguments: I) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        let arguments = arguments
            .into_iter()
            .map(|argument| self.arena.text(",").append(self.arena.line()).append(argument));
        self.arena.concat(arguments).group()
    }

    fn parenthesized<I>(&self, documents: I) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        self.delimited("(", documents, ")")
    }

    fn bracketed<I>(&self, documents: I) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        self.delimited("[", documents, "]")
    }

    fn braced<I>(&self, documents: I) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        let separator = self.arena.text(",").append(self.arena.line());
        let documents = self.arena.intersperse(documents, separator);
        let documents = self.arena.line().append(documents).nest(2);
        let closing_line = self.arena.line();
        self.arena.text("{").append(documents).append(closing_line).append("}").group()
    }

    fn delimited<I>(&self, open: &'static str, documents: I, close: &'static str) -> Doc<'a>
    where
        I: IntoIterator<Item = Doc<'a>>,
    {
        let separator = self.arena.text(",").append(self.arena.line());
        let documents = self.arena.intersperse(documents, separator);
        let documents = self.arena.softline_().append(documents).nest(2);
        let closing_line = self.arena.softline_();
        self.arena.text(open).append(documents).append(closing_line).append(close).group()
    }
}

fn field_is_valid_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(initial) = characters.next() else {
        return false;
    };
    let valid_initial = initial.is_ascii_alphabetic() || initial == '_' || initial == '$';
    let valid_subsequent = characters
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '$');
    valid_initial && valid_subsequent
}
