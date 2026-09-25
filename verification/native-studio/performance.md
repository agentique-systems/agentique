# Native rendering and camera measurements

These measurements exercise deterministic visual fixtures. They do not establish
semantic acceptance of the real Agentique model or its runtime publication.

Both final native camera scenarios passed their assertions and exited 0. The 1k
scene sustained roughly 16.7 ms intervals through pan and zoom. Reusing ordered
visible objects and removing redundant checks reduced the quiet 10k pan P95 from
26.8 to 20.1 ms and zoom P95 from 28.0 to 19.3 ms. These are single-run comparisons;
remaining tails do not establish 60 FPS throughout every 10k interaction.

The native shell renders through its retained wgpu scene on Windows, using an
NVIDIA GeForce RTX 3060 Ti (Vulkan, driver 616.92) and AMD Ryzen 5 5600X. Vsync is
enabled. A delivered interval near 16.67 ms describes the observed 60 Hz cadence,
not maximum renderer throughput or a GPU execution timestamp.

## Reproduction

Build from the primary checkout so path dependencies reuse their existing build
artifacts. The GPU application has an independent Cargo workspace and lockfile:

```text
cargo build --locked --offline --release --config profile.release.lto=false --manifest-path crates/studio-native/Cargo.toml --target-dir target
target/release/agq-studio-native.exe --fixture stress1000 --scenario stress --scenario-report verification/generated/native-studio/stress1000-input.json --no-restore
target/release/agq-studio-native.exe --fixture stress10000 --scenario stress --scenario-report verification/generated/native-studio/stress10000-input.json --no-restore
```

Run the two native windows serially after other GPU work and compilation finish.
Use `verification/scripts/native_studio_check.py NAME COMMAND...` to retain the
exact command, output location, exit code and wall duration.

The stress scenario performs 60 warmup frames, 120 steady frames, 120 pointer-drag
frames, 120 wheel-zoom frames, and 30 settling frames. Pointer and wheel events
enter egui's ordinary RawInput route. It asserts camera displacement, meaningful
zoom, the maximum off-center pointer-anchor error throughout zoom, and a reduction
in the visible node count. Failure produces a failed report and nonzero exit.

## Final native camera results after visibility reuse

The final release binary is identified by SHA-256
`284b522bd38818655aedd5004cb8eaeda71f05d2629cf097aa3cc7e44a77a717`, built from
clean source commit `f4f6356261c4b05595f824f5ebb86e662f92d54d`. It uses the exact
release/LTO override shown above. Both runs were serial after compilation and
workspace tests completed, with no competing native window.

| Final visual fixture / phase | Median interval | P95 interval | Samples |
| --- | ---: | ---: | ---: |
| 1k / steady | 16.667 ms | 16.999 ms | 120 |
| 1k / pan | 16.662 ms | 16.921 ms | 120 |
| 1k / zoom | 16.664 ms | 16.922 ms | 120 |
| 10k / steady | 16.670 ms | 16.955 ms | 120 |
| 10k / pan | 16.690 ms | 20.052 ms | 120 |
| 10k / zoom | 16.677 ms | 19.280 ms | 120 |

| Final native metric | 1k nodes / 2k edges | 10k nodes / 20k edges |
| --- | ---: | ---: |
| Scene build including layout/index | 15.207 ms | 139.833 ms |
| Hit-test median / P95 | 3.1 / 5.1 us | 3.95 / 9.5 us |
| Handled pan to next UI update median / P95 | 16.501 / 16.811 ms | 16.517 / 19.903 ms |
| Handled zoom to next UI update median / P95 | 16.509 / 16.774 ms | 16.482 / 19.139 ms |
| Minimum visible nodes | 168 | 1,440 |
| Maximum zoom anchor error | 0.001465 world units | 0.002013 world units |
| Latest batch CPU upload | 0.293 ms | 4.223 ms |
| Latest batch bytes / instances | 650,240 / 8,128 | 6,416,000 / 80,200 |
| Draw calls | 2 | 2 |

The viewport now resolves ordered visible records once per frame, shares the
borrowed result between geometry and label/accessibility passes, and borrows
selected relationship IDs. Revision checks, containment order, LOD, selection
and GPU passes remain intact. The existing 21 native tests and strict Clippy
passed after this change, and both final camera assertion scenarios exited 0.

The observed quiet 10k pan and zoom P95 reductions are approximately 25% and 31%.
There are no timing distributions for upload or scene construction, so their
individual sample variation must not be presented as an optimization result.
GPU execution duration and physical input-to-photon latency remain unavailable.

## Earlier native camera runs before visibility optimization

Actual phase summaries and command exits are retained in
[performance-metrics.json](performance-metrics.json), including the executable
SHA-256 and the measured source identity. Runs were serial on the final integrated
release binary; remaining workspace unit tests were running in the background.
There was no competing native rendering or Rust compilation.

| Visual fixture / phase | Median interval | P95 interval | Samples |
| --- | ---: | ---: | ---: |
| 1k / steady | 16.663 ms | 16.887 ms | 120 |
| 1k / pan | 16.664 ms | 16.933 ms | 120 |
| 1k / zoom | 16.657 ms | 16.975 ms | 120 |
| 10k / steady | 16.653 ms | 17.106 ms | 120 |
| 10k / pan | 16.619 ms | 29.251 ms | 120 |
| 10k / zoom | 16.736 ms | 29.510 ms | 120 |
| 10k / steady, quiet repeat | 16.660 ms | 16.865 ms | 120 |
| 10k / pan, quiet repeat | 16.472 ms | 26.821 ms | 120 |
| 10k / zoom, quiet repeat | 16.773 ms | 28.000 ms | 120 |

