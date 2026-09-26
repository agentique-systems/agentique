# Real native acceptance runner

**First actual run reached Validated; the complete journey remains pending.**
Runtime rematerialization, ordinary authentication, bundle verification and
installation passed. [Run 01](real-run01/diagnosis.md) created the real durable
repository and passed its baseline semantic assertion, then failed screenshot
delivery after an identified scenario repaint scheduling defect. No successful
visual journey, authored candidate or candidate commit/restart is claimed by
that run.

The `real` and `real-restart` scenarios use the ordinary native application,
authenticated runtime bootstrap, ModelRepository, ModelingService, semantic
projections and serialized worker replies. Every interaction is injected as
egui RawInput. The runner cannot construct a semantic scene, inject a worker
result, alter model state or directly call platform mutation methods.

The first run performs the following:

1. Open the authenticated Agentique project and require a Validated revision,
   Complete producer closure and exact current self-model source hashes.
2. Select and focus ModelingPlatform, return through architecture, inspect
   ModelRepository and its modeled effective interfaces.
3. Request agent dependencies, open Graph World, deliberately include adjacent
   standard semantics, and select a real derived relationship by hit-tested
   native geometry. Explain must return its canonical subject, producer rule,
   nonempty evidence and a visible causal diagram.
4. Open Requirements World and History; compare the actual parent under an
   identical Graph overview lens and require the actual added AgentRuntime,
   then restore the current revision.
5. Prepare `CreatePartUsage(alphaStudioObserver)` through the ordinary dialog.
   Pan the retained current world using native input while reconstruction runs.
   Preparation lasting at least one second must demonstrate camera movement
   while mutation is pending. Every measured input-hook gap must stay within
   a coarse 250 ms non-freeze bound, including completion between hooks. This
   does not establish 60 Hz rendering or physical presentation latency.
6. Review Current, Candidate and Diff, select the canonical added part, validate
   through the normal operator command, commit through that same command path,
   and observe the exact validated revision at the durable branch head.

All steps wait for ordinary worker completion and assert revision coherence.
Working candidates remain explicitly Working until platform validation returns.
Screenshots carry sidecar JSON with real-data designation, project/revision,
candidate phase, selected identity, closure metadata, image digest and actual
native metrics. Screenshot requests are correlated by user data and the captured
context must remain unchanged until delivery. The eight gallery subjects are
included, plus candidate difference and committed History.

First-process success is `journey_passed_restart_pending`, not complete native
acceptance. A **separate process** runs `real-restart`, reauthenticates the runtime
and stored revision, compares the complete committed manifest/validation receipt,
checks the exact canonical added ElementId, inspects it and opens History.
Its report records the first report's content digest. Only that process can set
`restart_verified: true`.

## Running after authenticated runtime installation

Use an absolute path to a new isolated database. The launch check runs before
the model worker starts and refuses an existing database or sidecars, fixture
mode, session restoration, automatic frame-limit closure, and old evidence
paths. Choose a new report and gallery path on every attempt. Failed runs do not
delete repositories or overwrite previous evidence.

```powershell
$acceptanceDb = 'C:\AgentiqueAcceptance\run01\agentique.sqlite'
target/release/agq-studio-native.exe --root . --runtime-dir 'C:\AgentiqueRuntime' --database $acceptanceDb --no-restore --scenario real --scenario-report verification/native-studio-alpha/real-run01/journey.json --gallery verification/native-studio-alpha/real-run01/gallery
target/release/agq-studio-native.exe --root . --runtime-dir 'C:\AgentiqueRuntime' --database $acceptanceDb --no-restore --scenario real-restart --restart-report verification/native-studio-alpha/real-run01/journey.json --scenario-report verification/native-studio-alpha/real-run01/restart.json --gallery verification/native-studio-alpha/real-run01/restart-gallery
```

Run serially and record actual commands, output and exit codes with
`verification/scripts/native_alpha_check.py`. The default real-scenario wall
deadline is four hours including bootstrap and semantic work; adjust with
`--scenario-timeout-seconds` only when a measured workload requires it. Failure
exits 2 and retains the running/failed report. The report is a disposable test
artifact, not repository or publication authority.

Current instrumentation prerequisites are in `real_targets.rs`: setup Project
buttons, explorer search/rows, candidate modes and adjacent-standard checkbox.
Existing fixture automation supplies only shared widget rectangles for the
palette, candidate dialog, History rows and Explain window; its fixture runner
is never invoked by real acceptance.

Exact source matching intentionally catches a bootstrap importer that omits a
new self-model document. The existing bootstrap imports five architecture files
and AgentFabric; a future new `.sysml` must be included deliberately.

## Limits

This is one native input journey, not independent human usability judgment,
screen-reader certification or a performance benchmark. Its timing observations
describe its workload and cannot establish GPU execution or input-to-photon
latency. Gallery/state and first/restart reports must be reviewed before claiming
real first light. Any missing or occluded derived relationship fails the journey;
the runner does not replace it with a hand-selected semantic response.
