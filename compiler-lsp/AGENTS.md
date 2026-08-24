## Scope

Compiler LSP crates adapt compiler data to editor and package-tooling features. Compiler semantics
belong in `compiler-frontend`; generic infrastructure belongs in `compiler-core`.

## Components

- **analyzer**: IDE features such as completion, definition, references, rename, hover, semantic
  tokens, code actions, and diagnostics
- **documentation**: renders checked modules and source documentation into package documentation and
  TypeScript-facing schemas
- **spago**: reads Spago lockfiles and maps source files to packages

Keep LSP position conversion and protocol types at this boundary. Analyzer features should consume
stable frontend IDs and source maps rather than reimplementing parsing, name resolution, or checking.

## Verification

- Run `cargo check -p <crate-name> --tests` for a changed crate.
- Run `cargo nextest run -p <crate-name>` for crate unit tests.
- Run `just t lsp` when analyzer requests, responses, capabilities, positions, or editor-visible
  behavior can change.
- Regenerate snapshots through the owning test command and inspect every changed snapshot.
