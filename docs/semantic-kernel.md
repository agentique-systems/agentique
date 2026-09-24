# Semantic kernel, second generation

[ADR 0022](adr/0022-kerml-publication-vs-conformance.md) distinguishes a strict
Snapshot, a canonical KerML publication and a potentially incomplete conformance
report. `SemanticContext` identifies partial derivation overlays explicitly.
Partial overlays do not assert complete implied inclusion; callers can still run
the strict assertion check separately. Kernel validation is unchanged.

`DerivationBuilder::association_occurrence` adds canonical derived association
facts through the same registry and navigation validation as declared links.
Its identity includes the rule, subject, semantic output role, association and
oriented descriptor/endpoint identities. Ordered-end positions remain explicit
validated data; insertion order and random allocation do not determine identity.
Occurrence explanations are ordinary `FactKey::AssociationOccurrence` evidence.
Navigation retains occurrence provenance and does not store duplicate slots.
Derived-only associations are also available to derivation; declared writes stay
prohibited. A canonical authored slot carrier and an occurrence cannot both
store the same association, and a projected end cannot also have a derived slot.

`DerivationBuilder::from_overlay` adds a producer stage over the same immutable
declared revision. Earlier facts and explanations remain available, and ordered
extensions retain their declared prefix and earlier contributors. Conflicts,
missing dependencies and explanation cycles reject the candidate atomically.
Rebasing derived facts onto changed source inputs remains unsupported.

Immutable kernel explanations are interned by exact content and shared between
records, indexes and explanation lookup. Queries distinguish original declared
evidence from a later derived collection with the same fact key. Both origins
remain inspectable; a dependency on the original collection never traverses
unrelated overlay additions. Producer evidence retains immediate canonical
dependencies and their existing DAG instead of copying transitive proofs into
every derived relationship.

`ExplanationPool` and `element_with_explanation` share proofs during planning and
enqueueing, before materialization. Rule identity and automatic subject evidence
are still validated; adding evidence never mutates a caller's shared proof.
Iterative Tarjan validation walks borrowed dependency sets and stores only
per-vertex traversal state, avoiding forward/reverse copies of dense evidence.

`StructuralSearchPool` shares exact negative-search populations independently of
proof interning. Producers share one immutable search set across their outputs;
kernel overlay handoff retains those shared sets. Combining evidence takes the
exact union without changing any caller's set. Public query iteration still
returns ordered search contents. Logical entry counts and retained distinct-set
counts expose the allocation difference without changing semantic identities.
Private producer/status queries carry these sets through intermediate results
instead of expanding the same searches at every fact read and subquery merge.
Persistent producer evidence, dependency read sets and the retained positional
proofs expand them at their exact contract boundaries. Ordinary public queries
and the reference full scan remain eager; cache lifetime and semantic identities
remain tied to their immutable context.

`Snapshot::with_immutable_dependency` starts an independent authored history over
a shared immutable overlay. Its dependency Element records remain shared across
projects and revisions. Kernel transactions protect dependency records, slots,
occurrences and inverse ownership edits; they can still create local relationships
that reference dependency identities. Final-candidate validation also rejects new
local composite slots, occurrence projections or derived navigation that would
change dependency ownership. Local derivations retain the dependency's
explanation DAG, including declared evidence beneath extended collections. This
kernel operation does not certify that a dependency is an accepted language
publication; that remains the language facade's responsibility.

`Snapshot::with_immutable_dependency_in_registry` validates an extension of the
dependency registry, rebuilds project indexes and retains the exact original
dependency and record Arcs. Existing descriptors, sources, effective class
contracts and storage interpretations cannot change. New SysML classes can
redefine inherited properties in their own contexts. Construction previews also
retain the protected dependency; `project_construction_context` binds incomplete
local roots without making accepted library roots see authored declarations.
Generic registry compatibility and language profile/publication authentication
remain separate checks.

