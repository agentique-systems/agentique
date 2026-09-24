# ADR 0024: generation-2 in-memory modeling workspace

Status: proposed pending workspace runtime acceptance. The implementation is
integrated after the [language-foundation gate](../../verification/summaries/final-language-acceptance/semantic-closure-readiness.md)
passed and ADR 0026 was adopted. Integration does not establish completion of
the workspace's own acceptance suite.

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
KerML dependencies. The accepted-SysML constructor also mounts the
authenticated Systems dependency and supplies effective language contexts.
The additive `SourceInputs`/`SourceCompilation` boundary retains recovered inputs,
construction obligations and unresolved endpoints without changing the existing
strict `SourceProject` API. Kernel `DeclaredConstructionHistory` carries authored
identity reservations through Working states. The
[frontend boundary review](../modeling-workspace-frontend-boundary.md) records the
design and the strict frontend's original limitations.

## Proposed decision

The integrated `agq-modeling-workspace` follows the accepted KerML v9 and Systems
v3 publications and the passing effective authored language vertical. Keep
`agq-workspace` operational and unchanged. Actual workspace runtime acceptance
remains pending at reviewed source `5bdbc60`.
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

`ProjectRevisionId` is a distinct opaque project-history type. Its fresh identity
does not reuse the kernel revision of a semantic snapshot; it exists for Working
states too. `ProjectId`, `DocumentId`, `SourceRevisionId`, `SyntaxNodeId`, kernel
`RevisionId` and canonical `ElementId` retain their separate meanings. Paths and
qualified names locate declarations; neither owns their identity.

Commands carry an expected head revision and document changes. The frontend
reconciles syntax identities, lowers canonical declarations and produces a kernel
ChangeSet. Transactional construction creates a fresh immutable revision before
the workspace advances its head. Failure leaves the head unchanged. Earlier
revision handles retain their documents, graph, query context and diagnostics.

A Working revision may contain unresolved references and incomplete producer
evidence. It binds exact document versions and syntax to the current construction
or strict declared graph, derived frontier, diagnostics, reference results,
closure certificate and authenticated standard dependencies. Validation is a
checked transition requiring strict kernel validity, mandatory-reference
completeness, matching canonical endpoints, no blocking diagnostics, authenticated
KerML and SysML contexts, fully closed producer certificates and a finding-free
applicable effective-query audit bound to the exact SysML semantic context.
The retained audit covers every local canonical subject, including derived
records, through the strict finalizer's effective-query dispatcher. It excludes
accepted dependency subjects and issues no standard publication. Missing or
mismatched audit context and Invalid/Incomplete effective results block validation.
A `ValidatedProjectRevision` has a private constructor and retains the exact
validated working revision. A successful source edit or stored snapshot alone
does not create this type.

The initial operations are open, add KerML/SysML document, edit, remove, obtain
head, read diagnostics and query. The query facade exposes the shared ModelView
and borrowed language evaluators without copying inherited members or extending
syntax lifetimes into canonical storage. Declared, current-graph and effective
answers retain their distinct contracts.

The implementation and acceptance matrix are in
[the phase-1 design](../modeling-workspace-phase1-design.md). Working results use
the additive frontend construction carrier; they do not relabel a previous strict
snapshot as the current malformed edit.
An accepted document operation advances the Working head even when language
validation fails; operational failures such as a stale base or invalid edit span
leave the head unchanged. Neither outcome mutates a previously validated handle.

## Acceptance fixture

Use the Agentique self-model and the rich authored modeling-server vertical.
Revision 1 contains nested parts, attributes, inherited ports, connections,
redefinition, action/state and requirement/constraint structure. Editing the
workspace subsystem creates revision 2; revision 1
remains readable and unchanged. Standard ElementIds and publication Arcs remain
identical. Unchanged authored identities are preserved where syntax reconciliation
permits. An unresolved edit produces Working, and cannot produce Validated.

The permanent 12-test workspace suite has six phase-1 tests, one self-model test
and five Working-state tests. Its validated scale fixture requires 100 mixed
documents in each of five Validated revisions and four concurrent immutable
readers. The separate recovery-scale fixture retains a fourth Working revision
with 99 documents; it does not substitute for the validated scale gate.
Verification-only observers
check actual standard table/index/proof storage and scheduler evaluations;
publication Arc equality alone does not establish absence of graph copies or
producer replay. Accepted-cache tests are explicit, fail when their dependencies
are unavailable, and never acquire or build a substitute publication.

## Ownership and future adapters

The workspace owns authored project history; language crates own meaning. Query
contexts borrow a particular immutable revision. Old revisions never observe
new edits, and no ordinary editing API exposes arbitrary graph mutation. Each
revision retains complete canonical relationships and declared/derived provenance;
workspace DTOs, views and future runtime IR are separate representations.

Future persistence stores and reconstructs exact revisions and their dependencies.
It does not become semantic truth. A future repository/application adapter must
finish its durable transaction before updating an externally acknowledged head.
This in-memory crate promises atomic visibility, not durable storage. Branches,
repository services and Systems Modeling API adapters belong to the
[Phase 2 roadmap](../modeling-platform-phase2-roadmap.md).

Standard publication is rare and cached. Initial authored construction is a
separate cost; Phase 1 may recompute authored semantics on edits while sharing
standards. Future provider-footprint invalidation must remain equivalent to full
authored recomputation. ADR 0027 component sealing is not a prerequisite.

The language-foundation prerequisite passed before integration. Kernel invariants
and exact accepted-cache identity restoration on the shared-storage representation
have also passed; see the [current command ledger](../../verification/summaries/final-audit-semantic-closure/commands.json).
Workspace self-model, Working/Validated and scale runtime acceptance remain
separate pending gates. ADR adoption awaits those results.
