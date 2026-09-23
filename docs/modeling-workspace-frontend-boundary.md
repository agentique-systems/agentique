# Working source revisions: frontend boundary

Status: preparation for ADR 0024; production workspace integration remains gated.
This reviews the prepared `SourceProject::with_accepted_sysml_standard_libraries`
path in the authored-integration workstream. It adds no production API.

## Reuse and remaining gaps

The accepted constructor already supplies the right semantic dependency:
`AcceptedSourceDependency` mounts the accepted Systems overlay, which retains
accepted KerML, and creates both language query contexts. Its construction and
construction-overlay context methods already accept pending namespace and
specialization populations. Reuse them; a Working revision needs no second
semantic store or new kernel interpretation contract.

The existing strict source history cannot directly represent all Working states:

| Current boundary | Required change |
| --- | --- |
| `SourceProject::apply` rejects any incomplete production document before lowering | Preserve the document and its recovery diagnostics in a successful Working input revision. |
| `lower_accepted_source` ends through `draft.strict_snapshot()?` | Retain the candidate/partial overlay when lower-bound obligations remain. Build reference diagnostics against this exact frontier before strict promotion. |
| `ProjectRevision::revision()` delegates to `snapshot().revision()` | Assign the workspace revision independently, including source-only revisions. Expose an optional kernel revision as a separate value. |
| Candidate query contexts currently receive empty pending-scope sets | Carry omitted/recovered source populations into every construction query and producer context. A missing graph edge is not proof that the source population is closed. |
| `SourceModel` always owns a strict snapshot; public graph/query access is unconditional | Add an explicit graph-state carrier and fallible query access for Working results. Preserve the existing strict API. |
| `ReferenceAssertion` already retains resolution evidence and source origin | Reuse it on construction queries; retain Invalid/Incomplete and endpoint mismatch without converting them to success. |
| Document removal already exists in `ProjectChange` | Reconstruct from the current document set. A missing required target becomes a Working obligation, so removal must no longer fail solely at strict promotion. |

The internal document map now uses `Arc<ProjectDocument>` while preserving
borrowed accessors. Cloning it shares unchanged source and syntax arenas; that
previously identified fix is implemented.

## Recommended additive carrier

Extract immutable source-input preparation and compilation from the existing
strict history wrapper. Keep one lowering implementation. The workspace owns
head/history and consumes this additive frontend API; it need not wrap a second
frontend-owned history. Suggested signatures are design, not implemented symbols:

```rust,ignore
pub struct SourceInputs { /* project/root, Arc documents, dependency, identity history */ }
pub struct SourceCompilation { /* Arc<SourceInputs>, frontier, evidence, diagnostics */ }

impl SourceInputs {
    pub fn with_accepted_sysml(
        publication: Arc<CanonicalSysmlSystemsLibrary>,
    ) -> Result<Self, SourceBuildError>;

    pub fn apply(
        &self,
        changes: impl IntoIterator<Item = ProjectChange>,
    ) -> Result<Self, SourceEditError>;

    pub fn compile(
        self: &Arc<Self>,
        previous: Option<&SourceCompilation>,
    ) -> Result<SourceCompilation, SourceBuildError>;
}

// Private owned representation: callers cannot replace evidence or graph parts.
enum SourceFrontier {
    Unavailable(QueryUnavailable),
    Construction {
        declared: Arc<ConstructionView>,
        semantic: Option<ConstructionOverlay>,
    },
    Strict {
        declared: Snapshot,
        semantic: Option<DerivedOverlay>,
    },
}
```

The compilation retains roots, source map, mandatory reference assertions,
explicit pending scopes, producer status and an optional closure certificate.
The certificate belongs to the exact currently queried frontier, including any
derived overlay; switching to its declared graph cannot retain that certificate
without checked revalidation. The owning compilation keeps immutable graph data;
query contexts/caches borrow it and are created per reader. No self-referential
Rust owner/borrower structure is needed.

