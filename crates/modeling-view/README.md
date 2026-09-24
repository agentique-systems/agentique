# Revision-bound Studio views

`agq-modeling-view` projects a borrowed Gen2 `ProjectRevision`. It has no database,
HTTP, frontend, source mutation or provider dependency. Projection DTOs are
discardable display results, never canonical or editable model records.

The public operations are `project(revision, definition)`, `inspect(revision,
element)` and `explain(revision, element)`. All return the immutable project
revision identity. Every viewport node carries its original `ElementId`; every
model edge carries its canonical relationship record identity. Connection order
does not establish flow direction. Explanation arrows are explicitly marked
presentation-only.

All property reads resolve inherited metamodel identities to their effective
replacements before reading canonical storage. The focused SysML snapshot test
checks actual OwningMembership, FeatureMembership and FeatureTyping records,
their original PartUsage endpoint, ownership and direct part/port/requirement
counts. It does not construct a surrogate display graph to establish those facts.

`ViewDefinition` version 1 is project metadata containing a lens, focus,
relationship families, bounded neighborhood depth, standard expansion and hidden
identities. It contains no node snapshots. It is separate from modeled SysML
View/Viewpoint concepts. Hidden elements remain in the model.

The Architecture lens selects named PartDefinitions and PartUsages. It chooses a
system focus using canonical ownership and typing reachability, with no project
names embedded in its implementation. One architecture level traverses two real
relations: a definition owns a usage, and that usage is typed by a definition.
The engine keeps both identities and relations visible. It does not turn typing
into ownership. Focused views can additionally expose ports and connectors.

The SemanticGraph lens selects named local records and independently filters
ownership, typing, specialization, subsetting, redefinition, connection and
reference relations. Standards are excluded by default; explicit expansion adds
adjacent standard endpoints, never all standard-library records. The Requirements
lens selects requirement and verification metaclasses plus their actual semantic
neighbors. It does not manufacture satisfaction or verification edges when the
model has none.

Feature counts and node summaries are directly owned populations. Selected-element
inspection separately executes the real effective queries, preserves original
inherited identities and reports query completeness, diagnostics and evidence/read
counts. A query display summary is not a reusable cache proof. Exact UTF-8 source
ranges remain bound to document and source-revision identities. Multiplicity is
shown as a canonical expression identity; no runtime value is invented.

Explain projects immediate canonical provenance with the actual producer rule
identity. Known SysML rule names come from the active profile's producer registry.
The compact view prioritizes subject/endpoint and authored facts and displays at
most eight immediate dependencies; total evidence count and truncation are
explicit. Kernel provenance remains the full proof authority.

Far, Medium and Near detail levels are a presentation contract with camera-scale
defaults. Camera movement, selecting already loaded nodes and client-side layout
do not edit, rebuild or validate the model.

Focused tests cover canonical ownership, immutable revision identity, bounded
cyclic neighborhoods, distinct usage/typing paths, metadata round trips and proof
identity. `tests/accepted_views.rs` additionally exercises project, effective
inspection, derived explanations and immutable old views across two authored
revisions, using only the already accepted KerML and Systems cache files. It is
ignored by default and requires `AGENTIQUE_KERML_CACHE` and
`AGENTIQUE_SYSTEMS_CACHE`; it never publishes or rebuilds standards.
