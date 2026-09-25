# Native framework probes

These are **visual fixtures**, not the production Studio or semantic acceptance.
See [the decision and evidence](../../docs/native-studio-bakeoff.md).

Each subdirectory is an isolated Cargo workspace with a committed lockfile:

```powershell
cargo run --locked --manifest-path tools/native-bakeoff/egui/Cargo.toml
cargo run --locked --manifest-path tools/native-bakeoff/slint/Cargo.toml
cargo run --locked --manifest-path tools/native-bakeoff/gpui/Cargo.toml
```

For a GPUI Windows release build, set `GPUI_FXC_PATH` to the installed Windows
SDK's `x64/fxc.exe` if it is not already on PATH. This machine used:

```powershell
$env:GPUI_FXC_PATH='C:/Program Files (x86)/Windows Kits/10/bin/10.0.22621.0/x64/fxc.exe'
cargo build --release --locked --manifest-path tools/native-bakeoff/gpui/Cargo.toml
```

Use `-- --bench` to request 360 redraws and print delivered frame cadence. egui
also reports UI update percentiles. These are not GPU timestamp measurements,
input latency measurements, or stable absolute acceptance gates. Run one
candidate at a time after compilation, on an unobscured desktop. Initial startup
and shader work are included; debug and release results must be distinguished.

`capture.ps1 -Candidate egui` launches a benchmark and captures its native window
under `verification/generated/native-bakeoff/`. Use `-Profile release` after a
release build. A slow run may outlive the script's bounded wait and print its PID;
do not confuse that timeout with a completed benchmark. Generated logs and images
are not committed as routine regression evidence.

The shared shader submits exactly 1,000 nodes and 2,000 edges; there is no culling.
The GPUI probe uses an equivalent native canvas workload rather than that shader.
The grid is intentionally dense and does not represent the product's layout.
