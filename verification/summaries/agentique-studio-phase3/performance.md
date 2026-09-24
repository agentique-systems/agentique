# Authored reconstruction frontier

The Phase 3 performance change reduces declared kernel record mutation and
allocation work beyond document parsing/lowering. An edit with a strict authored
predecessor reconstructs the complete desired declarations, then submits an
ordinary kernel transaction over that predecessor containing changed/new/removed
local records only. Unchanged canonical record allocations remain shared.

Reference refinement does not trust the preceding resolved endpoints: absent
current-pass endpoints are cleared before the new candidate is queried. Source
origins, record identities, association identities/order, retired reservations,
incomplete obligations and full candidate validation remain authoritative.
Effective producer facts are never copied into source declarations. Incomplete
predecessors and the full-reconstruction oracle retain the existing full path.

`CompilationWork.records_rebuilt` measures created, changed and removed records
across all construction passes. The additive `records_reused` counter measures
retained declared records on which no record/slot mutation was submitted. These
are work counters, not claims about semantic acceptance or latency.

## Deliberate remaining frontiers

- **Kernel validation/indexes:** still rebuilt/validated over the complete
  candidate. This change does not claim incremental invariant validation.
- **Producers:** existing checkpoint rebind verifies exact query/provider reads,
  including negative searches; broad dependencies still reopen work. Transferring
  derived overlays onto changed declared graphs remains unsupported. Narrowing
  this safely needs dependency-qualified output retraction/replay and equivalence
  evidence; document separation alone is insufficient.
- **Effective audit:** every final local subject remains audited. The existing
  audit report stores findings/counts but not a reusable per-subject footprint
  covering positive, negative, provider and closure reads. Reusing that report
  solely because a subject's own record is unchanged would be unsound.

## Evidence boundary

The three focused oracle tests pass. The complete `agq-kerml-text` library suite
passes **63 tests, 0 failures, 2 accepted-publication tests ignored** (15.05 s).
Focused Clippy over the library and its tests passes with `-D warnings`.

Observed work/times from the focused debug run:

| Edit fixture | Incremental changed/removed records | Full rebuilt records | Retained records | Incremental construction | Full construction |
| --- | ---: | ---: | ---: | ---: | ---: |
| Two-document rename | 14 | 12 | 4 | 5.087 ms | 6.194 ms |
| Two-document add part | 16 | 14 | 4 | 6.864 ms | 7.005 ms |
| Two-document remove part | 12 | 9 | 4 | 3.456 ms | 5.509 ms |
| Two-document remove typing | 14 | 11 | 4 | 3.882 ms | 4.815 ms |
| Two-document unresolved reference | 15 | 12 | 4 | 4.440 ms | 5.183 ms |
| 100-document rename | 9 | 401 | 396 | 24.721 ms | 161.748 ms |

The 100-document fixture reduces kernel record mutations by **97.8%**. Small
fixtures can submit more mutations than reconstruction because conservative
syntax reconciliation retires/recreates edited declarations; those deletions
are counted honestly. They still share the unchanged declarations.

Times are single debug observations during concurrent Studio development,
including lowering, contextual completion and kernel construction on already
parsed inputs. They do not isolate the new kernel optimization from existing
lowering reuse, and exclude accepted semantic producer closure/effective audit,
repository durability and UI round trips. No end-to-end interactive latency
improvement is claimed.

Focused construction oracles use real Gen2 descriptors, syntax reconciliation,
lowering and kernel transactions without fabricating accepted publications.
They compare complete canonical records/provenance, occurrences, reference
assertions, source maps and construction obligations with full reconstruction;
they also check unchanged record allocation sharing and prior immutability.

The 100-document construction fixture is a bounded kernel-work characterization.
It is **not** the separate 100-document durable repository/semantic-closure gate.
Whole-workspace and accepted-publication runtime results belong to the integration
ledger. End-to-end semantic edit latency remains unmeasured in this session until
the independently accepted publication/cache inputs are available.

Commands, actual outputs and exit codes are recorded in
`performance-commands.json` in this directory.

## Full-path collision follow-up

The integration review caught an unintended difference when a full construction
base already contained an output identity. Full construction now bypasses all
old-record and old-occurrence reuse, unconditionally creating/linking as the
original implementation did. The added collision regression requires the native
kernel `ReusedIdentity` error for both no-cache and reuse-disabled paths, and
checks the base remains unchanged.

All four focused reconstruction oracles pass after the correction. The full
library suite passes **64 tests, 0 failures, 2 accepted-cache tests ignored**
(19.95 s); focused Clippy, formatting and diff whitespace checks also pass. The
100-document work counts remain 9 changed/removed versus 401 full records, with
396 retained. A second debug timing observation during concurrent workspace
verification was 96.606 ms incremental and 461.696 ms full. The variation from
the earlier run reinforces that these are characterization observations, not
an interactive latency acceptance target.
