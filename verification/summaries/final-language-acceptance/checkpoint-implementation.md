# Unaccepted publication frontier continuation

This implements a separate kernel archive format for strict and construction
producer frontiers. Existing accepted publication archive bytes are unchanged.
Frontier archives preserve declared/derived separation, proof/search metadata,
ordered-reference contributions, protected dependency identity and lower-bound
construction obligations. Restoring a construction archive never creates a
strict snapshot or accepted publication.

`PublicationFrontierSession` stores completed Structural/StableProperties strata
and completed scheduler invocations. Optional contextual intervals are explicit;
the default interval of zero does not serialize every round or subject. Graphs
are compressed local deltas over the separately supplied immutable dependency.
The journal retains earlier invocation finals so source/reference reconstruction
can recover completed producer work without replaying it. It stores the current
worklist, original evaluation rows, transport reads, certificate, statuses,
dependency index, deferred binding population, stratum and round budget.

Each journal is immutable and externally SHA-256 pinned. ZIP state and graph
entries have separate hashes checked over the actual decoded byte stream. Source
bytes/profile/selected-path mode/accepted dependency are independently pinned by
the frontend. Each invocation also checks its original graph, semantic contract,
registry, options and extension stratum mode. Restored graph and certificate
identities must match. A checkpoint can only continue the ordinary scheduler;
the normal strict publication acceptance gates remain mandatory.

New immutable objects are synchronized before a new immutable journal is written
and synchronized. Only then does the session advance its current journal and emit
the externally retainable path/hash pin. Failure leaves the previous pin intact.
No checkpoint object is accepted publication authority.

Initial compiler stage gate (Windows PowerShell, single job, shared low-artifact
target `target/bridge-self-model`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_INCREMENTAL=0`):

| Command | Exit | Actual output/result |
| --- | --- | --- |
| `cargo check -p agq-kernel` | 1 | E0451: construction overlay `inner` private; fixed sibling-module visibility |
| `cargo check -p agq-kernel` | 0 | Finished dev profile in 1.57s |
| `cargo check -p agq-kerml-text` | 1 | Diagnostic deserialize lifetime required static; fixed owned wire decoding with bounded code interning |
| `cargo check -p agq-kerml-text` | 1 | E0308/E0282: source identity mixed digest bytes and string array; fixed framed byte encoding |
| `cargo check -p agq-kerml-text` | 0 | Finished dev profile in 1.00s |
| `cargo fmt --all` | 0 | No output |

Focused equivalence and tamper stage gates now pass with the integrated
incremental certificate builder and round instrumentation. The four scheduler
tests include externally interrupting actual Structural-to-ContextualBindings
work, restoring it, and comparing canonical records, ordered contributions,
proof/search sets, query answers, certificate rows/read transport and digests.
Both strict and construction frontiers are exercised. A two-invocation session
restores its earlier completed invocation without producer replay, then resumes
the unfinished construction invocation. Changed source pins, initial graphs,
semantic contexts, journal bytes and archive bytes are rejected.

The kernel archive group passes 9 tests, including exact frontier roundtrips,
retained construction deficits/protected dependency sharing, legacy archive byte
compatibility, and missing/cyclic selected-contribution evidence rejection.
All 38 active producer-worklist regressions pass; the two explicitly ignored
scale probes are unchanged. The four checkpoint tests also pass with
`AGQ_CERTIFICATE_VERIFY_FULL_REBUILD=1`, comparing incremental and reference
certificate issuance throughout uninterrupted and resumed computations.

Checkpoint serialization streams logical read rows from borrowed storage and
retains only a unique-atom lookup, rather than copying every row into index
vectors. The transport digest streams atom hashes. Resume releases the obsolete
initial overlay before decoding the saved successor. The journal uses a synced
temporary file followed by rename; partial writes cannot occupy its final
content-addressed pathname. Existing graph/journal objects are authenticated
before reuse. Decoded-stream pins prevent a hash-then-reopen race from changing
the restored state or graph.

Actual focused command results are in `checkpoint-commands.json`. Medium
uninterrupted/resumed corpus equivalence and full Systems acceptance remain lead
integration gates; this record does not assert either outcome.
