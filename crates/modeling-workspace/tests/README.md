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

After the readiness gate and implementation, the same command runs every held
test. It requires `AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`, both exact
trusted accepted publications. Missing paths, invalid receipts or unavailable
accepted Systems restoration fail the requested test; they never skip a test,
build standards or substitute synthetic acceptance. A process-wide `OnceLock`
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

`ProjectChange`, `ProjectDocument`, syntax edits, standard publication facade
types, closure/evidence structures and all semantic queries used in the tests
already exist in Gen2. Reuse those contracts; the workspace owns history and
atomic publication of the prepared frontend result.

The verification observer is a proposed implementation hook, not a stable public
product API. It can be replaced with an equivalent test-only callback at the
scheduler boundary. The required assertion is that no accepted standard subject
is scheduled during a local edit. No ordinary caller receives graph mutation.

## Test obligations

| Test | Observable contract |
| --- | --- |
| Mixed documents and edits | Explicit KerML/SysML adds; distinct revisions and parents; borrowed language graph identity; old source/context/diagnostics remain unchanged; original inherited port ID and suppressed redefined part; shared unchanged document/syntax allocations. |
| Recovery and repair | Recovered bytes become a Working head; no stale provider or Complete negative lookup; repair uses the same document identity; parsed unresolved names also remain Working and repair to Validated. |
| Removal/re-addition | Removal succeeds as a Working revision, references lose the removed endpoint, old revision still resolves it, re-added path gets a fresh document and semantic identity. |
| Operational failures and sharing | Stale head, unknown document, duplicate path and invalid UTF-8 edit publish nothing; independent authored projects get different authored IDs while sharing standards. |
| 100-document scaling | Fifty KerML/SysML pairs, port edit, redefinition, provider removal and repair; retain all five revisions; four readers perform eight passes over groups 000/025/049, including explicit Working/unavailable projections; record source/syntax size, closure size, producer counters and elapsed time. |

Pointer checks establish that this implementation shares existing immutable
standard records and unchanged syntax allocations. They do not freeze a future
arena/index representation. There is no speed threshold. Rebuilding authored
semantics remains permitted; copying accepted graphs or rerunning their producers
does not.

The separate `agq-kerml-text --test workspace_edit_inputs` preflight runs now
against the real production frontend. It covers all base/edited/recovered fixture
texts and 100 generated documents. It establishes syntax suitability only.

This suite supplements the held Agentique self-model/rich programmatic-equivalence
language gate. It does not replace those semantic acceptance obligations or claim
that the language readiness gate passed.
