//! Opt-in camera benchmark through ordinary egui RawInput. Fixture-only;
//! no direct scene mutation and no semantic service commands.
use crate::{
    app::StudioApp,
    automation::{self, ScenarioStatus, Target},
    timing::Samples,
};
use agq_studio_scene::Point;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Vec2};
use std::{path::Path, time::Instant};

#[derive(Clone, Default)]
struct Runner {
    frame: usize,
    last: Option<Instant>,
    steady: Samples,
    pan: Samples,
    zoom: Samples,
    pan_origin: Option<Pos2>,
    pan_camera: Option<Point>,
    pan_max_world: f32,
    zoom_pointer: Option<Pos2>,
    zoom_anchor: Option<Point>,
    initial_zoom: f32,
    maximum_zoom: f32,
    maximum_anchor_error_world: f32,
    minimum_visible: Option<usize>,
    completed: bool,
}

pub fn drive(
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    report_path: &Path,
) -> Result<ScenarioStatus, String> {
    let id = egui::Id::new("native-camera-stress-scenario");
    let mut runner = ctx
        .data(|data| data.get_temp::<Runner>(id))
        .unwrap_or_default();
    if runner.completed {
        return Ok(ScenarioStatus::Complete);
    }
    let result = runner.advance(app, ctx, input);
    if !matches!(result, Ok(ScenarioStatus::Running)) {
        let report = runner.report(app, ctx, &result);
        if let Some(parent) = report_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(
            report_path,
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|error| format!("Cannot persist stress scenario report: {error}"))?;
        println!("{report}");
        runner.completed = true;
    }
    ctx.data_mut(|data| data.insert_temp(id, runner));
    if matches!(result, Ok(ScenarioStatus::Running)) {
        ctx.request_repaint();
    }
    result
}

