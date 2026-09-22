# SysML semantic foundation preparation

Status: **SYSML SYSTEMS LIBRARY FOUNDATION INCOMPLETE**. This record prepares
S0–S5 while the lead works on KerML publication. It does not start a SysML
production semantic path or establish an accepted KerML dependency.

The offline inventory covers **21/21 original pinned Systems Library documents,
86,538 source bytes, 154 imports and 165 definition introducers**. Shared lossless
lexing preserves every source byte. **SysML parse acceptance remains 0/21**:
keyword surfaces are implementation requirements, not successful productions.
The 424 family usage keyword surfaces are not canonical Usage counts; implicit
usages, anonymous parameters, specialized constructs and member wrappers require
the actual SysML grammar and canonical lowering.

- [Corpus inventory](corpus-inventory.json): exact document identities, import
  targets/visibility/ranges, all requested construct families and observed shared
  KerML/SysML extension syntax surfaces. Omitted family entries mean zero source
  occurrences. No source text was rewritten.
- [Architecture and implementation plan](architecture-plan.md): concrete reuse
  seams, dependency checks and the next corpus-driven slices.
- [Authority decisions](authority-decision.json): final SysML 2.0 authority,
  preserved archive metadata discrepancy and one specific grammar review item.
- [Summary](summary.json) and [capability status](capability-status.json): scope,
  actual command results and explicit unimplemented semantic obligations.

| Document | Bytes | Imports | Definition introducers | Required extension surfaces |
| --- | ---: | ---: | ---: | --- |
| Actions.sysml | 14335 | 24 | 18 | Action, parameter/reference usage, assignment, perform, while, succession |
| Allocations.sysml | 788 | 2 | 1 | Allocation |
| AnalysisCases.sysml | 1038 | 5 | 1 | Analysis, subject |
| Attributes.sysml | 626 | 2 | 0 | None: aliases to KerML DataValue/dataValues |
| Calculations.sysml | 990 | 4 | 1 | Calculation |
| Cases.sysml | 1619 | 6 | 1 | Case, subject, objective |
| Connections.sysml | 2133 | 18 | 2 | Connection, reference usage and ends |
| Constraints.sysml | 1263 | 4 | 1 | Constraint, return |
| Flows.sysml | 4987 | 13 | 4 | Flow definitions, message usages, event occurrence, connect, succession |
| Interfaces.sysml | 3467 | 10 | 3 | Interface, port, calculation, inline expression body |
| Items.sysml | 3946 | 15 | 2 | Item, occurrence, event occurrence, attribute, assert constraint |
| Metadata.sysml | 813 | 4 | 1 | Metadata definition; metadataItems is explicitly an ItemUsage |
| Parts.sysml | 1844 | 10 | 1 | Part, port, action, state |
| Ports.sysml | 1522 | 2 | 1 | Port and reference usage |
| Requirements.sysml | 5466 | 11 | 8 | Requirement, constraint, concern, subject, return |
| StandardViewDefinitions.sysml | 6115 | 1 | 8 | View definitions and short names |
| States.sysml | 3369 | 10 | 2 | State/action, entry/do/exit, succession, assert, bind |
| SysML.sysml | 24613 | 3 | 98 | 93 reflective metadata definitions, five enumerations, derived item/attribute usages |
| UseCases.sysml | 1271 | 2 | 1 | Use case, subject, objective |
| VerificationCases.sysml | 2488 | 4 | 5 | Verification, enumeration, calculation, metadata, requirement |
| Views.sysml | 3845 | 4 | 6 | View, viewpoint, rendering, satisfy requirement, require |

The archive is `standards/artifacts/Systems-Library.kpar`, SHA-256
`df7d8b2c6e08232ca7ce123a63148949c383fcbeaeba8d89c27ceece43793a1f`.
The four-archive content identity remains
`sha256:6cceb50286d6edd411f327201b6016b5651744d4a30175c78e019810d482e928`.
The April 2026 library release is separate and was not used.

Reproduce with:

```text
cargo run --locked --offline -p agq-standard-libraries --example sysml_foundation_inventory -- --write
cargo run --locked --offline -p agq-standard-libraries --example sysml_foundation_inventory -- --check
```

The inventory uses the existing verified KPAR loader and lexer. The bounded KerML
parser is called only to obtain its token partition; its parser recovery is not
reported as a SysML parse result. Every quoted name, string, comment and note is
excluded from keyword counts. Family definition counts are cross-checked against
every unquoted `def` token. This tooling is an inventory, not library ingestion.

The existing `FrontendUnavailable` behavior and generation-1 release obligations
remain unchanged. Canonical lowering, mixed project resolution, Definition/Usage
queries, the authored vertical and Systems Library publication remain unfinished.
