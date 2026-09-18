# ADR 0011: Structural registration and metamodel conformance

Status: accepted for implementation, 2026-09-18. Supersedes ADR 0010's use
of the authority conflict as an unconditional runtime registration blocker.
ADR 0010 and all earlier failed gates remain historical evidence.

The registry represents descriptors; it is not a general UML/CMOF certification
engine. Normative capture, structural integrity, runtime interpretation and
authoring conformance are separate contracts. An error in the last contract
does not imply failure of the first three.

## Exact source finding

The [v3 reproduction](../../verification/language-core-completion-v3/stage0/results.json)
reproduces ADR 0010's findings and both pre-change complete runtime failures.
The pinned SysML edge is `Systems-Flows-A_flowDefinition_definedFlow-definedFlow`
(`2abb2284-8e25-51ac-b486-1792cc60e1b1`) redefining
`Systems-DefinitionAndUsage-Definition-ownedAction`
(`2e4efe58-2d09-5275-991e-104649d59bf3`). Both endpoints, their owners,
opposites, types, bounds, order, derivation and source evidence are representable.
The redefining end is association-owned, derived and non-navigable. Its owning
association has no parent. Its navigation context is Interaction, which has no
Definition ancestor. FlowUsage's conformance to ActionUsage does not repair this.

Current [OMG issue records](https://issues.omg.org/issues/spec/SysML/2.0),
[resolved SysML issues](https://issues.omg.org/issues/spec/SysML/2.0/fixed),
[resolved KerML issues](https://issues.omg.org/issues/spec/KerML/1.0/fixed),
and [release material](https://github.com/Systems-Modeling/SysML-v2-Release/releases/tag/2026-08)
were inspected. No correction for this exact pair was found. Acquisition bytes,
URLs, timestamps and hashes are retained in `verification/language-core-completion-v3/research/`.
The release's KerML 1.1/SysML 2.1 Beta 2 resolutions are explicitly preliminary;
they do not replace the pinned 1.0/2.0 runtime baseline.
[UML24-85](https://issues.omg.org/issues/UML24-85),
[UML24-86](https://issues.omg.org/issues/UML24-86) and
[UML24-96](https://issues.omg.org/issues/UML24-96) remain resolved, with the
scopes described in ADR 0010. Final UML Property authoring constraints remain
applicable diagnostics; no invented generalization or repaired type is justified.

The original Systems Library identifies the same 20250201 metamodel and retains
`FlowUsage::flowDefinition : Interaction`; it provides no replacement association
hierarchy. Original library bytes and PDF Figure 22 remain unchanged.

## Runtime interpretation

Raw redefinition is always queryable. Class slot replacement follows only
class-owned to class-owned edges with strict owner ancestry, compatible domain,
contained multiplicity and preserved composition. Traversal never crosses an
association-owned end. Association inheritance is a separate descriptor graph.
Its inherited ends may be replaced only along an actual association ancestry and
compatible end contract. Mixed ownership does not manufacture a class slot.

The questioned edge is unnecessary for class effective properties, authored
storage or navigation: it is not a class declaration or an authored navigable
surface. It remains metadata. A future algorithm explicitly requiring its
replacement meaning must return unsupported/incomplete with the edge as evidence.
No automatic derived evaluator is inferred from `redefines` or `isDerived`.
A future inverse-navigation derivation can depend on `FlowUsage::flowDefinition`
and the exact opposite incidence without treating this metadata edge as a
replacement for a Definition slot. The generic overlay can hold that independently
justified result; the disputed replacement meaning is not required to store it.

## Integrity and diagnostics

Registration rejects duplicate typed identities, dangling/wrong-kind references,
unresolved domains, malformed bounds, incoherent member/opposite/owner identities,
and cyclic classifier/association inheritance. These prevent safe graph exposure.
Typed identity namespaces remain distinct; equal numeric payloads in different
ID types are not collisions. Generated source keys must map injectively within
their typed identity namespace.

Authoring name conflicts, questionable redefinition/subsetting contexts,
inconsistent replacement contracts and raw property relation cycles are retained
and diagnosed. Names never identify slots; visited-set traversal terminates.
An unsafe or ambiguous effective-property computation fails explicitly at that
query boundary, including when invoked by model validation. No invalid edge is
silently used to suppress an inherited slot. Safe raw graph access remains possible.

Diagnostic rule, severity, subject, related identities, source and disposition
are typed data. A reviewed baseline anomaly retains error severity. Its review
requires exact artifact/version/hash, both external IDs and both descriptor IDs;
future artifacts inherit nothing. Strict conformance rejects errors including
reviewed errors. Runtime readiness requires complete structural registration and
the implemented runtime contracts, independently of conformance certification.

The migration is safe because the affected authoring predicates neither allocate
identity nor establish graph referential integrity. Their operational consequences
are checked separately before property replacement or storage. This is a division
of responsibilities, not permission to ignore an anomaly.


## Migrated predicates and why retaining the graph is safe

| Predicate | Representation and runtime obligation |
| --- | --- |
| Duplicate class/property/literal display names | Names are not identity. Raw descriptors and exact slots remain distinct. Ambiguous name queries return a typed conflict. |
| Subsetting names/context/domain/upper bound | All endpoints are resolved. Raw subset edges are never executed as assignments. Finite contributor traversal returns unsupported relations as evidence; no invalid inclusion computes a value. |
| Property redefinition context/domain/bounds/composition | Raw metadata remains intact. Incompatible class-to-class replacement makes affected effective queries fail; mixed ownership never enters the class replacement graph. Association interpretation independently checks association ancestry and contract. |
| Raw property redefinition cycles | Graph storage is finite. Traversal uses visited sets. Strict owner/association ancestry prevents a cycle from becoming an accepted replacement algorithm. |
| Derived-union flag inconsistency | Flags remain representable. An affected class cannot be used by effective-property/model validation. No authored union evaluation is inferred. |
| Opposite type versus owning class | Incidence/opposite identities remain coherent. Every concrete reference validates both its target type and source context against the opposite type. A questionable authoring type never licenses a corrupt link. |
| Several effective properties redefine one ancestor | All surviving exact IDs remain available. An ambiguous inherited alias returns `PropertyConflict`; no arbitrary replacement is selected. |

See [the runtime contract](../language-core-runtime-contract.md) for numeric,
association-storage, derived-state and context boundaries. Additional local subset
context/contract diagnostics from the full SysML graph remain unreviewed; v3 does
not automatically grant them the exact `definedFlow` disposition. The baseline
review is not a general waiver for the specification or the rule.
