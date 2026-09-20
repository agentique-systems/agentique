# KerML semantic closure v8

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**

This milestone stops during the required up-front authority preflight at
**KLCV8-F-001 / KERML11-8**, independently reproduced on the exact pinned
`ControlFunctions::'.'` declaration. It does not stop on KERML11-145 or on
ordinary implementation workload. The five KERML11-145 corrections are authorized
but have not been implemented as operational v6 at this early stop.

Work began from clean fetched `main` at
`ed05cc1e30f955ed73af3689e9b98b20f192d63f` on branch
`semantics/kerml-result-domain-errata-closure-v8`. See [preflight.json](preflight.json).
No historical profile manifest or verification artifact is rewritten. The
default operational profile remains v2.

## Authority and independent reproduction

- [acquisition.json](acquisition.json) pins current official responses, retrieval
  times, byte counts, URLs and SHA-256 identities. Acquisition is explicit and
  is never invoked by a build or an offline verifier.
- [authority-matrix.json](authority-matrix.json) contains exactly the five
  KERML11-145 rules, their external IDs and original bodies/prose, graph/domain
  requirements, related structural constraints, checked exact source spans,
  current implementation/XMI behavior and proposed operational interpretation.
  [formal-pdf-clauses.json](formal-pdf-clauses.json) retains the pinned PDF pages.
- [the constraint authority map](../../standards/kerml-1.0-constraint-authority-map.json)
  cross-references all 258 formal rules against the 410 retained official issue
  records. [open-issue-preflight.json](open-issue-preflight.json) lists all 119
  open records, including unlinked risks. Rule-name matches are supplemented by
  explicit related-rule review. Missing matches are not non-applicability proof;
  unresolved applicability remains explicit risk metadata at this stop.
- [feature-reference-authority-conflict.json](feature-reference-authority-conflict.json)
  independently proves the separate inner reference-binding contradiction.
  It includes result subsetting, positional result/end redefinitions, base
  specializations, chain conformance and complete relevant supertype closures.
  No Rust semantic helper or KERML11-145 implementation is called.
- [canonical-source-witness.json](canonical-source-witness.json) ties the fresh
  Agentique declaration identities and ownership to exact source bytes at
  `[645,650)`. Reference-XMI identities are compared separately. Construction
  is not presented as a complete binding graph or accepted publication.
- [the arbitrary-name canonical test](../../crates/kerml-semantics/tests/feature_reference_authority_v8.rs)
  includes the relevant subsetting, chain, base and positional facts, and checks
  Complete query answers under all six implemented profiles.

The inner binding must relate referent C, featured by Function F, and raw result
R, featured by nested Expression E. F and E do not specialize each other. Neither
candidate domain nor an empty featuring set satisfies both endpoints. The
reference already uses plain OwningMembership and F-domain TypeFeaturing; this
still fails the raw-result endpoint check.

The independent proof separately constructs the authorized outer Function
direction with contextual result `[E,R]`: that outer domain passes while the
inner connector remains invalid. Repairing the latter changes a sixth rule
family, `checkFeatureReferenceExpressionBindingConnector`, which is not
authorized. No such correction is adopted.

The current reference release head matches the retained XMI commit
`fb97b754f29588b8e9c7a35f370880cd15eb29e7`; all 36 input file hashes are retained.
Current pilot commit `5cca16d846016e62bb1e54e0e50e675254a022ef` implements a
contextual valuation chain, but uses raw results in the other relevant adapters.
The Index adapter also uses a Collection guard rather than the published Array
guard; that separate KERML11-69 issue is recorded without adopting it.

## Fresh corpus and coverage

The fresh declaration command exported 29,140 records. Its bytes equal the v7
declaration export; [archived-exports.json](archived-exports.json) pins both the
uncompressed identity and deterministic lossless archive. This is a measured
historical reproducibility result, not a hard-coded expected count.

[quality-comparison.json](quality-comparison.json),
[grouped-diagnostics.json](grouped-diagnostics.json), and
[reference-obligation-audit.json](reference-obligation-audit.json) report the
fresh full library audit and compare it with historical observations. The audit
uses deterministic batches of 128 elements while retaining one semantic context
identity. [memory-samples.json](memory-samples.json) records process memory;
sampling began after refinement had started, with Windows' process-start peak
working-set counter also recorded. No timing threshold is used as correctness.

| Fresh v5 observation | Count |
| --- | ---: |
| Canonical records | 29,140 |
| References | 4,000 |
| Unresolved references | 4 |
| Incomplete reference answers | 64 |
| Ambiguous candidate sets / stored endpoint mismatches | 0 / 0 |
| EndFeatureMembership findings | 0 |
| Namespace distinguishability findings | 2 |
| Expression result cardinality findings | 0 |
| Unevaluated expression/function elements | 3,901 |

These totals reproduce the v7 observations; production language semantics are
unchanged at this Gate 1 stop. The fresh audit completed with exit 1 after
1,894.672 seconds; duration is recorded only for reproducibility.

[validation-coverage.json](validation-coverage.json) retains every formal rule.
The five authorized KERML11-145 entries are reviewed operational errata with
explicitly pending implementation. The former F entry is no longer an
unauthorized conflict. The new F entry is KERML11-8. Category B obligations and
pending erratum implementation remain structural work; none is put under
DeferredExecution. The coverage gate remains nonzero.
The authority categories are A=38, B=208, E=11, F=1. E includes six already
implemented v5 target corrections and five newly authorized v6 corrections
whose implementation is explicitly pending. There are therefore still 213
structural implementation obligations. The fresh audit executes 44 named checks
(38 ordinary checks plus the six implemented v5 errata checks).

