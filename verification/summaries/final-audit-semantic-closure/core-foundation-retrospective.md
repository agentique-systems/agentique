# Core-foundation architectural retrospective

Reviewed production source: `0fcc0ff`, after the held workspace and exact accepted
v3 dependency integration. This is a bounded code and dependency review of the
original architectural principles, not another standard-rule audit. The
[language-readiness decision](../final-language-acceptance/semantic-closure-readiness.md)
remains established. Workspace validation and runtime acceptance on this new
storage implementation are still pending at this reviewed source.

No new architectural inversion or identity/storage defect was found. One known
validation coverage gap remains actionable: the integrated frontend checks
`effective_usages` alone before workspace validation. The assigned correction
must retain the full applicable local effective audit and its exact semantic
context. This review does not represent that pending correction as implemented.

## Principles and observed boundaries

| Principle | Production-code assessment |
| --- | --- |
| Parser is separate from canonical model | `SourceInputs` owns immutable source documents and production syntax. `SourceCompilation` separately owns declared construction/strict frontiers, derived semantics, references and diagnostics. `ProjectRevision` delegates to this exact compilation; syntax trees and platform DTOs do not become canonical records. |
| Generic language-agnostic kernel | `SharedMap`, identity reservations, `DeclaredConstructionHistory`, snapshots, associations and provenance contain generic kernel identities. They contain no SysML metaclass logic. The new workspace depends inward; the measured dependency check below includes its normal and test edges. |
| Relationships are first-class | Canonical relationship records and association occurrence identities remain intact. Association navigation and inverse/reference indexes project existing carriers. Reprojecting a touched inherited association group retains its occurrence identities and semantic positions; it does not create another writable relationship. |
| Declared is distinct from derived | Declared-history reconciliation rejects local derived records/occurrences. `strict_snapshot()` exposes declared inputs; `semantic_model()` includes the derived overlay. The scheduler and cache retain producer identities, selected proofs and searches separately from declarations. Accepted dependencies retain their original declared/derived provenance. |
| Immutable revisions | `ProjectWorkspace::apply` checks the exact expected head, prepares inputs and compiles before changing head/history. Published revision fields are private and accessed through immutable `Arc` handles. Validation retains the exact Working handle; failure does not rewrite the head or a prior revision. Operational edit/build errors publish nothing. |
| Source is distinct from semantic identity | Project, workspace revision, kernel revision, document, syntax node and element IDs remain separate. Reconciled edits may retain syntax identity; replacement does not guess reconciliation, path rename retains the document, and removal/re-add allocates a new document. Declared history checks kind and source-node continuity, preserves temporary omissions, and permanently reserves explicitly retired IDs. |
| Provenance and explainability | Source maps remain revision-specific. Kernel derived evidence uses canonical dependencies, rule identities and shared proof/search pools. Allocation addresses occur only as internal sharing/deduplication observations, never as persisted semantic identity or publication authority. |
| Effective inheritance without copies | Workspace query factories use the existing KerML/SysML effective APIs on the same model. They return canonical inherited Feature/Membership identities. The workspace introduces no inherited-record materializer or flattened language model. Runtime inherited-identity/redefinition tests remain required on this integration. |
| Negative conclusions require closure | Construction queries receive the actual pending source roots and current draft; there is no fallback to an earlier complete model. Initial, mounted and rebound certificates are checked against exact contexts. Producer convergence alone is insufficient: `validate` also checks full certificate closure, reference completeness and context compatibility. The remaining effective-audit gap is described below. |
| Standard publications are immutable and shared | Accepted standard tables, records, associations, indexes, proof/search pools and identity reservations remain behind immutable base allocations. Kernel write guards reject direct dependency changes; ownership guards reject indirect composite changes. Local references to standard elements can contribute project-local inverse/navigation projections without changing the publication's own view. |
| SysML composes KerML | Both query factories borrow the compilation's shared canonical graph. The SysML context authenticates the accepted Systems dependency and its retained accepted KerML publication. The v3 syntax/profile selection comes from that publication, and workspace code does not implement a second type system or standard loader. |
| Execution remains separate IR | Working/Validated is a versioned structural platform acceptance boundary. No action evaluator, simulation engine, behavioral execution result or full language-conformance claim is introduced. The self-model remains a model of conceptual architecture with separately checked implementation mappings. |

The reviewed boundaries are implemented in
[SharedMap](../../../crates/kernel/src/shared_map.rs),
[kernel model and indexes](../../../crates/kernel/src/model.rs),
[association projection](../../../crates/kernel/src/association.rs),
[declared history](../../../crates/kernel/src/declared_history.rs),
[source inputs and compilation](../../../crates/kerml-text/src/source_inputs.rs),
[accepted source context](../../../crates/kerml-text/src/sysml/source.rs) and
[ProjectWorkspace](../../../crates/modeling-workspace/src/lib.rs).

## Sharing and history review

