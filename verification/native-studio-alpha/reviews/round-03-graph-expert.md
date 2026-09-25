# Round 03 — graph visualization expert

Review date: 2026-09-25. Independent screenshot review; no implementation changes by this reviewer.

## Evidence and scope

Viewed all eight actual native captures with `view_image`:

- [System World](../round-02-after/01-system-world.png)
- [Focused subsystem](../round-02-after/02-focused-subsystem.png)
- [Graph World](../round-02-after/03-graph-world.png)
- [Requirements World](../round-02-after/04-requirements-world.png)
- [Explain](../round-02-after/05-explain.png)
- [History diff](../round-02-after/06-history-diff.png)
- [Agent dependency view](../round-02-after/07-agent-view.png)
- [Candidate](../round-02-after/08-candidate.png)

Every capture explicitly identifies visual fixture data. These images establish visual findings only, not real semantic correctness, hover behavior, interaction latency, or accessibility qualification.

## Product judgment

Graph World is still foundation quality. The five-node agent neighborhood is a more useful reasoning view than the twelve-node Graph World. The latter presents small cards distributed across a wide horizontal band with almost invisible, unlabeled relationships; an engineer cannot answer what a relationship means from the scene. System containment is readable, but connectivity loses meaning when focus or LOD changes. Requirements is closer to useful at this fixture size, with significant endpoint-identity and long-name weaknesses.

## Ranked problems

### 1. Graph World's default view suppresses the information that makes it a graph

**Severity: high. Evidence: 03, also 02 and 08.** At 49% zoom the twelve cards occupy roughly the bottom-middle quarter of the graph canvas. Relationship paths and arrowheads are barely distinguishable from the dark background and dot grid. No relationship labels are visible, including the selected ModelingPlatform's relationships. The nine relationship-family buttons do not explain line appearance, direction, or authored/derived distinction. All nine initially look active, providing little prioritization.

**Change requested:** Set a useful minimum screen-space relationship contrast and arrowhead size independent of node LOD. Keep the selected object's immediate relationships legible while de-emphasizing unrelated paths. Use a compact visible legend for family, direction, and derived style. Choose a more compact initial graph layout or focused neighborhood so ordinary names and relevant labels survive fitting.

**Acceptance evidence:** New Graph World screenshot must make a named directed relationship traceable at the initial fit without opening the inspector; selected-neighborhood emphasis must also work when the selected item is a container/definition.

### 2. Shared path segments and crossings can be read as semantic junctions

**Severity: high. Evidence: 01, 04, 06, 07.** In the agent view, the left `contract` path crosses the vertical `revision-bound service` route, while the corresponding two right-side routes repeat the pattern. They share the same bright style; crossings have no bridge or separation. In Requirements, `satisfies` and `verifies` converge onto the same final horizontal segment and arrowhead into each requirement. The upper path's identity disappears just where its target should become most explicit. System World's two right-side relationships are similarly crowded into a narrow vertical corridor beside ModelingPlatform.

**Change requested:** Separate parallel segments and target lanes; place arrowheads at distinct endpoint anchors. Distinguish non-junction crossings geometrically. Retain separate hit targets and individual relationship inspection. Do not add bundling until an individual edge can be traced reliably.

**Acceptance evidence:** Select or hover each of the two routes separately and capture its complete source-to-target path, including its own endpoint. A crossing must not imply connectivity between the crossed relationships.

### 3. Focus and zoom discard boundary connectivity without explaining the loss

**Severity: high. Evidence: 01 versus 02; 07; 08.** The System view exposes square port marks, but only selected ModelRepository reveals `request` and `result`. The focused ModelingPlatform shows three cards and six anonymous squares; its external connections vanish from the visible context. No boundary proxy port, external-neighbor stub, or omitted-connection count tells the operator that the subsystem communicates with other systems. Internal paths are faint and unlabeled when the container is selected. At candidate summary zoom even port marks disappear, with no visible replacement summary.

**Change requested:** Expose selected/focused port names and known interface information at readable scale. Preserve external connectivity through explicitly labeled boundary proxies or a truthful connection summary. Mark unknown direction as unknown; do not assign engineering direction from the drawing's left/right placement. Ensure container selection reveals internal relationships as useful context.

