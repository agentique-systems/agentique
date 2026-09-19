# KerML library-content errata publication v5

Operational v3 now applies the reviewed KERML11-76 model corrections in all four
named libraries. **The library foundation remains incomplete.** Strict semantic
publication is blocked by the separately reproduced **KLCV5-F-001 / KERML11-68**
feature-chain end-conformance conflict. Ordinary inference and validation gaps
remain required, not waived.

The [gate report](gate-status.json) records every requested gate. The
[authority-conflict review](../../docs/kerml-feature-chain-end-authority-conflict.md)
explains why the prescribed nested non-end feature must redefine an end feature,
contradicting the pinned end-conformance constraint. The
[independent witness](authority-stop.json) proves the contradiction from pinned
KerML 1.0 clauses/source and current implied reference XMI. A canonical,
arbitrary-name Rust witness checks all four endpoint-flag combinations.
KERML11-76's open status is not the reason for stopping.

## Actual results

| Check | Fresh v2 baseline | Operational v3 |
| --- | ---: | ---: |
| Losslessly parsed files | 36 | 36 |
| Recovery | 0 | 0 |
| Canonical records | 29,087 | 29,140 |
| Active source-reference assertions | 4,003 | 4,000 |
| Preserved superseded assertions | 0 | 3 |
| Unresolved / ambiguous / mismatched endpoints | 0 / 0 / 0 | 0 / 0 / 0 |
| Incomplete reference answers | 453 | 453 |
| Mandatory lower-bound obligations | 0 | 0 |
| Namespace distinguishability findings | 21 | 6 |
| Expression-result findings | 3,494 | 3,496 |
| Unevaluated executable elements | 3,899 | 3,901 |
| Strict semantic publication accepted | No | No |

[Quality comparison](quality-comparison.json) retains every remaining
distinguishability/expression finding. All six distinguishability findings are
category B, ordinary Agentique inference defects. Objects and VectorFunctions
have no remaining distinguishability findings. FeatureReferencingPerformances
and Observation contain the reviewed repair facts but still require ordinary
inherited-result/feature-chain end inference. Performances and Triggers account
for the other three findings. No additional library-specific facts conceal these
gaps.

[Obligation causes](obligation-causes.json) classifies every incomplete answer:
453 have `KQ_IMPLIED_NAMING_ORDER`, including 399 outside expression contexts and
54 inside them. There are 143 incomplete query subjects. The 408 naming rows in
the quality report include repeated document-level diagnostics.
All 3,496 expression findings have exactly one owned result membership; inherited
result inference/validation remains incomplete. No structural finding receives
an execution exemption. [Formal coverage](validation-coverage.json) inventories
all 258 named constraints, records 28 explicit checks, and conservatively retains
unestablished applicability/coverage as required work.

The full [obligation audit](operational-obligations-2/obligations.json.gz) records
ordinary `Snapshot::apply` accepting 29,140 records with its base unchanged.
Its legacy field `strict_publication_attempt` refers only to kernel storage;
the separate `publication_accepted` field is false. No accepted bindings, accepted
facade or accepted authored-library dependency is issued.

## Correction authority and independent verification

The [frozen manifest](../../standards/kerml-1.0-operational-library-errata-v3.json)
has four entries, 21 source-qualified selectors and 401 typed operations. Its
SHA-256 is `c762275c6a7c4cb310c4ba3084683d62125a9319228d8653f39956c61794ae30`.
KERML11-76 remains open; this is a project-authorized operational correction,
not an adopted OMG correction.

- [Authority matrix](authority-matrix.json): all four libraries, ten root
  patterns, exact sources/memberships/features/ancestors and all 21 baseline
  diagnostics individually classified. Eighteen baseline diagnostic rows concern
  the KERML11-76 defects; three concern ordinary inference defects.
- [Independent four-model witnesses](four-model-witnesses.json): pinned/historical
  defects and current source/XMI repair facts, including latent monitor-end
  collisions not emitted while naming was incomplete.
- [Published-to-v3 semantic diff](operational-patch-diff.json): 53 introduced
  records, no removed IDs, no unreviewed changes across element, membership,
  specialization, redefinition, typing, ownership and provenance dimensions.
  This Python verifier does not call the Rust correction transform.
- [Source assertion diff](source-assertion-diff.json): every original assertion
  remains byte/evidence-identical; only the three reviewed Objects typings are
  exposed as superseded instead of active operational assertions.
