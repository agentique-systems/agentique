# Held workspace integration with authenticated frontier input

Base: `7223938`, including the exact-input checkpoint attachment fix originally
prepared in `7f3cd23`, native self-model assertions from `639736a`, and the
conditional architecture documentation. This isolated branch consolidates the
unchanged held production intent of `cb84f64` and test follow-up `60bfa21`.
The earlier commits and worktrees remain intact. This does not activate workspace
production or grant Systems publication authority in the acceptance worktree.

The sole textual conflict was additive methods in `derivation_input.rs`.
`matches_archive_input` is retained unchanged beside the shared reservation
accessors. Source review confirms:

- Strict/construction input kinds remain distinct. Both element and occurrence
  reservation sets must match, including identities absent from the graph;
  construction obligations must also match.
- The immutable dependency must be the same Arc. Registry compatibility is
  checked in both directions, and canonical records, occurrences, navigation,
  statuses, searches and selected contributions must be exactly equal.
- SharedMap/SharedSet equality compares ordered effective contents, including
  inherited entries and local overrides; physical storage layout is not used as
  semantic equality. Thus this check authenticates the same content as the
  prior BTreeMap/BTreeSet representation.
- Decoding authenticates the declared input before substituting the supplied
  Snapshot/ConstructionView and before validating the restored overlay. Subsequent
  builders retain that exact supplied input and its revision. The scheduler's
  two restore call sites still use the new `read_*_frontier_on` APIs.
- Record, occurrence, navigation, status and search dependency protections,
  retired-ID rejection, selected proof/search checks, ownership validation and
  proof-cycle checks remain in place. Accepted dependency graph/evidence tables
  attach through immutable bases; local restored rows remain local.

The existing construction-frontier regression now also observes the physical
dependency tables after exact-input attachment. It requires all expected
publication table identities and zero copied dependency entries, alongside the
retained original-Arc, revision, obligations, semantic-content and continuation
checks. This assertion is not yet compiled or executed.

Only source checks run for this consolidation: recorded Rust formatting,
`git diff --check`, and an exact source comparison against `7223938` for the
authentication matcher, both attachment APIs, the attach-before-validation block,
the scheduler adapter and its checkpoint-equivalence tests. Their actual commands,
outputs and exit codes are in `workspace-authenticated-frontier-commands.json`.
No Rust build/test, accepted-cache restore or corpus process ran here.

After readiness, perform the serialized final checks in
`workspace-post-readiness.md`. The focused first runtime gates for this merge are:

```text
cargo test --locked --offline -p agq-kernel --features verification --test archive --test declared_history --test immutable_dependencies --test derived_associations -- --test-threads=1
cargo test --locked --offline -p agq-kerml-semantics --features verification --lib frontier_tests -- --test-threads=1
cargo test --locked --offline -p agq-modeling-workspace --features verification --test working_states --test self_model --test phase1 -- --ignored --test-threads=1 --nocapture
```

The last command requires the actual accepted cache inputs and receipt described
in the final run plan. Earlier passing ledgers retain their original source
identities; they do not claim this new integration has compiled or passed.
