# KerML semantic closure v10 evidence

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**

Operational KerML 1.0/v7 implements the authorized KERML11-1 selector. This
milestone does not complete ordinary structural semantics or accept a standard
library publication. The default operational profile remains v2. No accepted
bindings, `LoadedKermlStandardLibraries`, or accepted-library project integration
are exposed.

Work started from clean `main` after fetching `origin/main`, at
`59abd9801d59de025bf7e346464a32848047fd80`, on
`semantics/kerml-cross-feature-errata-closure-v10`. See
[git-context.json](git-context.json). Historical records are retained unchanged.

## Implemented correction and ordinary progress

- [The v7 correction manifest](../../standards/kerml-1.0-operational-owned-cross-feature-errata-v7.json)
  extends v6 with only KERML11-1. [The profile manifest](../../standards/kerml-1.0-operational-profile-v7.json)
  pins its predecessor, correction and authority packet. Context, rule proof,
  derived identity and authored metadata are profile-qualified; v2 remains the
  default.
- `owned_cross_feature` selects in normative owned-membership order, using
  metaclass conformance for all five exclusions. Missing eligibility gives none;
  incomplete order/population gives Incomplete and no selected identity.
  Separate queries cover `is_owned_cross_feature`, `owned_cross_subsetting`, and
  the actual crossed chain's `cross_feature`. Missing graph implications are not
  manufactured by these queries.
- A reproduced v9 integration defect placed a contextual FeatureValue result
  directly under its end. V7 owns it through its consuming Subsetting or
  ReferenceSubsetting, so the exact selector cannot confuse it with a cross
  Feature. V6's graph and frozen producer identity seed remain reproducible.
- Required library general scopes, before optional redundancy pruning, resolve
  the four previously unresolved references without library-specific aliases.
  Canonical typing closure improves positive owner-type evidence. Negative
  closure remains incomplete where not all implied typing is established.
- The formal Occurrence snapshots domain, initial FeatureValue `that.startShot`
  context, feature target, value owner and unique structural result are supported.
  No runtime expression is evaluated.
- Producer plans operate on bounded subject batches with one immutable context.
  Merge rejects disagreeing facts or ownership sequences. Materialization requires
  the exact validated Snapshot. Full-corpus execution exposed and repaired a
  provenance defect: association navigation now cites canonical association
  occurrences, not nonexistent declared slots.
- Query proof origins are shared immutably within an evaluator. The public
  `fact_origins` map now holds `Arc<Origin>` values; `.as_ref()` exposes the same
  origin. Answer equality, debug encoding, full dependency closure and canonical
  producer identities are preserved. This addresses observed deep-copy costs in
  expanded-corpus proof handling without dropping evidence.
  Per-fact search evidence now uses the kernel's indexed lookup instead of
  scanning the complete overlay search map for each fact. The indexed read is
  checked against the original complete iterator, including empty results.
- V7 producer answer lists are normalized consistently for single and merged
  plans. A failing regression exposed caller-order differences; the repair
  compares complete producer answers, evidence and contextual-result lists for
  forward, reversed and partitioned input. These lists enumerate produced
  identities; semantic ownership sequences and historical profile output lists
  are unchanged.

## Authority and independent verification

[authority-packet.json](authority-packet.json) contains the exact published
operations/constraints, issue status/update, commit and parent, full before/after
implementation and regression, both pinned Occurrences witnesses, current pilot,
36 reference XMI identities, preliminary material and independent domain proof.
KERML11-1 remains formally open; the project correction is not a claim of OMG
adoption. Acquisition is an explicit maintenance operation, never a build step.

`authority.py --git --check` independently verifies Git commit/blob identities and
the exact two-file diff for `553cf8205c19241c9127ab264f8372f5b58d3895`. The diff has
82 insertions and two deletions, including the dedicated later-eligible-Feature
regression. The current pilot's equivalent selector is checked separately.

[selector-verification.json](selector-verification.json) compares the Rust
selector against a second implementation that imports no Rust query. It checks
all 11,632 source Features, actual owned-membership sequences, 152 selected cross
Features, both exact Occurrences witnesses, and 120 synthetic rows across all
eight profiles. This source selector audit is distinct from expanded graph
validation. Canonical overlay regressions separately cover FeatureValue and
KERML11-8 binding infrastructure, no candidate, incomplete order, inherited
members, corrected origins, and reversed member IDs.

[expanded-selector-verification.json](expanded-selector-verification.json)
independently replays that selector on the two full-overlay witness sequences.
It checks source ownership order preservation, actual generated BindingConnectors
and FeatureValue Expressions, and the absence of an eligible cross Feature.

