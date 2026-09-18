# KerML standard-library bootstrap v1

Base: `1eb1424` (fetched `origin/main`, 2026-09-18). Initial working tree clean.
Branch: `semantics/kerml-standard-library-bootstrap-v1`.

Current result: **KERML STANDARD LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**.
The current acceptance review is
[`docs/kerml-standard-library-foundation-review.md`](../../docs/kerml-standard-library-foundation-review.md).
Earlier gate descriptions below describe the state at those gates, not claims
that later semantic publication has succeeded.

Evidence directories are append-only. `run.mjs` records commands, complete outputs,
actual exit codes and elapsed time. Earlier milestone evidence and the exact
pinned source/artifact files are retained. No Systems Library semantic work or
platform work is part of this milestone.

## Gate 0

The grammar-context inventory recognizes all 36 KerML source documents and
records 199 production kinds with source ranges and examples. It is generated
from exact verified source/lexer output using the maintained KerML 1.0 grammar
projection. The first-derivation reference tree does not claim precedence AST,
canonical lowering, resolution or semantic validation. ADR 0013 records the
parser architecture comparison and each grammar transcription disposition.

`gate-0/results.json` records all six commands at exit zero: inventory currentness,
recognizer tests, existing syntax/source tests, formatting and both complete
structural runtime gates. This establishes only Gate 0.

The supplied PDF extraction is retained as reading evidence. The grammar input
file contains the verified exact sources and token ranges used during initial
analysis; reproduction normally obtains fresh inputs directly from the verified
offline loader. No acquisition occurs in either path.

Remaining gates: richer production CST, precedence, recovery, corpus frontend,
complete canonical lowering, import/alias/visibility resolution, atomic library
publication, validated bindings, loaded context identity, semantic validation,
quality gate, authored integration, dependency audit and stress/review acceptance.

## Gate 1

`agq-kerml-syntax::production` now supplies the grammar frontend, a lossless token
partition and typed production arena, explicit recovery and published grammar
discrepancies. Runtime tables are generated and checked in; Table 6 precedence and
associativity have focused structural tests. All source traversal and tree
materialization are iterative and chart work is bounded. Syntax IDs are deterministic
within an exact source revision; edits conservatively retain unaffected ranges.

The existing bounded authored adapter remains available during lowering migration.
It does not gain canonical support merely because a production tree can recognize
additional syntax. `gate-1/results.json` records seven successful commands,
including focused production tests, authored regressions, Clippy, strict Rustdoc
and both runtime checks. The whole corpus frontend report is the next gate.

## Gate 2

All 36 exact pinned KerML documents parse through the production frontend with
zero recovery and exact contiguous token/source preservation. The corpus tests
repeat every parse and compare all node kinds, ranges and syntax IDs. The syntax
quality report gives per-document counts and separates 42 anonymous-invariant
grammar discrepancies across nine documents from parser diagnostics.

`gate-2/results.json` records all seven commands at zero. The complete report is
`gate-2/syntax-quality.json`; `corpus-first.json` preserves the initial run.
Canonical/resolution/semantic counts remain null: these phases have not run.
This syntax gate does not weaken or pass the library semantic quality gate.

## Construction prerequisite (Gate 3 in progress)

The corpus requires inspection of declarations before mandatory reference targets
are resolved, including Interaction participant associations. The kernel now
offers a separate unpublished `ConstructionView` with explicit lower-bound
obligations. Strict Snapshot publication is unchanged. Tests cover inspection,
repair and publication, immutability, dangling references, wrong value kinds and
upper bounds. `construction-1/results.json` records six successful commands,
including kernel/semantic/authored tests, Clippy, strict Rustdoc and both complete
runtime checks. This is a construction prerequisite, not Gate 3 acceptance.

## Canonical construction and semantic-query implementation

`construction-2/results.json` records all six focused commands at zero. The
candidate uses ordinary complete-registry kernel records, deterministic immutable
library locators and separate source-map evidence. Repeated corpus construction
compares all records, slots, occurrences, obligations and source maps. Tests also
check the single result of every Expression, including typed casts.

Semantic lookup now handles imports, aliases, visibility, inheritance, qualified
and root-qualified names, relationship-specific contexts and project boundaries.
Adversarial tests exercise changes that invalidate misses and hits, renamed
diamond redefinitions, cycles, long inheritance and concurrent queries. The 22
standard anchors are validated in the canonical candidate and have a deterministic
stale-checked manifest. Candidate bindings do not accept the complete library.

`resolution-obligations-1.json` through `resolution-obligations-6.json` preserve
intermediate diagnostic attempts. Reports 1–5 used an unbound construction query
for their displayed diagnostic details; their stored obligation counts came from
the refined candidate. Report 6 uses the candidate's actual library-bound context.
The current quality command supersedes these attempts without overwriting them.

## Current semantic quality gate

Run entirely offline:

```text
node verification/kerml-standard-library-bootstrap-v1/run.mjs quality <fresh-label>
```

The command runs `agq-kerml-text --example library_quality`, constructs the full
three-library candidate, refines references and evaluates the existing structural
query families. Each of the 36 documents reports syntax, recovery, source
preservation, candidate canonical element/relationship counts, actual unresolved,
ambiguous and incomplete reference counts, query diagnostics and unevaluated
expression/function counts. Full constraint validation remains null. Counts refer
to an unpublished kernel construction, never an accepted Snapshot.

`quality-1` retains the current output, JSON and actual nonzero exit. The gate
requires strict publication and full structural-semantic scope; executable
evaluation is separately unsupported. Remaining reference and Interaction
participant obligations are implementation gaps, not reclassified anomalies or
external-authority blockers.

Actual result: exit **1**; 29,265 candidate elements / 16,747 relationships;
4,003 assertions / 3,998 provisional endpoints; five unresolved names, zero
ambiguities, 57 incomplete reference answers and zero mismatched endpoints.
Ten required structural values remain absent (five redefinitions and five
Interaction participant relationships). The audit evaluated 60,560 query results,
with 530 incomplete and zero invalid. It reports 3,899 unevaluated expression or
function elements. All 36 syntax parses remain lossless with zero recovery.

The old `agq-standard-libraries --example audit` now checks the preserved baseline
lexical inventory and previous quality evidence. Its `--write` option rejects
rewriting historical evidence. The maintained production inventory is generated by
`tools/kerml-grammar/inventory.py`; current semantic quality is a separate command.

## Final verification

`final-1/results.json` records the required workspace formatting, Clippy, tests,
generated descriptors, both complete runtime gates, strict Rustdoc for all eight
generation-2 public crates, standards checks, frontend checks/build, Node and
browser tests. Additional checks cover grammar tables/inventory, the binding
manifest, historical lexical evidence, independent XMI translation and protected
artifact/evidence preservation. Each result contains the actual command and exit
code; process completion alone does not establish success.
All 19 final commands exited zero. Preservation verified 693 prior files.
`review-1` reruns standards integrity after the final coverage/review update.

`strict-1/results.json` records the separate KerML and SysML metamodel-authoring
conformance commands. Both remain nonzero, preserving published anomaly errors.
`preservation.py` compares prior authority and historical verification evidence
against base `1eb1424`, with exact artifact bytes and Git-text EOL normalization.
Systems Library semantics, SysML authored text and platform work remain absent.
