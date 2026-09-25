# Final performance extraction and quiet fixture commands

These commands are prepared from actual `Args`, `stress_automation.rs`,
`timing.rs`, `updates.rs` and `real_automation.rs`. This document does not claim
new native runs. Run fixtures serially while semantic reconstruction, native
journeys, compilers and other benchmarks are idle. Record the actual competing
processes, power/monitor configuration and adapter; quiet conditions cannot be
inferred from an attractive result.

## Exact synthetic camera scenarios

From the integration repository root, after its final native build, choose a
fresh output directory and record the launch receipt (command, UTC boundaries,
exit code, commit, dirty-source hashes if any, executable SHA256, and concurrent
process context). Existing evidence wrappers may provide that receipt. The
following are the exact native argument lists, not a request for a new build:

```powershell
$nativeExe = (Resolve-Path 'target/native-alpha/release/agq-studio-native.exe').Path
$perfDir = Join-Path (Get-Location) 'verification/native-studio-alpha/performance/quiet-final'
New-Item -ItemType Directory -Path $perfDir -ErrorAction Stop
Get-FileHash -Algorithm SHA256 -LiteralPath $nativeExe
git rev-parse HEAD
git status --short

& $nativeExe --root . --fixture stress1000 --no-restore --scenario stress --gpu-timestamps --scenario-report (Join-Path $perfDir '1k.json')
$oneKExit = $LASTEXITCODE
if ($oneKExit -ne 0) { throw "1k scenario failed: $oneKExit" }

& $nativeExe --root . --fixture stress10000 --no-restore --scenario stress --gpu-timestamps --scenario-report (Join-Path $perfDir '10k.json')
$tenKExit = $LASTEXITCODE
if ($tenKExit -ne 0) { throw "10k scenario failed: $tenKExit" }
```

The commands intentionally use **no `--frames` early stop**. Each stress scenario
closes after its own assertions and 60 warmup + 120 steady + 120 pan + 120 zoom
intervals, followed by 30 settling frames. It checks actual injected pointer/wheel
input, camera displacement, zoom anchoring and culling. `--gpu-timestamps` only
requests supported features; unavailable timing stays unavailable. Vsync remains
enabled, without asserting a particular monitor refresh rate. These fixtures do
not load or validate a real semantic model.

The native scenario itself writes the measurements, not a complete launch
receipt. Save independent receipts as `1k-process.json` / `10k-process.json` with
at least an integer `exit_code`; retain the recorded launch identity and actual
environment to make comparisons defensible. Do not retroactively associate a
rebuilt executable hash with an earlier run.

## Completed real-run extraction

The Python utility reads only existing files. It rejects a running/unknown real
journey or a process receipt without a terminal integer exit code. A failed
completed journey is legitimate evidence and remains labeled failed. Use fresh
output names; existing evidence is never overwritten.

```powershell
python verification/native-studio-alpha/performance/summarize_native_performance.py --journey verification/native-studio-alpha/real-run05/journey.json --process verification/native-studio-alpha/real-run05/process.json --log verification/native-studio-alpha/real-run05/process.log --gallery verification/native-studio-alpha/real-run05/gallery --out verification/native-studio-alpha/performance/run05-extracted.json --markdown verification/native-studio-alpha/performance/run05-extracted.md
```

For the final completed real run, replace those input/output paths. Add
`--stress REPORT PROCESS` once per completed synthetic run. Add `--evidence PATH`
for existing source/build/environment receipts, the hot/cold semantic oracle
receipt, and output from `reviews/audit_view_profile.py`. All such JSON records
and artifact digests are retained verbatim; the extractor does not rerun or
reinterpret their gates. For example:

```text
--stress verification/native-studio-alpha/performance/quiet-final/1k.json verification/native-studio-alpha/performance/quiet-final/1k-process.json
--stress verification/native-studio-alpha/performance/quiet-final/10k.json verification/native-studio-alpha/performance/quiet-final/10k-process.json
--evidence verification/native-studio-alpha/checks/real-run05-view-profile-audit.json
--evidence verification/native-studio-alpha/performance/create-part-shared-mount.json
```

## Measurement contracts

- Every input has its observed byte count and SHA256. The original launch receipt
  provides the recorded commit/executable identity; current source/executable
  content is never substituted. This utility cannot independently prove what a
  prior binary was built from. Record a build/source receipt separately.
- Real gallery measurements retain exact revision, view definition, projection
  and scene sizes, plus per-capture rolling windows. Windows may overlap and may
  include earlier worlds; they are not dedicated steady scene benchmarks.
- Frame/UI/hit/input/GPU samples keep their recorded counts. P95 is suppressed
  below 20 observations. Query/cache calls use count/min/median/max, not p95.
  Query phases are retained exactly; inclusive parent and child phases must not
  be added together. A cache miss includes query work and must not be added to it.
- Native preparation/validation/commit assertion durations include action entry,
  queue/work and UI observation. The additional preparation clock begins before
  Prepare-button release and ends when a candidate and no pending mutation are
  observed. Neither boundary isolates compilation, audit or validation engine
  time. Use the separate semantic oracle for those measurements.
- The first asserted Validated view is timed from runner start, not physical
  process launch, first GPU presentation or first visible photon. Focus step
  durations also include native input and settling.
- GPU timestamps cover the scene pass only. Submission, presentation and physical
  input-to-photon remain unmeasured. Missing final metrics in a failed journey
  remain null even when earlier gallery windows exist.

Offline regression command:

```text
python verification/native-studio-alpha/performance/test_summarize_native_performance.py
```
