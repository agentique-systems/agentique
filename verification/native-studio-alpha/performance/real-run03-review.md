# Real native run03: measurements and remaining qualification

Run03 reached the real Validated revision `8dcfb224-55e6-4d0d-ac3a-84fcfffb9a46`
and completed 24 journey assertions, then failed the background pan assertion.
It did not complete candidate construction, concurrent Inspector qualification,
validation, commit or restart. Process exit was 2 after 601.864 s; peak working
set was 5,852,807,168 bytes for the whole process. The unchanged six-document
model's projection reports 785 local canonical elements, excluding accepted
standard elements. These are not the 1,101 construction-work records from the
separate Create Part oracle.

The [extracted observations](real-run03-observations.json) retain exact numbers
and SHA-256 identities of all 15 input reports. They are read-only extraction,
not another test run. [Run03 process evidence](../real-run03/process.json) pins
executable `9cd59692c4ee414ecdfa16ac6633f1e1f719f11d84f7dbffb304baab00fbc61f`.

## Real scene snapshots

Times below are milliseconds, rounded to three decimals. N/E means node/edge
counts. Build is the last scene build, not a distribution. L/R/D includes layout,
routing and diff construction; index/outliner is the remaining separate interval.

| Capture | Projection N/E -> scene N/E | Build | L/R/D | UI CPU p95 | Update interval p95 |
| --- | --- | ---: | ---: | ---: | ---: |
| Run02 System baseline | 38/37 -> 38/37 | 3.995 | 3.362 | 0.629 | 13.725 |
| Run03 System | 7/6 -> 7/6 | 0.563 | 0.334 | 0.606 | 14.720 |
| Run02 ModelingPlatform | 41/49 -> 32/49 | 0.591 | 0.522 | 0.630 | 13.811 |
| Run03 ModelingPlatform | 20/20 -> 18/20 | 0.580 | 0.541 | 0.571 | 14.542 |
| Run03 ModelRepository | 5/4 -> 4/4 | 0.078 | 0.065 | 0.550 | 11.209 |
| Run03 Graph | 19/23 -> 18/23 | 0.723 | 0.386 | 0.768 | 10.908 |
| Run03 Requirements | 6/6 -> 6/6 | 0.146 | 0.127 | 0.581 | 10.120 |
| Run03 Explain | 41/60 -> 40/60 | 3.812 | 3.316 | 0.754 | 9.822 |
| Run03 History diff | 159/425 -> 130/405 | 62.550 | 61.767 | 0.786 | 7.841 |
| Run03 agent view | 19/23 -> 18/23 | 2.122 | 2.076 | 0.700 | 12.481 |

Scene contents, layout memory, code and workload changed. These rows do not
establish causal performance gains. Port screenshots repeat their preceding
scene-build values and are not independent build measurements. Run02 stopped
before repository focus, so there is no corresponding later-world baseline.
The narrowed dependency and corrected part-count patches postdate these images.

Frame/CPU/GPU summaries retain the latest 240 samples globally, not a separately
reset per-world trial. The 60-frame warmup applies to update intervals at startup.
History's recent 7.841 ms p95 therefore does not disprove its 62.550 ms build
spike: its scene is below the native 750-node asynchronous threshold, and the
builder's L/R/D timer includes both comparison sides and diff rerouting.

GPU scene-pass p95 across run03 captures was 0.002048–0.012288 ms, with zero
timestamp errors. It excludes text, chrome, upload, submission and presentation.
Hit-test p95 was 3.3–9.9 microseconds. Every gallery has zero pan and zoom
gesture samples; selection has zero or one. There is **no real-model pan/zoom
p95 qualification** here. OS delivery, frame submission, presentation and photon
latency remain unmeasured.

## Operator latency is a different observation

