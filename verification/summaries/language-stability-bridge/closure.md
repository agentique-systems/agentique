# Producer closure integration and regressions

Current status: focused corrections are integrated; Systems publication and
language readiness remain unaccepted. The latest completed corpus gate (`08e4a83`) has
532/534 Actions references Complete, two Incomplete and zero kernel obligations.
Following the earlier 499/534 audit, the loop, assertion and receiver regressions close; the receiver
passed on `55b5af8` in 33.80 seconds. The complete SysML package then passed
73 tests in 71.38 seconds, including unsupported variation remaining Incomplete.
These synthetic fixtures use genuinely closed
layered dependencies, not fabricated accepted standard publications.

Initial zero-writer closure accounts for unresolved source endpoints, pending
namespace populations and applicable future writers. Checkpoint transport compares
registry/context, original and current records, endpoints, negative searches,
selected support and producer opportunities; unchanged-looking aggregates alone
cannot preserve a conclusion. Explain distinguishes `AcceptedDependency` from
`LocalProducerClosure`. Immutable dependency records are protected while local
source/inverse populations remain open to qualifying local writers. A definitely
absent owner requires ownership closure; an existing owner's negative class test
also requires effective typing closure. The optional context/certificate contract
is `/2`, the producer registry is `/4`, SysML queries are `/5`, and historical
accepted KerML `/26` interpretation and receipt identities remain unchanged.

## Defect and regression matrix

Implementation/review commits below identify the corrections. Exact command
identities and hashes are in [commands.json](commands.json) and its source-ledger
ranges; the retained observation table covers older Markdown-only results.

