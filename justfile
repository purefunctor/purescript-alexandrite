_default:
    just --list

set positional-arguments

[doc("Generate coverage for local tests")]
coverage:
  cargo llvm-cov clean --workspace
  cargo llvm-cov nextest --no-report
  cargo llvm-cov nextest --no-report -p purescript-alexandrite -p compiler-scripts
  cargo llvm-cov nextest --no-report -p tests-integration

[doc("Generate coverage with the package set")]
coverage-full: coverage
  cargo llvm-cov nextest --no-report -p tests-compatibility

[doc("Generate coverage report for Codecov")]
coverage-codecov:
  cargo llvm-cov report --codecov --output-path codecov.json

[doc("Generate coverage report as HTML")]
coverage-html:
  cargo llvm-cov report --html

@integration *args="":
  cargo nextest run -p tests-integration "$@" --status-level=fail --final-status-level=fail --failure-output=final

[doc("Run integration tests with snapshot diffing: backend|checking|semantic|lowering|resolving|lsp")]
@t *args="":
  cargo run -q -p compiler-scripts --release -- "$@"

[doc("Run package-set benchmarks (e.g. just bench --bench checking_single_core)")]
@bench *args="":
  cargo criterion -p tests-compatibility "$@"

[doc("Compare package compatibility with a base revision using release builds")]
@compatibility base="origin/main":
  bash .agents/skills/running-compatibility-checks/scripts/run.sh {{quote(base)}}

[doc("Apply clippy fixes and format")]
fix:
  cargo clippy --workspace --fix && cargo fmt

[doc("Update THIRDPARTY.toml")]
[working-directory: 'compiler-bin']
licenses:
  cargo bundle-licenses --prefer MIT -o ../THIRDPARTY.toml

[doc("Update the release version and third-party licenses")]
prepare-release version:
  cargo set-version --package purescript-alexandrite "{{version}}"
  just licenses

[doc("Format imports with module granularity")]
@format *args="":
  cargo +nightly fmt {{args}} -- --config imports_granularity=Module
