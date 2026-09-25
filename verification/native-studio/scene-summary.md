# Native scene foundation evidence

Scene source: `768bd23160b9fd1a5b9891e839ba68abb3294c3b`, following
`72591ef` and `cad73ad`. Windows x86_64, Rust 1.92.0, AMD Ryzen 5 5600X
(6 cores / 12 logical processors). All fixtures are explicitly visual evidence;
none authenticates standards, establishes semantic acceptance or commits a model.

| Actual command | Exit | Observed result |
| --- | ---: | --- |
| `cargo check -p agq-studio-scene` | 0 | Public scene adapter compiled |
| `cargo fmt --all -- --check` | 0 | Formatting passed after local fixes |
| `cargo clippy -p agq-studio-scene --all-targets -- -D warnings` | 0 | All scene targets clean |
| `cargo test -p agq-studio-scene` | 0 | 19 integration tests passed; 0 failed |
| `cargo doc -p agq-studio-scene --no-deps` | 0 | Public crate documentation generated |
| `cargo run -p agq-studio-scene --example scene_benchmark` | 0 | Development-profile baseline recorded |
| `cargo run --release --config profile.release.lto=false -p agq-studio-scene --example scene_benchmark` | 0 | Optimized 1k / 10k fixtures completed |

Raw command outputs and exit codes are local generated evidence in
`verification/generated/native-studio/scene-checks.json`, following ADR 0021.
During development, Clippy caught a collapsible conditional and a default-field
reassignment in a test; both were fixed. A formatting gate caught unformatted
edits and subsequently passed. The final tested implementation has no suppressed
lint or ignored correctness test. The 19-test run includes the final one-hop
neighborhood addition; preceding raw captures retain the earlier 18-test result.

## Retained optimized measurements

These are one-run measurements under concurrent development workloads, not
statistical performance gates. Full numeric results are retained in
`scene-performance.json`; the checked-in benchmark reproduces the measurement.

| Metric | 1,000 nodes / 2,000 edges | 10,000 nodes / 20,000 edges |
| --- | ---: | ---: |
| Projection adapter | 1.05 ms | 16.49 ms |
| Layout | 1.62 ms | 27.74 ms |
| Scene including layout + routing | 23.61 ms | 200.71 ms |
| Spatial index construction | 2.21 ms | 28.32 ms |
| Hit test, mean of 10,000 probes | 1.13 us | 2.37 us |
| Viewport culling, mean of 100 probes | 50.23 us | 73.81 us |
| Reported obstructed routes | 0 | 0 |

Camera pan/zoom arithmetic averaged about 0.005 us in the optimized loop. This
does **not** establish input-to-frame latency. GPU upload, GPU frame duration and
steady rendering FPS belong to the native renderer's separate measurements.
Cold release compilation took 8m03s, chiefly generated language crate codegen;
the subsequent scene-only rebuild and benchmark completed in under 10 seconds.

## Verified contracts

Tests exercise mixed-revision and duplicate-identity rejection, iterative
ownership-cycle rejection, containment, sibling separation, stable local edits,
collapse/expand restoration, exact semantic port attachment, pointer-anchored
zoom, LOD hysteresis, spatial hits/marquee/culling (including crossing edges),
large-coordinate overflow safety, revision-bound removed ghosts, exact category
mapping, topology cycles, distinct parallel lanes, obstacle detours, self loops
and deterministic one-hop expansion resolving feature ports to original owners.

System World uses hierarchy layout. Graph and Requirements use a separate
SCC-condensed layered topology layout over the same DTO/scene engine. A bounded
orthogonal router exposes `Obstructed` if it cannot find a clear corridor;
arbitrary dense scenes are not claimed to have globally optimal routing.

Presentation category mapping is exact metaclass-name matching at one adapter,
with an Unknown fallback. Direction remains unspecified unless the public view
contract carries authoritative port direction. Collapsed/omitted endpoints are
omitted instead of silently reattached to another semantic element. Layout,
camera, overlays and comparison marks never enter canonical model storage.
