# Modeling platform Phase 2 roadmap

Status: prepared; activation follows accepted ProjectWorkspace Phase 1. This
document specifies the next boundary and adds no repository, transport or storage
implementation.

The canonical kernel graph and language query contracts remain authoritative.
`agq-modeling-workspace` owns immutable authored project history and a Working
head over shared accepted standards. Phase 2 should make that history available
through repository and application contracts without giving those layers their
own language semantics.

## Repository and branch ownership

Add a Gen2 repository abstraction above `ProjectWorkspace`, separate from Gen1
`agq-workspace` and `agq-application`. Its operations load an exact project revision,
enumerate history, and conditionally move a named head from an expected revision
to an already constructed candidate. Use explicit repository/project/branch
identities. `ProjectRevisionId` remains authored-history identity; a branch name,
storage row key or kernel `RevisionId` does not replace it.

Start with one-parent commits. Branch creation points another head to an immutable
revision; it does not copy standards or semantic records. Merge semantics need a
separate reviewed source reconciliation contract. Concurrent head updates must
reject stale expected revisions and return the current revision identity.

Persistence is an adapter to these contracts. It stores exact sources, identity
history, accepted dependency identities and enough authenticated data to restore
the canonical revision. Private archive encoding and cache layout may evolve;
restoration must preserve semantic identity, provenance, relationship order,
negative/search evidence and certificate compatibility. A restored Working
revision does not become Validated merely because storage succeeded.

The repository transaction writes candidate revision data and the head change
atomically. Memory changes and acknowledgement follow its durable commit. Failed
writes, conflicts and restarts cannot expose a half-committed head or modify any
previous revision.

## Model query and application service

Add an application-facing service above the repository/workspace contracts.
Every request identifies a project and revision, or resolves a named head once
at request start. Multi-query requests use that same immutable revision so an
intervening edit cannot mix answers from different inputs.

Expose existing KerML/SysML borrowed queries, diagnostics, source mapping and
explicit platform validation results through application result types. Preserve
Complete, Incomplete and Invalid and distinguish current-graph from effective
producer-closed queries. A response DTO projects the graph; it never becomes a
mutable semantic store. Validation is the versioned platform contract, not full
standards conformance or executable-model acceptance.

Document edits carry the expected head and exact source operation. The service
constructs a candidate with the real frontend before asking the repository to
commit. Query services and view services consume language contracts rather than
parsing names or recreating inherited records themselves.

## Systems Modeling API adapter

Implement the Systems Modeling API as an adapter above those application
contracts. Pin its own specification/version and map supported operations to
explicit workspace/repository behaviors. Transport identifiers map to canonical
ElementIds and project revisions without redefining identity. Pagination and
continuations retain their revision binding.

Unsupported mutations and unavailable semantics return explicit results. An API
request cannot create a graph record that bypasses the authored editing and
validation contract. If a future API requires canonical programmatic edits, that
becomes a separately reviewed command surface with provenance and reconciliation
rules; it is not an escape hatch in the source-edit API.

## Phase 2 acceptance

- Revision reads and source/query projections remain identical before and after
  repository restoration; accepted standard identities and storage stay shared.
- Competing updates to the same expected head admit one commit; the losing
  candidate remains inspectable without becoming head.
- Two branches and retained readers see independent immutable histories.
- A failed transaction or interrupted acknowledgement cannot mutate a committed
  revision or leave an acknowledged head without durable data.
- Transport roundtrips preserve revision and ElementId bindings, completeness,
  provenance and diagnostics; unsupported operations fail explicitly.
- Dependency checks keep Gen1 product crates, database drivers and HTTP transport
  outside the Gen2 language engine and in-memory workspace.

Standard publication, initial authored build and ordinary authored edit remain
separate performance classes. Future query/provider-footprint invalidation may
reduce edit cost, with full authored recomputation as its semantic oracle. This
roadmap does not require ADR 0027 adoption or broaden language conformance.
