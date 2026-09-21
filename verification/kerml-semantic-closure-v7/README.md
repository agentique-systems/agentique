# KerML semantic closure v7

Historical raw command captures were pruned from the current tree under ADR 0021.
The [cleanup report](../summaries/repository-hygiene.md) identifies their archive
revision. Command-result paths and preservation inventories below describe the
historical run; retained authority and offline proof inputs remain local.

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**

Operational v5 contains the exact six authorized formal-target corrections in
KERML11-205/206/207. The new stop is **KLCV7-F-001 / KERML11-145**, independently
reproduced in pinned `ControlFunctions::'.'`. Its result binding cannot meet the
published ownedFeature/raw-result identity requirements and required featuring
domain together. The reference's plain owning membership chooses different
semantics. No correction for issue 145 is applied.

See [the independent conflict](../../docs/kerml-result-binding-authority-conflict.md),
[ADR 0017](../../docs/adr/0017-operational-formal-constraint-target-corrections.md),
and the appended [foundation review](../../docs/kerml-standard-library-foundation-review.md).
This is an authority stop, not a stop on ordinary implementation workload.

## Implemented and preserved

- An explicit v5 profile/manifest; all prior profiles and manifests remain distinct.
  The default operational profile remains v2; v3 correction facts retain their IDs.
- Six typed formal rule/target contracts, exact canonical path binding, profile and
  library identity, metaclass, uniqueness and visibility checks, and evidence/search
  dependencies. Missing targets have no fallback. Incomplete owner typing is explicit.
- Six-profile arbitrary-name, reflexive-target and corrupt-binding witnesses.
  The original v6 subobject witness remains unchanged.
- Generic newly owned EndFeatureMembership construction sets its Feature's end flag.
  Both the canonical query test and an independent dump audit check every member;
  all 180 v6 end findings are removed without source patches or shared-feature mutation.
- Required library-base redundancy follows implied parent bases. Full positional
  closure is not claimed. The experimental inherited-parameter projection was
  removed because it exceeded the formal owned-parameter rule.

## Authority and publication boundary

[authority-matrix.json](authority-matrix.json) contains six separate rows with exact
formal IDs/bodies, prose, wrong/correct target, issue/status/hash, pinned library
declarations, reference map/commit and available exact-target test evidence. The
issue's own `Occurence`/`Occurences` spelling is retained in the 206 rows; pinned
declarations/prose/reference establish the correct `Occurrences` spelling.
Reference tests are inspected source evidence, not a claim that a new pilot
Maven/Xpect build was executed.

[result-binding-authority-conflict.json](result-binding-authority-conflict.json)
includes exact source/XMI/PDF and reference graph/implementation evidence. The
failed featuring predicate concerns the **required published ownedFeature** case;
the actual reference plain-owning connector is not asserted to fail its own
featuring test. The canonical witness checks distinct identities and complete
ownership/specialization answers under all six profiles.

[validation-coverage.json](validation-coverage.json) inventories all 258 constraints:
A=38, B=213, E=6, F=1. The structural publication coverage gate deliberately exits 1.
No B rule is relabeled execution or non-applicable. There is no accepted Snapshot,
accepted binding regeneration, `LoadedKermlStandardLibraries`, or authored reuse of
an accepted library. The existing candidate binding stale check still fails and its
manifest remains unchanged. Six-profile authored metadata tests are not accepted
library integration. Execution, Systems Library publication and SysML work remain
outside scope.

## Fresh measurements

[quality-comparison.json](quality-comparison.json) records the fresh pre-change v4
printed counts and the complete final v5 JSON audit. The baseline command omitted
`--output`, so only its printed count fields are claimed. Historical v6 values are
not used as acceptance expectations.

| Measured final v5 population | Count |
| --- | ---: |
| Canonical records / references | 29,140 / 4,000 |
| Unresolved / incomplete reference answers | 4 / 64 |
| Ambiguous candidate sets / stored endpoint mismatches | 0 / 0 |
| EndFeatureMembership findings | 0 |
| Namespace distinguishability findings | 2 |
| Expression-result cardinality findings | 0 |
| Explicit checks / named formal constraints | 44 / 258 |
| Separately unevaluated expression/function elements | 3,901 |

