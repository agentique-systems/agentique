# Integration checks

These are focused stage checks, not Systems publication or language-readiness
acceptance. Commands, exact source identities, output hashes and exit codes are
recorded in `summary.json`; raw logs remain ignored under `verification/generated/`.

The low-artifact wrapper uses a separate integration target, disables incremental
compilation and debug symbols, and keeps normal test assertions and overflow
checks. The latest disk preflight had over 34 GiB available. No caches were deleted.

| Check | Actual result |
| --- | --- |
| `cargo test --locked --offline -p agq-kernel` | Exit 0, including archive, construction and proof transport tests |
| `cargo test --locked --offline -p agq-kerml-semantics` at `c2a7016` | Exit 0; 259 passed, 2 existing profiling probes ignored |
| `cargo test --locked --offline -p agq-kerml-semantics --test foundation` | Exit 0; 29 passed after retaining actual ownership search evidence |
| `cargo test --locked --offline -p agq-kerml-semantics --lib` after reference-scalar and semantic-target bounds | Exit 0; 107 passed, 2 existing profiling probes ignored |
| `cargo test --locked --offline -p agq-kerml-text --lib` at `76934a0` | Exit 0; 32 passed, including pinned Systems declarations and independent programmatic/textual Vehicle equivalence |
| `cargo fmt --all -- --check` at `3357421` | Exit 0 |
| KerML and SysML metamodel runtime gates | Exit 0; commands in the two runtime JSON summaries |
| Grammar and metamodel stale checks | Exit 0; actual commands in the command summary |
| Frontend check, build, tests and standards check | Exit 0; no browser-facing code changed |

The first broader KerML package run failed four existing proof-consistency tests:
an inherited-feature proof still named a broad incoming-edge search after the
query had switched to a precise ownership projection. The production proof now
reuses actual property evidence. The invariant and tests were retained unchanged.
An independent positional test also caught a dropped FeatureChaining population;
that production fix was integrated before the successful package run.

A later library run caught one stale assertion expecting the broad owned
relationship search. Its replacement requires both actual dependencies: the
ordered Specialization population and the End feature population. Canonical
positive proof assertions remain unchanged. The subsequent 107-test run passed.

The initial SysML grammar command incorrectly used `--check`; this generator's
default invocation is its stale check. The corrected command passed. An initial
runtime-gate command used the wrong executable name; the actual `metamodel-gen`
binary passed both baseline checks. Neither correction changes test semantics.

Subsequent descriptor/projection changes require their focused checks and the
final integration checks. The workspace test has not yet been run in this
milestone. No full Systems publication attempt has been used.