`SourceInputs::apply` is pure input preparation: it allocates/reconciles document
and syntax identities but cannot publish a workspace head. Compilation retains
an `Arc` to those exact inputs. The workspace performs its expected-head check,
prepares and compiles, then records one new Working revision atomically. A
language-invalid result is still a successful compilation result. Invalid edit
spans, missing document commands, duplicate paths, resource failure and invalid
accepted dependency identity remain typed operational errors that publish nothing.

Do not blanket-convert every `LibraryLoadError` into a language diagnostic:
classify expected syntax/unsupported-construction/unresolved-input failures
separately from internal invariant and dependency errors. If user input cannot
produce a safe kernel construction, retain it with `Unavailable` and diagnostics.
If producers cannot complete a safe construction, retain its partial state and
status; never attach a certificate from an earlier frontier.

The existing strict `SourceProject` constructors/apply path can call this shared
input/compile pipeline and require a strict result, retaining their current
return types and history behavior. Do not silently change `ProjectRevision`
snapshot/query guarantees, or overload its kernel `RevisionId` to mean a
source-only workspace revision.

## Recovery, removal and identity

For the first Working implementation, partition inputs by complete and supported
production syntax. Keep every original document. Lower complete documents; omit
an unsafe document as a whole and mark the project root namespace pending in
all query/scheduler factories. This is deliberately conservative. It prevents a
missing provider from proving a negative conclusion and requires no fabricated
declarations or partial-grammar lowering. More precise safe-subtree lowering is
future work. If the remaining inputs themselves cannot form a safe construction,
return explicit query unavailability.

A removed, previously complete document is genuinely absent from the new input.
Its old declarations/derived outputs must disappear, and remaining required
references become explicit failures or construction obligations. Do not mark a
removed document as merely pending forever. Re-adding the same path receives a
new DocumentId; replacement/edit reconciliation follows the existing syntax
contract and does not promise restored historical ElementIds.

Preserve identity history across Working-to-Working and Working-to-strict
transitions. `ConstructionView` carries used identities internally, but does not
currently expose a public `change_set`/advance API. `lowering::publish` preserves
strict Snapshot lineage only. The frontend must retain a source identity ledger
or equivalent checked reconciliation state covering intermediate Working inputs,
including explicit removals. Do not reset identity history to accepted-standard
roots or present the last strict graph as the new revision. A temporary omission
for recovery is distinct from an explicit deletion: it cannot by itself retire
every syntax identity in the document. The implementation should test this
distinction before claiming Working identity continuity.

## Exact query facade

```rust,ignore
impl SourceCompilation {
    pub fn inputs(&self) -> &SourceInputs;
    pub fn strict_snapshot(&self) -> Option<&Snapshot>;
    pub fn construction(&self) -> Option<&ConstructionView>;
    pub fn kernel_revision(&self) -> Option<RevisionId>;
    pub fn diagnostics(&self) -> &[SourceDiagnostic];
    pub fn references(&self) -> &[ReferenceAssertion];
    pub fn producer_status(&self) -> Option<&AuthoredProducerStatus>;
    pub fn producer_closure(&self) -> Option<&Arc<ProducerClosureCertificate>>;
    pub fn kerml_queries(&self) -> Result<KerMlQueries<'_>, QueryUnavailable>;
    pub fn sysml_queries(&self) -> Result<SysmlQueries<'_>, QueryUnavailable>;
}
```

Construction and partial overlays remain queryable through the mounted
dependency's existing context factories, carrying exact obligations and pending
scopes. Compose SysML with `SysmlSemanticContext::for_closed_dependency` using
that same KerML context. `QueryUnavailable` distinguishes no safe construction
from an unsupported frontend/interpretation; it never selects an older revision.
An available evaluator can return Incomplete/Invalid answers normally. Query
availability is separate from validation, and a strict snapshot alone does not
make a `ValidatedProjectRevision`.

`SourceDiagnostic` may unify syntax, construction, reference and producer
diagnostic envelopes while retaining their native evidence/source identities;
it must not become a second semantic graph. Existing source-origin and query
evidence structures remain authoritative.

## Shared dependency storage review

Read-only inspection at `b49afe5`; no accepted cache was loaded and no scaling
result is claimed. The existing code shares canonical record/proof payloads,
but **does not yet provide a borrowed standard graph with local-only storage**.
Consequently publication/record `Arc` equality alone cannot establish phase I8's
no-standard-graph-copy requirement.

