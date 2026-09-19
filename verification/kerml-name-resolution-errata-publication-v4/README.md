# KerML name-resolution errata and publication v4

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**

The project-authorized KERML11-140 operational semantic correction is implemented
under `agentique-kerml-1.0-operational/2`. Published and operational v1 behavior
remain independently reproducible. A separate pinned-library source conflict,
**KNRV4-F-001**, blocks accepted publication. See the
[authority review](../../docs/kerml-pinned-objects-authority-conflict.md) and
[independent source/XMI packet](separate-authority/authority-packet.json).
KERML11-140's open status is not the reason for this stop.

Operational v2 has all 4,003 endpoints and zero mandatory lower bounds. Of the
reference answers, 453 remain incomplete. The expanded validation reports 3,494
expression-result findings and 21 distinguishability findings. The
[validation inventory](validation-inventory.json) lists 258 named formal class
constraints and the 27 explicit validation checks; runtime coverage remains
incomplete. Only the independently checked Objects findings are classified as
the separate authority conflict.

All 124 redefinition assertions matched by the reference XMI have complete
answers and matching target sets. Ordinary kernel Snapshot validation accepts
29,087 records in the isolated storage experiment. Accepted semantic publication
remains false.

| Profile | Reference obligations | Mandatory lower bounds | Accepted semantic publication |
| --- | ---: | ---: | --- |
| Published KerML 1.0 | 513 | 10 | No |
| Operational KerML 1.0/v1 | 513 | 5 | No |
| Operational KerML 1.0/v2 | 453 | 0 | No |

These reference-obligation counts include incomplete answers with known targets.
Both earlier profiles retain the five original unresolved redefinitions. All
five resolve completely under v2; the summary records each rule path separately.

## Authority and reproducibility

- [Preflight](preflight.json): clean worktree, fetched `main`, base commit
  `7f1897e8c18ce0d8ab418984dc5c57f794965dd1`, dedicated
  `semantics/kerml-name-resolution-errata-publication-v4` branch.
- [Frozen KERML11-140 authority packet](authority-packet.json): original issue,
  status/update date, formal and preliminary clauses, actual reference scope and
  linker code, tests, release notes, exact two nested target mappings and hashes.
- [Acquisition index](acquisition.json): original URLs, resolved URLs, timestamps,
  byte counts and SHA-256 hashes for all captures, including later structural
  corroboration. Acquisition is an explicit maintenance operation, never a build.
- [Formal structural clause extracts](structural-authority.json) and
  [raw constraint inventory](constraint-inventory-source.json). The latter is not
  a completed validation inventory and includes operation-body references.
- [V1 byte preservation](v1-byte-preservation.json): restore the CRLF bytes
  already identified by the frozen v1 digest after Git's LF normalization.
  KERML11-81 descriptors and interpretation are unchanged.
- [Preserved records](preserved.json): 820 original records, including 797
  historical verification files. Source/library pins are also verified by the
  ordinary library loader and standards checks.
- [Final source state](source-state.json) and [evidence inventory](evidence-inventory.json):
  branch, base commit, working changes and exact source/evidence hashes.

The [profile document](../../docs/operational-kerml-profile.md) and
[v2 manifest](../../standards/kerml-1.0-operational-errata-v2.json) distinguish the
formal baseline, operational v1 and operational v2. Reference XMI and current
preliminary specifications are explicitly non-normative corroboration.

## Semantic evidence

The final [summary](summary.json) records complete profile obligation counts,
quality findings, exact XMI matches and the disposition of the original five
unresolved redefinitions. The detailed artifacts preserve unsuccessful answers
as well as successes:

| Evidence | Artifact |
| --- | --- |
| Three profile identities and frozen descriptors | [profiles-2/results.json](profiles-2/results.json) |
| Published/v1 arbitrary-name failure and v2 success | Workspace tests in [repair-2](repair-2/results.json), profile tests, and [authored-3](authored-3/results.json) |
| Adversarial redefinition matrix | [42 scenario/order records](synthetic-matrix.json), [matrix-8](matrix-8/results.json) |
| Independent separate-conflict witness under all profiles | [conflict-3](conflict-3/results.json), [source/XMI verifier](objects-authority.py) |
| Operational-v2 full obligation audit and strict kernel Snapshot experiment | [obligations-7/obligations.json](obligations-7/obligations.json) |
| Published full obligation audit | [published-2/obligations.json](published-2/obligations.json) |
| Operational-v1 full obligation audit | [v1-2/obligations.json](v1-2/obligations.json) |
| Independent reference target comparison | [obligations-7/reference-comparison.json](obligations-7/reference-comparison.json) |
| Complete corpus structural quality report | [quality-4/library-quality.json](quality-4/library-quality.json) |

