# Real Native Studio review: editor interaction and agent workflow

**Judgment: not yet a confident Alpha demonstration.** The real current revision stays responsive while a candidate builds, and the revision-bound read workflow is credible. Candidate construction still takes 171.512 seconds. Selecting the new part leaves it offscreen in the Candidate screenshot. That is a product failure even though the original selection/Inspector assertion passed.

This review covers every one of the 16 actual 1600×1000 images in `verification/generated/native-studio-acceptance/real-run02/gallery`, with their semantic/layout sidecars. It does not substitute fixture pictures. Exact image and sidecar SHA-256 values, native executable identity, process command, exit code, and selected observations are retained in [real-run02-editor-ai-evidence.json](real-run02-editor-ai-evidence.json). This is a review of that binary, before the resulting source fixes are rebuilt.

## Editor interaction review

The largest criticism is visible in [08-candidate.png](../../generated/native-studio-acceptance/real-run02/gallery/08-candidate.png): `alphaStudioObserver` is selected in the Explorer and identified correctly by the Inspector, but is outside the viewport. The camera remains at 120% over an unrelated cropped portion of the owner. The subsequent [09-candidate-diff.png](../../generated/native-studio-acceptance/real-run02/gallery/09-candidate-diff.png) at 70% finally reveals it. An engineer should not need the next comparison mode to find the object they just selected.

Change prepared from this evidence: Explorer mouse/keyboard selection reveals an offscreen canonical scene object, preserving the current zoom when it fits. Already visible and partially visible objects retain the operator's camera. Reduced-motion mode moves immediately. Canvas selection keeps its existing behavior. Regressions cover offscreen, onscreen, partially visible, deselection, History, small viewports and several logical DPI scales. The real candidate assertion now requires the selected part's scene bounds to intersect the viewport after ordinary settling; no camera movement is injected by the automation. These fixes still require integrated tests and another actual capture.

Other criticisms are in [06a-earlier-revision.png](../../generated/native-studio-acceptance/real-run02/gallery/06a-earlier-revision.png), whose cards follow UUID order instead of lineage, and [06-history-diff.png](../../generated/native-studio-acceptance/real-run02/gallery/06-history-diff.png), whose initial comparison retains the unrelated requirement Inspector. [10-committed-history.png](../../generated/native-studio-acceptance/real-run02/gallery/10-committed-history.png) exposes raw edit-receipt JSON underneath a useful semantic title. The world team is preparing lineage ordering, initial durable comparison framing, and an explicit receipt disclosure. Those changes are not yet validated by these images.

Positive evidence is substantial. The focused System view exposes real port and interface relationships; inherited `repositoryRevisions` still belongs to the original Repository. The two actual earlier-revision visits and Return-to-head operations restore the saved world, semantic focus, selection and camera. Requirement exploration before and after the first visit has identical image SHA-256. The Graph return retains center and zoom; the viewport height changes appropriately when the Diff toolbar closes. This is stronger than merely comparing a parent revision without opening it.

Remaining weakness: responsiveness does not make a three-minute modeling command acceptable. Dense full-graph labels and crossings still need separate adversarial review. This run does not establish sustained memory stability or physical input-to-photon timing.

## Agent workflow review

[07-agent-view.png](../../generated/native-studio-acceptance/real-run02/gallery/07-agent-view.png) clearly identifies the built-in semantic query agent, dependency intent, ModelRepository subject, input revision, result completeness, and temporary read-only result. Its separate view recommendation is labeled `Deterministic view mock`; it does not pretend to be a general LLM provider. Provenance is inspectable. [05-explain.png](../../generated/native-studio-acceptance/real-run02/gallery/05-explain.png) shows an actual rule/evidence path, explicitly partial proof, and a bounded subset of supporting facts rather than claiming to display the entire derivation.

Largest criticism: the recommendation repeats Graph World after that world is already open and occupies considerable Inspector space. The dependency graph is fitted at 46% with several truncated names, so the visual result takes more reading than its ten-element size suggests. The useful engineering operation itself is credible within its stated scope.

Change during this review: no broader agent capability was added. The product work remains focused on selection visibility, comparison context, and accurate semantic labels. The world team is correcting generic `ELEMENT`/question-mark rendering for the real requirement's ReferenceUsage and ConstraintUsage. [04a-requirement-neighborhood.png](../../generated/native-studio-acceptance/real-run02/gallery/04a-requirement-neighborhood.png) already honestly shows the one real obligation and its subject/type path; it explicitly reports zero satisfaction and verification declarations. This is no proof of requirement satisfaction.

Remaining weakness: recommendation chrome could be smaller, dense names remain difficult, and the real self-model contains only one requirement. This review supports an Alpha-quality bounded dependency/Explain agent experience, not general agent or dense requirement population qualification.

## Observed background work and boundaries

The native process exited 0 after 617.943 seconds; all 39 journey assertions passed. The report outcome is `journey_passed_restart_pending`, so this run alone does not establish restart persistence. Its first asserted Validated view occurred 420.593 seconds after scenario start, including fresh project creation and the source import commits. That value is not a warm-open measurement.

Candidate preparation was 171.512 seconds. During it, the driver observed 28,283 native input hooks with a maximum 14 ms gap and a real camera pan. Selecting `systemsModelingApi` on the unchanged Validated revision returned a matching Inspector in 315 ms from selection, or 254 ms from request observation, while the mutation worker was still pending. The response retained project `b55b83eb-e084-4d72-9208-695f37046ad4`, revision `a7f64fce-43b2-4be6-8fb5-69c16f646dc3`, runtime epoch 1 and request 51. This proves the observed background interaction; it is not a 60 Hz or physical-presentation qualification.

The Validate UI step took 2,592 ms and Commit took 860 ms. These assertion intervals include intentional automation lead/settling frames, so they are not pure service timings. The service log separately records `commit_prepared.total` at 133.9804 ms, predominantly the 132.4632 ms repository commit. Process peak working set was 6,769,532,928 bytes; 250 ms process-tree sampling observed 6,752,354,304 bytes. Those peaks span fresh startup and candidate work, not a leak-controlled soak.

## GPU timestamp investigation

The run retained 32,085 positive scene-pass timestamp samples and 17 entries in the aggregate `gpu_timestamp_errors` counter. Its process log has no graphics surface or device fault messages. Small Graph/Requirements pass samples are quantized in 0.001024 ms increments. Zero-duration quantization is consequently a plausible explanation for some samples, not an established diagnosis of the historical count.

The implementation combines four different observations: device poll error, mapping error, a zero or reversed timestamp pair (`gpu_timing.rs`), and surface-epoch invalidation (`gpu.rs`). A changed surface epoch correctly discards in-flight timing buffers because eframe may drop an encoder after failed acquisition. `surface_recovery.rs` advances the epoch on surface/device failure and prints fault messages. The retained aggregate cannot identify which cause produced these 17 entries. Future instrumentation should separate those causes and retain valid measured zero intervals. No hardware device-loss or device-recovery qualification is claimed here.

## Verification performed for the resulting interaction patch

`rustfmt --edition 2024 crates/studio-native/src/actions.rs crates/studio-native/src/real_automation.rs` exited 0. `git diff --check` exited 0. The only emitted warning concerned a separate workflow file's CRLF conversion. No local compile/test or second native workload was started while the integration lead's actual soak was running. Integrated test and recapture results belong to the later build receipt.
