# Current immutable revision reads during candidate construction

Status: held implementation for independent review and integrated checks. No real
concurrency acceptance or new timing claim is established by this document.

The serial modeling worker currently queues Inspector, Explain and Source behind
candidate preparation. A responding camera therefore does not establish that the
current revision remains usefully explorable during the observed long preparation.

## Capability boundary

`studio-platform/src/reader.rs` adds `StudioRevisionReader`. Only
`StudioPlatform::revision_reader` can mint it, after `Authority::Read` and ordinary
service resolution. The handle stores one `BoundRevision`; it exposes binding,
inspect, explain and source only. It retains no service, repository, mutable policy,
candidate operations or raw-checkpoint constructor. No fixture can mint it.

The immutable snapshot shares its existing storage. Its source surface reads the
retained document, not the current filesystem. Existing accepted publication and
durable validation contracts are unchanged. The capability does not invent policy
revocation; the native host independently controls whether its results may be shown.

## Ordering and bounded work

After a real Projection has successfully replaced its scene and revision binding,
the UI queues a reader-pin operation on the serial worker before requesting
inspection or processing another native action. Existing busy-state command gates
keep candidate preparation behind that pin. Candidate reads, candidate lifecycle,
historical ghosts and all mutations retain the original serial path.

The separate reader worker has at most three waiting requests, one latest request
for each panel, plus one executing request. A mutex protects only this mailbox and
the opaque reader reference; it is released before any semantic query executes.
Replacing, cancelling, resetting or failing queued requests emits a terminal
response for each removed ID. A query failure or caught query panic also returns a
terminal failure. An enqueue failure cannot leave an unminted scope waiting for a
pin that will never arrive.

## Response fences

All worker replies now carry the runtime-open epoch. The bridge advances that epoch
and discards read capabilities before attempting a runtime-open enqueue, including
when enqueueing or later authentication fails. Current read replies additionally
require the exact project/revision binding, selected element, current panel request
ID, matching result identity, and an open Explain/Source surface when applicable.
Candidate lifecycle changes alone do not invalidate a read of the still-current
immutable baseline. Selecting a candidate object cannot use the current reader.

Cancelled queued work is retired immediately by terminal response. An executing
read finishes on its own immutable handle and its stale completion is discarded.
There is no unsafe cancellation of a semantic query and no shared platform mutex.

## Required qualification

Added focused tests cover read authorization before resolution, denied/failed pins,
worker Send/Sync, bounded latest-panel replacement and every terminal cancellation,
runtime/selection/revision/project/request/result fences, dismissed source panels,
and current-read response application while a serial mutation remains pending.
These are tests of routing and capability boundaries, not fabricated real-model
evidence. They have not been run at this handoff: the root's real native process has
the active standards/memory slot. Formatting and diff checks are recorded separately.

Before accepting this product behavior, run the integrated tests/Clippy, then
ordinary native input must select another real baseline element during actual
candidate preparation and receive its matching Inspector before that preparation
completes. Record exact request/revision/element, observed latency and candidate
pending state. Read-query cost itself remains measurable work; this change removes
its queue dependency on candidate reconstruction and does not claim 60 Hz queries.
