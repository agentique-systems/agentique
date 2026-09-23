# Query/cache timing gap: source review only

The current completed-frontier metrics report
`planning_including_query_cache_micros`. That includes producer control flow,
query evaluation, cache work and plan/evidence accumulation, excluding the
separately timed dependency-index update. It is not a distinct query timer.
The existing `negative_queries_certified` counter counts successful closure
checks, not their duration. No evaluator/cache timing already exists to aggregate.
No runtime code, receipt, effect registry or corpus execution changed in this review.

`KerMlQueries` owns seven context-local memo tables. Their access points are:

| Cache | Concrete access point |
| --- | --- |
| Namespace population | `namespaces.rs::membership_population`, lookup near line 351 and insert near 729 |
| Effective names | `naming.rs::effective_names`, lookup and insert near lines 25/33 |
| Relationship source roles | `relationship_sources.rs::source_roles`, lookup and insert near lines 19/36 |
| Library specializations | `implicit.rs::library_specializations`, lookup near 121, clear/insert near 130 |
| Result parameters | `ordering.rs::result_parameters`, lookup near 57, clear/insert near 66 |
| Fact origins | `queries.rs::fact_origin`, lookup near 348 and insert near 372 |
| Declared fact origins | `queries.rs::declared_fact_origin`, entry/get-or-insert near 386 |

The smallest complete **cache-operation** observation would add private,
opt-in timing/hit/miss accounting at those accesses. Time lock acquisition,
lookup plus returned-value clone, insertion and bounded eviction; exclude the
miss's semantic computation between lookup and insert. Retain exactly the
current lock lifetimes and return-value/evidence behavior. Declared-origin
lookup currently fills under its lock: report that small fill as cache work
rather than silently moving the lock or changing evaluation order.

Capture each evaluator's counters at the end of its batch in
`producer_worklist.rs`, beside the existing `q.negative_queries_certified()`
accumulation. Count each batch once, including ReferenceFullScan. The SysML
extension uses the same borrowed KerML evaluator. A fresh evaluator/fork resets
its memo tables; counters belong to the batch/round, not the immutable context,
graph, certificate, checkpoint semantic state or receipt identity. Disabled
observation should avoid clock reads in normal authored queries.

This small insertion establishes distinct cache-access time. It does **not**
establish total query time: query implementations are distributed methods with
recursive composition and no single dispatch boundary. Timing `result()` would
measure result initialization, and timing its returned value's lifetime would
also include producer/graph work. Neither is a valid shortcut.

For a bounded next profile, add query-entry guards to the measured tail's
`implied_redefinitions`, `owning_type`, `effective_features`,
`all_redefined_features`, `featuring_types`, `reference_referent` and
`reference_binding_context` methods. Label the result **instrumented tail-query
time**, since uninstrumented queries and direct canonical reads remain outside
it. These cover the principal query calls in VariableFeaturing,
FeatureReferenceExpression and PositionalRedefinition. Existing family timers
still include graph/evidence construction around the queries.

Nested query calls must not be summed as independent wall time. An opt-in
per-thread batch observation scope can charge elapsed intervals to the current
category (planner/query/cache), restoring the caller's category on guard drop.
Alternatively retain an inclusive outermost query timer and clearly report
cache time as a subset. Per-evaluator atomic depth alone is unsuitable if a
future batch shares that evaluator among parallel readers: unrelated concurrent
queries are not nested. Do not let timing state enter query evidence or require
another semantic effect registry.

To claim total query time, audit and instrument all actual query-entry boundaries
reachable from the scheduler, including composed callbacks; a handful of cache
hooks cannot prove that coverage. A deterministic nested-scope accounting test
and a focused unchanged-result/evidence comparison should accompany any runtime
insertion. Existing logs cannot retroactively provide that missing measurement.
