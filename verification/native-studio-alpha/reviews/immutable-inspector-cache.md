# Completed Inspector cache in the immutable revision reader

The reader now retains at most 16 successful Inspector DTOs, with least-recently-used eviction, inside its already authorized immutable `BoundRevision`. Keys are canonical ElementIds within that reader only. There is no shared global cache, persisted cache or retained semantic evaluator. Errors are never stored. Successful incomplete query summaries, diagnostics, profile and evidence counts remain unchanged.

Every insertion checks the outer revision, nested element revision and requested element ID. Computation and returned DTO cloning occur outside the cache mutex. Concurrent duplicate completions retain one exact value; conflicting values for the same immutable input fail instead of overwriting it. Callers receive independent owned copies. Cache poisoning can be bypassed for an ordinary fresh read; it cannot mint authority. Fixed entry count bounds retained inspectors, not their total byte size.

Read authorization still precedes reader creation. Native epoch/revision/request/selection fences and candidate/ghost routing are untouched. Scope reset drops the reader; an executing read may keep its own old Arc until its fenced response completes. A hit still waits behind any read already running on the single read lane: this removes repeat computation, not all queue wait.

When `AGENTIQUE_VIEW_PROFILE=1`, the distinct `agentique-studio-inspector-cache/1` record reports `revision_reader_inspect`, hit/miss, outcome and reader-call wall time. A miss includes the ordinary modeling-view computation; a hit includes lookup and DTO cloning. Neither includes queue wait, and neither is a query-phase/GPU/input-latency measurement. Existing modeling-view records remain unchanged. No clocks or records are emitted by this cache when profiling is disabled.

Seven component regressions cover incomplete DTO fidelity and independent clones; repeated errors; wrong element/outer/nested revision; LRU eviction; separate reader bindings; concurrent duplicate completion outside the mutex; and conflicting concurrent completion. The existing ignored real platform gate also compares an ordinary real Inspector with the reader's first and repeated reads, including serialized equality and caller-copy independence. That gate remains an actual accepted-runtime test and was not executed here.

Source checks actually run:

```text
rustfmt --edition 2024 crates/studio-platform/src/reader.rs crates/studio-platform/src/reader/inspector_cache.rs crates/studio-platform/tests/real_model.rs
exit 0; no output

git diff --check
exit 0; no output
```

No build or runtime consumer ran in this worktree. Integration should run focused `agq-studio-platform` reader tests, ordinary native stale-read tests, Clippy, and the already ignored `native_in_process_self_model_candidate_commit_and_restore` gate only when the standards-consumer slot is free. No performance or real-model acceptance is claimed before those runs. This commit is independent of the held mount/audit/self-model work and the separately proposed call-local query reuse.
