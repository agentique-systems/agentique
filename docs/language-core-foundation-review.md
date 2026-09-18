# Language core foundation review, 2026-09-18

**LANGUAGE CORE STRUCTURAL FOUNDATION INCOMPLETE — DO NOT PROCEED**

Fetched base: `76aef699ee7b2d4f910c7483e42340998770ac5c`; clean worktree;
branch `foundation/language-core-completion-v1`. No `agq-sysml` runtime crate
exists. No new runtime language descriptor is emitted by this work.

## Decision

[ADR 0009](adr/0009-property-redefinition-context.md) documents a Category F
authority gap after reproducing the original failure. UML24-85 recognizes mixed
ownership but neither its inspected resolution summary nor final UML 2.5.1
establishes the intended replacement for the actual `definedFlow` edge. The
final publication lacks the proposed Property context override; the actual
associations have no generalizations. Interaction does not inherit Definition.
An authoritative interpretation of this exact case is required before changing
the invariant. This is not a claim that every mixed-ownership redefinition is
invalid, nor a proof that the existing kernel rule is generally correct.

Gate 0 has a reproducible diagnosis but its authority question is unresolved.
Gate 1's invariant correction and ten required generic regression cases are
**not implemented**. The additive full-audit tools below continue investigation
without accepting a new rule. Stages 3–10 implementation and acceptance remain
unperformed; Stage 11's review concludes incomplete. No Pack 2B work began.

## Complete diagnostic inventory

| Source | Classes | Properties | Associations | Enumerations |
| --- | ---: | ---: | ---: | ---: |
| KerML 1.0 | 82 | 313 | 131 | 2 |
| SysML 2.0, excluding KerML | 93 | 402 | 188 | 5 |
| Combined | 175 | 715 | 319 | 7 |

All 319 associations have exactly two member ends. The
[KerML report](../standards/generated/kerml-1.0/full-audit.json) and
[combined report](../standards/generated/sysml-2.0/full-audit.json) enumerate
every end, ownership, navigability, multiplicity, ordering, uniqueness,
derivation, opposite and association ancestry. Descriptor inventory does not
certify canonical association storage or writes from either endpoint.

The complete manifests expose exact IDs, domains, source-qualified references,
source attributes and byte ranges. A separate Python XML reader verifies their
complete classifier/property/literal sets and direct structural facts against
original XMI, including UUID-v5 identity encoding. It does not invoke the Rust
importer or read neutral IR as authority. It does not independently certify
effective inheritance, storage or retained operation execution.

The generator attempts translation of the entire selected descriptor graph.
Both full translations stop on unsupported association inheritance; therefore
**atomic registration is not reached**. Reports separately enumerate known
translation gaps so that this first failure does not hide primitive domains or
the context investigation. No filtered subset is registered as a replacement
for the complete graph. This is not a complete UML well-formedness checker.

| Category | Exact finding | Disposition |
| --- | --- | --- |
| C | KerML `Kernel-Expressions-LiteralInteger-value`, PrimitiveTypes Integer | Unsupported; never mapped to historical i64. |
| C | KerML `Kernel-Expressions-LiteralRational-value`, PrimitiveTypes Real | Unsupported; no implicit f64 approximation. |
| D | KerML `Kernel-Interactions-A_participantFeature_Interaction` inherits `Kernel-Connectors-A_participantFeature_Association` | Source inheritance retained; generic runtime association inheritance still needed. |
| E | Five same-name subset edges | Exact source/hash-qualified diagnostic dispositions; edges retained. |
| F | SysML `Systems-Flows-A_flowDefinition_definedFlow-definedFlow` redefines `Systems-DefinitionAndUsage-Definition-ownedAction` | ADR 0009; no source rewrite, inferred hierarchy or waiver. |

No A importer defect is established by the direct-XMI comparison. No B kernel
correction is claimed established by this investigation. The missing association
shape is classified once as D, rather than counted again under B.

## Domain and derived-property boundaries

Only Boolean, String, Integer and Real are referenced as external primitive
domains by abstract-syntax properties: 37, 27, 1 and 1 references respectively
in the combined graph. No property references Natural, UnlimitedNatural or a
URI primitive. UML LiteralUnlimitedNatural nodes encoding multiplicity bounds
are authoring syntax, not extra modeled property domains. Library-defined
semantic value types for eventual user models are a separate subsystem.

Integer and Real require a deliberate value contract before runtime expansion.
No proposed decimal/floating-point representation is silently treated as the
mathematical real domain. Existing i64 is historical storage only.

