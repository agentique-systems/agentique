# KerML operational errata publication v3

The operational KERML11-81 correction is implemented and independently verified.
Canonical library publication is **incomplete**, stopped at Gate 7 by the separate
KOPV3-F-001 / KERML11-140 authority conflict. There is no accepted library Snapshot
or `LoadedKermlStandardLibraries` facade, and no SysML semantic readiness claim.

Work began with a clean worktree on fetched `origin/main`, commit
`0927d90e2d882dd798e03e30b25150a75ce3ca4e`, and proceeded on
`semantics/kerml-operational-errata-publication-v3`. No changes were made on main.
Normative artifacts, generated raw descriptors and prior evidence are preserved.

## Authority and implementation

- `authority-evidence.json` pins fresh official KERML11-81 and RTF responses,
  current SysML-v2-Release commit `fb97b754f29588b8e9c7a35f370880cd15eb29e7`,
  release metadata, the 2026-05 notes listing the issue, and the preliminary
  revised KerML PDF. That PDF is version 1.1 Beta 2, August 2026. Its use here is
  corroboration only. No revised `KerML.xmi` exists in the inspected release tree.
- `inspected-authority.json` records independently extracted pages from the
  pinned 1.0 PDF and the preliminary revision, including the redefinition rule.
- `standards/kerml-1.0-operational-errata.json` is the reviewed manifest. Its exact
  bytes are also pinned by the runtime implementation. Changes require review
  and explicit versioning; runtime/build paths never acquire source artifacts.
- `omg-kerml-1.0-published/1` retains the exact generated raw graph. Legacy
  `descriptors()` and `registry()` keep their meaning.
- `agentique-kerml-1.0-operational/1` deletes two reviewed associations and their
  four exclusively owned ends in a separate descriptor set. The XML ownership
  closure contains eleven source nodes: the six runtime descriptors, their
  generalization and multiplicity literals. There are no outside incoming edges.
- The original artifact identity, profile ID, manifest digest, effective graph,
  library content, semantic rules and bindings participate in SemanticContext.
  Query version `/7` changes context identity, not the existing resolution rules.
  Authored generation-2 KerML and library construction select the operational
  profile explicitly. The generic kernel contains no KERML11-81 knowledge.

See [ADR 0014](../../docs/adr/0014-operational-standard-errata-profiles.md).

## Gates and evidence

| Gate | Actual outcome |
| --- | --- |
| 0–2 | Explicit profiles, reviewed manifest and separate APIs implemented; raw APIs unchanged. |
| 3 | Independent direct XML/UUID/compiled runtime verification confirms exactly six deletions and no other changes. Both participant witnesses pass. |
| 4–5 | Explicit operational generation-2 construction and profile-aware contexts implemented. Mismatched profiles and reintroduced removed descriptors are rejected. |
| 6 | Reproduced full corpus refinement: published participant obligations 5, operational 0. Five non-errata redefinition obligations unchanged. |
| 7 | Blocked by independently reproduced KERML11-140 authority conflict, not by KERML11-81's status. |
| 8–17 | Incomplete following authority stop: effective features, structural validation, canonical acceptance, accepted bindings and authored-library integration remain required. |
| 18 | Published graph, contradiction witness and separate strict conformance diagnostics retained. |
| 19 | Foundation review appended; historical incomplete sections retained. |

`gate-6-diff-1/obligation-diff.json` compares the preserved v2 published run with
the new operational run using identical `/6` semantic rules and exact library
content. The profile-aware `/7` identity was versioned afterward. This isolates
the schema correction: all five remaining obligation records are identical.
The runtime still rejects publication for a missing required redefinition target.
No participant Features, association occurrences or derived participant facts
were constructed to discharge removed schema obligations.

The operational full obligation report contains 62 reference findings (five
complete empty answers and 57 incomplete answers), five mandatory lower-bound
obligations and 530 incomplete structural query results. KERML11-81 does not
discharge these. The quality command retains a failing structural publication
gate and separately reports unevaluated executable bodies.

## New stop condition

`KERML11-140.html`, `KERML11-140-capture.json` and `authority-conflict.json` pin
the independent authority finding. The current official issue describes why
8.2.3.5.1 cannot resolve the intended nested time-slice redefinition pattern; it
does not supply an unambiguous corrected algorithm. The preliminary revised PDF
still contains the same rule. Two corpus `monitoredFeature` references are
complete empty answers under the specified starting scopes. The new arbitrary
name witness (`gate-7-witness-1`) reproduces this under both profiles without
libraries, imports, chains, conjugation or incomplete inheritance evidence.

A separate lexical lookup finds the intended enclosing feature, but inserting
that lookup would change the published rule and violate the task's prohibition
on permissive lexical fallback. No such correction was added. Other unresolved
references and unsupported effective-feature cases remain ordinary obligations;
this report does not classify all of them as authority conflicts.

See [the authority analysis](../../docs/kerml-operational-library-publication-authority-conflict.md).

## Verification records

`run.mjs` creates fresh append-only directories. Each `results.json` records the
exact commands, captured stdout/stderr, timestamps, elapsed times and exit codes.
Strict Rustdoc uses `RUSTDOCFLAGS=-D warnings`. Legacy tools that regenerate shared
verification files have their outputs copied here and original bytes restored.

- `profiles-1`, `profiles-2`: raw/effective descriptors, both participant witnesses,
  context identity and independent descriptor verification.
- `focused-1`: focused language tests passed.
- `gate-6-operational-1`: retained initial compile failure; corrected in
  `gate-6-operational-2`, whose full obligation capture exits 0.
- `gate-6-diff-1`: exact obligation diff exits 0.
- `gate-7-witness-1`: nested redefinition authority witness exits 0.
- `final-1`: all requested workspace, generation-1 frontend, generator, runtime,
  Rustdoc and ancillary commands; its initial Clippy default-derive finding is
  preserved and corrected in `final-repair-1`.
- `strict-1`: strict published metamodel conformance remains separately nonzero.
- `quality-1`: fresh operational corpus quality; structural acceptance is required
  and remains nonzero. Execution is excluded from this gate, never claimed complete.
- `review-1`: authority inspection, independent diff, protected artifact/history
  preservation, standards integrity and whitespace verification.

The final machine-readable `verified-results.json` distinguishes ordinary check
success from the intentionally nonzero conformance and incomplete publication
gates. Successful verification commands do not establish library acceptance.

## Final observed results

All 27 distinct final ordinary verification commands exited 0; initial failures remain preserved. Both complete structural runtime gates and strict Rustdoc passed. Both separate published conformance commands exited 1. The library quality command exited 1.

The fresh quality run reports 36/36 sources lossless, 0 recovery, 5 unresolved references, 0 ambiguous references, 0 mismatched endpoints, 57 incomplete reference answers, 5 mandatory obligations and 530 incomplete structural query results. It separately reports 3,899 unevaluated executable expressions/functions. No Snapshot is accepted. Preservation checked 838 protected historical/artifact files.