`agq-sysml-semantics` composes that KerML context through a 13-field dependency
contract. Its deterministic digest binds the accepted KerML Operational v9
publication, both profiles, descriptor graphs, library identities, grammar and
semantic manifests, scoped standard bindings and rule sets to shared query and
producer answers. Attachment authenticates the checked-in receipt and expected
contract; overlay attachment also requires the exact protected dependency.
Context forks borrow the same graph with independent query caches. Historical
KerML-only contexts and publication receipt encoding remain unchanged.

SysML naming operations participate in shared namespace lookup through an
explicit language naming hook. Its presence has a separate context identity;
forks retain its implementation and failed applicability searches retain their
dependencies. Transition payloads use canonical Subsetting and ordered
FeatureChaining records. Neither mechanism inserts aliases or copied names.

The shared producer worklist operates on new Systems records and derived facts.
Construction overlays retain missing endpoint obligations while structural
bootstrap and reference refinement establish declared links; strict publication
uses full combined closure. KerML producers are not replayed on the sealed
dependency. `canonical_fact_evidence` retains existing proof/search dependencies
and failed or incomplete observations. SysML projections reuse that evidence
without copying inherited usages. Direct and current-graph queries remain
distinct from effective queries with unfinished producer obligations.

Operational v2 (reusing the frozen Operational v1 grammar) parses all 21 exact Systems sources without recovery, and generic
lowering constructs 7,591 declared elements with 1,327 reference assertions while
retaining StandardLibrary source provenance. Published grammar behavior remains
13 complete parses and eight retained failures. The public Systems facade gates
identity, provenance, bindings, mandatory references, capabilities and producer
closure before acceptance. The final monolithic scheduler reached Complete with
26,532 closed producer pairs and 452,052 closed requirements. An authenticated
read-only finalizer restored that exact frontier without producer replay and
completed all 1,327 mandatory references. Its effective SysML population audit
rejected acceptance with 674 findings, including missing enumeration-owner typing.
No accepted artifacts were issued. The three bounded authority decisions remain
recorded in the immutable Operational v2 correction manifest.
Neither declared construction nor context attachment
certifies a Systems publication. See the
[finalization evidence](../verification/summaries/systems-finalization/README.md).

The KerML `CanonicalPublicationBuilder` starts from declared producer subjects
under Operational v9 by default, with explicit historical profile selection.
A metaclass applicability index excludes unrelated records;
subsequent immutable frontiers schedule only new subjects and readers invalidated
by canonical positive or bounded negative search dependencies. Every producer in
a frontier observes the same graph. Contributions merge by semantic identity before
the next frontier, so traversal order and batch size cannot select winners.
The retained reference full scan compares records, occurrences, ownership, proofs,
query answers, capabilities and semantic digests on fixtures.

Operational v8 closure separates structural implications from context-sensitive
reference-result and FeatureValue bindings. Valuation subsettings, invocation
specialization, typing, featuring and contextual chains close first: these can
change which featuring context is nearest. Only then may the two binding roles
materialize their selected context. Their OwningMembership infrastructure does
not add features or source specialization edges to the prior domain population.
Generated subjects still run normal producers, and dirty context readers still
re-evaluate; conflicts remain errors. Both worklist and independent full-scan
closures use this phase boundary. The public one-shot planning API and earlier
operational profiles retain their existing behavior. A structural fixed point
alone cannot issue complete publication evidence.

`CompletePublicationOverlay` is issued only after producers answer Complete on an
unchanged graph and all publication capability queries succeed. Stage-budget
exhaustion is incomplete, not acceptance. Scoped closures and capability reports
cannot promote an overlay. Internal producer queries retain immediate canonical
proof edges and bounded searches without repeatedly expanding the same explanation
DAG. Public evidence-bearing queries retain their full contract. This phase contract is independent
of `KerMlConformanceReport` and does not alone establish accepted library or
authored-project publication; those gates are recorded in the
[milestone evidence](../verification/kerml-complete-publication/README.md).

