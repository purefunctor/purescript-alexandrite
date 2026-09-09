#!/usr/bin/env bash
set -euo pipefail

# Buildkite indexes jobs from zero; nextest partitions start at one.
partition="hash:$((BUILDKITE_PARALLEL_JOB + 1))/${BUILDKITE_PARALLEL_JOB_COUNT}"

case "${1:-}" in
  integration)
    just integration --locked --no-fail-fast --partition "$partition"
    ;;
  e2e)
    just e2e-prepare
    just e2e --locked --no-fail-fast --partition "$partition"
    ;;
  *)
    echo "Usage: $0 integration|e2e" >&2
    exit 1
    ;;
esac
