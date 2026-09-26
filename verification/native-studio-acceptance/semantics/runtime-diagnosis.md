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
`git diff --check` returned exit code 0. Compiled tests and before/after timing
remain pending. Local builds were deliberately not started while the real profile
ran and C: had approximately 20 MB free. This document claims no measured runtime
speedup yet.
