# Held phase-1 integration tests

These are executable Rust integration-test bodies for the additive
`agq-modeling-workspace` boundary selected by ADR 0024. There is deliberately no
crate manifest, production implementation, root workspace membership or Gen1
adapter in this change. The language readiness gate remains required.

The authority for the prospective interface is
[the phase-1 design](../../../docs/modeling-workspace-phase1-design.md) and
[the frontend boundary](../../../docs/modeling-workspace-frontend-boundary.md).
The symbols below make their conceptual interface concrete for implementation;
they are not claims that those methods already exist. Method spelling may be
adapted during implementation without weakening the observable assertions.

## Current blockers and execution

The actual command

```powershell
cargo test --manifest-path crates/modeling-workspace/Cargo.toml --test phase1 --features verification -- --ignored --test-threads=1
```

currently exits 1: `manifest path crates/modeling-workspace/Cargo.toml does not
exist`. Consequently these five integration tests have **not been type-checked
or executed** against a workspace implementation. Rustfmt checks their Rust
syntax; that is not a substitute for compilation.

After the readiness gate and implementation, add `--test working_states` to run
the supplemental recovery/identity and unsupported-capability tests as well.
Those bodies also remain uncompiled and unexecuted while the manifest is absent.
The tests require `AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`, both exact
trusted accepted publications. Missing paths, invalid receipts or unavailable
accepted Systems restoration fail the requested test; they never skip a test,
build standards or substitute synthetic acceptance. Both cache files and the
compiled Systems receipt are checked before loading the large KerML publication.
A process-wide `OnceLock`
shares the restored facade across the tests. Ignored tests are excluded from
ordinary package verification until explicitly requested.

Further concrete prerequisites are:

1. `ProjectWorkspace`, immutable Working/Validated handles and independent
   workspace revision IDs do not exist yet.
2. The proposed frontend `SourceInputs`/`SourceCompilation` boundary does not
   exist. Current `SourceProject::apply` rejects recovered documents, and accepted
   lowering requires a strict snapshot. Wrapping that API unchanged cannot pass
   the recovery, unresolved-reference and removal tests.
3. Trusted Systems restoration still requires a genuinely accepted receipt.
   The existing language acceptance harness remains independently gated.
4. A test-only scheduler observer must retain every subject actually evaluated
   during each authored compilation. Total counters alone cannot prove that
   accepted standard producers were not replayed.
5. A test-only storage observer must distinguish retained publication tables from
   copied dependency entries and sparse projections contributed by local carriers.
   Shared facade/record `Arc`s alone do not establish the no-copy requirement.

## Proposed facade used by the tests

All fallible operations return typed errors with Debug implementations. These
tests do not freeze error formatting or invent a semantic DTO representation.

| Receiver | Proposed surface |
| --- | --- |
| `ProjectWorkspace` | `open(Arc<CanonicalSysmlSystemsLibrary>)`; authenticate its nested accepted KerML publication. |
| Workspace history | `head() -> &Arc<WorkingProjectRevision>`, `revision(ProjectRevisionId) -> Option<&Arc<WorkingProjectRevision>>`. |
| Document commands | `add_kerml(expected_head, path: &str, source: &str)`, `add_sysml(...)`, `apply(expected_head, impl IntoIterator<Item = ProjectChange>)`; each returns a new immutable `Arc<WorkingProjectRevision>`. |
| Revision identity | `revision() -> ProjectRevisionId`, `parent() -> Option<ProjectRevisionId>`, `root() -> ElementId`; workspace IDs remain distinct from kernel/source revision types. |
| Documents | `document_at(&str) -> Option<&ProjectDocument>`, `documents() -> impl Iterator<Item = (&str, &ProjectDocument)>`; reuse current exact frontend source/syntax identities. |
| Revision graph | `strict_snapshot() -> Option<&Snapshot>`, `semantic_model() -> Option<&ModelView>`; an unavailable current graph never falls back to an earlier revision. |
| Revision evidence | `diagnostics()`, `references() -> &[ReferenceAssertion]`, `producer_status() -> Option<&AuthoredProducerStatus>`, `producer_closure() -> Option<&Arc<ProducerClosureCertificate>>`. |
| Query facade | `kerml_queries() -> Result<KerMlQueries<'_>, QueryUnavailable>`, `sysml_queries() -> Result<SysmlQueries<'_>, QueryUnavailable>` borrowing the exact revision graph. |
| Shared dependencies | `accepted_sysml() -> &Arc<CanonicalSysmlSystemsLibrary>`, `accepted_kerml() -> &Arc<CanonicalKermlStandardLibraries>`. |
| Validation | `validate(self: &Arc<WorkingProjectRevision>) -> Result<ValidatedProjectRevision, ValidationFailure>`; `ValidatedProjectRevision::working() -> &Arc<WorkingProjectRevision>`; no public unchecked constructor. |
| Verification feature only | `testing::producer_subjects(&WorkingProjectRevision) -> impl Iterator<Item = ElementId>`; actual observed scheduling events, including reconstruction rounds, not a post-hoc filter of authored records. |
| Storage verification only | `testing::publication_storage(&CanonicalSysmlSystemsLibrary)` returns actual base-table identity tokens for both accepted layers; `testing::dependency_storage(&WorkingProjectRevision)` reports those tokens, typed `copied_dependency_entries` counts and `local_projection_entries`. |

`ProjectChange`, `ProjectDocument`, syntax edits, standard publication facade
types, closure/evidence structures and all semantic queries used in the tests
already exist in Gen2. Reuse those contracts; the workspace owns history and
atomic publication of the prepared frontend result.

These verification observers are proposed implementation hooks, not stable public
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
| 100-document scaling | Fifty KerML/SysML pairs, port edit, redefinition, provider removal and repair; capture each revision's baseline before the next edit, then retain all five; four readers perform eight passes over groups 000/025/049, including explicit Working/unavailable projections; record source/syntax size, closure size, producer counters and elapsed time. |
| Supplemental Working states | Targeted recovery preserves an unaffected same-document declaration through repair; removal/re-addition retires its identity; unresolved references retain exact source origins; an unsupported variation remains Working even when producers converge and the graph has a full closure certificate. |

Every proposed Validated revision must carry a certificate fully closed over its
actual semantic graph, in addition to Complete producer status and references.
This is necessary but not sufficient: independent unsupported semantic
capabilities still prevent validation. Pointer assertions cover payload/syntax
sharing; the separate storage observer tests that base maps, indexes and proof
tables were not copied. There is no speed threshold. Rebuilding authored
semantics remains permitted; copying accepted graphs or rerunning their producers
does not. These are prepared assertions, not passed workspace checks.

The separate `agq-kerml-text --test workspace_edit_inputs` preflight runs now
against the real production frontend. It covers all base/edited/recovered fixture
texts and 100 generated documents. It establishes syntax suitability only.

This suite supplements the held Agentique self-model/rich programmatic-equivalence
language gate. It does not replace those semantic acceptance obligations or claim
that the language readiness gate passed.
