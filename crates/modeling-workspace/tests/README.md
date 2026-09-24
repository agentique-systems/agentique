# Phase-1 integration test contract

These integration tests exercise the additive `agq-modeling-workspace` boundary
selected by ADR 0024. Its production implementation, manifest and root workspace
membership are integrated. The language foundation passed and ADR 0026 is
adopted; workspace runtime acceptance remains pending at reviewed source
`5bdbc60`. There is no Gen1 adapter.

The documented contract for the implemented interface is
[the phase-1 design](../../../docs/modeling-workspace-phase1-design.md) and
[the frontend boundary](../../../docs/modeling-workspace-frontend-boundary.md).
The symbols below describe the implementation and its observable assertions.

## Pending runtime acceptance and execution

The explicit accepted-publication command is

```powershell
cargo test --locked --offline -p agq-modeling-workspace --features verification -- --ignored --test-threads=1
```

The suite contains 12 prepared acceptance tests: six in `phase1`, one in
`self_model` and five in `working_states`. It covers mixed documents, recovery,
invalid effective typing, scale and self-model dogfooding. No passing workspace
runtime result is claimed here; actual results belong in the
[current command ledger](../../../verification/summaries/final-audit-semantic-closure/commands.json),
separately from compilation and passing storage/cache prerequisite checks.
The tests require `AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`, both exact
trusted accepted publications. Missing paths, invalid receipts or unavailable
accepted Systems restoration fail the requested test; they never skip a test,
build standards or substitute synthetic acceptance. Both cache files and the
compiled Systems receipt are checked before loading the large KerML publication.
A process-wide `OnceLock`
shares the restored facade across the tests. Ignored tests are excluded from
ordinary package verification until explicitly requested.

`SourceInputs`/`SourceCompilation` retain current recovered bytes and their
construction frontier independently from the existing strict `SourceProject`
API. Kernel-owned identity history distinguishes temporary omission from
deletion, including deletion while the current revision remains Working.
Verification-only observers record actual scheduler evaluations and retained
kernel table ownership. Trusted Systems restoration still requires an accepted
receipt. The separate language self-model/effective-core harness has passed;
it does not establish these workspace tests.

## Facade used by the tests

All fallible operations return typed errors with Debug implementations. These
tests do not freeze error formatting or invent a semantic DTO representation.

| Receiver | Implemented surface |
| --- | --- |
| `ProjectWorkspace` | `open(Arc<CanonicalSysmlSystemsLibrary>)`; authenticate its nested accepted KerML publication. |
| Workspace history | `head() -> &Arc<WorkingProjectRevision>`, `revision(ProjectRevisionId) -> Option<&Arc<WorkingProjectRevision>>`. |
| Document commands | `add_kerml(expected_head, path: &str, source: &str)`, `add_sysml(...)`, `apply(expected_head, impl IntoIterator<Item = ProjectChange>)`; each returns a new immutable `Arc<WorkingProjectRevision>`. |
| Revision identity | `revision() -> ProjectRevisionId`, `parent() -> Option<ProjectRevisionId>`, `root() -> ElementId`; workspace IDs remain distinct from kernel/source revision types. |
| Documents | `document_at(&str) -> Option<&ProjectDocument>`, `documents() -> impl Iterator<Item = (&str, &ProjectDocument)>`; reuse current exact frontend source/syntax identities. |
| Revision graph | `strict_snapshot() -> Option<&Snapshot>`, `semantic_model() -> Option<&ModelView>`; an unavailable current graph never falls back to an earlier revision. |
| Revision evidence | `diagnostics()`, `references() -> &[ReferenceAssertion]`, `producer_status() -> Option<&AuthoredProducerStatus>`, `producer_closure() -> Option<&Arc<ProducerClosureCertificate>>`. |
| Effective audit | `effective_audit() -> Option<&SourceEffectiveAudit>` retains the exact `SysmlSemanticContextId`, sorted local canonical subjects including derived records, applicable operation counts and all findings. Accepted dependency subjects are excluded. |
| Query facade | `kerml_queries() -> Result<KerMlQueries<'_>, QueryUnavailable>`, `sysml_queries() -> Result<SysmlQueries<'_>, QueryUnavailable>` borrowing the exact revision graph. |
| Shared dependencies | `accepted_sysml() -> &Arc<CanonicalSysmlSystemsLibrary>`, `accepted_kerml() -> &Arc<CanonicalKermlStandardLibraries>`. |
| Validation | `validate(self: &Arc<WorkingProjectRevision>) -> Result<ValidatedProjectRevision, ValidationFailure>`; `ValidatedProjectRevision::working() -> &Arc<WorkingProjectRevision>`; no public unchecked constructor. |
| Verification feature only | `testing::producer_subjects(&WorkingProjectRevision) -> impl Iterator<Item = ElementId>`; actual observed scheduling events, including reconstruction rounds, not a post-hoc filter of authored records. |
| Storage verification only | `testing::publication_storage(&CanonicalSysmlSystemsLibrary)` returns actual base-table identity tokens for both accepted layers; `testing::dependency_storage(&WorkingProjectRevision)` reports those tokens, typed `copied_dependency_entries` counts and `local_projection_entries`. |

