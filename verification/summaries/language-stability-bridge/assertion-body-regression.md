# Assertion body closure regression

The bounded Actions audit at `actions-positive-witness/report.json` reports
499/534 Complete references, 35 Incomplete and zero kernel obligations. Compared
with the 493/534 run, six Items references closed; the remaining clusters are
TriggerAction (2), loop body (5), four Items assertions (22), and a States assertion
(6). All five assertion subjects retain incomplete ExpressionResult and
MayTimeVary processing, with FeatureValue pending.

`unnamed_assertion_body_closes_with_inherited_result_and_expression_binding`
reproduces a non-loop cycle on root `6bda603`. It extends the genuinely closed
layered fixture with an unnamed AssertConstraintUsage under an Occurrence owner,
a ResultExpressionMembership body, and a Boolean literal with its own structural
result. The assertion inherits its return parameter; no inherited copy is added.
The synthetic Performances anchors include the real evaluations →
booleanEvaluations → true/falseEvaluations specialization structure, avoiding
ambiguity from otherwise unrelated placeholder return parameters.

Focused command: `cargo test -p agq-sysml-semantics
unnamed_assertion_body_closes_with_inherited_result_and_expression_binding --
--nocapture`, with the low-disk profile and `AGQ_PRODUCER_CAUSAL_TRACE=1`.
Result: exit 101, one failing test after 16.06 seconds. The final stage is
Incomplete, with five incomplete producer pairs and no arity/invalid diagnostic.
The intended assertion remains Complete; it is not weakened to preserve the bug.

The causal trace includes ExpressionResult(50022) reading its own pending
FeatureValue population and importing unrelated sibling/transition proof reads;
MayTimeVary(50022) depends on VariableFeaturing(50022). The ignored diagnostic
trace is `verification/generated/language-stability-bridge/assertion-body-micro.log`.
This commit changes tests only; it does not assert a root cause or rerun a corpus
publication. An earlier fixture draft correctly failed result arity and was fixed
before retaining this regression.

The reproduced defect was an overstated `ExpressionResult` producer contract.
The existing Expression/Function only gains memberships; its contextual chaining,
reference subsetting and featuring concern generated helper features. The
descriptor now keeps chaining in fresh effects and enumerates all seven emitted
relationship classes, including BindingConnector. It cannot introduce a
FeatureValue carrier. No query or negative closure requirement was weakened.
The registry digest changes; accepted historical KerML publication bytes and
the `/26` identity remain untouched.

The same assertion micro now reaches Complete (exit 0, 18.91 s). Its effective
result remains the inherited canonical identity and its expression binding is
materialized. Two focused KerML tests pass (exit 0):
`expression_result_effects_preserve_subject_typing_and_restrict_value_carriers`
and `expression_and_function_result_emissions_fit_exact_relationship_contract`.
They check closure/read boundaries, reject chaining on the existing subject,
allow chaining on a generated helper, audit actual producer output, and enumerate
all emitted Relationship subclasses for Expression and Function in both the
published and Operational v9 profiles. `cargo fmt --all -- --check` and
`git diff --check` also pass. Verification used one build job and the low-disk
profile; no corpus or accepted-cache load was run.
## Independent effect-contract review

**GO on `ba9f465`** (review-worktree equivalent `fb719b6`). The reviewer traced
the ExpressionResult dispatch through every `contextual` and `binding` emission
path, including FunctionResult and the earlier raw-inner profile branch. Only
fresh helper Features receive FeatureChaining; existing expressions/functions
receive memberships. Real existing-subject chaining remains rejected by effect
validation. The initial draft omitted BindingConnector, itself a Relationship;
the final seven-class set includes it. No other omitted emission was found.

| Command | Actual result | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure -- --nocapture` | 64 passed, 11.76 s, including exact actual relationship emission sets for Expression/Function under Published/v9 and existing/fresh effect enforcement | 0 |
| `cargo test --locked --offline -p agq-sysml-semantics --lib unnamed_assertion_body_closes_with_inherited_result_and_expression_binding -- --nocapture` | Complete, 1 passed, 19.27 s | 0 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib publication_overlay::restoration::tests -- --nocapture` | All 5 accepted-restoration guards passed, 2.31 s | 0 |
| `git diff --check` | No output | 0 |

Actual low-artifact command/output records are in `expression-effects-review.json`.
The combined producer-registry identity changes as required by the corrected
contract. Historical accepted KerML `/26` artifacts and rule bodies are unchanged;
the existing restoration checks retain historical context/protected dependency
behavior and reject identity/source/capability tampering. This scoped review did
not run a corpus publication or establish Systems acceptance.
