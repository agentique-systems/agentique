# Semantic kernel, second generation

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
The kernel has only `uuid` and `thiserror` as direct dependencies.

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

General association storage, full numeric domains, complete constraints, imports
and inherited name resolution, multi-document projects, generation-2 library
ingestion and SysML remain open. Source parsing exists for the bounded KerML slice;
application migration, persistence, API and execution remain separate future work.


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