[authority-applicability.json](authority-applicability.json) traverses all 258
named constraints, 218 operation/derived-property entries and 410 retained issue
records. Execution continued after each new conflict. **Traversal is complete;
exhaustive applicability proof closure is not.** Provisional issue dispositions
are explicitly marked and cannot satisfy the final authority gate.
There are 41 provisional constraint dispositions. A subsequent
[scope review of all 63 initially remaining issue records](additional-issue-scope-review.json)
records which questions concern structural corpus proofs, structural evaluability,
instance interpretation, unadopted extensions, documentation/interchange, or
absent corpus antecedents. It independently checks the absence of naked Types,
FlowEnds and PayloadFeatures. This scope review does not close conditional
applicability proofs or defer any structural rule. Unproved cases are not
promoted to authority blockers without independent local reproduction.

All independently proved conflicts are in one
[blocker register](../../standards/kerml-1.0-operational-authority-blockers.json):

| Blocker | Exact witness and unresolved choice |
| --- | --- |
| KLCV10-F-001 / KERML11-2 | `Occurrence::surroundedByOccurrences::surroundingSpace`: a required unary crossing violates the greater-than-one-end predicate. Current reference removes the end flag; no source correction is authorized. |
| KLCV10-F-002 / KERML11-4 | `TransitionPerformance::accept` multiplicity: one applicable rule requires an empty domain, another a nonempty owning-feature domain. |
| KLCV10-F-003 / no matching retained issue | `SelfLink::sameThing::self2`: literal `excluding(self)` removes neither association end when evaluated on the cross Feature, selecting a Cartesian domain unlike the corroborated binary opposite-end domain. KERML11-1 does not change this rule. |
| KLCV10-F-004 / KERML11-75 | `VectorFunctions` imports five public NumericalFunctions memberships whose names collide with owned memberships. The published OCL includes those identities; the published prose excludes them. |

These four are proved conflicts, not an exhaustive final total while other
applicability analyses remain open. No additional correction is applied.

[import-authority.json](import-authority.json) independently checks all five
imported/owned membership pairs, their visibility, explicit names and metaclass
conformance. The reference XMI retains the same pairs, and the frozen current
pilot appends the imported memberships without the prose's collision filter.
The local proof needs no inherited-name closure or runtime evaluation. Its
creation continued after the first three conflicts; broader import closure is
still unfinished.

[supplemental-authority-review.json](supplemental-authority-review.json) verifies
all 23 reference IndexExpression guards, both constructors' inherited defaults,
three Transfers Flow witnesses, zero pinned initial FeatureValues, and all 657
explicit FeatureReferenceExpression referents. The last population independently
rules out the missing-operand antecedents of KERML11-182 and KERML11-32 in the
pinned source and implemented producer set. Constructor default binding and the
other unresolved analyses are not silently marked non-applicable.
The constructor rule has published prose and a `TBD` OCL body. Neither current
reference constructor result owns a BindingConnector. That omission is retained
as reference behavior; it is neither a proof of a fifth authority contradiction
nor permission to omit the required structural default bindings.

[index-authority.json](index-authority.json) traces the first operand, canonical
result and relevant typing subject for every pinned IndexExpression. Seven have
explicit positive specialization paths to both Array and Collection, ruling out
a guard difference for those cases independently of the reference model.
Complete negative implied-type closure remains unproved for the other sixteen;
the operational Array guard is unchanged.

[remaining-structural-investigations.json](remaining-structural-investigations.json)
traces all 15 reference-binding context findings to multiplicity bounds: nine
ordinary multiplicities and six cross multiplicities. It also preserves both
Triggers parameter/redefinition paths. Those traces do not authorize library
edits or claim the ordinary positional implementation is finished.

[parameter-position-authority.json](parameter-position-authority.json) independently
reconstructs KERML11-79's exact owned parameter order. The first local parameter
explicitly redefines `receiver` and must also redefine `payload` positionally;
the checked direction predicate permits both. This proves the reported
unintended consequence, not an additional contradiction by itself. Endpoint
identities are explicitly cross-checked against the retained source-refinement
audit; this is not a second complete name resolver or complete dependent
structural validation. The current reference's source change is not adopted.

[source-structural-populations.json](source-structural-populations.json) enumerates
every source FeatureValue, feature chain and owned end/parameter/result sequence
across all 36 documents. It checks value Expression ownership and preserves
canonical ownership positions independently of Rust queries. Its complete source
inventory is explicitly distinct from incomplete implied/domain/positional
semantic closure.

