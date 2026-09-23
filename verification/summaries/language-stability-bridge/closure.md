# Producer closure integration and regressions

Current status: Systems publication and language readiness remain unaccepted.
The latest completed corpus gate, `actions-owned-cross-subsetting`, has **534/534 Actions
references Complete**, zero reference failures and zero kernel obligations.
Producer closure converges Complete with zero diagnostics; the certificate closes
all 11,553 applicable pairs and every required closure. The scoped gate passes.
Details and exact counts are in the [current summary](README.md) and
`publication-commands.json`. Medium/full publication has not yet run.

The matching Flows and Items regressions now pass after `efff33c` scopes
`owned_cross_subsetting` to its typed relationship population. Candidate order,
subtype eligibility and the pending-specialization guard are unchanged. The
actual-planner red demonstrates that a VariableFeaturing snapshot previously
reopened this query through an irrelevant whole-owned-relationship read.
Eligible direct/future CrossSubsetting writers, unknown classes, pending
providers and independent broad reads still prevent closure; excluded
Redefinition/FeatureMembership writers do not.

Real pinned-source frontend checks distinguish Flows' plain cross Feature with
`[1]` from Items' named ReferenceUsage cross with `[0..*]` and two redefinitions.
Both genuine closed-dependency fixtures now have fully closed graph certificates,
Complete mayTimeVary and true isVariable. Flows' two-test module passes in 33.54 s;
Items' fixture passes in 33.37 s. The actual-planner controls, typed-population
guards, 15 existing cross-profile tests, three-package all-target Clippy and
formatting pass. Initial compile/fixture/format failures and the actual semantic
red remain recorded. Exact commands and verified raw hashes are consolidated
from `flows-end-commands.json` (11 records) and `items-end-causality.json`
(10 records) in `commands.json`. These are focused gates, not corpus acceptance.

The following observations retain earlier integration history; current corpus
status is the summary above.

`c8fb238` makes the temporary causal reverse index borrow its immutable
`ProducerRead` rows instead of cloning each enum and exclusion vector. Persistent
evidence, traversal/order, effects and blocked-set semantics are unchanged;
independent review confirmed the references remain valid for the whole call.
Existing producer regressions pass **149 tests**, with two preexisting explicit
scale probes ignored, including scheduler/full-scan equivalence. Both additional
typed cross-subsetting population guards pass. Memory savings are unmeasured;
the successful `actions-owned-cross-subsetting` executable predates this optimization.

The reproduced defect was historical proof-search contamination across an
authenticated dependency boundary. A closed dependency's derived parameter proof
retained an incoming/model search; importing it into the project made an unrelated
expression producer reopen positional redefinition. `f73f8ec` seals only exact,
unchanged Element/Property/AssociationOccurrence facts from an authenticated
dependency. Canonical proof roots and declared origins remain available, with
historical evidence interpreted in the dependency's context. Explicit current
inverse/source searches remain live. Untrusted immutable graphs confer no seal.
The regression matrix (`54b10be`) covers both evaluator modes, derived records and
selected appends, archive fallback, changed inverse projections and both merge
orders. Independent review permits the corrected Actions audit; no corpus result
is inferred from these focused checks. Accepted KerML bytes and identity are unchanged.
The producer sweep passes 113 tests with two existing ignores. The full KerML
semantic package passes 352 tests (two existing ignores), including all five
trusted-restoration guards and doc tests, in 174.00 seconds. Two-package
all-target Clippy and formatting pass after test-only Copy cleanup `3a3095d`.
Exact commands, the red reproduction and the initial lint failure are retained in
`commands.json` under the `positional-broad-reads-review.json` source-ledger range.

Broad-read diagnosis uses `AGQ_PRODUCER_READ_TRACE=1`, an explicit
`AGQ_PRODUCER_CAUSAL_SUBJECTS` list and optional `AGQ_PRODUCER_READ_FAMILIES` names.
At most 16 records per evaluation identify direct language reads, shared/expanded
kernel searches and missing read-row fallbacks. Attribution scans at most 4096
retained facts per search and labels unknown origins. The disabled hook changes
no evidence or acceptance. Two-package all-target compilation, Clippy and fmt pass;
records are in the `producer-read-trace-review.json` range of `commands.json`.

The pre-regression integration sweep on clean `a80cd1b` passes all 76 SysML
semantic tests and 93 text frontend tests (two explicit accepted-cache ignores),
including doc tests. Exact commands are consolidated from
`sealed-boundary-integration-commands.json`; these earlier results do not supersede
the subsequently added red expression fixture. The compiled inverse inventory
finds only two scalar inverse-storage properties in Published/v9, both already
using ownership-bounded source searches. Generalizing every inverse query to
source relationships would be unsound for remote-target carriers; no such change
was made. The inventory test and fmt pass (`inverse-read-review.json` range).

The `actions-read-origins` diagnostic was deliberately stopped after 195.766
seconds (requested exit 124, 4,554.6 MiB peak private memory), without a final
semantic result. Existing `closure future cause` records identify the remaining
global edge: pending subject creators activate hypothetical future owner writers
against unrelated read targets. This is now covered by a focused regression;
no medium/full attempt was consumed. The latest completed corpus counts above
remain authoritative until a corrected scoped run completes.

An actual `KerML.Invocation` descriptor left a nested argument expression's
result typing open solely because the enclosing invocation was pending. Its
planner instead writes the invocation and, for non-Function targets, its own
direct result. The test-first reproduction fails on `83ca429`; Function and
non-Function planner controls pass.

`SubjectAndOwnedResults` is an appended generic scope selecting the subject and
direct ReturnParameterMembership-owned Features. Masks, causal readers and effect
audits share the selector. Unknown endpoints and potential ownership/reference
writes remain conservative; reconstruction recomputes the selection. Existing
scope discriminants and accepted KerML publication bytes remain unchanged.

The narrowed scope also exposed an existing missing FeatureChainExpression
Membership effect: creation of its source-target member under an existing input
had accidentally borrowed Invocation's former broad permission. The descriptor
now declares that actual effect. The corresponding audit control rejects its
removal; producer output is unchanged.

Five scope tests, the actual feature-chain effect audit, and package library/test
Clippy pass. The producer/context regression sweep passes 119 tests, with two
explicit publication-scale probes ignored (103.60 seconds test runtime).
Exact commands, earlier failing controls and output hashes are in
the `invocation-scope-commands.json` source-ledger range of `commands.json`. The combined language expression gate and corpus
acceptance remain separate checks.

