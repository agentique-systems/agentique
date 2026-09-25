# Native Studio release and accessibility qualification

## Latest qualification snapshot

Reconciled against committed records and current source on 2026-09-25. This
update ran no tests or runtime consumers. **Partial Windows qualification:** the
bounded fixture keyboard/UIA journeys pass, and a real accepted-model semantic
baseline was reached. A complete real semantic operator journey is not established
by these records. Physical device failure, mixed DPI, real IME composition,
screen-reader speech/usability and macOS/Linux remain unqualified.

| Area | Latest status | Evidence and practical limit |
|---|---|---|
| Native Windows / real model | Actual semantic baseline reached; complete real journey pending | [Real run diagnosis](real-run01/diagnosis.md) and [journey](real-run01/journey.json): accepted KerML v9 / Systems v3, six real sources, Validated head and Complete closure. Process exited 2 after scenario/screenshot scheduling failure; no real screenshot, candidate or candidate commit from that run. |
| AccessKit names, focus and port actions | Bounded fixture smoke passed | [Integrated UIA](accessibility/integrated-uia-native-window.json): seven checks, named search/palette, owner-qualified port Buttons, focus and Invoke. Does not qualify every edge, zoom level or engineering action. |
| Keyboard operation | Fixture visual journey passed; real semantic journey unqualified | [Integrated keyboard](accessibility/integrated-keyboard.json): 21 assertions without pointer events, including navigation, Explain and visual candidate review/cancel. Fixture validation/commit stays prohibited; this is not a keyboard-only real commit. |
| Narrator | Coexistence/action smoke passed; speech unqualified | [Narrator coexistence](accessibility/narrator-coexistence.json): eight assertions with Narrator alive. Spoken output was not listened to or assessed. |
| Resize / minimize | Bounded native smoke passed | [Integrated UIA](accessibility/integrated-uia-native-window.json) exercised minimize/restore and four sizes. This is not resize stress, mixed-DPI movement or device recovery. |
| Surface loss / timeout / OOM | Recovery logic unit-tested; physical fault handling unqualified | [Four state/callback tests](checks/surface-recovery-native-tests.txt) passed. [Current implementation](../../crates/studio-native/src/surface_recovery.rs) bounds retries, reconfigures Lost/Outdated surfaces and exposes stop/restart guidance. No physical GPU/surface/OOM fault was induced. |
| Device loss | Explicit restart-required state implemented; physical recovery unqualified | Current callback records device loss and preserves an OS-title message; it does not recreate a device or renderer in process. Callback unit tests do not qualify physical device removal. |
| Reduced motion | Fixture toggle plus current camera/toolkit control | [Keyboard evidence](accessibility/integrated-keyboard.json), [app initialization](../../crates/studio-native/src/app.rs) and [actions](../../crates/studio-native/src/actions.rs) cover camera and toolkit animation settings. Broader accessibility adequacy and OS preference following remain unqualified. |
| High contrast | Fixture toggle passed; adequacy unqualified | [Round 03](round-03-after/journey.json) retains the toggle assertion. No complete contrast, color-vision or OS high-contrast qualification. |
| Unicode / DPI | Long-name/Unicode fixture exercised; physical DPI unqualified | [Typography journey](typography-width-after/journey.json) passed. No guarantee for every script/font face; no 125/150/200% or mixed-monitor transition qualification. |
| IME | Composition guard implemented; real composition unqualified | Current [keyboard handler](../../crates/studio-native/src/actions.rs) yields while composing, and app input tracks preedit/commit. No real composition, candidate-window placement or cancellation test occurred. |
| Presentation restoration | Bounded unit checks passed; crash/power-loss qualification absent | [Native unit output](checks/presentation-state-native-tests.txt) includes session roundtrip, invalid-state rejection and interrupted/oversized replacement checks. No physical crash/power-loss injection is claimed. |
| macOS / Linux | Unqualified | No native build or runtime result for either platform is retained in this audit. |

Current source also has filtered Explorer arrow navigation, focus-to-selection
for canvas labels, named disclosure controls and Ctrl/Cmd+Z mapped to presentation
Back. That shortcut does not undo durable semantic history. These supersede the
initial source gaps below; implementation alone does not extend measured keyboard
or assistive-technology qualification beyond the linked scenarios.

