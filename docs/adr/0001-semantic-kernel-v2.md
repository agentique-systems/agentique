# ADR 0001: A generic, immutable semantic kernel

Status: accepted for the first foundation milestone; implementation refinements
are recorded here. Baseline: KerML 1.0, SysML 2.0, Systems Modeling API & Services
1.0 (future integration only).

## Problem and evidence

The existing engine intentionally implements a bounded subset. Its source-linked
records, separate binary relationships and mutable expansion are useful reference
material, but cannot be the canonical substrate for the full language architecture.
`agq-kernel` is additive and depends on none of the previous domain crates. No
parser, application, workspace or simulation migration is part of this decision.

Reviewed: AGENTS.md, README, docs/architecture.md, standards/coverage.json,
crates/model/src/lib.rs, crates/semantics/src/{lib,expansion}.rs, supplied PDFs,
standards/{baseline-lock,lock}.json and vendored .project.json/.meta.json files.
In particular KerML 1.0 sections 8.3.1, 8.3.2.1.2–3, 8.3.3.1.8 and 8.3.3.3.7
establish abstract syntax, stable element identity and reified relationships;
SysML 2.0 section 8.3.11 describes parts. API 1.0 section 7.1.2 separates data
identity from versions and immutable commits. Kernel revisions are not API commits.

The [KerML 1.0 publication](https://www.omg.org/spec/KerML/1.0) lists normative
[MOF XMI](https://www.omg.org/spec/KerML/20250201/KerML.xmi) and
[abstract-syntax JSON](https://www.omg.org/spec/KerML/20250201/KerML.json).
The [SysML 2.0 publication](https://www.omg.org/spec/SysML/2.0) lists normative
[MOF XMI](https://www.omg.org/spec/SysML/20250201/SysML.xmi).
These are the intended inputs for later descriptor generation. The vendored
KerML.kerml and SysML.sysml reflective models corroborate the structure; the
vendored KerML-Model-Interchange.json describes project metadata, not the complete
metamodel. Do not mistake it for a normative descriptor source.

All original PDFs, HTML and library bytes remain unchanged. The repository's
April 2026 library corrections retain their separately documented provenance;
they do not change the 1.0/2.0 target. Preliminary 1.1/2.1 material is not used.
The small programmatic test metamodel is explicitly illustrative, not an import
or conformance claim; intermediate classes, redefinitions and semantic rules are
deliberately omitted.

## Decisions and invariants

* Every element, including every relationship, has opaque stable `ElementId`
  identity. Typed UUID-sized IDs are independent of names, source locations,
  revision identity, database rows and any future dense handles. Callers allocate
  declared IDs; duplicate/reused IDs within a history are rejected.
* The generic record contains ID, metaclass, property slots and origin only.
  Owner, name, qualified name, type and specialization are not universal fields.
  Relationships are ordinary records whose descriptors supply endpoints and
  other properties. Collections allow arbitrary arity. No convenience field
  duplicates canonical relationship truth; inherited features are not copied.
* Immutable registries own versioned metamodel descriptors. Classes have typed
  identity and direct supertypes; properties have typed identity, owner, value
  kind, multiplicity, ordering, uniqueness, derivation and containment metadata.
  Multiple inheritance computes a deduplicated property union. Cycles, unknown
  descriptors, duplicate IDs and ambiguous inherited names fail construction.
  Redefinition/subsetting/opposites need an explicit future descriptor extension;
  this milestone never guesses override semantics from names.
* Scalar, ordered collection, unordered unique collection and unordered bag are
  distinct value shapes. Missing slots differ from present empty collections.
  Primitive values initially include Boolean, signed 64-bit Integer and String;
  this is not a claim to cover normative unbounded Integer or Real. References
  carry ElementId and validate against the descriptor's target class.
* Declared snapshots and derived overlays are separate types. Declared slots
  cannot write derived properties. A revision-bound overlay can supply computed
  properties and derived elements, with rule identity and explicit dependencies.
  It never overwrites declared slots. Uncomputed required derived properties
  are allowed: structural acceptance is not semantic completion or verification.
* All changes pass through a change set. Validation examines the final candidate
  so forward references within a transaction work. Dangling/ill-typed references,
  illegal slots and invalid multiplicities fail atomically. Deletion is explicit,
  with no implicit cascades. Composite references enforce one owning slot and no
  containment cycles; callers delete the necessary dependent records together.
* Snapshots share their immutable registry and unchanged `Arc<ElementRecord>`
  values. Construction initially clones ordered map structure and rebuilds
  indexes. APIs expose borrowed records and deterministic iterators, not maps.
  Persistent maps and incremental indexes can replace this after measurement.
  Snapshots are Send + Sync with no locks or interior mutability.
* A change set binds to its exact base snapshot and reserves one new revision ID.
  Appending an operation changes that candidate ID, including after an earlier
  application, so one revision ID cannot describe two different candidate states.
  Applying the same set to the same base gives the same observable contents and
  revision. Distinct branches receive distinct revision IDs. Allocation randomness
  is outside application of a change set. Revisions are not persistence commits.
* Indexes contain metaclass membership (including supertypes), incoming and outgoing
  reference occurrences with property and position. They are reconstructible
  caches, not additional semantic facts. Ordered maps/sets define public order.
* Authored source, standard-library, transformation and internal generation origins
  are distinct. Source origin uses document/syntax IDs and byte ranges independently
  of semantic identity. Slot origins preserve fact provenance as well as record
  origin. Derived explanations reference elements, slots or other derived facts.
  Dependencies must resolve and form an acyclic explanation graph. Whole-overlay
  invalidation on declared revision change is the conservative initial policy.
  The subject is included automatically; producers are responsible for complete
  supporting evidence. An overlay's base revision is not an identity for its
  result set: future memoization must also identify rule set/computation context.
* Derived element IDs use a private, versioned UUID-v5 domain over a typed rule,
  subject and output key. This is a kernel-local deterministic policy, not an OMG
  ID formula. Rule IDs must change when rule identity/meaning changes. Later
  normative import/export and cross-tool identity policy remain explicit work.

## Alternatives and boundaries

Rust struct inheritance cannot represent metamodel multiple inheritance. A large
enum of language kinds or arbitrary string kinds hardwires semantics into storage.
Neither is adopted. Future generated wrappers hold an ElementId and snapshot
reference and obtain properties through generic queries without duplicate storage.

A single mutable expanded model obscures authored versus inferred facts and makes
old revisions unstable. Full deep cloning is simple but needlessly duplicates
unchanged records. Persistent collection libraries are deferred until benchmarks
justify them. `uuid` (identity and deterministic derivation) and `thiserror` (typed
errors) are the only direct dependencies, already used in this Apache-2.0 workspace;
no serialization, graph database, async runtime or unsafe code is introduced.

Salsa may later cache semantic queries keyed by revision and typed semantic IDs.
It is not a storage dependency and has not been experimentally evaluated here.
Explanation dependencies initially describe positive facts; negative/search-space
dependencies and selective invalidation require a later query contract.

Parser/CST, normative abstract syntax lowering, name resolution, full semantic
elaboration/validation, standard-library loading, interchange, API, persistence,
diagrams and execution are deferred. They need a trustworthy shared substrate
first. Execution will compile a separate IR; runtime state never enters records.
The kernel enforces only the structural invariants explicitly described here;
successful snapshot or overlay construction never implies language verification.
