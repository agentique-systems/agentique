//! Opt-in benchmarks through ordinary egui RawInput: the camera (on a
//! stress fixture; it changes no model and runs no model commands) and the
//! Conversation (a generated 200-message conversation in a new project,
//! scrolled, then streamed into; ROADMAP S4.1 gate G3).
//! Compiled only with `--features automation`.
use crate::{
    app::StudioApp,
    automation::ScenarioStatus,
    conversation::Live,
    targets::{Target, target},
    timing::Samples,
};
use agq_assistant::{Entry, ToolResult};
use agq_studio_scene::Point;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Vec2};
use serde_json::json;
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
    zoom_trace: Vec<serde_json::Value>,
    first_anchor_divergence_frame: Option<usize>,
}

pub fn drive(
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    report_path: Option<&Path>,
) -> Result<ScenarioStatus, String> {
    let id = egui::Id::new("native-camera-stress-scenario");
    let mut runner = ctx
        .data(|data| data.get_temp::<Runner>(id))
        .unwrap_or_default();
    if runner.completed {
        return Ok(ScenarioStatus::Complete);
    }
    let mut result = runner.advance(app, ctx, input);
    if !matches!(result, Ok(ScenarioStatus::Running)) {
        // A completed run that misses a budget fails (R-27, §8.6); the
        // report says so too.
        let budgets = runner.budgets(app);
        let missed = crate::budgets::missed(&budgets);
        if result.is_ok() && !missed.is_empty() {
            result = Err(format!("Budgets missed: {}", missed.join("; ")));
        }
        let report = runner.report(app, ctx, &result, &budgets);
        if let Some(report_path) = report_path {
            if let Some(parent) = report_path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::write(
                report_path,
                serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
            )
            .map_err(|error| format!("Cannot persist stress scenario report: {error}"))?;
        }
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
            || app.project.is_some()
        {
            return Err(
                "Stress input scenario requires a stress visual fixture and refuses live bindings"
                    .into(),
            );
        }
        if app.fit_pending || app.gpu_stats.lock().is_ok_and(|s| s.draw_calls == 0) {
            if app.frame_number > 600 {
                return Err("Native stress fixture did not become ready".into());
            }
            return Ok(ScenarioStatus::Running);
        }
        let viewport = target(ctx, Target::Viewport)?;
        if self.zoom_anchor.is_some() {
            let error = self.anchor_error(app, ctx);
            if error.is_some_and(|error| error > 0.25)
                && self.first_anchor_divergence_frame.is_none()
            {
                self.first_anchor_divergence_frame = Some(self.frame);
            }
            self.zoom_trace.push(serde_json::json!({
                "frame": self.frame,
                "camera_from_previous_update": app.camera,
                "viewport": [viewport.left(), viewport.top(), viewport.width(), viewport.height()],
                "stored_zoom_pointer": self.zoom_pointer.map(|p| [p.x, p.y]),
                "previous_input_pointer": ctx.input(|i| i.pointer.latest_pos()).map(|p| [p.x, p.y]),
                "previous_smooth_scroll_y": ctx.input(|i| i.smooth_scroll_delta.y),
                "incoming_os_events_before_synthetic_injection": input.events.iter().map(|event| format!("{event:?}")).collect::<Vec<_>>(),
                "pixels_per_point": ctx.pixels_per_point(),
                "scene_generation": app.generation,
                "camera_animation_pending": app.camera_target.is_some(),
                "anchor_error_world": error,
            }));
        }
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
                    phase: egui::TouchPhase::Move,
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
        let viewport = target(ctx, Target::Viewport).ok()?;
        let pointer = self.zoom_pointer?;
        let after = app.camera.screen_to_world(Point::new(
            pointer.x - viewport.left(),
            pointer.y - viewport.top(),
        ));
        Some(self.zoom_anchor?.distance(after))
    }
    /// The budgets of ROADMAP §3.3 this run measures. The start budget is
    /// not among them: a fixture's scene is built before the first frame, so
    /// it is measured by a run on the start screen instead.
    fn budgets(&self, app: &StudioApp) -> Vec<serde_json::Value> {
        use crate::budgets;
        let elements = app.scene.nodes.len();
        let pan_input = app.timing.input_summary(crate::timing::InputKind::Pan).p95;
        let zoom_input = app.timing.input_summary(crate::timing::InputKind::Zoom).p95;
        vec![
            budgets::result(
                "pan frame interval p95",
                budgets::frame_p95_ms(elements),
                self.pan.summary().p95,
            ),
            budgets::result(
                "zoom frame interval p95",
                budgets::frame_p95_ms(elements),
                self.zoom.summary().p95,
            ),
            budgets::result(
                "pan input to next update p95",
                budgets::input_p95_ms(elements),
                pan_input,
            ),
            budgets::result(
                "zoom input to next update p95",
                budgets::input_p95_ms(elements),
                zoom_input,
            ),
        ]
    }
    fn report(
        &self,
        app: &StudioApp,
        ctx: &egui::Context,
        result: &Result<ScenarioStatus, String>,
        budgets: &[serde_json::Value],
    ) -> serde_json::Value {
        serde_json::json!({
            "format": "agentique-native-stress-v1",
            "budgets": budgets,
            "passed": matches!(result, Ok(ScenarioStatus::Complete)),
            "failure": result.as_ref().err(),
            "first_anchor_divergence_frame": self.first_anchor_divergence_frame,
            "zoom_input_trace": self.zoom_trace,
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
            "scope": "Deterministic synthetic pointer/wheel events enter the native RawInput path; camera behavior is asserted from resulting state. 60 warmup + 120 steady + 120 pan + 120 zoom intervals, then 30 settling frames. Vsync remains enabled. Optional GPU timestamps measure only the scene pass; see native_metrics for availability and scope. Physical input-to-photon latency is not measured."
        })
    }
}

