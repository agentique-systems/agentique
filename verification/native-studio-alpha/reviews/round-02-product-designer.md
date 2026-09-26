# Round 02 — independent product designer review

Reviewed all eight actual native screenshots in `verification/native-studio-alpha/round-01-after/` with image inspection. These are this round's **before** evidence. Every image is explicitly a visual fixture; this review makes no semantic-runtime acceptance claim. Review date: 2026-09-25.

## Product judgment

**Would I publicly demonstrate this as an alpha engineering environment? Not yet.** System World has become a coherent, restrained editor surface: containment is legible, selected objects have clear emphasis, the panel layout is consistent, and the removed ghost in Diff communicates a useful concept. However, the view defaults still undermine basic reading in Graph World, Requirements does not explain its relationships, and the agent result has almost no visible account of the question it answered. Those are comprehension failures, not a missing-decoration problem. A guided prototype demonstration is credible; an unaided thirty-minute engineering session is not yet evidenced.

## Ranked issues and practical changes

1. **Graph defaults shrink engineering names below useful reading size.** `03-graph-world.png`, `05-explain.png`, and `07-agent-view.png` put the architecture in a thin strip in the lower-middle canvas at 48–49% zoom, while several hundred pixels above and below are vacant. Graph labels appear approximately 8–9 physical pixels high; controls occupy more visual attention than the graph. Fit should use the actual graph viewport beneath the controls and prioritize a minimum readable node-label size. Give the graph a less wide initial layout, use additional rows where sensible, and keep label text readable independently of world scale at overview LOD. The dependency view should fit the relevant highlighted neighborhood, retaining dim context outside it. Acceptance: names of the five dependency nodes can be read without leaning in, and one can identify the selected node and its neighbors before reading the toolbar.

2. **Requirements World shows connectivity without saying what the connections mean.** In `04-requirements-world.png`, architecture parts, actions, and requirements are cleanly separated spatially but have neither column headings nor relationship labels. Two paths enter each requirement with no visible indication of satisfaction versus verification. The bottom requirement title truncates despite ample vertical space within the card. Add semantic column headings such as Architecture / Verification / Requirement when the underlying relationship families support them; label the relevant relationship paths with their actual family and provide a compact legend. Use the spare card height for a two-line requirement title. Show link-presence summaries rather than pass/fail. Acceptance: a new engineer can explain what Crash recovery acceptance does in relation to Durable engineering history using only the visible view.

3. **Agent output is visually anonymous.** `07-agent-view.png` has a tiny TEMPORARY VIEW marker and a footer saying temporary dependency view, but no prominent intent, target, revision, authority, or explanation of the highlight. All relationship-family buttons remain bright, so the operator cannot tell whether this is a query result or an ordinary filtered graph. Add a compact result strip: “Dependencies of ModelRepository,” selected agent/provider or “local semantic query,” target revision, result count, and “View only · model unchanged,” plus a clear Dismiss/Return action. Make active relationship filters visually distinguishable. Acceptance: a screenshot alone tells the operator what was requested, what is highlighted, and whether any model mutation occurred.

4. **Candidate and diff styling do not yet support confident review.** `08-candidate.png` correctly disables unsupported semantic actions and identifies a visual preview, but the new element is tiny and the mode is communicated chiefly in a large bottom bar away from the changed object. `06-history-diff.png` has an added/removed/changed legend whose entries share the same green text although changed/removed cards use amber; the new badge is amber even on a green outlined node. The comparison's baseline and target are only represented indirectly in the small footer. Add a compact, persistent comparison header near the world breadcrumb with baseline → target and change counts. Use matching text + symbol + color for the legend and node badges. Candidate review should frame the selected new object and its owner rather than automatically shrinking the entire architecture. Retain explicit “not committed” for real working candidates and truthful fixture language for these fixtures. Acceptance: current/candidate/diff, target revision, and the three change classes are discernible without searching the footer.

5. **Engineering meaning loses to repeated chrome and low-value metadata.** `01-system-world.png`, `02-focused-subsystem.png`, and `06-history-diff.png` dedicate repeated card space to PART / port count while most ports have no visible name; only selected ModelRepository exposes request/result. `02-focused-subsystem.png` enlarges three nodes but the relationships become faint unlabeled right-side lines, losing information exactly when the operator focuses in. Inspector sections include empty Relationships and generic “Projection identity is preserved…” text, while actions are clipped below the viewport in the longer inspectors. At focused/near LOD, surface available port names and selected/hovered relation names. Put high-value actions under the identity block or in a fixed inspector footer. Omit empty sections or say explicitly “No relationships in this view.” Move implementation assurance text to Advanced. Acceptance: focusing ModelingPlatform reveals more meaningful connection detail, and Explain/Dependencies are reachable without discovering a hidden scroll region.

## Coverage of the eight images

| Screenshot | Observed strength | Remaining product weakness |
| --- | --- | --- |
| 01 System | Containment and selected component are immediately apparent. | Labels cross near the selected card; unrelated connectivity is very faint and ports are mostly anonymous. |
| 02 Focus | Breadcrumb and focused container make the navigation destination clear. | Large empty margins and unlabelled internal connections provide little new engineering understanding. |
| 03 Graph | Stable familiar panels and explicit relationship families. | Tiny names, faint edges, and an overbearing toolbar dominate the result. |
| 04 Requirements | Shape/category differences make the three kinds of object distinguishable. | Missing relationship meaning, no column headings, avoidable title truncation. |
| 05 Explain | Small causal sequence is preferable to a raw evidence dump, with honest fixture disclosure. | The generic “Authored intent → Semantic rule → Derived relationship” explains the mechanism, not this selected relationship; real evidence remains untested. |
| 06 History diff | Removed ghost remains spatially present; added/changed positions are comprehensible. | Baseline/target and change summary are missing from the principal view; legend colors disagree with nodes. |
| 07 Agent | A highlight overlay creates a visible result without mutation. | No visible agent/intent/authority result context; selected neighborhood remains too small. |
| 08 Candidate | Current/Candidate/Diff and unsupported validation are stated honestly. | Entire-world fit shrinks the object under review; comparison context is weak. |

## Changes and after evidence

This independent reviewer implemented no features. Changes and new screenshots are **pending the integration lead's iteration**. The integration lead should append the exact changes made, after-image paths, and remaining weaknesses; this before-only review must not be counted as a completed before/after quality loop until that evidence exists.

## Limits

This is a static product-design review. It does not certify keyboard behavior, motion, hit targets, scrolling, runtime correctness, or performance. The screenshots do not exercise long qualified names, Unicode, mixed DPI, high contrast, or all panel states. Fixture clarity must not be presented as real Agentique semantic acceptance.

## Changes and after evidence

After this review, dependency results now frame the actual one-hop neighborhood (5 visible nodes at 99% rather than the 12-node overview at 49%). Agent intent, target revision, read-only authority and Dismiss are visible above the Inspector. Requirements show actual satisfies/verifies labels and literal link coverage, and scene name minimum size increased to 12 logical pixels. Diff breadcrumbs show before/after revision IDs and added/removed/changed counts.

After images: [Agent view](../round-02-after/07-agent-view.png), [Requirements](../round-02-after/04-requirements-world.png), [Diff](../round-02-after/06-history-diff.png). The full gallery and 39-step native input report are in `round-02-after`; `checks/gallery-round2-after.json` records exit 0. Parent inspected Requirements and Agent images directly.

Remaining weakness: full Graph overview still requires focus for detail; long requirement titles elide; formal semantic status remains unqualified without runtime. No public alpha acceptance follows.
