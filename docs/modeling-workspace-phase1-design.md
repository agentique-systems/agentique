# In-memory modeling workspace phase 1

Status: bounded in-memory Phase 1 accepted; all 12 accepted-cache workspace tests
passed and ADR 0024 is adopted. The
[language-readiness gate](../verification/summaries/final-language-acceptance/semantic-closure-readiness.md)
passed and [ADR 0026](adr/0026-language-foundation-stability-contract.md) is adopted.
Kernel invariants and exact accepted-cache restoration on the shared-storage
representation have passed. The self-model, Working-state, four edit/recovery
lifecycle, five-Validated-revision scale and separate recovery-scale results are
measured in the
[runtime record](../verification/summaries/final-audit-semantic-closure/workspace-runtime-acceptance.md);
the [earlier Phase 1 summary](../verification/summaries/modeling-workspace-phase1/README.md)
remains historical preparation evidence.
[ADR 0024](adr/0024-gen2-modeling-workspace.md) selects the additive
`agq-modeling-workspace` boundary; the
[generation audit](gen1-gen2-architecture-audit.md) explains the reuse choices.

## Integrated implementation

The implementation was recovered from the actual retained branch
`foundation/workspace-authenticated-frontier-held`, commit
`e5a65f8b3f6413b3d2094dc88c172cbba6435c3d` (parent
`7223938437dc8ab7c684961bfb76eaba6c6c92f0`). It does not depend on an unavailable
short Git object. The reviewed recovery was integrated and reconciled with the
accepted v3 profile, then extended with the full authored effective audit and
five-validated-revision scale fixture. Its pieces are:

| Boundary | Implemented carrier |
| --- | --- |
| Authored history/head | `agq-modeling-workspace::ProjectWorkspace`, with opaque `ProjectRevisionId`, retained parents and immutable revision handles. |
| Exact source state | `agq-kerml-text::SourceInputs` shares unchanged `ProjectDocument`/syntax values; source edits produce fresh source revisions. |
| Working semantics | `SourceCompilation` binds strict or construction-backed current graphs, references, diagnostics and closure evidence. Recovery is retained without using old graph answers. |
| Checked validation | `WorkingProjectRevision::validate` issues only the private-construction `ValidatedProjectRevision` for the exact accepted handle under `Phase1V1`, after all structural/reference/closure/context gates and its matching finding-free effective audit. |
| Effective audit | `SourceCompilation` retains `SourceEffectiveAudit`: exact SysML semantic context, sorted local canonical subjects including derived records, operation counts and findings from the strict finalizer's effective-query dispatcher. Accepted dependency subjects are excluded; source origins and canonical diagnostic identities are retained. |
| Identity retirement | Kernel `DeclaredConstructionHistory` retains reservations through incomplete construction; explicit source deletion retires identities. |
| Immutable dependencies | Kernel shared-base maps, indexes, proof/search storage and sparse local projections retain accepted standard allocations. |
| Revision queries | Borrowed KerML/SysML evaluators, semantic element/source lookup, metadata and native completeness/evidence. |
| Verification | All twelve accepted-cache tests passed: the self-model, five Working-state tests, four phase-1 lifecycle tests, five Validated revisions each with 100 documents and four parallel readers, and the separate recovery-scale case with a Working fourth revision, repaired fifth revision and four readers making eight passes. |

The ordinary API offers `add_document`, explicit `add_kerml`/`add_sysml`,
`edit_document`, `remove_document` and atomic batches of `ProjectChange`. A stale
head or operational error publishes no revision. An incomplete language result
can publish Working and refuses Validated promotion. Accepted publication
authentication is performed by the language facade before workspace construction.

The following sections preserve the original design and review rationale. Their
statements about unimplemented storage/frontend seams describe the pre-integration
baseline, not the accepted Phase 1 implementation above. Phase 2
is specified separately in the [roadmap](modeling-platform-phase2-roadmap.md).

## Ownership and identity

`ProjectWorkspace` owns immutable accepted KerML/Systems publication handles, a
project identity, a revision map and a Working head. Every successful document
operation creates a new immutable `ProjectRevision`. Use the existing frontend's
document/source revision and syntax reconciliation contracts where possible.
Do not invent name-derived semantic identity or copy semantic records into a
workspace DTO graph. Borrow views/evaluators from the revision's graph lifetime.

A revision binds these distinct identities and values:

- its workspace revision ID and parent, independent of source content hashes;
- each document's ID, language, path label, exact bytes/source identity and syntax
  revision;
- the canonical declared input or explicit incomplete construction, its kernel
  revision identity when available, and any derived overlay;
- the exact semantic context and applicable closure certificate;
- diagnostics and mandatory-reference results belonging to these same inputs;
- both accepted publication identities and their immutable dependency graph.

