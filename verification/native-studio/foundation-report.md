# Agentique Native Studio foundation

**AGENTIQUE NATIVE STUDIO FOUNDATION COMPLETE**

Native product architecture and deterministic fixture visual/interaction acceptance
are complete. Real-self-model semantic acceptance is blocked on the missing
authenticated runtime pair and is not claimed by this status.

Branch: `platform/native-studio-production-foundation`. The mission started from
a clean worktree after fetching `origin/main` at
`9fa5c1357b4faff837fff507173d0f90bb4ead9f`. Independent scene, framework and runtime
work used isolated worktrees; integration and quality reviews ran in this branch.

## Architecture

Selected UI framework: **egui/eframe 0.33.3**. Selected GPU stack: **wgpu 27.0.1**.
[ADR 0030](../../docs/adr/0030-agentique-native-studio-ui-and-rendering.md) is adopted
for this foundation. Representative egui, Slint and GPUI applications were built,
run and measured. Final warmed 1k-node/2k-edge cadence was 59.98, 60.41 and 60.00
FPS respectively. These vsync observations do not establish throughput ceilings.
The decision favors the demonstrated direct GPU composition boundary and keeps
toolkit dependence out of the scene and platform crates.

React Studio is **Agentique Studio Prototype 0**, retained as a functioning web
client, interaction reference and regression surface. HTTP and the Systems
Modeling API remain supported. The native application calls `studio-platform`
in-process through service, repository, view and candidate contracts.

The three production subsystems are:

- `studio-scene`: disposable, revision-bound presentation, layout, routing,
  spatial lookup, camera and LOD. It owns no canonical semantics.
- `studio-platform`: runtime bootstrap and bounded in-process modeling operations,
  shared self-model import, source-backed candidates and durable commit handling.
- `studio-native`: shell, worlds, command registry, navigation, shared selection,
  inspector, overlays, local session and retained GPU rendering.

Only the outer native UI crate has a separate Cargo workspace. Its UI feature
closure otherwise adds optional transitive edges to the accepted language lock.
The root language closure is proven identical to the authenticated original
69-package closure. The proof compares resolved edges, package identities and
checksums and rejects ambiguity; original source-input and publication receipts
are unchanged. See [compatibility evidence](../compatibility/README.md).

## Native engine

| Capability | Foundation implementation |
| --- | --- |
| GPU semantic scene | Yes; retained instance buffers, four ordered passes, camera uniform, toolkit glyph atlas |
| Camera | Yes; pointer-anchored zoom, pan, fit, focus and presentation navigation history |
| LOD | Yes; five levels with hysteresis and label/port detail suppression |
| Spatial hit testing / culling | Yes; indexed nodes, ports, edges, marquee and crossing-edge visibility |
| Hierarchical layout | Yes; containment-aware hierarchy and separate SCC-condensed graph layout |
| Edge routing | Yes; bounded orthogonal routing, obstacle detours, semantic ports, cycles and parallel lanes |
| Layout stability | Yes; identity-based layout memory per World, local-edit and comparison tests |
| Real ports | Yes; feature identity, independent selection, inspection and routed endpoints |
| Containment | Yes; nested layout, collapse, expand, focus and owner navigation |

Port direction remains unspecified when the public projection does not supply it.
Unknown semantic kinds remain generic. The bounded router reports obstructed
routes rather than claiming a globally optimal result for arbitrary dense graphs.
Removed comparison objects retain their old revision, and surviving container
envelopes include their removed children without changing saved layout state.

## Native product

System World, Graph World, Requirements foundation, Outliner, Inspector, Explain,
History, visual diff, command palette, agent semantic view and candidate world
are implemented on the shared scene and product contracts. Graph families and
deliberate incoming/outgoing neighborhoods include feature-port ownership.
History uses durable parent relationships and branch labels in live mode.

| Native product surface | Implemented |
| --- | --- |
| System World / Graph World | Yes / yes |
| Outliner / Inspector / Explain | Yes / yes / yes |
| History / visual diff | Yes / yes |
| Command palette / agent semantic view | Yes / yes |
| Candidate world / visual semantic edit | Yes / yes; Create nested PartUsage, with the acceptance boundary below |

The one semantic editing path is **Create nested PartUsage**. A live project
prepares source, reconstructs a candidate, validates it, and commits only through
an explicit operator action and durable compare-and-set receipt. A refused CAS
can be cancelled; an unknown acknowledgement retains the exact operation for
retry. There is no hidden mutable semantic undo stack and no agent auto-commit.

Fixture mode provides a clearly labeled visual candidate response to typed intent.
It cannot validate or commit. Rename, connection creation and deletion are not
implemented. Agent activity supplies dependency overlays and a provider-neutral
illustrative choice distribution; it does not claim calibrated probabilities or
an external provider integration.

Live navigation, restoration and candidate comparison request the intended World
and focus explicitly. Candidate/base pairs use the same lens. Stale worker replies
cannot replace the current context. Retained selection refreshes inspection
without losing multi-selection, and source loading never substitutes a different
revision's candidate source.

## Quality and evidence

