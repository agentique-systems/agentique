# Native decision provider use

The previous native agent card displayed a fixed Graph recommendation and 0.72
weight. A second block constructed an illustrative DecisionResult directly, rather
than invoking the repository's DecisionModel. Neither represented actual provider
output. Both have been replaced by one recommendation card.

The selected provider is the existing `MockViewDecision`, called through
`&dyn DecisionModel` with a bounded next-view Choice question. Its observation names
the exact retained dependency inquiry root and revision, not all query result nodes
or the operator's later selection. A presentation cache includes runtime-open epoch,
project binding, revision, inquiry root/name and fixture status. Missing or stale
observation context cannot publish a cached recommendation.

The current mock returns a Choice and no weights or confidence. The UI therefore
shows "Choice only · no weights returned" and identifies the deterministic mock.
Decision details preserve the actual provider identity, intent, revision, inquiry
subject and offered views. No distribution is manufactured or normalized. If the
contract returns weights, only unique offered options with finite [0,1] values are
displayed, explicitly uncalibrated; absent options remain absent. Returned confidence
is similarly checked and never described as calibrated.

The host rejects another result revision, another question, non-Choice values,
unoffered choices and malformed weights/confidence. Only the four offered Worlds
map to existing navigation commands. No result can validate, commit or otherwise
mutate the model. Opening a recommendation requires an explicit operator click;
the action reads "Recommended view is open" when appropriate.

Focused tests cover actual mock routing for different intents, retained identity
without invented weights, malformed provider counterexamples, changed inquiry roots,
and stale revision/fixture observations. They have not been run in this held patch.
No live provider, external inference or semantic consumer was added or launched.
Formatting/diff receipts are retained. Compilation and an actual screenshot remain
required before qualifying the final visual result.