/// The S4.1 chat fixture (gate G3): 200 messages, 100 from the Operator and
/// 100 replies, about 60,000 words in all, with 50 code blocks, 100 tool
/// cards and 300 inline element names. Deterministic.
pub fn chat_fixture() -> (Vec<Entry>, Vec<ToolResult>) {
    const WORDS: [&str; 16] = [
        "the", "gateway", "stores", "each", "link", "in", "a", "table", "and", "returns", "its",
        "short", "code", "while", "requests", "wait",
    ];
    let mut entries = Vec::new();
    let mut results = Vec::new();
    for turn in 0..100 {
        entries.push(Entry::Operator {
            text: format!(
                "Turn {turn}: please look at `Shop::part{}` and explain how the requests flow through it, then change what needs changing.",
                turn % 40
            ),
        });
        let mut reply = String::new();
        for paragraph in 0..8 {
            if paragraph == 3 {
                // Three element names per reply: 300 in all.
                for link in 0..3 {
                    reply.push_str(&format!("`Shop::part{}` ", (turn * 3 + link) % 40));
                }
                reply.push_str("\n\n");
            }
            for word in 0..75 {
                reply.push_str(WORDS[(turn * 7 + paragraph * 3 + word) % WORDS.len()]);
                reply.push(if word % 17 == 16 { '.' } else { ' ' });
            }
            reply.push_str("\n\n");
        }
        if turn % 2 == 0 {
            reply.push_str("```rust\nfn shorten(url: &str) -> String {\n    let code = hash(url);\n    store(code, url);\n    code\n}\n```\n");
        }
        let id = format!("bench-{turn}");
        entries.push(Entry::Assistant {
            content: vec![
                json!({ "type": "text", "text": reply }),
                json!({
                    "type": "tool_use",
                    "id": id,
                    "name": agq_assistant::tools::APPLY_CHANGES,
                    "input": { "description": format!("Change part {turn}"), "operations": [] },
                }),
            ],
        });
        results.push(ToolResult {
            tool_use_id: id,
            content: "Changed.".into(),
            is_error: false,
            change: None,
        });
    }
    (entries, results)
}

#[derive(Clone, Default)]
struct ChatRunner {
    frame: usize,
    last: Option<Instant>,
    loaded: Option<Instant>,
    words: usize,
    first_frame_ms: Option<f64>,
    first_frame_ui_cpu_ms: Option<f64>,
    scroll: Samples,
    scroll_ui_cpu: Samples,
    stream: Samples,
    stream_ui_cpu: Samples,
    stream_started: Option<Instant>,
    streamed_chars: usize,
    completed: bool,
}

/// Frames of each phase of the chat benchmark.
const CHAT_LOAD: usize = 30;
const CHAT_SCROLL: std::ops::Range<usize> = 60..240;
const CHAT_STREAM: std::ops::Range<usize> = 250..550;
const CHAT_END: usize = 560;

pub fn drive_chat(
    app: &mut StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    report_path: Option<&Path>,
) -> Result<ScenarioStatus, String> {
    let id = egui::Id::new("native-chat-benchmark");
    let mut runner = ctx
        .data(|data| data.get_temp::<ChatRunner>(id))
        .unwrap_or_default();
    if runner.completed {
        return Ok(ScenarioStatus::Complete);
    }
    let result = runner.advance(app, ctx, input);
    if !matches!(result, Ok(ScenarioStatus::Running)) {
        let report = runner.report(app, &result);
        if let Some(report_path) = report_path {
            if let Some(parent) = report_path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::write(
                report_path,
                serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
            )
            .map_err(|error| format!("Cannot persist chat benchmark report: {error}"))?;
        }
        println!("{report}");
        runner.completed = true;
    }
    ctx.data_mut(|data| data.insert_temp(id, runner));
    if matches!(result, Ok(ScenarioStatus::Running)) {
        ctx.request_repaint();
    }
    result
}

