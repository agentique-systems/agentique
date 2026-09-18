# ADR 0013: maintained KerML grammar and lossless production trees

Status: accepted for the KerML standard-library bootstrap, 2026-09-18.
Implementation and acceptance remain separate gates.

The authority is the supplied, pinned KerML **1.0** PDF, clauses 8.2.2--8.2.5,
including the expression precedence/associativity rules in Table 6. No preliminary
1.1 grammar or SysML frontend is introduced. Grammar recognition is independent
of name resolution and canonical semantic identity.

## Evidence and inventory

`standards/grammar/kerml-1.0.ebnf` is a maintained recognition projection of the
published textual grammar. It strips abstract-syntax assignments, never library
source text. `tools/kerml-grammar/inventory.py` compiles EBNF into a finite Earley
chart. Its inputs are produced by `agq-standard-libraries --example grammar_inputs`
from verified original KPARs and the existing lossless lexer. It does not scan
library keywords to guess declarations. Each occurrence comes from a complete
derivation under `RootNamespace`, with enclosing grammar context and byte ranges.

The first complete corpus derivation uses **199 production kinds** in **36 files**.
The machine-readable inventory records all production ranges, counts, examples,
documents and separate frontend/CST/lowering/resolution/interpretation support.
Per-document production lists expose the dependency closure missing from the
bounded frontend. Empty grammar productions are recorded at zero-width ranges.
The inventory chooses the first grammar derivation deterministically; its
expression derivations are recognition evidence, **not** a precedence-resolved AST.
The runtime frontend must implement Table 6 before expression AST acceptance.

The corpus contains 204 imports (167 membership and 37 namespace), one alias,
1,760 feature elements, 505 classifier declarations, 1,498 multiplicity bounds,
334 documentation elements, 1,092 owned expressions and 323 return memberships.
Expression wrappers and ownership productions are included; these counts are not
canonical model element counts. Metadata classifiers are used, while metadata
feature applications, recursive imports and conjugations are not used in this
exact corpus. Unused productions are reported rather than assumed required.

## Alternatives

| Approach | Assessment against this corpus and authored editing |
| --- | --- |
| Extend the current recursive-descent parser | Good recovery and edit reconciliation, but three declaration variants cannot represent the 199-production closure. Adding keyword branches to the existing file would hide contextual distinctions and duplicate grammar authority. |
| Maintain grammar data and generate recognition tables | Selected. Gives production-level traceability, deterministic stale checks and a reusable grammar inventory; retains explicit abstract-syntax construction outside recognition. |
| External LR/PEG parser generator | Possible, but indirect left recursion, nullable productions and expression disambiguation need transformation; generators do not supply provenance, identity reconciliation or authored recovery. No verified Rust grammar dependency exists in this repository. |
| Rowan with handwritten recognition | Rowan provides a mature immutable tree, but does not recognize grammar or resolve the ambiguity. Introducing interning and green-tree identity is unnecessary for immutable source/range storage at this stage. |
| Generalized Rust chart recognizer with a range arena | Selected recognition mechanism for the generated tables. Handles indirect left recursion and nullable grammar without semantic predicates. Bound chart work and source nesting explicitly; retain tokens/source once, production nodes as typed arena views. |

This decision follows executable corpus recognition, not a comparative performance
claim. The reference recognizer is maintenance tooling. Runtime parsing must work
offline with checked-in generated tables, with no Python, generator, acquisition or
build-script dependency. Grammar edits must regenerate and stale-check the tables.

## Runtime representation and recovery contract

Keep source bytes and the contiguous UTF-8 token partition. Production kinds form
a generated enum; nodes hold kind, source range, syntax identity and child indices.
Borrowed typed views expose packages/namespaces, membership forms, imports, aliases,
names, visibility, classifier/feature headers, relationship clauses, multiplicity,
directions/modifiers, annotations and expression structure. Do not create a Rust
record type for every grammar wrapper. Neither a syntax node nor its qualified
name is a canonical semantic identity.

Use Table 6 to disambiguate expression structure; do not evaluate expressions.
Preserve every expression and explicit unsupported evaluation status during later
lowering. Malformed authored documents retain recovery ranges and known valid
declarations. Recovery must not hoist nested unknown text into the enclosing scope.
Old source revisions remain immutable, and reconciliation remains conservative
and edit-evidence based. Library identity uses the separate immutable allocator.

Chart recognition has a cubic worst-case time and quadratic chart-space boundary
for ambiguous input. Enforce work limits and retain an explicit failure, rather
than guessing a parse. Corpus success is not a claim that arbitrary authored
inputs have linear cost. Future optimizations may replace chart internals while
preserving the production/range/recovery contract.

## Published grammar transcription dispositions

These are transparent recognition interpretations, not edits to authority files.
No strict metamodel-authoring diagnostic changes as a result of this ADR.

* Printed pages 88--89: `Feature` has an unmatched closing parenthesis. The
  projection groups the two prefix alternatives before `ValuePart? TypeBody`.
* Page 87 `SpecificType`, page 89 `BasicFeaturePrefix`/`FeaturePrefix`, and page
  105 `PrefixMetadataFeature` have missing/mistyped rule separators. Only the
  EBNF punctuation is normalized; alternatives are retained.
* Page 94 requires `FeatureDeclaration` in `Invariant`, but page 57 explicitly
  illustrates `inv { ... }` and `inv false { ... }`. Anonymous invariants also
  occur in the pinned libraries. Recognition allows an absent declaration and
  records this published grammar discrepancy; no name or declaration is fabricated.
* Pages 95--96 spell the metaclassification operator rule with inconsistent
  capitalization. The projection uses `MetaclassificationTestOperator`.
  Its `@@` is two `@` tokens under the symbol list on page 78.
* Page 99's `NonFeatureChainPrimaryArgumentMember` refers to `PrimaryArgument`
  despite the adjoining non-feature-chain productions. The projection follows
  those adjoining productions to prevent recursive feature-chain ambiguity.
* Page 101 misspells `InstantiatedTypeMember` as `InstatiatedTypeMember`.
* Page 100 references an undefined `InvocationTypeMember`; the projection retains
  that production name with `QualifiedName` recognition. Its abstract-syntax
  interpretation still requires explicit lowering rather than an invented object.
* Page 103 references undefined `ItemFlowDeclaration`; the projection uses the
  adjoining `FlowDeclaration`. The unmatched payload-alternative parenthesis on
  that page is closed. No flow token sequence is rewritten.

The inventory is deliberately distinct from semantic loading. Gate 0 success
does not satisfy any canonical publication, import resolution or binding gate.