| Boundary / concrete failure | Corrected behavior and permanent coverage | Implementation / observed outcome |
| --- | --- | --- |
| Initial closure ignored source providers: Feature 1 appeared to have no types while FeatureTyping 3 had `type=2` but unresolved `typedFeature` | `initial_closure_` tests keep unknown FeatureTyping, Redefinition, Subsetting, FeatureChaining and Membership endpoints open; pending namespace ownership is an independent provider even without a lower-bound obligation | Reviewed `2aa9fb8`; initial tests exit 101; `4390c42` plus `9ad3d3a` fix direct and transitive cases; independent subset 52 passed, owner's subset 53 passed |
| Rebinding reopened a Feature but retained a completed Classifier reader; delayed attachment could adopt a Membership already owning a Step | Causal readers and effects reopen transitively; pending owning-carrier masks propagate to descendants. Lost output support also reopens evaluations | `9ad3d3a`; independent review permits the next scoped audit, not publication |
| Synthetic immutable fixtures had no authentic dependency closure | `ProducerClosedDependency` checks the exact graph/context/certificate and independently expected full registry; genuine KerML ? Systems ? authored layering shares Arcs. Ordinary `isPortion=false` exercises negative predicates | Witness `aaba779` / root `de501b5`; fixture `857b12e`; original 49 passed/5 failed; final 62 SysML tests passed. No accepted receipt is minted |
| Equal aggregate `[a,b,c]` concealed original `[a]` versus `[a,b]`; mixed current/source reads could miss the first append | Opted-in producer digest and checkpoint fingerprints include original slot values/origins. `DeclaredProperty` preserves source-only reads while mixed reads retain their current root/search in either merge order and evaluator mode | Independent counterexamples on `3942e68`; fix `7120fb7` (root source binding `47554aa`); 59 independent closure tests, producer identity and anchor-provenance tests passed; dependent archive bytes and shared dependency preserved |
| Owner absence, unknown owner and producer-introducible owner were conflated by insufficient providers | `formal_owner_absence_requires_a_witness_until_delayed_attachment`, `initial_zero_writer_proof_closes_absent_owner_without_a_scheduler_round`, and pending-namespace tests retain typed closure evidence and its source | Layered Actions tests assert a false Complete formal antecedent only with real closure. Five immutable-boundary checks cover external source relationships and noncomposite inverse populations |
| Unnamed Constraint/AssertConstraint/Connection usages imported sibling append proofs through inverse ownership; broad feature populations blocked binary ends | Selected original edge evidence retains the current inverse search. `owned_end_features` is direct, ordered and excludes inherited ends; real end writers, `isEnd` changes and pending sources still block | Fixture `a96fb66` / `72dadb9`; correction `0c96e7a` / root `f6eaac7`; original Incomplete, then Complete in 19.39 s; independent 61 closure tests and 20.51 s fixture pass |
| Positive ancestry used to suppress a redundant proposal imported unrelated ancestor evidence; B,C,A planning lost the retained proposal's antecedents | `canonical_specialization_witness` retains a selected canonical path, endpoint/provenance/provider/conjugation guards and implied-edge filtering. No witness is not an absence claim. Ordered suppression retains its off-subject antecedents | `e3e92e0` (root `b509a81`), lint-only `6a31278`; five path tests, two proposal tests, owner-edit and B,C,A regressions pass. Additional ordinary implied edges may remain: no inherited copies or minimal-edge-count promise |
| Positioned parameter/end/result queries imported creation proofs of excluded snapshot members | Selected carriers retain positive proofs; excluded members retain scalar, endpoint and identity guards. Failed/uncomputed scalar states remain non-Complete; reconstruction catches class changes, retargeting and scalar activation | `e3e92e0`; three `positioned_guard` tests and mixed/broad-read regression pass. Existing identity reads are inert only during additive production, not reconstruction |
| `mayTimeVary` positive Occurrence owner imported an unrelated derived ancestor whose proof read the child's EffectiveTyping | Use a selected Occurrence specialization witness; ordinary complete ancestry remains fallback. Negative owner/exclusion closure requirements stay intact | `1fa256d`; old-code assertion exit 101; independent four `may_time` tests pass. While-loop fixture has only local whileTest/body plus inherited untilTest, true mayTimeVary and one shared snapshot domain |
| Assertion result producer claimed existing-subject FeatureChain / FeatureValue capabilities, causing its own value population to stay open | ExpressionResult existing-subject effects are membership-only; chaining/subsetting/featuring belong to fresh helpers. Exact seven relationship classes include BindingConnector and exclude FeatureValue | Fixture `cb74d93` on `6bda603`: five incomplete pairs, exit 101, 16.06 s; fix `ba9f465`: Complete in 18.91 s; independent 64 closure tests, assertion Complete in 19.27 s, five historical restoration guards pass |
| Structural value typing and contextual binding shared one evaluation/effect family | `FeatureValuation` retains subject Subsetting; later contextual binding has independent evaluation and helper effects. Partial certificates cannot mark deferred nondefault binding complete. Every value retains its own flags; changes reopen it | `0f05107`, rule-ownership tests `f6b691d` / `0fd99ce`, profile correction `a7743b0`; 97 producer tests pass, two existing ignores; full KerML package 327 passed plus doc tests |
| A producer could borrow another family's effects or an applicable scheduled subject | Registry `/4` binds exclusive rule claims, rejects duplicates, and checks applicability to the actual derivation-key subject. Unclaimed behavior is unchanged | Same valuation correction; five independent ownership tests and five historical KerML restoration tests pass |
| Transition payload suppression imported an unrelated derived ancestor | Exhaustive ancestry only discovers candidates; suppression requires one Complete canonical specialization witness plus the exact ordered trigger/payload chain and original trigger/input antecedents | `4425065` (review `8af0d6f`); six transition tests pass, deterministic IDs and trigger-before-payload order preserved |
| A selected appended relationship fell back to the complete owner-slot proof, importing unrelated snapshot contributions | Kernel per-reference support retains explicit append/adoption proof, only owner/selected-target automatic dependencies, position and batch searches. Broad current reads still merge in either order; search-only updates evict unattributable precision | Kernel `b6bcfaa`; five final independent semantic regressions pass; production `fa3bbdd`, tests `dc888fd` / `7adca6b` (root `4f18a0d`); earlier failed guard/proof assertions are retained in the ledger |
| A newly created helper's initial two-FeatureChaining slot had no per-reference support, leaving seven receiver producer pairs open | Initial ordered derived slots retain selected target and validated owner creation evidence, including record/slot negative searches. Repeated targets fall back to aggregate evidence. Copy-on-write, construction and dependency paths preserve the optional cache | `4fa87cd` / root `55b5af8`; seven kernel regressions and 119 package tests plus three doc tests pass; receiver test now Complete in 33.80 s |