impl ChatRunner {
    fn advance(
        &mut self,
        app: &mut StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<ScenarioStatus, String> {
        if app.project.is_none() {
            if self.frame > 0 {
                return Err(format!("The chat benchmark has no project: {}", app.status));
            }
            let folder = app
                .args
                .project
                .clone()
                .ok_or("The chat benchmark needs --project <a new folder>")?;
            app.create_project(&folder, "Shop");
            app.conversation.shown = true;
            self.frame += 1;
            return Ok(ScenarioStatus::Running);
        }
        let now = Instant::now();
        let interval = self
            .last
            .replace(now)
            .map(|previous| now.duration_since(previous).as_secs_f64() * 1000.0);
        // The UI pass that just ended belongs to the previous frame.
        let ui_cpu = app.timing.latency_report()["ui_cpu_ms"]["last"].as_f64();
        let previous = self.frame.saturating_sub(1);
        if let Some(interval) = interval {
            if CHAT_SCROLL.contains(&previous) {
                self.scroll.push(interval);
                if let Some(cpu) = ui_cpu {
                    self.scroll_ui_cpu.push(cpu);
                }
            } else if CHAT_STREAM.contains(&previous) {
                self.stream.push(interval);
                if let Some(cpu) = ui_cpu {
                    self.stream_ui_cpu.push(cpu);
                }
            }
        }
        if self.frame == CHAT_LOAD {
            let (entries, results) = chat_fixture();
            self.words = entries
                .iter()
                .map(|entry| match entry {
                    Entry::Operator { text } => text.split_whitespace().count(),
                    Entry::Assistant { content } => content
                        .iter()
                        .filter_map(|block| block["text"].as_str())
                        .map(|text| text.split_whitespace().count())
                        .sum(),
                    _ => 0,
                })
                .sum();
            app.conversation.conversation.entries = entries;
            app.conversation.results = results
                .into_iter()
                .map(|result| (result.tool_use_id.clone(), result))
                .collect();
            self.loaded = Some(now);
        } else if self.frame == CHAT_LOAD + 1 {
            // The pass that drew all 200 messages for the first time.
            self.first_frame_ms = self
                .loaded
                .map(|loaded| now.duration_since(loaded).as_secs_f64() * 1000.0);
            self.first_frame_ui_cpu_ms = ui_cpu;
        }
        if CHAT_SCROLL.contains(&self.frame) {
            // Over the message list, just above the message input.
            let field = target(ctx, Target::Field("Message"))?;
            let over = Pos2::new(field.center().x, field.top() - 200.0);
            let half = (CHAT_SCROLL.start + CHAT_SCROLL.end) / 2;
            input.events.push(Event::PointerMoved(over));
            input.events.push(Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, if self.frame < half { 60.0 } else { -60.0 }),
                modifiers: Modifiers::NONE,
            });
        }
        if CHAT_STREAM.contains(&self.frame) {
            // About 100 tokens a second, at about four characters a token.
            let started = *self.stream_started.get_or_insert(now);
            let due = (now.duration_since(started).as_secs_f64() * 400.0) as usize;
            if !matches!(app.conversation.live.last(), Some(Live::Text(_))) {
                app.conversation.live.push(Live::Text(String::new()));
            }
            if let Some(Live::Text(text)) = app.conversation.live.last_mut() {
                const STREAM: &str = "The gateway keeps each short code with its link, and a request for a code reads the table once. ";
                while self.streamed_chars < due {
                    let at = self.streamed_chars % STREAM.len();
                    text.push_str(&STREAM[at..at + 1]);
                    self.streamed_chars += 1;
                    if self.streamed_chars.is_multiple_of(600) {
                        text.push_str("\n\n");
                    }
                }
            }
        }
        if self.frame >= CHAT_END {
            if self.streamed_chars == 0 {
                return Err("Nothing was streamed".into());
            }
            return Ok(ScenarioStatus::Complete);
        }
        self.frame += 1;
        Ok(ScenarioStatus::Running)
    }

    fn report(
        &self,
        app: &StudioApp,
        result: &Result<ScenarioStatus, String>,
    ) -> serde_json::Value {
        json!({
            "format": "agentique-native-chat-v1",
            "passed": matches!(result, Ok(ScenarioStatus::Complete)),
            "failure": result.as_ref().err(),
            "fixture": {
                "messages": app.conversation.conversation.entries.len(),
                "words": self.words,
                "code_blocks": 50,
                "tool_cards": 100,
                "element_names": 300,
                "element_names_resolve_to_links": false,
            },
            "reopen": {
                "first_frame_interval_ms": self.first_frame_ms,
                "first_frame_ui_cpu_ms": self.first_frame_ui_cpu_ms,
            },
            "scroll": { "frame_interval_ms": self.scroll.summary(), "ui_cpu_ms": self.scroll_ui_cpu.summary() },
            "stream": {
                "tokens_per_second": 100,
                "streamed_chars": self.streamed_chars,
                "frame_interval_ms": self.stream.summary(),
                "ui_cpu_ms": self.stream_ui_cpu.summary(),
            },
            "native_metrics": app.metrics_report(),
            "scope": "The conversation is set in memory (no JSON read); the first frame after it is set parses and lays out every message. Element names are inline code spans; the project has no elements, so they are not links. Scrolling is 90 frames up and 90 down by wheel; streaming appends about 400 characters a second to a live reply. Vsync on.",
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