The audited fresh-ownership contract is integrated in registry `/5`. New
records retain plan-local emitter claims independently of their canonical identity
keys; every claimant must satisfy creation, scope and existing-effect permissions.
Both attachment contributions and ownership slots on fresh parents require an
Ownership capability when adopting an existing child. Unknown creation contracts,
remote writers and pending ownership providers retain conservative global bounds.
The future-owner proof follows only the permitted attachment ancestry when these
contracts are established. Focused independent review permits this scope boundary.
On `577492f`, the producer sweep passes 120 tests (two existing ignores), and the
full KerML semantic package passes 368 tests across 36 suites (two existing ignores,
142.67 seconds), including all five trusted-restoration guards. These checks
precede the separate instantiation read correction below. Exact failing and final
passing records are retained in the two `future-scope-review` source-ledger ranges.

The pinned KerML 1.0 `InstantiationExpression::instantiatedType` operation
(`standards/normative/kerml-1.0/KerML.xmi`, lines 496–515) rejects
FeatureMemberships before selecting the first remaining owned Membership.
The query previously selected that same target but retained the entire
Membership population as a producer dependency. A pending writer of only
FeatureMemberships therefore reopened an otherwise complete Invocation row.

The query now uses the existing `owned_relationships_excluding` contract.
It preserves canonical ownership order, accepts eligible declared and derived
memberships, and retains the selected member endpoint evidence. It does not
certify absence from an incomplete source or change operator target lookup.

The targeted red regression used the actual Invocation descriptor and a pending
subject-scoped FeatureMembership writer. It returned the correct Function but
left Invocation Pending in both query evaluator modes. The precise query makes
that row Complete; unknown or qualifying Membership writers and explicit broad
reads merged in either order still reopen it. Additional controls exercise
selected target reconstruction and derived eligible memberships.

Exact commands, exits, source identities and output hashes are in
the `instantiation-population-commands.json` source-ledger range of `commands.json`. The initial additional-control build
failed because the test used nonexistent convenience methods; the corrected
fixture uses the ordinary context and materialization APIs. No standard
publication/cache was loaded and no historical accepted KerML identity changed.

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
is `/2`, the producer registry is `/5`, SysML queries are `/5`, and historical
accepted KerML `/26` interpretation and receipt identities remain unchanged.

Full-publication preflight found that scheduler `Complete` did not independently
require all certificate masks closed. The reusable `is_fully_closed` check now
guards live publication, trusted Systems restoration, scoped audit success and
closed-dependency mounting. A focused regression has all three applicable
evaluations Complete while an unresolved FeatureTyping provider leaves typing
open; acceptance rejects that certificate, accepts the linked control and rejects
missing subject coverage. The report includes required/closed requirement counts.
Two coverage tests, four dependency tests, example Clippy and formatting pass;
exact commands and hashes are in
[commands.json](commands.json). This changes no
producer semantics, accepted identity or cache bytes and performs no replay.
Read-only preflight also verified normal preparation reaches strict publication,
accepted reference counts come from its independent audit, all 69 role bindings
are source-checked, and cache/receipt/binding outputs are flushed with the local
Systems graph retaining the shared KerML dependency. No publication was run by
this preflight.

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
| Positive SelfLink/HappensLink/composite Action exclusion imported an unrelated ancestor's mayTimeVary proof | Select a canonical positive specialization witness while retaining owner, scalar and role antecedents. No witness preserves the exhaustive ancestry and negative closure requirements | Reproduction `afb092a` failed for Action and SelfLink; correction `039a035` / root `f15cfca` passes five focused tests, receiver Complete in 27.18 s, all-target SysML Clippy and formatting. Actual corrected corpus: 532/534; trigger own evaluations Complete, transitive typing open |
| Nonterminal chain typing kept the chain's EffectiveTyping open despite a closed terminal | Typing follows only the canonical last FeatureChaining endpoint. Every endpoint must be established; future chain, ownership and reference-scalar writers remain blockers. Other requirement masks retain all components because featuring uses the head | Reproduction `eff127a`; correction `df11c96` / root `2be4fc9`. Six chain guards and 103 producer tests pass (two existing ignores), as do five accepted-KerML restoration guards, two-package all-target Clippy and formatting. Independent semantic review permits the same bounded Actions audit; actual `b4ce4e2` Actions result remains 532/534, 871.047 seconds, 5,209.3 MiB; initial trigger typing mask remains blocked |
| Transition payload's transitive descendant write scope reopened the trigger and its nested parameters, although the actual rule writes only the transition's second directly owned input | Generic `OwnedParameterFeatures` bounds existing writes to direct directed non-result FeatureMembership children. Closure masks, causal reader propagation and output auditing share that bound; unknown inputs remain open, existing direction/ownership/reference writers widen it, and reconstruction recomputes it | Actual planner/descriptor counterexample `8f8e89f` exits 101; `9cd8f80` passes seven transition tests, six scope guards, aliased unknown/failed direction coverage and 109 producer tests (two existing ignores). Full SysML package: 76 passed in 50.44 s; five KerML restoration guards, two-package all-target Clippy and formatting pass. Independent review GO; no corpus acceptance claim |

The contribution cache is optional, read-only kernel evidence. Local archives omit
it without changing canonical bytes; a missing entry requires aggregate query
fallback and reopens transported selected-support reads. Exact supplied immutable
dependency entries remain available. Population searches and recursively retained
positive/negative premises are still required; the cache is not closure authority.

The final chain-correction trace finds Trigger's initial typing mask blocked,
with no pending/incomplete local typing family and no transitive blocker path.
The ancestor's descendant scope subsequently reproduced a separate false cycle
and is narrowed by `bd096ec`; its corpus result is pending. The chain fix
corrects its own proven generic false cycle without closing the earlier corpus.

The chain correction preserves the EffectiveTyping contract, registry and archive/
certificate formats; corrected closure bits change the normal certificate digest.
Specialization and conjugation propagation remain unchanged. Its exact command
records are consolidated from `c38b203` into `commands.json`, including fixture
compilation corrections and the superseded zero-test restoration filter.

The parameter scope is appended without changing existing scope discriminants.
Only the SysML transition descriptor changes its existing-subject capability;
the combined registry digest changes normally. Historical KerML descriptor
identities, query meanings and archive/certificate formats remain unchanged.
The scope deliberately includes all direct directed parameters, not just the
second input. Fresh-only direction effects do not reclassify an existing trigger;
unknown local membership cannot acquire a protected dependency element. Exact
focused commands, including the initial missing-endpoint fixture's unnecessary
obligation assertion and its correction, are retained in the transition scope ledger.

The bounded residual review of the 532/534 audit (`fc04dfa`) found 105 incomplete
mayTimeVary, 11 value-binding and two positional evaluations among diagnostic
subjects. Both positional subjects are TriggerAction's payload
`70506936-f8f9-5057-9e36-37230a4e02df` and receiver
`a1ff5677-154c-54fa-b0b4-78c9a54a3c73`. Value-context findings span Actions,
Flows, Items and Parts. The final filtered direct causal edge is TriggerAction's
mayTimeVary to its VariableFeaturing. The future-writer trace's
`042923d6-97a3-5b93-9d20-b4c1217ad9d9` is Items' `edge` expression at bytes
2767..2771; it labels the first queued creator, not a uniquely established cause.
The report exposed only 118 of 164 incomplete evaluations, so it cannot establish
that every residual finding shares that root. No separate defect was reproduced.
The next report includes every incomplete evaluation's subject and source locator.

