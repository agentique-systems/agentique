# Independent review: immutable current-revision reads

Reviewed exact commit `c302b4ae8a24b09d6df1e0b47d7603c33b0bebf4` on
`work/alpha-current-revision-reader`, based on `4e19f47`. Reviewer: runtime /
semantic authority stream. Scope: read-only source, diff and test inspection;
this review changed no implementation, authority record or runtime asset.

**Source review passes conditionally.** No source-level authority bypass,
cross-revision publication, unbounded pending-read queue or identified terminal
request leak remains in this concrete patch. Compilation, integrated tests and
real concurrent-read acceptance have not run at this review boundary. A responsive
canvas alone will not establish that the current model remains explorable.

## Authority and canonical identity

`StudioRevisionReader` has one private `BoundRevision` field. The only production
constructor is `StudioPlatform::revision_reader`, which checks `Authority::Read`
before ordinary `StudioPlatform::bound` / `ModelingService::resolve`. Independently
traced resolution: the manifest must match project, revision and accepted
publication binding before an authenticated cached revision can be reused;
ordinary reconstruction/restoration and validation checks remain unchanged.

The reader exposes binding, inspect, explain and source only. It retains no
service, repository, mutation/candidate method or mutable policy. It cannot resolve
a different identity or mint another capability. Source is read from the retained
revision's authored document, not a mutable workspace file. The native fixture
tests do not construct an accepted reader or grant semantic validation.

This is a retained capability, not a new revocation protocol. The existing policy
has no dynamic revocation contract. The host separately invalidates presentation
access on a runtime change; an already executing read may finish on its original
immutable handle, but its stale result cannot populate the current panels.

## Ordering, isolation and cancellation

After a real projection and scene swap succeeds, the app queues the reader pin
through the ordinary serial worker. A matching ready or pending pin is reused.
Candidate operations and historical/candidate object reads retain their original
serial authority path. The separate read worker holds no platform or service
mutex; its short mailbox lock is released before a semantic query executes.

The mailbox holds at most one waiting request for each of Inspector, Explain and
Source, plus one executing read. Replacement, selection cancellation, scope reset
and pin failure produce terminal replies for every removed waiting ID. Query
errors and caught query panics also produce terminal failures. An executing
semantic query is not forcibly interrupted or silently claimed cancelled.

The initially identified failure paths were addressed before this commit:

- A refused serial pin enqueue resets the unminted read scope, so later panel
  requests fail immediately instead of waiting for a capability that cannot arrive.
- Fixture switching explicitly clears the reader and its scope.
- Failed runtime-open enqueue still advances the epoch and invalidates the old
  capability. Later work cannot silently use the previous runtime.

All terminal IDs are removed from app pending state before stale-result filtering.
Stale responses therefore cannot remain counted as outstanding work merely because
the selected context has changed.

## Result fences and focused coverage

Current panel responses require the exact runtime-open epoch, project/revision,
selected canonical element, latest panel request ID and matching payload identity.
Explain and Source additionally require that their panels remain open. Reader-pin
completion checks the original request, runtime, displayed real binding and
projection revision. Superseded reader outputs cannot replace or fail a newer pin.

Candidate lifecycle changes alone do not invalidate an answer about the still
visible Current revision. Selecting a candidate object in Candidate or Diff does:
the canonical selected revision/candidate tuple must match a Current read exactly.
The implementation includes both the positive Current-with-pending-mutation test
and the requested negative Candidate/Diff tests. It also adds stale pin tests for
revision and runtime changes without fabricating an accepted reader.

Other focused tests cover Read-before-resolution, failed resolution, Send/Sync,
bounded panel replacement, cancellation terminal IDs, failed queueing, wrong
runtime/project/revision/selection/request, mismatched source payload, dismissed
source panel and actual serial pin failure with no opened service. These are
meaningful routing/authority tests; they are not real-model concurrency evidence.

## Required next gate

Inspected the committed [final format record](../checks/immutable-read-lane-format-final.json)
and [final diff record](../checks/immutable-read-lane-diff-final.json): both exited 0. No
compiler, test process or standards consumer ran as part of this review.

Require integrated compilation, focused tests and Clippy. Then use ordinary
native input on the authenticated Agentique revision: start real candidate
preparation, select another Current element, and observe its correctly bound
Inspector **before** preparation completes. Also qualify Source/Explain and
selection cancellation as appropriate. Retain request, revision, element,
candidate-pending state, actual timestamps and whole-process memory observations.

This patch removes a queue dependency; it does not prove that an individual
semantic query is fast, that running queries are preemptible, or that simultaneous
read/reconstruction peak memory fits every machine. Those remain measured product
questions. The revised real screenshots must separately pass the retained visual
critique; no fixture routing test substitutes for that review.
