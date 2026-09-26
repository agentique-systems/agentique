# Real run05: editor interaction review

Reviewer: independent game/editor interaction and product-quality review.
Actual native 1600 x 1000 PNGs were inspected, their hashes checked against JSON
sidecars, and the recorded ordinary-input assertions read. This reviewer did not
launch a build, runtime consumer, or application, and did not modify the running
journey. This review distinguishes screenshots, recorded application behavior,
and work that remains unqualified.

## Product judgment

**Would we confidently demo the complete Studio to a serious systems engineer?
Not yet.** The captured architecture/port/Explain workflow now conveys real
engineering meaning, and two concrete visual P1s from earlier rounds are
resolved. Default spatial diff still presents a crowded fragment beneath a large
control area. The run then stops at an acceptance-driver selection contradiction,
so candidate screenshots and durable commit/restart remain unqualified.

## Evidence identity

Run05 explicitly resumes the untouched failed run04 baseline, project
`07d1f9bd-8f5b-4811-a9ab-7dd6408f91d9`, revision
`62af4537-f2c9-4bf1-90d6-e0862d6b4141`. The entire baseline manifest equals
run04's manifest; the recorded predecessor-report digest independently matches
the exact run04 report bytes. Every inspected sidecar identifies
`real authenticated models/agentique` with `fixture: null`. Run05 repeats ordinary
runtime authentication/open and the complete acceptance steps.

The first five views can therefore be compared on the same canonical revision.
Run03 Explain/Diff images are older real-product comparisons from a separate
project, with the same authored source content; they are not same-ID or same-rule
layout comparisons.

## Actual before/after findings

| View | Actual evidence | Judgment |
|---|---|---|
| System | [run04](../real-run04/gallery/01-system-world.png), [run05](../real-run05/gallery/01-system-world.png) | Clear three-part hierarchy, legible names and restrained typing context. Seven scene nodes, three displayed edges; geometry unchanged. Engineering-workspace chrome replaces implementation language. |
| Platform | [run04](../real-run04/gallery/02-focused-subsystem.png), [run05](../real-run05/gallery/02-focused-subsystem.png) | Container boundary is understandable, with nine contained cards and external typing definitions. The default still fits the 18 scene nodes at 70.2%, making the component view small. |
| Selected Platform port | [run04](../real-run04/gallery/02a-platform-port.png), [run05](../real-run05/gallery/02a-platform-port.png) | **Observed ownership P1 closed.** Both actual port names occupy a clear container header. The cyan connection traverses the exterior and crosses no child-card body. Selection and Inspector still identify the actual port and owner. |
| Repository and inherited port | [Repository](../real-run05/gallery/02b-focused-repository.png), [port](../real-run05/gallery/02c-repository-port.png) | Useful original-owner explanation: ModelRepository inherits repositoryRevisions from Repository; the actual Repository boundary port is selected and Inspector shows Repository and ModelRevision. No copied inherited port is implied. |
| Graph | [run05 Graph](../real-run05/gallery/03-graph-world.png) | Six-node neighborhood is substantially more useful than the earlier global graph. The selected object is clear. Thin similar edges and suppressed labels still require active inspection to distinguish ownership, typing and specialization. |
| Requirements | [run05 Requirements](../real-run05/gallery/04-requirements-world.png) | Actual requirement -> subject -> architecture path is understandable and explicitly labeled. No pass/fail inference. Generic Element labels and ownership context still compete with the engineering question. This small case does not qualify dense requirements work. |
| Explain | [run03](../real-run03/gallery/05-explain.png), [run05](../real-run05/gallery/05-explain.png) | **Observed clipping P1 closed for the captured real explanation.** Three supports, named rule and readable consequence fit together. Summary discloses 3 of 8 supports and a partial proof with 351 recorded dependencies; exact Evidence and Advanced remain available. |
| History/diff | [run03](../real-run03/gallery/06-history-diff.png), [run05](../real-run05/gallery/06-history-diff.png) | Named changes and owner groups replace the anonymous 8.3% overview as the default. At 51.9%, names and addition markers can be read. The scene remains crowded and clipped; this is improved foundation quality rather than demonstrated alpha quality. |
| Agent view | [run05 Agent](../real-run05/gallery/07-agent-view.png) | Target, read-only authority, Complete status and temporary-view nature are explicit. Actual mock recommendation is clearly labeled deterministic, choice-only, with no manufactured weights. The graph is still a small horizontal band and the card occupies substantial Inspector space. |

