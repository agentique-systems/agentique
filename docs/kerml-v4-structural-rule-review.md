# KerML v4 structural rule review

These ordinary KerML 1.0 implementation changes are distinct from the
[operational KERML11-140 algorithm](operational-kerml-profile.md). They apply
under all three explicit profiles. `agq-kerml-query/11` identifies the final
query rules, proof construction and dependency behavior; changing the rule set
invalidates earlier answers independently of the profile identity.

The captured [structural authority](../verification/kerml-name-resolution-errata-publication-v4/structural-authority.json)
contains the actual formal PDF text for inherited membership, redefinition,
end ordering and relevant structural constraints. The normative metamodel XMI
remains the authority for property identities and operation bodies.

Implemented behavior includes inherited redefinition suppression across multiple
generalizations, imported membership identity, original-type membership under
conjugation, final feature-chain target inheritance, inherited end ordering for
positional redefinition, and binary-association library specialization. Original
elements and memberships remain canonical; inherited features are not copied.
CrossSubsetting's source is derived from its actual owner. Incoming target
references do not manufacture another source, and explicit incomplete or invalid
derived state remains visible.

Plain nonowning Membership names are optional stored values. Only owning
memberships derive names from their elements. An unnamed Feature follows the
first owned Redefinition for its effective name, without merging every target's
name. When no explicit Redefinition exists, a single required implied positional
redefinition establishes the name; multiple unordered implied targets remain
incomplete. That remaining limitation is not hidden by an arbitrary ID order.

Named expression declarations may inherit their result membership. Construction
therefore does not add an unnecessary unnamed local result to every such
declaration. Required local results for structural expression syntax remain
canonical owned features, including cast results. Synthetic result ownership
marks the owning element's implied-relationship inclusion flag.

`validate_namespace_distinguishability` checks effective membership names and
metaclass comparability. `validate_local_structure` records every constraint it
actually evaluates, including relevant feature flags, local chain arity,
conjugator count, self/count restrictions on union/intersection/difference,
result membership and feature-reference structure. This is a partial structural
validation inventory. Remaining structural, derived and implied relationship
constraints are explicitly mandatory before acceptance. The raw constraint
inventory is evidence for continued work, not a completed conformance claim.

The new distinguishability check exposed
[KNRV4-F-001](kerml-pinned-objects-authority-conflict.md). An ordinary kernel
Snapshot passing its storage and lower-bound checks would still require the
semantic quality gate. No ConstructionView is exported as an accepted library,
and no success of construction is equated with semantic verification.

## Authored project reproducibility

`SourceProject::with_profile` selects the interpretation for the project's
entire history. `baseline_profile()` exposes that explicit metadata. For example:

```rust
use agq_kerml::BaselineProfile;
use agq_kerml_text::SourceProject;

let published = SourceProject::with_profile(BaselineProfile::PublishedKerMl10)?;
let v1 = SourceProject::with_profile(BaselineProfile::OPERATIONAL_V1)?;
let v2 = SourceProject::with_profile(BaselineProfile::OPERATIONAL_V2)?;
```

`SourceProject::new()` chooses operational v2 and still stores that exact
profile explicitly. Each query context includes the profile ID, reviewed
manifest digest, descriptor digest and rule-set version alongside model and
source/dependency identities. The
[authored profile matrix](../verification/kerml-name-resolution-errata-publication-v4/authored-3/results.json)
uses identical source text under all three profiles and verifies the differing
redefinition results. This establishes profile selection, not integration with
an accepted standard-library Snapshot; that integration remains blocked.
