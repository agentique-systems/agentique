# Held production modeling workspace preparation

The isolated `foundation/modeling-workspace-held` branch prepares Phase M. It is
not integrated publication authority and has not run its accepted-cache semantic
tests. Foundation readiness, Agentique self-model acceptance and effective core
acceptance remain prerequisite gates owned by the lead acceptance workstream.

## Implementation

- `agq-modeling-workspace` adds immutable history, an independent workspace
  revision identity, expected-head document batches, `.kerml`/`.sysml` edits,
  Working/Validated handles, an explicit Phase1V1 acceptance contract and borrowed
  native query/source/element facades. It has no forbidden Gen1 dependency.
- Additive frontend `SourceInputs` and `SourceCompilation` reuse exact document
  preparation and accepted-language lowering. Current unresolved constructions
  and partial overlays remain available with native evidence. Recovered or
  explicitly unsupported documents are omitted conservatively and mark the
  root population pending in every producer and query context. Previous graphs
  are never current-source fallbacks.
- Kernel-owned declared identity history is reconciled before derived semantic
  construction. The source ledger explicitly retires removed syntax/document
  identities, including deletion during recovery; temporary omission alone does
  not retire a surviving source identity. Strict promotion preserves these
  reservations through checked kernel revalidation.
- SysML syntax authority derives from the actual accepted semantic profile,
  including explicit historical profiles. Capability diagnostics retain native
  effective-query pending evidence instead of inferring support from producer
  convergence or certificate coverage.
- Verification records actual scheduler subject evaluations. Storage adapters
  delegate to kernel physical-table observers; they do not invent no-copy
  counters from accepted facade handles.
- Permanent held tests cover the existing 100-document/five-revision/four-reader
  fixture, five-file Agentique self-model edit, batch failure atomicity and
  explicit unsupported-source Working/repair behavior.

## Verification status

Actual focused results:

- New frontend source/identity tests: 3 passed, 0 failed.
- Existing strict SourceProject tests: 13 passed, 0 failed.
- Workspace edit/scale syntax fixtures: 2 passed, 0 failed.
- Working/recovery syntax fixtures: 3 passed, 0 failed.
- Workspace, frontend and scheduler all-target Clippy with the verification
  feature and real shared storage: exit 0 with `-D warnings`.
- New workspace strict Rustdoc with verification: exit 0 with
  `RUSTDOCFLAGS=-D warnings`.
- All held workspace integration targets compile with the verification feature:
  `phase1`, `working_states` and `self_model`; no accepted-cache tests ran.
- Repeated the 3 source/history tests after integrating shared kernel storage:
  3 passed, 0 failed.
- `cargo fmt --all -- --check`: exit 0, no output.
- Normal dependency-tree boundary check: exit 0, no `agq-model`,
  `agq-semantics`, `agq-workspace` or `agq-application` dependency.

The held implementation depends on kernel history `5fbe883` and shared storage
`845c125`, and was verified after integrating main through `bf02cf8`. Those
kernel changes retain their own test/equivalence ledgers. Workspace observer
counts have not been measured against real accepted Systems because that
publication gate is still pending. No synthetic facade or fabricated receipt
was substituted.

Transient failures were resolved: the first identity test assumed which of
multiple deleted declaration/ancestor IDs the kernel would reject first; it
now asserts the reported ID is explicitly retired. Initial Clippy found a
large new error payload and construction enum; those payloads are boxed.
An initial boxing edit hit another origin field and was corrected immediately.
The production error classification continues distinguishing unsupported source
from operational/invariant failure; no blanket error-to-Working conversion was
introduced.

The attempted shell cleanup of an accidental local build target was rejected
automatically. The subsequent exact-target `cargo clean` succeeded, removing
535 artifact files (239.6 MiB). Source, evidence and caches were not removed.

Command/output/exit evidence is recorded in the adjacent command ledger; raw
logs remain under ignored `verification/generated/final-language-acceptance/workspace`.