## Corpus and publication gates

The source baseline in [baseline-v7.json](baseline-v7.json) measured 29,140 records,
4,000 required references, four unresolved, 64 incomplete, zero ambiguous, and
two distinguishability findings. It did not expand producers.

[full-v7-producers.json](full-v7-producers.json) records the first complete
three-library producer-plan traversal: zero unresolved, 10 incomplete references
and 20,194 proposed records. Proposed records are not an overlay.

The first two materialization attempts, [full-v7-publication.json](full-v7-publication.json)
and [full-v7-final.json](full-v7-final.json), are retained with their actual
`MissingDependency` error. Their zero validation counts mean validation did not
run; they are not passing validation results. The second attempt included the
positive typing and Occurrence domain changes and planned 20,212 records.

The first repaired overlay materialized successfully, but validation retained
roughly 10 GB of private memory. That attempt was explicitly interrupted and
restarted after releasing the aggregate producer proof before validation and
reducing validation batches from 128 to 16. Its log and nonzero termination are
retained in [full-v7-overlay/](full-v7-overlay/), with the actual observation in
[resource-restart.json](resource-restart.json). It did not produce a completed
validation report and is not a passing gate.

The next attempt completed all 4,000 expanded references (zero unresolved,
10 incomplete, zero ambiguous) and both exact expanded witness assertions, but
its validation remained unfinished. It was stopped for the shared-origin
resource repair; [full-v7-complete/](full-v7-complete/) and
[resource-restart-shared.json](resource-restart-shared.json) retain that outcome.
[disk-limit.json](disk-limit.json) records the compiler/capture disk failures and
the limited build-cache cleanup. A short shared-origin restart was also retained
in [full-v7-shared/](full-v7-shared/) before the indexed search-evidence repair;
see [resource-restart-indexed.json](resource-restart-indexed.json).

The final corpus report is [full-v7-complete.json](full-v7-complete.json).
The native audit completed with exit zero after 19,542 seconds; that exit denotes
a completed audit, not semantic acceptance. Its measured results are:

| Measurement | Result |
| --- | ---: |
| Canonical source records | 29,140 |
| Materialized derived records | 20,212 |
| Total expanded records | 49,352 |
| Required references, before and after expansion | 4,000 |
| Unresolved / incomplete / ambiguous / invalid references | 0 / 10 / 0 / 0 |
| Validation and derived-query evaluations | 183,094 |
| Incomplete / invalid query answers | 240 / 1,708 |
| Distinct validation diagnostics | 1,925 |
| Distinguishability findings | 4 member pairs in 2 Triggers namespaces |
| Independently proved authority conflicts | 4; exhaustive applicability remains unproved |

[audit-diagnostics.json](audit-diagnostics.json) traces every diagnostic subject
to pinned source and preserves the full breakdown: 1,706 implied-inclusion
failures, 212 unsupported variable-feature domains, two effective-owner typing
gaps, four distinguishability findings and one ambiguous implied-name answer.
The implied-inclusion flag cannot truthfully be set merely to suppress failures:
published Element semantics require **all** necessary implied relationships,
whereas this staging overlay is partial. The new `result`/`u` implied-name
finding under `VectorFunctions::sum0::s` still needs independent positional-order
analysis; the diagnostic alone is not a fifth proved authority conflict.

Of the 258 formal constraints, the coverage review records 44 implemented and
checked, six reviewed operational errata, eight blocked by registered authority
conflicts, and **200 NotYetImplemented**. Actual execution counts do not certify
successful outcomes. No missing structural constraint is deferred as execution.

Its command, input hashes, output and exit code are in
[full-v7-indexed/](full-v7-indexed/). It separately reports reference answers
before and after expansion and asserts the two exact expanded Occurrences
witnesses. This executable predates the final single-plan answer-list
normalization. Its audit always merges every producer batch, which already
performs exactly that normalization; canonical graph construction is unchanged.
The before/after regression and passing profile checks are retained in
[producer-answer-order-regression/](producer-answer-order-regression/) and
[producer-answer-profile-regressions/](producer-answer-profile-regressions/).
[audit-source-delta.json](audit-source-delta.json) reconstructs the exact audited
source bytes by removing only the normalization and four regression assertions;
all other captured inputs are unchanged. It does not claim that the retained
executable was rebuilt after that final change.
The same report separately reconstructs the exact initial blocker register:
only KLCV10-F-004 was appended during validation. The audit reads the register
at the end; this addition changes its blocker count without changing the graph
or query execution.
The
[publication review](publication-review.json), [structural coverage](structural-coverage.json)
and [derived coverage](derived-coverage.json) keep kernel validity, partial
expansion, validation, closed coverage and language acceptance separate.
`review.py --require-publication` is a strict evidence gate; it is not a finalized
production acceptance API and cannot construct an accepted publication.