Independent review retained the composite Action guard, complete antecedents,
selected proof and negative fallback. In the isolated closure checkout,
`cargo test -p agq-sysml-semantics may_time_vary -- --nocapture` exited 0 with
five tests, using disabled incremental/debug output and one build job. The existing
`sysml_diagnostic_sources.exe` locator with generated diagnostic input also exited
0 without cache restoration or producer replay. Its precise generated argument
and output hashes were not recorded; raw inputs/output remain under ignored
`verification/generated/residual-population/`. These observations do not accept
the corpus or substitute for the exact focused-run records in `commands.json`.

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
  and an unnamed NodeParameter with a bound FeatureReferenceExpression
  and canonical result. Baseline `2a3d9cb` failed with eight incomplete pairs and no
  arity/invalid/missing-input finding (18.92 s). The trace linked FeatureValue(60015)
  through generated contextual ownership to VariableFeaturing. Initial-reference
  support now closes this fixture. A later source-only construction check found
  that generic ParameterMembership lowering assigns input direction to both the
  local payload and receiver even though their syntax has no direction keyword.
  The fixture now preserves those defaults and verifies both positional
  redefinitions. Its action definition overrides only the payload with an unnamed
  ReferenceUsage and inherits the receiver from a two-parameter Behavior, matching
  the partial parameter override of AcceptMessageAction; it remains Complete.
  A pinned Actions lowering regression checks
  the exact trigger/child classes, composite and portion flags, input directions,
  membership owners and source bytes without restoring or replaying standards.
  This correction does not reproduce the actual corpus's two pending positional
  evaluations and does not establish publication acceptance.
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

An auxiliary accepted-ancestry diagnostic projection (`2b83b1c`, retained on its
agent branch) compiled, parsed and passed example Clippy. Its single debug cache
restoration was deliberately canceled to free the host for the corrected Actions
audit: requested exit 124, 177.437 seconds, 2,743.0 MiB peak private memory. No
authored construction/query verdict was produced. The unused experimental harness
is not integrated; all three actual command records are retained in the
`accepted-trigger-probe-commands.json` source-ledger range of `commands.json`.

The final `actions-sealed-proof` audit resolves all 534 scoped mandatory references,
but its remaining 103 incomplete producer pairs do not establish publication
acceptance. Source-only parsing maps 42 residual subjects to `EmptyFeature`, which
the SysML frontend lowers to **ReferenceUsage**, not plain KerML Feature. In
Items.sysml:86, `00719337` is the direct return of `contains(...)`; `017049d9` is
the return of nested FeatureReferenceExpression `inter` in
`inter.intersectionsOf`. Both are owned through ReturnParameterMembership with
output direction. The four FeatureValue diagnostics concern Actions.sysml:63
`(that as Action).this`, :220 `aState.aTransition.accepter.acceptedMessage`, :518
`seq#(index)`, and Items.sysml:103 `isEmpty(voids)`; all require complete featuring
domains. This mapping does not prove one common cause for the other concrete
end/occurrence usages.

Test-only `0338c59` (base `83ca429`) models `isSolid = isEmpty(voids)` over genuinely
closed synthetic dependency layers. Both variants remain red: plain Feature
results leave three incomplete pairs; frontend-equivalent ReferenceUsage results
leave five. The latter exposes direct invocation result typing and nested result
snapshot failures. Its causal trace shows Invocation reading nested expression/
result populations and mayTimeVary depending on the same subject's
VariableFeaturing through the derived mayTimeVary property. Descriptor/proof
corrections require a separate passing verification; no production code or
accepted cache was changed by this diagnosis.

| Exact observed command | Result | Exit |
| --- | --- | --- |
| `target/foundation-evidence/debug/examples/sysml_diagnostic_sources.exe verification/generated/language-stability-bridge/actions-sealed-proof/diagnostic-subjects.json` | Existing parser-only binary; 103 exact source locators; 9.80 s | 0 |
| `cargo test -p agq-sysml-semantics --lib expression_usage_tests -- --nocapture` | Test implementation in `0338c59`, before formatting; 2 failed, 15.87 s | 101 |
| `cargo test -p agq-sysml-semantics --lib tests::producer_tests::expression_usage_tests::invocation_value_with_frontend_reference_usage_results_closes -- --exact --nocapture` | Same test source, `AGQ_PRODUCER_CAUSAL_TRACE=1`; 1 failed, 15.93 s | 101 |
| `cargo fmt --all -- --check` and `git diff --check` | `0338c59`; no output | 0 |

Raw ignored output SHA-256 values: source locators
`a7c9a19866a075d08f78382242f5f0c619207e1398ef5cf3c7af1c799c338cfd`;
paired test `a281692257c3990205f9aa276fec7915c51e386866277c731b73401acb4a968e`;
causal trace `1578eb8f10dcecbc63cf2aac3c7993590a9800d8da70827210c71923785eaebb`.
The first source adapter attempt contained a UTF-8 BOM and exited 1 before parsing;
the corrected input used UTF-8 without BOM. An initial unqualified `--exact` test
filter selected zero tests and establishes no result. The qualified rerun above
is the recorded regression. No graph cache, producer corpus, or publication run
was used for source mapping.

Combined source `2e1e346` includes the direct invocation-result scope and audited
future-owner bound. The same paired command above still exits 101: 2 failed in
17.93 s. Plain Feature results retain three incomplete pairs; ReferenceUsage
results improve from five to four. Nested feature-reference typing and its result
snapshot diagnostics disappear; AttributeUsage typing, direct invocation-result
typing and the outer FeatureValue featuring boundary remain incomplete. The
qualified frontend command above with `AGQ_PRODUCER_CAUSAL_TRACE=1` also exits 101:
1 failed in 16.38 s. Its trace retains a future edge from the nested
FeatureReferenceExpression producer to Invocation through the invocation's broad
Membership population, then direct Invocation edges to its result's value
producers. Function-target result-write precision and target-membership read
precision remain diagnostic hypotheses, not verified fixes. Output SHA-256:
paired run `2fabde69ab5378d9a42622e6b572866dbc72943c06a1b417c5118ad87bbf9ca5`;
trace `ac1aaaf984b23246c3488d674a7ac5e8117467a8acf027bb58caf45e6d897f7b`.

Independent review of the future-owner bound is **GO** with guard follow-up
`f6362bc` (integrated as `30d7413`). Fresh creation requires declared capability;
all actual output emitters must satisfy their own scopes; ownership adoption is
checked both in attachment maps and fresh-owner slots. Existing semantic source
and canonical owner are audited separately, so a relationship owned by A cannot
use another batch subject B's typing permission. Unknown/transitive creation,
model-wide/reference-scalar writers and pending ownership providers retain the
global fallback. Reconstruction recomputes masks against the current graph and
registry. No further defect was found within these reviewed boundaries.

