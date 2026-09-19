# ADR 0016 — Operational validation semantics

Status: implemented for explicit Operational KerML 1.0/v4. Library semantic
publication is still unaccepted; see the independent KLCV6-F-001 authority stop.

## Decision and authority

The project authorizes precisely the KERML11-68 validation correction independently
verified in official pilot commit `d9231d21e621aeabeafa92aef026b3929c859116`.
The [evidence packet](../../verification/kerml-semantic-closure-v6/authority-packet.json)
includes the real Git commit object, its parent-to-commit diff, all changed test
files, exact before/after validator sources, pinned specification hashes, the open
OMG issue, current reference implied XMI, and arbitrary-name canonical tests.
Verification checks the Git object and blob identities, not just the commit title.

KERML11-68 remains open. This is an Agentique operational interpretation of KerML
1.0, not an adopted final KerML 1.1 correction. The captured preliminary revision
is corroboration only.

| Profile | Cumulative corrections |
| --- | --- |
| Published | None |
| Operational v1 | KERML11-81 descriptors |
| Operational v2 | v1 plus KERML11-140 resolution |
| Operational v3 | v2 plus reviewed KERML11-76 canonical library corrections |
| Operational v4 | v3 plus KERML11-68 validation semantics |

## Exact rule

For a non-end Feature redefining an end Feature, v4 reports an invalid
`validateRedefinitionEndConformance` when the redefining Feature's **owningType**
conforms to Association or Connector. Otherwise this constraint passes. Metaclass
conformance includes subtypes, such as AssociationStructure and BindingConnector.
The owning Type is derived from the owning FeatureMembership; an outer lexical
owner or a Type that merely inherits the Feature does not replace it.

Published and v1–v3 retain the unconditional published implication. Missing
endpoints or flags remain incomplete. No profile globally disables the constraint.
The result retains both end flags, actual owning Type and metaclass, disposition,
profile identity, manifest digest, rule path, positive facts and search evidence.

The [v4 manifest](../../standards/kerml-1.0-operational-validation-errata-v4.json)
is frozen by SHA-256
`525bae8868e6f1ce2705eb707560ef5597dc50ca3a810070e08717b368605953`.
It extends v3 without changing v3's identity or manifest bytes. Because v4 adds no
model transform, v3 correction facts retain their v3 origins and deterministic
IDs. Context validation recognizes only this explicit successor relationship.
A corpus regression compares every v3/v4 canonical record and source origin.

Pinned sources, metamodels, KPARs, stored end flags and source lowering are
unchanged. This validation correction does not supply missing feature-chain
structural semantics.

## Publication and metadata boundary

`BaselineProfile::OPERATIONAL_V4` is explicit. The historical `OPERATIONAL`
default remains v2 pending a separate accepted-publication decision. SourceProject
retains the chosen profile for its history, and its queries expose that identity
and manifest digest. SemanticContext and query proof identities carry the same
authority. A future accepted library publication and repository/API representation
must persist both values; no repository/API implementation is introduced here.

An ordinary Snapshot's structural acceptance cannot establish language-semantic
acceptance. No accepted binding manifest, `LoadedKermlStandardLibraries`, or
authored dependency on accepted libraries is issued in this change.

## Independent authority boundary

The broader constraint inventory independently reproduced
[KLCV6-F-001 / KERML11-205](../kerml-subobject-specialization-authority-conflict.md).
V4 does not authorize choosing that constraint's conflicting required target.
This is the reason for the new authority stop. Remaining naming, inheritance,
feature-chain, expression, validation and integration gaps remain implementation
obligations; none is reclassified as execution or used as the stop reason.
