# Native Studio framework engineering bakeoff

Decision: [ADR 0030](adr/0030-agentique-native-studio-ui-and-rendering.md).
Reproducible probes: [tools/native-bakeoff](../tools/native-bakeoff).
Actual command evidence: [bakeoff verification](../verification/native-studio/bakeoff.md).

## Method

Build real native shells on Windows x86_64 using Rust 1.92.0. The machine has an
AMD Ryzen 5 5600X and NVIDIA GeForce RTX 3060 Ti (driver 32.0.16.1692).
The fixture is a fixed 50 × 20 grid, 1,000 identities and 2,000 directed edges
(successor and next-row neighbors, including wraparound). It is a deliberately
dense rendering workload, not a proposed professional layout or a semantic model.

All three applications have a main window, top bar, left outliner, central GPU
viewport, right inspector and status surface, theme control, selection highlight,
wheel zoom, scrolling, text samples and dialog/context-menu surfaces. egui and
Slint share `scene.wgsl`: 3,000 instanced quads in one draw, representing nodes
and thick edges. Only a 32-byte view uniform changes. GPUI renders native quads
and a batched stroked path, not the shared shader. Therefore its measurements
include a different scene preparation path and are not an isolated renderer
ranking. All 1,000 nodes and 2,000 edges are submitted; there is no culling in
these probes. Production culling results belong to the native engine benchmark.

The small probes are intentionally independent Cargo workspaces. They do not
link language crates, alter standards, use the network at runtime, or assert
semantic validation. The normal application shell, focus behavior and GPU
integration are exercised; a triangle/hello-world is not the benchmark.

## Evidence levels and score

Scores represent demonstrated integration coverage, not subjective quality:
one point each for (1) native compile and launch, (2) actual 1k/2k scene,
(3) custom WGSL on a portable wgpu composition seam, (4) shared-device integration
without platform-private renderer code. egui **4/4**, Slint **4/4**, GPUI **2/4**,
custom winit/widgets **not measured**. This tie between egui and Slint is resolved
by the simpler direct render-pass boundary and permissive integration baseline,
not by pretending the measured debug frame rates predict optimized ceilings.

`Observed` means compiled/runtime evidence, `source` means inspected published
implementation or primary documentation, and `open` means not qualified here.

| Criterion | egui/eframe 0.33.3 | Slint 1.18.1 | GPUI 0.2.2 | custom winit/wgpu |
|---|---|---|---|---|
| Rendering architecture | Observed wgpu callback | Observed wgpu texture import | Observed native D3D11 canvas | Source: direct surface |
| Custom renderer sharing | Observed same device/pass | Observed same device/texture | Private platform renderer; extra seam needed | Application owns it |
| Input model | Response/input snapshot | Declarative focus/touch callbacks | Actions/focus/listeners | OS event loop only |
| Text quality | Latin/Greek visible; default CJK missing | Observed Latin/Greek/CJK; shaping infrastructure | Observed crisp CJK/system fallback | Must supply shaper/atlas |
| Accessibility | AccessKit enabled; scene tree open | Accessibility feature enabled; assistive test open | Packaged API qualification open | Must implement |
| IME | Source integration; composition test open | Source integration; composition test open | Source platform input handler; test open | Events available; editing must be built |
| Clipboard | Source toolkit integration; round trip open | Source toolkit integration; round trip open | Source platform clipboard; round trip open | Must integrate |
| Windowing | Native Windows observed | Native Windows observed | Native Windows observed | winit already used beneath candidates |
| Docking | Resizable panels observed; arbitrary docking open | Layout primitives; docking not implemented | Layout primitives; docking not implemented | All widget behavior owned locally |
| High DPI | Logical points and pixels-per-point; multi-monitor open | Scale-aware toolkit; probe target fixed-size | Pixels/scaled pixels; multi-monitor open | ScaleFactorChanged handling owned locally |
| Large viewport | Actual submitted 1k/2k | Actual submitted 1k/2k | Actual submitted 1k/2k | No equivalent prototype |
| Custom shaders | Shared WGSL executed | Shared WGSL executed | No public portable wgpu pass found | Direct wgpu |
| Animation | Repaint scheduling exercised | Rendering notifier redraw exercised | Animation frame request exercised | Explicit scheduling |
| Rust ergonomics | Small direct callback adapter | Declarative UI + generated Rust bindings | Entity/context ownership + style builders | Entire UI abstraction must be built |
| Stability/maturity | Pinned pre-1.0; breaking upgrades expected | Stable chrome; GPU API explicitly unstable | Pinned pre-1.0; platform/docs drift observed | Stable boundaries but application burden |
| Cross-platform | Documented desktop support; only Windows tested | Documented desktop support; only Windows tested | Published Windows code works despite stale overview | Documented winit/wgpu platform support |
| Build complexity | 370 resolved packages initially | 552 resolved packages initially | 586 resolved packages | Unmeasured total widget/text stack |
| Future 3D | Independent pipeline can share pass/resources | External texture/underlay path | Requires maintained custom-render seam | Fully controlled |
| License metadata | MIT OR Apache-2.0 | GPL / royalty-free / commercial options | Apache-2.0 | wgpu/winit permissive licenses |
| Ecosystem risk | Immediate-mode cost; shaping and custom a11y work | Versioned GPU seam; renderer selection complexity | Zed coupling; private renderers; a11y uncertainty | Owning a toolkit indefinitely |