Architecture's recorded commands `fresh-ownership-guards-concrete-carrier`
(`cargo test --locked --offline -p agq-kerml-semantics --lib fresh_ownership_tests -- --nocapture`)
and `emitter-shared-initial-value-control`
(`cargo test --locked --offline -p agq-kerml-semantics --lib emitter_scope_tests -- --nocapture`)
exited 0: four controls passed in 0.11 s and five in 0.32 s, respectively. Their
ledger records retain exact tested source/patch/output hashes. The reviewer did
not repeat those runs. This scope-soundness verdict does not resolve the separately
recorded expression micro failures or establish publication/readiness acceptance.

The paired expression regression is now **green** with precise non-feature
membership reads for `instantiated_type` (`53e0c6d`, regression `e0e68dc`, applied
over `e623a48`; local source `b104f85`). The fixture now also asserts
`certificate.is_fully_closed(closed.overlay.model())`, covering all local and
dependency subjects in addition to the existing per-subject masks and shared
dependency pointer. The exact paired command recorded above exited 0: two tests
passed in 19.27 s. Both plain Feature and frontend-equivalent ReferenceUsage
results converge Complete with fully closed certificates. No Function-specific
result-write scope change was needed to resolve this regression. Formatting and
diff checks passed. This is a bounded fixture result; actual Systems publication
and accepted-source gates remain separate.
Test output SHA-256:
`22564b963b3f5c6d4a57740a7b4a3f3867506c9561057757c7e743c36f739f79`.


A compound expression variant (`c840670`, based on `7613d06`) is **red**:
`contains(inter.intersectionsOf, face)` uses genuinely closed anchor layers, a
two-input Function signature, a FeatureChainExpression beneath the first
invocation argument, and frontend-equivalent ReferenceUsage results. It omits
the pinned Items expression's unrelated `union(...)` second argument to isolate
the nested chain. All dependencies close; the authored graph converges Incomplete
with three incomplete producer pairs. EffectiveTyping remains open for the
nested reference expression `73009` and chain result `73011`; result `73010`
cannot establish its variable-feature snapshot domain.

`cargo test --locked --offline -p agq-sysml-semantics --lib invocation_chain_value_with_frontend_reference_usage_results_closes -- --nocapture`
failed one test in 28.48 s (observed shell exit 1). Repeating with
`AGQ_PRODUCER_CAUSAL_TRACE=1` exited 101, one failure in 28.68 s (28.89 s wall).
Both used no incremental compilation, dev/test debug 0, two build jobs and the
isolated `target/bridge-self-model` target. The trace records direct propagation
from `73007/FeatureChainExpression` into the nested expression's owned/result
population and its result's positional redefinition; nested reference production
then contributes future owned/structural reads back to the chain. This matches
the independently reproduced overly broad chain descriptor scope; a correction
and green full-certificate run are still required. No accepted cache or corpus
publication was run. Rustfmt and diff checks exited 0. An earlier `--exact`
invocation used an unqualified filter and selected zero tests; its exit 0 is not
verification evidence.

The ignored local trace is `verification/generated/compound-expression/trace.log`,
SHA-256 `7e598cd5469c4e700f4fd1f6fbcad90d9a1a377072b825aea831e6b5b94affd6`.
The first failing console output was not retained as a file; its hash is unavailable.

The reviewed FeatureChainExpression scope now follows only transitive canonical
FeatureMembership containment. It includes the direct result and the first
input's source-target feature, but not expressions owned through FeatureValue.
The same selector bounds initial masks, causal reads, fresh-owner ancestry and
all effect-audit paths. Unknown endpoints and ownership/reference/provider
writers retain conservative bounds; reconstruction recomputes membership.
Actual planner controls cover present and newly created source-target features.
Five scope tests, 124 producer tests (two existing ignores), two-package
all-target Clippy, strict KerML Rustdoc and formatting pass. The faithful compound
fixture improves from three incomplete pairs to one; its direct chain result
still lacks EffectiveTyping closure, so another corpus run remains gated.
Exact red/green commands are consolidated from `feature-chain-footprint-review.json`.

The bounded future-reader traversal also avoids scanning unrelated reader keys.
It visits the same sorted bounded target set and uses the unchanged matching
logic; an unbounded creator still uses the complete reader map. Its nine-case
guard matrix and formatting pass (`future-read-traversal-review.json` range).
This optimization changes neither canonical semantics nor producer registry identity.


With the feature-containment scope correction `a634a96` (local `34930c2`),
`cargo test --locked --offline -p agq-sysml-semantics --lib expression_usage_tests -- --nocapture --test-threads=1`
exited 101: both simple controls passed, but the compound test still failed
(63.73 s tests, 75.02 s including compilation). It now has one incomplete pair,
`73011/deriveUsageMayTimeVary`; the nested reference expression and its result
close. The same low-disk environment was used. Captured output SHA-256:
`8ad63416b1164cf3811c41a121671c62d4c40bbdf1ad82a94d807ae03b6f3b76`.

A repeat of the exact compound command above with `AGQ_PRODUCER_CAUSAL_TRACE=1`
and `AGQ_PRODUCER_CAUSAL_SUBJECTS` selecting canonical IDs 73005–73015 exited 101
(32.71 s test, 38.04 s including compilation). A test-only diagnostic prints the
one EvaluatedIncomplete pair. The remaining chain-result VariableFeaturing
writer reaches the chain planner through `Structural(73007)` and broad
`Owned(73007, Membership)` reads, both directly and through potential future
helpers. EffectiveTyping for result `73011` remains open through its own
FeatureValuation/PositionalRedefinition and generated chain/source-target helper
writers. This is a residual dependency finding, not a passing compound gate.
The ignored `verification/generated/compound-expression/scoped-trace.log` has
SHA-256 `70d9229702e0cc9a460d28d6eb6dc1149c895f525ddf0f1a410627b7e46e90f8`;
the instrumented test file has SHA-256
`c4e490d2099b29c0e3ff226e9adf5949dd9ab48adce23cb9940ceb670f669543`.
Rustfmt and diff checks exited 0. No further publication or accepted-cache load
was performed for this diagnosis.

Independent review (`4d9c806`) also passes all five FeatureChain scope controls
and permits bounded integration. The planner footprint, alias/unknown-state
handling, pending provider guards, output attribution and reconstruction were
reviewed against the shared selector. It grants no corpus acceptance.