The historical v8 convergence slice exposes a publication-blocking symbolic-bound
reference: the retained v8 multiplicity interpretation gives its expression an
empty domain, while the required reference binding has no permitted context for
the outer Feature. Worklist and full scan agree on Incomplete. The
[v8 witness and authority impact](../verification/summaries/overnight-convergence/README.md)
retain this boundary. Operational v9 centralizes the two explicitly authorized
multiplicity context corrections in `multiplicity_featuring_context`: semantic
owning-Namespace Feature domain, or owning end Feature domain for an owned cross
Feature. Bound Expressions share the MultiplicityRange domain. Incomplete
premises remain incomplete; no lexical fallback or numeric evaluation is added.
The [v9 authority record](../verification/summaries/kerml-v9-publication/authority-decision.json)
keeps Published through v8 and their findings intact. Acceptance remains a
separate corpus gate.

Additive kernel handoff consumes an unshared overlay and transfers its accumulated
maps; retained readers force a safe copy. Proof interning persists across frontiers.
Cycle checking factors many facts with the same proof through one proof node, so
N outputs sharing M premises require N+M dependency edges rather than N×M.
Structural validation and navigation indexes are still rebuilt for frontiers
with changes; this is not an incremental-index implementation. A worklist
frontier with no pending writes reuses its immutable input after all proposed
records and scalar slots have been checked. Failure metadata and search-only
changes require materialization too. The reference full scan still rebuilds
independently. Published snapshots remain immutable and atomic validation is
unchanged.

Semantic rule set `/23` uses a framed content encoding for model and library
digests. A shared explanation is hashed once and referenced by its content hash
from each fact. Addresses only key a local memo and never enter the digest.
Records, values, ownership order, occurrences, source provenance, unsuccessful
computations and bounded searches remain part of the identity. This prevents
digest generation from expanding N copies of an M-premise shared proof.

`KerMlStatusQueries` returns explicitly evidence-free `QueryOutcome` values for
bulk checks. `QueryOutcomeWithReads` additionally carries `QueryReadSet`, reusing
the same positive/negative invalidation contract as publication closure. A read
set is not a proof or publication certificate. Cache reuse requires an unchanged
query contract and complete reporting of changed records, pending inputs and both
old and new navigation endpoints. Full explanations remain available through
`KerMlQueries` against the same canonical graph.

Negative effective conclusions additionally require a scheduler-issued
`ProducerClosureCertificate` ([ADR 0025](adr/0025-producer-closure-evidence.md)).
The certificate binds the exact graph, producer registry and semantic context;
it remains immutable evidence beside the graph. Stable family descriptors declare
applicability and potential effects, including future cross-subject writes.
Quiescence follows actual family dependencies, rather than graph convergence
alone. Typed closure search evidence explains which subject and requirement the
certificate covers. Without that evidence, a negative formal antecedent remains
Incomplete; a proved positive antecedent need not await unrelated producers.

[ADR 0027](adr/0027-compositional-semantic-publication.md) remains proposed research
defining the additional proof needed to compose semantic components. Current
Systems acceptance uses the monolithic dependency-driven fixed point; useful
source partitions are not a prerequisite. A dependency DAG and a projection
of an existing certificate are inspectable planning evidence, not acceptance.
Local record immutability does not freeze incoming searches: a later local
relationship can reference an earlier subject without changing its record.
Potential writers, zero-output evaluation reads and structural/contextual phase
dependencies must all be discharged before a component can seal. Existing
conservative global requirement masks cannot be removed merely by partitioning
the scheduler population.

Scoped publication also carries `PublicationProviderReads`. It distinguishes
observations of fixed declared identities and names from reads that can require
additional additive producers. General revision invalidation retains all reads.
Mutable ownership, incoming navigation, derived facts, missing identities and
coarse persistent kernel searches remain conservative provider obligations.
Discovered providers expand the slice before closure is rerun from its original
declared graph; a development size limit never establishes semantic completeness.

