# Unnamed usage closure regression

`unnamed_constraint_and_connection_usages_close_under_occurrence_owner` adds one
local OccurrenceDefinition and three unnamed composite usages (ConstraintUsage,
AssertConstraintUsage, ConnectionUsage) to the existing genuinely closed, layered
Actions fixture. Their owning memberships are unnamed too. No producer is
bypassed and no standard publication is fabricated.

On the source-population identity fix (`7120fb7`), the focused test fails its
expected Complete assertion: closure converges after seven fixed-point rounds,
but six producer pairs remain incomplete. EffectiveTyping remains open for all
three new usages and also for the independent transition and its parameters.

The causal trace contains this chain:

1. Connection `50023` / `deriveUsageMayTimeVary` blocks Constraint `50021` /
   `KerML.CrossDomain` through a `mayTimeVary` property read.
2. Constraint `50021` / `KerML.CrossDomain` is treated as a future producer cause
   for unrelated transition `50000` specialization, structural, and owned reads.
3. The transition typing population remains open, preserving negative-query
   diagnostics rather than inventing a closure proof.

This is a bounded reproduction, not a completed fix or an accepted-library gate.
The closure owner is investigating generic read/effect precision.

Verification (low-disk debug/test profile, two build jobs):

- `cargo test --locked --offline -p agq-sysml-semantics unnamed_constraint_and_connection_usages_close_under_occurrence_owner -- --nocapture`:
  exit 101; one failing regression, 18.66 seconds.
- Same command with `AGQ_PRODUCER_CAUSAL_TRACE=1`: exit 101; same failure,
  18.54 seconds. Raw trace remains ignored under `verification/generated/`.
- `cargo fmt --all`: exit 0.