The [chain audit](feature-chain-structural-audit.json) and
[positional audit](positional-redefinition-audit.json) distinguish the complete
local conflict proof from outstanding whole-corpus expansion. The
[distinguishability audit](distinguishability-audit.json) retains Triggers
findings as ordinary inference defects. The authority map's KERML11-72 metadata
is not evidence that those findings require an erratum.

## Verification and exit interpretation

Every captured command retains its actual command, output, duration and exit
code. [command-index.json](command-index.json) indexes successful, failed and
superseded attempts. [closing-verification.json](closing-verification.json)
summarizes final evidence without equating process completion with acceptance.

The [full existing matrix](full-matrix-1/results.json) passes formatting, Clippy,
workspace tests, generated-model stale checks, both structural runtime gates,
strict Rustdoc (`RUSTDOCFLAGS=-D warnings`), standards checking, frontend checks
and build, Node tests, browser tests, grammar inventories and independent runtime
verification. [final-rust-2](final-rust-2/results.json) reruns Rust checks after
strengthening the canonical witness. Historical profile regressions are retained
in [historical-profile-matrix](historical-profile-matrix/result.json).

The [product matrix](product-1/results.json) also reruns registers, extraction,
workspace build, frontend formatting, independent starter/fixture validation,
official pilot validation, recovery and the headless demo. Generated historical
product reports/screenshots are captured here and their original bytes restored.

Nonzero exits are retained and interpreted separately:

| Check | Meaning |
| --- | --- |
| Full library quality | Semantic publication remains incomplete; final diagnostics are retained. |
| All-constraint coverage/publication status | Not accepted; pending structural implementation and the independent conflict remain explicit. |
| Candidate binding stale check | The historical candidate binding manifest remains stale. Accepted regeneration requires semantic acceptance first. |
| Raw KerML/SysML conformance | Existing published metamodel authoring anomalies; separate from both passing runtime gates. |
| Independent fixture validation | The independent validator reproduces the pre-existing Controller `RES001` diagnostic for `r.count` and exits 1. This is not a newly introduced failure. |
| Legacy v5 preservation script | Existing acquisition-index discrepancy, first reported for `KERML11-72.html`; unchanged bytes match the clean base commit. The new v8 preservation audit passes. |

Development attempts are also retained: the first acquisition tried a nonexistent
`FeatureValueAdapter.java`; the first independent proof assumed the reference
used EndFeatureMembership rather than FeatureMembership with `isEnd=true`; two
canonical-fixture attempts omitted/misidentified required endpoint properties.
The corrected witnesses pass. The first long audit was explicitly stopped after
discovering that `--output` must be supplied as `--output=...`; its Windows exit
`4294967295` is retained. The corrected command is `baseline-quality-2`.

[preservation.json](preservation.json) verifies all 2,610 starting standards and
historical evidence files and all 12 new acquisition captures. Existing v5 index
discrepancies are reported against base Git blobs, never repaired in place.
The four legacy authority verifiers regenerate their reports; their replay
outputs are retained under `historical-replay-outputs/`, and the original bytes
are restored with starting-hash verification. See
[historical-replay-restoration.json](historical-replay-restoration.json).

## Gates not claimed at the early authority stop

There is no registered v6 profile, five-family production implementation,
seven-profile correction matrix, general contextual-result API, closed formal
coverage, strict accepted semantic publication, accepted binding regeneration,
`LoadedKermlStandardLibraries`, or authored accepted-library consumption.
[strict-publication-status.json](strict-publication-status.json) records those
facts explicitly. Full authored/deep/parallel stress acceptance is not claimed
by the bounded baseline corpus run.

The existing semantic-quality operation remains fail-closed; it has not been
replaced with a new acceptance implementation. Kernel storage acceptance and
the two runtime coverage gates do not establish language-semantic publication.

See [ADR 0018](../../docs/adr/0018-operational-result-domain-corrections.md), the
[new conflict review](../../docs/kerml-feature-reference-binding-authority-conflict.md),
and the [22-question foundation review](../../docs/kerml-standard-library-foundation-review.md).
SysML semantics, Systems Library publication, execution, persistence, APIs and
application migration remain out of scope.

## Offline reproduction

Run from the repository root. Python requires the already-used `pypdf` and
`beautifulsoup4` packages; no evidence is downloaded by these commands.

```text
python -B -X utf8 verification/kerml-semantic-closure-v8/authority-map.py --check
python -B -X utf8 verification/kerml-semantic-closure-v8/authority-matrix.py --check
python -B -X utf8 verification/kerml-semantic-closure-v8/feature-reference-authority.py --check
python -B -X utf8 verification/kerml-semantic-closure-v8/canonical-source-witness.py --check
python -B -X utf8 verification/kerml-semantic-closure-v8/archive.py --check
python -B -X utf8 verification/kerml-semantic-closure-v8/audits.py --check
python -B -X utf8 verification/kerml-semantic-closure-v8/audits.py --check --gate
python -B -X utf8 verification/kerml-semantic-closure-v8/preservation.py --check
cargo test --locked --offline -p agq-kerml-semantics --test feature_reference_authority_v8 -- --nocapture
```

The `--gate` command deliberately exits 1 while acceptance is false. For fresh
command capture use `capture.py` with a new label. `run.mjs` also requires a new
directory label and never overwrites an existing run directory. The archived
full-audit command records the exact release/offline options used here.
