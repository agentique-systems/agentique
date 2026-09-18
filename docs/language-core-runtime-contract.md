# Structural runtime contract, completion v3

[ADR 0011](adr/0011-structural-registration-vs-metamodel-conformance.md) separates
source capture, registry integrity, runtime interpretation and authoring conformance.
The complete production graph has 175 classes, 715 properties, 319 binary
associations, seven enumerations and four primitive domain descriptors. KerML
provides 82 classes; SysML adds 93 and imports the same KerML identities.
All 175 typed views borrow an `ElementId` and a `ModelView`; upcasts do not copy
records. Root/Core generated fixtures remain available outside production loading.

## Registration and interpretation

`MetamodelRegistry::from_descriptors` constructs the whole graph atomically.
Duplicate typed identities, missing owners/domains/references, invalid cardinality
encodings, incoherent association incidence/opposites, enumeration identity
collisions, primitive containment and classifier/association inheritance cycles
remain hard errors. These are representation or runtime integrity requirements.
Numeric payloads in different typed ID namespaces are independent identities.

`declared_redefinitions` returns exact source edges. Class replacement follows
compatible class-owned edges through strict owner ancestry only. An incompatible
class replacement blocks that class's effective-property queries and model edits.
Association-owned ends never become class slots. Association ancestry has its own
iterative traversal, inherited-end set and checked replacement operation. Diamonds
deduplicate identities. No association generalization becomes a semantic element.

Several distinct effective properties may redefine one ancestral property. This
occurs in the published SysML multiple-inheritance graph. Each exact property
remains readable; an ancestral alias resolving to several surviving properties
returns `PropertyConflict`. The registry does not arbitrarily pick one. A display
name shared by different identities similarly makes name lookup ambiguous, while
identity-based access remains defined.

`validate_conformance` produces typed rules, severity, category, subjects, related
identities, exact sources and dispositions. `require_conformance` rejects error
diagnostics including reviewed anomalies. The validator evaluates specified local
rules; it does not certify every UML/CMOF constraint. Newly exposed subset context
and contract diagnostics remain **unreviewed** in the full SysML audit. Recording
those relations is safe; no subset relation evaluates a value or establishes an
implicit link. The derived-union frontier reports unsupported local relations.

The exact `definedFlow` edge retains the disposition pinned to both descriptors,
both external IDs, SysML 2.0, the artifact URI and content hash. Its association
has no super-association; the end is derived and non-navigable. No current class
replacement or authored navigation needs its questionable replacement meaning.
An explicit attempt to interpret it as association inheritance fails with
`UnsupportedAssociationRedefinition`. Strict conformance still rejects it.

## Primitive values

Primitive properties reference descriptor identities from the pinned UML 2.5.1
PrimitiveTypes artifact. Boolean, String, Integer and Real are carriers, not
language-name dispatch in the kernel. Two primitive domains sharing a carrier
do not become interchangeable redefinition types.

Integer uses arbitrary-precision signed integer values. Its kernel lexical API
accepts an optional sign followed by ASCII decimal digits. Equality, hashing,
ordering and display do not preserve irrelevant signs or leading zeroes.

The pinned PrimitiveTypes XMI describes mathematical Real and separately supplies
an XML Schema `double` serialization tag. The latter is not authority to round
canonical values. KerML 1.0 8.4.4.9.2, printed page 266, explicitly explains that
`LiteralRational::value` uses the available UML/MOF Real type although finite
literals denote rational values. The sole Real-typed abstract-syntax property is
this literal value; full inventories verify that fact.

