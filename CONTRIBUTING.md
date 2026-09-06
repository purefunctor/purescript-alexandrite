# Contributing

Thank you for taking interest in contributing to Alexandrite.

## Integration tests

Run `just t backend`, `just t checking`, or `just t semantic` to test a category.
These commands prepare the exact registry packages in
[`tests-integration/registry-lock.json`](tests-integration/registry-lock.json)
before starting the fixture runner. Node.js 22 is required for backend execution.
Other categories do not load registry packages.

For direct nextest use, prepare the packages first:

```sh
just integration-prepare
cargo nextest run -p tests-integration
```

Preparation downloads digest-verified archives into `target/integration-packages`.
Source sets are published atomically and are immutable; concurrent preparation
commands are serialized. Once prepared, fixtures read only local sources and run
offline. A warm preparation also requires no network. Do not put downloaded
sources into fixture directories.

### Fixture ownership and execution

Backend fixtures compile their reachable dependency closure with the current
Alexandrite. Only fixture-owned generated JavaScript and adjacent FFI are kept in
`output/`; registry output and the compiler's `runtime.js` are not goldens.
`Main.functional.snap` records successful functional trees. Backend errors are
diagnostics in `Main.snap`, not raw errors in functional snapshots.

An optional `verify.mjs` is staged beside a fresh temporary `output/` containing
the complete generated program. Use imports such as `./output/Main/index.js` and
Node built-ins. The runner supplies ESM configuration; fixtures do not need a
`package.json`. Verification never executes tracked goldens. It must pass before
`just t backend --update-output` writes new goldens.

Use real package modules rather than local library stand-ins. Tests that must
deliberately replace a compiler-known module can declare the module and its reason
in a fixture-local `replacements.json`:

```json
{
  "Data.Generic.Rep": "Omit representation types to test the missing representation diagnostic."
}
```

Undeclared collisions and unused replacements fail. Replacements apply only to
registry modules, never Prim or another fixture module, and do not inherit the
package's FFI.

### Updating dependencies

Regenerate the lock explicitly, supplying the package-set version and all root
packages (the current roots are recorded in the lock):

```sh
cargo run -p tests-support -- lock <package-set-version> tests-integration/registry-lock.json <root-package>...
just integration-prepare
```

The lock command checks the transitive closure against package-set versions and
dependency ranges and records registry SHA-256 hashes. Review every resulting
fixture change: real package APIs, instances, and runtime representations may
differ from earlier versions. Run all affected categories without filters before
accepting the migration. Packages with FFI that imports additional assets need an
explicit staging design; the current runner copies adjacent FFI files only.

Registry packages are development dependencies, not compiler dependencies.
Release builds do not prepare them, and release archives must not include their
sources, FFI, or generated output. Cached packages retain their license files;
any third-party code retained in fixtures still needs its own provenance review.

## Agentic Coding

See [AGENTS.md](AGENTS.md) for agentic coding guidelines written for both humans and agents.
