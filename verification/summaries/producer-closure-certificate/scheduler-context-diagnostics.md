# Scheduler context mismatch diagnostics

The first Actions8 audit ended after Structural frontier 6 with the undifferentiated
`Derivation(InputContextMismatch)` error. That frontier added no elements. The
initial construction pass uses the positive reference-refinement extension, so
its next staged transition is ContextualBindings, with stable-property production
disabled. This is a diagnosis of the observed path, not a successful slice audit.

Read-only tracing found no changing context input in that factory: the accepted
dependency, root availability, bindings, formal targets and naming contract are
fixed. Certificate attachment rejection already has a distinct `Context` error.
The existing scheduler invariant now returns `SchedulerContextMismatch` with only
the changed identity field names, after applying its existing frontier-state
normalization. It does not relax any equality check or expand graph evidence.
Exhaustive destructuring requires newly introduced identity fields to appear in
this diagnostic. Producer evaluation/output contract diagnostics are separate.

A regression intentionally changes the interpretation contract at a later
stratum and verifies the rejection names exactly `semantic_extensions`.

Verification used the low-artifact environment and isolated
`target/foundation-evidence` target, based on `ea715cf` plus enum `e4e029c`.

| Command | Exit | Actual result |
| --- | ---: | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib context_` | 0 | 14 passed; 99 filtered out; 1.21 s |
| `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings` | 0 | Clean |
| `cargo fmt --all -- --check` | 0 | Clean |
| `git diff --check` | 0 | Clean |

No corpus candidate was run for this diagnostic change.
