# Native Studio alpha product judgment

Would we confidently demo this Native Studio to a serious systems engineer today?

**Not yet as the promised alpha product.** The demonstrated real-model workflow
is credible: an operator can understand architecture, inspect real interfaces,
ask for dependencies, create a nested part, review its alternate future, validate,
commit and reopen it. Requirements and dense design differences still need better
spatial communication. First-use semantic reads take seconds, candidate preparation
takes over two minutes, and the final 10k zoom qualification was inconsistent.
Those limits prevent a confident general thirty-minute engineering session.

**AGENTIQUE NATIVE STUDIO ALPHA NOT YET ACCEPTED**

The runtime blocker is resolved and real-model first light passed. This judgment
is about product quality. Work is retained on `platform/native-studio-alpha`,
created from fetched main `ded2a6e7dc616b9305508da2976d65282ebd7b4d`.
The final scope was frozen when the operator requested a wrap-up.

## Runtime

| Question | Result |
|---|---|
| Original cache recovered | No; the final historical/local/CI search did not recover the accepted pair. |
| Cache rematerialized | Yes, from the frozen accepted inputs/profile/producer contract. |
| KerML authenticated | Yes, accepted Operational v9 `81557335…74f12`. |
| SysML authenticated | Yes, accepted Systems v3 `25aeddb0…193fa`. |
| Runtime bundle | `accepted-runtime.agq-runtime`, 610,190,454 bytes; SHA256 `37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026`. Pack, verify, offline install and downloaded-asset verification passed. |
| Real Agentique repository | Yes; real six-document model reached Validated, then created and durably committed revision `9539bfed-f725-43d9-b5f8-8254040e7d42`; a separate process verified it. |

The Systems rematerialization has a versioned transport receipt. The original
semantic publication authority, receipts, bindings, profiles and original library
bytes remain preserved; no v10/v4 or broader conformance was created. Three
reviewed source fingerprints were refreshed only after exact reconstruction and
audit parity proof. The accepted 69-package language lock closure is unchanged.
See [runtime evidence](runtime/README.md), [transport review](reviews/runtime-transport-independent-review.md)
and [final freshness review](reviews/real-run06-editor-interaction.md).

