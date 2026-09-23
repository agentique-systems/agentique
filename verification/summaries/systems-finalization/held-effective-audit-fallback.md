# Bounded effective audit diagnostics

This change was prepared in isolation from finalizer `047807a`, then integrated
after the first finalization attempt rejected acceptance. It was not part of
that measured attempt and does not establish an acceptance result. No compiler
or test process was started while the lead's finalizer was running.

The effective SysML audit retains every existing query and proof requirement.
Its audit body is unchanged. Query contexts are bounded to eight subjects each.
The default executes one context at a time; `--audit-workers=2` opts into two
independent contexts borrowing the identical immutable graph. The option is
accepted only with `--finalize-converged`; absent, malformed, duplicate, zero,
negative and greater-than-two worker values cannot select an unbounded mode.
Omitting the option selects serial execution.

Each worker drops its query caches before returning a compact audit report.
At most two reports are retained before merging in original semantic subject
order, preserving per-subject diagnostic order and exact checked counts. Worker
panics propagate; they cannot become empty successful reports. Synchronized batch
begin/end observations include subject bounds, worker count, elapsed time and
finding count. These observations confer no publication authority. A two-worker
window emits its end records after both workers join, in subject order.

Regressions cover out-of-order worker completion,
exact serial/parallel report equality, the two-worker/eight-subject bounds, panic
propagation and CLI option rejection. Full acceptance audits must still pass.

| Command | Exit | Result |
| --- | ---: | --- |
| `cargo fmt --all -- --check` | 1 | Initial CLI test array formatting differed. |
| `cargo fmt --all` | 0 | Formatting corrected. |
| `cargo fmt --all -- --check` | 0 | Formatting passed. |

Actual integrated verification is recorded in [commands.json](commands.json).
Two workers can increase scratch memory despite
sharing the graph; the retained finalization watchdog and reserve remain required.
The fallback does not touch producers, closure certificates, checkpoint identity,
the public query evidence contract, or transactional artifact issuance.
