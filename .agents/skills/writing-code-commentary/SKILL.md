---
name: writing-code-commentary
description: "Writes and reviews Alexandrite compiler comments, algorithm traces, and documentation examples. Use when documenting non-obvious compiler transformations, revising inline commentary, or applying documentation-focused review nits."
---

# Writing Alexandrite Code Commentary

Make difficult compiler code reviewable without narrating code that already explains itself.

Use comments to expose an invariant, derivation, or non-obvious reason. For type-directed generation and other staged transformations, state the general rule once and trace representative examples beside the implementation steps that transform them.

## Establish the local style

Before writing comments:

1. Read the complete function or branch being documented.
2. Identify the fact a reader cannot recover cheaply from the code:
   - the target or invariant driving the algorithm;
   - why a transformation is valid;
   - why an apparently redundant operation is required;
   - how types or expressions evolve through staged construction;
   - where elaboration, evidence, or another compiler subsystem completes the work.
3. Inspect nearby comments and one analogous implementation elsewhere in the owning crate. Prefer established Alexandrite forms such as:
   - phase or branch rule followed by a type-state trace;
   - declaration or expression evolving beside each construction step;
   - match-arm rationale followed by concrete before/after equations;
   - algorithm responsibilities followed by a safety invariant.
4. Decide whether comments are needed at all. Do not comment straightforward control flow, repeat names, or turn implementation details into prose.

Useful precedents include `source/terms/form_ado.rs`, the `Tagged` declaration trace in `source/type_items.rs`, `match_union` in `core/constraint/compiler/prim_row.rs`, and the promotion trace in `core/unification.rs`. Follow the closest current precedent rather than reproducing one mechanically.

## Write from the goal, not from execution order

For type-directed transformations, begin with the target type or required expression as the goal. Explain why the next construct is needed and how it reduces the remaining goal.

Prefer:

```rust
// The target field must have type `r -> b`, so bind its argument. Applying
// the source function produces `a`; the result operation then produces the
// required `b`.
//
//   \argument -> function (source argument) :: r -> b
```

Avoid:

```rust
// The outer invocation creates a lambda, then the recursive invocation maps
// the result, and finally this returns the function.
```

The preferred form explains the obligation and the reason for each step. The rejected form narrates execution, obscures recursion, and uses positional labels whose meaning changes as the call tree deepens.

## Structure staged algorithm commentary

Use this structure when a branch or helper performs a non-obvious derivation.

### 1. State one generalized rule

Keep the branch-level comment independent of any one example:

```rust
// Function types transform arguments contravariantly and results
// covariantly. A missing operation leaves that side unchanged.
```

Say what responsibility distinguishes the branch. For example, a `Map` operation delegates traversal to an existing `Functor` instance; it is not merely another recursive traversal case.

### 2. Introduce concrete source and target types

Name the declaration when referring to its fields. Do not take a shortcut by presenting anonymous record or container fragments without the type that gives them context.

```text
// newtype Inventory a = Inventory (Array { item :: a })
//
// source :: Array { item :: a }
// target :: Array { item :: b }
```

If source and target differ, show both. Do not write a single schematic type and expect the reader to infer the transformation.

### 3. Trace examples beside the corresponding construction steps

Place each intermediate form immediately above the line or block that creates it. Keep the top-level comment about the rule; keep examples near the implementation details they illuminate.

```text
// Inventory
//   transformer :: { item :: a } -> { item :: b }
//   transformer = \element -> element { item = function element.item }
```

Prefer equations, types, and short statements over paragraphs. The trace should let a reviewer compare the documented stage directly with the code beneath it.

### 4. Complete every example in parallel

When using multiple examples, carry each one through every documented stage. Do not introduce two declarations and silently continue with only the simpler one.

Give examples short headings instead of punctuation separators:

```text
// NonEmpty
//   transformer = function
// Inventory
//   transformer = \element -> element { item = function element.item }
```

Each example must justify its space by exposing a distinct case, such as:

- no nested operation;
- recursive argument transformation;
- recursive result transformation;
- both transformations present;
- delegation through an existing instance;
- a record field update;
- an absent parameter that requires an explicit identity function.

Remove examples that merely repeat the same path with different names.

## Describe recursion by semantic role

Do not flatten recursive calls into a linear story. Do not call them "outer", "inner", "nested", or "recursive" when those words do not identify what the invocation transforms.

Name the operation and its input or role instead:

