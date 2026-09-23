# Compositional publication acceptance boundary

Read-only review at `068f7b0`; implementation contract ADR 0027 precedes the
test changes. Accepted KerML Operational v9 is unchanged. This note is a harness
and boundary review, not compositional equivalence or Systems acceptance.

## Existing acceptance and restoration

`crates/kerml-text/src/sysml/publication.rs::publish` constructs final validated
Systems bindings and a combined context before its unrestricted closure call.
After closure, its certificate-coverage, capability, authority, mandatory-reference,
binding and typed-SysML checks are read-only. Those checks can be retained as a
final global audit without invoking another producer scheduler. Input/source
identity, exact 21-document/reference population and provenance gates also remain
necessary; a component's success cannot replace them.

`publication_cache.rs` exports local graph records over the exact immutable KerML
dependency, plus facade and closure entries. `publication_restore.rs` authenticates
entry hashes/lengths, revalidates decoded graph content and the dependency contract,
then validates bindings and certificate coverage without replay. The finite
`trusted_publication.rs::CATALOGUE` is empty for Systems. It must stay empty until
independent acceptance. A composite tree needs authenticated tree structure and
component bindings; the current flat certificate restore expects its subjects to
equal every model element and is not a component-certificate constructor.

## Concrete sealing hazards

- `producer_closure.rs::producer_reads` maps incoming/association searches to
  `ProducerRead::Inverse`; it maps model/instance searches and missing element
  identities to `Global`. `effect_changes_read` treats every effect as changing
  `Global`, and every relationship or reference-scalar effect as changing
  `Inverse`. `producer_closure_rebind.rs::read_changed` invalidates both when any
  subject changes. A planner using these contracts cannot silently narrow them.
- `producer_broad_reads.rs::sealed_dependency_proof_searches_do_not_become_project_producer_reads`
  distinguishes a historical immutable proof search from an identical live
  Model/Incoming search. Adding the live search makes the reader Pending in both
  merge orders. The same fixture leaves an accepted relationship record unchanged
  while a local ownership addition changes its inverse navigation. Immutable
  record storage therefore does not prove immutable inverse query populations.
- `producer_future_scope.rs::binding_helpers_do_not_activate_future_owner_writers_at_unrelated_types`
  exercises bounded fresh ownership, unbounded creation/transitive activation,
  model writers, ownership writers, reference-scalar writers, reparenting and
  pending providers. The latter cases reopen otherwise empty target populations.
  Fresh-subject activation and mutable ownership must participate in the graph.
- Declared endpoint refinement and StandardSysmlBindings validation precede
  strict closure. All strategies must use the same final endpoint selection,
  roots, producer registry and dependency contract. Candidate construction's
  unbound/partial context is not interchangeable with the accepted context.
- Kernel `StructuralSearch::ProducerClosure` stores only subject and requirement,
  deliberately excluding a historical certificate digest. Public query evidence
  additionally binds the actual certificate. Preserve canonical search contents;
  compare composition-specific root identity separately from semantic equality.

## Exact equivalence harness

`tests/unit/closure_equivalence.rs` extends the existing permanent full-scan versus
worklist fixture comparator. It compares original declared slots,
derived identities and explanations, complete Element records (including slots,
source provenance and ownership order), occurrences, derived navigation, failed
computations, exact search sets, selected ordered-contribution evidence, aggregate
and producer semantic digests, each subject's six closure requirements and each
applicable family state. It compares exact queries for names, ownership, canonical
fact evidence, namespace memberships, supertypes, inherited features,
specialization, subsetting, redefinition and feature chains. Existing typing,
featuring, cross-feature and capability comparisons remain.
Mutation controls retain equal populations while changing a proof or negative
search, and reject those changes. They also distinguish failed-computation details,
closure requirements and allocation-independent equality.

The accepted medium fixture uses exactly:

```text
Actions,Attributes,Calculations,Connections,Constraints,Flows,Interfaces,
Items,Parts,Ports,Requirements,States,Views
```

The historical `systems-medium-result-populations` command is in the previous
`language-stability-bridge/publication-commands.json`: 1,726.844 seconds, 6,435.4
MiB peak private memory, exit 0. Only its JSON report and stage observations exist;
they do not contain an exact graph/evidence baseline. Its counts are not an
equivalence artifact. The original accepted KerML cache exists in the main worktree
at `verification/generated/kerml-v9-publication/canonical.publication.zip`
(540,739,848 bytes).

After synthetic composition passes, execute medium worklist and composition from
the same prepared strict candidate with final bindings, serially. Compare complete
logical graph/evidence/query/requirement artifacts or use the in-process comparator
when resources permit. Stream artifacts under ignored generated storage to avoid
holding two corpus closures. Kernel archive bytes alone are insufficient: they
also encode snapshot revision and transport tables, and omit selected-contribution
cache data that can affect precise producer evidence. No medium or full publication
run was started during this review.

Actual focused commands, output hashes and exit codes are recorded in
the `audit-commands.json` source-ledger records in `commands.json`; raw output
stays in ignored generated storage. Final
formatting and package all-target Clippy with warnings denied pass. The final
semantic library run passes 269 tests with two existing scale probes ignored;
this includes all five new comparator mutation controls and 33 worklist tests.
An initial compile error from an ambiguous proof type was corrected by naming
the kernel proof type explicitly; its failed command remains in the ledger.