impl Runner {
    fn advance(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<ScenarioStatus, String> {
        if !matches!(app.fixture.as_deref(), Some("stress1000" | "stress10000"))
            || app.binding.is_some()
        {
            return Err(
                "Stress input scenario requires a stress visual fixture and refuses live bindings"
                    .into(),
            );
        }
        if !app.ready || app.fit_pending || app.gpu_stats.lock().is_ok_and(|s| s.draw_calls == 0) {
            if app.frame_number > 600 {
                return Err("Native stress fixture did not become ready".into());
            }
            return Ok(ScenarioStatus::Running);
        }
        let viewport = automation::target(ctx, Target::Viewport)?;
        let now = Instant::now();
        if let Some(previous) = self.last.replace(now) {
            let interval = now.duration_since(previous).as_secs_f64() * 1000.0;
            match self.frame {
                61..=180 => self.steady.push(interval),
                181..=300 => self.pan.push(interval),
                301..=420 => self.zoom.push(interval),
                _ => {}
            }
        }
        if self.frame <= 300
            && let Some(before) = self.pan_camera
        {
            self.pan_max_world = self.pan_max_world.max(before.distance(app.camera.center));
        }
        if self.zoom_anchor.is_some() {
            self.maximum_zoom = self.maximum_zoom.max(app.camera.zoom);
            if let Some(error) = self.anchor_error(app, ctx) {
                self.maximum_anchor_error_world = self.maximum_anchor_error_world.max(error);
            }
            self.minimum_visible = Some(
                self.minimum_visible
                    .unwrap_or(usize::MAX)
                    .min(app.timing.visible_nodes),
            );
        }
        match self.frame {
            180 => {
                let origin = viewport.left_top() + Vec2::new(24.0, 24.0);
                self.pan_origin = Some(origin);
                self.pan_camera = Some(app.camera.center);
                pointer_button(input, origin, true);
            }
            181..=298 => {
                let step = (self.frame - 180) as f32;
                // A smooth triangle sweep stays within the viewport and returns
                // near the original context before the independent zoom phase.
                let offset = if step <= 59.0 { step } else { 118.0 - step };
                input.events.push(Event::PointerMoved(
                    self.pan_origin.unwrap() + Vec2::new(offset * 3.0, offset),
                ));
            }
            299 => pointer_button(input, self.pan_origin.unwrap(), false),
            300..=419 => {
                let pointer = *self.zoom_pointer.get_or_insert_with(|| {
                    viewport.min + Vec2::new(viewport.width() * 0.70, viewport.height() * 0.38)
                });
                if self.zoom_anchor.is_none() {
                    self.zoom_anchor = Some(app.camera.screen_to_world(Point::new(
                        pointer.x - viewport.left(),
                        pointer.y - viewport.top(),
                    )));
                    self.initial_zoom = app.camera.zoom;
                }
                input.events.push(Event::PointerMoved(pointer));
                input.events.push(Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: Vec2::new(0.0, if self.frame < 360 { 8.0 } else { -8.0 }),
                    modifiers: Modifiers::NONE,
                });
            }
            450 => {
                let error = self
                    .anchor_error(app, ctx)
                    .ok_or("Zoom anchor was never measured")?;
                if self.pan_max_world <= 20.0 {
                    return Err("Injected pointer drag did not pan the camera".into());
                }
                if self.maximum_zoom <= self.initial_zoom * 1.5 {
                    return Err("Injected wheel events did not meaningfully zoom the camera".into());
                }
                if self.maximum_anchor_error_world > 0.25 {
                    return Err(format!(
                        "Zoom lost its pointer anchor: maximum {} world units, final {error}",
                        self.maximum_anchor_error_world
                    ));
                }
                if self
                    .minimum_visible
                    .is_none_or(|visible| visible >= app.scene.nodes.len())
                {
                    return Err("Zoom did not exercise viewport culling".into());
                }
                return Ok(ScenarioStatus::Complete);
            }
            _ => {}
        }
        self.frame += 1;
        Ok(ScenarioStatus::Running)
    }
    fn anchor_error(&self, app: &StudioApp, ctx: &egui::Context) -> Option<f32> {
        let viewport = automation::target(ctx, Target::Viewport).ok()?;
        let pointer = self.zoom_pointer?;
        let after = app.camera.screen_to_world(Point::new(
            pointer.x - viewport.left(),
            pointer.y - viewport.top(),
        ));
        Some(self.zoom_anchor?.distance(after))
    }
    fn report(
        &self,
        app: &StudioApp,
        ctx: &egui::Context,
        result: &Result<ScenarioStatus, String>,
    ) -> serde_json::Value {
        serde_json::json!({
            "format": "agentique-native-stress-v1",
            "passed": matches!(result, Ok(ScenarioStatus::Complete)),
            "failure": result.as_ref().err(),
            "warmup_frames": 60,
            "phase_frame_intervals_ms": { "steady": self.steady.summary(), "pan": self.pan.summary(), "zoom": self.zoom.summary() },
            "camera_checks": {
                "maximum_pan_displacement_world": self.pan_max_world,
                "initial_zoom": self.initial_zoom,
                "maximum_zoom": self.maximum_zoom,
                "final_zoom": app.camera.zoom,
                "zoom_anchor_error_world": self.anchor_error(app, ctx),
                "maximum_zoom_anchor_error_world": self.maximum_anchor_error_world,
                "minimum_visible_nodes": self.minimum_visible,
            },
            "native_metrics": app.metrics_report(),
            "scope": "Deterministic synthetic pointer/wheel events enter the native RawInput path; camera behavior is asserted from resulting state. 60 warmup + 120 steady + 120 pan + 120 zoom intervals, then 30 settling frames. Vsync remains enabled. Neither GPU duration nor physical input-to-photon latency is measured."
        })
    }
}

fn pointer_button(input: &mut egui::RawInput, pos: Pos2, pressed: bool) {
    input.events.push(Event::PointerMoved(pos));
    input.events.push(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}
