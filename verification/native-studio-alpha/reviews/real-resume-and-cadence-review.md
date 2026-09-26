# Explicit failed-baseline resume and independent cadence review

`--scenario real --resume-report <failed-report.json>` permits the complete real
journey to reuse an already seeded, untouched validated baseline. It does not
skip authentication, opening, source checks, assertions, screenshots, candidate
construction, validation, commit, or the later separate `real-restart` process.

The predecessor must be a failed real report with a successful first baseline
assertion, an exact Validated manifest, coherent project/branch/revision fields,
and no created element/owner, candidate, commit, pending mutation, preparation
observation, or mutation-stage assertion. Accepted predecessor assertion names
must be an exact prefix of this journey before Create Part. In-flight pending
work is rejected. The requested database must be the exact existing absolute
path in that report. Current source filenames, population and content digests
must equal its baseline documents; duplicate manifest paths are rejected.

Preflight deliberately does not create an alternate SQLite or semantic decoder.
After the ordinary authenticated platform opens the existing repository, the
first baseline assertion independently compares the complete manifest, project,
branch and branch head. It repeats the source proof in this process. No later
step runs unless those comparisons pass. The report is provenance and a
comparison target, not a source of runtime or repository authority.

New report/gallery destinations must be unused. The prior report is parsed and
hashed from the same byte buffer; the fresh report retains its digest and marks
`resumed_baseline: true`. Assertions and gallery start empty, and execution starts
at the original first step. The successful-journey requirements for
`--scenario real-restart` are preserved. Resume is rejected for that scenario,
fixtures, presentation scenarios, or a simultaneous restart report.

Example, using the exact database path recorded by the failed run:

```powershell
.\target\native-alpha\release\agq-studio-native.exe --root . --scenario real --no-restore --resume-report .\verification\native-studio-alpha\real-run01\journey.json --database C:\Users\phili\github\agentique-systems\agentique\verification\generated\native-studio-alpha\real-run01\agentique.sqlite --scenario-report .\verification\native-studio-alpha\real-run02\journey.json --gallery .\verification\native-studio-alpha\real-run02\gallery
```

The inspected failed `real-run01` report records an actual Validated/Complete
baseline, 38 projected architecture nodes and 37 edges, then fails waiting for
the first screenshot. It contains no candidate/owner/commit or mutation
observations. Reuse is appropriate subject to the new process proving the same
repository manifest and current sources. This review does not change its failed
outcome or qualify its externally stimulated frame cadence as normal operation.

Three focused negative-test groups cover explicit resume versus ordinary fresh
launch/restart, exact byte-digest provenance, candidate/mutation/working or
nonfailed predecessors, mismatched identity/database, and changed source
population/content. Their tiny synthetic fixture tests only launch evidence
validation; the real semantic gate remains the ordinary live baseline assertion.
No native build or runtime consumer was started for this isolated patch.

## Pinned frame-pump source review

Independent inspection of installed pinned sources confirms:

- `eframe-0.33.3/src/native/epi_integration.rs:271` invokes
  `app.raw_input_hook` immediately before `egui_ctx.run` at line 273. The
  application's `update` runs inside that context pass.
- `egui-0.33.3/src/context.rs:100-115` starts each pass by resetting a viewport's
  delayed repaint to `Duration::MAX` when there is no outstanding repaint.
- `context.rs:139-147` gives an outstanding extra pass only to an immediate
  zero-delay request. A nonzero delay requested before the pass does not receive
  that protection. The pass begins this logic at line 430.

Scheduling opt-in scenario cadence from `App::update` is therefore an appropriate
bounded correction. It places the request after the reset and leaves ordinary
application cadence untouched. Keep the failed run and diagnostic external
redraw evidence: they do not establish normal handled-input responsiveness or
screenshot delivery. Qualify the corrected process without external repaint
injection. Maintain terminal-state guards because a queued final native pass may
arrive after a scenario has requested Close. Avoid immediate busy repainting as
a permanent workaround; it would distort frame/performance evidence.
