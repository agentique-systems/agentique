# Native local-view and session qualification

Scope: actual egui RawInput, ordinary Local views handlers, presentation files,
and a separate executable process. This is explicitly **architecture fixture**
qualification. It does not authenticate a runtime, validate semantics, commit a
model, or replace real-model acceptance.

## Implemented checks

The `presentation` scenario opens the Local views menu, enters a name, saves the
current presentation, renames the same bookmark without changing its binding or
presentation, focuses ModelingPlatform, opens Graph World, pans, updates the same
bookmark, leaves Graph World, and opens the saved bookmark by its ordinary row.
Assertions read resulting application state and on-disk bookmarks. They compare
the saved revision, view definition, camera, layout, expansion, collapsed IDs and
panel state. The driver never calls application actions or persistence methods.
The name field now has an explicit accessible label.

The final step waits for ordinary eight-second autosave. Its narrow scenario
exception is protected before application construction by an absolute, explicit,
unused database plus fresh presentation sidecars, report and gallery. Existing
fixture/real scenarios still cannot autosave. The scenario records the saved
session and view-file digests and captures the actual native surface.

The `presentation-restart` scenario requires that successful first report and
the exact saved bytes at the same isolated paths. It uses normal startup with
neither `--fixture` nor `--no-restore`; it does not write session state. Startup
now restores recognized disposable fixture sessions at the exact retained
fixture revision. Unknown fixture names, mixed real-project bindings, missing
presentation payloads and different fixture revisions are refused. This fixes
the previous startup path which ignored the saved fixture entirely. Camera
viewport dimensions follow the new window; the saved center/zoom are retained.

## Reproduce after integrated native build

Run from the repository root in PowerShell. The fresh directory name is important:
the driver deliberately refuses overwriting evidence or operator state.

```powershell
$presentationRun = Join-Path (Get-Location) ('verification/generated/native-studio/presentation-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $presentationRun | Out-Null
$nativeExecutable = Join-Path (Get-Location) 'target/native-alpha/release/agq-studio-native.exe'
$presentationDatabase = Join-Path $presentationRun 'qualification.sqlite'
$presentationReport = Join-Path $presentationRun 'journey.json'

& $nativeExecutable --root (Get-Location) --database $presentationDatabase --fixture architecture --no-restore --scenario presentation --scenario-report $presentationReport --gallery (Join-Path $presentationRun 'first-gallery') --scenario-timeout-seconds 180
if ($LASTEXITCODE -ne 0) { throw 'Presentation journey failed; retain report and files' }

& $nativeExecutable --root (Get-Location) --database $presentationDatabase --scenario presentation-restart --restart-report $presentationReport --scenario-report (Join-Path $presentationRun 'restart.json') --gallery (Join-Path $presentationRun 'restart-gallery') --scenario-timeout-seconds 180
if ($LASTEXITCODE -ne 0) { throw 'Presentation restart failed; retain report and files' }
```

Root can use `verification/scripts/native_alpha_check.py` around each executable
invocation to retain process exit, console output and executable identity. Both
scenario reports must have `passed: true`; the first alone is not restoration
acceptance. Both screenshots are labelled fixture presentation evidence.

## Evidence at handoff

- `cargo fmt --manifest-path crates/studio-native/Cargo.toml -- --check`: exit 0,
  recorded in `checks/presentation-driver-format.json` and its output file.
- `git diff --check`: exit 0.
- Added focused negative tests for launch isolation and startup fixture identity.
- **Compilation, native tests and both native processes have not run yet.** The
  integration lead requested one integrated build to avoid changing dependency
  worktree paths in the shared native cache during runtime restoration.

Source review also found that global Escape handling consumes the key before
ordinary menus. The integration lead owns the keyboard fix. This scenario now
requires Escape to dismiss the first menu while retaining the existing nonempty
canvas selection; the second dismissal exercises the ordinary toolbar toggle.
The driver therefore depends on integrating that keyboard-precedence fix.

## First actual native run: form closed on its own input

The integrated first run (`checks/presentation-actual-journey.json`, executable
SHA-256 `b941acf10802e3189883032ba150f894bd476fa192d84ae162cb982ddcc0eb11`)
exited 2 in 4.344 seconds. It opened Local views successfully, then failed the
name-entry assertion. The report retained the click at `[834.0, 283.5]` on frames
226/227, Ctrl+A at frame 229 and text at frame 230.

The cause is the ordinary form menu: egui 0.33.3's `ui.menu_button` inherits
`PopupCloseBehavior::CloseOnClick`, which closes on an internal click as well as
an external one. Clicking the text field closed the form before text arrived.
The form now uses explicit `MenuButton` configuration with
`CloseOnClickOutside`. Open still closes explicitly, and Escape remains handled
by egui. Save, rename, selection and update can keep the form open.

The driver now checks exact retained editor text in addition to current geometry
and keyboard focus. Assertion failures retain a native `*-failed.png` with state,
actual editor text and image identity before exiting nonzero. No persisted-model
or semantic assertion was weakened. The repair is formatted; integrated compile
and native rerun are delegated to the lead to preserve the shared build cache.

## Completed native process and restart qualification

The second attempt passed all presentation assertions and captured its surface,
then panicked on a queued input hook after requesting window close. Its exit 101
is retained in `checks/presentation-menu-fixed-journey.*`; its earlier report's
`passed` value alone is not acceptance. The runner now treats recorded completion
as terminal while native close events drain. Strict Clippy also identified a
manual modulo predicate, which was corrected.

The third run and separate restart both pass, with actual process exits 0 in
17.203 and 1.484 seconds. Executable SHA-256:
`8cf168f9806a9e19736fc3d98b4ce8816e63f28c7cd700e26be5e0dd32d687c0`.
See `presentation-run03/journey.json`, `presentation-run03/restart.json` and their
native galleries, plus `checks/presentation-terminal-fixed-journey.*` and
`checks/presentation-actual-restart.*`. The restart restores the exact saved
bookmark/session bytes, Graph view, camera, layout and presentation settings.
The captured surface remains explicitly labelled VISUAL FIXTURE. This closes
the presentation qualification, not real semantic edit/commit acceptance.
