# Real run06: final independent editor/product review

Would I confidently demonstrate this to a serious systems engineer? **Yes, as a
bounded alpha demonstration of real architecture, provenance and a reviewed
source-backed change.** The candidate workflow now completes against the actual
model. General product acceptance still needs judgment beyond that demonstration:
large comparisons remain difficult, Requirements is basic, and unassisted
30-minute use was not tested.

No feature changes, builds or runtime consumers were performed for this review.
All 13 run06 PNGs were viewed, and every PNG digest matched its real-data JSON
sidecar. Exact hashes and scene identities are in
[the evidence manifest](real-run06-editor-evidence.json).

## What the actual journey establishes

The version-1 real journey passed **33 of 33 assertions** in 532.740 seconds;
the native process exited 0 in 534.436 seconds. Its baseline manifest exactly
equals run05. The report digest is
`81d98cfa45e73d059ec3fa6b318d5d6422a7fd9ea7c82f316078780e9892f7ad`.
This is the original six-document self-model, not the held composed Studio
extension or held version-2 driver.

Ordinary input opened the authenticated Validated baseline, focused Platform
then Repository without the earlier Home workaround, inspected actual connected
and inherited ports, obtained dependencies, opened Graph/Requirements/Explain,
compared the parent, created a nested part, reviewed Current/Candidate/Diff,
validated and committed. The canonical added part is
`3670456b-6e5f-59ba-a083-9ac253de64a4`, owned by ModelingPlatform
`6bf68ca1-ecfa-5883-9608-747f1824c0e3`. The committed Validated revision is
`9539bfed-f725-43d9-b5f8-8254040e7d42`, parent
`62af4537-f2c9-4bf1-90d6-e0862d6b4141`.

The first-process outcome remains `journey_passed_restart_pending`. Separate
restart was running during the initial review. The independently checked
completion is recorded in the addendum below, not inferred from a screenshot.

## Iteration findings

**Closed: anonymous Graph relationships.** Run05's six-node, six-edge Graph at
57% had no incident labels. The [run06 Graph](../real-run06/gallery/03-graph-world.png)
shows `owns`, `typed by` and `specializes` beside the actual selected paths at
the same camera. Labels avoid the card text and communicate distinct relations
without labeling the entire large graph. The
[agent dependency view](../real-run06/gallery/07-agent-view.png) benefits too.
Its mock is explicitly a deterministic view choice with no returned weights;
read-only authority and the target revision are visible.

**Improved, still open: large history comparison.** The
[run05 comparison](../real-run05/gallery/06-history-diff.png) devoted most of the
upper workspace to controls. The
[run06 comparison](../real-run06/gallery/06-history-diff.png) retains complete
counts and offers a compact group selector plus Browse all 65 changes. The real
AgentArchitecture owner is now in frame. However, 159 projected nodes / 130 scene
nodes / 405 scene edges remain a dense cross-linked field at 42.9%. Long names
clip or wrap awkwardly, and the right side extends beyond the viewport. This is
useful local inspection, not an excellent large-change overview. The popup's
open-state usability was not captured in this journey.

**Closed: candidate acceptance contradiction; actual workflow now demonstrated.**
Run05 stopped before explicit new-part selection because its driver expected
that selection too early. Run06 first preserves the observed selection, then
selects the actual added part and completes the mode roundtrip. The
[Candidate screen](../real-run06/gallery/08-candidate.png) clearly states WORKING
CANDIDATE and Not committed, retains the correct owner/Inspector, and disables
Commit until validation. The [small candidate diff](../real-run06/gallery/09-candidate-diff.png)
shows the new part and changed owner together, with stable surrounding positions
and explicit Current/Candidate/Diff controls. The initial Candidate camera retains
the operator's earlier pan and clips some definition context; Fit/Diff provides
a coherent complete local view. This is a remaining presentation limitation,
not lost canonical selection.

**Retained: truthful ports and readable Explain.** The
[selected Platform port](../real-run06/gallery/02a-platform-port.png) has a clear
owner header and an exterior connection route. Inspector names the actual
opposite port and leaves unknown direction unspecified. The
[inherited Repository port](../real-run06/gallery/02c-repository-port.png) retains
its original Repository owner. [Explain](../real-run06/gallery/05-explain.png)
fits three actual supporting facts, rule and consequence together and explicitly
discloses the partial proof. Exact Evidence and Advanced remain available.

## Surface ratings

These are judgments of the real scenes and workflow demonstrated here, not
blanket claims for every model size or operation.

| Surface | Rating | Evidence and remaining limit |
| --- | --- | --- |
| System World | Alpha-quality | Root, subsystem, ports and inherited interface are understandable; Platform definition cards still consume considerable width and small labels truncate. |
| Graph World | Alpha-quality | Selected sparse neighborhood now communicates relationship meaning; dense general graphs are not qualified by this image. |
| Requirements World | Foundation | Real requirement→subject→architecture path is useful and truthful; generic Element cards/question-mark icons and only one exercised requirement leave limited engineering depth. |
| Inspector | Alpha-quality | Meaning, owner, interface, connection and source precede Advanced details; actual selected identities match. |
| Explain | Alpha-quality | Human-readable consequence and bounded real causal diagram are visible together; partial evidence is honestly disclosed. |
| History | Foundation | Committed change summary, revision and branch are visible; older rows still say generic Design revision and do not communicate their changes well. |
| Diff | Foundation | Small candidate comparison is clear; the large historical comparison remains crowded and partially clipped. |
| Candidate World | Alpha-quality | Real prepare/review/validate/commit completed with explicit authority and phase; only this create-part edit class is visually qualified. |
| Agent overlays/views | Alpha-quality | Useful revision-bound dependency view, explicit read authority and honest provider choice; this does not qualify general autonomous agents. |

