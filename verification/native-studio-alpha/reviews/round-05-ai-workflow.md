# Round 05 — independent AI-native workflow review

Reviewer persona: AI-native engineering workflow designer. Reviewed 2026-09-25. This review inspected all eight native GPU captures in `round-04-after`, their state sidecars, the passing native input journey, and the agent, candidate, Explain, and decision presentation code. It did not run the accepted runtime or observe a real semantic commit. The screenshots consistently identify themselves as visual fixtures; that qualification must survive any demo or report.

## Product judgment

**Would I publicly demo this as an alpha engineering agent workflow? Not yet.** I would demonstrate the spatial interaction work as an explicitly qualified preview. The dependency action now produces a readable, revision-bound visible result, and the candidate bar makes review authority considerably clearer than a chat transcript would. But the operator still cannot read a complete account of who acted, on which original target, with what result and authority. The only demonstrated candidate cannot be validated or committed. Neither a passing fixture journey nor the presence of production code establishes that the intended human/agent loop works on Agentique.

The strongest product direction is the separation between exploration and proposal approval: temporary read-only views, retained candidate review modes, and disabled fixture validation/commit. Keep that division. Strengthen its visible explanation before adding more agent actions.

## Screenshot evidence

| Native capture | Observation relevant to operator trust |
| --- | --- |
| [01 System](../round-04-after/01-system-world.png) | Names, ownership containers and selected relationships are readable. The product can support discussion of architecture. No agent presence is implied merely because agent components exist in the model. |
| [02 Focus](../round-04-after/02-focused-subsystem.png) | The notice that six connections continue outside the focus prevents a false impression of completeness. This is the right pattern for bounded agent results too. |
| [03 Graph](../round-04-after/03-graph-world.png) | The repository's small neighborhood is legible and visibly selected. Relationship filters provide operator control over the scope. |
| [04 Requirements](../round-04-after/04-requirements-world.png) | Satisfy and verify are distinguishable, and the text explicitly declines to equate their presence with verification success. This supports honest review, although it is still fixture evidence. |
| [05 Explain](../round-04-after/05-explain.png) | The causal strip is understandable and the amber fixture disclaimer is unambiguous. Its generic authored-intent/rule/relationship text does not demonstrate an explanation of an actual derived fact. |
| [06 History/diff](../round-04-after/06-history-diff.png) | Added, changed and removed objects remain visible; the revision pair and counts establish a comparison. There is no short intent or engineering consequence summary, so the operator must reconstruct the purpose from renamed objects. |
| [07 Agent result](../round-04-after/07-agent-view.png) | A temporary view, target revision, visible element count and read-only authority are visible. The original query subject, named actor/provider, completion state, and query bounds are absent. The graph looks the same as a normal manually requested neighborhood. |
| [08 Candidate](../round-04-after/08-candidate.png) | Current/Candidate/Diff and disabled Validate/Commit are explicit. The candidate is only 58% scale despite requesting Focus changes. Its inspector also reads `owns → ModelingPlatform` while identifying ModelingPlatform as its owner; this reverses the natural reading of the containment fact. |

## Ranked changes

1. **P1 — make agent activity an accountable result, not an anonymous view label.** The visible card says only “Agent intent · show related architecture.” Retain the original selected query subject and exact target revision when the action is requested; do not recompute the intent from whatever becomes selected later. Show an actor/action identity, pending/completed/failed state, result type, bounded scope, and read-only authority. Distinguish the real semantic dependency query from the illustrative lens-choice mock. A concise card can do this without introducing a chat panel. Acceptance: changing the inspected node after the result arrives must not rewrite what the action originally targeted; an obsolete completion must not label a newer scene as its own result.

2. **P1 — show decision provenance where its scores are shown.** Code in `agents.rs` supplies the fixed provider `native-illustrative-decision`, fixed values 0.17/0.72/0.08/0.03, and no confidence. The Inspector renders “Illustrative choice distribution,” but omits the provider name and places the entire decision section below ordinary element detail; it is not visible in capture 07. The scores are not observed semantic analysis or calibrated confidence. Put a small mock/provider label beside the suggestion, state that the values are illustrative and uncalibrated, and make the optional detail discoverable near the agent result. Do not imply that the mock performed the dependency query. Acceptance: an operator can identify the source and meaning of the numbers without opening implementation code.

