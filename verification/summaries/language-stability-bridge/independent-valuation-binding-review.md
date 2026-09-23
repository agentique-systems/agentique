# Independent valuation/binding review

Scope: production `0f05107` (review checkout `e15067d`), with independent rule-ownership regressions `f6b691d`, `0fd99ce`, and profile correction `a7743b0`.

**GO for the structural valuation/contextual binding split.** This is not a Systems publication or receiver-fixture acceptance result.

- `FeatureValuation` retains the real existing-subject Subsetting capability. Its evidence is frozen before featuring/domain reads, separately for each FeatureValue, and then joined across values. Contextual binding does not regain that permission through another registered family.
- A deferred nondefault binding has no completed Structural-stage evaluation row. Empty/default-only populations can still complete from their own closed inputs. The partial-certificate regression verifies closed effective typing alongside pending binding and open membership; changing a value flag reopens valuation after reconstruction, and both values retain their flag dependencies.
- Exclusive rule ownership is resolved from the entire registry before applicability. The claiming family must apply to the actual derivation-key subject, even when it applies to another scheduled subject under Model scope. Duplicate claims are rejected deterministically, and claims change registry identity. Unclaimed producer behavior remains unchanged.
- The existing accepted KerML restoration tests still pass. No catalogue entry, accepted artifact, historical publication receipt, or default profile changed.

Independent verification: `cargo test --locked --offline -p agq-kerml-semantics --lib producer_` passed 97 tests, with two pre-existing ignored tests; this includes all five ownership regressions and the new value-strata regression. `cargo test --locked --offline -p agq-kerml-semantics --lib publication_overlay::restoration::tests` passed all five tests. `cargo fmt --all -- --check` exited 0 with no output. Exact test commands, source identity, exit codes, durations, and output hashes are in [the compact command summary](independent-valuation-binding-review.json).

Remaining bounded receiver diagnosis: class-filtered ownership still falls back to the complete aggregate slot proof when any selected relationship was appended. That can import an unrelated snapshot contribution. Its correction belongs in selected ownership evidence, preserving each selected append/adoption proof and the caller's population search; changing structural parameter ordering is unnecessary. The transition redundancy proof remains a separate pending review.