A workspace revision ID must exist even when no strict kernel Snapshot is
available; it cannot be aliased unconditionally to `Snapshot::revision()`.
Equal source text in independent projects does not imply equal authored IDs.
Standard IDs and publication identities do remain identical across projects.

Systems already depends on the exact accepted KerML publication. The authored
dependency chain must retain that transitive identity and protection, with
records shared through immutable handles. Context validation must authenticate
the full chain. A check that only accepts an immediate KerML pointer is too
narrow once the immediate dependency is an accepted Systems publication. Do not
flatten/copy the combined libraries or create a second mutable root to evade it.

## Document operation and validation boundary

Conceptual public operations are `add_kerml`, `add_sysml`, `edit_document` and
`remove_document`, each requiring an expected head ID. A batch form may reuse
frontend `ProjectChange` values. Ordinary callers do not receive graph mutation
or producer APIs.

The [frontend boundary review](modeling-workspace-frontend-boundary.md) specifies
the additive immutable source-input/result carrier, query facade, recovery
scope evidence and concrete 100-document fixtures. It reuses the accepted
Systems constructor while preserving the existing strict SourceProject API.

The sequence is: check expected head and edit validity; construct candidate
documents/syntax; lower available declarations and retain missing obligations;
run local semantic closure as supported; collect exact diagnostics; then publish
one immutable Working revision and advance head. Sources and graphs become
visible together. A stale base, missing document, invalid byte boundary or
resource failure publishes nothing and reserves no leaked authored identities.

A syntactically malformed or unresolved edit is a successfully recorded Working
revision with diagnostics. It is not an operational edit failure. Old validated
handles remain valid for their exact old inputs, but are never exposed as the
current edit's graph. There is no silent fallback to the previous head's answers.

The strict mixed `SourceProject::apply` still rejects recovered production syntax;
its strict lowering requires a snapshot. The additive `SourceInputs`/
`SourceCompilation` implementation reuses parsing, document identity and
reconciliation while retaining an explicit construction result boundary.
Working revisions
may expose a `ConstructionView` and obligations, an optional strict snapshot,
and an optional partial derived overlay/certificate. If syntax cannot produce
any safe construction, retain source/syntax diagnostics and report semantic
queries unavailable for that revision. Do not use fabricated placeholder
targets or claim that every Working revision has a strict `ModelView`.

`WorkingProjectRevision` is an immutable handle. `ValidatedProjectRevision` has
a private checked constructor retaining that exact handle and a versioned
acceptance result. Validation requires:

- no syntax recovery or unsupported frontend/construct affecting the claimed
  supported language contract;
- a strict canonical snapshot with zero kernel construction obligations;
- Complete applicable producer closure and exact context/certificate attachment;
- all mandatory authored references Complete, with one endpoint agreeing with
  the graph and zero unresolved, ambiguous, invalid or mismatching results;
- authenticated accepted standard dependency identities and bindings;
- a retained finding-free applicable effective audit matching the revision's
  exact SysML semantic context, covering local declared and derived subjects;
- no blocking diagnostics under the explicit platform acceptance contract.

Unimplemented unrelated conformance validators are reported as coverage, not
converted into success or an additional blanket foundation gate. Validation
never claims executable semantics. Returning from an edit operation does not
establish this acceptance result.

## Query surface and concurrency

`workspace.revision(id)` and `workspace.head()` return retained immutable
handles. `revision.diagnostics()` always works. `kerml_queries()` and
`sysml_queries()` return borrowed language evaluators when the current graph
supports them, or an explicit unavailable/incomplete construction result.
Effective answers continue carrying language completeness/evidence; the
workspace does not turn a partial value into a Complete DTO. The validated
handle can expose guaranteed strict graph access under its acceptance contract.

Keep language contexts and caches scoped to their immutable revision. Parallel
readers can create independent evaluator forks borrowing the same records;
query-cache interior behavior need not become part of the public workspace
contract. Only head/history mutation requires exclusive access. This phase has
no async executor, HTTP transport, database, event log or durable-commit promise.

## Acceptance contract and runtime evidence