Highest remaining visual issues are the large-comparison density, Platform
definition/label density, and the basic Requirements vocabulary. These are
retained findings for the frozen result, not new feature requests during wrap-up.
The screenshots do not qualify Back/Forward behavior, reduced motion, IME,
screen readers or every keyboard/context-menu route.

## Responsiveness scope

The actual prepare observation lasted 135.029 seconds and recorded a maximum
input-hook gap of 11 ms over 22,281 hooks. Ordinary camera panning occurred while
the mutation was pending. A different baseline object, ViewService, returned
an exact baseline Inspector while preparation was still pending: 2,927 ms from
selection, 2,866 ms from request observation. This is real background-read
evidence; an approximately three-second first inspection still feels slow.

The enclosing interaction steps took 138.028 seconds for prepare, 29.197 seconds
for validation and 8.823 seconds for commit. They include ordinary input, worker
responses and UI settling and are not isolated semantic phase timings. Loading
the parent comparison took 110.830 seconds. Scene rebuilds were 0.091 ms for the
sparse Graph, 36.564 ms for the large comparison and 0.598 ms for candidate Diff.
The recorded roughly 6–7 ms frame tails use opt-in automation cadence. They do
not establish physical input latency, VSync frame rate, or representative manual
pan/zoom p95. No performance target is promoted solely from those screenshots.

## Final three-source freshness review

Independently compared `freshness-final/proposal.json`, its patch, the applied
manifest and the previously qualified `b10fe62c` source bytes. Exactly these
three input hashes changed, and each new hash equals the qualified source:

| Source | Qualified SHA-256 |
| --- | --- |
| `source_checkpoint.rs` | `8465c80bf7707c540399265d0447f5b0d44dba7730ff4ec3818a6f5e62010368` |
| `source_inputs.rs` | `f15171c15dd27c2e61c05c3dfffc426b1714b14cc09e7c8716bbb046d1f30128` |
| `sysml/publication.rs` | `682f079a4e2c6c412b897d159f93f3b495f7fd8d69a675aec11f8ae6df9fbef7` |

All other manifest fields, accepted receipt/bindings hashes and publication
identity are unchanged. The final standards receipt exits 0 and its output hash
matches; it records the unchanged accepted dependency closure of 69 packages,
digest `5cd40b34553cfc18534928eaad2465888325770e7755d4d85b1ef0550b48b340`.
The separately reviewed full answer parity, independent cold reconstruction and
actual native validation/commit justify this bounded freshness bookkeeping.
It grants no new publication authority or broader conformance. The composed
Studio model, version-2 journey and held navigation fixes remain unqualified
and are excluded from this review's product claims.

## Separate-process restart addendum

The independent restart now passed all **five assertions**, with outcome
`passed`, `restart_verified: true`, native exit 0, report elapsed 192.955 seconds
and measured process wall time 194.745 seconds. Its report SHA-256 is
`18e3054ddc3a8e1f5854e67397eeae26dac8260f4009fdd3282ff7f144972a35`.
The referenced first-report digest exactly matches the retained run06 report:
`81d98cfa45e73d059ec3fa6b318d5d6422a7fd9ea7c82f316078780e9892f7ad`.

The actual executable was rehashed and equals both process receipts:
`c6a8013dbf7db7103153ca287758390f58e6f1db72e6a57405b2a87c6b5fc57b`.
The restarted committed manifest equals the first process's full manifest,
including revision `9539bfed-f725-43d9-b5f8-8254040e7d42`; project and branch
also match. Ordinary selection/focus exposes the same authored
`alphaStudioObserver`, canonical ID `3670456b-6e5f-59ba-a083-9ac253de64a4`,
under exact owner `6bf68ca1-ecfa-5883-9608-747f1824c0e3`. The five assertions
cover durable project opening, owner selection, owner focus, exact part
inspection and durable History.

All four restart captures were viewed and their PNG/sidecar digests verified:
[System](../real-run06/restart-gallery/10-restarted-system-world.png),
[owner](../real-run06/restart-gallery/10a-restarted-part-owner.png),
[committed part](../real-run06/restart-gallery/11-restarted-committed-element.png),
and [History](../real-run06/restart-gallery/12-restarted-history.png).
Every scene is bound to the committed revision and explicitly records real
authenticated semantic data without a fixture. Exact hashes extend the existing
evidence manifest. This completes the real-model journey through durable restart.

Overall judgment remains **AGENTIQUE NATIVE STUDIO ALPHA NOT YET ACCEPTED**.
The successful real gate does not resolve Requirements/Diff quality, first-view
latency, new-project workflow limits or intermittent stress acceptance. The
parent reports that the first quiet 10k run failed its zoom-anchor gate and a
repeat passed; both are retained while the performance reviewer diagnoses the
difference. This addendum makes no independent diagnosis of that stress result.
No further feature work or runtime consumer was started by this reviewer.
