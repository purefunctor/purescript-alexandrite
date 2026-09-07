## Ownership

The frontend turns PureScript source into checked semantic data. Its representations must support
editor introspection and query-based incremental builds, rather than assume a traditional sequence
of compiler phases.

Infrastructure shared across compiler stages belongs in `compiler-core`; executable code generation
belongs in `compiler-backend`.

## Incremental contracts

Preserve lossless syntax and incremental reuse. Interning, arena allocation, and stable identities
allow semantic results to survive trivial source changes; do not introduce dependencies on source
ranges where stable identities are needed for reuse.

Run integration categories whose semantic behavior can change, including checking, lowering, and
resolving as applicable.