**Acceptance evidence:** Capture focused ModelingPlatform with a comprehensible external connection and selected port identity. Add a collapsed-subsystem capture: none of the supplied eight images demonstrates proxy-port behavior, so that requirement remains unqualified.

### 4. Global fitting breaks comparison scale and hides the reason for change

**Severity: medium-high. Evidence: 01, 06, 08.** The system screenshot is at 104%, history diff at 99%, and candidate at 69%. A single newly added nested part produces a much smaller whole-system presentation, with unchanged cards and relationships less readable. The added node sits at the bottom of a tall container while much of the horizontal canvas is empty. The comparison header reports counts but does not name the change. The green added/selected treatment also competes with the normal blue selection vocabulary.

**Change requested:** Preserve camera scale and unchanged positions during revision/candidate switching; bring a new part into view locally or provide an explicit focus-change action. Keep selection recognizable independently of diff status. Show a short named structural-change summary and permit focusing it.

**Acceptance evidence:** Before/after captures at the same camera show the same selected unchanged node in the same place, plus the added node near its owner. Report median/p95 unchanged-node displacement separately from camera movement. Screenshots alone cannot establish this metric.

### 5. Engineering identity and relationship roles become unreadable before the diagram is actually dense

**Severity: medium. Evidence: 03, 04, 05, 06, 08.** Graph cards use very small names despite having only twelve elements. `Responsive spatial navigati...` truncates a requirement title at 111% zoom; the full requirement wording is the primary reason to visit that world. Long `RevisionModelingService` nearly consumes its System card width. Explain shows a generic three-box rule pipeline; it does not visually connect concrete source object, rule identity, and resulting relationship. Generic type/origin text consumes card space while key names and relationship roles lose detail.

**Change requested:** Wrap important requirement and component names with bounded measured card growth; prioritize names over repeated `Authored` captions. Add concise column/role labels to Requirements. Make Explain's causal nodes concrete and navigable when real evidence exists. Keep fixture explanations explicitly illustrative, as they are now.

**Acceptance evidence:** Capture adversarial long and Unicode names in Graph and Requirements at normal initial scale without unreadable labels or overlaps. Real Explain must show actual evidence identities before semantic acceptance.

## Specific qualification status

| Concern | Judgment from these captures |
| --- | --- |
| Edge identity | Selected individual edge in 05 is traceable; most default Graph edges are not readable enough. |
| Cross-container routing | Visible, but narrow shared corridors and ambiguous intersections remain. |
| Proxy ports | Not demonstrated. Focus loses visible external connectivity. |
| Path labels | Useful in selected System/agent views; absent in default Graph and focused container. |
| Selection | Clear blue selected Part border; definition selection does not visibly expose useful graph context. Diff selection needs stronger independent treatment. |
| Graph versus hierarchy | Different layouts are present, but initial Graph offers less engineering understanding than hierarchy. Agent neighborhood is the preferable starting point. |
| Requirements | Relationship presence is truthfully distinguished from verification success; path convergence and truncated requirement identity impede use. |
| Explain | Causal diagram is present, but fixture-generic and therefore unqualified for real provenance understanding. |
| Diff/candidate | Added/changed/removed states are visible; scale and layout continuity require measured evidence. |

## Changes made and after evidence

The integration team changed the default Graph entry to the selection's immediate neighborhood, retaining an explicit loaded overview. [Graph after](../round-03-after/03-graph-world.png) now presents five readable nodes at 100%, with named incident relationships. Ordinary node attachment lanes are separated; [Requirements after](../round-03-after/04-requirements-world.png) shows distinct satisfies/verifies arrowheads. Scene tests also select each final corridor independently and retain exact modeled port coordinates. [Focused subsystem after](../round-03-after/02-focused-subsystem.png) now states that six connections continue outside the focus and offers their graph context. Requirement names have two lines; the awkward final-letter word break visible in this capture is being corrected in round four.

The complete [native journey](../round-03-after/journey.json) passed through actual egui input and captured all eight surfaces. These are explicitly visual fixtures, not real-model acceptance. Arbitrary crossing bridges, candidate camera preservation and concrete real Explain evidence remain unresolved. Local insertion geometry displacement is separately measured in the scene layout evidence; this does not establish camera continuity.
