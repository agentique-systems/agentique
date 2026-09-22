# SysML foundation implementation seams

This is a proposed implementation plan under ADR 0012, not accepted publication
or implemented semantic behavior. Production semantic work starts only after
K10–K13 accept the real KerML corpus and authored integration.

## S0–S3: one graph and a composed context

`agq-sysml` already provides all 93 SysML descriptors and borrowed views. Retain
`{ElementId, &ModelView}` and the language-agnostic kernel records. Add
`agq-sysml-semantics` depending on kernel, KerML descriptors/semantics and SysML
descriptors. Neither parsing nor transport belongs in that crate.

### Implemented acceptance and cache boundary

Reviewed against `d322c1d` and its accepted-cache implementation. The checked-in
receipt still says `unaccepted`; these APIs do not establish corpus acceptance.

`agq-kerml-semantics` now owns the parser-independent
`AcceptedPublicationReceipt::checked_in()` and
`CompletePublicationOverlay::restore_accepted(overlay, roots, libraries, receipt)`.
The receipt is compiled from the separately checked-in acceptance and binding
manifests; arbitrary cache JSON cannot construct it. Restoration re-encodes the
actual graph and checks its archive hash, complete semantic identity, capability
families and standard-role population. A restored object records
`restored_from_receipt() == true`; an ordinary producer-closed overlay does not.
This existing boundary replaces the earlier plan to invent a receipt mechanism.

The source-facing `CanonicalKermlStandardLibraries::restore_cache` additionally
checks bounded ZIP entries, verified source content, roots/source-map metadata
and the accepted binding manifest. Keep that I/O/provenance facade in
`agq-kerml-text`; the future semantic crate can consume the restored complete
overlay, its existing context/bindings and the trusted receipt without depending
on the parser. Its constructor must require this accepted route, not any object
of type `CompletePublicationOverlay`: the public canonical builder seals closure
without independently establishing source/mandatory-reference acceptance.

Restore under the receipt's original KerML v9 registry first. The kernel archive
pins the exact registry and semantic restoration pins the exact descriptor digest.
Loading the archive under a combined SysML registry, or changing its recorded
digest to make that load pass, would destroy the accepted dependency contract.

### Remaining kernel registry-extension seam

`Snapshot::with_immutable_dependency(Arc<DerivedOverlay>)` already starts an
independent authored history, shares canonical records and protects dependency
facts. It currently clones the dependency's `ModelView`, including its KerML-only
registry. It cannot yet create a SysML record in that history.

Add a fallible, language-neutral counterpart accepting a validated extension
registry; the following is a proposed API, not existing code:

```rust
Snapshot::with_immutable_dependency_in_registry(dependency, registry)
    -> Result<Snapshot, /* typed registry/dependency error */>
```

Implement compatibility inside `agq-kernel`'s registry/model boundary, where all
descriptor maps and navigation state are available. Require:

1. Every base metamodel, class, property, association, enumeration, primitive
   and source entry remains present and identical. Preserve existing review
   evidence. New descriptor identities may extend the graph; duplicate IDs or
   replacement of base metadata must fail atomically.
2. Every old class retains its effective properties and resolution of old property
   IDs. Preserve old association effective ends, storage policy and primitive
   storage domains. Merely retaining raw descriptors is insufficient: adding a
   property owned by an old class could change its effective contract.
3. Revalidate/rebuild the project `ModelView` using existing strict structural
   validation and association/index construction. Retain the same record Arcs,
   occurrence identities, declared source, proofs, computation failures and
   search dependencies; carry all reserved/retired IDs into the new history.
   Do not make another canonical copy of the library or recompute its producers.
4. Store the original dependency Arc unchanged. The accepted overlay and its
   registry, archive identity and digest remain unchanged; only the authored
   view uses the larger registry. Existing edit, derivation and ownership guards
   must remain active on both `apply` and `preview`.

At the language boundary, assemble
`agq_kerml::descriptors_for_profile(OPERATIONAL_V9)` plus
`agq_sysml::own_descriptors()` across every `DescriptorSet` field, then call
`MetamodelRegistry::from_descriptors`. The current `agq_sysml::descriptors()`
starts from published KerML and must not silently supply this v9 dependency.
Reuse `SemanticContext::bind` through its public constructors: it already admits
independent classes/subclasses while checking exact KerML descriptors, base-class
effective properties and source identities, and rejects profile-excluded raw
descriptors smuggled back as extensions. Generic kernel compatibility and this
language/profile check are complementary.

