# Round 04 — editor interaction review

Reviewer persona: independent game/editor interaction designer. Reviewed 2026-09-25.

Would I confidently demo this as an alpha engineering editor to a serious systems engineer today? **Not yet.** I would demo the spatial direction as work in progress. Selection, containment and the readable neighborhood are credible, but the operator still cannot rely on a consistent keyboard action model or preserve attention when moving into candidate review. These are interaction defects rather than missing decorative polish.

## Evidence and limits

I inspected all eight actual native captures, their state evidence and [the native input journey](../round-03-after/journey.json). All 41 recorded assertions passed. The journey explicitly covers deterministic visual fixtures, not semantic validation or a durable commit. I also inspected current native actions, viewport, panels, navigation, app and update code. I did not drive a fresh interactive session; code findings below are distinguished from captured behavior.

| Capture | Interaction observation |
| --- | --- |
| [01 System](../round-03-after/01-system-world.png) | Selected repository has a strong outline and matching Explorer row; architecture is readable at 104%. Inspector actions disappear below the fold. |
| [02 focused subsystem](../round-03-after/02-focused-subsystem.png) | Focus is visibly distinct, and the external-connection count avoids implying isolation. Breadcrumb and Up affordance make the location understandable. Ports remain anonymous until individually inspected. |
| [03 Graph](../round-03-after/03-graph-world.png) | Five-object neighborhood at 100% is substantially more usable than the complete overview. Incoming/outgoing/next-hop controls have no visible explanation of what their scope will be. |
| [04 Requirements](../round-03-after/04-requirements-world.png) | Requirement links are legible and terminate separately. The last title wraps into a lone `n`; parent is already addressing word boundaries. Bottom status still describes Graph overview. |
| [05 Explain](../round-03-after/05-explain.png) | The visible explanation window has an intelligible causal direction and a truthful fixture label. Its underlying complete graph is small; closing it should return the operator to the exact prior reasoning context. |
| [06 history diff](../round-03-after/06-history-diff.png) | Added, changed and removed objects are recognizable; the unchanged repository remains selected. This image shows a spatial diff, not the History page or its branch/revision interaction. |
| [07 agent view](../round-03-after/07-agent-view.png) | Temporary view and read-only authority are obvious. The result is immediately useful as a neighborhood; dismissing should restore the prior exploration state. |
| [08 candidate](../round-03-after/08-candidate.png) | Review mode and unavailable semantic actions are honest. The new child is selected, but the entire world shrinks to 69%, reducing the new object's legibility precisely when it matters most. |

The recorded input does establish double-click focus, F, Alt-Up, Alt-Left, selection and multiselection, pan, anchored zoom, world shortcuts, Explain opening/closing, a dependency view, a visual candidate and cancellation. It does **not** establish keyboard-only operation: each palette action clicks the palette input, part creation clicks controls, and object/edge selection uses pointer geometry. Reduced motion is enabled near the beginning and remains enabled, so this run does not qualify animated focus, back or forward. Candidate Current/Candidate/Diff switching is not exercised.

## Ranked problems and requested changes

### 1. The palette is not yet a keyboard command interface

**High, code-confirmed.** `panels.rs::dialogs` executes the first available command on Enter and has no highlighted result navigated by Up/Down. Element results below the command list only activate on a button click. Thus typing a specific element name does not provide the advertised keyboard path to `Focus: ModelRepository`. `keyboard()` handles global outliner arrows only after returning early when a text input owns focus. The automated journey clicks the search field and cannot detect this gap.

Requested change: keep one explicit result cursor spanning enabled command and element results, give it a visible highlight, move it with Up/Down and execute it with Enter. Clear or select the previous query on opening. Make disabled reasons visible without allowing Enter to silently invoke a different action than the highlighted row. Provide a keyboard path through the create-part name and Prepare action. Do not implement durable revision undo under Ctrl+Z; either define a narrowly presentation-only action or report that shortcut as unfinished.

Acceptance: a new scenario must send no pointer events from palette open through element focus, creation dialog, text entry and candidate preparation. It should choose a non-first result with Down+Enter, prove the intended target ID, and exercise disabled actions without executing another result.

### 2. Context menus depend on stale selection rather than the pointed context

**High, code-confirmed.** `viewport.rs` selects a hovered target on secondary click, but an empty-canvas secondary click keeps the previous selection. Every context gets the same seven-item menu: Focus, Dependencies, Explain, Source, Create, Neighbors and Fit. A click on empty canvas can therefore offer a valid Create command against a previously selected part elsewhere. Port and relationship menus also contain disabled or irrelevant node actions. This is a practical wrong-target hazard, even though the semantic command itself remains validated.

Requested change: bind the menu to a captured hit target when it opens. Empty canvas should offer navigation/view actions; a node should show only actions meaningful for that node; a port should expose its owner, connections and explanation; an edge should expose the relationship and its endpoints. Avoid invalid clutter, while retaining explicit reasons where an unavailable lifecycle action helps understanding.

Acceptance: native scenarios open node, port, edge and empty menus, inspect their target identity and available action IDs, and prove that empty-canvas actions cannot create within a stale node selection. A menu stays bound to its opening context if the pointer moves away.

### 3. Candidate review breaks the mental map and forgets an absent selection