## Historical initial audit (superseded where noted)

The following initial observations, matrix, risks and proposed next checks are
preserved as historical evidence for their recorded binary. They do not describe
the latest implementation where the snapshot above or dated addenda differ.

Audit date: 2026-09-25. This is a bounded Windows source inspection and actual
Windows UI Automation smoke test, not a release certification. No native source,
model, installed input method, accessibility setting, or driver was changed by
the audit. No build was launched.

**Result: partial Windows qualification.** Semantic names, several focus targets,
and two UI Automation invocations worked in the actual native application. Full
screen-reader operation, a keyboard-only engineering journey, mixed-DPI movement,
IME composition, GPU device loss, and non-Windows operation remain unqualified.

**Later qualification, 16:57 UTC:** the rebuilt port-accessibility binary passed
seven actual UIA checks and the separate keyboard-only visual journey. This
supersedes the earlier search-name, port-action, keyboard-journey and bounded
resize/minimize observations below. It does not qualify spoken screen-reader
output, real IME composition, mixed DPI, device loss or semantic commits.
See the dated addendum at the end of this record.

## Artifact and environment

- Main checkout HEAD when recorded: `f93a2a03e72ba5c116030297763256321e233e45`.
  Integration work was in progress; this does not assert the executable equals
  every source change present in the working directory.
- Executable actually inspected:
  `target/native-alpha/release/agq-studio-native.exe`, 37,150,720 bytes.
- Executable SHA-256:
  `cb84786759e68dd9ee94007db2863a018f6c8cfbdf1249540652ac48483ac850`.
- Windows 10 Home, version `10.0.19045`, build `19045`, 64-bit.
- Registry `AppliedDPI`: `96`. WMI reported two active monitors, `SAM0526` and
  `AUS270E`. These observations do **not** establish their effective per-monitor
  scaling or qualify moving the window between different scale factors.
- Native dependencies inspected locally: `eframe 0.33.3`, `egui-wgpu 0.33.3`,
  `egui-winit 0.33.3`, `winit 0.30.13`, `wgpu 27.0.1` from the Cargo registry.
- Narrator exists at `C:/Windows/System32/Narrator.exe`.
  Windows SDK Inspect exists at
  `C:/Program Files (x86)/Windows Kits/10/bin/10.0.22621.0/x64/inspect.exe`.
  NVDA and JAWS were absent from the checked installation paths and uninstall
  entries; a portable installation elsewhere was not exhaustively searched.
- Configured input languages: `en-SE`, `en-GB`; the latter exposes input method
  `0809:0000041D`. No CJK composition input method was configured in this list.
- Available font files: Segoe UI (955,804 bytes), Segoe UI Symbol (2,454,728 bytes),
  Microsoft YaHei collection (19,704,352 bytes).

## Historical initial qualification matrix

| Area | Status | Evidence and limit |
|---|---|---|
| Windows native launch | Qualified for the inspected fixture binary | Actual native window and AccessKit/UIA tree inspected; no runtime-backed semantic claim. |
| AccessKit semantic names | Partially qualified | 98 UIA descendants, 90 named; semantic node names, exact kind and origin present at the tested zoom. |
| UIA focus and basic actions | Partially qualified | Focus reached Commands, an outliner component, and a canvas semantic label. Outliner and Commands Invoke patterns worked. |
| Narrator spoken output | Unqualified | Narrator is installed but was not launched. No speech recording or human listening result was obtained. Tool availability is not the blocker. |
| Complete keyboard-only journey | Unqualified | Existing scenario mixes pointer and keyboard input; audit did not complete an all-keyboard candidate/commit journey. |
| Reduced motion | Camera control qualified by existing fixture scenario; broader behavior unqualified | Preference toggles through the palette; camera target interpolation becomes immediate. Egui window/hover animations are not controlled by this preference. |
| High contrast | Toggle qualified by existing fixture scenario; accessibility adequacy unqualified | Token changes and saved preference exist. No full text/control/edge contrast, OS high-contrast, or color-vision qualification was performed. |
| Resize/minimize handling | Source-inspected, runtime unqualified | Eframe handles nonzero resize and ignores zero-sized Windows minimize notifications; no resize/minimize stress was run here. |
| Lost/outdated surface | Source-inspected, fault injection unqualified | App callback requests surface reconfiguration. This is not device recreation. |
| GPU device loss / OOM | Unqualified; known recovery gap | No Studio device-loss recovery path; surface OOM currently skips frames and reports to stderr. |
| DPI | Single-environment native display only | Logical geometry and current `pixels_per_point` are used. No 125/150/200% or mixed-monitor transition was exercised. |
| Unicode / fallback | Source and installed-font availability inspected | Unicode fixtures and fallback loading exist; font availability is not proof that every glyph, collection face, or script renders correctly. |
| IME | Integration path inspected; composition unqualified | Winit preedit/commit and cursor-area forwarding exist; no real IME composition was performed. |
| macOS / Linux | Unqualified | Backend/features exist in dependencies; no build or runtime check on either platform in this audit. |
| Presentation persistence | Source-inspected, parent tests required | Unique temporary, flush, size/finite validation, atomic replacement exist in `session.rs`; no crash/power-loss injection was performed. |