The actual selected Platform connection is the same canonical InterfaceUsage
`bf57b7b0-660f-5e0c-9a0c-4590634d51f8`, from clientQueries
`206b48b1-d0bb-52ed-8248-50e05afe4ecc` to platformQueries
`f1c9c9f3-1880-58bd-b1c9-df9e1eea8239`. Inspector retains
ModelingPlatform `6bf68ca1-ecfa-5883-9608-747f1824c0e3` as owner and explicitly
says direction is not specified.

Explain selects real derived Subclassification
`039dfb97-d460-563f-be96-9d582b5a13d3`, with recorded rule
`db927468-b5ff-edac-1421-887601e79813`, from ModelRepository to the standard Part
definition. The friendly consequence is grounded in that exact edge. Run03
showed another selected derived relationship, so this demonstrates successful
bounded rendering, not identical-evidence before/after equivalence.

## Ranked remaining product problems

1. **P1 acceptance gate — First Candidate mode check expects a selection that
   the driver has not made.** The reported error says selection was lost. Actual
   before/after states preserve ViewService identically on the same candidate
   revision. `real_automation.rs:2073–2105` requires the new part immediately on
   the first Candidate mode check, but ordinary `Action::Select(PART)` is the
   following step (`1035–1039`). Background exploration intentionally selected
   ViewService. This is an observed test-driver contradiction, not evidence of
   production selection loss. **Acceptance:** first transition preserves its
   actual prior selection; after explicit new-part selection, the subsequent
   Current/Candidate/Diff sequence must retain the stronger intended new-part
   selection and revision-isolation assertions. Do not force production to erase
   the operator's current selection merely to satisfy the premature assertion.
2. **P2 — Diff's controls dominate the actual review area.** The change panel,
   relationship-family controls and disabled exploration row occupy roughly the
   upper 400 pixels below the main navigation. The remaining canvas displays a
   dense lower band, with cards cut at its bottom/right. The selected
   AgentArchitecture group has no visible selected owner anchor. The status
   explicitly reports context for 14 of 26 changes; 30 of 130 scene nodes are
   visible while the full 159-node/425-edge projection remains retained.
   **Requested improvement:** make the owner-group overview more compact and
   establish a clear starting object for review. Keep full before/after evidence,
   stable positions, offscreen changes and whole-comparison access.
   **Acceptance:** the default comparison shows a readable named change and its
   structural context without requiring the operator to decipher a clipped mesh.
3. **P2 — Platform focus still gives too much space to typing context.** Eight
   definition cards consume roughly a third of the canvas and reduce the owned
   component grid to 70.2% zoom. The SystemsModelingApi definition name remains
   truncated. **Acceptance:** owned components have a readable first-focus view,
   with type context deliberately secondary but still inspectable through normal
   controls. Do not alter projection truth to curate a cleaner screenshot.
4. **P2 — Visited focus navigation remains difficult to reconstruct.** Repository
   focus shows Agentique / ModelRepository without the visited Platform context.
   Inspector correctly names the canonical lexical owner, PlatformArchitecture.
   **Acceptance:** expose visited focus/back context separately from canonical
   ownership; do not manufacture a semantic Platform->Repository ownership edge.
