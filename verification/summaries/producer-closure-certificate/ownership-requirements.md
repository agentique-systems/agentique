# Ownership effects reach every closure requirement

A regression constructs a strict graph with three existing, unowned
relationship carriers. An additive kernel derivation attaches them to existing
owners without creating elements or changing their reference targets.

| Complete current-graph query | Before attachment | After attachment |
| --- | --- | --- |
| `feature_types(feature)` through ReferenceSubsetting's implied source | Empty | Target's Classifier |
| `owning_type(feature)` | None | Owning Classifier |
| `featuring_types(feature)` | Empty | Owning Classifier |
| `effective_features(classifier)` | Empty | Original Feature identity |
| `effective_names(other_feature)` through owned Redefinition | Empty | Target's declared name |
| `common_connector_context([feature])` | None | Owning Classifier |

Before the correction, a generic Model-scope Ownership-only producer marked
Incomplete still allowed EffectiveTyping closure. The regression reproduced that
false positive after confirming all six before/after query results. Ownership is
now an explicit dependency of all six requirements. The test checks the complete
matrix with Pending, EvaluatedIncomplete and EvaluatedComplete producer states.
Registry identity already includes this requirement/effect mapping.

Verification on `532dfa7` plus this change used the established low-artifact
environment and isolated `target/foundation-evidence` directory.

| Command | Exit | Actual result |
| --- | ---: | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib ownership_attachment_changes_queries` before fix | 1 | Reproduced premature EffectiveTyping closure |
| Same command after fix | 0 | 1 passed; 0.17 s |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure_tests` | 0 | 38 passed including 60k-subject fixture; 12.21 s |
| `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings` | 0 | Clean |
| `cargo fmt --all -- --check` | 0 | Clean |
| `git diff --check` | 0 | Clean |

No corpus candidate or publication acceptance is claimed.