`ExactDecimal` represents finite decimal/scientific literals as an arbitrary
precision coefficient times ten to an arbitrary precision exponent. The kernel
lexical API accepts a signed decimal significand with at least one digit and an
optional signed decimal exponent. It is a value-input API, not a KerML tokenizer.
Trailing coefficient zeroes are removed, zero has a single identity, and comparison
does not expand enormous exponents. `0.1`, `0.10` and `1e-1` have identical values
and hashes; nearby values that binary64 conflates remain distinct. NaN, infinity,
whitespace, separators and malformed exponents are rejected. No floating point,
arithmetic, irrational-number representation or language evaluation is implied.
This finite literal carrier is not a claim to enumerate all mathematical reals.

## Association storage

The full audits contain a machine-readable `association_shapes` dictionary and
classify every association, including ownership, navigability, derivation, both
end bounds, ordering, uniqueness, ancestry and redefinition. The generic policies
are exclusive:

* A supported class end stores the single canonical slot. Its association-owned
  inverse needs no separately authored storage. Ordered class slots paired with
  optional scalar class inverses reconstruct the inverse from the index.
* Additional authored shapes use `AssociationOccurrence`: one identity, association,
  two endpoint values, per-end positions and declared provenance. No duplicate
  class slot is permitted for the same association. Projections carry all link
  identities as provenance; each link retains its own original evidence.
* Associations with no non-derived navigable end remain descriptor/derived metadata.
  Non-navigability alone never creates a writable class property.

Occurrence storage covers the published Behavior `involvesFeature`, Interaction
`participantFeature`, ordered annotation inverse, and narrowed association ends
that do not satisfy the earlier single-slot contract. Both endpoint types,
participating inverse bounds, ordering, uniqueness and composite ownership are
validated. Required navigable ends are checked for applicable context instances.
An inverse's non-navigable lower bound is not used to manufacture authored slots
or links on every possible class instance. Global completeness of an association
interpretation is a model-conformance question, outside structural snapshot edits.
The exact lower bounds remain exposed for that validation layer.

Snapshots own immutable links and incidence indexes. Change sets insert, remove
and reorder links atomically. Deletion must explicitly repair incident links;
there is no cascade. Retired occurrence IDs cannot be reused in a history.
Branching and concurrent readers retain the original state. `move_inverse` edits
the one canonical ordered class slot and requires an explicit insertion position.
Association ancestry queries retain actual endpoint identities; they do not infer
alignment of un-redefined ends or execute language-specific subsetting rules.

## Derived states and evidence

Property reads distinguish `Absent`, `NotComputed`, `Computed`, `Incomplete`
and `Invalid`. Computed empty collections are values. Unsuccessful computations
retain typed reasons, diagnostics, rule evidence and positive/search dependencies.
Searches include absence and the exact descriptor graph. A computed assertion
cannot depend on an incomplete/invalid derived assertion as if it were complete.
Association-owned computed results remain separate overlay navigation, never fabricated class slots.
Overlays remain bound to their immutable declared revision. Language derivations
and fixed-point evaluation remain above the kernel.

`derived_union_frontier` traverses direct/transitive subsets, reflexive/cyclic
subsets, diamonds and applicable class redefinitions with explicit visited sets.
It returns contributors, inspected adjacency and unsupported relations, not a
computed union. Consumers must depend on the whole descriptor graph so newly
added incoming edges invalidate negative searches. Ordering of an evaluated union
is not invented from metadata traversal order.

## Semantic context and limits

KerML query rules are versioned `agq-kerml-query/4`. Context checks exact normative
KerML contracts while accepting unchanged contracts inside larger registries.
Its identity includes exact descriptor/source content, declared records,
association occurrences, overlay failures and searches, rule version, library pins,
options and pending assertions. Unrelated descriptors change context identity but
do not change query answers. Altered normative contracts remain rejected.

Runtime readiness and strict conformance are intentionally separate commands:

```text
cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-runtime --check
cargo run --locked --offline -p agq-metamodel-gen -- --baseline sysml-2.0 --require-conformance --check
```

The second command may fail while the first succeeds. Neither executes retained
OCL, SysML rules, libraries, parsing, transformation or simulation. Generation-1
release coverage remains a separate obligation.
