# SysML semantic foundation

Status: **SYSML SYSTEMS LIBRARY FOUNDATION INCOMPLETE**. The textual and query
foundation is implemented. Canonical KerML publication, accepted
bindings, the v9 default and actual authored integration have passed; see the
[KerML completion record](../kerml-v9-publication/README.md). SysML production
work now composes that exact immutable dependency over the same kernel graph.

The offline inventory covers **21/21 original pinned Systems Library documents,
86,538 source bytes, 154 imports and 165 definition introducers**. Shared lossless
lexing preserves every source byte. The shared production frontend now parses
**13/21** against the printed final grammar and preserves **21/21** byte for byte.
The eight failures match four recorded, unadopted grammar interpretations;
[frontend evidence](frontend-implementation.md) distinguishes them from unsupported
implementation. The original inventory's 424 family usage keyword surfaces are
not canonical Usage counts; implicit
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
- [Candidate status](candidate-status.json): all 21 document construction outcomes
  and the actual accepted-publication vertical result.

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

The kernel can mount the accepted library under a compatible larger registry
without copying records or changing the original dependency. Candidate contexts
retain missing obligations and asymmetric root visibility. The shared textual
constructor supports ordinary Attribute/Item/Part/Port definitions and usages,
canonical relationships, stable source identities and authored edit isolation.
Unsupported actions, states and other production families remain explicit gaps.

The parser-free `agq-sysml-semantics` crate authenticates the accepted KerML
publication and composes Definition/Usage typing, specialization, ownership,
inherited identity sets, subsetting, redefinition and names. Item/Part subset
projections preserve valid additional types; Attribute/Port narrowed domains
remain distinct. Hierarchy, provenance, search dependencies and completeness come
from KerML queries. Binding uniqueness accounts for pending namespace contents.
Current-graph answers are separate from unfinished effective SysML implications.

The actual accepted-cache integration passed the Engine/Vehicle/SportsCar authored
vertical and its independently constructed kernel equivalent. Both preserve the
original inherited `engine` identity, explicit typing, specialization and qualified
name. Mixed authored KerML/SysML resolution, stable IDs, immutable edit isolation
and four dependency-contract mismatch rejections passed. The monitored run exited
0 in **81.28 seconds**, peaking at **4,228,792,320 private bytes**, with no resource
stop and no KerML producer replay. Earlier runs exposed a typed-projection defect
and a fixture assertion that excluded valid inherited standard-library Features;
both were corrected, with their failed commands retained in `commands.json`.

The unpublished Systems candidate constructs **Attributes.sysml and Ports.sysml**.
It retains **17 kernel obligations and 19 reference assertions**. Eight documents
need the four unapproved grammar interpretations; eleven other documents require
canonical construction for actions, states, constraints, calculations, metadata,
case or view families. Missing Item/Part anchors and candidate reference refinement
remain explicit. The candidate is not mounted as an accepted Systems dependency.

Effective SysML producer closure remains incomplete, including standard base
relationships, `mayTimeVary`, conditional implications and the unapproved composite
Item `subitem`/`subitems` authority discrepancy. Broader Port/Connection/Interface
and conjugated-port semantics remain separate work. No original library bytes or
final grammar authority were changed. Generation-1 release obligations remain
separate and unchanged.

Final integration verification passed: **514 workspace tests, zero failures,
two intentionally ignored**, workspace Clippy with warnings denied, formatting,
strict Rustdoc for the touched generation-2 crates, both runtime generation gates,
metamodel and grammar stale checks, frontend check/build/tests and standards
checks. The four browser tests passed at the KerML integration stage and were not
repeated for semantic-only edits. The source/correction provenance fixture was
strengthened after activating the v9 default; the earlier failed workspace run
and successful rerun remain in [commands.json](commands.json). Original authority
and library bytes also match the fetched base.
