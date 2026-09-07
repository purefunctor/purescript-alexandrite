## Ownership

Compiler LSP crates adapt compiler data to editor and package-tooling features. Compiler semantics
belong in `compiler-frontend`; generic infrastructure belongs in `compiler-core`.

Keep LSP position conversion and protocol types at this boundary. Analyzer features should consume
stable frontend IDs and source maps rather than reimplementing parsing, name resolution, or
checking.

## Editor contracts

Run `just t lsp` when analyzer requests, responses, capabilities, positions, or editor-visible
behavior can change. Changes to protocol adaptation must preserve the underlying compiler semantics.
