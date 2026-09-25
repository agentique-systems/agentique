# ADR 0030: Agentique Native Studio UI and rendering architecture

Status: accepted for the native foundation. Full platform, accessibility and
international text qualification remain release gates.

## Context and evidence

ADR 0029 proved revision-bound product contracts in React. That client is
**Agentique Studio Prototype 0**, retained as a web client, interaction reference
and regression surface. Its SVG renderer and component boundaries are not the
production spatial runtime architecture.

The [engineering bakeoff](../native-studio-bakeoff.md) built representative
Windows applications in egui/eframe, Slint and GPUI. Each renders the same
deterministic 1,000-node, 2,000-edge visual workload with outliner, inspector,
theme, selection, zoom and popup surfaces. Slint and egui execute the same custom
WGSL; GPUI uses its native canvas renderer. The evidence records implementation
gaps and distinguishes delivered cadence from GPU timing. The decision does not
pretend that a small prototype establishes comprehensive toolkit superiority.

## Decision

Use **egui/eframe 0.33.3 for application chrome and wgpu 27.0.1 for a retained
semantic scene renderer**. Pin versions and Cargo.lock; upgrade deliberately.
Use the egui-wgpu callback boundary to share a device, queue and render pass.
Enable GPU backend features explicitly; enabling eframe's `wgpu` feature alone
with its defaults disabled does not select a platform GPU backend.

The primary reason is a demonstrated direct GPU composition seam with a small
Rust integration surface. Both the scene engine and in-process platform adapter
remain independent of egui. Native panels are not one widget per model element:
large lists are virtualized and the scene uses retained batches. Slint remains
a credible alternative if its declarative chrome materially improves product
quality; GPUI remains a credible alternative if a maintained portable custom
GPU seam and required accessibility support become available.

## Boundaries

```text
studio-native: native shell, theme, world tabs, panels, commands, GPU adapter
    -> studio-scene: disposable scene, camera, layout/routing, spatial index, LOD
    -> studio-platform: in-process product/session boundary
        -> modeling-view + modeling-agent + modeling-service
            -> repository -> ProjectWorkspace -> canonical graph
```

`studio-scene` must not import the UI toolkit or own canonical records. Rendering
consumes revision-bound projections and presentation geometry. Semantic visual
identity includes revision and element/relationship identity independently of
coordinates. Camera, selection, expansion, animation and layout belong to the
presentation session. A scene can be rebuilt or discarded without losing a
semantic edit. Toolkit callbacks and GPU resources stay behind the native GPU
adapter; they must not leak into layout or model-service contracts.

Application commands express typed semantic intent. A model edit prepares a
source-backed candidate, reconstructs through existing services, validates and
commits only under operator authority. Candidate cancellation never rewinds
durable branch history. Agents can propose a temporary view or candidate, never
silently move a branch head. Presentation gestures never masquerade as edits.

Local desktop access is in-process through product/service/view contracts.
The shell does not access SQLite tables, kernel storage or producer scheduling
internals. HTTP and the Systems Modeling API remain supported adapters for
external agents, automation, remote clients and interoperability.

Each window owns presentation context while sharing stable semantic identities
and service access. Future 3D renderers may share selection and revision identity;
the 2D camera and layout are not universal model coordinates. This decision adds
no simulation or other executable semantics.

## Quality obligations and consequences

The built-in egui font set did not render the bakeoff CJK sample. The native
product must provide appropriate fallback fonts, deliberate typography and
ellipsis/tooltips. Glyph coverage alone does not establish complex shaping,
bidirectional editing or IME correctness. Those require explicit language and
platform tests; a shaped-text adapter may be added without changing the scene
contract. GPUI's observed Windows fallback rendered the same CJK sample clearly.

Enable AccessKit for chrome, and supply a semantic accessibility representation
for custom scene objects. A GPU callback is not automatically accessible.
Keyboard equivalents, visible focus, high contrast and reduced motion are
product requirements. Screen-reader operation, IME composition and mixed-DPI
monitor transitions remain explicitly unqualified until exercised.

Retained node/edge buffers, bounded text work, culling, stable incremental layout
and query-bounded worlds provide scale. Measure CPU scene updates, upload,
render cadence and eventually GPU timestamps separately. An idle application
does not need continuous repainting; animation/input requests drive frames.
Surface/device loss, state restoration and multi-window ownership must remain
explicit renderer/session concerns.

The bakeoff tools are isolated experiments, not production dependencies. Their
synthetic grid has no semantic acceptance status. Missing authenticated runtime
assets do not prevent renderer development and never justify substituting a
fixture for real-model acceptance or republishing standards.

## Alternatives

- **Slint + wgpu:** the actual shared-device texture-import prototype builds
  and renders. Retained declarative chrome, text and accessibility are attractive.
  The public wgpu seam is explicitly versioned and unstable; texture composition
  adds target sizing and synchronization work. GPL/commercial/royalty-free terms
  differ from the repository's permissive dependency baseline. This is not an
  exclusion on licensing grounds: desktop use has a royalty-free option.
- **GPUI + custom scene:** a real Windows prototype builds and renders, with
  strong observed text quality. Published 0.2.2 uses a private D3D11 renderer on
  Windows rather than a public shared wgpu render pass. A portable wgpu scene
  would need additional integration work. Its entity system must not become
  semantic ownership, and accessibility qualification remains unresolved.
- **winit + wgpu + custom widgets:** maximum renderer control, but winit only
  supplies windows/events. Rebuilding text editing, IME, focus, widgets and
  accessibility would spend the foundation on a UI toolkit. Assessed at the
  documented architecture seam; no equivalent custom-widget prototype or
  performance claim is made.
- **React/Electron/Tauri as flagship:** preserve browser contracts and clients,
  but do not put them on the flagship native spatial rendering path.