`ProjectChange`, `ProjectDocument`, syntax edits, standard publication facade
types, closure/evidence structures and all semantic queries used in the tests
already exist in Gen2. Reuse those contracts; the workspace owns history and
atomic publication of the prepared frontend result.

These verification observers are implementation hooks, not stable public
product APIs. Equivalent internal callbacks or observations are acceptable. The
scheduling assertion is that no accepted standard subject is evaluated during a
local edit. No ordinary caller receives graph mutation.

Storage tokens must identify the retained containers, not just the publication
facade or record payloads. The typed entry counts cover records, occurrences,
navigation and index tables, reserved/retired identities, explanation maps,
proof adjacency, search interning and optional reference contributions. Assert
zero copied dependency entries in an empty mount, every retained revision
(including Working), and two independent workspaces. Local proofs that happen
to equal dependency proofs are permitted. Sparse navigation/inverse projections
with a standard subject key are also permitted when supported by local carriers;
report them separately instead of classifying them as copies by key alone. The
observer must inspect actual storage ownership, not infer sharing from the
accepted `Arc` fields. This is an implementation acceptance check whose
representation may change with the kernel.

## Test obligations

| Test | Observable contract |
| --- | --- |
| Mixed documents and edits | Explicit KerML/SysML adds; distinct revisions and parents; borrowed language graph identity; old source/context/diagnostics/reference evidence remain unchanged; original inherited port ID and suppressed redefined part; stable syntax/semantic identity for unaffected declarations inside the edited document, and shared unchanged document/syntax allocations. |
| Recovery and repair | Recovered bytes become a Working head; no stale provider or Complete negative lookup; repair uses the same document identity; parsed unresolved names also remain Working and repair to Validated. |
| Removal/re-addition | Removal succeeds as a Working revision, references lose the removed endpoint, old revision still resolves it, re-added path gets a fresh document and semantic identity. |
| Operational failures and sharing | Stale head, unknown document, duplicate path and invalid UTF-8 edit publish nothing; independent authored projects get different authored IDs while sharing standards. |
| 100-document recovery sequence | `hundred_documents_five_revisions_and_parallel_borrowed_reads`: fifty KerML/SysML pairs, port edit, redefinition, provider removal and repair; r4 has 99 documents and is Working, while r1/r2/r3/r5 have 100 documents and validate. Four readers perform eight passes over the five retained revisions, including Working/unavailable projections. This is not the five-validated-revision gate. |
| Five validated 100-document revisions | `hundred_documents_five_validated_revisions_and_four_parallel_readers`: every retained revision has 50 KerML and 50 SysML documents and a Validated handle. Port/redefinition edits in groups 025 and 049 preserve earlier baselines; four barrier-synchronized readers compare values, canonical IDs, contexts and bounded evidence across all five revisions in two passes. |
| Supplemental Working states | Targeted recovery preserves an unaffected same-document declaration through repair; removal/re-addition retires its identity; unresolved references retain exact source origins; an unsupported variation remains Working even when producers converge and the graph has a full closure certificate. |
| Invalid effective typing and repair | A parsed attribute typed by a PartDefinition retains its canonical typing, Complete references and closed producer certificate, but the effective attribute-definition query is Invalid and the retained audit blocks validation. Changing the definition to an AttributeDefinition must validate a new context while leaving the earlier revision and rejection unchanged. |

Every Validated revision must carry a certificate fully closed over its
actual semantic graph, in addition to Complete producer status and references.
It also requires a finding-free applicable effective audit matching that exact
SysML context. The audit reuses the strict publication query dispatcher without
issuing a standard publication. Missing, mismatched, Invalid or Incomplete audit
evidence and independent unsupported capabilities prevent validation.
Pointer assertions cover payload/syntax
sharing; the separate storage observer tests that base maps, indexes and proof
tables were not copied. There is no speed threshold. Rebuilding authored
semantics remains permitted; copying accepted graphs or rerunning their producers
does not. Prepared assertions are not passed workspace acceptance checks.

The separate `agq-kerml-text --test workspace_edit_inputs` preflight runs
against the real production frontend. It covers all base/edited/recovered fixture
texts and 100 generated documents. It establishes syntax suitability only.

This suite supplements the passed Agentique self-model/rich programmatic-equivalence
[language gate](../../../verification/summaries/final-language-acceptance/semantic-closure-readiness.md).
Workspace completion requires execution of this suite against the accepted caches.
