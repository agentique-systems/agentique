# Warm real-project read profile, source 42e7e187

The authenticated CI helper reopened a SQLite backup of real-run03's committed
repository in a separate process. Its compiled source is exactly
`42e7e187de823a3b855d530dcee84dbe8c06b59f`, identified in `backup.json` and the
retained `../ci-helpers-03` build receipt. The observer's checkout-at-launch field
is `d17884ff`, an evidence-only descendant, not the helper's compiled source.
No local semantic workload or compiler overlapped this measurement.

| Boundary | Observed |
| --- | ---: |
| Warm project open, including runtime | 151.657 s |
| KerML runtime restoration | 31.077 s |
| SysML runtime restoration | 24.064 s |
| Repository open | 27.6 ms |
| Revision restoration | 95.975 s |
| First System projection | 48.9 ms |
| Focused System projection | 255.0 ms |
| Focused Graph projection | 44.1 ms |
| Requirements projection | 44.8 ms |
| First Inspector | 183.3 ms |
| Repeated Inspector | 0.206 ms |
| Repeated projections | 0.794–0.864 ms |
| Process peak working set | 5,524,643,840 bytes |

The process exited 0 after 154.765 seconds. Repeated DTOs compared exactly.
These are service/read boundaries; layout, GPU upload and first displayed frame
are measured separately by the real native journey. Timings are nested and must
not be summed. The prior read-profile-02 warm open was 181.258 seconds, but these
single observations do not estimate run variance. The revision and compiled
source are explicitly different; this is progress evidence, not an isolated
optimization attribution or a satisfied warm-open target.
