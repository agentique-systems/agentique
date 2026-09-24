# ADR 0028: generation-2 modeling repository and revision service

Status: adopted architecture for Phase 2; implementation acceptance is recorded
separately in `verification/summaries/modeling-platform-phase2/`.

## Decision

The repository consumes `agq-modeling-workspace`. The application service consumes
both; protocol and SQLite adapters depend inward on these contracts. Neither the
kernel nor language crates know repository, SQL or HTTP concepts. Gen1 remains
operational with independent release obligations. ADR 0024 Phase1V1, ADR 0026,
KerML Operational v9 and Systems Operational v3 remain unchanged.

RepositoryId, ProjectId, BranchId, ProjectRevisionId, DocumentId,
SourceRevisionId, kernel RevisionId and ElementId are distinct identities.
Names and paths are mutable labels. A revision has at most one same-project
parent. Branches reference existing immutable revisions; creation copies no
semantic graph. Deleting a branch retains all revisions. Merges and garbage
collection are deferred.

Candidates are constructed in an isolated workspace fork. Repository commit
writes all source blobs, the identity checkpoint, manifest and receipt, registers
the revision and conditionally changes its branch head in one durable transaction.
Only transaction success permits the service to acknowledge a new head. A stale
expected head returns its actual value and never silently retries. An operation
identity binds the exact request and enables retry after durable commit but lost
acknowledgement. Reusing an operation identity for different content is an error.

## Durable representation

Version 1 is a deliberate manifest, exact UTF-8 source bytes addressed by SHA-256,
and a separately versioned frontend identity checkpoint. The checkpoint preserves
source/syntax/semantic identity reconciliation and retired identity reservations;
it is reconstruction input, not a second semantic model. Source blobs deduplicate
across revisions. Paths, languages, DocumentIds and SourceRevisionIds are retained
per revision. Exact accepted publication digests reference an authenticated
catalogue; standard graphs are never serialized into project revisions.

The manifest distinguishes Working from Validated(Phase1V1). Restoring Validated
requires reconstructing the exact source identity binding, matching the semantic
receipt and rerunning the checked workspace validation transition. A database
flag alone never creates a Validated handle. Integrity hashes detect corruption;
they do not turn untrusted rows into accepted language authority.

An optional semantic cache must bind source/identity inputs, accepted publications,
language/descriptor/rule context, closure certificate and cache format. Missing,
corrupt or incompatible caches are discardable and rebuild from durable source.
The implementation provides an authenticated local-frontier decoder and
conservatively rebuilds for unknown formats. Cached graphs exclude accepted
dependency records. Cache support is verified separately from source restoration.
The in-memory revision cache retains immutable handles keyed by exact revision,
publication and semantic-context identity; eviction cannot remove durable data.

## Service and protocol

A request resolves its explicit revision or branch once and retains that immutable
handle. Owned query projections preserve canonical identity, metaclass, source,
completeness and evidence. Current graph and effective queries are distinct.
Diffs compare canonical identities and distinguish declared facts from derived
consequences. Edits are source-backed and always pass through the workspace.

Systems Modeling API 1.0 mapping uses the pinned PDF/OpenAPI/schema. Its inventory
is independent of Gen1 coverage. Unsupported programmatic commit and merge
semantics remain explicit. HTTP is a separate Gen2 adapter; pagination binds the
resolved revision and query, including on later pages after branch movement.

## Incrementality and acceptance

Unchanged immutable frontend inputs may be reused. Semantic reuse requires the
existing positive/negative/provider read and producer-effect contracts; inability
to prove an affected frontier falls back to full authored reconstruction. Exact
full-rebuild semantic equivalence is the gate. ADR 0027 sealing is not required.

Fault tests cover writes, CAS, transaction commit and lost acknowledgement. Reopen
must find only complete referenced revisions. Durable self-model and mixed
100-document history tests follow focused fixtures. Actual outcomes, commands,
exit codes and separate performance classes follow ADR 0021. This decision does
not itself establish any implementation acceptance or whole-language conformance.
