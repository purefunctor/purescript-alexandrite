---
name: running-compatibility-checks
description: "Runs Alexandrite package-set compatibility comparisons with release-built verifiers. Use when checking the current checkout against a base revision, investigating compatibility regressions, or reviewing compatibility reports."
---

# Running Compatibility Checks

Use the repository's compatibility runner to compare the current checkout with a base revision. The runner mirrors the CI package selection (`core` and `acme`), builds both verifiers in release mode, and preserves the generated reports.

## Run the comparison

From the repository root:

```bash
just compatibility
```

The default base is the local `origin/main` ref. When the comparison must use the latest remote `main`, fetch it first:

```bash
git fetch origin main
just compatibility
```

Pass another commit, branch, or tag when requested:

```bash
just compatibility <base-ref>
```

Do not require a clean worktree. The candidate is the current checkout, including uncommitted source changes. The base runs in a temporary detached worktree, so the command does not switch or modify the active checkout. The candidate build and prepared corpus are written under `target/`.

## Interpret the result

The runner prints the Markdown summary and the report directory. Inspect at least:

- `summary.md` for introduced and fixed diagnostics
- `comparison.json` for the structured comparison
- `base.log` and `candidate.log` when either verifier fails
- `comparison.log` when report comparison fails

Exit statuses have stable meanings:

- `0`: no compatibility regressions
- `1`: the candidate introduces compatibility errors; the reports are valid and must be reviewed
- `2`: preparation, verification, or comparison could not execute correctly

Do not describe status 1 as a broken checker. Report the introduced errors from `summary.md`. For status 2, diagnose the relevant log and distinguish network/package-corpus failures from compiler failures.

## Run one revision without comparison

Only use a direct verification when the user explicitly wants one revision rather than a base/candidate regression check:

```bash
cargo run --release -p tests-compatibility -- prepare --preset core --preset acme
cargo run --release -p tests-compatibility -- verify --preset core --preset acme
```

The direct verifier also uses status 1 for a valid report containing errors and status 2 for an execution failure.
