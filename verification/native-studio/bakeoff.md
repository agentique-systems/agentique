# Native framework bakeoff evidence

Base: `9fa5c1357b4faff837fff507173d0f90bb4ead9f`, fetched main, isolated
`work/native-bakeoff` worktree. Probe inputs and lockfiles are retained in
`tools/native-bakeoff/`; exact tested iterations are described below. Routine
stdout/stderr and screenshots are in ignored
`verification/generated/native-bakeoff/`, following ADR 0021.

Toolchain: rustc 1.92.0 (`ded5c06cf`), x86_64-pc-windows-msvc, LLVM 21.1.3.
Hardware: Ryzen 5 5600X, RTX 3060 Ti, driver 32.0.16.1692.
These are visual fixtures; no language or runtime acceptance is asserted.

## Commands actually executed

| Command | Exit | Actual result |
|---|---:|---|
| `cargo info eframe@0.36.2` | 0 | MSRV 1.95; installed toolchain is 1.92 |
| `cargo info eframe@0.33.3` | 0 | MSRV 1.88; MIT OR Apache-2.0 |
| `cargo info slint@1.18.1` | 0 | MSRV 1.92; GPU feature available |
| `cargo info gpui@0.2.2` | 0 | Apache-2.0; Windows backend in source |
| `cargo build --manifest-path tools/native-bakeoff/egui/Cargo.toml` | 0 | Initial dependency build 60.98 s; real shell incremental build 8.91 s |
| `cargo build --manifest-path tools/native-bakeoff/slint/Cargo.toml` | 0 | Initial dependency build 174.92 s; real GPU shell build 18.03 s after source fix |
| `cargo build --manifest-path tools/native-bakeoff/gpui/Cargo.toml` | 0 | Initial dependency build 214.96 s; real canvas shell build 17.99 s |
| `agq-bakeoff-egui.exe --bench` (initial feature set) | panic | `No wgpu backend feature ... was enabled`; fixed by explicit backend features |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tools/native-bakeoff/capture.ps1 -Candidate egui` | 0 | 360 frames; 6209.58 ms; 57.97 delivered fps; UI update p50 0.901 ms, p95 1.121 ms |
| `agq-bakeoff-slint.exe --bench` | completed | 360 frames; 48246.51 ms; 7.46 delivered fps; first launcher did not retain application exit code |
| `agq-bakeoff-gpui.exe --bench` | 0 | 360 frames; 109064.10 ms; 3.30 delivered fps, before list/path optimization |
| `cargo build --release --jobs 2 --manifest-path tools/native-bakeoff/slint/Cargo.toml` | 0 | Optimized real GPU shell compiled in 8m 44s |
| `cargo build --release --jobs 2 --manifest-path tools/native-bakeoff/gpui/Cargo.toml` | 101 | Build script: `Failed to find fxc.exe` |
| `rg --files 'C:/Program Files (x86)/Windows Kits/10/bin' -g fxc.exe -g dxc.exe` | 0 | Found installed x64 fxc at SDK 10.0.22621.0 |
| `cargo clippy --locked --manifest-path tools/native-bakeoff/egui/Cargo.toml --all-targets -- -D warnings` | 0 | No warnings, before diagnostic adapter print was added |

Initial dependency timings include acquisition/cache locks and parallel builds,
and used placeholder entry points to warm independent dependency graphs. They
are build-environment observations, not representative application build gates.
The subsequent real-shell builds and launches establish GPU integration.

The first Slint source compile failed on Rust's lexing of inline Slint hex colors
ending in `e`; changing the color spelling fixed it. The first GPUI launch
capture timed out at the script's bounded wait; the application later completed.
Neither failure is a semantic acceptance result.

Debug measurements included other build activity and different preparation
paths. They **must not be interpreted as toolkit performance ceilings**. Slint
and egui execute identical WGSL without culling; GPUI originally rebuilt an
unvirtualized list and individual paths. GPUI now uses a virtualized list and
one batched path. Release measurements and final-source checks are a follow-up
to this initial integration record, not silently inferred from earlier results.

Three native windows were rendered. egui and GPUI captures were inspected;
GPUI rendered the CJK text sample, while default egui fonts displayed missing
glyphs. Slint's initial capture selected a 1 × 1 helper window: the capture tool
now selects the largest visible window belonging to the exact launched process.
No bakeoff screenshot is claimed as a product-quality review round.

## Explicit open qualifications

Release comparison, final-source Clippy, a representative Slint screenshot,
screen-reader behavior, full keyboard interaction, IME composition, clipboard,
mixed DPI and non-Windows runtime testing remain open in this initial record.
The architectural decision is supported by the demonstrated GPU composition
seams and does not depend on declaring these unfinished checks successful.
