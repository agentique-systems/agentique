# Working source revisions: frontend boundary

Status: additive frontend and workspace implementation integrated at reviewed
source `5bdbc60`; workspace runtime acceptance remains pending. The
[language foundation](../verification/summaries/final-language-acceptance/semantic-closure-readiness.md)
passed and ADR 0026 is adopted. The sections below retain their historical design
and source baselines; the current implementation is summarized first.

## Current implementation contract

`SourceInputs` owns immutable authored documents and the accepted Systems v3
dependency with nested KerML v9. `SourceCompilation` owns the exact current
strict or construction frontier, references, diagnostics, checked declared
identity history and semantic query contexts. It preserves recovered inputs
without substituting a prior graph. The existing strict `SourceProject` API keeps
its separate contract; workspace head/history belongs to `ProjectWorkspace`.

Compilation now retains `SourceEffectiveAudit`, containing the exact
`SysmlSemanticContextId`, sorted local canonical subjects including derived
records, and the applicable operation report. It invokes the strict finalizer's
effective-query dispatcher in bounded batches on that revision. Accepted
dependency subjects are excluded. Canonical diagnostic identities and available
source origins remain inspectable; an absent direct origin for a derived subject
is not fabricated. The audit neither issues a publication nor replays standards.

Workspace validation requires the existing syntax, strict construction,
reference, producer-certificate and context gates plus a matching finding-free
effective audit. Missing or mismatched context and Invalid/Incomplete effective
results remain blocking. The malformed attribute-to-PartDefinition fixture
requires closed producers and complete references, rejection by the actual typed
query/audit, and a repaired new revision that leaves the old rejection unchanged.

Kernel `DeclaredConstructionHistory` distinguishes temporary omission from
explicit deletion. Immutable base tables share accepted graph, index, proof and
search storage with local deltas and project-local inverse projections. Kernel
invariants and exact trusted restoration of the issued cache have passed on this
representation; actual workspace revision sharing, dogfooding, Working/Validated
and scale tests remain pending. The
[command ledger](../verification/summaries/final-audit-semantic-closure/commands.json)
records prerequisites separately from those runtime results.

The current suite has 12 prepared acceptance tests. In addition to the original
five-revision recovery sequence, which has a 99-document Working fourth revision,
a separate fixture requires five Validated revisions with 100 mixed documents
each and four parallel readers. Neither fixture is reported as passed here.

## Historical reuse audit and design requirements

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
    pub fn semantic_model(&self) -> Option<&ModelView>;
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

`strict_snapshot` and `construction` expose the declared frontier. `semantic_model`
exposes the current construction or strict overlay when present, otherwise its
current declared model; it returns `None` when no safe current graph exists.
Both query factories must borrow that same model, and any returned producer
certificate must be attached to its exact context. This is the graph-identity
contract asserted by the held workspace tests.

The workspace's checked `ValidatedProjectRevision` conversion cannot forward
`ProjectRevision::is_complete_slice()`: that existing strict-source convenience
check does not establish all workspace acceptance obligations. Apply the existing
phase-1 criteria to the exact Working handle: a strict snapshot with zero
construction obligations, authenticated dependencies, converged and Complete
producer status with full certificate coverage, mandatory reference agreement,
and no blocking source or capability diagnostics. The held
unsupported-Variation case remains a Working-state discriminator even with full
producer closure; it adds no foundation conformance gate.

`SourceDiagnostic` may unify syntax, construction, reference and producer
diagnostic envelopes while retaining their native evidence/source identities;
it must not become a second semantic graph. Existing source-origin and query
evidence structures remain authoritative.

## Implementation handoff after readiness

Read-only inspection at `9152fb9`; this section adds no implemented API or test
result. The following extraction can be owned independently of the kernel's
shared-table refactor, after agreeing the identity-history seam below.

