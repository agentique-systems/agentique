# Native input scenario

`crates/studio-native/src/automation.rs` provides an opt-in smoke runner for the
actual eframe application. It injects egui `RawInput` keyboard and pointer events
before each native frame, then asserts the resulting application state on later
frames. It never calls `execute`, prepares model commands itself or writes a
repository. The driver requires the explicit `architecture` fixture and rejects
any live project/branch binding.

The `vertical` scenario has 37 stages: selection and shift-selection, Escape,
double-click and F focus, owner/back navigation, canvas drag, anchored wheel
zoom, Graph World, derived-edge hit selection, Explain, command palette,
dependency overlay, revision comparison, nested-part dialog text entry and
candidate preview, disabled validation/commit attempts, cancellation and stale
selection removal, actual history-row selection, contrast/theme and a final
engineering selection.

Every stage records the injected events, before/after state, expected result and
pass/fail result. A stage deadline is 180 settling frames; the global deadline is
2,400 native frames. Frame time supplied to egui advances deterministically at
1/60 second for reproducible double-click and scroll behavior. These timings are
test inputs, not performance measurements. Captured frame timings from scenario
runs must not be reported as physical input latency.

An intermediate report has `outcome: running` and `passed: false`. Failed
assertions are written with `outcome: failed` before returning `Err`. Only all 37
assertions passing produces `outcome: passed`. The caller must terminate with a
nonzero exit on errors; completing the native event loop alone is not acceptance.
Reports belong in ignored `verification/generated/native-studio/`.

## Integration hooks

Register `mod automation;` in the binary. Add opt-in CLI arguments:

```rust
#[arg(long, value_parser = ["vertical", "stress"])]
scenario: Option<String>,
#[arg(long, default_value = "verification/generated/native-studio/interaction-report.json")]
scenario_report: PathBuf,
```

Add the ordinary eframe input hook; no new application state field is needed:

```rust
fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
    if let Some(name) = &self.args.scenario {
        let outcome = if name == "stress" {
            crate::stress_automation::drive(self, ctx, raw, &self.args.scenario_report)
        } else {
            crate::automation::drive(self, ctx, raw, name, &self.args.scenario_report)
        };
        match outcome {
            Ok(crate::automation::ScenarioStatus::Running) => {}
            Ok(crate::automation::ScenarioStatus::Complete) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => {
                eprintln!("Native fixture interaction FAILED: {error}");
                std::process::exit(2);
            }
        }
    }
}
```

Existing frame-count/screenshot exit logic must not close the window before the
scenario completes. Session saving should be skipped for explicit scenario runs.
The driver requests repaint while running; no timer, remote service or hidden
alternative action handler is necessary.

Record actual widget rectangles after ordinary UI creation:

```rust
use crate::automation::{self, Target};
automation::record(ui.ctx(), Target::Viewport, viewport_rect);
automation::record(ui.ctx(), Target::PaletteInput, input_response.rect);
automation::record(ui.ctx(), Target::CandidateName, name_response.rect);
automation::record(ui.ctx(), Target::CandidatePrepare, prepare_response.rect);
automation::record(ui.ctx(), Target::HistoryRevision(revision), history_row_rect);
```

Node and edge clicks use the actual camera/scene geometry and spatial hit index.
Widget rectangles locate ordinary palette/dialog/history widgets without hard
coded screen pixels. Recording them does not invoke or bypass any widget action.

## Run and evaluate

```powershell
cargo run --manifest-path crates/studio-native/Cargo.toml --release -- --fixture architecture --no-restore --scenario vertical --scenario-report verification/generated/native-studio/interaction-report.json
```

The exit code must be zero **and** the report must contain `passed: true`,
`outcome: passed`, 37 assertions and no failed assertion. Retain the actual command,
exit code and assertion summary after running it. The runner's existence or unit
tests do not establish that the native scenario passed. The integration lead
performs and records the actual native run.

The native shell has its own Cargo workspace, so use its manifest path rather
than `cargo run -p agq-studio-native` from the language workspace. The integration
currently reports scenario errors and exits with code 2; a process failure must
not be converted to a passing report by a wrapper script.

Current verification status: the first three actual native runs exposed palette
input timing, cross-object click counting and a port-identity assertion mismatch.
Those corrections are recorded in [the quality review](quality-review.md).
The corrected release runs `interaction-4.json` and `interaction-final.json`
both passed all **37 stages**, with zero failed assertions and process exit
code **0**. The actual final invocation from the repository root was:

```powershell
target/release/agq-studio-native.exe --fixture architecture --no-restore --scenario vertical --scenario-report verification/generated/native-studio/interaction-final.json
```

Run 4 used the same arguments with report path
`verification/generated/native-studio/interaction-4.json`. Both deterministic
reports have SHA-256
`e90f0dda72946dcd35c89c89e5e6fbc45ba9b0334fe41233e974aad472ae78a8`.
The [compact acceptance record](interaction-acceptance.json) retains their
commands, process results, scope and all assertion names/results. No current
executable hash is attributed to these runs: the executable was subsequently
rebuilt. This evidence accepts the visual fixture interaction path; disabled
fixture validation/commit are intentional authority-boundary checks. Real
runtime-backed semantic reconstruction, validation and commit are not established
by the scenario.