[milestone-gates.json](milestone-gates.json) accounts for every requested gate,
including ordinary work and the unaccepted-fixture integration still unfinished.
It separates completed implemented-producer execution from complete required
semantic expansion and withholds every accepted-publication API.

Ordinary work still includes negative effective-typing closure for 10 required
references, Triggers suppression, complete positional/feature-chain/cross-feature
implications, required derived operations, structural validation, full authority
applicability proof, and acceptance/integration APIs. No unfinished structural
rule is classified DeferredExecution. The authority-only completion phrase does
not apply.

## Commands, failures and rechecks

Each command capture directory contains the exact argument vector, output,
duration, exit code and Rust/standards input hashes. `run.mjs` retains equivalent
per-command logs and `results.json` for matrices. Evidence directories are
append-only; unsuccessful attempts are retained.
The standalone standards check's generated output is preserved separately in
[latest-standards-integrity.json](latest-standards-integrity.json); the original
tracked output was restored byte for byte, as recorded in
[historical-standards-output-restoration.json](historical-standards-output-restoration.json).

[final-verification-plan-order/results.json](final-verification-plan-order/results.json) records exit
zero for all required commands:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run --locked --offline -p agq-metamodel-gen -- --check
npm run check
npm run build
npm test
npm run test:e2e
npm run standards:check
```

Both complete structural runtime commands also exit zero:

```text
cargo run --locked --offline -p agq-metamodel-gen -- --baseline kerml-1.0 --require-runtime --check
cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-runtime --check
```

Strict Rustdoc ran with `RUSTDOCFLAGS=-D warnings` for all language/kernel/library
and generator crates and exited zero. Grammar checks, historical lexical corpus
and the independent runtime-XMI verifier also pass. These do not establish KerML
library semantic acceptance.

Important retained nonzero outcomes:

- The authority closure gate exits one because conditional applicability proofs
  remain open. The [strict semantic publication evidence gate](strict-semantic-publication/)
  also exits one;
  ordinary structural gaps remain alongside the four registered conflicts.
- The historical v5 preservation verifier fails on acquisition-index hashes
  already inconsistent on clean `main`. [preservation.json](preservation.json)
  independently proves all 3,076 starting standards/evidence files unchanged,
  verifies new acquisitions, and proves all nine inherited discrepancies have
  exactly their base-commit bytes. Historical indexes and artifacts are not
  rewritten to turn the old checker green.
- Both raw published metamodel conformance commands fail in
  [raw-conformance/results.json](raw-conformance/results.json). They remain
  separate from the passing complete structural runtime gates.
- [stale-binding-gate/](stale-binding-gate/) exits one: the binding manifest is
  stale. Accepted regeneration/version advancement is withheld until semantic
  acceptance.
- The independent fixture validator reports the existing `Controller.sysml`
  `RES001` reference finding in [operational-product/](operational-product/).
  Starter validation, official pilot validation, process recovery and headless
  demo pass. No SysML semantics or product migration was attempted.
- Early compilation/Clippy issues, the intentionally failing contextual-ownership
  regression, a reserved-name fixture mistake and the first provenance test
  assertion are retained. Their repairs are covered by the passing final matrix.
  The independent import proof's first attempt also retained its failed root
  assumption; the repaired proof checks the actual unnamed canonical library root.

Focused captures include eight-profile lineage, selector matrix, independent
selector, exact Git diff, typing, renamed-scope resolution, initial value graph,
reversed/merged producer partitions, navigation provenance and concurrent
immutable readers. The reader regression compares complete values, evidence,
dependencies and context identities across four readers and four batch sizes;
no timing threshold defines correctness. Full-corpus partition invariance across
every producer family remains beyond the completed proof scope.

[resource-samples.jsonl](resource-samples.jsonl) records observed memory/CPU use.
[resource-inspection.json](resource-inspection.json) and the retained native
stack-sampling source record the long expanded-reference phase. Sampling confirms
an active lookup/cloning path, not a completion percentage or semantic result.
The native inspector and its Windows build command are diagnostic maintenance
tools, not dependencies of library acquisition, builds or semantic acceptance.

[ADR 0020](../../docs/adr/0020-operational-owned-cross-feature-correction.md) and
the [v10 foundation review](../../docs/kerml-standard-library-foundation-review.md)
record the authority boundary, integration design and all 26 required answers.