The contribution cache is optional, read-only kernel evidence. Local archives omit
it without changing canonical bytes; a missing entry requires aggregate query
fallback and reopens transported selected-support reads. Exact supplied immutable
dependency entries remain available. Population searches and recursively retained
positive/negative premises are still required; the cache is not closure authority.

## Distinct failures retained for reproduction

- The original Actions selected-population report was 493/534 Complete, 41
  Incomplete, zero kernel obligations; SHA-256
  `4f5ef175af3f8b6a67b0ceee85d8ea0904655ee7c2745f69f151cde657608a9c`.
  It partitioned into loop body 5, TriggerAction 2, Items assertions 28 and States
  assertion 6. `Item::checkedConstraints` (`24f969b0-b31e-527a-8093-fedcbda1df00`)
  appeared in 22 diagnostic sets. Its 245 diagnostics were 169 closure, 62 variable
  featuring, 13 value-context and one owner-type finding. The later 499/534 audit
  closed six Items references: remaining clusters were 5/2/22/6, not a new authority
  ambiguity. Source mapping found all 185 diagnostic subjects without producer replay.
- The assertion fixture preserves a real ResultExpressionMembership, Boolean
  result and inherited canonical return parameter, with evaluations ? Boolean ?
  true/false anchor inheritance. The trace exposed ExpressionResult(50022) reading
  its own pending FeatureValue population; an earlier invalid arity fixture was
  corrected before the retained reproduction.
- The receiver fixture preserves AcceptAction ? StateUsage ? TransitionUsage ?
  AcceptActionUsage, inherited inout payload/input receiver, two transition inputs,
  and an unnamed directionless NodeParameter with a bound FeatureReferenceExpression
  and canonical result. Baseline `2a3d9cb` failed with eight incomplete pairs and no
  arity/invalid/missing-input finding (18.92 s). The trace linked FeatureValue(60015)
  through generated contextual ownership to VariableFeaturing. Initial-reference
  support now closes this fixture; the actual corpus still needs its next audit.
- Fixture-only failures remain distinguished from semantic failures: abstract
  Relationship replaced by concrete FeatureTyping; missing visibility corrected;
  a lost fixture ownership map restored; digest assertions updated to the explicit
  producer domain; ambiguous Explanation import qualified. A cross-feature test
  also caught selected-origin Arc allocation churn, corrected by `522b17c` without
  changing values, completeness or proof contents. The untyped combined-anchor
  experiment was removed because it lacked the real accepted KerML boundary.

The 60,000-subject/80-family probe remains a 2,220,216-byte compact certificate
(300,000 applicable pairs, 299,995 closed, one incomplete); issuance observations
were 6,305 and 6,082 ms. Optional checkpoint/revalidation reads are accounted
separately. Corpus proof and read-storage measurements are in the milestone README.

## Verification records

[commands.json](commands.json) retains every ordinary command record unchanged,
including source/patch identities, output hashes and exits. Its `source_ledgers`
map preserves the original filename, source Git commit, Git-blob SHA-256 and
zero-based contiguous command range for each former review ledger. Duplicate
command names and unsuccessful runs remain distinct records.
The final contribution review (`17c56c7`) passes seven kernel, five selected-proof,
97 producer and five historical-restoration tests (two pre-existing scale ignores).
Earlier checks exposed a temporary module-hook conflict, incomplete fixture state,
compact-evidence expansion assumptions and three aggregate-specific assertions;
only test setup/assertions changed, preserving selected guards and broad reads.
No failed run or zero-test filter becomes a passing gate. Most focused work used
Windows low-artifact settings (dev/test debug and incremental disabled, one or two
jobs, isolated targets); exact overrides remain in the command records. Raw output
and causal traces remain ignored under `verification/generated/`.

