# Optimized implementation: real retained-repository profile

The ordinary authenticated cache path passed against the same SQLite copy and
exact revision as `../read-profile-01`. This is not a dramatic restoration win.
The executable was built by Windows CI run 36231357374 at
`a6e1f41d2b549a8be10a79dfa3ec786c090b65b3`, using the checked-in release profile
with thin LTO. Its SHA-256 is recorded in `measurement.json` and the retained
helper build receipt. The checkout commit reported by the launcher is not the
binary's compilation commit.

| Boundary | Earlier retained-context build | Optimized build |
| --- | ---: | ---: |
| Runtime KerML authentication/load | 37.242 s | 36.444 s |
| Runtime Systems authentication/load | 38.716 s | 29.278 s |
| Revision restoration, inclusive | 116.734 s | 114.935 s |
| Warm project open, inclusive | 193.691 s | 181.258 s |
| First System projection | 64.1 ms | 66.3 ms |
| Focused System | 323.3 ms | 376.7 ms |
| Focused Graph | 56.0 ms | 60.2 ms |
| Requirements | 56.6 ms | 59.7 ms |
| Inspector first read | 252.0 ms | 233.2 ms |
| Inspector repeat | 0.22 ms | 0.33 ms |
| Repeated projections | 0.93-1.00 ms | 0.87-1.33 ms |

The cache still rebuilds 1,101 records, lowers 18 documents, reparses six,
evaluates 332 producer subjects and audits all 791 local subjects. Its new reuse
counters are zero on this persisted restoration path. Compilation takes 108.494 s:
source preparation 14.702 s, strict kernel validation 0.620 s, final closure
37.723 s, final references 7.439 s, and effective audit 47.949 s. Audit reuse setup
is a nested 0.417 s. Cache blob authentication is 32 ms and decode/frontier hashing
208 ms. Optimizing those small phases cannot solve the open latency.

Runtime tracing identifies additional disjoint costs: KerML graph decode/kernel
validation 19.222 s, decoded graph recanonicalization 7.353 s, and Systems graph
decode/kernel/dependency authentication 17.629 s. Dependency memoization preserves
the exact accepted archive identity. No acceptance or authentication work was
omitted to obtain this result.

Peak process working set was 5,516,029,952 bytes; whole-process wall time was
184.924 s. No other local build, test or semantic workload ran during measurement.
The 16 GB host nevertheless had less than 1 GB free physical memory during this
run. These are single observations under host memory pressure, not distributions
or shipping SLAs. Native queue, layout, GPU upload and displayed-frame time are
excluded. Golden-edit incrementality is a separate pending oracle.
