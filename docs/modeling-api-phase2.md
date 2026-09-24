# Generation-2 Systems Modeling API mapping

Authority is the checked-in Systems Modeling API and Services 1.0 PDF
`SysAPI.pdf` (formal/2026-03-04), `standards/artifacts/OpenAPI.json`
(ptc/25-02-30), and `standards/artifacts/Schema.json` (ptc/25-02-31).
No Gen1 DTO, Store, revision or model determines this adapter's semantics.

`node tools/gen2-api-inventory.mjs --check` verifies the deterministic inventory
and authority hashes. `standards/gen2-api-mapping.json` records explicit support
decisions; the generated `standards/gen2-api-coverage.json` contains all 35
operation IDs, exact paths, parameters, request/response schemas and decisions.
The generator checks every shared schema shape against both machine-readable
artifacts, accounting for their different reference URI spellings. Generation-1
coverage remains historical and independent.

## Repository mapping

PDF 7.1.2 and 7.2.3 map Project to a repository project, Branch to a mutable
revision reference, and Commit to an immutable ProjectRevision. A branch's
`head` redefines `referencedCommit`; they therefore identify the same commit.
`previousCommit` contains zero or one predecessor. Branch creation references
an existing revision without copying a graph. Creation timestamps are durable
resource metadata, not regenerated on read. A branch name is not a UUID.

The pinned Commit schema's `owningProject` reference comment says Branch, while
the property name and PDF 7.1.2 say Project. The adapter follows the PDF ownership
contract and returns a Project reference. The JSON reference shape is unchanged.

ProjectRequest's name and optional description map directly to create-project
with an initial revision and default branch. Supplied unsupported options must
be rejected explicitly; they must not be silently ignored. BranchRequest's
non-null head maps directly to create-at-revision. Null heads are outside this
repository profile because durable heads always resolve to existing revisions.

## Semantic projections

Element `@id` remains the canonical ElementId and `@type` retains its concrete
metaclass. It must never be widened to Element to pretend the smaller Element
schema is satisfied. The normative concrete schemas require all inherited
properties; a partial projection must retain partial coverage and explicitly
identify that limitation. Missing derived/effective properties are never
fabricated as empty arrays or null values. DTOs remain projections of the
revision-bound service's owned typed values, never a mutable parallel graph.

Diff uses canonical identity. PDF 7.2.3 defines DataDifference as a pair of
DataVersions: baseData is absent for additions and compareData is absent for
removals. Document/source changes and validation changes remain service-level
information; they are not fabricated as standard Element payload mutations.

## Unsupported mutation and merge

POST commit's normative CommitRequest contains DataVersion payload mutations.
That contract does not equal ApplyDocumentChanges, which edits exact source and
rebuilds canonical semantics. POST commit is explicitly unsupported. Source
commands remain behind the modeling service until a reviewed provenance and
source reconciliation contract exists.

Automatic merge is explicitly unsupported. A subsequent contract must define
common-ancestor selection, DocumentId and SyntaxNodeId reconciliation, rename
and delete/edit conflicts, source authority, canonical ElementId preservation,
validation receipts, and atomic multi-parent commit semantics. A textual merge
alone cannot establish a valid semantic merge.

## Transport and pagination

The new route boundary is `/api/gen2`; it does not redirect or replace the Gen1
browser server. HTTP depends on the API mapping and service. Neither the kernel,
language crates nor modeling workspace depends on HTTP.

PDF 8.1 pagination uses JSON array bodies, `page[size]`, `page[after]`,
`page[before]`, and Link response headers. Immutable semantic cursors bind
project, resolved ProjectRevisionId, operation, filters and page size. Branch
resolution occurs before the first query; a continuation carries the original
revision and cannot silently read a newer branch head. The cursor checksum
detects corruption and is not an authorization credential. Repository access
must be authorized separately by an embedding host.

## Following platform work

Phase 3 can add views/projections, diagram identity and source-backed modeling
commands, richer property-complete API DTOs, project usages, query operations,
tags, reviewed merge/reconciliation and separate repository retention policies.
Execution and simulation are outside this platform milestone.