The [content-addressed distribution procedure](../../docs/runtime-publication-distribution.md)
and release-asset workflow are present. The retained asset is a maintainer-access
**draft release**; public runtime distribution and execution of the new release
workflow remain outstanding. The [README](../../README.md#run-native-studio)
now explains native build/run commands, installation and explicitly labeled fixtures.

## Visual product

These ratings concern the actual bounded scenes reviewed in the [real gallery](GALLERY.md).
They do not certify arbitrary dense models or every interaction.

| Surface | Judgment | Evidence / remaining weakness |
|---|---|---|
| System World | Alpha-quality | Clear containment, focus and actual ports; external definition context can crowd the edge of the view. |
| Graph World | Alpha-quality | Useful bounded neighborhood with readable selected relationship labels; broad dense exploration needs more qualification. |
| Requirements World | Foundation | Exact subject-to-architecture path works; generic intermediate objects and limited traceability density remain weak. |
| Inspector | Alpha-quality | Engineering identity, owner, ports, provenance and source precede Advanced evidence. First uncached reads are slow. |
| Explain | Alpha-quality | Human summary and causal evidence diagram; partial support counts remain explicit. |
| History | Foundation | Branch/revision/validation and committed history are sound; complex owner comparisons remain visually dense. |
| Diff | Foundation | Added/removed/changed identity, owner context and ghosts are retained; the 130-node scene still clips long labels. |
| Candidate World | Alpha-quality | Working/not committed, Current/Candidate/Diff, intent/actor and validate-before-commit are clear in the real journey. |
| Agent overlays | Alpha-quality | Demonstrated dependency view changes the world and names intent, revision, authority and mock decision provenance. General autonomous engineering is outside this result. |

## Operator workflow

**The coherent real journey passed.** [Run06](real-run06/journey.json) passed 33
native input/state assertions in 534.436 seconds of process wall time. It used the
accepted runtime, durable ModelRepository, ModelingService and ordinary semantic
projections. It navigated Agentique → ModelingPlatform → ModelRepository, inspected
canonical/inherited ports, displayed real derived Explain, Graph, Requirements,
agent dependencies and parent history, created `alphaStudioObserver`, reviewed
Current/Candidate/Diff, validated and committed. No source editing was required.

The [separate restart](real-run06/restart.json) passed five assertions in 194.745
seconds: runtime authentication, exact durable manifest/head, ordinary owner focus,
the same canonical part `3670456b-6e5f-59ba-a083-9ac253de64a4`, and History. Thirteen
first-process and four restart screenshots have checked image hashes. Both runs
used executable SHA256 `c6a8013dbf7db7103153ca287758390f58e6f1db72e6a57405b2a87c6b5fc57b`,
built from `7270d6e5`; subsequent production change was the equivalent profiling
boolean cleanup, with a later equivalent test-helper cleanup. Neither changes
the demonstrated interaction or semantic behavior.

The general new-project journey remains incomplete: setup seeds/opens Agentique,
and a generic starter-system wizard has not been delivered. Bounded Rename is
implemented, but its accepted-runtime rename/source-only restore/CAS gate was
not run. Saved presentation views and separate presentation restart passed their
own explicitly fixture-based qualification.

## Performance

Final synthetic runs were serial, without local compilers or semantic consumers,
on RTX 3060 Ti/Vulkan, Windows, reported 165 Hz, vsync and High performance power.
Ordinary desktop activity remained possible. Each phase has 120 native update
intervals. These figures are not physical display latency or a causal comparison
with the original approximately 60 Hz baseline.

| Workload | Steady median / p95 ms | Pan p95 ms | Zoom p95 ms | Build / layout ms | Hit p95 µs | Scene GPU median ms |
|---|---:|---:|---:|---:|---:|---:|
| 1k nodes / 2k edges, PASS | 6.060 / 6.195 | 6.162 | 6.182 | 14.854 / 13.028 | 1.9 | 0.0287 |
| 10k / 20k, first **FAIL** | 6.052 / 6.320 | 13.273 | 13.130 | 148.208 / 128.657 | 9.1 | 0.1577 |
| 10k / 20k, unchanged repeat PASS | 6.054 / 6.232 | 12.492 | 11.371 | 146.602 / 126.674 | 6.8 | 0.1638 |

The first 10k run lost the benchmark's fixed pointer anchor by 1,042 world units;
the repeat error was 0.002. Cause is unproven. Both runs are retained in the
[measurement closeout](performance/final-measurements-closeout.md). A possible
ordinary-input/smooth-scroll interaction in the test driver is a hypothesis,
not a fix or proof that the application is correct.

Real model: 76,147 canonical elements, 785 local and 75,362 standard before the
edit; 76,153 canonical after it. Projections deliberately remain relevant:
System 7 nodes/6 edges, focused platform 20/20, Graph 6/6, Requirements 5/5;
the broad History projection has 159/425 and a 130/405 scene. Real scene builds
range from 0.056–0.264 ms for ordinary captured views to 36.564 ms for that History
diff. The first asserted Validated resumed view arrived after 175.418 seconds;
the platform focus step took 3.550 seconds including input/query/settling.
Real capture rolling frame p95 values were approximately 6.27–6.75 ms; overlapping
windows are not independent per-world benchmarks.

Candidate Prepare release → observed Working candidate: **135.029 s**. Whole
prepare/validate/commit UI steps: **138.028 / 29.197 / 8.823 s**, including queues,
follow-up views and observation. Native pan remained active; 22,281 input hooks
had an 11 ms maximum gap, and a real current Inspector completed while preparation
was pending. Focus/filter/lens projection changes still wait behind that work.
Cancellation discards eventual results; it does not interrupt the compiler.

The exact cold-reconstruction oracle passed, including 16 invalid shared-mount
rejections. Prepare was 128.619 s; command compile 117.859 s versus independent
cold compile 118.516 s, with 6.50 GB process peak working set. Both reparsed six
documents, rebuilt 1,101 local records and evaluated 717 producer/791 audit
subjects. **No true incremental semantic speedup is claimed.** Duplicate audit
queries were removed with exact answer/diagnostic parity; closure and refinement
remain dominant. Query-context reuse separately reduced paired projection and
Inspector medians from about 5.5 s to 2.9 s. Completed cache hits in run06 measured
0.183 ms median for Architecture and 0.030 ms for Inspector, excluding queues.

GPU timestamps bracket only the custom scene pass; text/chrome/upload/present
are excluded. Raw-input-to-UI and following-update timings are retained. Frame
submission, presentation and photon latency remain unmeasured. Large presentation
rebuilds use a background path, but cross-revision atomic scene construction can
still occupy the UI thread. [Full scoped measurements](performance/final-combined-extracted.md)
retain sizes, counts, phases and missing measurements.

Nine adversarial fixtures in both hierarchy and graph layouts retained unchanged
node top-left displacement median/p95 of 0/0 world units for the tested local
edits, with no sibling overlaps or containment failures. This [fixture metric](product/routing-layout-quality.txt)
does not establish screen-space continuity for every real revision change.

## Five formal visual review rounds

Each linked review has screenshots, ranked criticism, changes and replacement
captures. These five rounds used explicitly labeled fixtures; the later real
reviews and completed journey qualify the actual semantic workflow separately.

| Round | Largest criticism | Change made | Remaining weakness |
|---|---|---|---|
| [Systems engineer](reviews/round-01-systems-engineer.md) | Candidate appeared in the wrong overlapping subsystem. | Owner-relative containment/diff geometry and lifecycle labels. | Small relationship labels and missing requirement roles. |
| [Product designer](reviews/round-02-product-designer.md) | Anonymous agent result and unclear requirement links. | Readable neighborhood, inquiry/revision/authority and literal link labels. | Dense overview and long requirement labels. |
| [Graph expert](reviews/round-03-graph-expert.md) | Tiny cards, faint paths and convergent endpoints. | Neighborhood scope, incident-edge names and separated attachment lanes. | Crossing ambiguity and candidate camera continuity. |
| [Editor interaction](reviews/round-04-editor-interaction.md) | Wrong-context menus, weak palette keyboard use and reframe/selection loss. | Context-specific menus, keyboard palette and review selection/camera preservation. | Ordinary Back still loses Graph exploration state. |
| [AI workflow](reviews/round-05-ai-workflow.md) | Unclear decision provenance and authoritative-sounding mock weights. | Visible inquiry/authority, explicit provider mock, correct endpoints and local proposal framing. | Only a bounded dependency agent operation is demonstrated. |

Real iterations additionally corrected misplaced port labels, Explain clipping,
hidden graph relationship names and oversized History controls. The
[final independent review](reviews/real-run06-editor-interaction.md) verifies
those improvements and retains the remaining density criticism.

## Architecture

The kernel remains canonical; language/query contracts and in-process application
authority remain inward. Native responsibilities now have coherent modules for
history, engineering inspection, part editing, immutable read workers, scene
building, saved presentation, palette, relationship labels, timing and recovery.
Revision/runtime/request fences prevent stale worker replies from replacing newer
state; durable commits precede checkpoint/head changes. Large action/update and
automation modules still deserve ongoing responsibility review.

egui/eframe remains viable for this interaction model. The concrete limitation is
full GPU device recreation behind eframe's private integration: surface loss and
resize use bounded reconfiguration; unrecoverable device loss requires restart.
There is no basis here to reopen ADR 0030.

## Verification

Final local command results are recorded in `checks/final-*.json` with actual
commands, output hashes and exit codes. Initial final Clippy found a test-only
boolean simplification; it was corrected without changing the preserved oracle
reference bodies. The first final browser run timed out during a cold server
build competing for Cargo locks; its failure is retained beside a serial retry.
The completed command receipts are:

| Required command | Result / receipt |
|---|---|
| `cargo fmt --all -- --check` | PASS, exit 0 — [receipt](checks/final-root-format-after-lint.json) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, exit 0 after the retained lint correction — [receipt](checks/final-root-clippy-retry.json) |
| `cargo test --workspace` | PASS, exit 0, 863.859 s including build — [receipt](checks/final-root-tests.json) |
| `npm run check` | PASS, exit 0 — [receipt](checks/final-npm-check.json) |
| `npm run build` | PASS, exit 0 — [receipt](checks/final-npm-build.json) |
| `npm test` | PASS, exit 0 — [receipt](checks/final-npm-test.json) |
| `npm run test:e2e` | PASS, exit 0, eight tests on serial retry — [receipt](checks/final-npm-e2e-warm.json) |
| `npm run standards:check` | PASS, exit 0 — [receipt](checks/final-standards-after-lint.json) |
| `cargo run --locked --offline -p agq-metamodel-gen -- --check` | PASS, exit 0 — [receipt](checks/final-metamodel.json) |

Separate native workspace [format](checks/final-native-format.json),
[Clippy](checks/final-native-clippy.json) and [tests](checks/final-native-tests.json)
passed. [Rustdoc](checks/final-rustdoc.json) for the changed language/workspace/view/
platform/scene crates passed. The [final staged output-hash audit](checks/final-evidence-complete.json)
verified 312 command-output receipts without repairing bytes.
The staged audit initially encountered nested CI JSON with a different schema;
its Git path selection now matches the filesystem check's direct-child command
receipt population. The [retry](checks/final-evidence-staged-retry.json) verified
308 staged output hashes with no repairs. Nested CI entries have their own
authenticated acquisition and retention manifests.

The current native regression suite has 132 tests. Exact accepted-runtime audit,
query-reuse and independent cold-reconstruction gates passed separately; ignored
runtime tests are not represented as ordinary workspace-test coverage. The broader
platform test combining CAS, bounded Rename and forced source-only restoration
remains unrun. The CI independent-validator `Controller.sysml` RES001 disagreement
is historical and unchanged; it remains visible rather than skipped.

[CI run 36221839366](https://github.com/agentique-systems/agentique/actions/runs/36221839366)
at `4241d7e9` passed native compilation/lint/tests, workspace tests, standards,
metamodel, frontend, browser, runtime-setup, process recovery and official Pilot
checks. Overall CI was **failed**: the test-helper Clippy predicate was corrected
in `6b815325` and passed the local retry; independent-fixture RES001 remains
byte-identical to the earlier recorded result. The [actual CI logs and entry hashes](checks/ci-36221839366/retention.json)
are retained. The final documentation push may trigger another run; no later
CI result or release completion is inferred.

## Release risks and retained follow-up

| Area | Exact status |
|---|---|
| IME | Real composition smoke test unqualified. |
| Screen reader | Windows UIA names/actions, keyboard scenarios and Narrator coexistence checked; spoken usability not certified. |
| DPI/fonts | Long names and Unicode exercised; physical mixed-DPI/high-DPI transitions and broad font fallback remain unqualified. |
| Device loss | Surface resize/loss recovery implemented; complete device recreation is unavailable, restart path remains. |
| Cross-platform | Windows native execution tested; [Ubuntu native compile/lint/unit CI](checks/ci-4241d7e-native-status.json) passed at `4241d7e9`, covering the delivered native/platform changes. The later root test-helper boolean cleanup is qualified locally. Linux/macOS interactive GPU qualification remains open. |
| Runtime distribution | Authenticated draft asset and offline install work; public availability/new release pipeline execution remain open. |

Priority follow-up: resolve the intermittent 10k anchor gate with input/viewport
tracing; improve Requirements and dense Diff; reduce first-use semantic latency
and permit current-view projection while preparing; deliver generic new-project
creation; qualify real Rename/source-only restore; then qualify the composed
NativeStudio self-model extension. Draft composition/runner work (`9118585`,
`b154fcf`, `6372570`) and Back/Forward retention (`ced761e7`) remain isolated and
unintegrated because their final runtime/interaction qualification was not completed.
No 3D, simulation, execution IR, live general-purpose LLM or automatic merge was
added. The next milestone has concrete product defects and evidence to work from.