3. **P1 — make relationship direction trustworthy during proposal review.** Capture 08's `owns → ModelingPlatform` reads as if the proposed child owns its parent. Inspection of the fixture node relationship code shows a counterpart is chosen for either end, then the same outward arrow is rendered unconditionally. Preserve actual endpoint order: show source → target, or clearly mark incoming versus outgoing. Never infer port flow from a directed semantic edge. Acceptance: both sides of an ownership relationship tell the same containment story. Review the production engineering relationship rows for the same readability issue even though this screenshot establishes only the fixture defect.

4. **P1 — focus a proposal on its changed engineering objects.** The explicit Focus changes action still includes broad changed containers and yields a miniature whole-system diagram. The author cannot inspect the new part comfortably while deciding whether to approve. Prefer changed leaf objects and enough ownership context, with an explicit fit-all escape. Preserve the camera when switching Current/Candidate/Diff. The parent is already improving this behavior; require an after image, not only the code change. Acceptance: the added part's full name and its owner are readable at the default review moment, with no overlap or loss of the existing comparison state.

5. **P2 — dismissing an agent result should restore the operator's place.** Current Inspector code clears focus and expansion then requests a new projection and fit. This is a reset, not a return from a temporary result. Save and restore the initiating presentation when practical, including focus, camera, world and filters, only if the revision still matches. The current screenshot cannot establish that return journey. Acceptance: request dependencies from a focused subsystem, inspect another node, dismiss, and return to the prior engineering context without a global camera jump.

6. **P2 — proposal review needs a small intent/consequence summary.** The candidate bar explains lifecycle but does not name the proposed operation and target owner; the diff strip provides counts but no human-readable purpose. “Add ScenarioNestedPart within ModelingPlatform” would make this example reviewable without deciphering revision identifiers. Show only actual declared changes and known structural consequences; do not manufacture rationale or claim a requirement is satisfied. This should also identify operator-originated work separately from any future agent proposal.

## Code and interaction evidence, with limits

The [native journey](../round-04-after/journey.json) passed its recorded checks, including dependency view creation, candidate selection, Current/Candidate/Diff camera and selection continuity, rejection of fixture validation/commit, cancellation, and immutable history navigation. This is useful interaction evidence. Its own scope explicitly excludes semantic validation and durable commit acceptance.

`CandidateReview` has distinct Working, Validated, commit-acknowledgement-pending and Committed text; Working/Validated retain “Not committed.” That is the correct authority boundary. The preparation panel also says that validation has not started and describes cancellation as discarding eventual reconstruction. These are code observations, not proof that a lengthy real reconstruction stays responsive, honors cancellation, and never swaps a stale candidate.

The real Explain path provides summary, causal diagram, bounded-evidence disclosure, exact evidence and advanced details. Capture 05 exercises the illustrative branch only. A genuine rule, its actual recorded evidence, and a human-readable conclusion remain mandatory real-model review evidence.

## Required after evidence and remaining unknowns

Before closing this round, retain a new agent screenshot showing accountable action identity and mock decision provenance, a candidate screenshot showing readable changes and correct ownership direction, and a passing journey after those changes. Record any criticism deferred instead of treating this review as approval.

Overall product acceptance additionally requires authenticated real Agentique first light and a real prepare → review → validate → commit → restart journey. Current evidence does not establish live query completeness, slow-operation responsiveness, stale-result rejection under overlapping requests, failed-provider recovery, durable commit recovery, or successful reconstruction cancellation. No live general-purpose LLM integration is requested or needed to fix these presentation and authority issues.

## Integration follow-up

The integration now retains the dependency request's original subject and revision, reports running/completed/failed query state, and names the built-in query separately from the illustrative decision mock. The visible card explicitly says that the mock's 0.72 weight is not calibrated confidence. Live projection completion populates actual result identities. [Agent after](../round-05-after/07-agent-view.png) shows this accountable result card. Incoming relationship labels now follow the actual directed endpoints; [candidate after](../round-05-after/08-candidate.png) shows `owns ← ModelingPlatform`. Explicit Focus changes prioritizes changed leaf objects so the new component is readable. Its owner is stated in the Inspector; the large container header may be above the viewport.

The [native vertical journey](../round-05-after/journey.json) passed after these changes. A separate [keyboard-only journey](../keyboard-journey.json) passed navigation, inspection, dependency view, Explain, requirements, source-free component preparation, review mode switching, cancellation and History using key/text input only. These are fixture interaction qualifications, not semantic validation or commit acceptance.

Dismissal still resets to the loaded view instead of restoring every earlier exploration setting. The candidate header does not yet retain a rich operator intent/agent rationale summary. These criticisms remain open; this completed review loop does not grant overall alpha or real-model acceptance.