The points awarded above do not mean all interaction requirements passed. The
GPUI probe lacks editable text/IME and initially lacked drag panning; Slint's
simple overlay dialog does not prove modal focus trapping. The probes include
keyboard focus paths but no screen-reader run, accessibility audit, clipboard
round-trip or mixed-DPI monitor test. Those are explicit open product gates.

## Measured optimized workload

Final probes use 60 warmup frames followed by 360 measured frames, run serially
after their builds and the root's heavy compilation finished. All submitted
1,000 nodes and 2,000 edges. Final native screenshots were inspected. Default
vsync behavior is retained, so these numbers demonstrate display cadence, not
maximum throughput or a statistically established performance ranking.

| Probe | Measured interval | Delivered FPS | UI update p50 / p95 | GPU path |
|---|---:|---:|---:|---|
| egui/eframe | 6001.60 ms | 59.98 | 0.121 / 0.169 ms | RTX 3060 Ti / Vulkan, shared pass |
| Slint | 5959.65 ms | 60.41 | Not instrumented | RTX 3060 Ti / Vulkan, imported texture |
| GPUI | 5999.90 ms | 60.00 | Not instrumented | Native Windows D3D11 canvas |

The egui timer covers application UI construction, not all toolkit tessellation,
submission or GPU work. GPU timestamp duration, input-to-photon latency,
10k-node rendering and isolated shader throughput are **not measured by these
probes**. Production scene-engine measurements are separate. Earlier debug and
startup-inclusive numbers remain historical observations in the evidence record.

The first capture harness inherited a hidden startup state in GPUI, suppressing
its native window, and briefly selected console/helper windows. It now restricts
capture to the exact process and explicit probe title, reveals only that native
UI, waits for window animation, and removes stale captures. These harness defects
are not attributed to toolkit throughput. Slint's standard-widget palette was
also synchronized with the probe's theme; otherwise dark canvas with OS-light
widgets made the outliner illegible. Corrected screenshots were reviewed.

## Observations affecting the decision

An initial eframe build succeeded but launch panicked because disabling default
features also removed every native wgpu backend. The probe now enables explicit
backends. This is an integration defect found and fixed, not a toolkit failure.

The first GPUI implementation rebuilt a large widget list and tessellated each
edge separately. It was changed to `uniform_list` and one stroked edge path.
That finding reinforces the production separation between virtualized chrome
and retained scene batches. Debug timing before this change must not be cited
as GPUI's production limit.

Slint's texture path works with its 1.18.1 wgpu 30 API. Its published documentation
explicitly excludes these versioned GPU APIs from normal minor-version stability.
The probe uses the toolkit's device and queue and imports the texture without
CPU pixel readback. Its fixed-size render target is a probe limitation, not a
recommended high-DPI production design.

GPUI's published Windows source uses Direct3D 11, and its public canvas is a
real GPU renderer. It deserves serious consideration for a native engineering
application: system font fallback was visibly better than the unconfigured egui
default. The missing shared-wgpu seam is the main architectural tradeoff here.
No claim is made that current forks or unreleased GPUI branches have identical
constraints.

The first GPUI release build failed because `fxc.exe` was neither on PATH nor at
the build script's hard-coded Windows SDK version. The installed 10.0.22621.0
compiler was found and passed through the supported `GPUI_FXC_PATH` variable.
No toolchain download or platform renderer patch was needed. This practical
packaging dependency is retained in the reproduction instructions.

Custom winit/wgpu/widgets was screened out before a fourth full implementation.
The missing widgets, text editor, focus and accessibility layers make it a much
larger product burden. This is an architecture assessment, not a measured loss.

## Primary sources consulted

- [egui-wgpu custom callback API](https://docs.rs/egui-wgpu/0.33.3/egui_wgpu/trait.CallbackTrait.html): GPU preparation and render-pass composition.
- [eframe native options](https://docs.rs/eframe/0.33.3/eframe/struct.NativeOptions.html): windows, DPI-facing viewport configuration and rendering options.
- [egui context](https://docs.rs/egui/0.33.3/egui/struct.Context.html): AccessKit node construction, font registration, clipboard and viewport APIs.
- [egui upstream](https://github.com/emilk/egui): supported platforms, license and immediate-mode constraints.
- [Slint GPU features](https://docs.slint.dev/latest/docs/rust/slint/docs/cargo_features/): versioned unstable wgpu integration.
- [Slint renderers](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/): backend/text tradeoffs.
- [Slint pricing and license options](https://slint.dev/pricing): desktop royalty-free availability and alternative terms.
- [GPUI 0.2.2 API](https://docs.rs/gpui/0.2.2/gpui/): entity/render/canvas/input model and pre-1.0 status.
- GPUI 0.2.2 registry sources: `src/platform/windows/directx_renderer.rs`, `direct_write.rs`, `window.rs`, and `Cargo.toml` (exact bytes covered by Cargo.lock registry checksums).
- [winit 0.30.13](https://docs.rs/winit/0.30.13/winit/): window/event/DPI scope; no widget or drawing implementation.

Framework metadata was resolved on 2026-09-25. Latest eframe 0.36.2 requires Rust
1.95; this experiment deliberately pins 0.33.3 (MSRV 1.88) on installed Rust 1.92.
The pins are experiment identities, not a claim that these are every candidate's
newest or fastest available versions.