**High, captured and code-confirmed.** Journey `text entry prepares candidate preview` changes zoom from `0.9957507` to `0.69930875` and camera center Y from `353` to `434`; compare captures 06 and 08. A local child insertion produces a global reframe. `change_comparison()` rebuilds and `install_scene()` reconciles selection against the visible scene. A candidate-only selection disappears in Current and is not stored for restoration when returning to Candidate. In the live path, `Output::CandidateView` sets `fit_pending = true`, so mode queries can reframe again even if the user deliberately navigated the candidate.

Requested change: preserve shared-element selection, focus and camera across review modes; retain a separate review selection for objects absent in Current, while clearly communicating that absence. Avoid unconditional fit on every candidate or projection response. Prefer a restrained reveal of the changed object or an explicit Fit changes action, with the previous view available on cancellation. Never preserve an inspector bound to the wrong revision in order to achieve this.

Acceptance: select a new child, switch Diff → Current → Candidate → Diff, and assert revision-correct Inspector, recovered child selection and stable camera where bounds allow. Repeat with an unchanged node. Record a native screenshot sequence and unchanged-node screen displacement, not just world-coordinate layout displacement.

### 4. Spatial navigation is only partly animated and partly remembered

**Medium/high, code-confirmed, not motion-qualified by this journey.** Leaf Focus assigns `camera_target`, but container Focus and Up set `fit_pending` and `viewport()` performs immediate fit; Back/Forward set center and zoom directly and clear `camera_target`. Therefore entering the flagship subsystem is not using the existing interpolation. `Location` stores revision/world/focus/camera, but not expanded neighborhood, selection or agent overlay; `restore_location()` explicitly clears expanded neighborhoods. The same Back action cannot reliably recover a reasoning view.

Requested change: use one navigation transition policy for focus, fit, Up, Back and Forward, respecting reduced motion and interrupted manual gestures. Preserve enough disposable view state for Back to restore the last graph exploration. Update the current history entry when departing after pan/zoom and capture the destination after fitting, so stored cameras are meaningful. Keep this presentation history distinct from immutable semantic history.

Acceptance: native normal-motion and reduced-motion scenarios visit a subsystem, pan, enter a deeper object, go Back/Forward and verify restored focus/camera. Another sequence expands a graph neighborhood, opens a temporary agent view and returns to the prior neighborhood. Frame samples should establish interpolation only for commanded transitions and no animation during ordinary pan.

### 5. Status and shortcut copy can contradict what is visible

**Medium, captured and code-confirmed.** Requirements displays the status `Graph overview · focus a selection to inspect its neighborhood`. After fixture cancellation, the journey's status remains `Candidate visual preview · install runtime for semantic reconstruction` through History and back to System, despite `candidate: false`. `Cancel candidate` advertises Esc in the command list, but Escape actually closes a surface, clears selection or navigates Back; it does not cancel the candidate. These small contradictions undermine confidence in revision-sensitive operations.

Requested change: update status on lifecycle/world transitions, or derive persistent state labels from actual state rather than retaining arbitrary action text indefinitely. Remove the unsupported Esc label or define an explicit, safe candidate-discard keyboard action. Keep Escape's existing layered dismissal behavior predictable.

Acceptance: cancel a preview and assert the status states cancellation/current revision; change worlds and assert there is no obsolete Graph/Candidate assertion. Every advertised shortcut has a corresponding native assertion or is removed from the UI.

## Product judgment

System containment and selection are approaching alpha visual quality on this fixture. The neighborhood Graph and the new requirement links are useful. Candidate authority labels are correctly restrained. I cannot rate the full editor interaction alpha-quality while wrong-context menus, incomplete palette keyboard navigation and candidate reframing remain. The complete gallery is more coherent than a graph demo, but an engineer still needs guidance to discover the intended happy path.

No conclusion here establishes real-model behavior, true semantic candidate validation, durable commit/restart, device loss or screen-reader compatibility. The fixture journey is useful evidence with a deliberately narrower scope.

## Changes and after evidence

The integration changed context menus by target category and clears the old selection on an empty-canvas right-click. The command palette now has a keyboard cursor, arrow navigation, Enter activation and visible-object focus results. Candidate creation preserves the camera; switching Current → Candidate → Diff retains the candidate-only selection without exposing it in Current. Reframing is an explicit Focus changes command. World/cancel status text and the unsupported Escape shortcut were corrected. Focus, fit, Up, Back and Forward now share camera interpolation, although the retained journey exercises reduced motion.

The [new native journey](../round-04-after/journey.json) passed, including pointer-free palette search/arrow activation, camera equality at candidate creation and each mode switch, and candidate selection restoration. [Candidate after](../round-04-after/08-candidate.png) and [Requirements after](../round-04-after/04-requirements-world.png) are actual native GPU captures. The requirement wraps at a word boundary. The explicit Focus changes command still frames a large changed container too broadly; the next iteration will prioritize changed leaf objects. Back does not yet retain graph expansion/overlay state, and the entire journey is not keyboard-only.

An intermediate acceptance instrumentation defect deadlocked egui by asking for frame metadata while holding its context data lock. The process was stopped, the failure retained in checks/native-round-four-deadlock-state.json, and metadata is now obtained before entering the lock. The corrected native journey passed; no semantic operation occurred during that failed attempt.
