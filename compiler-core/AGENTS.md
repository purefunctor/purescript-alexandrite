## Scope

Compiler core owns stage-independent infrastructure shared by the frontend, backend, LSP, and
compiler entry points. Language analysis and transformations belong in `compiler-frontend`;
executable code generation belongs in `compiler-backend`.

## Components

- **building**: query-based parallel build engine and compiler pipeline orchestration
- **building-types**: query keys, results, and interfaces shared by pipeline components
- **files**: virtual PureScript and foreign-file storage with stable file IDs
- **interner**: generic sequential and parallel interners
- **prim-constants**: embedded primitive PureScript modules

Keep these crates independent of user-facing protocol concerns. New source-language passes and
semantic representations should not be added here merely because several consumers use them.

## Verification

- Run `cargo check -p <crate-name> --tests` for a changed crate.
- Run `cargo nextest run -p <crate-name>` when the crate has unit tests.
- Changes to `building` can affect every compiler stage; run the integration-test categories for
  each changed query or pipeline behavior.

## Key Concepts

- `building` is a query-based incremental build engine, not a traditional phase driver.
- Query cancellation, cycle detection, and stable identities are cross-stage contracts.
- Infrastructure dependencies should point outward only when orchestration requires them; generic
  primitives should remain stage-independent.
