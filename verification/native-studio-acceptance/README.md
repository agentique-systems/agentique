# Native Studio Alpha acceptance evidence

Work remains in progress. The latest completed real workflow is a useful
engineering demonstration, but the measured semantic waits and incomplete
qualification prevent Alpha acceptance. Reduced work counters are not a
wall-clock performance result.

## Completed measurements

- [Actual self-model journey and complete gallery](real-run02/README.md):
  39 assertions passed, including a nested Part candidate, review, validation,
  durable commit, real ports and derived explanation. Preparation took 171.512 s.
  The current revision remained interactive during construction.
- [Same-runner independent semantic oracle](ci-oracle-01/README.md): 697,419
  exact observations and 16 malformed reuse rejections passed. The tested
  optimization regressed preparation from 161.497 s to 219.531 s; current cold
  reconstruction took 163.268 s. The artifact predates the corrective checkpoint
  and audit changes. Full lossless evidence and a hash-checking restore helper
  are retained.
- [Read-path profile](read-profile-02/README.md): warm open remained 181.258 s.
  Bounded loaded System/Graph/Requirements projections were 66/60/60 ms; focused
  projection 377 ms; Inspector 233 ms initially and 0.33 ms on repeat.
- [Source-only separate-process recovery](source-recovery-02/README.md): passed
  after semantic caches were removed, with exact durable identities and checked
  semantic answers. Rebuild took 180.324 s.
- [Camera stress](camera-stress01/README.md): native 10k pointer zoom passed.
  Ordered raw-event and numeric property tests accompany the camera correction.
- [First soak and its rejection](soak01-accounting-failure.md): 14.06 minutes
  of interaction, separate-process restart and 77 native assertions passed, but
  the external gate correctly rejected a terminal cycle-count error. The old
  report is preserved. A corrected driver must complete a fresh full soak.
- [Published runtime qualification](runtime-qualification-01/README.md): exact
  pinned runtime bundle authenticated, installed and publicly downloadable.
- [Independent-validator CI diagnosis](ci-independent-fixtures-023423-diagnosis.md):
  the pre-existing Gen1 named-payload disagreement remains a nonzero release
  obligation; it was not hidden to make this mission green.

## Current corrections awaiting real qualification

The shared-dependency checkpoint avoids rehashing the same immutable accepted
proof graph twice. Audit signatures distinguish actual query inputs from outputs
that merely cite those inputs. Expanded positive proof reads and a sticky compact
evidence marker prevent unsafe reuse. See the [checkpoint review](semantics/shared-dependency-checkpoint-review.md)
and [audit review](semantics/audit-separation-review.md). New exact real-model
equivalence and timing must qualify these changes before any performance claim.

History lineage, initial durable Diff framing and candidate visibility were
corrected from actual screenshots. Subsequent fixes address nearest parallel
relationship selection, exact Requirements element labels, cancellation failure
messages, GPU timing diagnostics and terminal soak accounting. Their native/scene
unit gates passed; the new real gallery and full soak are separate obligations.

The [product review ledger](reviews/alpha-redteam-ledger.md) records actual image
criticism, changes and remaining weaknesses. It explicitly distinguishes five
reviewer perspectives from five completed post-change review rounds.

Historical freshness proposals are unapplied and intentionally stale after later
semantic changes. A new exact source-bound proof and independent review are
required before updating implementation pins. Authoritative source, identities,
accepted publication identities and validation obligations remain unchanged.
