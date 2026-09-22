# Independent selected-ownership and direct-end review

Reviewed the five-file correction `f6eaac7` (closure-owner `0c96e7a`) and its
unnamed usage regression `72dadb9`. Scope was limited to the changed ownership
support and direct end-population paths; no publication or conformance audit ran.

Disposition: **go for the corrected bounded Actions gate**. This review does not
establish Systems publication acceptance or foundation readiness.

No additional soundness finding in the inspected change. The inverse property
path retains its current `SourceRelationships` search and uses original support
only if the selected endpoint occurs in the original stored slot. A new derived
endpoint still falls back to current proof expansion. Explicit broad reads retain
their aggregate property and closure evidence in both merge orders and evaluator
modes. Existing source-provider openness and reconstruction checks remain active.

`owned_end_features` reuses the direct ordered membership projection and reads
each Feature's `isEnd`. It does not traverse specializations. Its retained end
population distinguishes proven non-end membership producers from actual end
producers and `isEnd` writers; the latter still block closure. A pending source
namespace explicitly prevents a complete result. SysML changes only the binary
Connection/Interface and Flow predicates to use that population.

The independent closure subset passed all 61 tests, including both new
regressions, inherited-end exclusion, pending namespace and unresolved provider
cases. The whole unnamed Constraint/Connection usage fixture also passed with
complete closure (one test, 20.51 seconds). Exact commands, actual exits and output hashes are recorded in
`selected-ownership-review.json` under ADR 0021. Raw logs are ignored artifacts.
