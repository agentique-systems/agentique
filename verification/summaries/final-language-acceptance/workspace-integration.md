# Held workspace integration on lossless publication archives

Base: `0e05b27` (`Preserve selected proof and search evidence in Systems caches`).
The isolated `foundation/workspace-integration-held` branch consolidates declared
history `5fbe883`, physical shared storage `845c125` and production workspace
`170b953`. Publication authority and workspace activation remain gated.

The archive merge retains both contracts: dependent restore forks the immutable
status, search and selected-reference-contribution tables, while the lossless
archive restores local selected proof/search support. Wire formats and selected
evidence authentication remain those of the base publication implementation.

The lossless dependent archive regression now observes physical storage after
restore. Its base-table identities exactly match the supplied immutable
publication and its copied-dependency-entry counts are zero. The same regression
checks aggregate facts, selected contributions, searches, rewritten bytes and
rejection of an aggregate-equivalent dependency with different selected evidence.

Actual integrated checks:

- Kernel library, archive, declared history, immutable dependency and derived
  association groups: 49 passed, 0 failed.
- Lossless selected-evidence roundtrip with the new storage assertions:
  1 passed, 0 failed.
- Frontend source-history unit regressions: 3 passed, 0 failed.
- Strict SourceProject and workspace syntax/edit/recovery fixtures:
  18 passed, 0 failed.
- All ten held workspace acceptance tests compile with `verification`, including
  the self-model and 100-document/five-revision/parallel-reader tests.
- Kernel/frontend/workspace all-target Clippy with `verification`: exit 0 with
  `-D warnings`; `cargo fmt --all -- --check`: exit 0, no output.

No accepted publication cache was loaded and no held semantic acceptance test
was executed. Those assertions remain subject to the lead's H–K readiness gate.
The adjacent command ledger records actual output summaries, exit codes and
SHA-256 hashes of retained raw logs. Prior component ledgers remain attributable
to their original isolated worktrees and are not relabeled as integration runs.