Three native visual review rounds are documented in
[quality-review.md](quality-review.md). The retained product screenshots are
[System World](native-system-fixture.png) and
[revision comparison](native-diff-fixture.png). Both are actual native GPU surface
captures of explicitly labeled deterministic **visual fixtures**.

CPU scene measurements and their scope are retained in
[scene-performance.json](scene-performance.json). Layout measured 1.62 ms at
1k nodes and 27.74 ms at 10k; mean indexed hit testing measured 1.13 and 2.37 µs.
Those measurements are geometry workloads, not semantic edit acceptance or
physical input latency.

The native timing report distinguishes warmed wall-frame intervals, CPU scene
build, CPU queue upload, indexed hits and handled gesture-to-next-UI-update
samples. Absent measurements serialize as null. GPU timestamps and physical
input-to-photon latency remain unmeasured.

The actual native keyboard/pointer vertical passed **37/37 assertions**, including
selection, focus, camera, derived-edge Explain, dependency views, revision diff,
nested-part dialog, safe fixture rejection of validation/commit, cancellation and
history. [Interaction acceptance](interaction-acceptance.json) retains the command
exits and assertion identities. It is not a user study or runtime acceptance.

[Native performance evidence](performance.md) separates steady, pan and zoom
phases at 1k/2k and 10k/20k. The measured scale limit and optimization follow-up
remain visible alongside the results; GPU and physical input latency are not
inferred from frame cadence.

| Final measurement | 1k nodes / 2k edges | 10k nodes / 20k edges |
| --- | ---: | ---: |
| Steady median frame interval | 16.667 ms (~60 FPS) | 16.670 ms (~60 FPS) |
| Pan / zoom P95 frame interval | 16.921 / 16.922 ms | 20.052 / 19.280 ms |
| Scene build including layout and index | 15.207 ms | 139.833 ms |
| Separate CPU layout fixture measurement | 1.62 ms | 27.74 ms |
| Native hit-test median / P95 | 3.1 / 5.1 µs | 3.95 / 9.5 µs |
| Scene draw calls | 2 | 2 |

The final measurements use a Ryzen 5 5600X and RTX 3060 Ti/Vulkan with vsync,
after competing builds and tests finished. Reusing ordered visible records and
removing redundant relationship-ID allocations reduced the observed 10k pan/zoom
P95 by approximately 25%/31% against the earlier quiet run. This is a single-run
comparison; 10k interaction tails still exceed one 60 Hz frame. Geometry, culling
and pointer-anchor assertions passed, and the final native System screenshot is
byte-identical to the retained third-round capture.

## Runtime and acceptance boundaries

**Accepted bundle recovered: no. First Light real self-model: no.** Local archives,
available worktrees, Git objects, release records and retained CI evidence did
not yield the authenticated accepted publication pair. No standards were
republished. Exact expected identities and the separate recovery/distribution
plan are in [runtime-recovery.md](runtime-recovery.md).

Native fixture acceptance concerns rendering, geometry, interactions and safe
disabled semantic actions. Native real-self-model acceptance remains pending
installation of an authenticated bundle and the real source-backed operator
sequence. Fixture evidence does not establish language conformance, candidate
validation success, durable real-model edits or runtime publication acceptance.

The foundation includes keyboard operation, native accessibility integration,
semantic canvas descriptions, high contrast, reduced motion, Unicode font fallback
and atomic presentation restoration. Mixed-DPI transitions, screen-reader/IME and
complex-script qualification, device-loss recovery, installers and multi-window
operation remain release work. No 3D or executable simulation semantics were
introduced.

## Verification

All required regression commands completed successfully:

| Check | Exit / result |
| --- | --- |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` with documented low-artifact settings | 0; 1,067 passed, 11 existing gated/ignored tests |
| `npm run check`, `npm run build`, `npm test`, `npm run format:check` | 0; 25 Node tests passed |
| `npm run test:e2e` | 0; 8 browser tests passed |
| `npm run standards:check` | 0; immutable inputs and exact accepted language closure verified |
| `cargo run --locked --offline -p agq-metamodel-gen -- --check` | 0 |
| Standalone native formatting, strict Clippy and tests | 0; 21 native tests passed |
| Public scene/platform Rustdoc | 0 |
| `npm run test:studio` | 0; missing-runtime setup passed, real-runtime sequence gated/skipped |
| Actual native vertical and 1k/10k camera scenarios | 0; explicit state assertions passed |

The unloaded native setup was also launched and captured. It rendered no scene
objects or GPU scene draws, offered authenticated bundle installation, and exposed
fixtures only through an explicit action. A focused test protects that separation.

Actual command summaries are retained in `verification/summaries/native-studio/`;
full stdout/stderr stays in ignored `verification/generated/native-studio/` under
the repository evidence policy. Failed attempts remain recorded. The Windows
workspace test run first exhausted disk and then hit an MSVC debug-symbol limit;
recovery used Cargo cleanup of generated targets and the repository's documented
`low_artifact.py` wrapper. This disables debug symbols and incremental artifacts
while retaining normal test assertions, optimization levels and overflow checks.

No broad standards publication or ignored conformance corpus was run. Runtime
dependent tests remain explicitly gated on the missing authenticated assets.