| Slice / concrete files | Minimum change and existing code to reuse |
| --- | --- |
| Immutable inputs: `project.rs`, additive `source.rs`, `lib.rs` | Move the existing `ProjectDocument::{parse,edit}` and `SourceProject::apply` document-map preparation into `SourceInputs`. Retain the project/root, limits, explicit dialect/profile, accepted dependency and `Arc<ProjectDocument>` map. `Edit` uses production syntax reconciliation; `Replace` currently reparses without reconciliation, `RenameDocument` retains the document, and remove/add allocates a new DocumentId. Keep those distinct contracts. The workspace owns expected-head checks and history; these functions publish neither. |
| Typed construction outcomes: `library/{mod,construction,sysml_construction}.rs` | Separate source-supportedness failures from dependency/invariant failures at their emission sites. Today `LibraryLoadError::Interpretation(String)` mixes unsupported productions/string literals with duplicate canonical locators, illegal/derived property writes and context errors. Introduce a typed internal unsupported-source case carrying production/source origin; adapt the strict API back to its existing error contract. Do not classify by message text or catch all `Interpretation`/kernel errors as language diagnostics. `check_supported_sysml` is a useful first pass, but does not cover every later construction failure. |
| Retained compilation: `sysml/source.rs`, `sysml.rs`, `library/{mod,refinement}.rs` | Factor `lower_accepted_source` before `draft.strict_snapshot()?` into the proposed `SourceCompilation`. Reuse `construct_on`, reference refinement, `LibraryDraft`'s declared/semantic candidate and both existing scheduler entry points. Assemble `source_references` against the final current construction context before strict promotion, preserving mandatory missing endpoints and their current origins. Keep producer incompleteness/convergence separate from syntax, reference and capability diagnostics. |
| Exact query context: `sysml/source.rs` | Store one pending-specialization/pending-namespace carrier with the compilation, and thread it through candidate queries, reconstruction closures, checkpoint capture/rebind and final query factories. Reuse `AcceptedSourceDependency` and the mounted dependency's construction factories. `project_overlay_context` currently accepts no pending scopes: keep `Construction`/`ConstructionOverlay` while omitted input leaves any pending scope, even when kernel obligations are zero. Promote only when those scopes are empty; do not lose them by taking the strict branch. |
| Strict adapter: `project.rs`, `lowering.rs` | Let existing strict `SourceProject` consume the shared preparation/compilation path and require a strict outcome before changing its head. Preserve its existing `ProjectRevision::snapshot`, kernel revision identity and unconditional query guarantees. The new workspace consumes the additive carrier directly, without nesting another history. |

Use `SourceEditError` for invalid paths, missing documents, invalid edit spans and
parse resource limits; use `SourceBuildError` for invalid accepted identity,
registry/context mismatch and internal construction/proof invariants. Recovered
syntax, explicitly unsupported input and unresolved/ambiguous/wrong-kind
references belong to successful Working results. A scheduler that safely returns
a partial frontier or exhausts its iteration budget supplies Working status;
an internal scheduler error is not automatically such a result. Any kernel
rejection classified as source-caused must have an explicit typed mapping and a
source-origin test. Preserve unmapped failures as operational errors.

Whole-document omission is the first recovery policy. Keep all document bytes,
syntax and diagnostics, lower only complete supported documents, and mark the
existing project root namespace pending whenever any input was omitted. Carry
explicit specialization obligations as well. A root created over the remaining
documents is current construction, not a substitute for omitted declarations.
Explicit document removal instead rebuilds the genuinely smaller input; unresolved
consumers retain their failed assertions and ordinary construction obligations.
Never turn a permanently removed document into an indefinite pending provider.

The missing kernel capability is **declared construction identity history**, not
another semantic model. `Snapshot::preview` retains reservations only in its
unpublished `ConstructionView`; that type currently has no advance/promotion API.
`lowering::publish` advances strict snapshots only, while `construct_on` starts
from the dependency mount each time. Neither preserves retirement through an
unavailable or omitted Working input by itself. Agree these operations with the
kernel owner before implementing the frontend carrier (names remain proposed):

```rust,ignore
// Opaque kernel-owned local reservations; dependency reservations stay borrowed.
DeclaredConstructionHistory::from_snapshot(&Snapshot) -> Self;
history.retire(&DeclaredIdentitySet) -> Result<Self, ModelError>;
history.reconcile(ConstructionView)
    -> Result<(Self, ConstructionView), ModelError>;
ConstructionView::revalidate_declared(self) -> Result<Snapshot, ModelError>;
```

`retire` must work without a current safe graph, so an explicit deletion while
already Working still advances history. `reconcile` binds the exact registry,
dependency and prior history; it accepts a reserved, temporarily unmaterialized
identity only as continuation of that same declaration/record kind, rejects
retired or protected identity reuse, and reserves newly admitted local element
and occurrence IDs. Declared revalidation reruns strict kernel validation and
carries this same history rather than rebuilding from an empty mount. Reuse the
existing `ConstructionOverlay::revalidate` only after its exact declared-input
and reservation check succeeds; closure reuse still requires checkpoint rebind.
All operations return
immutable results; a failed input/build operation commits none of them. The
kernel implementation may combine these operations, but cannot expose unchecked
reservation import or a generic retired-ID resurrection switch.

The frontend's private `SourceIdentityLedger` maps retained syntax/owned-role
identities to these declarations and occurrences; it contains no record values
or semantic answers. Reconcile it after **each** accepted input edit, including
Working-to-Working transitions, rather than against the last strict compilation.
Omitting a document from lowering does not retire a declaration whose syntax
identity survived. Actual syntax deletion/replacement or document removal does;
re-adding bytes follows the parser's fresh identity contract. Keep derived-output
identity governed by the existing derivation machinery, not this authored ledger.
The kernel owns checked reservations; the frontend owns the evidence that an
input identity continued or was removed. Both must pass recovery/delete/re-add
controls before workspace identity continuity is claimed.