The directed-input correction `0c84fce` (regression `8cf95d8`, local source
`b5e3dd5`) preserves both simple controls but does not yet close the compound
graph. The same three-test command exited 101: two passed, one failed in 64.63 s
(75.51 s including compilation), output SHA-256
`0c9e0d5263697d48e0294d140529398106e24ce2c093fb9151841b2839f77a21`.
The same filtered compound trace command exited 101 in 32.79 s (33.01 s wall),
output SHA-256 `1432600d49e2c2c2d6f8e935dcd14bb0a4b00b35391960b4463f320a4d1f0fd4`.
It retains the single `73011/deriveUsageMayTimeVary` blocker and the broad
membership reads described above. Inspection identifies a separate broad read
in `reference_referent`, which selects the first non-parameter membership but
merges the whole membership population. Merely excluding FeatureMembership
would alter the existing selection semantics; any refinement must retain exact
ordering and unresolved/provider guards. No further run or nested-chain
extension was made before resolving this residual.

First-input population controls preserve exact `in` selection, return memberships,
canonical order, absent direction and pending/invalid inputs. The private Directed
projection composes the existing Parameter and Result search contracts; no public
population kind is added. Two focused tests, two-package all-target Clippy and fmt
pass (`first-input-population-review.json` source-ledger range).

The actual Index/Select planner writes only its own structural result and fresh
contextual helpers. Publication-producer profiles enforce that ownership before
emitting; older profiles retain their original scope. The descriptor now uses
SubjectAndOwnedResults. Its future-helper ancestry reuses the exact selected
roots as Feature containment does, retaining conservative fallback for unknown
endpoints, providers, ownership/reference writers and unscoped creators. Five
focused tests cover both expression types, the Array guard, inherited-result
staging, causal readers, effect audit and reconstruction. A descriptor-only
intermediate correction fixed initial masks but still failed the future-reader
control; the final shared roots correction closes both. The producer sweep
passes 129 tests (two existing scale ignores), all-target semantic Clippy and
formatting pass, and independent review permits bounded integration. Exact
commands and the initial fixture errors are retained in the
`index-select-scope-commands.json` source-ledger range. Canonical outputs, rule
IDs and accepted KerML bytes remain unchanged; the combined descriptor digest
changes. No corpus result follows from these focused checks.

The complete FeatureChain planner's retained-read control passes on both fresh
and materialized frontiers, with source-target present and absent (four cases,
0.64 seconds). An otherwise Complete chain row remains Complete with its direct
result's VariableFeaturing row pending. This confirms the chain queries together
no longer carry that false direct cycle; it does not establish the enclosing
SysML expression's closure. The latest full expression trace locates the
remaining path through the nested FeatureReferenceExpression's ancestor-context
reads. `bridge-actual-chain-planner` and `bridge-materialized-chain-planner` are
recorded in `commands.json`. A speculative release build was explicitly stopped
(exit 4294967295) when the combined fixture remained red; no corpus used it.


Nested-chain preparation `b7d3d99` adds
`contains(inter.intersectionsOf.otherFeature, face)` as a separate unexecuted
test. After importing the Index/Select result scope `4317e65` and referent-prefix
source `17f93d4`, clean local source `0922022` was tested with
`cargo test --locked --offline -p agq-sysml-semantics --lib expression_usage_tests -- --nocapture --test-threads=1 --skip invocation_nested_chain`.
It exited 101: both simple controls passed and the original compound case still
failed on `73011/deriveUsageMayTimeVary` (62.62 s tests, 73.69 s including build).
Output SHA-256 `6ba508197cec8af1b3f423f237b8a47640b58e1f31a64937a720c2eb18c69f13`.
The nested test remains unrun pending this base gate. An earlier runner started
before prerequisite cherry-pick conflicts were resolved; it was explicitly
terminated (shell exit 1) and excluded from these results. Its retained partial
output hash is `888e33ffcbf8d3acaa17753cfa68abe864753c5a283715b4241f5ddd18801f1c`.

The original-prefix referent correction has four adversarial controls covering
pending/derived-only populations, current endpoint evidence, archive restoration,
mixed broad reads and reconstruction. Independent review checks kernel append
and archive prefix invariants; the four controls and causal regression pass
independently. Two-package all-target Clippy and formatting pass. Exact commands,
including the first missing-import compile error, are consolidated from
`declared-referent-review.json` and `referent-prefix-independent-commands.json`.
These controls establish the prefix boundary, not whole-expression acceptance.

Bounded read-origin tracing now optionally covers Owned/Structural reads with
`AGQ_PRODUCER_READ_TRACE_OWNED=1` and an optional comma-separated
`AGQ_PRODUCER_READ_TARGETS` owner list. Existing subject/family filters and limits
of 16 emitted records and 4096 inspected facts remain. Shared and explicit reads
are labeled; absent attribution is not presented as a unique source. Disabled
tracing changes no semantic evidence. Test-target compilation and fmt pass
(`owned-read-origin-review.json` range); the hook has no publication effect.


Final-source diagnostics distinguish the remaining path from the earlier one.
On `3213b35`, the exact compound command (without `--exact`) above with
`AGQ_PRODUCER_CAUSAL_TRACE=1`, `AGQ_PRODUCER_READ_TRACE=1` and
`AGQ_PRODUCER_CAUSAL_SUBJECTS=00000000-0000-0000-0000-000000011d2f,00000000-0000-0000-0000-000000011d33`
exited 101 (33.63 s test, 33.86 s wall), output SHA-256
`e7074ff4bcb5cf7f552f3f9bf2bfb3425a62b47ccea48f0ce3ac6affc1d41e35`.
The direct chain-result VariableFeaturing→FeatureChainExpression edge is gone.
The surviving route is result `73011` VariableFeaturing→nested expression
`73009` FeatureReferenceExpression through ancestor owned/structural reads,
then `73009`→chain `73007` through input `73008` reads. The original read-origin
hook emits nothing here because it only covers Global/Inverse dependencies.

Diagnostic-only hook `6066ec1` (local `e1f7080`) was then used for one repeat of
that compound command with both trace flags above and these exact overrides:

```text
AGQ_PRODUCER_READ_TRACE_OWNED=1
AGQ_PRODUCER_CAUSAL_SUBJECTS=00000000-0000-0000-0000-000000011d31
AGQ_PRODUCER_READ_FAMILIES=KerML.FeatureReferenceExpression
AGQ_PRODUCER_READ_TARGETS=00000000-0000-0000-0000-000000011d2f
```

It exited 101 (32.09 s test, 42.71 s including compilation), output SHA-256
`3061e9f6c78749bc3e6a769bca4bf2042923e5471cef3b4ee93c4d223ef3742f`.
The trace establishes a **direct-language current-query**
`OwnedRelationships(73007, Membership)` read in `73009`'s producer. It also
records imported-shared `RelationshipStructure(73007)` searches attached to
facts `89ee892f-0787-5046-84f1-a8a16d0de7ca` and
`b2de7a45-bdd1-5a35-bc7e-943c6743a8dc`; direct and imported evidence coexist.
Inspection identifies the compatibility check's full `direct_features`
populations as a candidate live-query origin, pending a focused planner proof.
These are diagnostics, not passing gates; the nested case remains unrun. Both
ignored logs are under `verification/generated/compound-expression/` as
`final-source-trace.log` and `owned-origin-trace.log`.

