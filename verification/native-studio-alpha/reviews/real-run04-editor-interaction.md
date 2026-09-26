# Real run04: editor interaction and spatial meaning

Reviewer role: game/editor interaction designer. Reviewed the actual 1600 × 1000
native PNGs with their semantic JSON sidecars and the completed failed journey
report. No build, runtime consumer, or current process was run by this reviewer.

## Product judgment

**Would we confidently demo the complete Studio journey to a serious systems
engineer? Not yet.** The opening hierarchy and the repository's inherited port
are now understandable. The Platform port placement still suggests incorrect
ownership, and the actual journey stops at the dependency view before reaching
Explain, history, candidate review, validation, or commit. This is a useful
product iteration over real semantic data, not complete alpha acceptance.

## Exact evidence and comparison limits

Run04 opened a new real project `07d1f9bd-8f5b-4811-a9ab-7dd6408f91d9` and reached
Validated baseline `62af4537-f2c9-4bf1-90d6-e0862d6b4141`. Every inspected sidecar
has `fixture: null` and `semantic_data: real authenticated models/agentique`.
All five PNG hashes were independently recalculated and match their sidecars.

Run03 used baseline `8dcfb224-55e6-4d0d-ac3a-84fcfffb9a46` in a different project.
All six source-document content digests match exactly between the two baseline
manifests. The following is therefore a comparison of the same authored
engineering content in separate projects, not a same-revision camera-displacement
or canonical-ID stability measurement.

| View | Actual before | Actual after | Observed change |
| --- | --- | --- | --- |
| System | [run03 System](../real-run03/gallery/01-system-world.png) | [run04 System](../real-run04/gallery/01-system-world.png) | Same 7 projected/scene nodes and 6 canonical edges; visible edges fall from 6 to 3. Container ownership no longer competes with the same geometry. Zoom remains 1.1896. |
| Platform | [run03 Platform](../real-run03/gallery/02-focused-subsystem.png) | [run04 Platform](../real-run04/gallery/02-focused-subsystem.png) | Same 20 projected nodes/20 edges, 18 scene nodes; visible edges fall from 20 to 9. Child rows remain visible in Explorer. Zoom 0.7051 → 0.7080. |
| Selected Platform port | [run03 clientQueries](../real-run03/gallery/02a-platform-port.png) | [run04 clientQueries](../real-run04/gallery/02a-platform-port.png) | The selected port marker, surrounding focus outline, and actual name are now visible at overview scale. Placement remains misleading. |
| Repository | [run03 Repository](../real-run03/gallery/02b-focused-repository.png) | [run04 Repository](../real-run04/gallery/02b-focused-repository.png) | Same 5 projected nodes/4 edges and 4 scene nodes; visible edges fall from 4 to 2. Containment and specialization are easier to distinguish. |
| Inherited port | [run03 repositoryRevisions](../real-run03/gallery/02c-repository-port.png) | [run04 repositoryRevisions](../real-run04/gallery/02c-repository-port.png) | Selected port and name are legible; Inspector explicitly identifies its actual Repository owner and ModelRevision type. |

## Ranked remaining problems

1. **P1 — selected Platform port suggests the wrong owner.** In
   [02a](../real-run04/gallery/02a-platform-port.png), `clientQueries` is placed
   inside the `workspace` child card. The cyan connection runs through the middle
   row containing `workspace`, `validationService`, and `systemsModelingApi`.
   An engineer can reasonably read it as an interface between those components.
   The same-revision sidecar instead identifies both ports as owned by
   ModelingPlatform. The canonical connection is
   `bf57b7b0-660f-5e0c-9a0c-4590634d51f8`, joining `clientQueries`
   (`206b48b1-d0bb-52ed-8248-50e05afe4ecc`) and `platformQueries`
   (`f1c9c9f3-1880-58bd-b1c9-df9e1eea8239`). The Inspector is correct; the
   spatial communication is not. **Requested correction:** reserve a container
   boundary lane for its ports and labels, and route that connection through
   empty container space or a perimeter channel. **Acceptance:** at this same
   normal window size, neither port label overlaps a child card and no connection
   segment crosses a child body. Each endpoint still selects its actual canonical
   port; no inferred direction is added.

