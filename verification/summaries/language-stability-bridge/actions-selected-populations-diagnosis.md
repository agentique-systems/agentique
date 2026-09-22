# Actions selected-population diagnosis

Read-only inspection of
`verification/generated/language-stability-bridge/actions-selected-populations/report.json`:
493/534 Complete, 41 Incomplete, zero kernel obligations. No unresolved,
ambiguous, invalid or mismatched mandatory reference. This is scoped diagnosis,
not accepted publication evidence. No corpus or producer run was started.
Report SHA-256: `4f5ef175af3f8b6a67b0ceee85d8ea0904655ee7c2745f69f151cde657608a9c`.

The 41 references have 12 distinct diagnostic-subject signatures, covering 13
subjects. They partition into seven source clusters (line numbers refer to the
unchanged Systems KPAR source bytes):

| Source cluster | References | Immediate blocked subjects |
| --- | ---: | --- |
| Actions:517, ForLoop body | 5 | ActionBodyParameter `b7597899` (assigned separately) |
| Actions:222, transition accept `apayload: Anything via receiver` | 2 | TriggerAction `35c0b23f` |
| Items:59, cast/dimension asserted constraint | 4 | AssertConstraintUsage `b7b86df4`, local anchor `24f969b0` |
| Items:61–72, enveloping item and asserted constraint | 8 | ItemUsage `ed560604`, AssertConstraintUsage `b5c5b6b4`, anchor `24f969b0` |
| Items:83–86, redefined faces and asserted constraint | 7 | ItemUsage `e4549b21`, nested `inter` `ea66cc20`, AssertConstraintUsage `864cabf4`, anchor `24f969b0` |
| Items:88–92, redefined edges and asserted constraint | 9 | ItemUsage `a1b62a10`, nested `inter` `1431af2f`, AssertConstraintUsage `4b2fa25a`, anchor `24f969b0` |
| States:77, exclusive-state sequencing asserted constraint | 6 | AssertConstraintUsage `9a58b06c` |

`24f969b0-b31e-527a-8093-fedcbda1df00` is Item::checkedConstraints, the original
`abstract constraint checkedConstraints: ConstraintCheck[0..*] :> constraintChecks,
ownedPerformances` declaration at Items:124. It appears directly in 22 reference
diagnostic sets. Its generic Constraint/Occurrence specialization families are
Pending; its checked-constraint specialization is EvaluatedComplete.

All five asserted-constraint subjects have KerML.ExpressionResult
EvaluatedIncomplete, KerML.FeatureValue and VariableFeaturing Pending, and
deriveUsageMayTimeVary EvaluatedIncomplete. Their own generic Constraint,
checked-constraint and Occurrence specialization evaluations are Complete.
That does not establish their dependency closure. The result-expression producer
queries outer and inner structural results, so retained result/featuring proofs
can still reach the shared open anchors.

The transition TriggerAction has all AcceptAction, Action and Occurrence
specializations EvaluatedComplete; VariableFeaturing is Pending and
deriveUsageMayTimeVary is EvaluatedIncomplete. Its `via receiver` NodeParameter
`a1ff5677-154c-54fa-b0b4-78c9a54a3c73` also has FeatureValue and
PositionalRedefinition EvaluatedIncomplete. This is a bound parameter dependency
shape, rather than a missing selected endpoint.

The 245 final diagnostics comprise 169 KQ_PRODUCER_CLOSURE, 62
KQ_VARIABLE_FEATURING (missing snapshot domain), 13 KQ_VALUE_CONTEXT (incomplete
featuring domains), and the one known ForLoop owner-type antecedent. There are
no additional target, arity or unsupported-rule diagnostics. All 185 distinct
diagnostic subjects map to original declared source productions.

**No independent additional defect is established by this report.** Its bounded
explanations expose evaluation states, not retained per-reader causal paths.
The local standard-anchor cycle reproduced by the closure owner can therefore
explain these clusters too. Do not start another publication run to distinguish
them. If they survive that correction, the smallest useful fixtures are:

- An occurrence-owned asserted constraint with a real result expression, such
  as `assert constraint { x == x }`, using a local ConstraintCheck anchor over
  sealed KerML. Include ResultExpressionMembership and its result dependency;
  an empty constraint body does not exercise this path.
- A transition accept action with a bound `via receiver` NodeParameter over
  local Systems anchors. Check the trigger's complete specialization evidence,
  its variable flag, and the parameter's positional/value featuring closure.

Source mapping command (exit 0):
`target/foundation-evidence/debug/examples/sysml_diagnostic_sources.exe <diagnosis-subjects.json>`.
Its input was the report's final 245 diagnostics; it found all 185 subject IDs
without replaying producers. Generated input, mapped JSONL and empty unmapped
output remain in the report directory and are ignored under ADR 0021.