## Actual Windows accessibility smoke

The application was launched with `--fixture architecture --no-restore` and a
bounded `--frames` count. `--frames` prevents Studio session writes, so the smoke
did not replace the operator's saved presentation. The native window was visible
to UIA and reported as on-screen despite the launcher using `-WindowStyle Hidden`.

The tree exposed these actual examples:

| Name | UIA type | Keyboard focus | Action pattern |
|---|---|---|---|
| `Commands  Ctrl K` | Button | SetFocus succeeded | Invoke |
| `ModelingService` in the outliner | Button | SetFocus succeeded | Invoke |
| `ModelRepository, PartUsage, Authored` on canvas | Text | SetFocus succeeded | **None** |
| `ModelingPlatform, PartDefinition, Authored` | Text | Reported focusable | Not invoked |
| `ProjectWorkspace, PartUsage, Authored` | Text | Reported focusable | Not invoked |

Invoking the `ModelingService` outliner button resulted in both its outliner
Button and a `ModelingService` Text heading in the Inspector. Invoking Commands
focused an Edit control supporting Value and Text patterns. Its UIA Name was
empty; HelpText was `Find a command or focus an element…`.

The initial tree also contained an unnamed search Edit with HelpText
`Find an element…`, and five unnamed focusable Custom controls. Disclosure
buttons were named only `▾`, without the associated subsystem name. These are
concrete naming/focus risks, not a general claim that AccessKit is missing.

The canvas label's lack of an action pattern is a product limitation: it exposes
readable content but does not provide the same selectable/invokable contract as
the outliner button. Source inspection also found that keyboard F/Enter uses the
existing semantic selection rather than the currently Tab-focused canvas label.
The integration lead was notified during the audit; the inspected binary is not
qualified for a subsequent fix until that binary is exercised again.

Process accountability:

- PID `24428`: first successful tree/focus probe, bounded at 7,200 frames. It was
  absent at the later action probe; no process exit receipt was captured, so its
  exact exit code is unknown. No continuing process with this PID remained.
- PID `30476`: an early exact-name probe raced accessibility initialization and
  found no target. The script failed; `finally` closed the audit-owned process,
  which returned exit code 0. This attempt does not count as a passed action test.
- PID `26240`: settled-tree and action probe passed. Closed through
  `CloseMainWindow`, exit code 0, then confirmed absent.
- Final audit query found all three PIDs absent and Narrator absent. No general
  process-name termination was used; no other Studio instance was closed.

## Historical initial risks

1. **Device loss and exhaustion have no operator recovery experience.**
   [main.rs](../../crates/studio-native/src/main.rs) maps `Lost`/`Outdated` to
   `RecreateSurface`. In the pinned `egui-wgpu/src/winit.rs`, that action calls
   `configure_surface` using the existing render state and skips the current
   frame. [gpu.rs](../../crates/studio-native/src/gpu.rs) installs device-bound
   scene resources once. No Studio `set_device_lost_callback`, device rebuild,
   or resource restoration path was found. `OutOfMemory` falls into SkipFrame;
   this can leave the operator without rendered feedback. Wgpu's default
   uncaptured-error handler is fatal. Do not advertise graceful device-loss
   recovery from the surface callback alone.