`SharedMap` is storage machinery, not an authorization boundary. Its local map
can shadow a base entry, so correctness still depends on the kernel's existing
dependency-write, ownership, derived-fact and archive validation. Those guards
remain on the observed mutation/restore paths. Mutable lookup, removal and retain
operate on local rows; they do not copy a base entry as a side effect of reading
it. Cloning shares the local `Arc` until a local mutation requires copy-on-write.
Ordered iteration merges local and inherited keys deterministically.

Model indexes retain an immutable base and rebuild local indexes. A local
association can extend a group involving a standard element; that touched group's
navigation/reference positions are recomputed and its old projected entries are
suppressed. Such sparse project-local projections are legitimate observations,
not copied canonical standard records. The
[storage observer](../../../crates/kernel/src/storage_observer.rs) distinguishes
them from duplicated authoritative standard entries and checks actual backing
tables, beyond facade `Arc` equality. The accepted-cache test must still confirm
that this representation change preserves the already-issued semantic identity.

`DeclaredConstructionHistory` keeps a shared anchor and local identity metadata.
The frontend supplies explicit deletion from the live document/syntax ledger;
an omitted document with retained syntax is not silently considered deleted.
Reconciliation requires the same accepted dependency, compatible exact registry,
same record kind and continuing source identity. Reservation history travels
through construction states and strict promotion. The workspace's exclusive
mutable head is distinct from its immutable retained history.

These choices avoid whole-standard table copying, but they do not establish
constant-time edits, fully incremental semantic closure or bounded revision
retention. Local copy-on-write, local identity ledgers and read-only traversal of
accepted populations still have costs. Changing these implementation details is
permitted by ADR 0026 only while semantic identity, ordering and evidence remain
unchanged. Persistence and history-retention policy remain later platform work.

## Required validation correction and remaining evidence

At `0fcc0ff`, `SourceInputs::compile` loops over local Definition/Usage records
and asks only `effective_usages`. `WorkingProjectRevision::validate` rejects
diagnostics, construction, recovered syntax, incomplete producer closure,
incomplete references and incompatible contexts, but it cannot infer the success
of unqueried typed projections, interface ends or return/parameter collections.
This repeats the coverage gap exposed by the strict final audit and must be
closed before claiming the stronger workspace Validated contract.

The assigned correction is to reuse the strict applicable effective audit for
all local canonical subjects, including derived local records, store the report
and exact `SysmlSemanticContextId` on the immutable compilation, and require that
matching report to be finding-free during validation. Missing or mismatched
context must not validate. Accepted dependency subjects remain excluded from this
authored audit; no Systems publication or binding issuance runs during edits.
The malformed attribute/part typing fixture and its repaired revision must show
that actual invalid effective semantics remain blocking even if producers close.

The following remain unclaimed by this review:

- Integrated kernel history, immutable-dependency, touched-association projection
  and archive-identity tests, followed by restoration of the exact accepted cache.
- The complete effective-audit correction and its adversarial Working/Validated
  fixtures on the integrated source.
- Real workspace self-model validated revisions, old-revision immutability and
  physical standard-sharing observations.
- One hundred mixed documents, five validated revisions and four parallel
  readers. The original fixture's removed-provider Working revision is useful
  recovery coverage but does not establish five validated revisions.
- Required final formatting, Clippy, workspace tests, Rustdoc, frontend,
  standards, grammar/metamodel and dependency gates on the final integration.

The [held workspace review](workspace-integration-review.md) and earlier storage
audits remain historical evidence. Their prior map-copy observations are not
silently rewritten; the new source and actual sharing tests must establish the
current result. No general conformance expansion or renewed KerML publication is
required by this retrospective.

## Verification actually performed

`python verification/scripts/generation_boundary.py` exited **0** on `0fcc0ff`.
It performs offline Cargo metadata inspection without compilation. Actual output:

```json
{
  "format": "agentique-generation-boundary/1",
  "workspace_packages": 20,
  "protected_packages": [
    "agq-kerml",
    "agq-kerml-semantics",
    "agq-kerml-syntax",
    "agq-kerml-text",
    "agq-kernel",
    "agq-modeling-workspace",
    "agq-standard-libraries",
    "agq-sysml",
    "agq-sysml-semantics"
  ],
  "checked_dependency_kinds": ["normal", "build", "dev", "optional", "target"],
  "scope": "all declared workspace edges; third-party packages are leaves",
  "violations": []
}
```

No Rust build, cache restoration, semantic query evaluation, standard producer
run or workspace acceptance test was executed by this review.

Documentation checks also passed:

| Exact command | Exit | Output |
| --- | --- | --- |
| `git diff --check` | 0 | Empty |
| `python -c "import pathlib,re; p=pathlib.Path('verification/summaries/final-audit-semantic-closure/core-foundation-retrospective.md'); links=re.findall(r'\]\(([^)]+)\)',p.read_text()); assert all((p.parent/link.split('#')[0]).is_file() for link in links); print(str(len(links))+' local links valid')"` | 0 | `10 local links valid` |
