# Runtime authentication diagnosis and unmeasured optimization draft

The real pre-optimization profile is recorded in
`../read-profile-01/measurement.json` and `../read-profile-01/README.md`.
KerML restoration took 37.242 s and Systems restoration took 38.716 s.
This is authentication and reconstruction of accepted publications, not producer
replay. The subsequent project restoration is a separate 116.734 s boundary.

The installed runtime ZIP entry headers were read with Python 3.12 `zipfile` from
`C:/Users/phili/.agentique/publications/633ea89eb39f8a9e301f2bd5199994455cdf28c8d373fdbd69be30a1402ebcf4`.
The command iterated `ZipFile(path).infolist()` and printed each entry's filename,
file_size and compress_size; exit code 0. This inspected metadata only, without
running concurrent semantic work.

| Archive entry | Uncompressed bytes | Compressed bytes |
| --- | ---: | ---: |
| KerML facade.json | 78,187,642 | 6,351,249 |
| KerML kernel.jsonl | 2,176,394,780 | 469,514,042 |
| Systems facade.json | 19,746,846 | 1,790,473 |
| Systems closure.json | 6,759,870 | 1,469,723 |
| Systems kernel.jsonl | 748,560,767 | 131,062,194 |

Both restorers authenticate the uncompressed graph stream, decompress it again
for decoding, and reserialize the decoded graph to compare its canonical archive
digest with the independently trusted receipt. Systems dependent-archive header
validation also serializes the entire KerML dependency, including selected
contributions for the evidence format. Recanonicalizing Systems repeats that
same dependency serialization. Systems then constructs producer-aware query
contexts before and after validating its standard bindings.

The draft memoizes only the exact dependency archive digest in the immutable
`DerivedOverlay` allocation. Legacy and evidence formats use separate cells.
Clones share the exact graph and cells. Every newly built or restored overlay has
empty cells, including builders that move storage from a consumed predecessor.
Errors are not cached. There is no public setter or caller-supplied digest, and
no authentication, canonicalization, context, certificate or semantic check was
removed. The new regression covers different archive formats, optional evidence
lost by legacy restoration, repeated clones, changed overlays with both shared
and consumed predecessor storage, and rejection of the wrong dependency.

`AGENTIQUE_RUNTIME_RESTORE_TRACE=1` now opts into individual restore phases on
stderr. `RUNTIME_RESTORE_PHASE` reports disjoint outer phases per publication;
KerML `RUNTIME_RESTORE_SUBPHASE` lines break down its outer canonicalization and
context-authentication phase and must not be added again to the outer total.

Combining input hashing with decoding could remove one decompression pass while
still hashing the exact consumed bytes through EOF. Removing decoded-graph
recanonicalization is not part of this draft: the semantic model digest omits
selected contribution metadata and retired identity reservations that the archive
binds. The public graph-taking KerML restoration API must authenticate the graph
it receives rather than trusting a digest supplied by its caller.

Validation so far: targeted `rustfmt --check --edition 2024 --config
skip_children=true` over the nine modified/new Rust files returned exit code 0;
`git diff --check` returned exit code 0. After disk space became available, the
integration lead ran all 14 kernel archive tests, including the new digest-cache
regression; all passed. The four closed-audit reuse tests and two immutable
producer-row/rebind tests also passed. Exact commands, output and exit codes are
in `../../native-studio-alpha/checks/acceptance-kernel-archive-tests.*`,
`acceptance-closed-audit-tests.*`, `acceptance-immutable-family-tests.*` and
`acceptance-immutable-rebind-tests.*`. Real before/after timing remains pending.
This document claims no measured runtime speedup yet.

## Measured single-pass graph input opportunity

`read-profile-02/measurement.log` separates current graph restoration costs:

| Phase | KerML | SysML |
| --- | ---: | ---: |
| Input graph hash/read/decompression | 4.255 s | 1.371 s |
| Graph decode and kernel/dependency validation | 19.222 s | 17.629 s |
| Decoded graph canonical re-encoding/authentication | 7.353 s | 3.726 s |
| Semantic context setup | 2.255 s | 2.865 s initial + 2.878 s bound |

The checked-in receipts pin 2,176,394,780 graph bytes for KerML and 748,560,767 for Systems, totaling 2,924,955,547 uncompressed bytes. `library/publication_cache.rs` and `sysml/publication_restore.rs` currently decompress and hash that material, reopen the graph entry, then decompress it again for the kernel archive reader. A digesting reader around the actual decoder input could remove the duplicate read/decompression. The 5.626 s input-authentication phase is an upper bound on the removable work, not a prediction: SHA-256 still needs to run during decoding. The much larger 36.851 s decoding/validation and 11.079 s re-encoding costs remain.

A bounded patch could stay inside the text crate: one private counting/digesting `Read` adapter plus the two restoration call sites. It must check ZIP entry size against the independently trusted receipt, use a checked `expected + 1` read bound, retain actual byte count and observed EOF, reject short/excess input and I/O/CRC failures, and verify the exact raw input digest before creating any accepted publication facade. `Read` on an empty buffer must not masquerade as EOF. The kernel reader already rejects trailing content after its End record using `fill_buf`; successful decoding therefore must have observed actual EOF, not merely stopped at a syntactically complete prefix. The helper should not drain unparsed trailing content and silently accept it.

The decoded graph re-encoding, dependency authentication, source/facade checks, context binding and closure receipt checks must remain untouched. They authenticate the actual reconstructed immutable graph and protect the direct accepted-restore API from caller-supplied digest claims, parser normalization or a changed seekable input. Combining stream authentication with parsing moves graph-record allocation before the final digest decision; it does not grant semantic authority before that decision. Trusted total-byte limits and the kernel's 64 MiB per-entry line bound remain necessary. No production change was made for this review.

Required regressions for such a patch: valid identical roundtrip under short/irregular reads, wrong digest, short and excess streams, zero-length read calls, missing EOF, trailing JSON/whitespace, corruption/CRC error, changed seekable input, and unknown/duplicate graph fields still failing the unchanged receipt/recanonicalization boundary. The real authenticated runtime profile and exact independent command/cold oracle must quantify any actual gain after integration.
