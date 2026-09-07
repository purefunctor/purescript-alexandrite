## Ownership

This crate prepares pinned registry sources for test harnesses. It owns package resolution, archive
verification, extraction, and cache publication, not compiler semantics or fixture assertions.

Keep preparation separate from fixture execution: prepared-source lookup must remain offline and
read-only. Preserve digest verification, serialized preparation, and atomic publication of complete
source sets. Downloaded packages are development dependencies and must not become compiler release
contents or vendored fixture libraries.

## Tests

Small constructed metadata and archives are appropriate for local resolution, validation,
extraction, and cache invariants. Such tests should not require a live registry or a compiler
pipeline. Verify the use of prepared packages by compilation through `tests-integration`, using its
existing loader rather than building another one here.
