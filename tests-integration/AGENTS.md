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
- Use the unified `compiler` category for pipeline behavior and keep its `Main.purs` entry focused.
  Review the report that owns the contract: checking types, diagnostics for every fixture-owned
  module with stable relative paths, recovery semantic trees, or successful/explicitly rejected
  functional trees. Use runtime verification only when execution is the contract.
- Use real registry modules for library dependencies rather than handwritten stand-ins or vendored
  copies. Deliberately malformed or substituted registry modules must declare their reason through
  the existing fixture replacement mechanism.
- Keep goldens focused on reachable fixture-owned generated JavaScript. Optional `verify.mjs`
  verification must exercise a freshly generated full dependency closure, not tracked goldens.
- Verify harness changes through representative fixtures and affected category runs. A separate unit
  test is justified for an isolated harness algorithm only when it protects a distinct local
  invariant.

Use the `workflow-integration-tests` skill for fixture conventions and snapshot commands, and
`CONTRIBUTING.md` for package preparation and ownership. Preserve the root verification gates.
