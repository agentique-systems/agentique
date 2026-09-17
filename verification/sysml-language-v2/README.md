# SysML language generation 2: staged verification

Base: `932bbd4b77237216385fcfd4c09aefb499223b04` (`origin/main`, fetched before
implementation). Worktree was clean. Branch: `foundation/sysml-language-v2`.

## Gate 0 — passed

Architecture and contributor guidance distinguish generation 1 from generation 2.
The v0.1 release register and obligations are preserved. The separate generation-2
register records descriptor, query, text, library, SysML, API and execution gaps.
ADR follow-up annotations preserve history while correcting current ownership
storage and frontend guidance.

All seven commands in [stage-0/results.json](stage-0/results.json) actually ran and
returned zero: kernel, KerML, semantic and text tests; extraction integrity;
engineering tests; local documentation-link/status checks. Logs accompany results.
Run `node verification/sysml-language-v2/run.mjs stage-0` to reproduce.

Passing a gate establishes only its stated scope. Later gates are not completed
by this record. No SysML conformance or application migration is claimed.

## Gate 1 — passed, informative fixture unavailable

Pinned the formal SysML 2.0 XMI and JSON and reverified/reused the existing official
Systems Library and three required Kernel archives. The exact dependency closure,
entry hashes and provenance are in the [library set](../../standards/normative/sysml-2.0/library-set.json).
The informative example's unavailable endpoint is recorded explicitly in the
[input lock](../../standards/normative/sysml-2.0/lock.json); no substituted fixture
or normative status is invented. It is optional to the planned authored slice.

All three commands in [stage-1/results.json](stage-1/results.json) returned zero:
offline artifact tests (including changed upstream/local bytes, roles and versions),
standards integrity and the existing KerML generator currentness check. Original
KerML metamodel bytes/IDs/generated output, old locks and library bytes are unchanged.
Acquisition is an explicit maintenance tool, separate from builds and tests.