The kernel graph archive is a structural transport for a root Snapshot and its
DerivedOverlay. It streams interned proof/search tables and preserves identities,
provenance, occurrence order and retired identity reservations. Loading validates
kernel invariants and conveys no language publication acceptance. Protected
dependency snapshots cannot be flattened through this archive API. An accepted
language cache additionally needs independently pinned publication, descriptor,
rule, library and source identities; its artifact hash is distinct from the
canonical semantic digest.

Reference refinement reconstructs ordinary candidates and caches provisional
reference outcomes against those read dependencies. `QueryInvalidationSet` keeps
only sorted affected-element keys and the global-search flag after the cache has
consumed a query's evidence. Equivalence tests check its decisions against the
full read set. The final mandatory-reference audit still requires Complete,
exactly one candidate and agreement with the stored canonical endpoint.
Effective names are memoized only within an immutable query evaluator; forks
and new revisions begin with fresh traversal caches.

Kernel incoming references have an additional property index containing offsets
into the existing ordered occurrence vectors. It covers stored slots, association
occurrences and derived navigation without copying reference records. Language
queries can select effective source-role properties while keeping the broad
negative incoming-search dependency. Registry property resolution also uses its
existing effective-property index without allocating a descriptor vector for
each read. Neither optimization changes canonical records or iteration order.

`CanonicalKermlStandardLibraries::publish` additionally checks the exact verified
source construction and every mandatory reference against the closed graph. Its
private immutable facade is the only standard-library input accepted by
`SourceProject::with_standard_libraries`. Authored contexts retain the complete
publication digest, profile, bindings and library identities; library roots never
gain visibility of authored roots. The bounded authored syntax supports named
aliases, visibility and nonrecursive namespace imports. Unsupported import forms
and unresolved namespace populations remain explicit incomplete input.

Effective feature lookup retains the acyclic fast path and uses the existing
namespace membership fixed point for cyclic specialization. It preserves
membership identity, redefinition suppression, visibility and search evidence;
failure to converge remains incomplete.

Operational v8 carries independent authority identities for the owned-cross
domain and KERML11-75 import-population corrections. Selection of an owned cross
Feature still uses v7; v8 excludes its owning end when deriving the domain.
Binary domains use the opposite end's Types, while n-ary domains require an
ordered Cartesian construction over every other end. Imported Memberships retain
canonical identity: owned collisions are removed first, then distinct colliding
imports are excluded symmetrically after deduplication.

`StandardKermlBindings` validates each role against its authoritative library
artifact and retains a `LibrarySetIdentity`. `CollectionsArray` is anchored in
the Data Type Library; `ThingsThat` and `OccurrenceStartShot` are Semantic Library
roles. These construction bindings do not establish an accepted publication.
The [current capability matrix](../verification/summaries/kerml-v9-publication/capability-status.json)
records whole-corpus publication and integration results separately from
incomplete conformance coverage. V9 centralizes ordinary and owned-cross
multiplicity featuring context, including every bound Expression; the exact
operational authority is retained in [ADR 0023](adr/0023-operational-multiplicity-context-v9.md).

[ADR 0009](adr/0009-property-redefinition-context.md) reassesses the Property
context invariant without changing it. Final UML text and the resolved mixed
ownership issue do not establish a safe correction for the pinned failing edge.
The [complete foundation review](language-core-foundation-review.md) records
unimplemented full-registry/value/association obligations; the bounded kernel
capabilities documented below must not be read as full language completion.

`agq-kernel` is an additive semantic substrate. It does not implement full KerML
or SysML validation. The compatibility target is KerML 1.0 / SysML 2.0; the
[decision record](adr/0001-semantic-kernel-v2.md) identifies the standards evidence,
invariants and deferred work. The earlier application still uses its original
crates. New language work targets generation 2; see the
[architecture](architecture.md) and [status register](../standards/v2-coverage.json).
The kernel remains language-agnostic; its archive and exact numeric carriers add
serialization, hashing and arbitrary-precision dependencies without parser or
language-crate dependencies.

## Where to start