5. **P2 — Graph relationships are not self-explanatory at the initial camera.**
   The six-node neighborhood is appropriately bounded, but unlabelled parallel
   cyan/gray lines require hover/selection before their meaning is apparent. The
   ten-element agent view sits at 46.9% zoom, with truncated names and substantial
   unused vertical space. **Acceptance:** useful relationship identification and
   comfortably readable engineering names at the default neighborhood camera,
   while keeping selective rather than simultaneous labels for dense cases.
6. **P3 — Requirements still exposes generic presentation terms.** subjectWorkspace
   and preservesPriorState use Element and question-mark outliner icons. The
   header says `1 requirements`. The actual SubjectMembership then FeatureTyping
   path is correct and readable; polish should humanize only known kinds rather
   than infer satisfaction or verification semantics.

## Recorded interaction and geometry

The journey passes the formerly failing dependency action, real Graph and
derived-edge selection/Explain, the canonical Requirement subject path, History
comparison and atomic return to Current. Its Create Part dialog now opens the
actual pinned owner's focused Architecture projection before allowing preparation.
These are recorded application assertions, not deductions from screenshots.

During actual preparation, ordinary input selects baseline ViewService
`1ce77981-59b7-5a69-b978-88f84b7d1f59`. Its exact baseline Inspector returns while
`mutation_pending` remains true: 2,884 ms after the observed request and 2,945 ms
after selection. The retained report includes the actual DTO, request, runtime
epoch and revision binding. This establishes a current-revision read completed
before candidate preparation; it does not make the query instantaneous or measure
physical input-to-photon latency.

As a presentation-version comparison on this identical baseline, corresponding
node top-left coordinates have these world-unit displacements from run04 to
run05: System, all seven zero; Repository, all four zero; Platform, 18 nodes,
median 10, nearest-rank p95 20, maximum 20. The small Platform shift reserves the
new port header. This is not a small semantic-edit displacement benchmark.

The completed preparation observation spans 140,216 ms with 23,136 observed raw
input hooks and a maximum hook gap of 13 ms. A real camera pan and the Inspector
response occur while mutation is pending. This passes the driver's declared
250-ms maximum-gap bound. It is not a 60 Hz or physical-presentation qualification.
The full preparation assertion takes 143,240 ms, a different enclosing scope.

The History/diff sidecar separately records 37.170 ms scene build for 130 scene
nodes and 405 scene edges, with layout/routing/diff included. Its rolling
240-sample CPU update interval is median 6.060 / p95 6.492 ms. GPU timestamps
cover only the scene pass: median 0.024576 / p95 0.044032 ms, with two timestamp
errors recorded. Text, chrome, upload and presentation are excluded from that GPU
scope. This automated repaint cadence is not a desktop-vsync or input-to-photon
benchmark; no physical presentation or general 60 Hz claim follows from it.

## Candidate and durable continuation

The final [journey](../real-run05/journey.json) is **failed**, 504,943 ms, with 25
passed assertions and the failing first Candidate-mode check. It contains an
actual Working candidate `19f46877-550e-4d05-95da-f7b4dcc74e72`, adding
`65a22118-37dd-51dc-9256-3d519aa0050f` under the exact original ModelingPlatform
owner. The same-lens added-part and source-backed construction checks passed.

For the failed step, both recorded selections are
`Node(ElementId(38420268876060022078063346281745293145))`, the inspected
ViewService. Both scene/selection/Inspector revisions are the actual candidate
revision, and both cameras are `[160, 348, 0.55172414]`. The only intended mode
change is Diff -> Candidate. The evidence therefore does not support the error
label's assertion that selection was lost. This reviewer initially relayed that
label, then corrected the diagnosis after examining the states and driver order.

No `08-candidate` or candidate-diff screenshot was emitted, and no validation,
commit or independent restart passed in this run. The failed report remains
failed. Its final metrics field is null; the preparation evidence and earlier
captured sidecar metrics above remain separately available.

Exact PNG and sidecar SHA-256 identities, plus the failed journey and predecessor
report digests, are retained in the accompanying
[review evidence manifest](real-run05-editor-evidence.json). These bind this
review to the inspected artifacts without rewriting any original capture.
