# In-memory modeling workspace phase 1

Status: design prepared; production crate integration remains gated by
[ADR 0026](adr/0026-language-foundation-stability-contract.md). The tests below
are an implementation acceptance plan, not claims of runtime test success.
[ADR 0024](adr/0024-gen2-modeling-workspace.md) selects the additive
`agq-modeling-workspace` boundary; the
[generation audit](gen1-gen2-architecture-audit.md) explains the reuse choices.

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
scope evidence and concrete 100-document fixture. It reuses the prepared accepted
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

The current mixed `SourceProject::apply` rejects recovered production syntax;
`sysml::lower_source` requires `draft.strict_snapshot()`. Reuse its parsing,
document identity and reconciliation machinery, while adding an explicit
construction result boundary in the frontend for this phase. Working revisions
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

## Acceptance tests to implement after the gate

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
| Programmatic equivalence | Build equivalent rich subsystem declarations with kernel/language builders and via authored source. | Compare semantic projections and relationships modulo authored-ID mapping, preserving standard IDs and evidence/completeness obligations. Equal text-generated IDs alone is not the oracle. |
| Scaling/read isolation | Create 100 small mixed documents, retain several revisions, and query old/new heads on parallel threads with independent evaluators. | Deterministic revision-specific answers, shared standard records, no accepted-library producer replay. Record elapsed time, retained inputs and local producer scope; no hard speed threshold. |

The permanent self-model semantic tests remain in the language acceptance layer;
workspace editing tests consume it as a fixture. Implementation traceability and
Cargo dependency checks remain separate, so language semantics never read Rust
manifests to establish model truth.

## Initial cost model and remaining incrementality

SourceProject currently rebuilds candidate dependencies across its complete
document input. Phase 1 may rebuild authored construction/overlays where necessary;
do not claim a fully incremental compiler. Unchanged syntax/source values and
accepted standards must remain shareable. Tiny edits must not copy whole standard
graphs or rerun their producers. Observe local producer evaluation counts and
standard record sharing in the scaling test, not timing alone.

The [storage review](modeling-workspace-frontend-boundary.md#shared-dependency-storage-review)
finds shared record/proof payloads but full standard map/index replication in the
current snapshot mount and revision builders. Phase I8 remains unverified; Arc
pointer checks do not prove its no-copy requirement. Plan an additive shared-base
view with local storage before claiming that requirement, while leaving authored
index rebuilding as an explicit initial limitation. The review also identifies
construction-draft lifetime reductions for the accepted authored fixture; it does
not claim measured savings or authorize production workspace integration early.

Future work may refine document dependency invalidation, persistent authored
indexes, incremental derivation and revision retention policies. It must preserve
the immutable revision and evidence contracts. SQLite persistence, network APIs,
diagram rendering, transformations, execution IR, simulation, assistant features
and Gen1 migration are explicitly outside phase 1.
