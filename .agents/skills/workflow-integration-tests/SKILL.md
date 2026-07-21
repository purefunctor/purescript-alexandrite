---
name: workflow-integration-tests
description: "Workflow for adding and updating Alexandrite integration-test fixtures for checking, semantic trees, lowering, resolving, and LSP behavior. Use when creating compiler integration tests, reviewing fixture snapshots, or using `just t <category>`."
---

# Workflow: Alexandrite Integration Tests

Use the command reference at `reference/compiler-scripts.md` for test runner syntax, snapshot workflows, filters, and trace debugging.

**Language:** Fixtures use PureScript syntax, not Haskell.

## Choose the category first

| Category | Alias | Use for | Harness pattern |
|----------|-------|---------|-----------------|
| `checking` | `c` | Type checking, inference, kinds, roles, constraints, diagnostics after checking | `Main.purs` only |
| `semantic` | `s` | Checked semantic tree declarations, typed expressions and binders, and explicit evidence | `Main.purs` only |
| `lowering` | `l` | Lowered core output, binding/equation structure, source-to-core name links | every `.purs` file |
| `resolving` | `r` | Name resolution, imports, exports, qualification, duplicate-name diagnostics | every `.purs` file |
| `lsp` | - | Hover, definition, completion, import edits, source locations in LSP reports | `Main.purs` only |

When a behavior spans phases, test the earliest category that directly owns the behavior. Add a later-phase fixture only if the later report is the clearest way to make the regression reviewable.

## Creating a Test

### 1. Create fixture directory

```bash
just t <category> --create "descriptive name"
```

The CLI picks the next timestamped fixture number, creates the folder under `tests-integration/fixtures/<category>/`, and writes a `Main.purs` template.

Tests are auto-discovered by `build.rs`.

### 2. Write focused PureScript modules

Keep each fixture about one behavior. Use a small `Main.purs` by default, and add supporting modules only when imports, exports, qualification, or cross-module behavior are part of the test.

#### Checking fixtures

Pair explicitly checked and inferred variants when both modes matter:

```purescript
module Main where

-- Checking mode: explicit signature constrains type checker
test :: Array Int -> Int
test [x] = x

-- Inference mode: type checker infers unconstrained
test' [x] = x
```

Name declarations predictably: `test`, `test'`, `test2`, `test2'`, etc. Include only edge cases relevant to the behavior.

#### Semantic fixtures

Write source that exposes the checked semantic structure being tested. Keep fixtures focused on the smallest declaration, expression, binder, or evidence shape that distinguishes the behavior.

#### Lowering fixtures

Write source that exposes the lowered structure being tested. Prefer simple declarations whose snapshot makes binding, equation, or source-link changes obvious.

#### Resolving fixtures

Use descriptive module names when multiple modules participate, such as `Library.purs`, `ReExporter.purs`, and `Main.purs`. Because every `.purs` file in a resolving fixture is snapshotted, review all generated `.snap` files before accepting.

#### LSP fixtures

Use `Main.purs` as the scenario driver. Add supporting modules for imported symbols and completion candidates. LSP snapshots are generated from `Main.purs`; supporting modules influence the report but do not get their own LSP snapshots.

### 3. Run and review

```bash
just t <category> NNN MMM
```

### 4. Accept or reject snapshots

```bash
just t <category> NNN --diff         # Inspect a fixture diff
just t <category> NNN --accept       # Accept a specific fixture
just t <category> NNN --reject       # Reject a specific fixture
just t <category> --accept --confirm # Accept all pending snapshots
```

## Multi-File Tests

For imports, re-exports, or cross-module behavior:

```
tests-integration/fixtures/<category>/NNN_import_test/
├── Main.purs    # Scenario driver
├── Lib.purs     # Supporting module
└── Main.snap    # Generated snapshot where the harness snapshots Main.purs
```

**Lib.purs:**
```purescript
module Lib where

life :: Int
life = 42

data Maybe a = Just a | Nothing
```

**Main.purs:**
```purescript
module Main where

import Lib (life, Maybe(..))

test :: Maybe Int
test = Just life
```

- Module name must match filename
- Checking, semantic, and LSP fixtures snapshot only `Main.purs`
- Lowering and resolving fixtures snapshot every `.purs` file

## Snapshot Review Focus

### Checking

```
Terms
functionName :: InferredOrCheckedType
...

Types
TypeName :: Kind
...

Errors
ErrorKind { details } at [location]
```

Check inferred/checked types, kind/role output, constraints, diagnostics, and source locations.

### Semantic

Check semantic declaration kinds, finalized types and kinds, constructor arguments, binders, expressions, and explicit evidence. Confirm that syntax sugar expected to disappear is absent and syntax intentionally preserved by checking remains present.

### Lowering

Check the lowered module report, especially declarations, binders, equations, and source links. Unexpected name-link changes are often as important as textual output changes.

### Resolving

Check local/imported/exported references, qualification, hidden imports, re-exports, and duplicate-name diagnostics. Multi-module resolving fixtures can update many snapshots; each changed module should be intentional.

### LSP

Check hover text, definitions, completions, edits, and reported positions. Review both the source excerpt and the resulting LSP payload.

## Acceptance Criteria

Before accepting, verify:

1. **The category is appropriate**
   - Checking owns type inference/checking behavior
   - Semantic owns the typed semantic tree produced by checking
   - Lowering owns lowered core/source-link behavior
   - Resolving owns name/import/export behavior
   - LSP owns editor-facing reports

2. **The fixture is narrow**
   - One behavior per fixture
   - Supporting modules exist only when they clarify the behavior

3. **Snapshots are intentional**
   - Checking types are correct
   - `test :: Array Int -> Int` - signature preserved
   - `test' :: forall t. Array t -> t` - polymorphism inferred
   - Semantic/lowering/resolving/LSP changes match the feature or bug being tested

4. **No unexpected `???`**
   - `test :: ???` - STOP: inference failure
   - `CannotUnify { ??? -> ???, Int }` - OK in error tests

5. **Errors appear where expected**
   - Confirm error kind matches (`NoInstanceFound`, `CannotUnify`)
   - Verify location points to correct declaration

6. **Polymorphism is appropriate in checking snapshots**
   - Type variables scoped correctly
   - Constraints propagate as expected

## Common Issues

| Symptom | Likely Cause |
|---------|--------------|
| `test :: ???` | Syntax error or undefined names |
| Unexpected monomorphism | Missing polymorphic context |
| Wrong error location | Check binder/expression placement |
| Missing types in snapshot | Module header or imports incorrect |
| Missing expected module snapshot | Category snapshots only `Main.purs` (`checking`, `semantic`, `lsp`) or module filename does not match module header |
| Extra resolving/lowering snapshot | Every `.purs` file is snapshotted in `resolving` and `lowering` |
