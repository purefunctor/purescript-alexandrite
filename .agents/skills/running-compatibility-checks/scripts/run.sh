#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: run.sh [base-ref]

Compare the current checkout with base-ref using release-built compatibility
verifiers. base-ref defaults to origin/main.
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

base_ref="${1:-origin/main}"
workspace=$(git rev-parse --show-toplevel)
base_commit=$(git -C "$workspace" rev-parse --verify "${base_ref}^{commit}") || {
  printf 'compatibility: base revision not found: %s\n' "$base_ref" >&2
  exit 2
}

timestamp=$(date -u +%Y%m%dT%H%M%SZ)
report_dir=${COMPATIBILITY_REPORT_DIR:-"$workspace/target/compatibility-reports/$timestamp-$$"}
temporary_dir=$(mktemp -d "${TMPDIR:-/tmp}/alexandrite-compatibility.XXXXXX")
base_workspace="$temporary_dir/base"

cleanup() {
  git -C "$workspace" worktree remove --force "$base_workspace" >/dev/null 2>&1 || true
  rm -rf "$temporary_dir"
}

trap cleanup EXIT
trap 'exit 130' INT TERM

mkdir -p "$report_dir"

printf 'Compatibility base: %s (%s)\n' "$base_ref" "$base_commit"
printf 'Building candidate verifier in release mode...\n'
cargo build --manifest-path "$workspace/Cargo.toml" \
  --target-dir "$workspace/target" \
  --release \
  -p tests-compatibility

printf 'Creating temporary base worktree...\n'
git -C "$workspace" worktree add --detach "$base_workspace" "$base_commit" >/dev/null

printf 'Building base verifier in release mode...\n'
cargo build --manifest-path "$base_workspace/Cargo.toml" \
  --target-dir "$base_workspace/target" \
  --release \
  -p tests-compatibility

candidate_verifier="$workspace/target/release/tests-compatibility"
base_verifier="$base_workspace/target/release/tests-compatibility"
corpus_dir="$workspace/target/compatibility"

printf 'Preparing core and Acme corpus...\n'
(
  cd "$workspace"
  "$candidate_verifier" prepare --preset core --preset acme
)

printf 'Collecting base and candidate reports...\n'
set +e
(
  cd "$base_workspace"
  "$base_verifier" verify \
    --preset core \
    --preset acme \
    --registry-dir "$corpus_dir/registry" \
    --index-dir "$corpus_dir/registry-index" \
    --cache-dir "$corpus_dir" \
    --json-output "$report_dir/base.json"
) >"$report_dir/base.log" 2>&1
base_status=$?

(
  cd "$workspace"
  "$candidate_verifier" verify \
    --preset core \
    --preset acme \
    --registry-dir "$corpus_dir/registry" \
    --index-dir "$corpus_dir/registry-index" \
    --cache-dir "$corpus_dir" \
    --json-output "$report_dir/candidate.json"
) >"$report_dir/candidate.log" 2>&1
candidate_status=$?
set -e

if [[ "$base_status" != 0 && "$base_status" != 1 ]]; then
  printf 'compatibility: base verifier failed with status %s; inspect %s/base.log\n' \
    "$base_status" "$report_dir" >&2
  exit 2
fi

if [[ "$candidate_status" != 0 && "$candidate_status" != 1 ]]; then
  printf 'compatibility: candidate verifier failed with status %s; inspect %s/candidate.log\n' \
    "$candidate_status" "$report_dir" >&2
  exit 2
fi

printf 'Comparing reports...\n'
set +e
(
  cd "$workspace"
  "$candidate_verifier" compare \
    --base-report "$report_dir/base.json" \
    --candidate-report "$report_dir/candidate.json" \
    --json-output "$report_dir/comparison.json" \
    --summary-output "$report_dir/summary.md"
) >"$report_dir/comparison.log" 2>&1
comparison_status=$?
set -e

if [[ "$comparison_status" != 0 && "$comparison_status" != 1 ]]; then
  printf 'compatibility: comparison failed with status %s; inspect %s/comparison.log\n' \
    "$comparison_status" "$report_dir" >&2
  exit 2
fi

printf '\n'
cat "$report_dir/summary.md"
printf '\nCompatibility reports: %s\n' "$report_dir"
exit "$comparison_status"
