# Generation-2 SQLite repository adapter

This adapter persists the versioned source-backed repository contract. It does
not deserialize Generation-1 models and does not treat database rows as a
canonical semantic graph.

Each connection enables foreign keys, WAL and `synchronous=FULL`. Schema
initialization and writers take SQLite's immediate transaction lock. Independent
processes may open the same database; SQLite serializes writes, while the
repository's explicit expected-head comparison determines which candidate wins.
An application ID, exact schema version and repository format distinguish this
database from Generation 1 and refuse unsupported formats. Opening an
unsupported database does not rewrite its version.

A commit writes content-addressed source blobs, the immutable revision manifest,
document references, the branch head and an operation receipt in one transaction.
Acknowledgement follows transaction commit. An error before commit rolls back
all writes. A lost acknowledgement after commit has an uncertain outcome for
the caller; an exact retry uses the durable operation receipt. Reusing an
operation identity for a different request is rejected.

Branches are named references to existing same-project revisions. Deleting a
branch retains every revision and source blob. Garbage collection and automatic
merge are intentionally outside this adapter.

Source blobs are stored in SQLite because authored text is small and doing so
makes ownership and atomic durability straightforward. Semantic caches are
optional opaque accelerators. This milestone does not establish a large-graph
cache performance threshold; their storage layout can change under a future
versioned adapter format. Cache checksum failure permits source reconstruction,
whereas a manifest or source checksum failure rejects repository truth.

The offline integrity API checks storage structure and source authentication.
Publication availability and validation receipts require the separate service
semantic authentication step; a successful SQLite integrity check alone never
issues a Validated revision.