Run `cargo test -p agq-kernel`. The programmatic fixture is in
`crates/kernel/tests/common/mod.rs`; `tests/snapshots.rs` demonstrates the seven
distinct elements Engine, Vehicle, engine, SportsCar, OwningMembership,
FeatureTyping and Specialization. No source text, parser or library loader is used.
`tests/derivations.rs` demonstrates effective inherited members and an implied
transitive specialization, including chains of evidence without copying features.
These are explicitly simplified architecture fixtures, not normative definitions
or a complete inheritance algorithm. In particular the fixture does not implement
property redefinition or the full intermediate class hierarchy.

The kernel now supports explicit structural redefinition and association metadata;
`crates/kerml` supplies generated normative Root/Core descriptors and separate tests.
[ADR 0003](adr/0003-normative-root-core-descriptors.md) documents source evidence,
effective property resolution and the association write boundary. The follow-up
[ADR 0004](adr/0004-kerml-typed-views.md) adds borrowed KerML typed views and permits
a unique class-owned association slot when its opposite is association-owned,
non-navigable, unordered and 0..*. This is one canonical storage surface; derived
inverses are not automatically derived. [ADR 0005](adr/0005-kerml-semantic-query-foundation.md)
subsequently adds the ordered-many/scalar-inverse shape used by both ownership
pairs: one ordered canonical slot and a reconstructible inverse index. Writes to
the inverse remain refused. Other unsupported shapes fail explicitly.
`supports_slot_storage` exposes the write capability; `navigation_slot` reads
supported canonical and inverse surfaces without duplicate storage.
Use `MetamodelRegistry::from_descriptors` for sets containing associations or enums;
`resolve_property` maps an inherited ID to its effective replacement. Derived-union
and subset metadata do not themselves execute language rules.

The crate-level Rustdoc includes a runnable minimal transaction. Main APIs:

| API | Responsibility |
| --- | --- |
| `MetamodelRegistry::new` | Validate exact versioned descriptors and inheritance |
| `Snapshot::new` / `change_set` / `apply` | Construct immutable declared revisions |
| `ChangeSet::{create,remove,set,clear}` | Explicit candidate operations |
| `ModelView::{element,elements,instances}` | Stable identity and metaclass queries |
| `ModelView::{incoming,outgoing}` | Generic reference occurrence queries |
| `ElementRecord::{slot,slots,origin}` | Read modeled properties and provenance |
| `DerivationBuilder` / `DerivedOverlay::explain` | Publish inference results and inspect evidence |

## Records and references

`ElementId` is a typed 128-bit identity; `ElementId::new()` allocates a UUID.
Importers may supply an externally assigned ID with `from_u128`. The caller must
preserve identity across revisions and independently ensure global allocation
uniqueness. The kernel prevents duplicate/reused IDs in a declared history and
collisions when adding implied elements. No identifier is a storage index.

A record contains ID, metaclass, origin and slots. A name is an ordinary property.
A relationship is an ordinary record whose properties reference its participants.
Each relationship has its own identity and may contain more than two participants.
Incoming references to a feature therefore point to relationship records and their
specific property slots; they are not themselves additional semantic relationships.
No universal owner, type, qualified name or child list is stored.

Descriptors drive shape: upper bound zero or one means scalar if present; larger
or unbounded upper limits mean a collection. Ordered collections retain order;
unique unordered collections use sets; nonunique unordered collections use bags
normalized into value order. Duplicates in ordered unique properties are errors.
Missing and present-empty collections differ. References must target an existing
element of a compatible class. Primitive domains currently cover Boolean, i64 and
String, plus descriptor-identified enumerations. Unbounded integers, real numbers
and richer datatypes await an explicit standards-driven value-domain extension.
The generated Root/Core closure needs only Boolean, String and enums; it does not
narrow normative numeric domains to i64. There is no JSON catch-all.

## Publication and invariants

A changeset binds to an exact snapshot. Appending any operation reserves a new
candidate revision ID; applying an unchanged changeset twice gives identical
contents and the same candidate revision. Snapshot clones retain exact base
identity, but separately reconstructed/replayed snapshots are not interchangeable
transaction bases. No persistence/repository commit semantics are implied.