Implement and verify these slices using the existing
`workspace_working_inputs.rs` parser controls and held `working_states.rs`
observations, plus focused frontend tests for typed error classification and
pending-scope preservation at zero kernel obligations. Require a construction
history test that deletes and re-adds a declaration while no strict graph exists.
The storage observer and scheduler observer are separate verification hooks;
neither supplies this identity history or establishes language acceptance.

## Working-state acceptance discriminators

The held [`working_states.rs`](../crates/modeling-workspace/tests/working_states.rs)
target supplements the main phase-1 suite. It has no production crate or Cargo
membership and has not been compiled or run against a workspace. Its inputs have
an independent real-parser preflight in
[`workspace_working_inputs.rs`](../crates/kerml-text/tests/workspace_working_inputs.rs).
Parser success establishes source suitability and syntax reconciliation only.

These cases distinguish input publication, query completeness and validation:

| Case | Required Working result | Required discriminating observation |
| --- | --- | --- |
| A disjoint declaration survives temporary recovery | A targeted edit makes a later package in the same document malformed. Record a new Working head with exact source and syntax; the initial safe-lowering policy omits that document's semantic contribution. | The earlier complete PartDefinition retains its syntax-node identity across malformed and repaired inputs. Repair retains its semantic ID because that reconciled declaration was never explicitly deleted. During recovery, the omitted declaration cannot appear as a stale current answer and its absence cannot produce a Complete negative. |
| The same document is explicitly removed and re-added | Remove the repaired document, then add identical bytes at the same path. | The fresh DocumentId, syntax-node ID and authored ElementId differ. The prior retained validated handle still reports its original source, context and answers. Recovery omission and deletion cannot share an indiscriminate retirement path. |
| A mandatory type target becomes unresolved | Replace only `Storage::Repository` with `Storage::MissingRepository`; syntax remains Parsed and the current input becomes Working. | Keep the failed reference assertion even if the required relationship cannot be materialized. Its origin names the current DocumentId, SourceRevisionId and source range; available reference/query contexts belong to the current frontier. No answer retains the old provider as its type, and effective type queries cannot claim Complete from a missing provider population. |
| The provider is removed while its consumer is already Working | Starting from that unresolved input, remove the provider document, restore the original reference name while the provider remains absent, then re-add the provider. | Every intermediate Working head keeps the failed assertion and current origin/context; neither raw typing endpoints nor type answers retain the retired provider. Re-addition allocates a fresh provider/document identity and resolves the unchanged consumer only to that provider. Every retained earlier revision keeps its own source, context and answer. |
| Parsed variation remains unsupported | Add `variation part selected` under a PartDefinition, preserving its canonical flag and queryable authored value. | The real scheduler converges and the certificate covers the entire graph, while `effective_usages` remains Incomplete with `PendingSysmlRule::Variation`. Validation rejects with a capability diagnostic. Removing the modifier can validate; the old Working answer remains Incomplete. |

The last case is an explicit Operational v2 capability boundary already exercised
by `producer_closure_does_not_erase_unsupported_variation_semantics`. It is not a
new requirement to implement variation in this milestone. A future profile that
implements that capability must deliberately replace this negative fixture with
another documented unsupported case or its positive acceptance test. Do not
silently make the assertion disappear because producer completion now succeeds.

The real parser preflight additionally deletes and reinserts the disjoint
declaration while its document remains recovered, then repairs the other package.
Unchanged nodes can survive temporary recovery, but a deleted node must not regain
its old syntax identity when identical bytes return. This preflight tests the
existing `production::Document::edit` reconciliation contract only; it does not
establish semantic identity retirement or successful Working publication.

Source inspection at `0e3d7f5` confirms why the semantic sequence remains held:
`SourceProject::apply` clones the current document map, reconciles edits, and rejects
incomplete production syntax before lowering. `lower_accepted_source` requires
`draft.strict_snapshot()?` before assembling the final reference assertions. A
rejected batch updates neither the head nor history. Successful Working input
publication must therefore be implemented at the proposed additive boundary; it
cannot be inferred from the current strict path's atomic error behavior. Retirement
must follow the latest successful Working inputs across consecutive failures of
language validation, rather than restarting from the last Validated snapshot.

The private Validated constructor must evaluate the supported platform contract
in addition to `is_fully_closed(model)`, zero kernel obligations and mandatory
reference results. `ProjectRevision::is_complete_slice()` currently reports the
existing strict frontend's narrower usability checks; it is not a substitute for
this validation boundary. A capability diagnostic must retain its subject/source
and pending semantic evidence without inventing a second semantic model.

For held tests, adapt prospective method spelling during implementation without
weakening these observations. Run `--test working_states --features verification
-- --ignored --test-threads=1` only after the language readiness gate, the additive
Working carrier and the actual workspace crate exist. Accepted publications are
then restored through the existing shared test support; there is no fallback to
building standards or substituting `SourceProject` for the missing workspace.

## Shared dependency storage review

This section preserves the pre-integration observations at their exact baseline.
The current shared-base implementation and pending workspace sharing gate are
described above; the recorded copies below are not assertions about `5bdbc60`.

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
