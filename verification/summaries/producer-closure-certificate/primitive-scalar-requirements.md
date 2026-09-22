# Primitive scalar closure requirements

A focused regression reproduced unsound `EffectiveNaming` closure with a pending
`Scalar(ELEMENT_DECLARED_NAME)` producer: `is_closed` returned true before the
writer closed. Primitive name and visibility properties were absent from the
Naming and Membership requirement masks; reference-scalar handling did not cover
these primitive values.

Both requirements now conservatively include Scalar effects. Featuring and
ValueContext already did. EffectiveTyping and EffectiveOwnership retain their
structural/reference dependencies, and the regression confirms these remain
closed for the independent primitive writes. The registry digest already encodes
the requirement/effect matrix, so the changed masks invalidate old registry-bound
certificates. No accepted KerML receipt encoding is changed.

The regression checks both declared-name and membership-visibility effects with
Pending, EvaluatedIncomplete and EvaluatedComplete writer states. Verification
used the low-artifact environment and isolated `target/foundation-evidence`
directory, after transport `de69dd0` and its tests `7a14a19`.

| Command | Exit | Actual result |
| --- | ---: | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib primitive_scalar_writers_keep_naming_and_membership_requirements_open` before fix | 1 | Reproduced premature EffectiveNaming closure |
| Same focused command after fix | 0 | 1 passed; 0.09 s |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure_tests` | 0 | 35 passed, including 60k-subject fixture; 10.15 s |
| `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings` | 0 | Clean |

No corpus candidate or publication acceptance is claimed.