At `4fa87cd`: seven contribution tests, 119 kernel package tests and three doc
tests pass; all-target kernel Clippy, strict Rustdoc and formatting pass. Earlier
`7120fb7`, `522b17c`, and `e3e92e0` package stages respectively passed 145/147/156
KerML unit tests plus integration suites and 62/63/66 SysML tests. These are
historical stage populations, not the final integrated package count. Full KerML follow-up at `c38f373` plus evidence-only `6dcd35b` passed
`cargo test --locked --offline -p agq-kerml-semantics` (exit 0, 145.84 s):
173 unit tests plus 163 integration tests across 34 binaries, two existing scale
ignores, and a successful zero-test Rustdoc run. The exact output hash is in
the `independent-selected-contribution-review.json` source range in
[commands.json](commands.json); no ignored cache gate ran.
Final full workspace verification belongs to the lead's integration gate.

<details>
<summary>Retained command observations from the consolidated Markdown</summary>

The JSON ledgers retain exact source/patch identities and output hashes where recorded.
These observations preserve the additional original arguments and exits. For an
observation lacking a corresponding JSON record, **output hash is unavailable**;
its precise tested source/patch and tool version are unrecorded unless stated.
An implementation commit in the matrix is not a substitute for a tested-tree identity.
Repeated commands at different stages are distinct observations, not new acceptance.

| ID | Exact observed command |
| --- | --- |
| C1 | `cargo check --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics` |
| C2 | `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure --no-run` |
| C3 | `cargo fmt --all` |
| C4 | `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure -- --nocapture` |
| C5 | `cargo test --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics` |
| C6 | `cargo test --locked --offline -p agq-kerml-semantics --lib initial_closure_ -- --nocapture` |
| C7 | `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings` |
| C8 | `cargo clippy --locked --offline -p agq-sysml-semantics --all-targets -- -D warnings` |
| C9 | `RUSTDOCFLAGS=-D warnings cargo doc --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics --no-deps` |
| C10 | `cargo test --locked --offline -p agq-sysml-semantics --lib actions_micro_closes -- --nocapture` |
| C11 | `cargo test --locked --offline -p agq-sysml-semantics` |
| C12 | `cargo fmt --all -- --check` |
| C13 | `cargo test --locked --offline -p agq-kerml-semantics --lib declared_ -- --nocapture` |
| C14 | `cargo test --locked --offline -p agq-kerml-semantics --lib declared_source_population -- --nocapture` |
| C15 | `cargo test --locked --offline -p agq-sysml-semantics --lib standard_anchor_path -- --nocapture` |
| C16 | `cargo test --locked --offline -p agq-kernel -p agq-kerml-semantics -p agq-sysml-semantics` |
| C17 | `cargo test --locked --offline -p agq-sysml-semantics --lib context_identity -- --nocapture` |
| C18 | `cargo clippy --locked --offline -p agq-kernel -p agq-kerml-semantics -p agq-sysml-semantics --all-targets -- -D warnings` |
| C19 | `RUSTDOCFLAGS=-D warnings cargo doc --locked --offline -p agq-kernel -p agq-kerml-semantics -p agq-sysml-semantics --no-deps` |
| C20 | `cargo test --locked --offline -p agq-kerml-semantics --lib mixed_declared_and_current_fact_reads_remain_current_before_first_append -- --nocapture` |
| C21 | `cargo clippy --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics --all-targets -- -D warnings` |
| C22 | `cargo test --locked --offline -p agq-sysml-semantics --lib unnamed_constraint_and_connection_usages_close_under_occurrence_owner -- --nocapture` |
| C23 | `cargo test --locked --offline -p agq-kerml-semantics --lib owned_end_population_ignores_only_proven_non_end_writers -- --nocapture` |
| C24 | `cargo test --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics --no-fail-fast` |
| C25 | `cargo test --locked --offline -p agq-sysml-semantics --lib input_action_body_closes_under_while_loop_with_nested_actions -- --nocapture` |
| C26 | `cargo test --locked --offline -p agq-sysml-semantics --lib` |
| C27 | `cargo test --locked --offline -p agq-kerml-semantics positioned_features_keep_exclusion_guards_without_nonmember_creation_proofs -- --nocapture` |
| C28 | `cargo test --locked --offline -p agq-kerml-semantics --lib` |
| C29 | `cargo test --locked --offline -p agq-kerml-semantics --lib positioned_guard -- --nocapture` |
| C30 | `cargo test --locked --offline -p agq-kerml-semantics --lib specialization_witness -- --nocapture` |
| C31 | `cargo test --locked --offline -p agq-sysml-semantics --lib proposal -- --nocapture` |
| C32 | `git diff --check` |
| C33 | `cargo test -p agq-sysml-semantics may_time_vary_positive_owner_uses_only_selected_occurrence_path -- --nocapture` before correction |
| C34 | `cargo test -p agq-sysml-semantics may_time_vary_ -- --nocapture` after correction |
| C35 | `cargo test -p agq-sysml-semantics input_action_body_closes_under_while_loop_with_nested_actions -- --nocapture` |
| C36 | `cargo clippy -p agq-sysml-semantics --all-targets -- -D warnings` |
| C37 | `cargo test --locked --offline -p agq-sysml-semantics --lib may_time -- --nocapture` |
| C38 | `cargo test --locked --offline -p agq-sysml-semantics --lib unnamed_assertion_body_closes_with_inherited_result_and_expression_binding -- --nocapture` |
| C39 | `cargo test --locked --offline -p agq-kerml-semantics --lib publication_overlay::restoration::tests -- --nocapture` |
| C40 | `cargo test -p agq-kernel --test reference_contributions` |
| C41 | `cargo test -p agq-kernel` |
| C42 | `cargo clippy -p agq-kernel --all-targets -- -D warnings` |
| C43 | `cargo doc -p agq-kernel --no-deps` with `RUSTDOCFLAGS=-D warnings` |
| C44 | `cargo test -p agq-sysml-semantics unnamed_assertion_body_closes_with_inherited_result_and_expression_binding -- --nocapture` |
| C45 | `cargo test -p agq-sysml-semantics trigger_receiver_node_parameter_closes_with_nested_feature_value -- --nocapture` |
| C46 | `cargo test -p agq-kerml-semantics --lib selected_append_and_whole_slot -- --nocapture` |
| C47 | `cargo test -p agq-kerml-semantics --lib producer_ -- --nocapture` |
| C48 | `cargo test -p agq-kerml-semantics` |
| C49 | `cargo clippy -p agq-kerml-semantics -p agq-sysml-semantics --all-targets -- -D warnings` |
| C50 | `cargo test -p agq-sysml-semantics --lib transition_tests -- --nocapture` |
| C51 | `cargo test --locked --offline -p agq-kerml-semantics --test cross_features_v10 produced_value_infrastructure_does_not_become_an_owned_cross_feature -- --nocapture` |
| C52 | `target/foundation-evidence/debug/examples/sysml_diagnostic_sources.exe <diagnosis-subjects.json>` |

