# Visible-scene traversal audit

Implementation commit: `f32f368304b96d262c7afc4c56117d5e7906c40b`.
Same Windows / Rust 1.92.0 / Ryzen 5 5600X machine as `scene-summary.md`.

Read-only review of the native renderer found that spatial culling selected a
small visible set, but label drawing and GPU batch preparation still scanned
every scene record. Incident-edge highlighting additionally scanned every port
for each visible edge. These operations can be independent of total scene size.

`SceneLookup` retains identity-to-array-index maps, plus a port-to-owner map.
`visible(&scene, &targets)` resolves culled targets into borrowed records in their
original draw order. Node/container aliases deduplicate. The index owns no second
copy of model or scene records. Direct identity lookup is expected O(1); visible
ordering is O(V log V). Rebuild it after scene replacement or applying a diff.
Identity/revision checks prevent stale indices returning unrelated records.

Actual commands: `cargo clippy -p agq-studio-scene --all-targets -- -D warnings`
(exit 0), `cargo test -p agq-studio-scene` (exit 0, 21 tests passed), and
`cargo run --release --config profile.release.lto=false -p agq-studio-scene
--example scene_benchmark` (exit 0).

| Measured operation | 1k nodes / 2k edges | 10k nodes / 20k edges |
| --- | ---: | ---: |
| Visible targets in fixed viewport | 96 | 84 |
| Previous full-scene visibility filter | 152.60 us | 1,686.73 us |
| Indexed visible record resolution | 4.70 us | 4.24 us |
| One-time scene lookup construction | 0.18 ms | 3.32 ms |

The reference filter reproduces the previous renderer's node/port/edge identity
membership checks, including edge identity string construction. It performs 100
passes. Indexed lookup performs 1,000 passes. These measure CPU traversal only;
they are not a GPU frame or end-to-end interaction latency claim. Concurrent
compiler workload varied from the previous scene benchmark, so cross-run scene
build timing differences must not be interpreted as a separate optimization.

The retained four-pass instancing architecture and camera uniform remain sound.
The next bounded renderer concern is CPU dash tessellation: a long derived edge
can allocate many dash quads even when only a small section crosses the viewport.
A shader dash mask can retain one quad per segment; viewport clipping and solid
low-LOD edges are other bounded alternatives. Text is already gated by LOD and
can reuse toolkit glyph/layout caches during pan. Report buffer preparation time
as CPU upload work; it does not measure GPU completion.
