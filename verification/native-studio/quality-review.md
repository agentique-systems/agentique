# Native Studio visual and interaction review

Three explicit visual review rounds were completed against captures of the
running native wgpu application. All scenes in this record are deterministic
visual fixtures. They establish presentation evidence, not accepted SysML
semantics or real-self-model acceptance.

## Round 1: hierarchy present, chrome unreadable

Reviewed `verification/generated/native-studio/round-1.png`, captured by the
native surface path; its run log is `visual-round-1.txt` in the same directory.
The three subsystem containers, port connections and selected ModelRepository
already read as a spatial architecture. The chrome did not meet the product
bar: dark surfaces contained nearly black text, bright default widgets broke
the theme, and the outliner listed all parents before their children instead
of grouping each subtree. Inspector relationships omitted endpoint names.

Improvements applied for the next capture: explicit native theme application,
readable control/text colors, parent-first subtree traversal in the outliner,
component summaries on containers, directed relationship arrowheads and
resolved relationship targets in the inspector. Selection remained a distinct
outline/halo rather than relying on node fill color.

## Round 2: readable shell, comparison exposed a containment defect

Reviewed `round-2.png`, `diff-review.png` and `requirements-review.png` in
`verification/generated/native-studio/`; their capture logs are
`visual-round-2.txt`, `visual-diff.txt` and `visual-requirements.txt`.
Round 2 confirms that the chrome and outliner problems are fixed. Ownership
groups are readable at normal scale and the inspector now names relationship
destinations, including semantic ports. Requirements also use the shared scene
engine with a distinct readable arrangement.

The comparison capture exposed a concrete continuity defect: the removed
Inspector ghost sat outside the shrunken NativeStudio container. Its displayed
component count also remained three although the after revision contains two;
ModelingPlatform similarly retained three despite gaining a fourth part.
Inspector type text still exposed `PartUsage` rather than a friendly label.

Improvements applied: comparison containers now union the required before/after
presentation bounds so removed children remain enclosed; this does not change
layout memory or semantic ownership. Fixture revision groups/counts are
recomputed after additions and removals, and normal inspector labels use
human-readable type names. Structural regression coverage accompanies the
container-bounds fix and fixture count correction.

## Round 3: final System World and comparison review

Reviewed `round-3.png` and `diff-final.png`, with logs `visual-round-3.txt` and
`visual-diff-final.txt`. Round 3 preserves the clear subsystem composition and
shows the friendly `Part usage` inspector label. The final comparison encloses
the removed Inspector within NativeStudio, reports two and four current
components for the affected containers, and keeps unchanged nodes in their
familiar order. Removed objects are ghosted and explicitly marked; the added
CandidateCoordinator has a halo and a `+ NEW` badge; changed objects have a
label and restrained border treatment. Direction and change therefore remain
legible without relying only on color.

Retained representative captures:

- [Native System World — visual fixture](native-system-fixture.png), identical
  to `round-3.png`, SHA-256
  `4d8c09a7752bc6c1afa2277bfec121072028db142ad86178423c3dcc87399f0b`.
- [Native revision comparison — visual fixture](native-diff-fixture.png),
  identical to `diff-final.png`, SHA-256
  `4ed8d73f8aa8e45fb38a8ed43d9a4a36989b3cc314ac8bd134e5dd9a3c897030`.

These are actual 1600 × 1000 native surface captures, not design mockups or
browser images. The capture logs identify an NVIDIA GeForce RTX 3060 Ti Vulkan
adapter and four semantic scene draw calls. The System World run has a roughly
16.66 ms median wall-frame interval with vsync enabled. This is neither GPU
timestamp timing nor a measurement of physical input latency; capture runs are
also too small to establish large-scene performance.

## Actual input review iterations

The input runner injects real egui keyboard/pointer events into native frames
and checks later application state. The first three actual runs all failed;
the failures are retained as engineering evidence, not counted as acceptance.