2. **Canvas accessibility is not a complete engineering interaction surface.**
   [viewport.rs](../../crates/studio-native/src/viewport.rs) creates names only
   for visible nodes when LOD is at least Summary. Ports and edges have no
   equivalent canvas Accessibility node contract in the inspected binary.
   Low-LOD overview does not expose those node proxies. Outliner access helps,
   but a user cannot infer equivalence for all spatial relationships.
3. **Focus and text-input ergonomics need a dedicated journey.**
   The app implements Ctrl/Cmd+K, F, Escape, owner navigation, arrows and Enter.
   Arrows navigate `outliner_order`, not the filtered visible search results.
   No Ctrl/Cmd+Z presentation/candidate behavior is implemented. This avoids
   implying mutable durable history but leaves the requested ergonomic action
   unavailable. Focusable canvas labels and unnamed controls require retesting
   after integration changes.
4. **IME Escape handling is a specific risk, not a reproduced failure.**
   [actions.rs](../../crates/studio-native/src/actions.rs) consumes Escape before
   `wants_keyboard_input`; it can close a dialog or clear selection. A real
   composition-cancel test must establish whether Escape is first handled by
   IME in this integration. Pasting Unicode or sending a Text event does not
   qualify composition, candidate-window placement, or IME cancellation.
5. **Reduced motion and high contrast cover only part of the shell.**
   Reduced motion applies to camera targets; ordinary panning is immediate.
   Fit/focus paths using `fit_pending` also set the camera immediately.
   [theme.rs](../../crates/studio-native/src/theme.rs) changes border/muted tokens
   for high contrast, but does not follow OS accessibility settings or disable
   toolkit animation. Scene labels can be as small as 8–11 logical pixels.
   Large text, keyboard focus visibility, disabled controls and faded unrelated
   elements need deliberate qualification, including light mode.
6. **Mixed DPI, fallback fonts and platform differences are untested behaviors.**
   Egui-winit updates native scale on `ScaleFactorChanged`; Studio supplies
   current `pixels_per_point` to its shader. Font fallback files are local,
   optional, and not redistributed. Windows font presence and two attached
   monitors do not establish cross-platform shaping or mixed-DPI continuity.

## Source and prior scenario evidence

The relevant first-party files were read directly in the current main checkout:
`main.rs`, `gpu.rs`, `viewport.rs`, `scene.wgsl`, `app.rs`, `actions.rs`,
`theme.rs`, `panels.rs`, `session.rs`, `automation.rs`, `Cargo.toml`.

Source hashes recorded at the audit boundary:

| File | SHA-256 |
|---|---|
| `main.rs` | `c2ad9c5150442566267072efba8aa59fb57817fd81947c5e4712976140e04d8c` |
| `viewport.rs` | `dd6b500e355cb34d600f8345b1fee116d7aa69da39e083fc7fc99fb90ca59106` |
| `actions.rs` | `3987d4d8995a7b4c2163a48aa7b6223b593746fcb769ca2d729e7bfbddbeb781` |
| `theme.rs` | `862aef860d10ff22ca2e7a2465ce768ce9955fd4070d484b86dcb7460bbcb7e8` |

Pinned dependency source inspected under
`C:/Users/phili/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`:

- `egui-wgpu-0.33.3/src/winit.rs`: surface error handling and resize implementation.
- `egui-wgpu-0.33.3/src/lib.rs`: `WgpuConfiguration`, default present mode and actions.
- `eframe-0.33.3/src/native/wgpu_integration.rs`: Windows zero-size minimize guard.
- `egui-winit-0.33.3/src/lib.rs`: IME preedit/commit forwarding, IME cursor area,
  scale-factor forwarding and AccessKit action events.
- `winit-0.30.13/src/platform_impl/windows/event_loop.rs`: DPI awareness/events.
- `wgpu-27.0.1/src/api/device.rs` and `src/backend/wgpu_core.rs`: device callbacks
  and the default uncaptured-error handler.