Creation precedes edits to a record, but references may point to records created
later in the same changeset. Structural validation checks the final candidate.
Any error leaves the original snapshot unchanged. Deletion never cascades:
the transaction must explicitly remove or repair references. Composite references
allow one owning slot and prohibit containment cycles. Inherited properties are
deduplicated through multiple inheritance; conflicting effective names fail.
Property redefinition/subsetting/opposite rules are not guessed.

An absent required *derived* property means uncomputed, not invalid declared input.
If a derived value is supplied, its type, multiplicity and references are validated.
Structural acceptance does not establish semantic completeness, conformance or
verification success.

Property redefinition must follow strict context ancestry and compatible domain,
bounds and composition. Collection flags are retained; values and borrowed reads
use the effective replacement's shape. Only redefinition edges participate
in `PropertyCycle` rejection. Subsetting checks local context/domain compatibility
and upper-bound narrowing; reflexive and cyclic set-inclusion metadata is
representable. `subset_closure` and `subset_contributors` traverse each reachable
property once. Neither computes derived-union values. Exact source naming
anomalies belong to the versioned standards diagnostics above the kernel, as
documented in [ADR 0008](adr/0008-property-subsetting-cycle-semantics.md).

## Derivation and explanation

Declared state includes authored, imported standard-library, transformation and
internally generated facts. Every record and present slot carries origin metadata.
Source evidence identifies a document, half-open byte range and optional syntax
node and source revision; none of these determines semantic identity. Transformation inputs are
historical evidence, not live references subject to dangling-reference checks.

`DerivationBuilder` builds a separate overlay, pinned to one declared snapshot.
Only the overlay can supply metamodel-declared derived slots or implied elements.
Its merged `ModelView` is a read projection, sharing original records except where
computed slots augment them. The original declared store remains accessible.
An overlay cannot replace an existing declared slot. Every derived element/slot
has a rule and dependencies on declared or derived element/slot facts. Its subject
is automatically included as evidence. Dependency existence and acyclicity are
checked; the rule producer must still provide complete, truthful evidence.

`explain(FactKey)` returns immediate evidence; follow derived dependencies to reach
declared facts. Implied element identity is deterministic over rule, subject and
output role in a versioned private UUID-v5 domain. The role is a typed `OutputKey`,
not a display name or position. This is not a normative OMG ID formula. Producers
must use distinct keys for distinct results and version rule IDs appropriately.

Overlays cannot be rebased. A changed declared revision requires fresh derivation;
older overlays continue to describe their pinned inputs. `base_revision()` identifies
only declared inputs, **not** the derived result set. A future query cache must also
identify the rule set and overlay/computation context. Missing facts are not negative
facts. The separate `agq-kerml-semantics` query contract supplies negative/search
dependencies, context identity, completeness and alternative proofs (ADR 0005).
Selective invalidation and query caching remain deferred.

## Extension and cost model

Snapshots use immutable `Arc` records and a shared registry. Applying changes clones
ordered map structure and identity history, copies edited records, validates the
candidate, and rebuilds indexes. Ancestor/effective-property sets are precomputed;
class queries index exact classes and all supertypes. Storage costs are currently
linear in elements plus references (and their metaclass memberships), with ordered
map lookup costs. Deep containment/evidence graphs use iterative cycle detection.
No arena, global interner, unsafe code, async runtime or interior-mutability scheme
is required. Snapshot and overlay types are tested as Send + Sync.

Public queries return borrowed records/values and deterministic iterators. Persistent
maps, finer sharing of large slots, incremental validation and indexes can replace
the initial implementation without changing semantic identity or query contracts.
Debug output is diagnostic, not a serialization contract. No interchange is exposed.

A generated `agq_kerml::views::Feature<'m>` holds only
`{ id: ElementId, model: &'m ModelView }`, validates its class with
`registry().is_subtype(...)`, and reads slots through `element` and effective
property resolution. It needs no duplicate data. Parser ASTs, SQL rows, diagrams and compiled executable
IR will remain separate representations outside this kernel. Salsa could later
memoize queries, but is neither evaluated nor a canonical storage dependency here.

