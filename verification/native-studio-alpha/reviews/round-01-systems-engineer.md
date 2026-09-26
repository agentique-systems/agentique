# Round 01 — systems engineer review

Reviewer: independent systems engineer persona. Review basis: direct inspection of all eight native screenshots in `../round-01-before/`. This is a screenshot review, not evidence that an interaction or semantic operation passed. Every image is explicitly a visual fixture.

## Judgment

**Not yet alpha-quality.** I can read the initial three-subsystem hierarchy and recognize a selected component, but I cannot confidently assess a proposed architecture change or reason about interfaces from these views. I would demo the direction with explicit limitations; I would not yet ask a systems engineer to trust this as a working engineering environment for thirty minutes.

## Ranked problems

1. **Candidate containment contradicts the stated architecture.** In `08-candidate.png`, the selected ScenarioNestedPart has Owner ModelingPlatform in the inspector, but is visually inside the overlapping AgentFabric rectangle. The AgentFabric container overlaps much of ModelingPlatform. This is a serious engineering communication failure: placement suggests a different owner from the semantic inspector. The Current/Candidate/Diff controls and disabled validation are helpful, but the title says only “Candidate revision”; the dominant view should explicitly say “Working candidate · not committed.” Re-layout affected containers without overlap and preserve unchanged component positions where possible. The candidate must pass a visible containment check before review can inspire confidence.

2. **Graph World shrinks readable content into a narrow band.** In `03-graph-world.png`, the twelve-node graph occupies roughly the middle fifth of the canvas height, with tiny labels and very faint edges despite extensive available space. All nine relationship-family controls look similarly active and there is no on-canvas relationship legend. In `07-agent-view.png`, dimming unrelated nodes helps identify a neighborhood, but the useful result is still tiny and the agent's intent, authority, target revision and result summary are absent from the main view. Fit the relevant neighborhood at a useful text scale, give selected relationships readable labels, and communicate incoming/outgoing meaning. A graph that technically fits is not yet useful for reasoning.

3. **Ports and links lack engineering meaning.** In `01-system-world.png` and `02-focused-subsystem.png`, square connection surfaces and “2 ports” establish that ports exist, but their names, interfaces and connection state are not visible. The inspector lists request/result under a generic Features heading, without mapping them clearly to the left/right squares. Link labels are absent on the canvas. I cannot tell which port a highlighted connection actually uses. Show port names at the near level of detail, expose the selected port's interface and connections, and explicitly retain unknown direction where appropriate. Do not infer direction from the left/right placement.

4. **Requirements World shows associations without the relationship needed to interpret them.** In `04-requirements-world.png`, three rows of Part → Action → Requirement are visually clear, but the connecting lines have no satisfy/verify/subject labels and there are no linked/unlinked or verification-relationship summaries. The path around the top of each action could mean a direct association, satisfaction or something else. “Authored · projection” does not answer the engineering question. Name relationship roles on this small view and organize the inspector around subject, satisfied by and verified by. Do not imply verification results merely because an Action is present.

5. **Explain and history do not yet explain consequences.** `05-explain.png` shows a selected derived Typing relationship, identity values and an Explain button, but no causal explanation or evidence chain. Its honest fixture warning is essential and should remain. `06-history-diff.png` successfully differentiates added, changed and removed elements with text and borders, yet no comparison endpoints, branch, parent, validation state or concise structural change summary are visible. Its bottom status still says “Temporary dependency view · model unchanged,” which is incongruous with the displayed diff. A reviewer needs to know what two states are being compared and why each change matters before approving it.

## What the images do establish

- `01-system-world.png`: containment and selected-object synchronization between canvas, outliner and inspector are understandable at this scale; the calm background does not compete with structure.
- `02-focused-subsystem.png`: the breadcrumb and larger focused container communicate entry into ModelingPlatform. Static screenshots cannot establish camera continuity or keyboard navigation quality.
- `03-graph-world.png`, `07-agent-view.png`: graph and temporary-overlay concepts are present; useful reading scale and relationship interpretation remain weak.
- `04-requirements-world.png`: requirement and action shapes differ beyond color, but traceability meaning is not yet explicit.
- `05-explain.png`: derived origin and fixture limitations are disclosed. A causal Explain experience is not demonstrated.
- `06-history-diff.png`: removed elements remain visible as ghosts and change markers use words as well as color. Revision confidence remains inadequate.
- `08-candidate.png`: selection and preview limitations are visible, and unavailable Validate/Commit actions are disabled. The overlapping containment is the highest-priority defect.

The inspector begins with useful engineering identity/owner/counts, but generic “Features,” “Relationships,” and “Projection identity is preserved…” copy still consume space that should answer definition, interface, source and requirement questions. Horizontal and vertical panel scrollbars are conspicuous in several images; actions are clipped near the bottom of the inspector.

## Required follow-through for this round

Changes made: **pending integration lead entry; no unobserved fixes credited.**

After screenshots: **pending integration lead capture and exact paths.**

Re-review priorities: first confirm candidate containment, then inspect the graph at a legible scale, then confirm ports and requirement links communicate their semantic roles. The fixture gallery cannot establish real-model acceptance, durable candidate validation or commit correctness.

## Changes and after evidence

Integrated containment sizing and translated diff ghosts, stable columns for local insertion, boundary proxy ports, selected port/path labels, engineering Inspector, causal Explain and explicit candidate lifecycle. The gallery runner now retains monotonic input time during screenshot delivery and checks Explain window geometry. Cleared stale agent status on comparison.

After images: [System](../round-01-after/01-system-world.png), [Explain](../round-01-after/05-explain.png), [Candidate](../round-01-after/08-candidate.png); the full eight-image gallery is in `round-01-after`. The native input journey completed with exit 0 (`checks/gallery-round1-after.json`). Parent independently inspected the new Explain and candidate images: the modal is visible and ScenarioNestedPart sits inside ModelingPlatform without container overlap.

Remaining weakness: Graph overview text is small, requirements links still lack visible roles, and real semantic data remains pending. These observations do not establish alpha acceptance.
