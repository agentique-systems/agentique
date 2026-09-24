# Gen2 modeling repository

This crate owns transport-neutral durable history contracts above
`agq-modeling-workspace`. It has no SQLite, HTTP or Gen1 dependency.

`RevisionManifest` v1 references exact source blobs, a versioned frontend identity
checkpoint and authenticated accepted-publication identities. `CandidateRevision`
validates storage completeness and checksums; it cannot assert semantic acceptance.
Validated manifests carry the actual Phase1V1 source/graph/context/closure receipt.
The modeling service rebuilds and checks that receipt before returning a validated
handle. Source bytes and canonical identities are distinct from path labels.

`ModelingRepository::commit_revision` atomically registers a complete candidate
and moves one branch from `expected_head`. A mismatch returns expected and actual
heads. An `OperationId` binds one exact request: replay after a lost acknowledgement
returns its committed receipt even if the branch has since advanced. Changing the
request while reusing that identity is an error. `OutcomeUnknown` means callers
must retry the original prepared request, not reconstruct a new candidate.

Branch creation adds a head at an existing revision; deletion preserves all
revisions and blobs. `list_revisions` includes detached history. Merging and GC
are outside this format's contract. Optional semantic caches never replace the
mandatory source and identity blobs.

See [ADR 0028](../../docs/adr/0028-gen2-modeling-repository-and-revision-service.md)
and the separate [SQLite adapter](../../adapters/modeling-sqlite/README.md).
