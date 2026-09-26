# Final measurement closeout

The real native edit and its separate-process restart both passed. The first
10k camera scenario failed its anchor assertion; the same-binary repeat passed.
Both results remain evidence. This note does not turn that repeat into proof
that the original failure was harmless or establish an overall acceptance label.

[Combined extraction](final-combined-extracted.md) and its
[complete JSON](final-combined-extracted.json) retain the real journey, all three
synthetic runs, restart report/receipt, environment and reconstruction receipt.
The earlier [run06-only extraction](run06-extracted.json) is unchanged. Its
`journey_passed_restart_pending` outcome is the original first-process result;
the independent restart evidence is separate, never rewritten into that report.

## Real durability and workflow

[Restart](../real-run06/restart.json) passed all five assertions and reports
`restart_verified: true`. Its [process](../real-run06/restart-process.json)
exited 0 after 194.745 s. It reopened the durable project, entered the actual
ModelingPlatform owner, inspected the same committed canonical Part, and
confirmed revision History. The first process had passed all 33 assertions,
including candidate review, validation and commit.

Both processes used executable SHA256
`c6a8013dbf7db7103153ca287758390f58e6f1db72e6a57405b2a87c6b5fc57b`.
The committed revision is `9539bfed-f725-43d9-b5f8-8254040e7d42`; the new Part is
`3670456b-6e5f-59ba-a083-9ac253de64a4` (`alphaStudioObserver`). This qualifies the
version-1 historical self-model journey, not the held composed-Studio model or
an independently source-only repository restore/second-edit/rename gate.

Native UI stage durations remain prepare 138.028 s, validate 29.197 s and commit
8.823 s; Prepare release to candidate observation is 135.029 s. These include
input sequencing, queue/work and observed UI completion. First-use view calls
still take seconds; cache hits exclude queue/presentation. No incremental
semantic speedup is inferred from these UI durations.

## All quiet fixture results

[Environment](quiet-final/environment.json): RTX 3060 Ti / Vulkan, Windows
display reports 1920×1080 at 165 Hz, High performance power scheme, vsync enabled.
The receipt records no competing native/compiler/semantic consumers during
these serial scenarios, while ordinary desktop/agent tools remained active.
This is a recorded quieter desktop condition, not a controlled hardware lab.
All three runs use the same executable above; their launch checkout is
`4241d7e9e524e5b858aa379e81b20bcb6f2d6a96`.

| Run and terminal outcome | Steady median / p95 ms | Pan p95 ms | Zoom p95 ms | Maximum anchor error, world units |
|---|---:|---:|---:|---:|
| [1k / 2k edges](quiet-final/1k.json), exit 0 | 6.060 / 6.195 | 6.162 | 6.182 | 0.001221 |
| [10k / 20k edges, first](quiet-final/10k.json), **exit 2 / failed** | 6.052 / 6.319 | 13.273 | 13.130 | **1042.277466** |
| [10k / 20k edges, repeat](quiet-final/10k-repeat.json), exit 0 | 6.054 / 6.232 | 12.492 | 11.371 | 0.002013 |

Each phase contains 120 native update-interval samples after 60 warmup frames.
These are CPU/application intervals, not measured presentation or photon time.
The failed run's timing samples are observations from a failed interaction
qualification; they are not silently promoted to an accepted benchmark.

| Run | Scene build ms | Layout/routing/diff ms | Hit p95 µs | Scene GPU median ms | Visibility CPU median ms |
|---|---:|---:|---:|---:|---:|
| 1k | 14.854 | 13.028 | 1.9 | 0.028672 | 0.238 |
| 10k first, failed | 148.208 | 128.657 | 9.1 | 0.157696 | 1.498 |
| 10k repeat, passed | 146.602 | 126.674 | 6.8 | 0.163840 | 1.402 |

Build/layout are the most recent build values. Hit/GPU/CPU values use their
recorded rolling windows, not independent phase distributions. GPU timestamps
cover only the custom scene pass, excluding labels/chrome/upload/presentation.
The 165 Hz setup and workload changes preclude a causal speed comparison with
the earlier approximately 60 Hz baseline.

## Bounded diagnosis of the 10k discrepancy

The [stress driver](../../../crates/studio-native/src/stress_automation.rs)
appends synthetic pointer and wheel events to existing RawInput. The
[raw-input hook](../../../crates/studio-native/src/app.rs) does not remove
ordinary OS events before this scenario. Frames 300–419 repeatedly inject a
fixed pointer and wheel; frames 420–449 inject no pointer while waiting for
scroll settling. The driver measures a stored absolute pointer against the
current viewport and camera, but does not retain an event trace, the first
divergent frame or viewport-geometry history.

The [viewport](../../../crates/studio-native/src/viewport.rs) consumes
`smooth_scroll_delta` at the currently observed hover/interact pointer.
`Camera2D::zoom_at` preserves that supplied pointer's world coordinate. Thus an
ordinary pointer move during remaining smooth scroll is a plausible way for
the benchmark's stored anchor and the actual zoom anchor to diverge. A viewport
move/resize or another camera action is also not excluded by the retained data.
The near-identical initial/final zoom scalars alone do not prove anchor stability.

**Cause remains unproven.** The aggregates cannot establish that any OS event
occurred, that the failure began during those 30 settling frames, or that the
application camera path is correct for every input sequence. The successful
same-binary repeat demonstrates non-reproduction once, not a fix. A future
bounded qualification should retain input provenance, actual pointer/viewport,
smooth-scroll values and first divergence, then test an isolated-input scenario
while preserving its original assertion. No source, build, benchmark or runtime
change was made during this closeout.
