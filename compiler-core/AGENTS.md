## Ownership

Compiler core owns stage-independent infrastructure shared by the frontend, backend, LSP, and
compiler entry points. Language analysis and transformations belong in `compiler-frontend`;
executable code generation belongs in `compiler-backend`.

Keep core independent of user-facing protocol concerns. Several consumers needing a source-language
pass or semantic representation does not make it stage-independent infrastructure. Dependencies may
point outward when pipeline orchestration requires them; generic primitives must remain independent.

## Incremental contracts

`building` is a query-based incremental build engine, not a traditional phase driver. Preserve query
cancellation, cycle detection, and stable identities as cross-stage contracts. Changes to building
queries or pipeline behavior require integration coverage for every affected compiler stage.
