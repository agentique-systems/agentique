# Native renderer investigation

These are actual Windows native runs on the NVIDIA GeForce RTX 3060 Ti,
Vulkan backend, driver 616.92. They use explicit synthetic fixtures, not the
Agentique semantic model. The stress driver injects pointer and wheel events
through RawInput and checks resulting camera motion, zoom anchoring and culling.
Each phase retains 120 frame intervals after 60 warmup frames. Vsync is enabled;
the monitor refresh rate and presentation timestamps are not controlled/measured.

## Diagnostic progression under concurrent semantic work

| 10k nodes / 20k edges | Original | First allocation reduction | Borrowed visibility |
|---|---:|---:|---:|
| Visibility CPU median, ms | 16.97 | 7.95 | 1.89 |
| UI CPU median, ms | 21.20 | 10.21 | 3.16 |
| Steady interval p95, ms | 28.07 | 15.84 | 6.43 |
| Pan interval p95, ms | 36.40 | 25.59 | 13.02 |
| Zoom interval p95, ms | 34.59 | 23.01 | 13.19 |

The machine was also doing accepted-publication reconstruction and compilation.
These are diagnostics with changing competing load, not an isolated benchmark
series or proof that every interval improvement came from this patch. The
[paired CPU microbenchmark](../visibility-performance.md) alternates the old and
new paths in one process and verifies exact visible identity and draw order at
every sample. Its pan median fell from 5.290 to 1.174 ms, p95 from 6.841 to
2.295 ms. No visible relationship was dropped to obtain this reduction.

The final integrated diagnostic's scene build took 225.89 ms: 202.22 ms for
layout/routing and 23.68 ms for indexes/outliner. Large changes within a loaded
revision build on the scene worker while the previous scene remains available.
Revision swaps still stage a synchronous scene to preserve revision coherence;
this is an outstanding first-view latency limit, not a claim of zero UI cost.
Hit-test median was 2.1 microseconds, p95 9.0. Changed GPU batch construction was
1.90 ms median / 5.79 ms p95; label/accessibility work was 0.33 / 0.49 ms.

Exact evidence:

- [Original diagnostic](10k-concurrent-reconstruction.json)
- [First reduction](10k-optimized-concurrent.json)
- [Integrated borrowed visibility](10k-borrowed-concurrent.json)
- [Integrated command and exit 0](../checks/stress10k-borrowed-concurrent.json)

## GPU timestamps and input boundaries

`--gpu-timestamps` requests timestamp features only when supported by the adapter.
Four bounded readback slots resolve/map in later frames with nonblocking polls.
The timestamps bracket the custom scene pass. Text, egui chrome, upload,
submission and presentation are outside that interval.

| Measured scene pass | Median | p95 | Readback errors |
|---|---:|---:|---:|
| 1k smoke | 0.0287 ms | 0.0328 ms | 0 |
| 10k borrowed-visibility run | 0.1587 ms | 0.2355 ms | 0 |

[The 1k smoke](1k-gpu-smoke-concurrent.json) used the earlier visibility path;
its steady/pan/zoom interval p95 values were 6.52/6.43/6.55 ms. Its outer scope
string predated optional timestamps and incorrectly said GPU duration was not
measured; the nested timestamp scope and samples are the actual observation.
The driver wording is corrected in subsequent artifacts; old evidence is intact.

At 10k, handled pan to following UI update measured 5.68 ms median / 12.70 ms p95;
zoom measured 5.93 / 12.67 ms. Raw input to completed UI shape construction
measured 3.15 / 8.39 ms. These application boundaries exclude OS input delivery.
Queue submission, presentation and physical input-to-photon remain explicitly
null. Scene GPU timing must not be described as total GPU frame time.

Real-model sizes and semantic candidate/validation timings remain separate gates.