| Call path / retained value | Shared allocations | Copied or rebuilt storage |
| --- | --- | --- |
| `SourceProject::with_accepted_sysml_standard_libraries` -> `AcceptedSourceDependency::new` -> `producer_closed_dependency` | Systems overlay, exact nested KerML publication, closure certificate and naming extension | A new checked mount/context per project; no graph copy in the witness itself. |
| `lower_accepted_source` -> `ProducerClosedDependency::project_snapshot` -> `Snapshot::with_immutable_dependency` | Registry and each `Arc<ElementRecord>`; record slots and their values therefore remain shared. Explanation, structural-search-set and ordered-contribution payloads remain shared. | `ModelView::clone` copies all record-map nodes, full navigation/class/reference indexes, association occurrence values, derived navigation/status maps and proof/search lookup-map nodes. Used-ID/occurrence sets are copied. This includes the standard population. |
| `Snapshot::preview` / `apply` -> `stage` -> `ModelView::build` | Unchanged record Arcs and immutable dependency handle | Full record/occurrence maps and used-ID sets; complete indexes are rebuilt. Mutated records use copy-on-write. `lowering::publish` rewrites every local declared record, even unchanged local records. |
| First `DerivationBuilder::build_inner` for a revision | Declared snapshot handle, standard records, explanations and search sets | Another full merged model map/index population; explanation/search intern tables are cloned. `ExplanationPool` also clones cached derived-dependency vectors. |
| Later additive producer materialization | Existing payloads; unshared previous overlay storage is moved with `Arc::try_unwrap` | Indexes are rebuilt; outstanding readers force map/interner clones. Normal worklist execution drops its context and consumes the prior frontier before building; the reference full-scan strategy deliberately retains it. |
| `ProjectRevision` / `SourceProject::history` | Unchanged `Arc<ProjectDocument>` values and accepted publication handles | One document path map, source map, diagnostics/references, declared model and derived model per revision. The overlay's declared handle aliases the revision snapshot; it is not a third declared allocation. History also retains the initial empty revision. |
| Borrowed query facade / checkpoint | Queries borrow `ModelView`; certificates and mounts use Arcs. A checkpoint shares the certificate's read metadata. | New query contexts validate/hash the entire merged graph and own fresh evaluator caches. Checkpoints copy compact certificate arrays and add subject fingerprints, but retain no graph/indexes. |

The relevant implementation is in [project.rs](../crates/kerml-text/src/project.rs),
[authored source lowering](../crates/kerml-text/src/sysml/source.rs),
[dependency mounting](../crates/kerml-semantics/src/producer_closed_dependency.rs),
[kernel snapshots](../crates/kernel/src/model.rs) and
[derived storage](../crates/kernel/src/derived.rs). Slots are owned by records,
not individually Arc-backed: modifying one shared record clones its slot map and
values, while its derived explanation Arcs remain shared. Association occurrences
and computed inverse/navigation slots are separate owned values, so sharing the
record pointer does not cover them.

For phase I8, retain one authenticated mount and immutable standard storage, then
use an additive kernel view with shared dependency lookup plus local records and
local navigation/index deltas (or an equivalent persistent representation).
Deterministic iteration and incoming/source relationship queries must merge both
populations, including local relationships whose endpoints are standard IDs.
Those incoming facts can keep a query population open even though the standard
record is immutable. Preserve the existing write/ownership protections, exact
context digests, original declared-slot lookup and closure invalidation; a simple
fallback to the dependency's closed answer is unsound. Wrapping today's full
`ModelView` in an Arc only postpones its copy to the first edit.

This is a storage implementation task after readiness, not another language
conformance gate. Rebuilding **authored** indexes and producers can remain an
explicit phase-1 limitation. Rebuilding full standard indexes/maps per retained
revision must be measured and reported as current replication, not described as
completed no-copy sharing or hidden under future incrementality. The 100-document,
five-revision test needs allocation/storage observations in addition to pointer
identity: distinguish shared standard payloads, copied standard maps/indexes,
local data and temporary compilation storage. Do not infer a 16 GiB fit from the
eight-document publication peak.

