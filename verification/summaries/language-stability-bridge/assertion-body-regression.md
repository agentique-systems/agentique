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
