# Real Native Studio journey, third execution

The real window passed all 39 assertions, including current-world navigation
during construction, requirement and graph exploration restoration, and a durable
nested PartUsage commit. Source `42e7e187de823a3b855d530dcee84dbe8c06b59f` was
built from a clean tracked checkout. The process exited 0 after 540.194 seconds.
This is a fresh repository bootstrap with two actual source imports. It is **not
a warm-open measurement**. Separate-process restart remains a separate gate.

All 16 screenshots, state snapshots, the full journey report, process memory
samples and raw profiling log are retained. `artifact-manifest.json` identifies
their original bytes and the compiled executable/build receipt.

| Observed boundary | Previous run02 | This run03 |
| --- | ---: | ---: |
| First asserted Validated view, fresh bootstrap | 420.593 s | 384.667 s |
| Candidate preparation, pending mutation interval | 171.512 s | 129.738 s |
| Current Inspector selection to reply during preparation | 315 ms | 345 ms |
| Same Inspector observed request to reply | 254 ms | 284 ms |
| Largest input-hook gap during preparation | 14 ms | 48 ms |
| Input hooks during preparation | 28,283 | 21,399 |
| Validate scripted action | 2.592 s | 2.713 s |
| Commit scripted action | 860 ms | 818 ms |
| Actual durable repository commit | 132.463 ms | 92.764 ms |
| Complete service commit | 133.980 ms | 94.407 ms |
| Process peak working set | 6,769,532,928 bytes | 6,464,499,712 bytes |

These are sequential observations on the same host, not a randomized benchmark.
The candidate is still far too slow, and this journey alone does not prove
incremental/cold exact equivalence. Scripted actions include 30 lead and 18 settle
frames. Validation reused existing semantic work; its durable candidate preparation
took 1.592 seconds, including 1.349 seconds frontier export and 222 ms encoding.

History now follows parent lineage and summarizes commit intent. The new part is
visible when selected. Requirements identify reference and constraint kinds.
The initial durable Diff remains poor: fitting its widely separated changes
produces an unreadable overview. That screenshot is retained for the next fix.

The final rolling 240-frame window measured 6.053 ms median / 6.739 ms p95, not a
whole-journey distribution. GPU timing observed 16 zero-duration samples and no
map, polling, non-monotonic or surface-invalidation errors. Zero-duration samples
are retained as valid timer quantization. This does not establish the cause of
the earlier executable's aggregate timestamp errors or qualify device recovery.