Existing [round-03-after/journey.json](round-03-after/journey.json) reports 41
passing assertions. The reduced-motion toggle, Escape clearing/closing and
high-contrast toggle assertions passed there. Its action list contains clicks and
double-clicks; those passes therefore do not establish a keyboard-only journey.
Earlier `round-01-mid/journey.json` records a failed Create dialog step; later
rounds passed. This audit neither reran nor relabeled those artifacts.

## Commands and results

Read-only source searches used `rg -n` for
`AccessKit|widget_info|IME|pixels_per_point|ScaleFactorChanged|device_lost|on_surface_error|reduced_motion`
in the paths above, followed by bounded `Get-Content -Encoding utf8` reads.
Commands exited 0 and produced the implementations summarized above. An initial
broad search over journey JSON was output-truncated; the structured Python
extraction below was used for the actual scenario conclusion.

Environment commands, all read-only:

```powershell
Get-CimInstance Win32_OperatingSystem |
  Select-Object Caption,Version,BuildNumber,OSArchitecture
Get-ItemProperty -LiteralPath 'HKCU:/Control Panel/Desktop/WindowMetrics' -Name AppliedDPI
Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBasicDisplayParams |
  Select-Object InstanceName,Active,MaxHorizontalImageSize,MaxVerticalImageSize
Get-WinUserLanguageList | Format-List *
Get-Item -LiteralPath 'C:/Windows/Fonts/segoeui.ttf',
  'C:/Windows/Fonts/seguisym.ttf','C:/Windows/Fonts/msyh.ttc' |
  Select-Object Name,Length
Get-ChildItem 'C:/Program Files (x86)/Windows Kits/10/bin' -Filter inspect.exe -Recurse
```

Exit 0; significant outputs are in the environment section. The first DPI probe
incorrectly used shell-style backslash escaping for the registry path and emitted
PowerShell parameter-binding errors despite wrapper exit 0. It was corrected to
the quoted `-LiteralPath` form above; the failed probe was not used as evidence.

The successful live probe used these operations, with the settled tree queried
before exact-name matching:

```powershell
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$auditBinary = 'C:/Users/phili/github/agentique-systems/agentique/target/native-alpha/release/agq-studio-native.exe'
$auditProcess = Start-Process -FilePath $auditBinary -ArgumentList @(
  '--fixture','architecture','--no-restore',
  '--root','C:/Users/phili/github/agentique-systems/agentique',
  '--frames','36000'
) -WindowStyle Hidden -PassThru
Start-Sleep -Milliseconds 1200
$auditProcess.Refresh()
$auditWindow = [System.Windows.Automation.AutomationElement]::FromHandle($auditProcess.MainWindowHandle)
$auditNodes = $auditWindow.FindAll(
  [System.Windows.Automation.TreeScope]::Descendants,
  [System.Windows.Automation.Condition]::TrueCondition)
Start-Sleep -Milliseconds 500
$auditNodes = $auditWindow.FindAll(
  [System.Windows.Automation.TreeScope]::Descendants,
  [System.Windows.Automation.Condition]::TrueCondition)
$auditTarget = $auditNodes | Where-Object {
  $_.Current.Name -eq 'ModelingService' -and
  $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button
} | Select-Object -First 1
([System.Windows.Automation.InvokePattern]$auditTarget.GetCurrentPattern(
  [System.Windows.Automation.InvokePattern]::Pattern)).Invoke()
```

The actual command then queried Inspector Text, invoked Commands, read
`AutomationElement.FocusedElement`, hashed the executable, and closed only its
own process in a `finally` block. Exit 0. Outputs: 98 settled nodes; Inspector
`ModelingService` Text; focused Edit with Value/Text patterns and placeholder
HelpText; PID 26240 exit 0 and absent after close.

The separate focus probe called `SetFocus()` for Commands, ModelingService and
the named canvas label, waited 250 ms between observations, then read
`AutomationElement.FocusedElement` and `GetSupportedPatterns()`. Exit 0. All
three focus names matched; canvas label pattern list was empty.