- [Historical reference comparison](reference-comparison.json): all 124 matched
  redefinition assertions and their complete target sets still agree.
- [All-36 current reference comparison](current-reference-comparison.json): all
  291 matched assertions are complete and select the expected target. The raw
  target-set check fails on one unrelated reference change: current OrderedMap
  adds OrderedCollection inheritance and another elements redefinition. Two
  named pinned declarations also no longer occur in current reference parents.
  [Independent drift review](reference-drift-review.json) records these three
  differences; none is imported into v3 or used as a semantic waiver.

[Acquisition records](acquisition.json) pin URLs, commit identities and hashes
for 160 reference/issue artifacts. Issue 72, original/current source diffs,
reference tests and issue-associated commit patches are retained. Acquisition is
an explicit maintenance action and is never invoked by build or offline tests.
[ADR 0015](../../docs/adr/0015-operational-standard-library-corrections.md) explains
the canonical transform, generic correction provenance, private deterministic
IDs, profile isolation and proof dependencies.

## Commands and preservation

[Command index](command-index.json) includes exact commands, real outputs, timing
and exit codes. Each runner directory retains its own result record and log.

| Evidence | Outcome |
| --- | --- |
| [Latest Rust matrix](final-rust-3/results.json) | fmt, workspace Clippy with denied warnings, workspace tests: 0 |
| [Latest strict Rustdoc](docs-final/results.json) | All public language/kernel crates, denied warnings: 0 |
| [Complete matrix](final-1/results.json) | Generation, both complete runtime gates, frontend check/build/tests/e2e, standards, grammar, lexical corpus and independent runtime: 0 |
| [Product matrix](product-1/results.json) | Register/extraction/build/format/starter/pilot/recovery/demo: 0; existing independent fixture diagnostic: 1 |
| [Authority verification](authority-final/results.json) | Four models, stop witness, patch diff, source assertions and preservation: 0 |
| [Full obligation audit](operational-obligations-2/result.json) | Audit process: 0; semantic publication: false |
| [Strict quality attempt](operational-quality/result.json) | 1; actual structural-semantic findings above |
| [Raw metamodel conformance](strict-1/results.json) | KerML and SysML: 1; separate historical source-authoring failures |

Both complete structural runtime audits contain zero runtime errors. Their
passing results do not imply raw metamodel conformance or library semantic
acceptance. The binding-manifest check exits 1 because the historical candidate
manifest is stale; acceptance-dependent regeneration is not performed. The
independent product fixture still reports `Controller.sysml` line 20, `RES001`
for feature path `r`. Generation-1 release obligations remain unchanged.

All failed and superseded attempts are retained. Initial model-export processes
were cancelled during resource scheduling; one declaration attempt found the
lockfile needed its new serde dependency, then the final locked run passed.
An initial patch verifier represented Integer debug output incorrectly; the
corrected independent verifier passes. Three initial four-model verifier runs
exposed assumptions about unnamed inherited results, multiplicity membership
and the declaring owner of an inherited dimension; their corrected witnesses
pass. The first obligation build was cancelled for memory scheduling and the
subsequent full audit completed. `final-rust-2` exposed the old test's whole-file
hash assumption after the required profile-document extension; `final-rust-3`
passes with the exact frozen v2 bytes/hash and append-only prefix checked.

The [frozen v2 definition](operational-profile-v2-frozen.md) retains SHA-256
`d25df7cca2777bc87c9c667949249f346db62192689cacfbd96e66574a7f48b5` and is an
unchanged prefix of the living profile document. Neither v1/v2 manifests nor
historical verification scripts were rewritten. The historical v4 script's old
whole-living-document hash assumption is superseded by
[explicit frozen-authority verification](historical-authority/result.json).

`preservation.py` checks all 1,319 original pinned/historical artifacts, all
acquisitions and every archived export. Original KPAR/source/specification bytes
are unchanged. Large new model/proof exports are losslessly gzipped; the
[archive index](archived-exports.json) records compressed and logical hashes.
`evidence_io.py` reads either representation. To rerun the historical comparison
script, decompress its logical audit input into a temporary file and pass that
path; its recorded run used the identical uncompressed bytes.

The [closing verification](closing-checks/result.json) records the final
preservation check, branch identity, whitespace check, review-link checks and
implementation/review file hashes.

The [v5 foundation review](../../docs/kerml-standard-library-foundation-review.md)
answers all 19 review questions. This evidence supports an explicit incomplete
milestone and a separate authority decision, not readiness for SysML semantics.
