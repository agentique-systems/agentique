# Shared SysML textual frontend

The production frontend now parses final SysML 2.0 through the existing
`agq-kerml-syntax` lexer, chart recognizer and lossless production arena.
**13/21 pinned Systems Library documents parse completely; 21/21 preserve exact
source bytes.** The eight failures correspond to the four pending authority
interpretations. None is adopted by this implementation. This is syntax
acceptance, not canonical lowering or Systems Library semantic publication.

The implementation starts from accepted KerML milestone `9b80b68`. It adds
`production::Dialect::{KerMl, SysMl}`, `parse_sysml`, `parse_with_dialect` and
`Document::dialect`. Existing `parse` remains KerML. Edits retain the dialect and
preserve unaffected syntax identities. Both dialects share `Document`, `Node`,
`Production`, limits, source origins and expression precedence. Their reserved
names and deterministic syntax identity domains are separate.

The Engine/Vehicle/engine/SportsCar source parses completely, retaining
`OwnedFeatureTyping` and `OwnedSubclassification` endpoint productions. Empty
conjugated-port membership/definition/conjugation wrappers remain in the syntax
arena for canonical lowering; the parser creates no kernel records. The shared
reference-kind vocabulary also distinguishes MembershipImport from NamespaceImport.

## Grammar and authority

[sysml-2.0.ebnf](../../../standards/grammar/sysml-2.0.ebnf) contains 349 final
SysML productions, overriding 81 shared names and adding 268. The remaining
186 productions come from the maintained KerML grammar. The shared vocabulary
has 535 production kinds; compiled recognition uses 2,032 SysML alternatives.
KerML retains its existing 1,118 alternatives and prior grammar decisions.

[sysml-2.0-source.json](../../../standards/grammar/sysml-2.0-source.json) preserves
the pinned PDF hash, exact rule/action text, page anchors, reserved keywords and
recognition projection. Non-consuming assignment labels, actions and reference
brackets are separated from recognition, not discarded as lowering authority.
Header layout and the printed unclosed `individual`/`at` quote delimiters are
recorded transcription normalizations; the terminal words come from the final
keyword list. The unquoted grammar lookahead marker in
`PayloadFeatureSpecializationPart` consumes no source token; the named
FeatureSpecialization must still match.

Four undefined printed nonterminals (`CalculationUsageDeclaration`,
`DefinitionExtensionKeyWord`, `FilterPackageImport`, `SendReceiverPart`) have
**zero alternatives**. They accept neither epsilon nor an invented sentinel.
They are outside the complete derivations of the accepted corpus documents.

The following failures reproduce the prior strict offline projection, now in
the Rust production parser. Source bytes and failure ranges are recorded in
[frontend-status.json](frontend-status.json).

| Documents | Pending interpretation | Final production anchor |
| --- | --- | --- |
| Allocations | Add missing AllocationDefinition dispatch | DefinitionElement, §8.2.2.5.1, printed p.168 |
| Cases, VerificationCases | Admit return parameter members | CaseBodyItem, §8.2.2.21, printed p.189 |
| Connections, Flows, Interfaces, Items | Owned end prefixes for occurrence/default reference usages | DefaultReferenceUsage, printed p.170; OccurrenceUsagePrefix, printed p.174 |
| Views | Optional assert/not before satisfy | SatisfyRequirementUsage, §8.2.2.20, printed p.189 |

These proposals remain **NOT ADOPTED**; see the exact unchanged proposals and
original printed anchors in [grammar-preparation-review.md](grammar-preparation-review.md).
There is no fallback that rewrites source or treats a recovered package as a
complete declaration. A diagnostic marks the farthest unmatched token; recovery
retains the incomplete source region without hoisting nested declarations.

## Focused verification

Actual commands run on 2026-09-22 in the isolated frontend worktree. Cargo used
`--target-dir C:/Users/phili/github/agentique-systems/agentique/target`.

| Command | Result | Exit |
| --- | --- | ---: |
| `python tools/kerml-grammar/generate.py --write` then the same command without `--write` | Shared kinds generated; existing KerML tables current | 0 |
| `python tools/sysml-grammar/generate.py --write` then the same command without `--write` | 535 kinds, 2,032 alternatives; tables current | 0 |
| `python -m unittest discover -s tools/kerml-grammar -p test_inventory.py` | 3 passed | 0 |
| `python -m unittest discover -s tools/sysml-grammar -p test_generate.py` | 3 passed; strict alternatives, undefined nonterminals and empty wrappers verified | 0 |
| `cargo test --offline -p agq-kerml-syntax` | Initial fixture expected DefinitionMember instead of top-level PackageMember; all other tests passed | 1 |
| `cargo test --locked --offline -p agq-kerml-syntax` | Corrected fixture; 13 tests passed, including 7 SysML tests and all 21 source documents; SysML test body 2.11 s | 0 |
| Existing `target/debug/examples/sysml_corpus.exe` piped to `frontend-status.json` | 13/21 complete, 21/21 byte-exact; eight explained authority failures | 0 |
| `cargo fmt -p agq-kerml-syntax` | Formatting applied | 0 |
| `cargo clippy --locked --offline -p agq-kerml-syntax --all-targets -- -D warnings` | No warnings; 2.89 s | 0 |
| `$env:RUSTDOCFLAGS='-D warnings'; cargo doc --locked --offline -p agq-kerml-syntax --no-deps` | Strict Rustdoc passed; 6.38 s | 0 |
| `cargo fmt -p agq-kerml-syntax -- --check` | Clean | 0 |
| `git diff --check` and `git diff --cached --check` | Clean | 0 |

The tests cover the authored vertical, original source identity, dialect-aware
edits and reserved names, unchanged KerML parsing, operator precedence, empty
port wrappers, strict rejection of each pending interpretation, bounded work and
malformed-source recovery. The first test run's failure was a test expectation
error, not an authority change. No browser or whole semantic corpus run was
performed for this parser change.
