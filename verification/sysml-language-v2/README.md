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

## Gate 2 — passed

The shared profile-driven importer handles SysML and authoritative references to
KerML classes, properties, packages, enums/literals and operations. ADR 0007 records
the published URI spelling and JSON namespace projection differences. Descriptor
key v1 and all four existing KerML generated artifacts remain byte-for-byte current.
The SysML neutral bundle is deterministic and checked by the default offline CLI.

All five commands in [stage-2/results.json](stage-2/results.json) returned zero:
format, strict generator Clippy, all generator tests, offline generated currentness
and standards integrity. Tests include source-qualified direct/transitive inheritance,
same-name isolation, unresolved/wrong-kind references, wrong baseline/artifact and
stale output rejection. This gate does not establish runtime SysML support.

## Gate 3 — blocked; stages 4–7 not started

The minimum PartDefinition/PartUsage closure has 54 SysML and 50 KerML classes,
254 associations, three enums and 562 properties. Both pinned metamodels contain
required self-subsetting properties. The supplied PDF figures repeat these edges;
the UML property naming constraint conflicts with them. No authoritative correction
was located. [Exact evidence, paths and resumption requirements](../../docs/sysml-v2-runtime-blocker.md)
explain why dropping metadata or weakening the kernel would be unjustified.

[stage-3/results.json](stage-3/results.json) records the readiness command's actual
exit **1**, followed by passing refusal regression tests and generated-currentness
checks. Passing those regression tests establishes correct refusal, not a passed
runtime gate. No new language crate or altered runtime descriptor was emitted.

## Commit sequence

| Commit | Gate | Result |
| --- | --- | --- |
| `01ade26` | 0: generation guidance | Passed |
| `abf3e64` | 1: formal inputs/library content set | Passed; informative fixture unavailable |
| `ec2f04d` | 2: shared metamodel pipeline | Passed |
| Current blocker/evidence commit | 3: runtime closure | Blocked; no later stage attempted |

## Delivered scope and remaining work

Created the versioned SysML input lock and library-set manifest, SysML neutral IR,
structural audit, baseline/graph/audit generator modules, provenance/integrity tools,
regression tests, ADR 0007, contributor/status guidance and this evidence directory.
The existing generator was refactored; no new runtime crates were created.
Full hashes and byte lengths are in the [input inventory](../../standards/normative/sysml-2.0/README.md)
and [library manifest](../../standards/normative/sysml-2.0/library-set.json).

Exact SysML runtime class support: **none**. Importing 93 SysML metaclasses and
computing a 54-class dependency closure does not register them in a runtime.
No KerML source-project/library-ingestion prerequisites, SysML semantic rules,
SysML textual syntax or mixed-language acceptance/performance model were added.
Existing KerML descriptors, semantics and text remain operational. Generation 1
and all supplied/pinned historical bytes remain preserved.

Resolve the normative blocker and complete gates 3–7 before beginning the
repository/API/platform milestone. The informative OMG example must remain labelled
unavailable until its actual official bytes can be acquired and checked.

## Final verification of completed work — passed

All 11 commands in [final/results.json](final/results.json) returned zero, with
actual stdout/stderr logs alongside: workspace format, strict Clippy, all Rust
tests, offline generator currentness, strict Rustdoc for the five existing public
language crates and generator, local documentation checks, standards integrity,
TypeScript check, frontend build, Node engineering tests and browser end-to-end tests.
The runner set `CARGO_NET_OFFLINE=true` and `RUSTDOCFLAGS=-D warnings`.
Historical standards/browser reports were copied into this directory and restored
at their original paths. Original inputs and KerML generated bytes have no diff
from the fetched base. Final documentation/status and CLI-help wording checks are
recorded separately in `handoff/`.