`CompletePublicationOverlay::project_context` already checks that the snapshot
retains this exact dependency model, installs asymmetric root availability and
preserves standard bindings, formal targets and `publication_dependency_digest`.
Keeping the original Arc makes that check reusable with the extended project
view. The composed context must keep two distinct descriptor identities: the
accepted KerML descriptor digest and the complete current SysML registry digest.
Descriptor-search caches must rebind to the new context; shared immutable proof
storage is not permission to reuse query outcomes across different registries.

The current kernel archive intentionally rejects protected dependency snapshots.
Do not flatten a SysML project or Systems candidate to bypass that boundary;
project/dependency persistence is a separate future archive contract.

A composed SysML context holds the existing KerML context/query evaluator plus
the SysML rule-set identity, combined descriptor digest, exact Systems candidate
or accepted identity, and validated `StandardSysmlBindings`. Its full identity
includes the authored/current revision, accepted KerML publication digest,
explicit operational profile, KerML library-set identity, both language rule-set
identities, descriptor graph and source set. Candidate and accepted Systems
states must remain distinct.

Reject a supplied dependency with a different publication digest, operational
profile, library set or descriptor graph before query construction. Use the
existing immutable dependency mechanism; local records may reference library
IDs, but cannot edit their ownership, slots or derived facts. Two authored
projects share immutable library storage and keep independent revisions. No
lookup may make the standard-library roots see authored roots.

Reuse `QueryResult`, completeness, evidence, positive reads and search dependencies.
Filtering a KerML result by SysML metaclass must preserve its incompleteness and
account for descriptor reads. An incomplete base query must never become Complete
just because the filter returned no elements.

### Focused extension and context tests after acceptance

Extend the existing suites below; their current tests establish reusable behavior,
not acceptance of the proposed registry-extension API.

| Existing suite | Required added coverage |
| --- | --- |
| `crates/kernel/tests/immutable_dependencies.rs` | Two independent projects share the original dependency Arc and record addresses under a larger registry; new subtype records are legal; derived/declared provenance, searches, failures and retired IDs survive; authored and derived writes, inverse ownership edits and root capture remain rejected. A rejected registry extension leaves both readers unchanged. |
| `crates/kernel/tests/metamodel.rs` | Reject missing/replaced base descriptors, source changes, changed base-class effective properties, changed old association ends/storage and primitive/enum domains. Accept descriptors owned by new classes, including redefinition of inherited properties in those new class contexts. |
| `crates/kerml-semantics/tests/foundation.rs` and `profiles.rs` | Extend `full_sysml_and_unrelated_extensions_preserve_kerml_answers_and_change_context_identity` to the v9 registry plus the real protected dependency; retain `operational_context_cannot_smuggle_deleted_descriptors_back_as_an_extension`. Base query values/completeness/evidence remain valid while the composed descriptor identity changes. |
| `crates/kerml-semantics/src/publication_restore_tests.rs` and `crates/kernel/tests/archive.rs` | Preserve exact-registry archive validation, receipt/graph/profile/rule/library/source tamper rejection, capability-family checks and zero replayed producer stages. The restored dependency still passes `project_context` after creating an extended-registry project; an independently reconstructed overlay with a matching digest still fails its exact dependency check. |

The future SysML context tests additionally reject a wrong accepted publication
digest, v8 or published profile, KerML library set, or pinned combined descriptor
graph. An unrelated registry extension may be valid for general KerML queries
yet still violate a SysML project's exact descriptor pin. Test that distinction.
Keep the dependency digest and standard IDs stable across a local edit while the
authored revision/model digest changes; verify parallel immutable readers and
library-root isolation with the real accepted cache. These are focused tests for
the new seam, not another full KerML producer or corpus acceptance run.

The relevant implementation points are `kernel/src/{metamodel,model,archive}.rs`,
`kerml-semantics/src/{context,publication_overlay,publication_restore}.rs`,
`kerml-text/src/library/publication_cache.rs` and `sysml/src/lib.rs`, all under
`crates/`. This plan changes none of those modules.

## S4–S7: shared source and textual architecture

SysML 2.0 §8.2.2.1.2 explicitly reuses KerML lexical structure with different
reserved keywords and special terminals. Reuse
`crates/kerml-syntax/src/lexer.rs`, source/revision identity, `ByteRange`,
`SourceOrigin` and syntax reconciliation. Make keyword recognition dialect-aware:
SysML permits names reserved only by KerML, and reserves its own added keywords.
Do not change KerML name acceptance globally.