| Test | Concrete stimulus | Independent observable assertion |
| --- | --- | --- |
| Mixed documents | Add a KerML contract and SysML subsystem referencing it against both accepted standards. | Each operation returns a distinct revision; mandatory endpoints resolve canonically and both language facades agree on shared identities. |
| Three edits | In the self-model/rich fixture, r1 has a workspace repository, r2 adds a port, r3 specializes the subsystem and redefines the repository. | Retained r1/r2 source bytes, digests, diagnostics and query answers remain unchanged; r3 suppresses the redefined inherited feature without creating an inherited copy. |
| Identity reconciliation | Edit a disjoint declaration with syntax identity preserved; separately replace a whole document without reconciliation. | The former retains unaffected authored IDs; the latter makes no unsupported identity-retention promise. Source revisions differ while retained standard IDs do not. |
| Remove and repair | Remove a document providing a referenced Definition, then restore equivalent source in a later revision. | Removed declaration no longer resolves; the broken revision is Working and cannot validate; earlier validated handles remain readable. Restored content does not resurrect retired IDs without authorized reconciliation. |
| Recovery | Insert an unclosed declaration and then repair it. | Working retains exact malformed bytes, syntax recovery and any safe construction; query unavailability/incompleteness is explicit; no stale previous graph is presented as current. Repair can validate against the same standards. |
| Operational failure | Apply a change from r1 after r2 is head; edit a non-UTF-8 boundary or unknown document. | The error leaves head, revision inventory, sources and identity reservations unchanged. |
| Certificate invalidation | Make a local edit that introduces a previously absent owner/type or provider; separately make an unrelated additive change. | Relevant negative answers reopen; unaffected witnesses survive only through checked revalidation. Compare resulting answers with a fresh independent authored construction. |
| Shared standards | Open two workspaces and create several revisions of each. | Standard publication handles/record sharing and exact IDs remain unchanged; authored queries cannot mutate library ownership or make library roots see authored names. |
| Trusted dependencies | Open with exact trusted publication restoration; alter one bound profile/registry/source/publication identity in a negative fixture. | Exact restore does not rerun library producers; mismatched receipt/context is rejected before constructing an accepted workspace dependency. |
| Validation type boundary | Try to validate incomplete closure, unresolved endpoint, recovered syntax and blocking diagnostics individually. | None produces `ValidatedProjectRevision`; an accepted fixture does. No public constructor or mutation bypass exists. |
| Effective typing rejection | Declare an AttributeUsage typed by a PartDefinition, then repair the definition to AttributeDefinition. | The invalid revision retains its canonical typing and complete producer/reference evidence but cannot validate; the repaired audit has a new exact context, and the old Invalid answer is unchanged. |
| Programmatic equivalence | Build equivalent rich subsystem declarations with kernel/language builders and via authored source. | Compare semantic projections and relationships modulo authored-ID mapping, preserving standard IDs and evidence/completeness obligations. Equal text-generated IDs alone is not the oracle. |
| Validated scaling/read isolation | Create 50 KerML and 50 SysML documents, retain five Validated revisions after port/redefinition edits, and read all five with four barrier-synchronized independent readers. | Each revision retains 100 documents; values, canonical identities, context and bounded evidence match baselines captured before later edits. Observe physical shared standard storage and no accepted-library producer replay; no hard speed threshold. |
| Recovery at scale | Retain the original five-revision removal/repair fixture, whose fourth revision has 99 documents and is Working. | Current unresolved evidence and old immutable answers remain correct. This supplements the five-validated-revision fixture rather than counting its Working state as Validated. |

The permanent self-model semantic tests remain in the language acceptance layer;
workspace editing tests consume it as a fixture. Implementation traceability and
Cargo dependency checks remain separate, so language semantics never read Rust
manifests to establish model truth.

## Initial cost model and remaining incrementality

Standard publication is a rare acceptance computation whose accepted result is
cached. Its workflow budget is distinct from the cost of frequent authored edits.
The future authored invalidation path is:

```text
document edit
    -> query/provider footprint invalidation
    -> affected producer populations
    -> closure certificate revalidation
    -> new immutable revision
```

ADR 0027's component planner, exact comparator and boundary tests may evolve into
this machinery. Current broad provider/search footprints do not prove useful
independent strata. The initial workspace therefore does not require component
sealing. Future invalidation must preserve negative searches, future producer
opportunities, relationship endpoints and accepted dependency identity, and must
compare resulting semantics with a full authored recomputation. No incremental
performance claim follows from immutable revision storage alone.

SourceProject currently rebuilds candidate dependencies across its complete
document input. Phase 1 may rebuild authored construction/overlays where necessary;
do not claim a fully incremental compiler. Unchanged syntax/source values and
accepted standards must remain shareable. Tiny edits must not copy whole standard
graphs or rerun their producers. Observe local producer evaluation counts and
standard record sharing in the scaling test, not timing alone.

