# Independent review: actual bounded decision-provider UI

Reviewed exact commit `ba7705451c819fe3a6e2af852358371020f1d28c`. Scope: source,
diff and focused-test inspection by the independent runtime/product reviewer.

**Source review passes; execution and visual qualification remain pending.**
The native card now calls the existing actual `MockViewDecision` through
`&dyn DecisionModel`, rather than constructing an illustrative result. Both
hardcoded native score displays are removed. No live provider or semantic
mutation capability is introduced.

The retained inquiry supplies the intent, exact revision and original root ID.
Later pointer selection does not silently change the observation. Cache identity
includes runtime epoch, project/revision binding, observed revision, retained root,
root name and fixture state. Stale/missing-root, revision and fixture contexts
cannot reuse a visible recommendation. The selected provider is the fixed local
mock; no network execution is performed in the UI frame.

The returned result must match the observation revision and exactly the offered
choice question. Only System, Graph, Requirements and History can be selected.
Empty provider identity, wrong question/type, duplicate answers, unoffered choice,
duplicate/unoffered weights and nonfinite or out-of-range weights/confidence are
rejected. Missing values stay missing: no inferred weights, normalization or
calibrated-probability claim is introduced.

The actual mock returns a choice with empty weights and no confidence. The card
labels it “Deterministic view mock”; the advanced observation shows the returned
provider identity, intent, revision and inquiry subject. Returned numeric values,
if any, are explicitly uncalibrated. The retained inquiry's target/revision remain
visible in the containing activity card.

Opening a recommendation requires an explicit operator click and maps only to
the existing World navigation commands. There is no prepare, validate, commit or
source-edit mapping. When the recommended World is already open, the action is
disabled and says so. This avoids presenting a redundant action as a new agent
result, though the mock remains a simple routing demonstration rather than an
architecture analysis capability.

Three focused test groups exercise actual mock routing for four intents; ten
malformed provider responses and preservation of partial returned weights; and
actual app observation caching with later selection, root replacement and stale
revision/fixture rejection. These were inspected, not executed by this reviewer.
They do not use a fake accepted runtime.

The retained [format record](../checks/native-decision-provider-format.json) and
[diff record](../checks/native-decision-provider-diff.json) both exited 0. No builds
or standards consumers ran during this independent review. Require integrated
tests/Clippy and a native capture before calling the card visually qualified.
Recheck that it explains the actual inquiry and leaves the engineering scene
prominent; source correctness does not establish polished agent workflow.
