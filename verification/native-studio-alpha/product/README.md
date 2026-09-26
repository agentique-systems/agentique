# Product / scene stream evidence

All captures and layout scenarios in this directory are **visual fixtures**, not
accepted runtime or real Agentique semantic acceptance.

`round-01-before.png` is the actual 1600 × 1000 native wgpu surface captured with
the previous main binary. Review: containment is coherent, but anonymous ports,
unlabeled selected paths, repetitive zero counters and uniformly weighted chrome
keep the world closer to a graph tool than an engineering explanation. Exact
baseline metrics are in `baseline-metrics.json`; this 90-frame capture is not a
steady-state performance qualification.

The first rendering batch addresses those critiques using vector category
marks, selected path labels, selected port names, visible connection indicators,
quieter unrelated edges, containment headers, and nonzero engineering summaries.
Updated gallery captures and independent critiques are owned by the integration
stream, so they reflect the combined application rather than a private build.

The candidate screenshot in the integration baseline revealed overlapping
containers. Its root cause was `apply_diff` unioning old and current absolute
bounds after collision avoidance moved a subsystem. The scene batch compares
extents in the current owner frame, reserves existing containment space before
packing neighbors, moves removed ghosts with owners, and reroutes their edges.
`candidate_diff_does_not_union_old_absolute_container_positions` reproduces the
four-to-five-child layout change and requires containment and non-overlap.

Collapsed subsystems now expose original external port identities at their
boundary. `proxy_for_owner` explicitly retains the original semantic owner;
direction remains unspecified. Only actual external Connection endpoints become
proxies. The proxy does not create a semantic port or imply compatibility.

`layout-quality.txt` records nine adversarial fixture classes in both layouts:
wide fan-out, deep hierarchy, many ports, dense cross-links, parallel edges, long
Unicode names, mixed requirements, candidate growth and cycles. It measures
unchanged node **top-left** displacement in world units; container size growth is
not translation. Debug-build timings under concurrent work are diagnostic, not
release performance claims. Obstructed routes remain explicit warnings.

## Commands and results

Working directory: the isolated `agentique-alpha-product` worktree.

| Command | Output | Exit |
|---|---|---|
| `../agentique/target/release/agq-studio-native.exe --fixture architecture --no-restore --screenshot verification/native-studio-alpha/product/round-01-before.png --frames 90 --metrics verification/native-studio-alpha/product/baseline-metrics.json` | actual native screenshot; metrics JSON | 0 |
| `cargo fmt --manifest-path crates/studio-native/Cargo.toml --all` | no output | 0 |
| `cargo fmt --all` | no output | 0 |
| `cargo test --locked --offline -p agq-studio-scene` | `scene-tests-final.txt` | 0 |
| `cargo clippy --locked --offline -p agq-studio-scene --all-targets -- -D warnings` | `scene-clippy.txt` | 0 |
| `cargo run --locked --offline -p agq-studio-scene --example layout_quality` | `layout-quality.txt` | see output footer |

GPU timestamps are not measured here. The default eframe device does not request
timestamp-query features, and the renderer owns the paint render pass. Accurate
bracketing requires optional feature negotiation, pass integration and
asynchronous readback. CPU upload submission time is not GPU execution time.

Unicode names are exercised in projection/layout and use elision that preserves
the original string. This does not qualify CJK fallback, mixed-DPI font rendering
or IME input. Those require actual platform smoke tests.