| Native metric | 1k nodes / 2k edges | 10k nodes / 20k edges |
| --- | ---: | ---: |
| Scene build including layout/index | 18.330 ms | 144.011 ms |
| Hit-test median / P95 | 3.8 / 8.0 us | 5.0 / 11.8 us |
| Handled pan to next UI update median / P95 | 16.492 / 16.751 ms | 16.399 / 29.436 ms |
| Handled zoom to next UI update median / P95 | 16.467 / 16.813 ms | 16.493 / 29.281 ms |
| Minimum visible nodes during zoom | 168 | 1,440 |
| Maximum zoom anchor error | 0.000610 world units | 0.002013 world units |
| Latest batch CPU upload | 0.231 ms | 2.680 ms |
| Latest batch bytes | 650,240 | 6,416,000 |
| Scene draw calls | 2 | 2 |
| Retained batch uploads over 451 UI updates | 108 | 147 |

Pan collected 117 handled-input observations in each run; zoom collected 129 and
131 because wheel smoothing continued into the settling period. Selection was
not exercised by this camera scenario and remains `null` with zero samples.
The native unit suite passed all 20 tests, including the four timing contract
tests; the integrated architecture input scenario passed 37 assertions. Their
logs are retained by the primary checkout's verification runner.

After the workspace regression finished, the 10k scenario was repeated with no
competing build, test or native window. It passed and exited 0. The later release
binary includes setup-surface refinements; its explicit stress path is unchanged.
The repeat has its own binary hash and source metadata in the JSON. Scene build
was 144.299 ms, hit testing measured median 6.1 us / P95 11.1 us, and the latest
batch CPU upload took 2.606 ms. Culling again reached 1,440 visible nodes and
maximum pointer-anchor error remained 0.002013 world units. Handled pan-to-next-UI
update measured median 16.206 ms / P95 26.662 ms, and zoom measured median
16.499 ms / P95 27.635 ms. The quiet run confirms the active tail remains without
the earlier competing workspace workload. The subsequent native suite has 21
passing tests after adding a bootstrap fixture-separation check; strict native
Clippy also passed.

The separate [scene benchmark](scene-summary.md) measures layout directly:
1.62 ms at 1k and 27.74 ms at 10k. That earlier CPU benchmark ran with different
concurrent development load and must not be subtracted from this native run to
infer a renderer stage cost.

## Bounded review and follow-up for active 10k tails

Source inspection identifies plausible allocation and packing costs, not a
measured breakdown of the original 28–29.5 ms tail. Before the bounded change,
`SceneLookup::visible` sorted visible indices twice in a rebuild frame, and
already-resolved edges cloned relationship IDs for membership checks. The final
change removed those duplicate operations. A changed visibility set still
rebuilds the visible batch, and GPU preparation still copies the four pass
vectors into a new contiguous vector before queue submission.

The remaining follow-up is to time visibility query/order, batch construction,
and CPU packing separately, then evaluate reuse across frames and upload storage.
The retained two-draw stress renderer and semantic identity boundary do not need
a speculative architecture change. GPU execution time remains unknown.

## Meaning of the counters

- Frame intervals are CPU wall intervals between UI updates. Per-phase summaries
  retain 120 observations; the general timing panel retains its latest 240 after
  discarding 60 initial intervals.
- Pan and zoom timings begin when the viewport handles a gesture and end when
  the following UI update begins. They do not measure OS delivery, the current
  frame's presentation, or physical input-to-photon latency.
- Scene build includes layout and spatial-index construction. Layout is not
  separately timed by this native counter.
- Hit-test time includes the spatial query and any low-LOD port-to-owner mapping.
- GPU upload time is CPU time preparing and submitting the latest retained batch;
  bytes and instance counts also describe that batch, not cumulative traffic.
- GPU timestamps and physical input-to-photon latency remain `null`. Empty input
  and frame distributions also produce `null`, with explicit zero sample counts.

Missing values previously appeared as zero, and asynchronous window close could
emit a second benchmark report. Both defects are corrected in the timing change.
Deterministic tests cover absent values, gesture separation, preservation of the
earliest input in a frame, warmup exclusion, bounded storage, and tail latency.

## Preliminary idle baseline

The pre-instrumentation release binary was run serially for 360 frames. Both
processes exited 0. Browser regression tests were running in the background;
these numbers are preliminary observations, not the final camera acceptance.

| Visual fixture | Median interval | P95 interval | Scene build | Latest upload CPU | Draw calls |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1k nodes / 2k edges | 16.654 ms | 17.052 ms | 22.120 ms | 0.920 ms | 2 |
| 10k nodes / 20k edges | 16.639 ms | 19.415 ms | 153.262 ms | 3.776 ms | 2 |

All nodes were visible in the fitted view. Uploaded bytes were 650,240 and
6,416,000 respectively. The old zero-valued input fields are discarded because
no input was exercised. Raw baseline commands, outputs and exit files remain in
the timing worktree's ignored `verification/generated/native-studio-timing/`.

## Independent screenshot review

Round 3 reads as the beginning of a professional engineering environment. The
theme is coherent, containers establish hierarchy, selection is distinct, and
the inspector shows friendly categories and named relationship counterparts.
The final diff screenshot keeps the removed Inspector inside its historical
container; added halos, changed badges and ghosted removal carry distinctions
beyond color. Category symbols remain small at normal scale and inspector actions
require scrolling. These are visible refinement opportunities, not performance
or real-model acceptance evidence.