The matrix checks actual and expected candidates, completeness, ambiguity,
positive evidence, search dependencies, profile and selected rule path. Reversed
relationship insertion orders retain the same target sets. The XMI comparison
checks each assertion and the complete target set for each matched declaration;
both exact nested `monitoredFeature` references must match the enclosing feature.

`Snapshot::apply` in the obligation audit checks ordinary kernel storage,
ownership and lower bounds. Its success does not establish KerML semantic
acceptance. The quality gate additionally checks named structural constraints
and explicitly records the still-incomplete validation inventory. No accepted
Snapshot facade, accepted bindings or authored consumption of accepted libraries
is claimed. Executable evaluation remains unevaluated and separately counted.

## Verification

Every run directory contains `results.json` with the actual command, timestamp,
duration, exit code and corresponding complete output log. Existing evidence is
never replaced by a rerun. The main verification records are:

- [final-1](final-1/results.json): full requested Rust, frontend, standards,
  generator, grammar, binding, preservation and structural runtime commands.
- [repair-2](repair-2/results.json): successful full workspace tests and Clippy
  after resource recovery; [lint-5](lint-5/results.json) checks final formatting
  and all-target Clippy after the audit cache-batching change.
- [docs-2](docs-2/results.json): strict Rustdoc with `RUSTDOCFLAGS=-D warnings`.
- [browser-2](browser-2/results.json): all four browser tests pass on rerun.
- [product-1](product-1/results.json) and [demo-2](demo-2/results.json): remaining
  existing product matrix commands, including extraction, release registers,
  workspace build, frontend format, independent validators, official pilot,
  process recovery and headless demo. The one independent fixture diagnostic
  `RES001` is identical to the historical record. The first demo command used a
  Windows-incompatible executable path; the corrected command passes.
- [strict-1](strict-1/results.json): raw KerML and SysML metamodel conformance
  remain nonzero, separately from both green complete structural runtime gates.
- [review-3](review-3/results.json): final authority hashes, independent descriptor
  comparison, historical preservation and standards checks. The earlier review-1 CRLF whitespace
  diagnostic was corrected by recognizing the frozen v1 manifest's line endings
  in `.gitattributes`; actual whitespace checks remain enabled.
- [summary-1](summary-1/results.json): final three-profile summary, followed by
  [closure-1](closure-1/results.json) recording the source and evidence hashes.

The historical 22-entry candidate binding manifest is preserved. Adding the
ordinary BinaryLink specialization requires a 23rd runtime role and binding
version `/2`; the existing manifest check therefore reports stale. Regenerating
accepted bindings is blocked until accepted semantic publication, and no
candidate-only IDs are relabeled as accepted.

[Resource recovery](resource-recovery.json) records disk exhaustion, failed
allocations and deliberate cancellation of waiting parallel audits. Only
regenerable workspace build cache entries were removed. The original browser
timeout screenshot, trace and context are retained in
[final-1/browser-failure](final-1/browser-failure). These incidents are not
classified as semantic authority conflicts.

## Running checks again

Use a fresh label for every capture, for example:

```powershell
node verification/kerml-name-resolution-errata-publication-v4/run.mjs matrix matrix-review-1
node verification/kerml-name-resolution-errata-publication-v4/run.mjs obligations obligations-review-1
node verification/kerml-name-resolution-errata-publication-v4/run.mjs quality quality-review-1
node verification/kerml-name-resolution-errata-publication-v4/run.mjs review review-new-1
```

Run full corpus audits sequentially to bound concurrent proof-cache memory.
The audit examples also reset memoization every 128 inputs and assert identical
semantic context identity across batches; all declarations and references remain
checked with the same rules.
The offline independent verifiers read preserved artifacts directly. Do not
rerun acquisition to replace reviewed files, switch to the later library bytes,
or interpret a successful process exit as accepted semantic publication.
