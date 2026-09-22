# ADR 0023: Operational v9 multiplicity context

Status: authorized for implementation. Canonical library acceptance is a separate gate.

Operational v9 extends v8 with exactly two explicitly authorized Agentique
interpretations: KERML11-4 and KERML11-3. These are not adopted KerML 1.0
corrections. Published through v8 retain their existing semantics and authority
identities. The default operational alias stays at v2 until canonical publication
is accepted; adding a profile is insufficient evidence to move that alias.

The pinned `checkMultiplicityTypeFeaturing` uses `owningType`. A Multiplicity is
owned through an OwningMembership, so the retained v8 interpretation produces an
empty domain for ordinary source multiplicities. The pinned
`checkMultiplicityRangeExpressionTypeFeaturing` requires the bound Expressions
to have the same domain. In `Transfers::Transfer::instant[instantNum]`, the
referent is featured by `Transfer`, and v8 cannot construct the required
reference-result binding. Independent full-scan and worklist fixtures retain
that historical result.

V9 uses one semantic operation to select the multiplicity context:

1. Read its semantic `owningNamespace`.
2. A non-Feature owner contributes no featuring types.
3. An ordinary Feature owner contributes its featuring types.
4. An owned cross Feature contributes the featuring types of its owning end
   Feature. Missing or incomplete semantic premises remain incomplete.

Each bound Expression inherits the resulting MultiplicityRange context. Nested
expression queries compose existing featuring rules. This is structural; it
does not evaluate bounds, introduce lexical namespace fallback, or choose a
domain using ElementId order. Multiple featuring types retain the existing
intersection semantics and query completeness contract.

[KERML11-4](https://issues.omg.org/issues/KERML11-4) describes the ownership
correction. [KERML11-3](https://issues.omg.org/issues/KERML11-3) explains why a
cross multiplicity bound needs the containing Association or Connector context.
Both issue pages still display open status at capture. The reference
implementation's [2026-07 release notes](https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation/releases/tag/2026-07)
report KERML11-3 approval on KerML 1.1 RTF Ballot 3; that approval does not change
the pinned published 1.0 baseline.

The reference implementation at commit
`5cca16d846016e62bb1e54e0e50e675254a022ef` reads `owningNamespace` in
`FeatureAdapter.addImplicitFeaturingTypesIfNecessary`, called for multiplicities.
`ExpressionAdapter.addImplicitFeaturingTypesIfNecessary` explicitly selects the
owning end domain for a cross multiplicity bound. Agentique's authorized
interpretation applies that domain consistently to the MultiplicityRange and
its bounds. The reference is corroboration, not a replacement specification.

The [authority record](../../verification/summaries/kerml-v9-publication/authority-decision.json)
pins exact formal rules, reference source identities, current issue captures,
historical impacts, and ordinary/cross corpus witnesses. KERML11-2 remains a
visible validation-only conflict. Earlier finding registers are retained.

Acceptance requires complete producer closure, zero publication capability
findings, complete and endpoint-correct mandatory references, and the sealed
canonical standard-library facade. Conformance breadth remains separate. After
acceptance, work proceeds to SysML semantics over the same kernel graph.
