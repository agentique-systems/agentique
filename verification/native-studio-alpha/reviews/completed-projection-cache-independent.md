# Independent source review: held completed projection cache

Reviewed exact commit `fc96d64d293dcb9f4bdb7e246f3e367fbf6198e6`, based on
`a64fc00`. No source changes, builds, tests, or runtime consumers were performed
by this reviewer. The patch is held outside the baseline native journey.

No blocking authority, revision, or candidate-boundary defect was found.
Approval is conditional on focused execution and the real platform gate. This
review does not establish cache performance or native product acceptance.

## Ranked remaining qualifications

1. **P2 — Entry count does not bound bytes.** Sixteen full presentation DTOs may
   still be large, and each returned owned clone temporarily adds another copy.
   The patch retains no canonical models or evaluators, but a real memory
   measurement is required before describing this as a memory improvement.
2. **P2 — New cache mechanics and real-gate changes remain unexecuted.** The
   nine prepared unit cases cover the meaningful failure boundaries. Run them
   and the existing real platform gate after the baseline native journey. The
   successful modeling-view query-reuse oracle does not execute this new cache.
3. **P2 — Hit latency and queue behavior remain unmeasured.** A hit still resolves
   the durable binding and clones/checks the full DTO. Binding can reconstruct a
   service-evicted revision. The cache also cannot bypass a main-worker mutation
   or an already running query. Its opt-in enclosing-call timing is appropriately
   separate from query phases, input latency, frame time and worker queue wait.

## Boundaries traced

- `studio-platform/src/lib.rs:115–120,135–148` invokes the existing
  `Authority::Read` and `ModelingService::resolve` path before any lookup, then
  compares the resolved manifest project/revision with the requested binding.
  `modeling-service/src/lib.rs:178–199` loads and verifies the current durable
  manifest, enforces the accepted publication binding, and only reuses a bound
  semantic revision when the complete manifest matches. Resolution refusal is
  returned unchanged before an old presentation can be used.
- The private cache key is exact `RevisionBinding` plus the entire
  `ViewDefinition`, including its name/version and ordered filters. Its validity
  relies on the established repository contract that revision manifests are
  immutable (`modeling-repository/src/lib.rs:454–458`). It does not introduce an
  assumption that mutable branch heads or runtime transports are revisions.
  The native bridge replaces the platform on Open, so a new runtime/platform
  instance does not share these entries.
- `projection_cache.rs:65–88,103–130` verifies the full returned definition,
  outer revision, and every node/edge revision before insertion and again on a
  hit. Only successful DTOs enter the cache. Incomplete semantics remain the
  exact successful DTO's completeness/warnings, not fabricated complete results.
  No errors are negative-cached. Full owned clones isolate caller mutation.
- The mutex covers at most 16 key comparisons, LRU changes and Arc bookkeeping.
  Ordinary resolution, semantic computation, nested identity scans, full DTO
  equality, returned cloning and evicted destruction occur outside it. Concurrent
  misses can compute twice; identical completions share the retained entry, and
  a disagreeing completion returns `PlatformError::Invalid` without replacing
  the first result. A poisoned cache bypasses retention, while resolution and
  output checks still apply. No retry loop or cache lock encloses semantic work.
- Entries own `Arc<ViewProjection>` only: copied presentation names, IDs,
  relationship metadata, groups, features and warnings. They retain no
  `BoundRevision`, semantic model, source checkpoint, query context, agent
  authority, or prepared candidate.
- `StudioPlatform::compare` still resolves both revisions ordinarily and calls
  modeling-view directly for each projection. Candidate construction/review
  likewise retains its original direct query path. The exact dependency-view
  factory goes through ordinary `project`; neither its traversal definition nor
  candidate phase/approval semantics are changed.

## Tests do not reduce fresh reconstruction to cache equality

The new real-gate first/repeat/caller-copy assertions establish exact DTO and
serialization preservation for repeated calls. They are additional checks.
After commit, `comparison.before` is freshly projected through the unchanged
uncached comparison path and compared with the earlier graph. The test then
drops the old platform before ordinary restart, making the next cache empty.

The subsequent source-only reconstruction still removes only the test's
discardable semantic artifact, constructs a new service, requires the actual
`DurableSource` load path and reaches `Validated`. Its new platform initially has
no entries. The added semantic fingerprint comparison around the second edit
checks the retained canonical predecessor directly; a cached presentation cannot
make a mutated predecessor pass. Thus the existing durable-source, restart,
candidate and commit obligations are not replaced by cache-hit comparisons.

The prepared inert-cache tests are explicitly presentation mechanics, not
authenticated semantic fixtures. They cover every definition field, both binding
coordinates, per-instance isolation, denied Read, failed/foreign resolution,
outer/nested revision mismatches, computation failure, LRU eviction, caller-copy
isolation, concurrent completion agreement/disagreement, and poisoned retention.
