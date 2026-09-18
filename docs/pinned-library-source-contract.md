# Exact pinned library source loading

`agq-standard-libraries` loads the four exact archives compiled from
`standards/normative/sysml-2.0/library-set.json`. It verifies the set digest,
archive SHA-256/size, complete entry inventory and every entry SHA-256/size.
No network access, archive extraction or byte repair is performed. Decompression
is bounded by each pinned entry size. Duplicate, unexpected and unsafe entry
paths fail. The byte provider can read local archives or supply bytes directly.

The loader checks `.project.json` against the pin and `.meta.json` against the
metamodel identity, then traverses all exact-version usage dependencies with
visited state. Every dependency's bytes are verified even along cycles. Unknown,
missing or wrong-content dependencies fail the entire load. The Systems metadata
index's absent `AnalysisCase.sysml` and the actual unindexed `AnalysisCases.sysml`
remain two diagnostics; both `.DS_Store` entries retain inventory evidence.
Original archives are unchanged.

All four `.project.json`/`.meta.json` pairs were inspected. They provide project
names/versions, dependency URIs, creation dates, metamodel URIs and name-to-file
indexes, but no authoritative semantic element identities. The textual archive
documents likewise have no element-identity sidecars. The following identity
scheme is explicitly private Agentique policy for exact immutable library bytes.

UUID-v5 uses namespace `2abdcfe0-9071-4db6-aa6b-ff4bbd55f188` and compact UTF-8
JSON arrays:

* LibraryId: `["agentique-pinned-library/1", specification, version, artifactURI,
  archiveSHA256, metamodelURI]`.
* DocumentId: `["agentique-library-document/1", libraryUUID, entryPath, entrySHA256]`.
* SourceRevisionId: `["agentique-library-source-revision/1", documentUUID, entrySHA256]`.
* ElementId allocation: `["agentique-library-element/1", documentUUID, entrySHA256,
  startByte, endByte, role, ordinal]`. Roles distinguish declarations, owning
  memberships, reference relationships, annotations and expressions. Only
  relationships use a nonzero ordinal. UTF-8 ranges must be within the verified
  immutable document.

The locator is safe only because every identity contains immutable artifact and
entry content. It is never used for authored documents. Allocating an identity
does not prove that a declaration exists at that range. A future library lowerer
must derive ranges and roles from actual accepted syntax and attach the exact
`DeclaredOrigin::StandardLibrary` supplied by the document. Library facts are
declared facts, not derived merely because loading was automatic.

This crate currently supplies verified source documents and identities. It does
not yet lower the complete libraries into canonical kernel models, certify their
references, or produce deterministic semantic contexts. Those Gate 2 acceptance
items remain coupled to Gates 4, 6, 11 and 12 and are explicitly incomplete.

The `audit` example scans all 57 original textual entries with exact token
partitions and records current frontend gaps. Its inventory distinguishes lexical
occurrences from grammar productions. SysML receives only a KerML lexical/parser
probe, explicitly labelled as such. No recovered package body is interpreted as
a loaded library. The quality report uses null for reference counts that were
not evaluated, and its `--require-semantic` command intentionally fails until
canonical library ingestion and validation exist.
