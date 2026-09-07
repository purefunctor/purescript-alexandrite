## Ownership

This crate owns source-based compiler integration tests through the public APIs and the combination
of compiler subsystems. Loading a file and observing its parsing, resolution, checked types,
diagnostics, semantic trees, generated code, or editor analysis belongs in the fixture harness.

Use the existing fixture loader, runners, and snapshot machinery. Do not reproduce this pipeline in
compiler-crate unit tests or add harness unit tests that merely load and compile another source
file. Local algorithm invariants belong beside their implementation; CLI and development-environment
contracts belong in `tests-e2e`.

## Fixture design

- Extend an existing fixture when it expresses the same case; add a fixture for a distinct behavior.
  Avoid duplicating a scenario across categories merely to increase coverage.
- Assert at the stage that owns the behavior. A type-checking regression needs checked results or
  diagnostics, not mandatory JavaScript execution. Use backend runtime verification when executable
  semantics are the contract; running generated JavaScript alone does not make a test CLI E2E.
- Use real registry modules for library dependencies rather than handwritten stand-ins or vendored
  copies. Deliberately malformed or substituted registry modules must declare their reason through
  the existing fixture replacement mechanism.
- Keep goldens focused on fixture-owned results. Registry dependency output is not fixture output;
  runtime verification must exercise freshly generated code, not tracked JavaScript goldens.
- Verify harness changes through representative fixtures and affected category runs. A separate unit
  test is justified for an isolated harness algorithm only when it protects a distinct local
  invariant.

Use the `workflow-integration-tests` skill for fixture conventions and snapshot commands, and
`CONTRIBUTING.md` for package preparation and ownership. Preserve the root verification gates.
