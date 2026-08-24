## Scope

The compiler frontend turns PureScript source into checked semantic data. Its components are
designed for editor introspection and compatibility with query-based incremental builds.

## Components

The component names listed below are crate names in this workspace.

- **lexing**: tokenization and the layout algorithm
- **parsing**: parsing into a rowan-based CST
- **syntax**: types for the rowan-based CST
- **sugar**: syntax desugaring (e.g., operator bracketing)
- **lowering**: core semantic representation, name resolution
- **indexing**: high-level relationships between module items
- **resolving**: name-indexed interface for module items
- **stabilizing**: assigns stable IDs to source ranges
- **checking**: type checking and elaboration
- **diagnostics**: error collection and rendering for LSP and tests
- **documenting**: extracts source documentation and associates it with indexed items

Infrastructure shared across compiler stages belongs in `compiler-core`; executable code generation
belongs in `compiler-backend`.

## Verification

- Run `cargo check -p <crate-name> --tests` for a changed crate.
- Run `cargo nextest run -p <crate-name>` for frontend unit tests.
- Use `just t checking`, `just t lowering`, or `just t resolving` when behavior in the corresponding
  integration-test category can change.
- Snapshot changes must be generated and reviewed through the owning test command; never edit them
  by hand.

## Key Concepts

- Uses rust-analyzer/rowan, a lossless syntax tree library inspired by Swift's libsyntax
- Query-based incremental builds (not traditional phase-based)
- Interning and arena allocation enable better caching (e.g., whitespace changes don't invalidate type checking)