- transformation of the function argument;
- transformation of the function result;
- traversal of the record field;
- transformer passed to `map`;
- first or second transformer passed to `bimap`.

Treat an operation tree or recipe as evidence that a reduction is allowed, not as a source of invented vocabulary.

## Choose examples and terminology conservatively

- Prefer recognizable PureScript declarations and ecosystem idioms.
- Check repository or upstream sources before claiming a term is standard.
- If a concise coined label helps, present it as a local mnemonic rather than established literature.
- Avoid domain-loaded names when they conflict with the concept being explained. For a function value, `f` can be clearer than `program` when “program” already has a Reader or CPS interpretation.
- Use exact phrases. For example, “record field updates” identifies the generated expression more precisely than “records.”

When a reviewer asks where terminology came from, find and cite the specific authoritative source. Do not reverse-justify a convenient name.

## Keep names semantically honest

Review names in code examples and generated output as part of documentation quality.

- Name a value for its current state, not a future or completed transformation. Prefer `element` to `mapped` for the value supplied to a mapping function.
- Give distinct generated binders distinct names when identical names would make a semantic snapshot look captured.
- Avoid one-off naming abstractions and trivial tests whose only assertion restates a formatting helper.
- If clearer commentary reveals a misleading production name, make the smallest rename and update affected snapshots. Do not leave code and commentary using competing vocabularies.

## Be precise about compiler boundaries

Document elaboration mechanics only when they explain why the generated term is correct.

For known polymorphic terms such as `map` or `bimap`, distinguish:

- the fully qualified, constrained type obtained from known terms;
- a fresh unification variable used during specialization;
- the concrete constructor inferred during elaboration;
- subtype or constraint work retained to generate instance evidence.

Do not imply that code generation starts with an already specialized, unconstrained helper. Conversely, omit solver bookkeeping that adds no explanatory value. Once a unification such as `?f := Array` makes specialization clear, do not repeat a redundant solved-evidence line.

Explain apparently redundant generated terms when they satisfy an interface. For example, `bimap` still needs a transformer for a type parameter absent from a field, so generation supplies `identity`; it does not omit the argument.

## Apply documentation review nits

Treat wording nits as correctness feedback when they reveal ambiguity about types, ownership, or algorithm stages.

For each nit:

1. Locate the implementation statement the wording claims to explain.
2. Check that nouns have a visible referent. Replace contextless words such as “function”, “invocation”, or “unchanged” with the actual role, expression, or binder.
3. Check every displayed expression for the correct source and target type.
4. Check that an example does not skip a decomposition step or change abstraction level halfway through.
5. Remove labels, separators, and evidence equations made redundant by headings or nearby context.
6. Read the complete comment sequence after local edits; a sentence can be correct alone while breaking the staged explanation.

## Avoid these failure modes

- Narrative comments that restate control flow.
- A large preamble separated from the code it explains.
- Positional recursion labels such as “outer invocation.”
- Examples presented as a linear execution of a recursive algorithm.
- Anonymous fragments when a concrete declaration provides necessary context.
- Multiple examples that cease to be parallel.
- Invented compiler concepts or uncited claims of standard terminology.
- Names that imply an operation has already happened.
- Solver details that do not motivate generated syntax or evidence.
- Comments that say “what” while leaving the governing “why” unstated.

## Preserve commentary with its code

When extracting or moving an emitter, move its staged documentation with it. Do not leave the algorithm in one module and its guide in another. Re-check relative links, named variants, helper names, and nearby examples after the move.

## Validate proportionally

For comment-only changes:

```bash
just format
git diff --check
```

Review the rendered diff as one continuous explanation.

If documentation-driven naming changes affect generated syntax or snapshots, run the narrow integration tests that own those snapshots and inspect every update. Use `cargo check -p <crate-name> --tests` only when executable Rust changed or the comment edit accompanies code changes requiring it.

## Final review checklist

- Does every comment expose a non-obvious invariant, reason, or transformation?
- Is the general rule stated once?
- Does each staged example sit beside the code that creates that stage?
- Are source and target types explicit and correct?
- Are recursive calls identified by semantic role?
- Do multiple examples remain parallel and cover distinct cases?
- Are terms authoritative or clearly marked as local mnemonics?
- Are names accurate for the value's current state?
- Are elaboration and evidence details included only when they explain correctness?
- Could any prose be replaced by a shorter type, equation, or before/after form?
- Does the commentary remain useful if local implementation details are refactored?