Reuse the lossless `production::Document` arena and iterative chart machinery in
`crates/kerml-syntax/src/production/`. Parameterize the generated grammar vocabulary
and root/keyword set, preserving KerML parsing unchanged. Common package, import,
alias, annotation, name, specialization, multiplicity and expression productions
should share their existing implementation. SysML Definition/Usage and their
specialized body/member productions extend that grammar. Syntax nodes retain
their grammar role and source origin; they are never semantic model records.

`SourceProject` currently stores SysML with `FrontendUnavailable`, while authored
KerML uses a bounded declaration frontend. Full library KerML uses the production
arena and `library/construction.rs`. Introduce a shared canonical construction
interface rather than translating SysML into fake KerML source or expanding the
bounded authored AST into a second full parser. Library provenance and authored
edit identity policies remain distinct inputs to the same kernel construction.

The current library builder fixes its registry to KerML and dispatches through
`library/vocabulary.rs`. Generalize that seam to the combined descriptors and
SysML production-to-metaclass/relationship mappings. Reuse ownership assembly,
pending endpoint assertions, source maps, imports, FeatureTyping,
Subclassification, Subsetting and Redefinition. Preserve relationship identities
and source syntax identities; never copy inherited records.

Implement parser slices in this order, then run all 21 documents:

| Slice | Exact corpus drivers | Required added grammar |
| --- | --- | --- |
| 1 | Attributes, Metadata, SysML, Items, Parts, Ports | Definition/Usage prefixes, metadata/item/part/port/attribute, anonymous reference/parameter usages, derived/abstract/end/direction modifiers |
| 2 | Connections, Interfaces, Flows, Allocations | Connection/end/port members, connect, message/flow, event occurrence, succession and symbolic bounds |
| 3 | Calculations, Constraints, Requirements, Cases, AnalysisCases, UseCases, VerificationCases | Calculation/result bodies, assert, subject/objective/requirement members, analysis/verification/use case, enum literals, metadata |
| 4 | Actions, States | Assignment, perform, while, action successions, entry/do/exit members, binding connectors |
| 5 | Views, StandardViewDefinitions | View/viewpoint/rendering, short names, satisfy/require bodies; resolve the recorded published grammar discrepancy explicitly |

The parser goal is 21/21 complete production trees, zero unexplained recovery and
exact bytes. Lexical inventory success does not satisfy that gate. Structural
expression construction is required; execution/evaluation is a later milestone.

## S8–S15: semantic foundation and authored vertical

Construct an unpublished Systems candidate over authored SysML/KerML documents
and the accepted KerML graph. Existing KerML lookup handles imports, aliases,
visibility, shadowing, cycles, qualified names and root availability. Retain
pending populations and endpoint assertions until resolved; do not turn missing
SysML frontend records into successful empty lookups.

Definition/Usage queries compose `feature_types`, `supertypes`,
`direct_features`, `effective_features`, `subsetted_features`,
`redefined_features` and `effective_names`. Definition-owned usages and
Usage-nested usages filter the existing canonical feature population by Usage.
Use the pinned property domains when selecting typing targets: existing KerML
classifiers can type specialized usages, so a blanket filter to SysML Definition
would lose valid typing. Separate direct from effective typing/specialization.
Qualified names use semantic ownership/naming and propagate ambiguous names.

Add Attribute, Item and Part queries as typed applications of those contracts.
Add Port, Connection and ConnectorAsUsage when required by the actual corpus;
do not claim that metaclass casting alone executes conjugation or connector rules.
Validate only standard anchors actually used by algorithms, including qualified
path, correct metaclass, exact library identity, visibility and uniqueness.
`Attributes::AttributeValue` is an alias to a KerML DataType, which matters to
binding validation. Reflection metadata definitions in `SysML.sysml` are ordinary
source declarations, not replacements for compiled runtime descriptors.

The first acceptance fixture is Engine/Vehicle/engine/SportsCar. Check textual
and direct-kernel semantic equivalence, original inherited `engine` identity,
unchanged record count after querying inheritance and evidence-bearing typing.
Syntax IDs need not match a programmatic model. Test wrong dependency identities,
mixed-source resolution and edit isolation independently of that happy path.