The review also found an authored peak-lifetime issue relevant to the self-model
gate. `lower_accepted_source` retained its base mount, construction draft (and any
construction overlay), desired strict snapshot and final strict/derived result
until its final reference audit. On edits the desired snapshot differs from the
published snapshot. `source_references` only needs the draft's reference list and
source map, so a metadata carrier can release the construction graph after strict
conversion/checkpoint capture and before final closure, as the Systems
`PublicationInputs::consume` path already does. Release an intermediate desired
snapshot after publishing it. Commit `9d32eef` implements these releases in the
accepted authored path, preserving reference/source metadata and checked closure
transport. Its regression audits typing and private aliases after the construction
allocation is released. Peak-memory savings remain unmeasured; accepted Systems
integration remains gated. Using one frontend compilation owner rather than
nesting `SourceProject` history under workspace history remains a design proposal.

Publication refinement already drops each prior draft before reconstruction;
its checkpoint retains hashes and shared producer reads, not old graph maps.
The last eight-document audit reported 5,284.5 MiB peak private memory, a
2,594,064-byte compact certificate and 406,184,536 bytes of optional revalidation
accounting. The latter is logical read-storage accounting, not an independent
measurement of unique heap allocations; it omits container overhead and can
count shared read arrays more than once. Trusted restoration omits these optional
reads, and the scheduler excludes immutable dependency subjects. Do not multiply
the publication's read figure by revision count or claim restored standards are
replayed. Measure authored revision certificates separately. Medium/full
publication and accepted workspace scaling remain unverified.

## Concrete fixtures and 100-document plan

[`working-revisions.json`](../verification/fixtures/modeling-workspace-phase1/working-revisions.json)
contains actual mixed source texts, one unresolved endpoint edit, one recovered
provider, removal/repair obligations and a 50-pair source generator. The two
parser-preflight tests exercise the real production frontend only. They establish
input suitability, not Working publication, reference closure or workspace success.

After the gate, open two workspaces with the same accepted `Arc` handles. In the
first, batch-add 50 KerML contract documents and 50 SysML worker documents. Each
worker types an attribute through its corresponding KerML datatype. Retain s1;
edit Worker025 to add a port (s2); add a specialization/redefinition in that same
document (s3); remove Contracts025 (s4 Working); re-add it (s5 repaired). Keep all
five revisions. No other document is reparsed for source identity reconciliation.
Authored semantic reconstruction may still inspect all current documents.

Use four scoped reader threads, each creating its own evaluator, to query s1–s5
for groups 000, 025 and 049 over eight passes. Compare canonical IDs and complete
answer projections, rather than cache identity or diagnostic formatting. Assert:

- accepted publication handles, nested KerML allocation and sampled standard
  record pointers remain identical across workspaces/revisions;
- unchanged document `Arc`s, source revisions and reconciled authored IDs stay
  identical; edited containing-node identity is only asserted where the syntax
  reconciliation contract actually preserves it;
- s1/s2 answers do not change after s3; the redefined inherited member retains
  original identity and no inherited record copy is introduced;
- s4 retains exact sources and failure evidence with no stale reference endpoint;
  s5 repairs that relationship using the newly added provider's identity;
- producer work is scoped to authored subjects and accepted publications are not
  replayed. Existing counters alone cannot prove which subjects ran; add a
  test-only evaluation observer or equivalent scheduler-boundary assertion,
  rather than infer this from total counts or elapsed time.

Record elapsed time, document/syntax bytes, local declared/derived counts,
`subjects_considered/evaluated`, family attempts, dirty reevaluations, overlay
materializations and certificate size. Record peak private memory externally if
available. There is no hard speed threshold. Rebuilding authored semantics is an
explicit phase-1 limitation. Standard graph copying or producer replay violates
the shared accepted-dependency boundary. Copying unchanged syntax arenas is a
memory regression the fixture should report. Document pointer checks guard the
chosen implementation; they do not freeze an allocator or arena layout as a
public semantic contract.
