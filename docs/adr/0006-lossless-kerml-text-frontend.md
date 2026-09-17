# ADR 0006: Lossless KerML text and reference-bearing lowering

Status: accepted before implementation, 2026-09-17.

## Decision and alternatives

Use a new handwritten recursive descent parser over an immutable, lossless token
stream and a range-based concrete syntax tree in `agq-kerml-syntax`. The source
buffer is authoritative. Every byte belongs to a token, including whitespace,
notes, comments, invalid tokens and recovery regions. Nodes describe nested
declarations and reference syntax; borrowed AST views interpret these nodes.
`agq-kerml-text` orchestrates source revisions, identity and lowering. Neither the
kernel nor semantic queries depend on either crate or on the old model.

| Approach | Preservation and editing | Identity and incremental work | Grammar, Rust and provenance |
| --- | --- | --- | --- |
| Rowan green/red CST with handwritten parser | Exact trivia leaves; explicit error nodes; excellent incomplete-text representation | Shared green subtrees and red navigation support later reparsing; green equality alone is not semantic identity | Good Rust integration and node ranges; another representation and dependency to maintain; grammar still handwritten |
| Handwritten parser over lossless tokens and immutable range CST (chosen) | Exact original bytes; all trivia and errors retained; explicit recovery boundaries | Revision-local ranges and independently allocated node IDs; edit continuity can reconcile IDs; full parse now, subtree reparse later | Small auditable production functions tied to normative clauses; simple Rust ownership; direct revision/range provenance |
| Parser generator (LR/PEG) | Can retain tokens; recovery and unfinished productions need grammar-specific infrastructure | Generated trees do not solve identity; incremental support varies | Grammar correspondence is attractive; semantic actions must remain syntactic; Rust toolchains available but error recovery requires separate design |
| Tree-sitter | Concrete tree and incremental recovery are strong; separate source/trivia policy still needed | Excellent incremental edits; node reuse is insufficient evidence of semantic continuity | Requires a maintained normative grammar, generated runtime/bindings and AST adapter; semantic references still need separate lowering |
| Adapt first-generation parser | Existing recovery/limits are useful evidence; source preservation already exists | Existing edit policies are informative | Coupled to old model representations; adapting that architecture would perpetuate the dependency boundary this frontend removes |

This chooses explicit source ownership and inspectable recovery over an immediate
incremental implementation. A Rowan migration remains possible behind the syntax
API. Do not claim incremental parsing: tokens and tree are rebuilt today. The
range CST is lossless through its owned immutable source, not a pretty-printer.
Its ordered token partition plus nested node ranges is the concrete representation.

## Normative slice

Authority: the supplied, unchanged `KerML.pdf`, KerML 1.0, clauses 8.2.2,
8.2.3.4–8.2.3.5, 8.2.4.1 and 8.2.4.3. The official publication landing page is
<https://www.omg.org/spec/KerML/1.0>. Grammar is checked against the local pinned
publication, not an unversioned third-party grammar.

Implement named `namespace`, `type`, and `feature` declarations with semicolon or
brace bodies, optional `abstract` on types, long names (basic or unrestricted),
and owned specialization/typing/subsetting/redefinition lists. Support the
normative symbolic and keyword forms, plus relative and `$::` qualified names.
Omitted member visibility is public. Other modifiers and productions are retained
with unsupported-syntax diagnostics. In particular, packages, short names,
imports, aliases, multiplicities, feature chains, expressions and SysML are out.

The printed TypeDeclaration requires one or more specialization/conjugation parts;
`type T;` must recover with a diagnostic, not become an accepted grammar extension.
A namespace-level `feature Base;` supplies a Type-compatible base in closed
fixtures without adding Classifier descriptors. Features in types use actual
FeatureMembership; namespace members use OwningMembership. Every document has the
normative unnamed RootNamespace. Notes (`//`, `//*…*/`) are trivia. Regular
`/*…*/` comments are modeled annotations in KerML; until annotation lowering is
implemented they are preserved with a diagnostic, never silently discarded.

## Source and identity contract

DocumentId, SourceRevisionId, byte range, SyntaxNodeId and ElementId are distinct.
SourceOrigin includes a source revision. Documents keep immutable revisions;
old origins can be inspected after editing. Kernel records contain only provenance
values, never CST pointers. Ranges always use UTF-8 byte boundaries.

ElementIds are fresh allocations associated with syntax-node continuity, never
hashes of offsets, names, or line numbers. A single edit can retain declaration
identity when its declaration keyword survives at the mapped location and its
significant header is unchanged, or when the edit replaces exactly its name token.
The owner must also retain continuity. Formatting-only replacements with identical
significant tokens may match unique sibling token signatures. Duplicate signatures
are ambiguous and receive fresh IDs. Syntax identity and semantic IDs use separate
allocations. Relationship identity is retained only with an unchanged unique
reference clause under a retained declaration; retargeting creates a new relation.

Whole-source replacement has no edit continuity and allocates fresh identities.
Deleting and later recreating a declaration never consults historical tombstones.
Moves, kind changes and structural replacements spanning the keyword do not gain
identity from similar names or positions. Precise edits carry evidence of continuity,
not a general heuristic for guessing user intent. This deliberately favors false
negatives over incorrectly merging identities. Reconciliation uses the immediate
previous revision only and does not change older trees or snapshots.

## Lowering and resolution

Parse into syntax views, construct declarations/ownership first, then ask
`agq-kerml-semantics` to resolve reference paths. The resolver implements an explicit
declared/public lexical slice, returning resolved, unresolved, ambiguous or
incomplete results with query evidence. Inherited/imported lookup is an explicit
gap, never guessed. The frontend does not decide what a name denotes.

Before resolution, the frontend supplies the semantic owning Type IDs of all
pending specialization-family assertions to `SemanticContext::for_working_snapshot`.
This set is validated and participates in the full semantic context identity.
Searching one of these scopes returns incomplete, so absent canonical edges cannot
cause a false lexical fallback. After resolved relationships are lowered, evidence
is rebound to the final published snapshot with only the remaining pending scopes.
The resolver also checks existing owned and incoming specialization relationships.
There is no speculative relationship snapshot and no fake missing-reference target.

The working model holds typed pending relationship assertions with SourceOrigin
and resolution results alongside a structurally valid kernel Snapshot. Only
resolved, type-correct assertions create canonical relationship elements; dangling
references and fake targets never enter the kernel. Required descriptor defaults
are supplied explicitly from this supported grammar's meaning (including unique
features), not synthesized by treating every required boolean as false.

Syntax success, recovered syntax, a semantic working model, and a model validated
for this bounded slice are distinct API states. Validation must inspect syntax,
resolution, and semantic-query completeness/diagnostics. Passing these checks is
not full KerML validation, library implication, or executable verification.

## Limits and consequences

Enforce byte, token and nesting budgets with typed failures/diagnostics; recovery
must make progress and preserve source. Test actual `.kerml` fixtures for trivia,
UTF-8, recovery, revision retention, reconciliation, canonical equivalence and
semantic diagnostics. Full language constraint checking, inherited/imported name
resolution and libraries remain prerequisites for a broader SysML frontend.