## Positive owned-Feature witness for compatibility

The actual `FeatureReferenceExpression` and `VariableFeaturing` descriptors
reproduce a causal cycle: compatibility is already false because a Feature owns
another Feature, but its whole membership search imports a pending child's
snapshot writer. The regression failed with reader state `Pending` on
`0ddafbc` and passed with the selected witness in `cda94f4`.

The KerML `Feature::isCompatibleWith` specialization branch and its evidence are
unchanged. Only the subsequent conjunction requiring both owned-Feature
populations to be empty has a positive counterexample path. A canonical owned
FeatureMembership and its current unique Feature endpoint prove nonemptiness.
Original ownership keeps its declared-slot guard; derived ownership retains its
selected contribution and creation proof, or the full aggregate proof when
precise contribution metadata is unavailable. No witness implies no absence
claim and falls back to the existing exhaustive queries.

The causal, truth-table, and reconstruction controls pass: specialization still
wins, either operand can prove nonemptiness, empty/unknown cases preserve their
original behavior, and removing or retargeting the selected carrier reopens its
reader. Separate proof controls cover native/archived derived evidence and
independent broad reads in both merge orders and evaluator modes. Commands,
outputs and fixture setup corrections are retained in
the `compatible-witness-commands.json` source-ledger range in `commands.json`. All five focused tests passed (1.75 s),
package all-target Clippy passed with warnings denied, and workspace formatting
passed; each final command exited 0.

The full compound authored fixture remains separately gated: its first run after
this fix still reported one incomplete MayTimeVary pair. This bounded proof
correction does not establish Systems publication or readiness acceptance.

The positive nonempty-feature compatibility witness `cda94f4` (regression
`eace5c8`, local source `d705327`) removes the broad chain Membership read.
The base three-test command above still exited 101: two simple controls passed,
the compound certificate remained incomplete on `73011/deriveUsageMayTimeVary`
(61.80 s tests, 73.01 s wall). Output SHA-256:
`d99701fecbf281f78a6ec090890d08d3b0336f84b7764c0c8ffb3d0c0ab7e132`.
The same owned-origin trace command and overrides above, now on `d705327`,
exited 101 in 32.00 s (32.24 s wall), output SHA-256
`7253a4698be6346c8ea58fc547e0be36bb97f045a2def595da60495ea02a4d56`.
Neither the direct Membership read nor the imported RelationshipStructure read
on chain `73007` remains; direct TypeFeaturing reads still connect the pending
result's VariableFeaturing producer to the nested FeatureReferenceExpression.

A test-only bounded helper inspection repeats that exact compound command with
`AGQ_EXPRESSION_HELPER_ORIGINS=89ee892f-0787-5046-84f1-a8a16d0de7ca,b2de7a45-bdd1-5a35-bc7e-943c6743a8dc`
in addition to the same trace overrides. It exited 101 in 32.52 s (38.22 s wall),
output SHA-256 `cf35e4c3daf042671fc6981892567d46f0386d893bde77ea83df46aa54752a66`.
The test file over `d705327` had SHA-256
`159d27d240ee48f184142b54b8d7695eda6e531450cff131768ca0f48843c3a5`.
The first helper is FeatureTyping owned by `73007`, targeting standard anchor
`20040`, with 11 declared dependencies and rule
`checkInvocationExpressionSpecialization`. The second is OwningMembership under
`73009`, generated by `checkFeatureReferenceExpressionBindingConnector`, with
138 immediate dependencies (the diagnostic prints only the first 64).
Rule names were resolved against the frozen v9 rule identity formula; no
producer identity changed. These results isolate a remaining TypeFeaturing
writer-scope investigation, not a successful closure gate. Raw outputs are
`compatibility-combined.log`, `compatibility-owned-origin-trace.log`, and
`compatibility-helper-origins.log` in the same ignored diagnostic directory.
The nested-chain test remains unrun. Formatting and diff checks passed.

Per-effect scope source `7ff867f` and its actual-footprint regression `e672a90`
were tested on local `df741c8`; its three modified production files match the
source commit exactly, including the earlier bounded-reader traversal. The
same base three-test command exited 101: two simple controls passed and the
compound case retained `73011/deriveUsageMayTimeVary` (57.97 s tests, 69.37 s
wall). Output SHA-256:
`f7d7128d553d73574c4fe8d73d7a73fea2a6e7394849b6838ac267492fb67cc8`.
Closed producer pairs increased from 105 to 130; this is not full closure.

One subsequent exact compound trace used the same three enabled trace flags,
with both `AGQ_PRODUCER_CAUSAL_SUBJECTS` and `AGQ_PRODUCER_READ_TARGETS` set to
`00000000-0000-0000-0000-000000011d33,00000000-0000-0000-0000-000000011d31,00000000-0000-0000-0000-000000011d2f`,
and `AGQ_PRODUCER_READ_FAMILIES=deriveUsageMayTimeVary,KerML.FeatureReferenceExpression,KerML.FeatureChainExpression`.
It exited 101 (28.25 s tests, 28.46 s wall), output SHA-256
`f5137d681b2b60bc9e4900f80c9a7246b09879f405c0dfda5109a3f864baf972`.
The ancestor TypeFeaturing writer edge is gone. The remaining
`73011/VariableFeaturing -> 73009/FeatureReferenceExpression` path reads the
outer Container `73000`'s owned-relationship property, structural population,
and Membership population. The selected read-origin targets did not include
`73000`, so this run does not establish that population's import channel.
The raw files are `effect-scoped-combined.log` and `effect-scoped-trace.log`
under the same ignored directory. Nested-chain execution remains gated.

## Per-effect producer scopes and independent review

The actual VariableFeaturing planner emits TypeFeaturing only on its variable
subject and snapshot membership on its owning Type. A focused causal regression
exposed the former shared scope treating owner featuring as writable. Descriptors
now permit exact per-effect scope overrides; VariableFeaturing uses Subject for
Featuring while retaining the separately checked membership/creation envelope.
The map changes registry identity under schema `/6`. The actual planner and
causal controls pass; 135 producer tests pass with two existing scale ignores,
two-package all-target Clippy and strict KerML Rustdoc pass. Exact command
records are consolidated from `variable-effect-scope-review.json`.

Reviewed production `7ff867f` independently. Scoped result: GO, no production
finding. Five adversarial tests passed (0.77 s); package library/test Clippy
with warnings denied and workspace formatting also passed. Controls cover:

- Narrowing/widening scopes, including an effective `Model` override on a
  default `Subject` descriptor and retained conservative non-typing masks.
- Future subject activation with `Subject`, `SubjectAndOwners`, and `Model`
  effects; absent current instances cannot erase future effects.
- Registry identity changes, rejection of stale direct/certificate transport,
  and rejection of overrides for effects the descriptor never declared.