2. **P1 — the application journey stops on its next ordinary action.** The
   [actual report](../real-run04/journey.json) records failure at step 9,
   `agent shows revision-bound dependencies`, with
   `View response rejected: requested and returned definitions differ` after
   549,971 ms total process time. This is observed failure, not an inferred
   screenshot defect. The exact definition guard correctly refuses a mismatched
   response, but the operator still cannot complete the intended workflow.
   **Acceptance:** the normal dependency action returns the requested exact
   revision/definition and visible overlay, followed by the unchanged real
   Explain/history/candidate/commit/restart gates. Do not remove the guard.

3. **P2 — Platform overview labels are too small for sustained engineering
   work.** [02](../real-run04/gallery/02-focused-subsystem.png) fits 18 scene
   nodes at 70.8%; names are roughly 10–11 screen pixels high and
   `SystemsModelingApi...` is truncated. Eight definition cards occupy roughly a
   third of the usable canvas width. Inspector and Explorer labels are readable,
   but reading the system itself requires repeated zoom or panel lookup.
   **Requested correction:** make the first focused view emphasize the owned
   architecture, with deliberate secondary presentation of typing context.
   **Acceptance:** engineering component names are readable at the initial focus
   camera, while every displayed typing relationship remains inspectable and
   explicit view controls retain access to the full context. Do not discard
   definitions merely to improve a screenshot.

4. **P2 — entering a subsystem still loses the visible navigation trail.** The
   Repository screen shows `Agentique … / ModelRepository`, rather than exposing
   the visited Platform context. This is not a request to fabricate semantic
   containment: ModelRepository's actual lexical owner is PlatformArchitecture.
   **Requested correction:** distinguish visited focus navigation from canonical
   ownership in the breadcrumb/history presentation. **Acceptance:** the operator
   can recognize and return to the previously visited Platform without guessing
   whether Up means semantic owner or prior focus. Back/Up behavior needs an
   actual input check; a static screenshot does not prove it.

5. **P3 — small language and discoverability issues remain.** Repository uses
   `1 parts` and `1 ports`; missing port direction is honestly reported as
   `not specified`, but the port Inspector does not make its connection state
   explicit when there are none. Correct pluralization and distinguish a complete
   zero-result connection query from a query with unavailable/incomplete results.
   Do not infer that absence of a displayed relationship proves disconnection.

## What the journey actually established

Eight assertions pass: accepted runtime/Validated closure, select/focus Platform,
inspect its actual connected port, select and enter Repository from that focused
Platform, inspect its original inherited port, and select Repository as the
dependency subject. This substantiates the ordinary input path through those
objects in addition to the visible screenshots. Candidate identity and phase are
null at failure, and no semantic mutation is pending.

The new Explorer visibility, selected-port marker, engineering-first port
Inspector, and inherited owner disclosure improve the product. They do not prove
keyboard-only completion, camera animation quality, candidate mental-map
stability, or durable restart.

## Explain and change-focused diff: still awaiting real after images

The run03 [Explain](../real-run03/gallery/05-explain.png) hides its rule and
conclusion below the visible causal diagram. Its [history diff](../real-run03/gallery/06-history-diff.png)
shows an anonymous horizontal strip at 8.3% zoom. Source fixes now select a bounded
connected explanation and a canonical owner change group with a readable camera.
Those fixes passed a separate source review; **run04 produced no after captures
for either view**, so this review cannot rate their new visual result.

The next actual gallery must show the evidence, rule and named consequence
together; the initial diff must identify a named structural change at readable
scale while retaining exact full before/after scope and removed ghosts. Candidate
Current/Candidate/Diff switching, background exploration, validation, commit and
restart still need actual successful evidence. Fixture images cannot close these
gaps.

The separately committed owner-view fix `e353768` addresses an independently
predicted nested-create visibility problem. It was not in the executable reviewed
here and has no visual acceptance from these captures.