## Foundation status and follow-up

1. Pin and hash authoritative 1.0/2.0 XMI artifacts; define descriptor generation
   and cross-artifact ID mapping, with explicit artifact/version provenance.
2. Define property redefinition, subsetting, opposites and derived unions before
   importing real descriptors. Choose canonical association directions so inverse
   and derived containment views do not create duplicate canonical ownership.
3. Decide imported non-UUID identifier handling, descriptor namespaces, version
   migration and the full primitive/enum value-domain contract.
4. Specify versioned semantic rules, query context identity, negative dependencies,
   alternative explanations, closure completeness and invalidation contracts.

ADR 0002 and ADR 0003 complete the KerML pin/import and structural descriptor
milestones above, including enum domains and source-qualified identities. ADR 0004
adds typed views and single-slot association storage. ADR 0005 supersedes the
earlier ownership-storage limitation and supplies bounded ownership, specialization,
typing and effective-feature queries. ADR 0006 adds lossless KerML text lowering
and declared lexical name resolution. These later decisions refine the earlier
milestones; their historical limitations are not the current capability boundary.

The preceding bounded milestone left association storage, numeric domains,
constraints, imports, inherited name resolution, multi-document projects and
library ingestion open. Later structural and canonical-publication milestones
supersede those limits. The current SysML foundation composes the accepted KerML
publication and shared textual frontend; its remaining reference, producer and
authority gates are recorded in the
[language stability bridge evidence](../verification/summaries/language-stability-bridge/README.md).
Application migration, persistence, API and execution remain separate future work.


## Completion v3: current structural boundary

This section supersedes the historical bounded and i64-only limitations above.
The complete 175-class graph is structurally registered with exact primitive domain
IDs, arbitrary precision Integer, finite exact decimal Real literals, separate
association inheritance, generic occurrence storage and borrowed typed views.
Derived results distinguish absent, not computed, computed, incomplete and invalid;
association-owned results remain overlay navigation rather than fabricated slots.

The [runtime contract](language-core-runtime-contract.md) is the detailed current
contract. Registry hard failures concern graph integrity and safe representation.
Authoring diagnostics are independently queryable and strict checking remains
available. Unsafe effective replacement and ambiguous alias queries fail explicitly.
Association facts have one writable carrier; links, indexes and derived results
remain revision-bound. Source and library bytes, generation-1 obligations and
historical blocker reports remain unchanged.

## Unpublished construction

`Snapshot::preview` inspects a transaction as a distinct `ConstructionView`.
Its ordinary canonical records and indexes can support semantic resolution before
every required endpoint is available. Missing lower bounds are sorted, typed
`ConstructionObligation` values. Present values, reference types, dangling targets,
upper bounds, uniqueness, containment and association ordering remain validated.

A construction view is never a Snapshot and has no publication shortcut. Preview
does not change its base or consume identities; `Snapshot::apply` still enforces
all structural invariants. This supports mutually dependent declarations without
inventing endpoints or weakening either structural runtime gate.

## Graph transport and publication authority

Kernel graph archives preserve structural data under their versioned format;
they never grant language-publication acceptance. Legacy structural archives,
lossless dependent evidence archives and construction/strict frontier archives
remain distinct. The evidence format retains exact selected ordered-reference
positions and proof/search support. A frontier archive can retain an unpublished
construction with obligations; restoring it cannot manufacture a strict Snapshot.

The language scheduler authenticates a durable frontier's exact source/context,
graph, producer registry and certificate state before continuing the monolithic
fixed point. An independently accepted language facade instead authenticates a
trusted standard cache and restores its accepted evidence without producer replay.
Matching graph counts or aggregate digests alone cannot replace either check.
See [ADR 0026](adr/0026-language-foundation-stability-contract.md); its adoption
still requires the separate language-readiness decision.