| Command | Original record / tested source where stated | Observed result | Exit |
| --- | --- | --- | --- |
| C1 | closure | Initial compile identified missing private dependency field in accepted-query constructor; corrected before tests | 1 |
| C2 | closure | Test binary compiled | 0 |
| C3 | closure | No output | 0 |
| C4 | closure | 47 passed, 0 failed; 10.73 seconds | 0 |
| C5 | closure | KerML unit/integration suite passed; SysML49 passed/5 failed: legacy synthetic immutable fixtures lacked actual dependency closure authority | 101 |
| C6 | closure | All3 adversarial provider regressions passed after generic fix | 0 |
| C4 | closure | 53 passed, 0 failed; 10.23 seconds. Compact 60,000-subject proof: 2,220,216 bytes, 6,082 ms issuance | 0 |
| C7 | closure | Finished, no warnings | 0 |
| C8 | closure | Finished, no warnings | 0 |
| C9 | closure | Both public semantic APIs documented, no warnings | 0 |
| C10 | closure | Initial adaptation caught Arc type/error conversion/receiver lifetime mistakes; corrected locally; focused test then passed | 101, then 0 |
| C11 | closure | 60 passed, 0 failed, 33.15 seconds; doc tests passed | 0 |
| C12 | closure | No output | 0 |
| C13 | closure | 10 passed, including both adversarial regressions | 0 |
| C5 | closure | First run: 144 KerML unit tests passed, 2 ignored, 1 old identity assertion failed because producer opt-in now changes the digest domain. Assertion now checks unchanged graph pointer and distinct optional digest. Package rerun recorded below after completion | 101 |
| C8 | closure | Finished, no warnings after witness fixture correction | 0 |
| C14 | closure | 3 passed; initial test setup omitted required membership visibility, corrected; mixed-read/reconstruction/provider tests pass | 101, then 0 |
| C15 | closure | 1 passed; derived adoption excluded, original source edit exposes ambiguity | 0 |
| C16 | closure | Kernel and KerML suites passed; SysML60 passed/1 failed because old test expected broad NamespaceMembers instead of precise declared-source evidence | 101 |
| C11 | closure | After updating that evidence assertion, 61 passed/0 failed, 33.16 seconds; doc tests passed | 0 |
| C17 | closure | Version5 context: 3 passed | 0 |
| C18 | closure | Finished, no warnings | 0 |
| C19 | closure | Three crates documented without warnings | 0 |
| C10 | closure | 1 passed, 18.89 seconds | 0 |
| C11 | closure | Initial layering adaptation lost the fixture ownership append map: 59 passed/3 failed. Restored original ownership before adding fixture relationships; final 62 passed/0 failed, 45.05 seconds; doc tests passed | 101, then 0 |
| C5 | closure at 7120fb7 | KerML 145 unit tests and all integration suites passed, 2 scale probes ignored; SysML 62 passed in 48.67 seconds; both doc-test suites passed. Includes exact optional publication receipt restoration and original-slot dependent archive restoration | 0 |
| C20 | closure at 7120fb7 | Both merge orders and ordinary/producer evaluation modes passed; persistent current-property evidence asserted | 0 |
| C21 | closure at 7120fb7 | Finished, no warnings | 0 |
| C9 | closure at 7120fb7 | Both public semantic APIs documented without warnings | 0 |
| C22 | closure at 7120fb7 | Original regression incomplete; inverse correction alone cleared both constraint subjects but left the Connection end-population cycle. With both corrections, Complete and 1 passed in 19.39 seconds | 101, then 0 |
| C4 | closure at 7120fb7 | 61 passed in 10.82 seconds, including selected inverse edge evidence, derived/broad fallback, pending owner providers, and end writer regressions; 60,000-subject proof remains 2,220,216 bytes | 0 |
| C23 | closure at 7120fb7 | Passed after adding explicit inherited-end exclusion and pending namespace source assertion | 0 |
| C24 | closure at 522b17c | KerML 147 unit tests and all integration suites passed, 2 scale probes ignored; SysML 63 passed in 47.81 seconds; both doc-test suites passed | 0 |
| C25 | closure at 522b17c | Local Systems version reproduced Incomplete; both corrections produce Complete, 1 passed in 31.67 seconds | 101, then 0 |
| C26 | closure at 522b17c | 66 passed in 50.70 seconds, including the loop and both suppressed-proposal antecedent regressions | 0 |
| C27 | closure at 522b17c | 1 passed; selected/derived proofs, excluded removal, scalar activation, carrier retarget, checkpoint reopening and mixed broad reads covered | 0 |
| C28 | closure at 522b17c | 155 passed; the sole failure used an abstract Relationship in the new review fixture, corrected to concrete FeatureTyping | 101 |
| C29 | closure at 522b17c | Corrected fixture: 3 passed, including carrier reconstruction and failed/uncomputed aliased scalar states | 0 |
| C24 | closure at e3e92e0 / 6a31278 | KerML 156 unit tests plus all integration suites passed, 2 scale probes ignored; SysML 66 passed; both doc-test suites passed | 0 |
| C21 | closure at e3e92e0 / 6a31278 | Passed after removing two redundant conversions in the new helper tests | 0 |
| C30 | positive-population-review | 5 passed, 0.37 s | 0 |
| C4 | positive-population-review | 62 passed, 14.04 s; 60,000-subject certificate 2,220,216 bytes | 0 |
| C29 | positive-population-review | 3 passed, 0.51 s | 0 |
| C31 | positive-population-review | 2 passed, 0.50 s | 0 |
| C25 | positive-population-review | 1 passed, Complete, 50.11 s | 0 |
| C32 | positive-population-review | No output | 0 |
| C33 | may-time-vary-positive-owner | exit 101; unrelated ancestor proof assertion failed | included in result |
| C34 | may-time-vary-positive-owner | exit 0; 3 passed | included in result |
| C35 | may-time-vary-positive-owner | exit 0; 1 passed, 32.59 s | included in result |
| C36 | may-time-vary-positive-owner | exit 0 | included in result |
| C12 | may-time-vary-positive-owner | exit 0 | included in result |
| C32 | may-time-vary-positive-owner | exit 0 | included in result |
| C37 | may-time-vary-positive-owner | 4 passed, 1.34 s | 0 |
| C30 | may-time-vary-positive-owner | 5 passed, 0.29 s; selected derived proof, changed/deleted edge reopening, implied filtering, conjugation/provider and chained-path cases | 0 |
| C25 | may-time-vary-positive-owner | Complete, 1 passed, 34.28 s; two local parameters plus inherited third retain exact IDs, varying body and shared snapshot | 0 |
| C4 | assertion-body-regression | 64 passed, 11.76 s, including exact actual relationship emission sets for Expression/Function under Published/v9 and existing/fresh effect enforcement | 0 |
| C38 | assertion-body-regression | Complete, 1 passed, 19.27 s | 0 |
| C39 | assertion-body-regression | All 5 accepted-restoration guards passed, 2.31 s | 0 |
| C40 | ordered-reference-contributions | 7 passed | 0 |
| C41 | ordered-reference-contributions | 119 package tests and 3 doc tests passed | 0 |
| C42 | ordered-reference-contributions | clean | 0 |
| C43 | ordered-reference-contributions | clean | 0 |
| C12 | ordered-reference-contributions | clean | 0 |
| C44 | 6bda603; assertion baseline | AGQ_PRODUCER_CAUSAL_TRACE=1; Incomplete, five pairs; 16.06 s | 101 |
| C45 | 2a3d9cb; receiver baseline | Incomplete, eight pairs; 18.92 s | 101 |
| C46 | selected-append baseline; precise source unrecorded | Selected-only answer wrongly contains sibling support; one failure, 0.16 s | 101 |
| C47 | valuation split 0f05107; precise tested tree unrecorded | 97 passed, 2 ignored; 31.60 s | 0 |
| C48 | valuation split follow-up; precise tested tree unrecorded | 327 passed, 2 ignored; doc tests passed | 0 |
| C49 | valuation split; precise tested tree unrecorded | No warnings; 9.55 s | 0 |
| C50 | transition correction 4425065; precise tested tree unrecorded | 6 passed; 0.67 s | 0 |
| C36 | transition correction; precise tested tree unrecorded | No warnings | 0 |
| C51 | 522b17c correction | 1 passed; 0.34 s; shared declared-origin allocation restored | 0 |
| C35 | variable-body follow-up; precise source unrecorded | 1 passed; 34.00 s; true mayTimeVary/shared snapshot asserted | 0 |
| C52 | selected-population read-only diagnosis; input path originally abbreviated | All 185 subjects mapped from 245 diagnostics; no producer replay | 0 |


Additional historical and follow-up observations; the final three rows use
corrected production `fa3bbdd` + `4fa87cd`, fixture `dc888fd`.
Source/patch/output hashes remain in the JSON ledger where available; otherwise
they are unavailable, not reconstructed:

| Exact observed command | Result | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-sysml-semantics --lib transition_tests` | Independent review `8af0d6f`: 6 passed; exact source/hash in its review JSON | 0 |
| `cargo test -p agq-kerml-semantics --lib initial_derived_ordered_entries -- --nocapture` | Pre-fix initial helper reproduction: ordered targets and typed search correct, but unwanted whole ownership Property read retained; 1 failure, 0.16 s | 101 |
| `cargo test -p agq-kerml-semantics --lib selected_contribution_merge -- --nocapture` | 2 passed; both evidence modes/merge orders, initial typed population retained; 0.17 s | 0 |
| `cargo test -p agq-kerml-semantics --lib selected_contribution_tests -- --nocapture` | 3 passed; changed guards reopen, missing archive cache uses aggregate proof; 0.28 s | 0 |
| `cargo test -p agq-sysml-semantics` | 73 passed, 71.38 s; docs passed | 0 |

</details>
