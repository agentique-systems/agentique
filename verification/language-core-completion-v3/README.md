# Language core structural runtime completion v3

Base: `000496c7a97953206ead3f125edf1e1c777342b5`, fetched `origin/main`;
initial worktree clean. Work is on `foundation/language-core-completion-v3`.
No normative source acquisition is part of generation or a build.

The current decision is in the appended
[foundation review](../../docs/language-core-foundation-review.md).
[ADR 0011](../../docs/adr/0011-structural-registration-vs-metamodel-conformance.md)
supersedes the unconditional registration blocker while preserving ADR 0010's
evidence. The [runtime contract](../../docs/language-core-runtime-contract.md)
documents the exact supported behavior and its conformance boundaries.

## Verification

[Verified results](verified-results.json) indexes the actual command, start time,
duration, exit code and complete output log for each final check. The latest Rust,
generator, standards, Rustdoc, independent XML, authority and preservation checks
are in [final-accepted](final-accepted/results.json). Frontend typechecking,
production build, Node tests and all four browser tests are in
[the full initial matrix](final/results.json); no frontend code changed afterward.
`run.mjs` records raw process output and refuses to overwrite an evidence directory.
Strict Rustdoc uses `RUSTDOCFLAGS=-D warnings` for all six public generation-2
language/kernel crates and the generator.

Both complete `--require-runtime --check` gates exit zero. Strict conformance is
separate: [strict-current](strict-current/results.json) records genuine exit 1
for each baseline. KerML has four reviewed same-name subset errors. The combined
graph has twelve diagnostics: five reviewed naming errors, the exact reviewed
`definedFlow` redefinition-context error, and six unreviewed local subset
context/contract errors. These remain error diagnostics; none is hidden or
automatically inherited by another artifact/version.

The independent [XML/Rust checker](independent_xmi.py) reuses the unchanged second
XML parser from v1, then independently checks emitted Rust identities, primitive
domains, inheritance, complete redefinitions/subsets, ownership, opposites,
cardinality/flags, sources and all 175 view classes. It uses direct XML and Python
UUIDv5, not importer or generator implementation helpers. Its current summary is
[independent-verification.json](independent-verification.json).

[Authority verification](authority-verification.json) compares all ADR 0010 source
facts, UML rules, resolution evidence and PDF evidence with the historical record.
Only the historical complete-audit portion is kept separate: it describes the old
blocked implementation. It then checks the exact typed v3 diagnostic and both
successful registries. The original v2 checker and evidence are unchanged.

The first full matrix exposed a KerML text test fixture still matching direct
Boolean/String enum cases instead of resolving primitive descriptors. That fixture
was corrected. Earlier development logs also record the generated large-array
Windows stack overflow, corrected by incremental heap collection construction.
The original v2 `authority.py --check` intentionally becomes stale when its bundled
complete-audit hashes change; `final` and `final-repair` retain those genuine failures.
The v3 authority wrapper checks the unchanged normative findings without rewriting
the old blocked-audit hashes. Later evidence supersedes these failures explicitly.

## Gate history and classifications

* [Stage 0](stage0/results.json): reproduced the prior authority findings and both
  failed complete gates before implementation.
* [Stage 1](stage1/results.json), [association inheritance](stage3/results.json),
  [numeric correction](stage4/results.json): complete translations rerun while
  remaining generic deficiencies were still reported.
* [Storage correction](stage6/results.json), [complete generation](stage7-9/results.json),
  [stress/context/independent checks](stage10-12/results.json), and
  [derived navigation](derived-navigation/results.json): complete gates pass.
  Later final checks include the additional completeness-propagation regression.

| Category | Finding and disposition |
| --- | --- |
| A | Generated whole-graph temporary arrays overflowed the default Windows doctest stack. Emission now constructs heap collections incrementally. |
| B | The prior effective-property rule rejected distinct surviving properties redefining one ancestor. Exact properties are now preserved; only ambiguous alias lookup fails. |
| C | Integer/Real lacked exact canonical carriers. Arbitrary precision integers and normalized finite decimal values now cover the pinned literal properties. |
| D | Generic association inheritance, additional authored association storage shapes and explicit unsuccessful derived states are implemented and tested. |
| E | Reviewed naming anomalies and the exact `definedFlow` authoring inconsistency remain visible. Six newly exposed local subset diagnostics remain unreviewed. |
| F | No unresolved authority is required for the supported structural runtime algorithms. Requesting an unsupported interpretation returns an explicit diagnostic. |

## Structural stress matrix

The complete graph has 175 classes, 715 properties, 319 associations and seven
enumerations (19 literals), with four shared primitive domains. The combined
machine-readable audit distinguishes 59 association shapes: 281 associations are
descriptor/derived metadata, 30 use class slots, two use ordered slots with scalar
inverse projections, and six use the new occurrence store.

* [Kernel tests](../../crates/kernel/tests/foundation_v3.rs): numeric canonical
  equality/hash/order, invalid lexicals/domains, association parent chains/diamonds,
  missing parents/cycles, end replacement, all ownership combinations, ordered
  nonunique occurrences, both-end navigation/provenance, inverse bounds, rollback,
  explicit deletion, branching/readers, derived states, cyclic/reflexive contributor
  traversal and 10,000-edge traversal on a small stack.
* [SysML tests](../../crates/sysml/tests/foundation.rs): every descriptor, twelve
  shuffled registration orders, the exact normative association inheritance,
  exact anomaly/provenance and nonreplacement, strict rejection, review pin
  invalidation, and identity-preserving borrowed cross-language upcasts.
* Existing kernel and semantic suites retain structural failure, snapshot,
  containment, reference, overlay and 1,500-node semantic graph checks. New
  semantic tests establish unchanged query answers under larger registries,
  distinct context identity, inverse editing through one fact, and explicit
  incomplete/invalid result propagation with search evidence.

## Preservation and scope

[Preservation inputs](preservation-inputs.json) and [the checker](preservation.mjs)
protect 519 existing files, including original supplied documents, normative and
library bytes, ADRs 0001–0010, old blocker evidence and generation-1 release records.
Every field in all five previous anomaly dispositions remains unchanged; their
target descriptor IDs were added, and one exact new disposition was appended.
The old foundation review remains the byte-exact prefix of the current document.
Original full audits and the previous review are also retained in this directory.

Current OMG and release research is preserved with URLs, retrieval timestamps and
SHA-256 hashes in [research/acquisition.json](research/acquisition.json). No exact
upstream correction was found. The latest inspected 2026-08 release's KerML 1.1 /
SysML 2.1 Beta 2 material is preliminary and informative only.

The v3 evidence directory is marked `-text` in `.gitattributes` so HTTP responses,
original captures and process output retain their recorded byte identities.

This milestone provides structural runtime readiness. It does not implement SysML
semantic rules or text, standard-library semantic ingestion, repository/API,
visualization, transformations, execution, simulation or application migration.