The fresh v4 baseline has 60 incomplete answers. V5 closes six in Clocks through
implied library-base redundancy and exposes ten in Observation through the newly
checked effective owner typing of `startObservation` and `cancelObservation`.
Those ten remain ordinary inference obligations. No supporting evidence is marked
Complete to preserve an earlier count. The final audit executes 113,179 queries,
with 118 incomplete and two invalid answers; these are different populations from
the 64 incomplete reference answers. The observed OS lifetime peak working set is
883,003,392 bytes (about 842 MiB).

The [end audit](end-membership-lowering.json), [unresolved audit](unresolved-reference-audit.json),
[incomplete-answer audit](incomplete-answer-audit.json),
[distinguishability audit](distinguishability-audit.json) and
[individual findings](individual-findings.json) keep their measured populations
separate. The positional, feature-chain and expression reports explicitly identify
remaining closure, rather than presenting an inventory as a passing gate.

The quality audit retains 128-element/reference proof batches and asserts identical
SemanticContext at each batch boundary. OS working-set observations are retained in
`memory-final.json`; they do not constitute accepted-publication stress coverage.
The complete 97 MiB declaration dump is losslessly compressed. `archived-exports.json`
pins raw and compressed hashes and lengths; `construction-audit.py` reads that archive.

## Reproduction

```text
python -B -X utf8 verification/kerml-semantic-closure-v7/authority.py --check
python -B -X utf8 verification/kerml-semantic-closure-v7/pdf-clauses.py --check
python -B -X utf8 verification/kerml-semantic-closure-v7/result-binding-authority.py --check
python -B -X utf8 verification/kerml-semantic-closure-v7/construction-audit.py --check
python -B -X utf8 verification/kerml-semantic-closure-v7/coverage.py --check --gate
python -B -X utf8 verification/kerml-semantic-closure-v7/audits.py --check
cargo test --locked --offline -p agq-kerml-semantics --test formal_targets_v5 --test result_binding_authority_v7 -- --nocapture
cargo test --locked --offline -p agq-kerml-text --test end_membership_v7 -- --nocapture
```

`acquire.py` is explicit maintenance only. Offline builds never fetch authority.
`capture.py` retains exact command arrays, outputs, exit codes and source input
hashes in fresh directories. `run.mjs` retains the complete existing matrix and
restores prior product-generated evidence bytes after capturing new output.
`command-index.json` and `closing-verification.json` summarize actual runs.

Both complete structural runtime gates exit zero. The required workspace checks,
strict Rustdoc and frontend checks are recorded. Strict raw metamodel authoring
conformance remains separately nonzero. The external fixture validator retains
its pre-existing Controller RES001 result. The historical v5 preservation script
retains its pre-existing acquisition-index failures; the v7 preservation audit
compares all turn-start bytes and independently confirms those differences existed
in the fetched clean main commit. No historical file or index is rewritten.

Failed/superseded attempts are retained: a Windows lock prevented the first v5
release executable rebuild; the next exploratory corpus run was canceled after
removing the inherited-parameter experiment. An empty-candidate traversal caused
an early sparse-graph test to run excessively and was fixed before the passing
workspace run. Fixture failures exposed a wrong registry selection and an
unintended composite flag on the enclosed-performance target; both were corrected.
Initial Clippy flagged an unused shared test import, subsequently fixed. These
attempts are not final counts or passing verification.

The first completed v5 audit precedes a final antecedent-ordering fix and remains
as superseded comparison evidence. `v5-quality-final-2` supplies final counts: a
complete negative owned-typing prerequisite now short-circuits before inspecting
the owner's potentially incomplete effective type closure. The regression fixture
separately checks an inapplicable rule and a genuinely incomplete typed antecedent.

The initial worktree was clean, main was fetched/fast-forward checked, and work is
on `semantics/kerml-formal-constraint-errata-closure-v7`. See `preflight.json`.
