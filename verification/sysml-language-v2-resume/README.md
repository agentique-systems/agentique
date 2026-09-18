# SysML language v2 resume: property semantics and Gate 3 reassessment

**PACK 2A INCOMPLETE — DO NOT START PACK 2B**

Fetched `origin/main`; worktree was clean and `main` matched
`736142f7f887252f83b44ed7c54fe540d138dbaf` (fetch exit 0, ahead/behind 0/0).
Created `foundation/sysml-language-v2-resume`. The original
[blocked-run record](../sysml-language-v2/README.md) and its logs are unchanged.
[Original audit](original-structural-audit.json) is a byte-preserved copy from
that base commit. The historical blocker document has an appended follow-up.

## Implemented correction

[ADR 0008](../../docs/adr/0008-property-subsetting-cycle-semantics.md) records the
direct UML 2.5.1 investigation and generic architectural correction. The old
kernel conflated `redefines` and `subsets` into an acyclic graph. Subsetting is
value-set inclusion and permits reflexive/cyclic representation. Redefinition
requires strict context ancestry; its cycles remain rejected. Local subset
context/domain checks remain and an upper-bound check was added. Lower-bound
narrowing applies to redefinition, not subsetting.

The unsupported requirement to preserve ordering/uniqueness monotonically across
redefinition was also removed after inspecting UML Property::isConsistentWith
and the effective collection API. Both source flags remain exact, and the
effective descriptor governs validated storage and reads. Type, multiplicity,
composition, ancestry, unknown references and replacement conflicts remain checked.

The exact KerML `targetAssociation` and SysML `analysisCaseOwningUsage` self-subsets
survive import and descriptor translation with their original UUIDs. The expanded
KerML Association closure also registers and exposes its self-edge through the
registry; forward and reverse traversal terminate. The complete SysML descriptor
set cannot yet register, for the independent failure below. No unsupported
runtime output is emitted and no SysML runtime test success is claimed.

[Policy v1](../../standards/baseline-anomalies.json) and the
[generated audit](../../standards/generated/sysml-2.0/structural-audit.json) report
the two self-subset naming violations and the separately inspected, nonreflexive
KerML `owningFeature` naming violation. Each disposition matches exact source,
hash, external IDs and descriptor UUID. The report is not a full UML conformance
audit. Synthetic new self-subsets and changed hashes/sources/versions/package
identities receive no automatic reviewed disposition.

Tests exercise ordinary, cyclic and reflexive subsetting; invalid context/domain/
upper bounds; genuine redefinition cycles; composition and multiplicity checks;
effective collection shape; finite, deduplicated diamond/cycle traversal; exact
source facts and descriptor identities; reviewed anomaly matching; stale audit
rejection and read-only CLI behavior. Derived-union evaluation remains deferred;
the tested forward/reverse metadata primitives are its bounded traversal foundation.

## Re-run Gate 3: still blocked for a different reason

The required closure remains 54 SysML classes, 50 KerML classes, 254 associations,
3 enums and 562 properties. Neutral import and descriptor translation support
this closure; atomic runtime registration does not.

`Systems-Flows-A_flowDefinition_definedFlow-definedFlow` redefines
`Systems-DefinitionAndUsage-Definition-ownedAction`. Its context is KerML
Interaction (the type of its opposite `FlowUsage::flowDefinition`); the base
context is SysML Definition. Interaction does not specialize Definition. The
required strict context check therefore rejects the redefinition. This is not a
cyclic-subsetting failure or a collection-shape restriction. The generated audit
contains exact source identities, byte ranges, context ancestry and dependency path.

The pinned SysML XMI lines 26–38 and 95–105 and PDF page 336 / printed page 304,
Figure 22 repeat these facts. See [PDF evidence](pdf-evidence.json) and
[UML evidence](uml-evidence.json). No inspected authoritative correction supports
a different target. Guessing a target, omitting metadata, changing the pin, or
waiving redefinition-context validation would bypass the requested gate.

Accordingly Gate 3 continuation and Gates 4–7 did not start. Runtime descriptor
scope remains the existing 29-class KerML Root/Core slice; no new runtime crates,
KerML project/library prerequisites, SysML semantic rules or SysML syntax were
published. There is no new source-to-model acceptance result. The official Simple
Vehicle example remains unavailable as recorded in the original input lock.
No alternate fixture is represented as official. No Pack 2B work was undertaken.

## Verification

The runner writes actual commands, stdout/stderr and exit codes to gate directories.
Run from the repository root:

```sh
node verification/sysml-language-v2-resume/run.mjs foundation
node verification/sysml-language-v2-resume/run.mjs stage-3
node verification/sysml-language-v2-resume/run.mjs final
```

`foundation` verifies the implemented generic correction. `stage-3` records the
readiness command's real exit 1 separately from successful refusal/currentness
regressions. A passing regression asserting refusal does not pass Gate 3.
`final` covers all requested workspace, standards and frontend/browser commands
and strict Rustdoc for all five existing generation-2 public crates and the
generator. There are no new public crates to document.

Results: [foundation](foundation/results.json), [runtime gate](stage-3/results.json),
[final checks](final/results.json). The runner uses `CARGO_NET_OFFLINE=true` and
`RUSTDOCFLAGS=-D warnings`; standards/browser artifacts are copied here and their
historical files restored. The pinned PDFs/XMI/JSON, library bytes, old coverage
and traceability records, and KerML generated artifacts are unchanged from the base.

## Remaining obligations

Resolve the independent redefinition-context failure before emitting the SysML
runtime and proceeding through the original gates. Full SysML registry visibility
of the second self-subset and its generated borrowed views remain blocked; raw
translation preservation alone does not satisfy that runtime acceptance criterion.
All multi-document/mixed-language projects, package/import/inherited lookup,
generation-2 standard-library ingestion/provenance, required multiplicity/visibility
support, SysML rules, lossless text and end-to-end immutable-revision acceptance
remain due. Derived unions, complete conformance and incremental evaluation remain
explicitly outside the implemented metadata correction.
