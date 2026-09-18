# ADR 0012: SysML semantics over KerML and canonical standard libraries

Status: accepted architecture and initial scope, 2026-09-18. Implementation
acceptance is recorded separately in the semantic foundation verification.

## Authority and ownership

Use the pinned SysML 2.0 and KerML 1.0 publications, XMI and exact original
KPAR content set. `standards/sysml-semantic-coverage.json` inventories every
SysML class, directly owned rule/operation bodies, inherited KerML contracts,
and library references. Inventory is not implementation coverage. Constraints
with apparent spelling/type inconsistencies retain their exact bodies; no
automatic correction follows from a name match.

The kernel remains language agnostic. `agq-sysml` supplies only structural
descriptors and borrowing views. `agq-sysml-semantics` owns SysML rules and
composes `agq-kerml-semantics`; neither structural crate depends on semantics.
SysML ownership, membership, specialization, feature typing, subsetting,
redefinition and effective features reuse KerML query conclusions and proofs.
There is one canonical kernel graph and no copied inherited elements.

Extend the existing semantic context/result/evidence contracts with versioned
language rule identities where needed. Do not define an incompatible proof
framework. Context identity includes the exact descriptor graph, model and
overlay, options, pending project assertions, both language rule versions and
the exact library content set. A missing prerequisite yields Incomplete.

## Initial semantic slice

Implement Definition/Usage first: owned/effective usages, definition typing,
specialization, reference/composition and applicable variation obligations.
The initial systems family is Item, Part, Attribute, Port, Connection and
ConnectorAsUsage, with Interface and the conjugated-port closure where required.
Library specializations must use validated resolved IDs backed by canonical
library declarations. In particular, ports require conjugation obligations;
checked metaclass casts alone do not implement their semantics. Behavior and
execution rules remain explicit deferred obligations. No execution success is
implied by structural or semantic inspection.

The exact library dependency closure is the Semantic, Data Type, Function and
Systems KPARs in `standards/normative/sysml-2.0/library-set.json`. This closure
contains cycles. Load and verify all required archives/entries before semantic
publication; traverse usage edges with visited state. Archive metadata and junk
remain inventoried. No build or test acquires or rewrites library bytes.

## Source projects and identity

A generation-2 source project owns immutable source revisions and publishes one
coherent kernel snapshot for all its documents. It does not reuse generation-1
workspace objects. Document order is explicit and deterministic. Build declared
elements/ownership together before asking semantic queries to resolve names.
Unresolved or incompatible assertions remain outside the canonical relationships
and carry source evidence. Syntax, resolution, KerML validation, SysML validation
and strict metamodel conformance remain distinct diagnostic domains.

DocumentId, SourceRevisionId, SyntaxNodeId and ElementId are separate identities.
Authored continuity uses immediate previous syntax plus explicit edit evidence
(ADR 0006). Unchanged documents keep their identities. Replacement, deletion and
recreation allocate fresh identities. Cross-document moves initially mean an
explicit deletion and insertion, therefore fresh identity; a future identity
transfer must be an explicit checked operation, never name/path matching.
Project construction is transactional in memory: failure publishes neither new
document heads nor a partial semantic snapshot. This is not repository persistence.

For immutable library elements, inspect interchange metadata for authoritative
element identities before choosing a private deterministic scheme. The scheme
must include artifact URI, content identity and an unambiguous immutable source
locator/semantic role. It applies only to those exact immutable bytes. Library
records and slots use StandardLibrary provenance; extra source evidence belongs
in a revision-bound source map. Loading an explicit declaration is not derivation.

## Completeness and dependency boundaries

Lossless syntax retains every byte, annotation and unsupported/recovery region.
No regex extraction constitutes library loading. Executable expressions may
remain unevaluated but must retain syntax and explicit incomplete evaluation.
The library quality gate reports every document and diagnostic category.

Queries retain positive facts and negative/search dependencies for namespaces,
imports, libraries, descriptors and derived conclusions. An absent lookup must
be invalidated by insertion into its searched space. Cycles use visited state;
no recursion on arbitrary graph depth or hash iteration dependent answers.

Strict metamodel conformance remains ADR 0011's separate authoring audit. Its
published errors are neither repaired nor converted into language-validation
success. Both complete structural registries must remain usable throughout.
