# KerML text v2 completion report

Branch: `frontend/kerml-text-v2`. Implemented as additive `agq-kerml-syntax` and
`agq-kerml-text` workspace crates. API usage and limitations are documented in
[docs/kerml-text.md](../../docs/kerml-text.md).

## Parser and CST

[ADR 0006](../../docs/adr/0006-lossless-kerml-text-frontend.md) was written before
implementation and compares Rowan green/red trees, lossless token parsing, parser
generators, Tree-sitter and adaptation of the old parser. The chosen handwritten
parser produces an immutable range CST over a complete token partition and owned
original source. Notes, whitespace, invalid tokens, modeled comments and recovery
regions retain their exact bytes. Borrowed declaration views expose syntax without
semantic denotation. Full reparsing is explicit; incremental reparsing is future work.

## Identity and source history

DocumentId, SourceRevisionId, SyntaxNodeId and ElementId are distinct. Semantic IDs
are independently allocated and reconciled only through immediate source-edit
continuity. Exact name-token edits and unambiguous formatting retain declaration,
membership and unchanged reference-clause relationship IDs. Ambiguous duplicates,
structural replacement, moves, retargeting headers and external whole replacement
do not have a preservation claim. Deletion followed by recreation allocates new IDs.
Reconciliation never resurrects identities from historical source revisions.

SourceOrigin now pins a source revision in addition to document/range/syntax ID.
Every canonical element and slot has source provenance. A generic transactional
kernel provenance update preserves the element ID and old snapshots. `Document`
retains earlier source/model revisions in memory. No parser object is stored in a
canonical record; detached snapshots remain usable without the frontend.

## Supported normative syntax

KerML 1.0 clauses 8.2.2, 8.2.3.4–8.2.3.5, 8.2.4.1 and 8.2.4.3: named namespaces,
types, features, abstract types, semicolon/brace bodies, owned specialization,
typing, subsetting and redefinition lists. Both symbolic and keyword relationship
forms are accepted. References support basic/unrestricted long names, relative
qualification and `$::` qualification. Default member visibility is public.
The printed TypeDeclaration requirement for specialization is enforced.

The implicit RootNamespace and actual OwningMembership, FeatureMembership,
Specialization, FeatureTyping, Subsetting and Redefinition records use generated
normative descriptors. Header relationships precede body memberships in canonical
ownedRelationship order. No ad-hoc syntax, Package stand-in or old-model lowering
was introduced.

## Lowering and diagnostics

Source → lossless tokens/range CST → borrowed syntax views → declarations and
ownership → semantic reference queries → kernel relationship records → bounded
validation. The semantic resolver handles declared lexical long names and public
qualified traversal. Pending specialization scope obligations are part of semantic
context identity, preventing unresolved/inherited scope gaps from silently falling
back to an outer name. Resolution evidence is rebound to the final model revision.

Working assertions explicitly distinguish resolved, unresolved, ambiguous,
wrong-kind and incomplete references. Only resolved targets enter canonical
relationship records. Syntax success, recovered syntax, working models and models
validated for this bounded slice are separate API states. `validate_slice` does
not claim full KerML constraints or executable verification success.

Recovery retains malformed and unsupported input, including `part def Engi`, while
constructing usable surrounding syntax. Invalid headers assert no declaration
semantics. Valid headers with incomplete bodies can form a working model. Unknown
bodies and string literals cannot inject declarations into outer scopes during
recovery. Regular `/*…*/` modeled comments are diagnosed as unsupported; lexical
notes remain trivia. Byte/token/nesting budgets are explicit, and failed edits do
not replace the current model.

## Actual verification

[results.json](results.json) records commands, timestamps, exit codes and log paths.
All seven required commands passed:

| Command | Actual result |
| --- | --- |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | exit 0; 161 passed, 0 failed, 0 ignored including doc-tests |
| `npm run check` | exit 0 |
| `npm run build` | exit 0 |
| `npm test` | exit 0; 6 passed |
| `npm run test:e2e` | exit 0; 4 passed |

Generated descriptor checks and standards integrity checks also passed. Original
HTML/PDF/library artifacts and generated normative descriptors were not modified.
[dependencies.txt](dependencies.txt) shows no `agq-model` dependency in the new
frontend and no syntax/parser dependency in the kernel or semantic query engine.

Thirteen new frontend integration tests use actual `.kerml` fixtures and cover
exact source/trivia reconstruction, CRLF, UTF-8 ranges, malformed/incomplete input,
identity reconciliation, relationship continuity, deletion/recreation, independent
programmatic/textual canonical equivalence, explicit reference failures, historical
source inspection, provenance and parser-independent snapshots. Two additional
semantic tests cover pending-scope context identity and public/private lookup with
evidence. Existing workspace and browser regressions pass.

## Next gap before SysML

Implement inherited/imported and multi-document name resolution, pinned library
models and implication, broader normative constraint checking and the required
descriptor expansion. Broaden KerML syntax for packages, annotations, aliases,
short names, visibility, multiplicity and expressions before layering SysML syntax
onto these boundaries. No SysML parser, graphical editing, simulation or HTTP
migration is included in this milestone.