Run02/03 ModelingPlatform focus steps took 13.650 / 16.001 s respectively;
run03 ModelRepository focus took 14.924 s. Run03 inherited-port inspection took
7.652 s, dependency view 11.404 s, Requirements 4.507 s, and parent comparison
173.482 s. These are actual journey step durations, **not isolated projection,
query or camera timings**. The driver includes 30 lead frames, its input sequence,
18 trailing frames, and waiting for pending operations. Scene-build timers start
after semantic projection. No current timer separates request queue, revision
resolution, projection queries, Inspector work, response delivery or camera
settling. The seconds-long steps need that breakdown before assigning a cause.

The initial open step was 12.533 s, but the successful baseline assertion arrived
278.309 s after runner start; bootstrap/idle waiting before the step is outside
its duration. Neither number is a clean cold-runtime restoration measurement.

## Immutable current reads

The read lane retains an opaque, Read-authorized immutable revision. Its mailbox
lock covers selection of work, not semantic execution. Runtime epoch, binding,
selected identity, panel and latest request guard response installation. A
separate worker therefore removes the candidate worker's queue dependency once
the reader is pinned; it does not make the underlying query cheaper.

One executing obsolete read remains non-preemptible; at most one pending request
per panel is retained. A slow old Inspector can still delay a newer Source or
Explain read on this single lane, and a pin queued after preparation must wait
for the serial worker. These are source-level latency boundaries, not observed
failures. Candidate and old-ghost reads intentionally retain their serial path.

Run03 recorded only five background input hooks, maximum gap 7 ms, before pan
failed. `background_current_inspection` and completed preparation timing are null.
This proves neither a freeze nor successful background inspection. Qualification
must retry pan, then inspect a different exact current object while preparation
remains pending, retaining selection/request/response times and context fences.

## Quiet measurements still required

1. Rerun identical 1k/2k and 10k/20k fixtures after the correctness fixes, with no
   semantic consumer/build, the same window/DPI/adapter and retained phase sample
   counts. Compare with [existing diagnostics](synthetic-renderer.md), explicitly
   marked concurrent-load: 10k borrowed visibility had steady/pan/zoom interval
   p95 6.43/13.02/13.19 ms and a 225.89 ms build. Those are not quiet baselines.
2. On the exact real revision/view/camera, measure quiet pan/zoom and both cold
   first view and warm repeated focus. Capture request queued/start/completion,
   revision resolution, projection, scene build and first matching UI frame
   separately. Retain scene and projection sizes for every sample.
3. Measure quiet current Inspector/Explain/Source, then the same requests during
   a real candidate. Separate reader-pin, mailbox wait and query wall time;
   retain correctness evidence for old-read discard and actual current response.
4. Measure history first-open versus cached reopen separately and record the
   complete transition-frame maximum. Its quiet post-open screenshots do not
   qualify revision restoration or scene-swap responsiveness.

## Paired diff presentation review

The initial root WIP filtered active System Ownership edges before scene build
but passed an unfiltered before projection. `SemanticScene::apply_diff` appends
every absent-before-edge identity as a Removed ghost; unchanged Ownership could
therefore reappear falsely removed. This also affects asymmetric family filters.

The lead's proposed shared `presentation_projection` is the sound boundary:
apply identical family/hidden/System-ownership policy to both disposable sides
before construction and diff, keeping original DTOs and owner metadata intact.
For Diff, `expanded=None` on both sides avoids current-ID clipping that would
erase genuine removed nodes. Change-group camera/row focus can reduce visual
attention without filtering either semantic projection or causing a new layout.

Disable ExpandIncoming, ExpandOutgoing, ExpandBoth and CollapseNeighborhood
through shared command availability while in Diff, with execution rechecking the
same guard. Otherwise they mutate an ignored local expansion and report success.
Hide/disable the toolbar's direct Show loaded view reset too. Neighbors is
selection-only, but currently queries only the active side; disable it in Diff
until comparison-union behavior is explicit, especially for removed selections.
Keep Fit and FocusChanges available. Regressions must retain genuine removed
allowed relationships and nodes while excluding hidden families on both sides;
disabled commands must leave projections, expansion, focus and scene unchanged.

No code, build, runtime consumer or semantic/freshness pin changed in this review.
