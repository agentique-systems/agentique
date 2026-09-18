# SysML generation 2 runtime gate: blocked on normative property metadata

Stage 3 is blocked; stages 0–2 are complete. No `agq-sysml` runtime crate, new
semantic rules, text lowering or application migration is claimed. The stop follows
the task's instruction to avoid inventing unsupported normative semantics.

## Reproduction and source evidence

Run the offline readiness gate (expected nonzero until the blocker is resolved):

```sh
cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-runtime --check
```

The [generated audit](../standards/generated/sysml-2.0/structural-audit.json)
records source-qualified identities, hashes, exact UTF-8 ranges and shortest
dependency paths. Its current `result` is `blocked`. The ordinary generator
`--check` verifies that import outputs and this report are current; it does **not**
assert runtime readiness. Regression tests assert that the readiness gate fails
without modifying outputs.

| Required property | Published evidence | Problem |
| --- | --- | --- |
| `Kernel-Associations-A_targetType_targetAssociation-targetAssociation` | [KerML.xmi](../standards/normative/kerml-1.0/KerML.xmi), lines 3613–3619; bytes 381360..382233; KerML.pdf PDF page 207, printed page 181, Figure 26 | `subsettedProperty` references this same property, in addition to the separate `association` subset. |
| `Systems-DefinitionAndUsage-A_analysisCaseOwningUsage_nestedAnalysisCase-analysisCaseOwningUsage` | [SysML.xmi](../standards/normative/sysml-2.0/SysML.xmi), lines 7685–7693; bytes 881706..883051; SysML.pdf PDF page 403, printed page 371, Figure 44 | Its only `subsettedProperty` reference points back to itself. |

The supplied PDFs repeat both self-subsetting annotations. They do not supply a
different target that could justify an importer correction. The relevant source
hashes are unchanged: KerML XMI `45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466`
and SysML XMI `caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f`.
The [PDF evidence record](../verification/sysml-language-v2/stage-3/pdf-evidence.json)
contains their existing hashes, page/figure locations and extracted local context.

The authoritative [UML 2.5.1 XMI](https://www.omg.org/spec/UML/20161101/UML.xmi)
constraint `Property-subsetted_property_names` requires differing names for a
property and each subsetted property. Self-subsetting cannot satisfy that constraint.
[MOF 2.5.1](https://www.omg.org/spec/MOF/2.5.1/PDF), clauses 14.1–14.2, uses UML
classes/associations for CMOF metamodel structure. Supporting acquisition hashes
and the short constraint body are recorded in
[UML evidence](../verification/sysml-language-v2/stage-3/uml-constraint-evidence.json).
UML was inspected as supporting evidence; it is not a new generator dependency.

## Why a smaller structural slice does not avoid the problem

Closure starts only with the authoritative PartDefinition and PartUsage IDs and
follows supertypes, declared properties, targets/owners, redefinitions/subsets,
associations and member ends. It yields 54 SysML classes, 50 KerML classes, 254
associations, three enums and 562 properties. Required primitive domains are only
Boolean and String. No read-only property, n-ary association or new numeric domain
causes this failure.

`PartUsage → ItemUsage → OccurrenceUsage → Usage → nestedAnalysisCase → its opposite`
reaches the SysML self-subset. `PartDefinition → ItemDefinition → OccurrenceDefinition
→ Definition → ownedConnection → ConnectorAsUsage → Connector → association →
Association → targetType → its opposite` reaches the KerML self-subset. The audit
contains the exact identity paths. Attribute, port and connection seeds produce
the same closure; dropping those optional capabilities cannot remove these edges.

The generic registry rejects the cycles with `PropertyCycle`. Its existing behavior
and all current KerML runtime descriptors are preserved. Neutral import may retain
inconsistent metadata without executing it; runtime structural registration is a
different gate. The JSON schema does not encode these association-owned ends or
their subset relationships, so a successful JSON comparison cannot resolve this.

## Decision and resumption requirement

Do not drop the edges, guess replacement targets from names, omit required
properties, or relax the kernel merely to accept this artifact. Treating the
self-subset as a no-op would preserve an extensional set tautology but would still
waive normative structural validation; there is no reviewed standards basis here
for that waiver. The PDFs corroborate the issue instead of disambiguating it.
No authoritative correction for these exact edges was located in the inspected
sources. This is not a claim that none exists.

Resume with an authoritative correction/erratum applicable to KerML 1.0 and SysML
2.0, or a separately authorized, explicitly non-conformant compatibility policy
that preserves raw evidence and identifies its deviations. A changed official
artifact must be pinned separately and reconcile ADR 0002's hash-qualified IDs;
do not replace historical bytes or quietly use preliminary 1.1/2.1 inputs.

Until then, stages 3–7 and the larger mixed-source performance fixture remain
unimplemented. Complete them before the repository/API/platform milestone. The
existing generation-1 application remains operational and separately verified.

## Follow-up, 2026-09-18: subset invariant corrected; independent blocker exposed

The preceding sections are the original Gate 3 discovery and remain historical
evidence. The resume task authorized reassessing the generic invariant using UML
2.5.1. [ADR 0008](adr/0008-property-subsetting-cycle-semantics.md) records the result:
subsetting is set inclusion and does not require general acyclicity. The kernel
now checks cycles only for redefinition and validates subset edges locally,
including upper bounds. Safe metadata traversal preserves reflexive/cyclic edges.

The two exact self-subsets remain unchanged through import and descriptor
translation. Source/hash/identity-qualified baseline diagnostics report their UML
naming violations. A third, separately reviewed nonreflexive `owningFeature`
naming anomaly is also explicit. No full UML conformance is claimed.

Gate 3 still fails, now on `definedFlow` redefining `Definition::ownedAction`:
the former has KerML Interaction as its context, which is not a specialization
of SysML Definition. Both the XMI and PDF Figure 22 retain this redefinition.
The revised [structural audit](../standards/generated/sysml-2.0/structural-audit.json)
contains the exact properties, source ranges, dependency path and context ancestry.
No guessed correction or redefinition waiver was applied. This is an independent
structural failure, not another subset-cycle rejection.

The [resume verification record](../verification/sysml-language-v2-resume/README.md)
includes a byte-preserved copy of the old audit. Gates 3 continuation and 4–7
remain unimplemented; Pack 2A is incomplete and Pack 2B must not start.
