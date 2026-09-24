# Gen2 modeling service

The service coordinates a transport-neutral repository and an authenticated
accepted Systems publication. It has no runtime SQL, Axum or Gen1 dependency.

Resolve `(ProjectId, RevisionSelector)` once to obtain a `BoundRevision`. Retain
that handle for all queries in a request; subsequent branch moves cannot change
its sources, graph, completeness or evidence. `current_element` and
`declared_element` are graph projections. `effective_members` and `effective_names`
return owned language query evidence, diagnostics and semantic context.
Source position lookup explicitly represents multiple matching elements.

`prepare_changes` returns an inspectable candidate without changing a durable
head. `commit_prepared` persists the complete candidate with CAS and returns only
after durable success. Retain the prepared value for exact idempotent retry after
an unknown acknowledgement. The convenience `apply_document_changes` performs
both steps; retrying it reconstructs a different candidate and is not the lost-ack
recovery protocol. Initial project creation allocates an empty Working R0; authored
R1 and later source-backed edits follow the ordinary commit contract.
`prepare_project` and `commit_project` provide the same stable-request retry path
for initial creation; `create_project` is their convenience composition.

Restoration authenticates source/identity/publication bindings and reconstructs
the workspace. Stored Working status remains Working even when the reconstructed
semantics could pass validation. Stored Validated status additionally requires
matching graph, context and closure identities and the actual checked Phase1V1
transition. This is supported platform acceptance, not whole-language conformance.

Validated candidates may carry a versioned cache of local effective facts. An
unavailable cache exporter leaves the source-backed candidate committable. Restoration
authenticates exact source identities, accepted publications, descriptor/profile/
producer context and closure identities, then repeats producer and effective-query
audits. It checks the persisted validation receipt independently. Missing, corrupt,
stale or unknown caches reconstruct from mandatory durable source instead. A valid
cache cannot substitute for missing source, or promote a stored Working revision.

The service writes `agq-project-semantic-cache/2`: a deterministic ZIP containing
small identity metadata and raw local-frontier bytes, with deflate level 1. The
legacy version 1 JSON decoder remains available. Both formats authenticate the
same workspace cache identities; neither changes the repository manifest or
language archive contract. Metadata is limited to 64 KiB, and compressed and
uncompressed cache payloads to 512 MiB each. Unsupported or oversized caches are
omitted on export or discarded on restore; mandatory source remains durable.

SQLite retains the cache transactionally. The measured legacy self-model database
spent 99.66% of its bytes on the cache payload and only 0.145% outside blobs.
Separate cache files would not address that encoding cost. Actual codec and
restoration measurements are recorded in the Phase 2 performance evidence.

The bounded revision cache is optional and evictable. Each entry binds the exact
revision, accepted publications and semantic context. Unqueryable Working revisions
can bypass it; a cache optimization cannot turn a durable success into failure.
`check_integrity` composes storage verification with reconstruction of all retained
revisions, including detached histories. It bypasses resident revision handles and
reports unusable cache references separately. It can be expensive by design.
Enumeration failures remain findings in that report, preserving earlier storage
errors instead of discarding them. `BoundRevision::load_path` reports actual source,
persisted-cache or immutable-memory reuse separately from validation authority.

The API adapter projects these owned values against the pinned Systems Modeling
API 1.0 shapes. Arbitrary semantic mutation, source/programmatic reconciliation,
automatic merge, views and execution remain separate contracts.