- Exact semantic relationship, scalar, and existing-child ownership auditing.
- A local incoming relationship reader of a protected dependency: a mixed
  reference-scalar capability still invalidates it despite a narrow Featuring
  override. Protected ownership collections remain immutable.

Source review confirms effect-specific routing in direct/future causal readers,
closure masks and semantic/scalar/ownership audits. Primitive/reference scalar
guards, unknown selected populations, provider ownership and immutable boundary
handling remain conservative. Fresh attachment auditing deliberately retains the
default creation envelope. The new map is included in registry schema `/6`;
canonical output keys and historical accepted KerML bytes are unchanged.

The first boundary fixture expected an evaluated producer on a protected subject;
such producers are correctly inapplicable. The next fixture read the protected
owned collection, which is correctly immutable even with external references.
The final control queries incoming FeatureTyping sources from a local reader,
exercising the intended open external-carrier boundary. These setup corrections
and all actual command results are retained in
the `per-effect-scope-independent-commands.json` source-ledger range in `commands.json`.

No package sweep, accepted cache load, or corpus run was performed for this
review. The separately reported compound fixture still has an incomplete pair;
this scoped GO does not establish publication or readiness acceptance.

The immediate owning-Type scope correction `56fde0d` (actual-footprint
regression `1eca5d6`, local source `363f7e7`) closes the compound fixture.
The same base three-test command exited 0: all three passed in 55.16 s
(67.59 s including compilation), output SHA-256
`6276a4c44ae38bdca3b682a83175f8b3cb92d121a07491771a55a61378c0efd8`.
Both frontend ReferenceUsage result shapes and the plain Feature control assert
the whole certificate is fully closed and retain their per-subject checks.
The relevant production source matches `56fde0d` exactly. This is a bounded
genuine-layered fixture result, not standard-library publication acceptance.

With that base gate green, the separately prepared nested fixture `b7d3d99`
was run for the first time using
`cargo test --locked --offline -p agq-sysml-semantics --lib invocation_nested_chain_with_frontend_reference_usage_results_closes -- --nocapture --test-threads=1`
on the same source. It exited 0: one passed in 23.64 s (23.86 s wall), output
SHA-256 `861d1fadf5df66b26a1b54d66f44d34b1a85d7f195bcc367ca08e8c33a1b6b4b`.
The `contains(inter.intersectionsOf.otherFeature, face)` shape closes the whole
certificate, including both chain results and every local requirement; each
expression uses the frontend's ReferenceUsage return shape. Raw outputs are
`immediate-owner-combined.log` and `immediate-owner-nested.log` in the same
ignored directory. Formatting and diff checks exited 0. No publication or
accepted-cache operation was performed.

## Immediate owning-Type scope and independent review

Production `56fde0d` independently reviewed: GO, no finding. The five new
controls passed (1.17 s), and the combined per-effect/immediate-owner control
set passed all ten tests (1.84 s). Library/test Clippy with warnings denied and
workspace formatting passed; every final command exited 0. Commands are recorded in
the `owning-type-scope-independent-commands.json` source-ledger range in `commands.json`.

The selector matches `owning_type`: Feature subject, its owning FeatureMembership,
then that membership's owning Type. Both canonical backing hops are checked.
It excludes lexical containers and non-FeatureMembership carriers. Tests cover
reparenting, detached carriers, truly absent canonical inverse navigation, and
NotComputed/Incomplete/Invalid inverse aliases. Unknown ownership, pending
namespace providers, Ownership effects and reference-scalar writers retain the
conservative fallback.

Future immediate-owner writes use exact possible attachment roots; custom
`SubjectAndOwners` contracts retain transitive ancestor reach. The independent
future-reader and certificate-mask assertions cover both. Semantic-effect and
fresh-attachment audits accept the subject/direct owner and reject ancestors
and unrelated targets. Canonical IDs, historical KerML publication bytes and
query evidence are unchanged.

This review ran no package sweep, standard-library cache load or corpus. Systems
publication and whole authored-fixture acceptance remain separate gates.

The final producer sweep after the equivalent static future-requirement
classification hoist `ff31820` passes 135 tests, with two existing scale probes
ignored (34.61 s). Two-package all-target Clippy, strict KerML Rustdoc and
formatting pass. The old transitive future-writer fixture now explicitly declares
its custom SubjectAndOwners contract (`b533e52`), preserving its original nine
adversarial expectations. Its initial failure and all final command results are
retained in the `variable-immediate-owner-review.json` source-ledger range.

## Corrected Actions audit: remaining end and indexed-assignment boundary

`actions-immediate-owner` uses source `fad6a95` and the successful release build
from `749f9c9`. It exits 1 normally after 1,327.469 s, peak private 6,160.2 MiB;
no watchdog stop. All eight pinned documents parse/construct byte-exactly,
534/534 mandatory references are Complete, kernel obligations and authority
conflicts are zero, and accepted KerML producers are not replayed.

Producer closure converges in 31 rounds but remains Incomplete and not fully
closed: 24 explicit incomplete pairs (21 mayTimeVary, two reference expressions,
one FeatureValue), 23 diagnostics, 10,815/11,439 closed applicable pairs and
382,442/407,028 closed requirements. It derives 3,341 local elements while
evaluating 19,917 subjects. Certificate/revalidation sizes are 2,628,939 and
712,671,848 bytes; two transport operations retain 1,225 and reopen 11,801
evaluations in aggregate. The report and raw watchdog evidence are ignored under
`verification/generated/language-stability-bridge/actions-immediate-owner/` and
`verification/generated/overnight-convergence/actions-immediate-owner/`.

Original KPAR document SHA-256 values were checked before mapping every reported
source range to `diagnostic-sources.jsonl`. Twenty MayTimeVary subjects are named
or anonymous end usages in Flows, Connections and Items. Their OwnedCrossing,
PositionalRedefinition and VariableFeaturing evaluations are Pending. The other
boundary is Actions line 518, `assign var := seq#(index);`: anonymous usage
`98c30374-e65a-579b-be50-0d23d18160d8` has incomplete MayTimeVary and FeatureValue;
its `seq` and `index` reference expressions remain incomplete. This is a final
scoped failure result, not acceptance. Medium/full publication and production
workspace integration remain gated pending concrete focused corrections.

## Eligible crossing membership population

The actual-planner regression `009428c` reproduces a false dependency from a
pending VariableFeaturing snapshot to OwnedCrossing and PositionalRedefinition.
Correction `e5e190a` queries the eligible OwningMembership population directly:
FeatureMembership is always excluded, and FeatureValue remains excluded only
in the corrected profiles. Selection, ordering and endpoint checks are unchanged.
No producer descriptor, registry identity or canonical output key changes.