The historical [storage review](modeling-workspace-frontend-boundary.md#shared-dependency-storage-review)
found shared record/proof payloads but full standard map/index replication at its
recorded source. The integrated kernel now uses immutable shared base tables and
local storage; kernel invariants and exact accepted-cache identity restoration
have passed. Workspace self-model, Working-state and validated-scale tests
observe physical shared standard tables, original record identities, zero copied
dependency entries and zero accepted producer replay. Recovery at scale also
passes with old revision answers intact. Facade Arc equality alone is
insufficient, and authored index rebuilding remains an explicit initial
limitation. The earlier
construction-lifetime analysis and its unmeasured savings remain historical.

### Minimum shared-storage implementation

The following implementation plan and ownership handoffs are retained historical
design rationale. Current implementation status is recorded above; these proposed
private type names and pre-integration gaps are not additional unfinished gates.

Read-only design review updated against `695e67e`, including selected contribution
and sealed dependency proof support; no storage implementation or benchmark is
claimed. Keep the current public `ModelView`, Snapshot, construction and overlay
contracts. Use the existing immutable dependency handle as the shared base and
ordinary local `BTreeMap` storage; no new collection dependency is needed.

The proposed private representation is:

```rust,ignore
struct ModelView {
    registry: Arc<MetamodelRegistry>,
    dependency: Option<Arc<DerivedOverlay>>,
    declared_source: Option<DerivationInput>,
    local: Arc<LocalModelTables>,
    indexes: Arc<LocalIndexes>,
}
```

`LocalModelTables` contains only this project's current records, occurrences,
explicit derived navigation, computation failures, searches and optional
ordered-reference contributions. Reconstructed inverse/association projection
slots belong to `LocalIndexes`, not to the explicit derived-navigation table.
A standalone publication uses the same representation without a
base; a dependent publication can itself retain its existing dependency chain.
Record IDs and occurrence IDs must be disjoint from the entire base, not resolved
by a local-wins overwrite rule. Local records may reference protected IDs.

Each revision owns its complete local population; it does not point to the prior
revision as another lookup layer. This bounds dependency traversal by publication
depth rather than edit count. Published tables/indexes are immutable. Staging
clones only local maps, sharing record payloads; additive materialization moves
local storage with `Arc::try_unwrap` when possible. Old readers force local copies
only. A local derived overlay can initially copy the local declared map; avoiding
that authored copy is not required for phase 1.

Implement the storage boundary in this order:

| Files / internal boundary | Required change |
| --- | --- |
| `kernel/src/model.rs` (`ModelView`, `DerivationModelParts`, `Snapshot::stage`, `apply`, `preview`) | Introduce the private local tables/indexes and a borrowed candidate lookup over dependency plus local tables. Mounting creates empty local storage. Builders mutate local tables only; collision checks and endpoint lookup see both populations. Replace direct map access with explicit local-mutation or combined-read helpers. |
| `model.rs` (`SnapshotData`, `ConstructionView`) and `derivation_input.rs` | Keep only local live/retired identity reservations. `has_used` checks local history, dependency live derived IDs and dependency reserved history recursively. Preserve protection of retired dependency IDs as well as live records. Construction/strict equivalence compares the same dependency, local data and local identity history. |
| `model.rs` query methods and `association.rs::project` | Preserve borrowed results, ordering and merged navigation using the index rules below. Give validators borrowed combined lookup/iteration instead of requiring one flattened map. Keep strict bounds, endpoint typing, uniqueness, inverse multiplicity and protected composite ownership checks. |
| `derived.rs`, `derived/construction.rs`, `derived/archive_restore.rs`, `derived/proof_graph.rs` | Store local explanation lookup/interner tables; `explain`, `facts`, proof-existence checks and search lookup fall back to the exact dependency. Do not clone its explanation/search pools or cached proof adjacency. Local proofs may cite base facts; base proofs cannot acquire local premises. Preserve local cycle checks, failure checks and exact-input strict promotion. |
| `archive.rs` | Decode dependent archives directly into local tables and retain the supplied dependency. Stop extending decoded maps with its records, occurrences and metadata. Encode the same logical ordered entries/reservations as today through merged iterators, preserving archive bytes and existing trusted receipts. Validate reservation subsets by borrowed membership; discard redundant decoded base reservations after validation. |
| `kerml-semantics/src/producer_closed_dependency.rs` and the authored frontend | Continue mounting the same authenticated publication Arc. Reuse one witness across workspace revisions; do not add a language acceptance constructor in the kernel. The existing source-input/result carrier owns local compilation and the workspace owns history. |

The kernel may use a private `model/storage.rs` module for borrowed lookup and
sorted merge helpers. These are implementation helpers, not a generic storage
backend trait or a new public collection API. Keep the extension-registry check:
old descriptor/class/index meanings must remain valid under
`require_extension_of`. Validate any newly applicable navigation obligations in
the combined registry; do not assume that all required association navigation
is covered by the old class property set. Such validation can borrow the base
without rebuilding its tables. The ordinary accepted Systems project mount uses
the already combined registry.

#### Implementation ownership handoff

Use three owners after readiness. The storage changes below remain sequential
within the kernel owner; frontend preparation can proceed against the existing
public graph APIs while that internal refactor runs.

| Owner | Files and reusable boundary | Deliverable handed to the next owner |
| --- | --- | --- |
| Kernel shared storage | `crates/kernel/src/model.rs`, `derivation_input.rs`, `association.rs`, `derived.rs`, `derived/{construction,archive_restore,proof_graph}.rs` and `archive.rs`. Preserve `Snapshot::with_immutable_dependency`, construction/strict views, borrowed `ModelView` queries and derived builders. | Shared dependency tables plus local records, indexes, reservations and proof pools, with the existing public graph/query contracts. Supply the actual storage observations required by held `tests/support/mod.rs`; workspace code must not inspect private maps or manufacture tokens from facade pointers. |
| Frontend Working compilation | `crates/kerml-text/src/project.rs`, `sysml.rs`, `sysml/source.rs`, `library/{mod,construction,refinement}.rs`, `lowering.rs` and the public exports in `lib.rs`. Extract the existing `ProjectDocument` parse/edit and `Arc` document map handling; reuse `AcceptedSourceDependency`, `LibraryDraft`, reference/source metadata and the existing local scheduler. | The additive `SourceInputs`/`SourceCompilation` boundary specified in the frontend review: immutable inputs, independently retained identity history, unavailable/construction/strict result, exact diagnostics/references and borrowed query factories. Preserve the strict `SourceProject` adapter. Own the small scheduler-observer plumbing in `kerml-semantics/src/producer_worklist.rs` so compilation retains actual evaluated subjects across reconstruction. |
| Workspace revisions and validation | After the gate, add `crates/modeling-workspace/src/{lib,revision,validation}.rs` and its manifest/workspace registration. Implement against the frontend carrier; keep `tests/{phase1,working_states}.rs` and `tests/support/mod.rs` as the existing acceptance boundary. | Expected-head transactions, independent workspace revision IDs, retained Working handles, checked Validated handles and direct language query delegation. Expose verification-only storage/scheduler observations by forwarding the owning layers' observations. No second semantic store or nested `SourceProject` history. |

Settle the frontend carrier signatures first so the workspace owner can implement
history and error handling without editing lowering. `SourceProject` currently
rejects recovered syntax before calling `lower_accepted_source`, which then
requires `LibraryDraft::strict_snapshot`; neither is a Working result carrier.
The frontend owner must retain the construction frontier and pending source
populations before attempting strict promotion. Reuse
`ProducerClosedDependency::{project_construction_context,project_construction_overlay_context,project_overlay_context}`
and `SysmlSemanticContext::for_closed_dependency`; keep accepted mount ownership
inside the frontend compilation rather than exporting its private helper.

Two cross-owner boundaries need explicit review. The kernel owns checked live and
retired identity reservations; the frontend owns the distinction between temporary
recovery omission and explicit document removal across compilations. Any new
construction-lineage operation must preserve both rather than minting a fresh
identity history at each Working edit. The kernel also owns storage accounting,
while the frontend owns scheduling observations; the workspace merely forwards
them. Complete frontend Working tests and all four storage slices before the
held 100-document/revision suite can establish phase I8. None of these owners
needs a new language acceptance authority or an accepted-standard replay.

#### First implementation slices after readiness

Keep these as sequential, reviewable kernel changes before integrating the
workspace. Intermediate commits do not establish I8 until all four slices pass.

| Slice | Concrete first changes | Boundary required before proceeding |
| --- | --- | --- |
| 1. Borrowed storage access | Add private `LocalModelTables` and `LocalIndexes`; move the current fields into them without changing public results. Introduce read helpers for records, occurrences, statuses, searches and explanation lookup. Replace validators' concrete `&BTreeMap` inputs with one private borrowed candidate view supporting lookup and sorted iteration. Keep mutations explicitly local. | The existing flat build still passes with identical canonical bytes. A test-only flat oracle remains independent of the new layered merge. No blanket local-wins record lookup is introduced. |
| 2. Dependency-backed snapshots | Add the dependency handle to the read view; make `with_immutable_dependency` start with empty local tables/indexes and local identity reservations. Thread this through `stage`, `apply`, `preview` and `DerivationInput::build_model`. Construct only local class/reference buckets plus touched projection groups; validate endpoints and ownership against the borrowed combined view. | Empty mounts contain zero dependency-sized local tables. Retained old snapshots and construction views preserve answers. Local references to dependency IDs are visible without allowing writes or ownership transfers into the dependency. The compatible-registry path gets the same checks, not a full-map fallback. |
| 3. Local derivation storage | Change first `DerivationBuilder::build_inner` to start empty local explanation/search pools instead of cloning `dependency.inner` pools. `DerivationModelParts` transfers local tables plus the shared dependency handle. Additive `Arc::try_unwrap` moves only local frontier storage; shared frontiers clone only those local maps. Route `DerivedOverlay::explain`/`facts` and proof-existence checks through local/dependency lookup. | Creation/append contributions, strict/construction promotion, local cycle detection, historical dependency proofs and failed computations retain their current meanings. Instrument both first and later materializations: no base interner, proof adjacency or index copy is hidden there. |
| 4. Archive and workspace integration | Remove dependency `records.extend`, `links.extend`, navigation/status/search copying and contribution-map cloning from dependent restoration. Retain the supplied dependency object, validate decoded local data against it, then connect the frontend compilation owner and revision handles. | Root and dependent archive compatibility, exact restoration, language proof-boundary controls and the 100-document/five-revision sharing fixture pass. Only then report I8 complete. |

Merged iterators need a concrete private cursor implementation. Recursively
calling a dependency's opaque `impl Iterator` cannot define a finite return type;
use ordered per-layer cursors with storage proportional to dependency depth,
without collecting all base keys. Resolve matching record/occurrence IDs as a
validation error. Projection groups are the explicit exception: their combined
replacement shadows the base group, never its canonical record.

Keep separate internal counters for local records, occurrences, reservations,
class/reference index entries, touched projection entries, explanation lookup
entries, proof adjacency and contribution entries. A local incoming edge may
legitimately create a bucket keyed by a dependency ID, so checking that every
index key is authored would be wrong. Check retained entries and their carriers,
plus unchanged dependency allocations, rather than keys or record pointers alone.

#### Concrete kernel handoff after publication readiness

Read-only recheck against `9152fb9`: the four slices above remain the minimum.
`Snapshot::with_immutable_dependency` still clones `ModelView`, and the first
`DerivationBuilder::build_inner` clones the dependency explanation/search pools.
`ProducerClosedDependency::project_snapshot` adds no separate copying step or
acceptance exception. No storage code, build or cache load accompanied this
recheck.

Use these private boundaries to make the first patches mechanical; the names
below are proposed implementation names, not additional public graph contracts:

| Slice | Concrete internal seam and acceptance discriminator |
| --- | --- |
| 1 | `model/storage.rs::CandidateView<'a>` borrows the registry, optional dependency and local records/occurrences. Supply `element`, `occurrence`, sorted record/occurrence cursors and reservation membership; use it in `model.rs::validate` and `association.rs::project` instead of concrete merged maps. A finite cursor holds one map/set iterator per publication layer and chooses the next key without collecting dependency keys. Public borrowed iterator signatures remain unchanged. |
| 2 | `LocalIndexes` separates local entries from `replacement_groups: BTreeSet<(ElementId, PropertyId)>` and retained combined projection/reference entries. Suppress base reference entries only for these groups. Preserve reference ordering `(source, property, position, target, carrier)` independently of association slot ordering `(explicit_position, target, occurrence_id)`. In `shared_dependency_projection.rs`, the first oracle must still shift the untouched base occurrence to position 2, retain both duplicate-target occurrences, round-trip identically and restore the base projection after removal; the second must reject the scalar inverse conflict without reserving the rejected IDs. Add physical sharing assertions to these existing oracles. |
| 3 | `OverlayData` owns a local explanation map and local interners. Add private `explanation_for`/`contains_derived_fact` lookup shared by strict/construction build, promotion and archive restoration; lookup falls back to the exact dependency. `proof_graph.rs` traverses local facts and their owning local pool, checking base premises before treating them as terminal. Do not intern base proofs merely to obtain cached adjacency. The minimum implementation needs no cross-layer interner deduplication: equal newly submitted local proofs may occupy the local pool. |
| 4 | `archive.rs` serializes merged logical entries/reservations in existing order but restores only local containers. Keep `declared_slot`, original-origin lookup and optional contribution lookup layer-aware; retain sparse current navigation on standard IDs. Existing context/digest code continues consuming the same logical public iterators, so allocation tokens never enter identities and historical KerML receipts require no migration. |

Introduce a non-default kernel `storage-observation` feature, forwarded by the
held workspace's `verification` feature, for the physical observer. `cfg(test)`
alone cannot expose it to downstream integration tests. A hidden testing module
should report opaque allocation tokens plus typed entry counts for actual model
tables, indexes, reservations, explanation lookup, proof adjacency, search pools
and contribution tables, recursively for both accepted layers. Inspect retained
containers; do not derive tokens from publication facades. Report locally
recomputed projection groups separately, including their base carriers, rather
than misclassifying those necessary projections as copied standards. This feeds
the existing `publication_storage`/`dependency_storage` assertions at empty mount,
two independent projects and all five retained 100-document revisions.

One cross-owner API choice remains before Working implementation: a checked
identity-lineage/reconstruction seam from a previous `ConstructionView`.
Currently `preview` retains candidate reservations but exposes no operation to
advance that lineage, and lowering publishes from a strict `Snapshot`. Kernel
and frontend owners must settle its signature and recovery-versus-deletion
reservation contract together; a fresh dependency mount per edit is insufficient.
This does not require exposing local tables or changing publication acceptance.
No remaining storage representation choice requires a new backend, inherited
element copies or a change to query completeness.

#### Index and navigation rules

Record/class/occurrence iteration merges disjoint sorted streams. Incoming,
outgoing and incidence queries merge base and local occurrences, preserving
today's sort key and carrier identity. `incoming_for_property` filters each
layer's indexed bucket, then merges; base offsets cannot index a combined vector.
Class queries retain the current exact/subtype-inclusive meaning. No query may
return only the base answer merely because its subject is a standard ID.

`navigation_slot` returns `Option<&Slot>`, so a temporary concatenated value is
insufficient. `LocalIndexes` must retain sparse **combined projection slots** for
each association/inverse group touched by local carriers. Build those slots from
both populations, validate the complete group's bounds, uniqueness and ordering,
and retain all contributing occurrence provenance. Untouched groups borrow the
base projection. Scalar inverse conflicts remain errors. These project-only
projections do not replace a standard record or mutate the publication's view.

Association reference positions need special care: `ModelView::build` currently
enumerates unordered occurrence positions in occurrence-ID order. A local link
whose ID sorts before a base link changes the latter's position in the combined
view. Rebuild reference entries for that affected `(source, property)` group and
suppress its old base entries during merged incoming/outgoing reads. Apply this
replacement to every affected target/property bucket, retaining occurrence IDs;
do not blindly append local entries or deduplicate equal endpoints. Ordered
positions remain explicit and must pass the existing contiguous-order check.
Allocate only touched groups/buckets, never all standard index buckets.

Local edits/removals rebuild local indexes and touched projections from the base
and current local population, so stale local inverse entries need no historical
tombstone chain. Reject composite crossings before treating containment as
independent local/base graphs: neither a new local owner of a standard element
nor new ownership under a protected standard record is permitted. Noncomposite
incoming relationships remain visible to project queries and their provider
closure checks. Full borrowed validation/digest scans may remain initially;
temporary allocations must be measured separately and must not flatten the base
into new graph/index tables as a hidden builder step.

Original declared-slot lookup must route standard subjects to the dependency's
declared projection and local subjects to this revision's declared input. Derived
stored facts, failures and contribution-cache lookup follow their owning layer.
Current inverse/association navigation must first consult the sparse combined
projection: subject membership alone does not make that value a dependency fact.

#### Proof and restoration boundaries

`SemanticContext::sealed_dependency_fact` authenticates an actual record, slot
or occurrence against the accepted/producer-closed dependency. Preserve its
exact value-and-origin check and shared-pointer fast path. Storage sharing alone
confers no language seal. A local inverse carrier can change a standard subject's
navigation slot while its record remains identical. Queries must then retain
the current population evidence. Historical searches of an exact sealed fact
stay in the dependency proof context; independently requested current searches
remain live, including when their values equal a historical search. Preserve
ordinary and producer query modes and both merge orders.

Keep dependency explanation/search pools owned by their publication. Local
proof-existence checks may borrow dependency facts, and `explain`/`facts` must
still expose the same logical assertions in deterministic order. Do not insert
every dependency proof into a local pool to make
`ExplanationPool::derived_dependencies` work: that copies the cached adjacency
the sharing change is meant to avoid. Local cycle checking may treat a validated
immutable dependency as a terminal boundary because it cannot point into the
new local layer; it must still traverse previous **local** explanations when an
additive batch changes an existing local proof. Missing and unsuccessful base
premises retain their existing rejection rules.

The ordered-reference cache now covers both fresh ordered-slot creation and
later append events. Preserve selected position, exact explanation and searches;
whole-slot search submissions still discard earlier precision when they cannot
be attributed. Legacy structural dependent archives retain the supplied
dependency's cache by lookup but omit local contributions; queries then use
aggregate evidence and transport must revalidate lost precision. The distinct
dependent evidence archive used for Systems cache transport preserves exact local
positions, proofs and searches and authenticates the supplied dependency's
selected support. Frontier archives additionally retain the unaccepted scheduler
continuation. These formats confer no acceptance themselves. Preserve each
format's bytes and authority contract; sharing does not justify changing semantic
identity or silently substituting one archive format for another.

Archive wire output retains its current logical ordering, proof/search table
assignment and complete identity reservations. Stream merged reservation sets
through a borrowed serializer rather than rebuilding a retained base-sized set;
decoded reservations may be validated and reduced to local history. Preserve
the declared/derived overlay boundary and original slots used by producer model
digests. Do not serialize transient producer-emitter attribution, bounded future
scope state or evaluator caches as canonical graph data. Existing dependent
archive protection and accepted receipt validation remain authoritative.

Shared storage does not itself justify preserving a closure witness. Context
digests, original declared slots, provider/search guards and Complete/Incomplete
meanings remain unchanged.

#### Storage-specific acceptance matrix

Extend the existing kernel tests before using the representation in the workspace:

| Existing test area | Additional discriminator |
| --- | --- |
| `immutable_dependencies`, `snapshots`, `construction` | Two projects and retained old revisions share the exact dependency tables/indexes. Local edit/remove/recovery cannot rewrite dependency records, occurrences, composite ownership or retired identities. Local tables contain no protected records or copied base reservation population. |
| `association_slots`, `derived_associations` | Local noncomposite carriers into a base subject appear in combined incoming, property-filtered incoming, incidence and navigation; the base facade stays unchanged. Check scalar inverse conflict, duplicate targets with distinct occurrences, ordered positions and a smaller local occurrence ID shifting unordered positions. Removing the local link restores the exact base projection. |
| `registry_extensions` | Existing classes reuse base index populations under a compatible extension; new local subclasses are found correctly. Incompatible descriptors and newly unsatisfied navigation bounds still reject. |
| `derivations`, `reference_contributions`, `structural_search_sharing` | Local proofs resolve base premises without copied base pools. Strict/construction/additive paths preserve declared versus derived facts, exact contribution support and fallback; changing a local carrier still reopens relevant negative/provider closure. |
| KerML sealed dependency / selected contribution controls | A sealed historical Incoming search does not become a current producer read; an identical explicit current search survives either merge order. A changed inverse projection on a protected subject remains live. Fresh-slot and append support keep precise guards; loss of local metadata on restore reopens transport conservatively. |
| `archive`, accepted publication restoration | Dependent roundtrip retains the supplied base allocation and canonical bytes/digests. Protected archive writes/reservations reject; legacy optional-contribution fallback, lossless evidence transport and frontier continuation remain distinct. Historical accepted receipt checks still pass. |
| Workspace 100-document/five-revision fixture | Observe local/base table entries, index entries, proof-pool entries and touched projection sizes, plus unique retained storage and peak temporary memory. Empty mounts allocate no standard-sized maps; tiny edits rebuild only authored/touched index populations. Parallel readers see revision-specific results and no standard producer replay. |

For small neutral graphs, compare every public graph/query projection with an
independently flattened test oracle using the existing builder, including order,
origins, failures and search evidence. Keep this oracle test-only. Add internal
storage observations rather than exposing allocator identity as a semantic API.
Run the kernel and language package gates, archive/receipt checks and workspace
fixture after implementation; none has been run for this design-only review.

## Future semantic recomputation after an authored edit

The first workspace may recompute the authored semantic population while borrowing
accepted standards. Shared storage, syntax reconciliation and semantic
incrementality are separate properties. A rare standard publication can be
expensive and cached; a frequent authored edit must eventually invalidate only
the work its changes can affect. No publication timing is an authored-edit target.

The intended future path is:

```text
document edit and exact source revision
    -> query/provider footprint invalidation
    -> affected existing and newly applicable producer populations
    -> closure certificate revalidation
    -> new immutable revision
```

Footprints must include positive facts, absent identities, negative searches,
provider availability, applicability/class changes, future helpers and relationship
endpoints in both directions. A local relationship to a standard element can
change a current incoming population without changing any accepted record.
Preserving old values is insufficient: unchanged closure also requires unchanged
relevant producer opportunities and closed search populations. Deletions and
disappearing opportunities must remove obsolete local facts and certificate rows.

Reuse the existing ProducerEffect and per-effect scope contracts. ADR 0027's
component planner can become an invalidation planner as read-side footprints gain
proof, with conservative fallback when a boundary is unknown. It remains research
infrastructure: this workspace does not depend on independent component sealing,
source-document SCCs, or an adopted ADR 0027. Do not rerun Systems partition
planning to justify an authored edit.

Any future incremental path must equal the reference authored recomputation for
declared/derived graphs, identities, ordered relationships, provenance, positive
and negative/search evidence, certificate contents and revision-bound effective
query answers. Test insertions into previously empty searches, removals, scope
changes and standard-targeting local relationships, and retain old revisions for
parallel readers. Allocation identity is a sharing observation, not semantic proof.

SQLite persistence, server/API migration, Systems Modeling API, diagrams,
transformations, execution IR, simulation, assistant features and Gen1 migration
remain outside this in-memory phase. Full language conformance remains a separate
coverage register, not another general foundation milestone.