| Actual report | Passing stages before failure | Finding and correction |
| --- | ---: | --- |
| `interaction-1.json` | 1 | The palette driver typed before reliable text focus and supplied logical Command without the native Ctrl modifier. It selected the default command. The driver now waits for palette geometry/focus, clicks its input, supplies the platform modifier and verifies the query before Enter. |
| `interaction-2.json` | 5 | Rapid independent clicks contaminated egui's time-based click count; the intended focus gesture was not recognized. The driver separates gestures. The product also gained a same-target, same-scene-generation, near-position double-click guard with a regression test, so clicks on different objects cannot accidentally focus. |
| `interaction-3.json` | 17 | The dependency action ran, but the assertion demanded component IDs where the overlay intentionally retained semantic port IDs. The check now normalizes endpoint owners before requiring ModelRepository, SystemWorld and DecisionAgent context. It preserves port identity and does not bypass the action. |

Reports and process output are under
`verification/generated/native-studio/interaction-{1,2,3}.json` and
`native-interaction-{1,2,3}.txt`.

Both subsequent actual release runs, `interaction-4.json` and
`interaction-final.json`, passed all **37 assertions**, with no failed assertion
and process exit code **0**. The final invocation from the repository root was:

```powershell
target/release/agq-studio-native.exe --fixture architecture --no-restore --scenario vertical --scenario-report verification/generated/native-studio/interaction-final.json
```

The preceding passing invocation used the same arguments with report path
`verification/generated/native-studio/interaction-4.json`. Recorded process
durations were 13.531 seconds for run 4 and 15.359 seconds for the final run;
these are automation durations, not a five-minute user study or latency metric.
The deterministic reports are byte-identical with SHA-256
`e90f0dda72946dcd35c89c89e5e6fbc45ba9b0334fe41233e974aad472ae78a8`.
[The retained acceptance summary](interaction-acceptance.json) records every
assertion name/result, both exact invocations, exit codes and report identities.
Process records are `verification/summaries/native-studio/native-interaction-4.json`
and `native-interaction-final.json` in that directory. The executable was rebuilt
after these runs; no hash of the current executable is attributed to either run.

## Product judgment and limits

The final visibility optimization was followed by another successful 37-stage
native run (`native-vertical-accepted`, exit 0). Its executable SHA-256 is retained
in `interaction-acceptance.json`. A fresh actual System World GPU capture
(`native-system-accepted`, exit 0) is byte-identical to the retained System World
PNG: `4d8c09a7752bc6c1afa2277bfec121072028db142ad86178423c3dcc87399f0b`.
This supplements structural, geometry and input assertions; it is not the sole
correctness check. The missing-runtime setup was also reviewed and refined to
show a clear installation path with optional technical details, while its scene
contains zero objects and submits zero scene draws.

The retained System World screenshot is a credible beginning of a serious
engineering environment: hierarchy reads immediately, the selection is
unambiguous, the view has spatial continuity, and the inspector communicates
engineering relationships. The first screenshot did not meet that bar; the
review loop materially improved it. The final comparison is suitable to show
publicly when clearly labeled as a visual fixture.

Together, the three visual rounds and two passing native input runs support
accepting a serious foundation for the fixture experience. The exercised path
includes camera/navigation, derived-edge selection, Explain, semantic dependency
presentation, stable comparison, nested-part candidate preview, disabled fixture
validation/commit, cancellation and revision-context changes. This is measured
application-state evidence from actual native frames, not a claim that a
five-minute independent user study occurred.

This judgment is bounded. The small architecture fixture is deliberately
composed; it does not establish routing quality for arbitrary dense models.
Some parallel relationships share narrow corridors. Long inspector sections
require scrolling. Full semantic screen-reader coverage of the GPU canvas,
cross-platform text/IME behavior and extended hands-on ergonomics are not
established by these captures and automated runs. Runtime-backed reconstruction,
validation and durable commit remain separate semantic acceptance obligations
and are not accepted by the fixture scenario.
