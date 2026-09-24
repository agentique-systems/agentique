# Modeling platform Phase 3 roadmap

Phase 2 establishes immutable source-backed revision and branch contracts. Its
measured acceptance record determines what is available; this roadmap does not
waive unfinished Phase 2 gates.

Views and projections should retain an explicit project revision and semantic
context. A view stores selections and presentation choices, not another editable
semantic graph. Diagram foundations follow those projection contracts; rendering
and execution are separate milestones.

Broader Systems Modeling API support should extend the deterministic pinned
inventory. Programmatic modeling commands require a reviewed mapping from typed
canonical operations to authored source, syntax reconciliation and provenance.
Standard POST commit must not bypass those rules. Effective property projections
need to preserve incomplete/invalid results and avoid manufacturing schema defaults.

## Merge and reconciliation

Automatic merge remains unsupported. Before implementation, adopt a versioned
three-way contract over a common parent, DocumentIds, SourceRevisionIds and
canonical identity reservations. Specify independently changed declarations,
rename versus delete, moved sources, competing ordered relationships, overlapping
source edits and retired identity conflicts. Qualified names cannot establish
element identity. Produce an inspectable source candidate with explicit conflicts;
run ordinary workspace construction/validation before a durable CAS commit.
No winner selection by timestamp or silent retry is acceptable.

Repository GC needs its own retained-reference policy, including tags, operation
receipts, branch deletion, detached history, active readers and cache eviction.
Deleting a head does not authorize deleting its history. Cache eviction is already
independent of durability. A future GC transaction must account for external
artifacts and crash recovery before removing content-addressed bytes.

Continue authored incrementality with measured source/lowering/producer/audit
frontiers and exact full-rebuild equivalence. Dependency-based audit reuse must
include negative/provider searches and potential writers. Component sealing is
not a prerequisite. Keep separate latency/memory classes for editing, validation,
restoration, paging and parallel readers.

Execution IR, simulation migration, assistant mutation and full Gen1 application
migration remain outside this roadmap's authorized implementation scope.
