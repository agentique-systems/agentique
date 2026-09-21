# ADR 0020: Operational owned-cross-feature correction

Status: Accepted for the narrowly authorized profile correction; library semantic
publication remains unaccepted.

Date: 2026-09-20

## Authority and decision

Operational KerML 1.0/v7 extends immutable v6 with exactly KERML11-1's reviewed
`Feature::ownedCrossFeature` selection. The default remains operational v2.
The [correction manifest](../../standards/kerml-1.0-operational-owned-cross-feature-errata-v7.json)
and [profile manifest](../../standards/kerml-1.0-operational-profile-v7.json) identify
the correction, its predecessor, and the frozen authority packet.

The pilot commit is `553cf8205c19241c9127ab264f8372f5b58d3895`, parent
`c1a76cebcff41f7a4bd105e48f8aaccbc446d835`. The offline verifier checks actual Git
commit/blob objects, the complete before/after implementation, the exact two-file
diff, and the added regression. It independently applies the reviewed source
delta to the complete original file. Its conclusion does not depend on the title.
The current pilot and all 36 current reference XMI blobs are checked separately.
KERML11-1 is open, updated 2026-04-21 01:22 GMT; project authorization and pilot
corroboration do not imply final OMG adoption. Preliminary material is retained
only as corroboration. KerML 1.0 remains the normative baseline.

Selection applies only to an end Feature with an owning Type. In ordered
`ownedMembership` projection, choose the first Feature that is neither a
Multiplicity, MetadataFeature nor BindingConnector, and whose owning Membership
is neither a FeatureMembership nor a FeatureValue. Exclusions use metaclass
conformance, including subclasses. Authored, implied and correction origins are
not additional exclusions. Incomplete ownership/population evidence returns
Incomplete with no selected ID. No candidate means none, without manufacturing
an owned cross Feature. Imported and inherited members cannot enter this owned
projection. Member IDs, insertion order and map order do not establish selection.

`owned_cross_feature`, `is_owned_cross_feature`, `owned_cross_subsetting`, and
`cross_feature` remain separately explainable queries. The last reads the second
Feature of the actual crossed chain; it does not replace a missing CrossSubsetting
with the selected owned Feature. Profile and manifest identity participate in
SemanticContext and proof identity. Existing v9 producer IDs retain their frozen
rule seed, so a query-version change does not rewrite historical derived IDs.

## Integration defect and ordinary support

A regression test showed that v9's contextual FeatureValue result was itself
owned directly by the end via an OwningMembership. The exact corrected selector
would legitimately select that new Feature. V7 owns that infrastructure through
its consuming Subsetting or ReferenceSubsetting. The selector's reviewed
exclusions are unchanged, and v6's graph remains reproducible. FeatureValue
Expression ownership and featuring obligations are not relaxed.

V7 also implements the published initial-value `that.startShot` binding context.
The chain's identity and evidence depend on its semantic operands, so separate
subject batches and reversed processing order produce the same graph. No pinned
FeatureValue has `isInitial=true`; the branch is verified with canonical fixtures.
Default values remain distinct from non-default binding obligations. No value is
executed or evaluated.

Other ordinary work includes canonical typing closure excluding CrossSubsetting,
mandatory metaclass library bases in that closure, the specific published
Occurrence snapshots domain, and separate feature-target/value-owner/result
queries. V2 target resolution now includes required implied general scopes before
optional redundancy removal, as the reviewed pilot resolution phase does. This
allows a renamed inherited member to retain an independently reachable original
name through a required general; it adds no aliases or lexical fallback on an
incomplete result.

Result producers can run in bounded subject batches against one immutable
SemanticContext. Proposed derivations are not storage or accepted publications.
Query answers now share immutable origin values within each evaluator instead
of repeatedly copying their derived dependency sets. `fact_origins` exposes
`Arc<Origin>` values; origin content, debug encoding and equality are unchanged.
The full dependency closure remains in every answer. Repeated, independent and
concurrent query regressions check complete answers, without timing thresholds.
The kernel also exposes recorded search evidence by fact key. Query proof
expansion uses this indexed read instead of scanning every recorded search for
every dependency. A direct comparison with the complete iterator preserves both
positive and empty lookup results; no search evidence is discarded.
Merging rejects disagreeing duplicate facts or ownership sequences before
mutation. Materialization requires the identical validated Snapshot. Construction
obligations cannot be bypassed by handing the plan a different Snapshot.

## Authority sweep and publication boundary

V10 traverses all 258 named constraints, 218 derived/operation inventory members,
and 410 retained issues. It continues past new conflicts and records independent
proofs in one [blocker register](../../standards/kerml-1.0-operational-authority-blockers.json):

- KLCV10-F-001 / KERML11-2: the pinned unary surroundingSpace end requires a
  crossing that fails the greater-than-one-end constraint.
- KLCV10-F-002 / KERML11-4: multiplicity featuring is required to be both empty
  and nonempty by two applicable structural rules.
- KLCV10-F-003: on a legitimate binary cross Feature, the literal
  `otherEnds->excluding(self)` removes the cross Feature rather than its owning
  end. It selects the Cartesian-product branch, unlike the corroborated binary
  opposite-end domain. No matching issue was found in the retained tracker.
- KLCV10-F-004 / KERML11-75: five public NumericalFunctions memberships
  imported into VectorFunctions collide with owned memberships. Published OCL
  includes them, while the published prose excludes them. Current pilot code
  corroborates the OCL; no import correction is authorized.

These are independent structural proofs, not failures attributed to unsupported
execution. No correction beyond v1-v7 is adopted. The inventory traversal is
complete, but full per-antecedent applicability proof closure is not claimed.
The supplemental review checks all 23 reference IndexExpression guards, both
constructors' inherited default candidates, and the exercised Transfers Flow
cycle. Those findings do not themselves authorize corrections.
The additional scope review reads all 63 remaining issue records, preserving
the distinction between structural proofs, structural evaluability, runtime
interpretation and proposed extensions. It does not use issue categorization to
mark unfinished structural constraints as deferred execution.

Ordinary structural implementation is still incomplete. Corpus producer execution,
kernel storage validity, a materialized partial overlay, closed structural
coverage, and accepted semantic publication are separate facts. No accepted
bindings or `LoadedKermlStandardLibraries` facade are issued. No SysML semantics,
execution, simulation, persistence or application migration is included.

## Accepted-library integration design

An eventual accepted publication must have a private acceptance constructor that
checks exact v7 profile/manifests, library-set identity, complete reference and
derived-fact evidence, distinguishability, all applicable structural constraints,
closed coverage and zero unresolved authority blockers. Its identity must combine
the validated Snapshot, overlay digest, rules, correction manifests and pinned
library identities. Kernel `Snapshot` alone is insufficient evidence.

`LoadedKermlStandardLibraries` will borrow/share that accepted publication and its
bindings, source map and diagnostics. It must not copy semantic records. Binding
creation must validate the accepted Snapshot, profile, library origin, complete
ownership path, metaclass, visibility and uniqueness. SourceProject dependencies
will share immutable libraries while authored revisions retain their own roots
and shadowing rules. A profile mismatch must fail before reference lookup.

The current tests establish v7 authored profile metadata and immutable historical
contexts, plus deliberately non-accepted canonical producer fixtures. They do not
claim the full accepted-library integration/facade test matrix is implemented.

See the [v10 evidence index](../../verification/kerml-semantic-closure-v10/README.md)
for actual commands, failures, rechecks, corpus measurements and remaining gates.