The first launch's optional `WaitForInputIdle(15000)` emitted an exception that
the process might not have a graphical interface, while the native window did
exist and subsequent UIA reads succeeded. This API result was not treated as a
launch failure or a GUI pass. One later action probe exited 1 because the bounded
PID 24428 no longer existed. Another exited 1 on a null target during startup;
its PID 30476 was cleaned up with process exit 0. Both failed probes are retained
here rather than omitted from qualification.

Journey extraction used Python's `json.loads` on `round-*/journey.json` and printed
`outcome`, `passed`, assertion count, and matching assertion names. Exit 0;
`round-03-after`: `outcome=passed`, `passed=True`, `assertions=41`.

Final process queries: `Get-Process -Id 24428,30476,26240` and
`Get-Process -Name Narrator` with missing-process errors suppressed produced no
processes. Exit 0. No spoken-output pass is claimed.

## Historical next-qualification list

Retest the rebuilt binary's actual UIA action patterns, names and Tab/Enter
selection after the reported defects are fixed. Complete one all-keyboard
engineering journey and one Narrator listening session. Then test an installed
composition IME, mixed 100/150/200% monitor transitions, minimize/restore and rapid
resize, followed by controlled surface/device-failure recovery on disposable
test state. Preserve exact binary identity and observed outputs for each check.

## Subsequent port, keyboard and window qualification

At 16:57 UTC the actual executable SHA-256
`60306b41d2ae8e6dc7bc7514524ce15848ffd94d1c3ee970ea37fed3f59a98a1`
passed all seven assertions in
[port-uia-native-window.json](accessibility/port-uia-native-window.json):

- Explorer search exposes the name `Find an element`.
- Focusing ModelingPlatform exposes six real fixture ports as accessible Buttons.
- A port supports UIA Invoke, accepts keyboard focus, and invocation selects its
  corresponding Inspector heading.
- Minimize followed by restore retains the interactive native window.
- Four resize requests (1280×800, 1600×1000, 1100×760, 1600×1000) retain an
  invokable command palette with named input `Command or element`.

The driver exited 0. Native stderr was empty. It closed only its owned PID12252,
which was subsequently confirmed absent. PowerShell did not expose the native
process exit code (recorded null); the driver exit is not substituted for it.
Two earlier attempts (PID9488 and PID21752) queried the console window returned
by MainWindowHandle and failed with an empty accessibility tree. Both were closed
and confirmed absent. The corrected driver selects the titled native window
within the exact owned process. Failed artifacts are retained.

The separate [keyboard journey](accessibility/keyboard-after-ports.json) passed
with no pointer events or source editing. It covers visual navigation, selection,
Explain, candidate review/cancel and the fixture validation/commit prohibition;
it does not establish a real semantic commit by keyboard.

Focused System World now shows port names without requiring an individual Part
selection. Port direction remains `not specified` when the projection provides
none. Toolkit and camera reduced-motion settings are now controlled together,
and restored reduced-motion preferences initialize the toolkit setting. Canvas
node focus selects the corresponding semantic object. Arrow navigation uses the
filtered Explorer rows, and disclosure controls include the owner's name.
These source changes do not establish comprehensive accessibility compliance.

The subsequent integrated build passed the seven UIA checks again at 17:09 UTC
(`accessibility/integrated-uia-native-window.json`, executable SHA-256
`0226b5e5f76bcfe982a17aa3a806b21de67430576dc5de72a5c6083c5535bc52`).
Port names now include their actual owner and known/unspecified direction, so
`request` on ModelRepository differs from `request` on ModelingService. The
separate `accessibility/integrated-keyboard.json` journey also passed.

At 17:11 UTC an additional actual Windows Narrator coexistence smoke check
completed the same native accessibility action sequence with Narrator running,
then confirmed that Narrator remained alive. All eight assertions passed,
driver exit 0 (`checks/narrator-coexistence-smoke.json` and
`accessibility/narrator-coexistence.json`). No prior Narrator session was present.
The script closed only its own native process and Narrator PID; Narrator required
a bounded forced close after its normal window-close request did not exit.
Subsequent process inspection found neither process running. **Spoken output was
not listened to or assessed**; this is a coexistence/action smoke check, not a
screen-reader speech or usability qualification. IME composition, mixed-DPI
monitor movement and controlled device failure remain unqualified.