The real combined SysML scheduler now closes two ConnectionDefinitions with
four ReferenceUsage/OccurrenceUsage ends, inherited positional and explicit
redefinition, variable snapshots and an immutable dependency. The entire
certificate and all per-end requirements close; alias-aware isVariable values
are true and inherited IDs remain original. This focused test passes in 14.45 s.
Direct/future eligible writers, unknown future classes and ownership mutation
remain open; excluded future FeatureMembership does not reopen crossing.
Fifteen crossing/profile integration tests, two-package all-target Clippy and
formatting pass. The ten actual attempts, including corrected test assertions,
are retained in the `variable-end-causality.json` range of `commands.json`.
These focused results do not supersede the latest failed Actions audit.

## Indexed assignment: directed valuation and argument populations

The permanent real-frontend test establishes the exact Actions line 518 shape:
a ReferenceUsage assignment input owns a noninitial FeatureValue and IndexExpression;
the Index result is a declared plain Feature, while both nested reference
expression results are ReferenceUsages. A genuine closed Action/Occurrence and
AssignmentAction dependency, including target(in)/replacementValues(inout),
reproduces all four remaining assignment producer failures in 22.19 s.

Correction `ee0fb76` closes the known-directed input's inapplicable valuation
without waiting on the indexed result. It retains canonical direction evidence;
unknown or failed aliases cannot certify nonapplicability. FeatureValue binding
keeps its required result/context reads. Independent review and four adversarial
groups pass, including all directions, both evaluator modes, pending writers,
checkpoint invalidation and unchanged Published versus v9 output targets.
The existing value-stratum test, Clippy and formatting also pass. Exact commands
are in the `directed-valuation-review.json` source-ledger range.

A separate valid ReferenceUsage Index-result control exposes an independent
argument-population overread. Correction `8286e2a` reads ParameterMembership
excluding ReturnParameterMembership, then FeatureValue, preserving first-eligible
order and endpoint evidence. Three guard tests retain eligible direct/future
writers, unknown providers and independently requested broad reads; selected
endpoint/ownership changes reopen the transported certificate.

On combined source `6dc335f`, all six expression fixtures pass in 132.26 s and the
connection-end fixture passes in 15.33 s. Both indexed cases assert true
mayTimeVary/isVariable, a fully closed certificate and the exact shared dependency.
Strict all-target Clippy for all three affected packages and formatting pass.
The `index-usage-commands.json` range retains every attempt and fixture correction;
raw output hashes were checked when consolidating. Root release source `905e4ee`
then builds successfully. The next Actions audit remains a separate required gate.

## End/index correction corpus result: four explicit-multiplicity ends remain

`actions-end-and-index` (clean source `0e3d7f5`, release `905e4ee`) finishes normally
with exit 1 in 1,409.547 s, peak private 6,124.8 MiB and no watchdog stop. All eight
original documents parse/construct, 534/534 references are Complete, selected
endpoints are 534, and kernel obligations/authority conflicts are zero. The
indexed assignment and sixteen end usages from the preceding audit now close.

Closure still converges Incomplete in 31 rounds with four MayTimeVary failures.
The exact remaining subjects are `63d0949c-20e2-5ffa-81a2-62b9826530dc` and
`86ddb7cd-ac22-5322-9aa7-ff4755f00198` (Flows lines 69/68, `[1] target/source`),
and `7661b183-d02f-5ba1-ae73-8f0f8b60b3c3` and
`fc5a3876-bdc7-555d-a38d-0720ca04e233` (Items lines 138/139, `[0..*]` ends).
OwnedCrossing, PositionalRedefinition and VariableFeaturing remain Pending on
all four; their EffectiveOwnership requirement is closed. CrossDomain,
FeatureValuation and FeatureValue are Complete. Source spans in the ignored
`diagnostic-sources.jsonl` were matched only after original KPAR SHA-256 validation.

There are 11,044/11,547 closed applicable pairs, 382,623/407,490 closed requirements,
20,030 subject evaluations, 3,418 derived elements and 6,197 local elements.
The certificate is 2,631,923 bytes; optional revalidation data is 682,310,664 bytes.
Two transports retain 1,235 and reopen 11,919 evaluations in aggregate. Final
report, stage stream and source-map hashes are retained with the command record.
This remains scoped failure evidence. A matching explicit-multiplicity fixture
must diagnose the remaining boundary before another corpus attempt.

The subsequent medium13 report at clean source `262fd5c` exhausts the generic
32-round limit: contextual stages 23–30 add 1,854, 338, 0, 9, 54, 10, 9 and 2
elements; stage 31 adds none but issues a changed certificate, increasing
closed pairs from the pre-issue observation 9,147 to 14,404. Incomplete subjects
are queued for another evaluation, which the resource limit prevents.
`converged=false` and Incomplete remain the correct result; this is not evidence
that the 14 remaining incomplete pairs will all resolve with more budget.

Source `fccc70f` sets one documented Systems publication resource budget of 64
rounds for preparation and the example's strict acceptance call. The generic
KerML default and explicit publication caller options are unchanged. The
example reports `round_limit` and `round_limit_reached` independently of semantic
acceptance. A small Function/FeatureReferenceExpression fixture proves that
stopping at the structural-to-contextual handoff stays Incomplete, whereas
sufficient ceilings yield a fully closed certificate with identical canonical
records, occurrence identities, proof facts, semantic context and certificate
digest. It uses no publication cache or synthetic acceptance marker.

| Command | Actual result | Output SHA-256 |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kerml-semantics --lib stratum_handoff_needs_budget_and_sufficient_limits_preserve_complete_outputs -- --nocapture --test-threads=1` | 0; one passed, 1.90 s test / 36.24 s wall | `37835cb76bf982147ee0ce1e29e2deeae0b81936f4aa5a1781872c155ad19581` |
| `cargo clippy --locked --offline -p agq-kerml-text --example sysml_systems_publication -- -D warnings` | 0; 4.83 s wall | `a331e3437259cc949a8fba037d6402e2cb2cc52a6b714b6ff77b4613142d58f5` |
| `cargo doc --locked --offline --no-deps -p agq-kerml-text` with `RUSTDOCFLAGS=-D warnings` | 0; 10.77 s wall | `713dfbcd40aa8f0db628faf0c0e9e50c9c0e2d0adb38c5537512b98820c1d8d6` |

All used one build job, one test thread, disabled incremental/debug artifacts,
and the isolated `target/bridge-self-model` target. The test executed before a
final formatting-only change; `cargo fmt --all -- --check` and `git diff --check`
then exited 0. The initial candidate test fixture used a plain chain with no
deferred binding handoff and therefore failed its fixture precondition (same
test command, exit 101, 47.84 s wall, output SHA-256
`59d24b159a9ded43e092ef03366574d8fc2b8a131aa6f5fccf5abbb3768822c7`);
that outcome is excluded as evidence of a scheduler defect. Raw logs are
`systems-round-budget-{binding-regression,clippy,rustdoc,regression}.log` under
the existing ignored bridge directory. No medium/full publication was rerun by
this change's author; acceptance remains unestablished.
