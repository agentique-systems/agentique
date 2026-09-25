# Checked borrowed visibility

This is a visual CPU optimization, independent of semantic candidate reconstruction.
It changes no language semantics, accepted publication identity or runtime contract.

## Source of work

The existing native path queried spatial bounds into owned `SceneTarget` values,
then resolved those identities through `SceneLookup` back to positions in the same
scene. Every visible edge name was sorted, cloned, hashed for lookup, compared for
identity and hashed again for the presentation batch key. Each route segment also
owned a separate copy of its relationship identity in the index.

The optimized spatial index stores one identity and original scene position per
object. Segment hits refer to this table by integer slot. Since the grid returns
items in insertion order, repeated segments of one edge can be skipped in one
pass. `SpatialIndex::visible_scene` checks the scene revision and exact identity
at each stored position before borrowing a record. It returns nodes, ports and
edges in the original scene order, preserving containment drawing order.

`VisibleScene` hashes borrowed identities for the renderer's existing batch key.
This is an identity hash, not a geometry or revision fingerprint: the native
renderer must continue including its scene generation and rendering options in
the full batch key. The spatial index still must be rebuilt after layout,
projection replacement or diff application. Identity checks prevent stale slots
from borrowing unrelated objects; they do not authorize reusing stale geometry.

The existing owned-target query, hit testing and marquee APIs remain available.
Edges are culled by individual routed segments, retaining edges that cross the
viewport while both endpoints are outside it. Scene records are never copied.

## Verification and measurement boundaries

`tests/visibility.rs` compares the borrowed path with both an independent full
scene scan and the existing public query/lookup API, across hierarchy and graph
layouts, adversarial fixtures, multiple moving viewports, collapsed proxy ports,
removed diff ghosts, huge query bounds and offscreen endpoints. It also checks
wrong revisions, reordered or removed records, and the identity hash.

`examples/visibility_benchmark.rs` uses real public scene APIs with explicitly
synthetic 1k/2k and 10k/20k fixtures. Fit, pan and zoom traces retain 120 samples
after warmup. Timing ends after culling, checked borrowed record lookup and batch
identity hashing, before final destruction of the returned record vectors. The
before and after boundaries are identical. This does not measure rendering,
input latency, GPU duration, presentation or semantic work.

The baseline is the committed parent visibility implementation at `96b9de4`,
included in this worktree by `c8f26b4`. Both builds use release optimization with
LTO and debug symbols disabled. Concurrent accepted-runtime rematerialization
and other native work limit comparisons between separate runs; the final example
also compares both APIs in the same process with alternating execution order.

The original harness is retained in `visibility-baseline.rs.txt`. Restore it as
`crates/studio-scene/examples/visibility_benchmark.rs` on the baseline commit to
reproduce that source configuration. The baseline scene rlib finished before the
source change and had SHA-256
`dc251c87cace6ba0fdc990bd0228ae555f12246108463b1625048db195ed9fcb`.

## Observed results

Milliseconds, CPU visibility plus identity hashing. These are microbenchmark
trace names, not native pan/zoom frame times. Fit covers the entire scene; pan
uses 85% of each scene dimension and zoom uses 30%, moving each viewport through
120 positions. Both paired paths assert exact visible identities and original
draw order at every position, outside the measured intervals.

| Fixture | Trace | Original median / p95 | Paired owned API median / p95 | Paired borrowed API median / p95 |
| --- | --- | ---: | ---: | ---: |
| 1k nodes / 2k edges | Fit | 0.704 / 0.823 | 0.428 / 0.678 | 0.209 / 0.325 |
| 1k nodes / 2k edges | Pan | 0.473 / 0.656 | 0.340 / 0.533 | 0.169 / 0.260 |
| 1k nodes / 2k edges | Zoom | 0.052 / 0.087 | 0.050 / 0.089 | 0.026 / 0.044 |
| 10k nodes / 20k edges | Fit | 9.425 / 12.021 | 7.059 / 9.604 | 1.774 / 2.906 |
| 10k nodes / 20k edges | Pan | 6.220 / 8.137 | 5.290 / 6.841 | 1.174 / 2.295 |
| 10k nodes / 20k edges | Zoom | 0.505 / 0.765 | 0.477 / 0.711 | 0.236 / 0.351 |

The paired 10k fit and pan medians fall by 75% and 78%, respectively. The paired
owned API uses the new shared identity storage too; it isolates the additional
benefit of avoiding owned-target sorting, cloning and reverse lookup. The
original implementation's 10k fit median identity resolution alone was 4.818 ms.
The borrowed path's combined 10k fit culling and resolution median was 1.366 ms.
Identity hashing is retained and measured, not omitted to make the number smaller.

Spatial index construction was observed at 21.090 ms before and 17.560 ms after
for 10k. That is one build observation per run, under differing concurrent load,
not a distribution or a strong isolated construction-speed claim. Peak memory,
GPU time, input-to-frame latency and real-model timings were not measured here.
Native frame performance must be measured after viewport integration; no 60 Hz
acceptance or semantic-edit speedup follows from these numbers alone.

## Check receipts

- `checks/visibility-before.json` and `.txt`: original release benchmark, exit 0.
- `checks/visibility-after.json` and `.txt`: paired release benchmark, exit 0.
- `checks/visibility-tests.json` and `.txt`: all scene targets, 35 tests pass.
- `checks/visibility-fmt.json` and `.txt`: workspace formatting, exit 0.
- `checks/visibility-clippy.json` and `.txt`: focused release Clippy with warnings denied, exit 0.

Each JSON receipt records the actual command, working directory, start time,
elapsed duration, exit code and SHA-256 of its retained output. Other workspace
and native integration checks remain the integration lead's responsibility.
