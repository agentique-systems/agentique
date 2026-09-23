# Feature valuation and contextual binding

Feature valuation now has a separate structural producer family. Its existing
subject Subsetting remains a typing effect. Contextual value binding retains
its own later evaluation, membership/binding effects and newly created helper
effects. An actual deferred nondefault binding is not certified in the
structural phase; empty/default-only populations can finish with their evidence.
Every FeatureValue contributes to both appropriate read sets, and structural
helper searches are finalized before contextual domain queries.

The registry now optionally assigns exclusive derivation rules to a family.
Schema `/4` includes those claims, rejects duplicates and prevents an output
from borrowing a different family's effects. Applicability is checked against
the derivation-key subject as well as the scheduled population. Existing
valuation/binding derivation rule IDs and historical accepted KerML bytes do
not change. FeatureReferenceExpression and binding relationship classes now
describe only their possible canonical carriers.

Validation (one build job, incremental/debug disabled):

- `cargo test -p agq-kerml-semantics --lib producer_ -- --nocapture`: exit 0;
  97 passed, 2 ignored, 31.60 s. Includes five independent rule-ownership tests
  and the partial-certificate/multiple-value/read-transport regression.
- `cargo clippy -p agq-kerml-semantics -p agq-sysml-semantics --all-targets -- -D warnings`:
  exit 0, 9.55 s.
- `cargo fmt --all`: exit 0.
- Follow-up `cargo test -p agq-kerml-semantics`: exit 0; 327 passed,
  2 ignored across unit/integration tests. Documentation tests also passed.

The receiver fixture now reaches Complete with the selected transition witness
and exact creation/append contribution support. The full SysML package passes
73 tests; see `selected-append-merge-regression.md`. Actions/Systems publication
acceptance remains a separate gate. Raw bounded test output is ignored under
`verification/generated/language-stability-bridge/`.