Full reports inventory derived unions with direct/transitive subset contributors,
visited-set traversal and a `not-computed` value state. These are tooling graph
facts, not an implemented generic evaluator or the requested runtime evidence/
dependency contract. Ordered and nonunique union materialization stays open.
Existing overlays distinguish missing derived values from computed empty values;
the full requested absent/not-computed/incomplete/invalid contract is uncompleted.

## Additional exact naming anomalies

The full KerML scan found two previously unselected association-owned properties
named `multiplicity`, both subsetting the different property also named
`multiplicity`, `Kernel-Multiplicities-A_bound_multiplicity-multiplicity`.
The violated rule is UML `Property-subsetted_property_names`, not graph acyclicity.

| External ID | Descriptor UUID | XMI byte range | Start line |
| --- | --- | --- | ---: |
| `Kernel-Multiplicities-A_lowerBound_multiplicity-multiplicity` | `e0163b2b-16c8-56bd-bec9-12006912c80b` | 297134..297866 | 2798 |
| `Kernel-Multiplicities-A_upperBound_multiplicity-multiplicity` | `7058090a-4533-5e37-9d92-874215b70b98` | 298229..298961 | 2809 |
| `Kernel-Multiplicities-A_bound_multiplicity-multiplicity` (target) | See full manifest | 299304..299993 | 2820 |

All three are derived, 0..1, unique, unordered, noncomposite ends typed by
MultiplicityRange. Each of the two exact subset targets and same names was
inspected directly in XMI. The new dispositions in
[the anomaly register](../standards/baseline-anomalies.json) apply only to
KerML 1.0 artifact `https://www.omg.org/spec/KerML/20250201/KerML.xmi`, SHA-256
`45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466` and those
exact source/target IDs and UUIDs. Neither inherited releases nor other edges
gain a disposition. All three earlier policy entries remain unchanged.

## Required completion questions

| Question | Answer |
| --- | --- |
| 1. Every KerML metaclass/property/association represented at runtime? | **No.** Full raw inventory exists; numeric domains and association inheritance block translation. |
| 2. Every SysML metaclass/property/association represented at runtime? | **No.** No runtime crate; dependencies and the context authority gap block it. |
| 3. Cross-metamodel references preserved exactly? | **Yes in raw manifests**, independently checked; complete runtime preservation is unproven. |
| 4. Remaining blanket exceptions? | No new exceptions; no claim of full UML validation. Readiness refuses unsupported constructs. |
| 5. Name-based descriptor hacks? | No new identity or runtime name exceptions. Existing frozen identity v1 is preserved. |
| 6. Source anomalies silently normalized? | No. Five naming anomalies are explicit, hash-qualified and unchanged. Context interpretation stays unresolved. |
| 7. Normative primitives narrowed? | No. Unsupported Integer/Real fail translation; historical i64 is not substituted. |
| 8. Association occurrences without duplicate writable truth? | Existing supported storage retains one truth. Full association storage/navigation audit and endpoint writes are uncompleted. |
| 9. Derived/uncomputed states distinguishable? | Existing overlays distinguish uncomputed and computed empty; complete five-state runtime contract remains due. |
| 10. Immutable snapshots sole canonical declared state? | Yes; this work adds only maintenance-tool reports, not another model graph. |
| 11. Typed views without duplicate semantic data? | Existing views unchanged. Full borrowed views not generated. |
| 12. Existing queries safe against full registry? | **Not established.** Context still requires the exact bounded registry; no full registry exists to test. |
| 13. Known gaps explicitly recorded? | Yes, here, in full reports and machine-readable coverage. This is not exhaustive conformance certification. |

The requested synthetic runtime stress matrix remains a later obligation;
only audit selection, identity retention, full-readiness refusal, report
currentness and read-only failure behavior are newly tested. Existing language,
snapshot and semantic suites are run at final verification within their original
scope. Passing them cannot certify the unavailable full registry.

## Verification and resumption

Actual commands, complete logs and exits are in
[the verification record](../verification/language-core-completion-v1/README.md).
`--require-runtime` now evaluates complete selected-language input for both
baselines; a successful bounded KerML generator no longer certifies full readiness.
`--audit-full --check` verifies diagnostic reports, including blocked reports;
its success is not a runtime gate pass.

Resume with the specific external authority named in ADR 0009, establish the
generic Property rule and its regression matrix, then complete all remaining
runtime/value/association/derived-state/context/stress gates. Complete reports
must be rerun after every correction. Do not proceed to SysML semantics or text,
platform APIs, repository, visualization or execution.
