# Modeling platform Phase 2

Base: fetched `origin/main` at `bc0ffc0e957e2ea605fa8717720faeb8dae83569`,
with a clean initial worktree. Branch:
`platform/modeling-platform-phase2-repository-service-api`.

Implementation acceptance remains pending the integrated cache, fresh-process
self-model, durable scale, HTTP, and final workspace gates. Command completion
alone does not establish acceptance. Failed attempts remain in the evidence.

The platform adds immutable durable project history above the accepted Phase1V1
workspace. Prepared source edits become durable revisions through an atomic
branch-head comparison and swap. A retained prepared request supports exact retry
after a lost acknowledgement. Revision-bound service handles preserve the same
graph, source identities, completeness and evidence throughout a request.

| Boundary | Implementation |
| --- | --- |
| Versioned manifests, source identities, receipts and repository contract | `agq-modeling-repository` |
| Transactions, checksums, source deduplication, CAS and recovery | `agq-modeling-sqlite` |
| Source-backed commands, immutable queries, source lookup, diff and integrity | `agq-modeling-service` |
| Pinned Systems Modeling API 1.0 projections and inventory | `agq-modeling-api` |
| Separate `/api/gen2` Axum router with bounded jobs and revision-bound pages | `agq-modeling-http` |

KerML Operational v9, Systems Operational v3, ADR 0024 and ADR 0026 remain the
accepted foundation. Standard graph bytes are external authenticated inputs.
Repository truth is exact durable source plus identity checkpoints and publication
bindings. Persisted authored graph caches are disposable and independently
checked against the source, context and closure receipt. Stored Working revisions
cannot acquire Validated authority from a cache or a database flag.

The first incremental path reuses unchanged parsing and immutable local lowering
fragments, and uses existing producer dependency invalidation conservatively.
All seven edit classes passed exact full authored reconstruction equivalence.
Measured lowering visits fell from six to one in the three-document fixture;
kernel reconstruction and effective audit remained broad, and wall time did not
improve. The full oracle starts with identical parsed source identities; its zero
parse count must not be presented as a cold source reconstruction measurement.

The standards inventory contains 35 operations: six supported, seven partial and
22 unsupported. Partial element projections do not fabricate missing semantic
properties. Programmatic POST commit, merge and deletion lifecycles remain
explicitly unsupported. This is not a claim of full Systems Modeling API
conformance. The standalone router is not mounted in the browser-visible Gen1
server, so the milestone's conditional browser/e2e gate does not apply.

Evidence is kept in the adjacent curated JSON files. `commands.json` records
actual build/check commands, exits and output hashes; `runtime.json` records the
monitored accepted-cache processes. Raw output remains under ignored
`verification/generated/`. The frontend, durability and API summaries distinguish
their own tested scopes from integrated platform acceptance.

Accepted-cache gates are named ignored tests in
`crates/modeling-service/tests/durable_platform.rs` and
`adapters/modeling-http/tests/vertical.rs`. Set `AGENTIQUE_KERML_CACHE` and
`AGENTIQUE_SYSTEMS_CACHE` to the existing accepted cache files. Run gates by exact
name, serially; the internal cold-restore helper requires its parent's explicit
database and expectation environment. No acquisition or publication rebuild is
part of verification. Release measurements disable LTO, as recorded by the build
command; cold process measurements do not imply an OS filesystem-cache flush.

Architecture and deferred source reconciliation, merge, views and GC policy are
documented in ADR 0028 and `docs/modeling-platform-phase3-roadmap.md`. Gen1 release
obligations remain independent.
