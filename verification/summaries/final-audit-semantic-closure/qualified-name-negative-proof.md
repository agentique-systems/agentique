# Qualified-name sibling exclusion: authority and compatibility review

Reviewed source: `ffdf5f7c04f67b8afb0f92ef324f216de06328df`.
The first actual accepted-v3 self-model run stopped in the independent
programmatic fixture, in the Complete assertion for
`effective_qualified_name(acceptedRevision)`. Its one naming diagnostic concerns
derived sibling `aa40c9c1-45a2-555b-ae91-f523c140283c`, whose complete effective
name population is `{"snapshots"}`. `acceptedRevision` itself is element 910079
and has an explicitly stored declared name. This is not a missing fixture name.

`unique_qualified_segment` previously required full-name selection for every
owned sibling. Its inherited-name selection limitation therefore made this
unrelated sibling block an otherwise explicit ownership-qualified path.
Selecting whether `snapshots` is a full or short name is unnecessary to prove
that it cannot be the requested full name `acceptedRevision`.

## Exact authority

Pinned KerML 1.0 XMI, `Root-Elements-Element-deriveElementQualifiedName`
(lines 9089–9107), compares the element's name against the names of the
owning namespace's owned members. It does not require selecting unrelated names.
`deriveElementName` uses `effectiveName()`. `Feature::effectiveName`, XMI lines
6049–6062 and KerML.pdf page 188 (printed page 162), returns declaredName whenever
either declaredName or declaredShortName is present. Only when both are absent
does it follow namingFeature. `effectiveShortName` has the same stopping condition.
The PDF cross-check was independently performed by the structural reviewer.

Consequently the frozen KerML evaluator's complete full/short-name union is an
upper bound on possible full names. A short-only declaration does not inherit a
different full name: its effective full name is null. The regression where short
name `w` redefines feature `engine` therefore correctly excludes that sibling
when searching full name `engine`. Asking for the short-only feature's own full
qualified name retains the existing conservative Incomplete result.

Authority SHA-256 values:

- `standards/normative/kerml-1.0/KerML.xmi`:
  `45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466`
- `KerML.pdf`:
  `3bcc96f989bfa9d05cd28e026df3351b795fe8d494187b87bff3db7d96373697`

SysML namingFeature overrides remain composed through the existing runtime
naming extension; no KerML evaluator, metamodel or operational interpretation is
changed.

## Bounded proof and closure

The added branch applies only when a sibling lacks an explicit declared full
name and the existing KerML answer is Complete, Determinate, nonempty, and does
not contain the sought full name. The actual naming answer, canonical
observations and search evidence remain in the composed result. Incomplete or
ambiguous names cannot establish exclusion. A possible overlap still follows
the existing full-name-selection path and remains Incomplete when unproved.

Current-graph queries can use this current-graph exclusion independently of a
producer certificate. The effective query additionally closes the sibling,
naming-source explanation endpoints, their Type ancestors, and the bounded
source/property/ownership/member populations searched by the naming answer.
Ordinary packages are not incorrectly cast to Type; generated KerML Features
are not incorrectly cast to SysML Definition or Usage. Missing witnesses retain
ProducerClosure searches and Incomplete results.

All six existing closure requirements are required on those bounded subjects.
This is deliberately conservative: source selection can compose feature_target,
chaining, typing, transition parameters and payload queries, whose writers are
not all covered by EffectiveNaming alone. Subjects are deduplicated across the
whole qualified path. The implementation does not indiscriminately close every
positive fact expanded from a derived producer's provenance DAG.

## ADR 0026 impact decision

This is compatible additional proved scope under the existing explicit-name
query contract, not an exemption justified by calling the old behavior a bug.
Every new exclusion had previously entered `qualified_segment` with a nonempty
name population, producing Incomplete and no qualified-name value. That result
made no Complete absence claim. The additional branch proves exactly the same
published qualified-name meaning with retained negative and closure evidence.

The explicit-full-name, empty-name, duplicate-sibling, pending-namespace,
ambiguous/overlapping-name and target-name-selection paths keep their prior
conditions. New closure requirements are applied only on the newly supported
exclusion branch. Previously Complete path values and guarantees are preserved;
focused regressions retain the named/unnamed root and explicit long/short-name
behavior.

A repository-wide Rust caller inventory finds no producer or finalizer caller
of current_qualified_name/effective_qualified_name. Callers are the public
effective wrapper, tests and `crates/kerml-text/examples/sysml_foundation.rs`.
The finalizer audit in `crates/kerml-text/src/sysml/publication.rs` calls
effective_names, which is unchanged. Canonical records, producers, scheduler
requirements and registry, accepted graph/certificate/bindings, profile manifests
and serialized receipt/cache contracts are unchanged. No `/6` rule-set or profile
identity bump is proposed for this additional proof route. A separate version
would be necessary if the interpretation, a previously Complete answer, or its
guarantee changed; this patch deliberately avoids those changes.

The integration owner must review the production-source freshness change,
retain the actual acceptance identities, and rerun the accepted self-model gate.
Existing accepted Systems publication and successful cache restoration are not
rewritten as a new producer closure. This review does not establish phase M
success; the failed run remains recorded.

Focused commands, actual exits and output hashes are recorded separately in
`qualified-name-verification.json`. No accepted cache or full corpus is loaded
by these focused tests.
