# Independent transition positive-chain review

**GO for `4425065`** (review checkout `8af0d6f`). The transition payload producer uses exhaustive ancestry only to discover possible witnesses. Suppression requires both the complete ordered chain ending in the selected trigger/payload identities and a positive canonical specialization witness from the second owned input parameter. Only those two proofs join the retained trigger/input antecedents. Missing witnesses cause the existing deterministic proposal; they never establish a negative conclusion.

This preserves existing equivalent-chain suppression and the ordinary derivation's canonical IDs and trigger-before-payload order. The regression proves exhaustive discovery sees an unrelated derived ancestor's premise while repeated suppression excludes it. The existing canonical-witness regressions, rerun with the preceding 97-test producer gate, cover selected-edge change/removal, implied filtering, conjugation, and cycles. The antecedent's own completeness still governs the result.

Independent `cargo test --locked --offline -p agq-sysml-semantics --lib transition_tests` passed all six tests. Exact command, exit code, source identity, duration, and output hash are recorded in [the compact command summary](independent-transition-positive-chain-review.json).

The receiver microfixture remains outside this decision: its aggregate ownership contribution evidence is under a separate correction. No publication attempt or acceptance claim was made.
