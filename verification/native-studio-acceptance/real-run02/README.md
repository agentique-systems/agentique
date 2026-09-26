# Real Native Studio journey, second execution

The actual window completed all 39 assertions and committed a nested
`alphaStudioObserver` PartUsage in the real Agentique self-model. The process
exited 0 after 617.943 seconds. This is a fresh, isolated repository bootstrap,
including two source imports, not a warm-open measurement. The first asserted
Validated view appeared at 420.593 seconds. Separate-process restart remains a
separate gate.

The retained executable identity and build receipt are in
`artifact-manifest.json`. `journey.json` contains the full before/after durable
manifests, semantic identities, observed input/reply bindings, assertions and
frame metrics. `process.log` is the raw service and projection timing output.
All 16 screenshots and their state snapshots are retained, including weak views.

| Observed boundary | Result |
| --- | ---: |
| Candidate preparation, pending mutation interval | 171.512 s |
| Current-revision Inspector, selection to reply during preparation | 315 ms |
| Same Inspector, observed request to reply | 254 ms |
| Largest input-hook gap during preparation | 14 ms |
| Input hooks during preparation | 28,283 |
| Validate scripted action | 2.592 s |
| Commit scripted action | 860 ms |
| Actual durable repository commit | 132.463 ms |
| Complete service `commit_prepared` | 133.980 ms |
| Process peak working set | 6,769,532,928 bytes |

Scripted action times include 30 lead and 18 settle frames; they are not isolated
service call timings or physical presentation latency. Validation's durable
candidate preparation was 1.560 s, including 1.321 s semantic-cache frontier
export and 218 ms encoding. It reused the candidate's completed semantic work.
The candidate preparation result does **not** demonstrate a speedup over the
previous approximately 135-second observation.

During preparation, real native camera pan and inspection of a different object
completed against the original Validated revision while mutation remained
pending. The inherited repository port and a derived relationship were inspected
and explained. Actual earlier revisions were visited twice; Return to head
restored full Requirements and Graph presentation state. The requirement-neighborhood
and restored-neighborhood screenshots are byte-identical. Requirement links are
shown as modeled relationships, without claiming satisfaction or verification.

Review found three material presentation defects: History cards used identifier
order, the initial durable Diff retained an unrelated Inspector and cropped
changes, and the selected new candidate part was offscreen. The committed History
card also exposed raw receipt JSON. These images are evidence for subsequent
fixes, not a claim that every surface was Alpha-quality in this executable.

Scene rendering used an RTX 3060 Ti through Vulkan. The final rolling 240-frame
window was 6.050 ms median / 6.558 ms p95; this is not a whole-journey distribution.
GPU timing reported 17 timestamp errors, so GPU samples are incomplete. No actual
device-loss recovery or physical input-to-photon qualification is established.
