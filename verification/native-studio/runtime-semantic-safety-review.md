# In-process semantic safety review

The review covers `agq-studio-platform`, with read-only feedback to the native
shell integrator. It does not claim real-model acceptance without runtime assets.

## Platform findings addressed

1. Initial candidate commit code treated atomic CAS refusal like unknown commit
   acknowledgement. A conflict proves no commit happened; it now returns the
   candidate to Validated so the operator can cancel/reprepare. Unknown storage
   outcomes keep the exact candidate and operation for retry.
2. Candidate eviction originally preceded preparation. Invalid proposals could
   unnecessarily evict a completed review handle. Admission now plans removal
   and removes only a completed handle after successful candidate projection.
   Eight active candidates and 32 retained handles remain the external bounds.
3. Candidate Source now uses the prepared candidate's exact source provenance,
   revision and added element identities. It never substitutes parent source.

## Tests

Lifecycle tests inject repository `OutcomeUnknown`, `Conflict`, and `Storage`
failures through the same private state transition methods used by production
candidate calls. They check that Read + Propose agents cannot call validation or
commit callbacks or retrieve cached commit responses; failed validation retains
Working state; display status cannot replace actual validated revision evidence;
unresolved commit blocks cancellation/revalidation; successful retry preserves
operation identity; and further replay does not invoke persistence again.
Capacity testing ensures unfinished previews and unresolved commits cannot be
evicted to admit additional candidates.

Bootstrap tests reject missing/invalid bundles without creating a model database,
distinguish package existence from authentication, and preserve the existing
first-run guard against adopting an intentionally empty authored revision.
Compile assertions prove the platform and projected data can cross the background
worker boundary.

The real-self-model integration gate remains explicit and ignored by default
until an existing accepted runtime is supplied. It includes candidate source
identity, parent immutability, competing commit conflict, durable commit and
restore. Pure lifecycle tests do not certify language semantics or durable
SQLite transactions; those remain covered by existing repository contracts and
the gated real-model vertical.

## Feedback sent to native shell integrator

- Apply bounded reading before allocating a saved session's full contents.
- Clear navigation on project changes, or bind navigation entries to ProjectId.
- Guard selection's revision against the current scene revision.
- Fence pending worker replies by their view/selection context; report actual
  runtime bootstrap phases and handle failed reopen explicitly.
- Use one selected-object revision context for Inspector, Explain and Source,
  including removed ghosts and newly added candidate elements.
- Query live semantic dependency neighborhoods through the platform instead of
  presenting a scan of currently projected edges as a complete dependency view.
- Reproject candidates when changing worlds rather than reusing the old lens.

These shell items are integration feedback, not a claim they have all been
implemented at the time this review record was written.
