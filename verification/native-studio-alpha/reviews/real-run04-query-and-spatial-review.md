# Real run04: query latency and spatial review

**Judgment:** the real hierarchy and inherited-port story are understandable, but this run does not pass the operator journey. The dependency view was rejected before Graph/agent screenshots or candidate work. Loaded Inspector/focus delays remain a major product issue despite inexpensive scene building.

Evidence is the actual [run04 process](../real-run04/process.json), [journey](../real-run04/journey.json), [stderr](../real-run04/process.log), and [independent read audit](../checks/real-run04-view-profile-audit.json). The binary SHA-256 is `3e0eb1fc0c5ffd1e0ce7fe9faa669f721d907d960edca8741ebb59cfb470772c`; the launch receipt records source HEAD `aaaba8bf44487e5947ba2b50354adbe2f802b2f6` and the integration lead identifies executable build source `655f7dd`. This review launched no build or runtime consumer.

The auditor read 20,128 bytes once at 2026-09-25T21:06:27.567132Z, SHA-256 `eec2423379e02b9cf9dc62019ae08d305d2d77db45294067b09a22d21ab0b88d`, containing 12 completed profile records. All five image hashes independently match their metadata. Each is explicitly real semantic data with fixture null, project `07d1f9bd-8f5b-4811-a9ab-7dd6408f91d9`, revision `62af4537-f2c9-4bf1-90d6-e0862d6b4141`. Focus names in the audit are joined only through matching project/revision/ElementId in retained projection DTOs.

## Actual phase measurements

All timings below are per-call application wall time in milliseconds. Inclusive parent phases are not summed with their children. They exclude the service queue, scene rendering, screenshot settling and frame presentation; none is physical input latency, GPU time or a p95 estimate.

| Operation | Total | Context construction | Share | Other substantial work |
|---|---:|---:|---:|---|
| System overview projection, record 1 | 2768.5894 | 2719.8821 | 98.24% | Connector queries 31.2031; local population 11.4824 |
| ModelingPlatform projection, record 4 | 5582.5397 | 2662.5128 + 2654.3708 | 95.24% | Focused interface queries 217.8959; connectors 30.1793 |
| ModelRepository projection, record 8 | 5512.9052 | 5332.7397 | 96.73% | Remaining operation 180.1655 |
| ModelRepository dependency projection, record 12 | 2699.9064 | 2654.4157 | 98.32% | Returned 10 nodes / 13 edges; response subsequently rejected by native request fence |
| Inspector, 8 calls | median 5507.0412; range 5479.9762–5624.5059 | median 5320.8538 | median 96.61% | Type queries 101.5073–222.1876; connector queries roughly 28–31 |

Inspector constructs a KerML evaluator and then a separate SysML evaluator for its profile label. For ModelingPlatform record 3 these are 2655.2609 and 2651.6367 ms respectively, versus 221.5545 ms of type queries. Focused Architecture similarly constructs separate connector and effective-interface evaluators. This confirms the expensive API boundary; assigning the exact cost to registry construction versus graph/certificate hashing still requires finer internal measurement.

Every record sees 76,147 canonical records, including 75,362 standard records and 785 local records. Exactly one local connector query runs; graph extraction sees 526 edges. The small visual scenes are not the seconds-scale cause: retained scene builds are 1.9973 ms at initial overview, 0.3019 ms at ModelingPlatform and 0.0577 ms at ModelRepository. These are individual scene-build observations, not stress benchmarks.

The first bounded optimization candidate is reuse of one already authenticated evaluator within a single projection call, with exact DTO, evidence, completeness, error and query-order parity. Inspector reuse must preserve its current SysML-profile fallback behavior. Removing one approximately 2.65-second construction is an opportunity, not a measured speedup. A repeated-view cache would not solve first-selection or uncached Inspector latency. Do not weaken publication authentication or treat a cached ElementId as revision authority.

## Journey outcome, separate from timing

The fresh run passed actual accepted-runtime authentication, validated self-model opening, ModelingPlatform focus, real query-connection port inspection, ModelRepository focus and original inherited-port inspection. The recorded focus stages were 11,386 and 11,187 ms; these include asynchronous projection, Inspector and driver settling, and are not isolated projection timings.

The process ended exit 2 after 551.667 seconds. Step 9 failed because the native request expected a ViewDefinition named `Semantic Graph`, while `StudioPlatform::dependencies` returned `Dependency neighborhood`. The exact-definition fence correctly refused the response. Its successful 10-node semantic projection does not establish an installed Graph view or agent overlay. There are no new Graph, agent, Requirements, Explain, Diff or Candidate screenshots, no preparation responsiveness result, no commit and no restart qualification from run04. The integration lead is fixing common request/response construction separately.

## Actual image critique

| Actual image | Improvement against run03 | Remaining concern |
|---|---|---|
| [System overview](../real-run04/gallery/01-system-world.png) | Seven readable objects at 119%; hierarchy is immediate; redundant ownership lines/labels are gone; ModelingPlatform now reports 8 actual parts. | Definition links lack a visible relation label at rest; this still asks a novice to infer the meaning of arrows. |
| [ModelingPlatform](../real-run04/gallery/02-focused-subsystem.png) | Eighteen scene nodes at 71%; both boundary port names now survive this level of detail; eight components are distinguishable. | The horizontal connection crosses the workspace and systemsModelingApi cards. Its port labels fall inside those cards even though both ports belong to ModelingPlatform. |
| [Selected clientQueries port](../real-run04/gallery/02a-platform-port.png) | Selected port now has a visible marker/name; Inspector clearly gives owner, SemanticQuery type, actual queryConnection/opposite endpoint and unknown direction. | **P1 visual engineering ambiguity:** the route disappears through component bodies and the clientQueries label appears inside workspace. Reading the Inspector repairs the meaning; the diagram itself still misleads. Reserve obstacle-free connection/label space without changing canonical ownership or inventing direction. |
| [ModelRepository focus](../real-run04/gallery/02b-focused-repository.png) | Four scene cards at 119%; actual Repository specialization and inherited interface context remain legible. | The breadcrumb compresses away the ModelingPlatform navigation context. Preserve meaningful focus history without pretending definitions have different canonical owners. Minor count grammar remains: “1 parts” / “1 ports”. |
| [Inherited repositoryRevisions](../real-run04/gallery/02c-repository-port.png) | Original port is selected on Repository; Inspector shows Repository owner and ModelRevision type. No inherited copy or false local ownership is implied. | The specialization/typing relationships could be more self-explanatory at rest, but this bounded engineering story is usable. |

Ranked remaining problems are: **P1** broken dependency handoff; **P1** multi-second Inspector and focus operations; **P1** ambiguous platform connection routing/labels. Breadcrumb continuity and count grammar are lower priority. System overview and inherited-port inspection improved, while overall System communication still needs connection work. Graph/agent visual quality remains unqualified by this run: the prior [run03 Graph](../real-run03/gallery/03-graph-world.png) and [agent](../real-run03/gallery/07-agent-view.png) criticisms cannot be closed using a rejected DTO.

A source-only follow-up reviewed History commit `3f73af4350589273d1ae2043312e12f409d65c4a`: RememberedGroup addresses the prior hidden-group chooser snapback and adds a focused state regression. This does not substitute for the missing real Diff screenshot.

