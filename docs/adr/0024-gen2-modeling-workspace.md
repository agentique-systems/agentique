# ADR 0024: generation-2 in-memory modeling workspace

Status: proposed; implementation is conditional on the language-foundation gate.

## Context and reuse audit

The operational product is generation 1. `crates/model` stores its own mutable,
source-linked `Model` and string identities. `crates/syntax` and
`crates/semantics` construct that representation. `crates/workspace::Revision`
owns a generation-1 `Model`, and its `Candidate`/`Edit` protocols compile through
generation-1 semantics. They cannot become the generation-2 semantic store.

`crates/application` contributes useful design precedents: commands identify their
base revision, store transactions atomically write receipts and events, and head
state follows a successful commit. Its current types import generation-1 model,
workspace and simulation types. `adapters/storage` demonstrates SQLite locking,
transaction and integrity practices but also implements those generation-1
application contracts. Neither crate belongs below the generation-2 kernel or
language crates, and persistence is outside this first workspace phase.

Generation 2 already has useful source infrastructure in
`agq-kerml-text::SourceProject`: immutable document revisions, explicit languages,
stable document identities, edit reconciliation, source maps and shared accepted
KerML dependencies. Its current mixed-language constructor does not establish an
accepted Systems dependency or a validated effective SysML revision. Those are
prerequisites for reuse, not assumptions conferred by a workspace wrapper.

## Proposed decision

Add `agq-modeling-workspace` after both standard publications and the effective
authored language vertical pass. Keep `agq-workspace` operational and unchanged.
The additive name makes the generation boundary visible to consumers.

The new crate depends inward on the kernel, textual frontend and KerML/SysML
semantic contracts. It has no generation-1, storage, HTTP, execution or simulation
dependency. Accepted standard publications are shared immutable `Arc` values;
authored revisions reference their original ElementIds and records.

```text
accepted standard publications
             |
       ProjectWorkspace
             |
    immutable ProjectRevision
             |
    WorkingProjectRevision
             |
   ValidatedProjectRevision
             |
      queries / diagnostics
```

Commands carry an expected head revision and document changes. The frontend
reconciles syntax identities, lowers canonical declarations and produces a kernel
ChangeSet. Transactional construction creates a fresh immutable revision before
the workspace advances its head. Failure leaves the head unchanged. Earlier
revision handles retain their documents, graph, query context and diagnostics.

A Working revision may contain unresolved references and incomplete producer
evidence. Validation is a checked transition requiring strict kernel validity,
mandatory-reference completeness and the applicable effective-query certificates.
A `ValidatedProjectRevision` has a private constructor and retains the exact
validated working revision. A successful source edit or stored snapshot alone
does not create this type.

The initial operations are open, add KerML/SysML document, edit, remove, obtain
head, read diagnostics and query. The query facade exposes the shared ModelView
and borrowed language evaluators without copying inherited members or extending
syntax lifetimes into canonical storage. Declared, current-graph and effective
answers retain their distinct contracts.

## Acceptance fixture

Use the rich authored Vehicle/SportsCar vertical. Revision 1 contains nested
parts, attributes, inherited ports, connections, redefinition, action/state and
requirement/constraint structure. Editing Vehicle creates revision 2; revision 1
remains readable and unchanged. Standard ElementIds and publication Arcs remain
identical. Unchanged authored identities are preserved where syntax reconciliation
permits. An unresolved edit produces Working, and cannot produce Validated.

No platform production code may integrate before the language-foundation
readiness summary passes. This proposal does not claim that gate passed.
