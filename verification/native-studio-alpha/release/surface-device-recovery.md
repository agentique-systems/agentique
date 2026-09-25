# Surface and device recovery: pinned API audit

Audit date: 2026-09-25. Pinned versions: eframe/egui-wgpu **0.33.3**, wgpu **27.0.1**. Exact local dependency source hashes and inspected locations are retained in [surface-api-audit.json](surface-api-audit.json). This follows the earlier [release qualification](../release-qualification.md); its original binary observations remain historical evidence.

## What the dependency already handles

Eframe's native resize handler ignores zero-sized Windows minimize events and passes nonzero dimensions to its Painter. Painter reconfigures the surface and recreates its depth/MSAA attachments for resize. These are inspected implementations, not a completed mixed-DPI/minimize stress qualification. Sources: [eframe native integration, lines 827–842](https://docs.rs/eframe/0.33.3/src/eframe/native/wgpu_integration.rs.html), [egui-wgpu Painter, resize implementation](https://docs.rs/egui-wgpu/0.33.3/src/egui_wgpu/winit.rs.html).

`SurfaceErrorAction::RecreateSurface` is more limited than its name suggests: Painter calls `configure_surface` on its existing surface with its existing device, then skips the failed frame. It does not create a new device or rebuild the renderer. Source: egui-wgpu `src/winit.rs:476–482`, with `configure_surface` at `91–120` in the linked pinned source.

## Focused changes

The old Studio callback requested reconfiguration for Lost/Outdated but did not schedule another frame. An idle window could therefore remain stale until another input. The new callback schedules five bounded retries at 16, 50, 100, 250 and 500 ms. Lost/Outdated use eframe's reconfiguration action; Timeout skips the failed acquisition and retries. Exhaustion reports a diagnostic in stderr and in the operating-system window title, and blocks model-edit input until a surface is acquired again. Resize/manual input can trigger a new acquisition. OutOfMemory/Other do not start an automatic retry loop.

A minimal paint callback records successful surface **acquisition**, including the startup screen. It resets the retry budget and resumes the Studio view after recovery. This is not a presentation timestamp or a claim that pixels reached the display.

The custom GPU timestamp ring is reset following a failed surface acquisition. In this pinned Painter, custom `prepare` callbacks run before `get_current_texture`; on failure, the encoder containing timestamp resolution/copy commands is dropped without submission. Treating those buffers as submitted would contaminate later measurements. Previously accepted timing samples remain; the interrupted ring is discarded and the timestamp error count advances. No semantic model or revision is touched.

The wgpu device-lost callback now makes actual device loss explicit in stderr and the OS window title. Normal `Destroyed` teardown is ignored. New editor input is blocked after device loss; the UI continues receiving in-flight semantic outcomes and its normal presentation persistence logic. It does not exit the process or interrupt a durable transaction. The custom scene stops submitting its own GPU commands to that device. Source for the callback: [wgpu Device, `set_device_lost_callback`](https://docs.rs/wgpu/27.0.1/wgpu/struct.Device.html#method.set_device_lost_callback).

## Exact limitation and remaining release risks

**Full device recreation is not implemented. Restart is required after actual device loss.** Eframe owns a private native Painter whose `render_state` is distinct from the `RenderState` clone exposed to the application. That clone includes shared renderer resources, but replacing its device/queue does not replace Painter's device, surfaces, attachments and texture bookkeeping. The pinned App contract exposes no coordinated replacement operation. Swapping only Studio's custom pipeline or `Frame::wgpu_render_state` would mix resources from different devices. Sources: egui-wgpu `src/winit.rs:33,87–89,198–208`, [RenderState definition](https://docs.rs/egui-wgpu/0.33.3/egui_wgpu/struct.RenderState.html), [eframe Frame render access](https://docs.rs/eframe/0.33.3/eframe/struct.Frame.html#method.wgpu_render_state).

Reconfiguration may itself fail if the native surface is no longer compatible with its original adapter. Painter's `configure_surface` uses an `expect` for the default configuration. There is no application hook here to replace an irrecoverable OS surface. Wgpu resource allocation/validation/internal errors outside surface acquisition retain the default uncaptured-error behavior, which may panic; this change does not claim general OOM or driver-fault recovery. The default error dispatch is inspected in wgpu `src/backend/wgpu_core.rs:645–683`.

No framework change or speculative renderer replacement was attempted. A future complete device recovery would need a coordinated eframe/Painter lifecycle capability or a narrowly maintained integration change, followed by restoring fonts, attachments, custom resources and device-specific timestamp queries together.

## Verification scope

Focused tests exercise bounded Lost retry, Timeout/OOM policy, reset after acquisition, persistent device-loss state, and ordinary Destroyed teardown. The device-loss message is injected into the callback state; no real driver reset is performed. These tests and the native build/check results are recorded separately by the integration lead using the shared build target.

Still unqualified: actual GPU reset/removal, driver failure during commit, physical surface loss across suspend/resume, prolonged resize/minimize stress, multi-monitor mixed DPI, and cross-platform recovery. Native screenshots and ordinary frame benchmarks cannot establish those passes.
