# Positive occurrence-owner evidence

Bounded reproduction: an owner has a declared specialization path to the pinned
Occurrence and an unrelated derived ancestor whose proof reads the child's
EffectiveTyping closure. `current_usage_may_time_vary` previously imported that
unrelated dependency from its exhaustive owner-ancestor query. The new regression
failed with exit 101 on the old code (raw output in ignored generated evidence).

The positive owner premise now uses the existing checked canonical path witness.
If no witness exists, ordinary complete ancestor semantics remain the fallback;
no negative conclusion is inferred from witness absence. Both negative closure
requirements remain unchanged. The regression also requires the selected edge's
endpoint evidence and verifies that a final scalar proposal still waits for the
child's excluded-type closure.

The loop micro now declares only whileTest and body, inheriting untilTest, as the
actual while-only syntax does. Its three effective parameters, true mayTimeVary,
shared snapshot domain and Complete typing are asserted. Removing the extra local
untilTest did not independently reproduce the corpus failure.

Low-disk environment, one build job; no corpus or accepted-cache load:

| Command | Result |
| --- | --- |
| `cargo test -p agq-sysml-semantics may_time_vary_positive_owner_uses_only_selected_occurrence_path -- --nocapture` before correction | exit 101; unrelated ancestor proof assertion failed |
| `cargo test -p agq-sysml-semantics may_time_vary_ -- --nocapture` after correction | exit 0; 3 passed |
| `cargo test -p agq-sysml-semantics input_action_body_closes_under_while_loop_with_nested_actions -- --nocapture` | exit 0; 1 passed, 32.59 s |
| `cargo clippy -p agq-sysml-semantics --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff --check` | exit 0 |
## Independent review

**GO for this scoped correction.** Independent checks on `1fa256d` over root
`31633ba` (review-worktree equivalent `4eae198`) found no remaining issue. The
positive owner premise retains the canonical helper's selected endpoint and
derivation evidence, including its provider/conjugation guards. No witness still
uses ordinary semantic ancestry. Negative owner-type and usage exclusion-type
closure requirements are unchanged.

| Command | Actual result | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-sysml-semantics --lib may_time -- --nocapture` | 4 passed, 1.34 s | 0 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib specialization_witness -- --nocapture` | 5 passed, 0.29 s; selected derived proof, changed/deleted edge reopening, implied filtering, conjugation/provider and chained-path cases | 0 |
| `cargo test --locked --offline -p agq-sysml-semantics --lib input_action_body_closes_under_while_loop_with_nested_actions -- --nocapture` | Complete, 1 passed, 34.28 s; two local parameters plus inherited third retain exact IDs, varying body and shared snapshot | 0 |
| `git diff --check` | No output | 0 |

`positive-owner-review.json` records actual commands, source/patch identity,
outputs and exits under the low-artifact runner. No corpus publication was run
by the reviewer; this result is not Systems publication or readiness acceptance.
