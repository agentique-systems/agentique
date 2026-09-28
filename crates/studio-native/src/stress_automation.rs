//! Opt-in benchmarks through the window's ordinary input dispatch: the
//! camera (on a stress fixture; it changes no model and runs no model
//! commands) and the Conversation (a generated 200-message conversation in a
//! new project, scrolled, then streamed into). Each runs one step at the
//! start of every frame, before the frame is drawn.
//! Compiled only with `--features automation`.
use crate::{
    automation::{self, Outcome},
    conversation::Live,
    studio::{Dirty, Studio},
    timing::Samples,
    ui::target,
    workspace::{StudioExt, Workspace},
};
use agq_assistant::{Entry, ToolResult};
use agq_studio_scene::Point;
use gpui::{
    App, Entity, Modifiers, PlatformInput, ScrollDelta, ScrollWheelEvent, TouchPhase, Window,
    point, px,
};
use serde_json::json;
use std::{path::Path, time::Instant};

/// Writes a benchmark's report, prints it, and fails the process when the
/// run failed or missed a budget.
fn finish(report: serde_json::Value, path: Option<&Path>) -> Outcome {
    if let Some(path) = path {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) =
            std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap_or_default())
        {
            eprintln!("Cannot write the report: {error}");
        }
    }
    println!("{report}");
    if report["passed"] != json!(true) {
        eprintln!("Benchmark FAILED: {}", report["failure"]);
        std::process::exit(2);
    }
    Outcome::Complete
}

fn wheel(at: gpui::Point<gpui::Pixels>, y: f32, window: &mut Window, cx: &mut App) {
    automation::move_to(at, None, window, cx);
    window.dispatch_event(
        PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(y))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        }),
        cx,
    );
}

#[derive(Default)]
pub struct Stress {
    frame: usize,
    waited: usize,
    last: Option<Instant>,
    steady: Samples,
    pan: Samples,
    zoom: Samples,
    pan_origin: Option<gpui::Point<gpui::Pixels>>,
    pan_camera: Option<Point>,
    pan_max_world: f32,
    zoom_pointer: Option<gpui::Point<gpui::Pixels>>,
    zoom_anchor: Option<Point>,
    initial_zoom: f32,
    maximum_zoom: f32,
    maximum_anchor_error_world: f32,
    minimum_visible: Option<usize>,
}

impl Stress {
    pub fn frame(
        &mut self,
        workspace: &Entity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> Outcome {
        let studio = workspace.read(cx).studio().clone();
        let mut result = self.advance(&studio, window, cx);
        if matches!(result, Ok(Outcome::Running)) {
            return Outcome::Running;
        }
        // A completed run that misses a budget fails (R-27, §8.6).
        let budgets = self.budgets(studio.read(cx));
        let missed = crate::budgets::missed(&budgets);
        if result.is_ok() && !missed.is_empty() {
            result = Err(format!("Budgets missed: {}", missed.join("; ")));
        }
        let report = self.report(studio.read(cx), &result, &budgets);
        finish(report, studio.read(cx).args.scenario_report.as_deref())
    }

    fn advance(
        &mut self,
        studio: &Entity<Studio>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Outcome, String> {
        let app = studio.read(cx);
        if !matches!(app.fixture.as_deref(), Some("stress1000" | "stress10000"))
            || app.project.is_some()
        {
            return Err("The stress benchmark needs a stress fixture and refuses projects".into());
        }
        if app.fit_pending || app.frame_number < 2 {
            self.waited += 1;
            if self.waited > 600 {
                return Err("The stress fixture did not become ready".into());
            }
            return Ok(Outcome::Running);
        }
        let viewport = target::find("Viewport").ok_or("the Surface is not shown")?;
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
            if let Some(error) = self.anchor_error(app) {
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
                let origin = viewport.origin + point(px(24.0), px(24.0));
                self.pan_origin = Some(origin);
                self.pan_camera = Some(app.camera.center);
                automation::move_to(origin, None, window, cx);
                automation::pointer_button(origin, true, window, cx);
            }
            181..=298 => {
                let step = (self.frame - 180) as f32;
                // A triangle sweep stays in the viewport and comes back near
                // where it started before the zoom phase.
                let offset = if step <= 59.0 { step } else { 118.0 - step };
                let origin = self.pan_origin.expect("set at frame 180");
                automation::drag_to(origin + point(px(offset * 3.0), px(offset)), window, cx);
            }
            299 => automation::pointer_button(
                self.pan_origin.expect("set at frame 180"),
                false,
                window,
                cx,
            ),
            300..=419 => {
                let pointer = *self.zoom_pointer.get_or_insert_with(|| {
                    viewport.origin + point(viewport.size.width * 0.70, viewport.size.height * 0.38)
                });
                if self.zoom_anchor.is_none() {
                    self.zoom_anchor = Some(app.camera.screen_to_world(local(pointer, viewport)));
                    self.initial_zoom = app.camera.zoom;
                }
                wheel(
                    pointer,
                    if self.frame < 360 { 8.0 } else { -8.0 },
                    window,
                    cx,
                );
            }
            450 => {
                let error = self
                    .anchor_error(app)
                    .ok_or("The zoom anchor was never measured")?;
                if self.pan_max_world <= 20.0 {
                    return Err("The pointer drag did not pan the camera".into());
                }
                if self.maximum_zoom <= self.initial_zoom * 1.5 {
                    return Err("The wheel did not zoom the camera meaningfully".into());
                }
                if self.maximum_anchor_error_world > 0.25 {
                    return Err(format!(
                        "Zoom lost its pointer anchor: at most {} world units, finally {error}",
                        self.maximum_anchor_error_world
                    ));
                }
                if self
                    .minimum_visible
                    .is_none_or(|visible| visible >= app.scene.nodes.len())
                {
                    return Err("Zooming did not exercise viewport culling".into());
                }
                return Ok(Outcome::Complete);
            }
            _ => {}
        }
        self.frame += 1;
        Ok(Outcome::Running)
    }

    fn anchor_error(&self, app: &Studio) -> Option<f32> {
        let viewport = target::find("Viewport")?;
        let after = app
            .camera
            .screen_to_world(local(self.zoom_pointer?, viewport));
        Some(self.zoom_anchor?.distance(after))
    }

    /// The budgets of ROADMAP §3.3 this run measures. The start budget is
    /// not among them: a fixture's scene is built before the first frame, so
    /// a run on the start screen measures it instead.
    fn budgets(&self, app: &Studio) -> Vec<serde_json::Value> {
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
        app: &Studio,
        result: &Result<Outcome, String>,
        budgets: &[serde_json::Value],
    ) -> serde_json::Value {
        json!({
            "format": "agentique-native-stress-v2",
            "budgets": budgets,
            "passed": result.is_ok(),
            "failure": result.as_ref().err(),
            "warmup_frames": 60,
            "phase_frame_intervals_ms": { "steady": self.steady.summary(), "pan": self.pan.summary(), "zoom": self.zoom.summary() },
            "camera_checks": {
                "maximum_pan_displacement_world": self.pan_max_world,
                "initial_zoom": self.initial_zoom,
                "maximum_zoom": self.maximum_zoom,
                "final_zoom": app.camera.zoom,
                "zoom_anchor_error_world": self.anchor_error(app),
                "maximum_zoom_anchor_error_world": self.maximum_anchor_error_world,
                "minimum_visible_nodes": self.minimum_visible,
            },
            "native_metrics": app.metrics_report(),
            "scope": "Synthetic pointer and wheel events enter GPUI's window dispatch at the start of each frame; camera behaviour is asserted from the resulting state. 60 warm-up + 120 steady + 120 pan + 120 zoom frame intervals, then 30 settling frames. Frame intervals are between GPUI's frame requests (the display's refresh, vsync on). GPU time and input-to-photon latency are not measured.",
        })
    }
}

fn local(pointer: gpui::Point<gpui::Pixels>, viewport: gpui::Bounds<gpui::Pixels>) -> Point {
    Point::new(
        f32::from(pointer.x - viewport.origin.x),
        f32::from(pointer.y - viewport.origin.y),
    )
}

/// The chat fixture: 200 messages, 100 from the Operator and 100 replies,
/// about 60,000 words in all, with 50 code blocks, 100 tool cards and 300
/// inline element names. Deterministic.
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
        entries.push(Entry::reply(
            None,
            &[
                json!({ "type": "text", "text": reply }),
                json!({
                    "type": "tool_use",
                    "id": id,
                    "name": agq_assistant::tools::APPLY_CHANGES,
                    "input": { "description": format!("Change part {turn}"), "operations": [] },
                }),
            ],
        ));
        results.push(ToolResult {
            tool_use_id: id,
            content: "Changed.".into(),
            is_error: false,
            change: None,
        });
    }
    (entries, results)
}

/// Frames of each phase of the chat benchmark.
const CHAT_LOAD: usize = 30;
const CHAT_SCROLL: std::ops::Range<usize> = 60..240;
const CHAT_STREAM: std::ops::Range<usize> = 250..550;
const CHAT_END: usize = 560;

#[derive(Default)]
pub struct Chat {
    frame: usize,
    last: Option<Instant>,
    loaded: Option<Instant>,
    words: usize,
    first_frame_ms: Option<f64>,
    first_frame_cpu_ms: Option<f64>,
    scroll: Samples,
    scroll_cpu: Samples,
    stream: Samples,
    stream_cpu: Samples,
    stream_started: Option<Instant>,
    streamed_chars: usize,
}

impl Chat {
    pub fn frame(
        &mut self,
        workspace: &Entity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> Outcome {
        let studio = workspace.read(cx).studio().clone();
        let result = self.advance(&studio, window, cx);
        if matches!(result, Ok(Outcome::Running)) {
            return Outcome::Running;
        }
        let report = self.report(studio.read(cx), &result);
        finish(report, studio.read(cx).args.scenario_report.as_deref())
    }

    fn advance(
        &mut self,
        studio: &Entity<Studio>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Outcome, String> {
        if studio.read(cx).project.is_none() {
            if self.frame > 0 {
                return Err(format!(
                    "The chat benchmark has no project: {}",
                    studio.read(cx).status
                ));
            }
            let folder = studio
                .read(cx)
                .args
                .project
                .clone()
                .ok_or("The chat benchmark needs --project <a new folder>")?;
            studio.act(cx, |studio| {
                studio.create_project(&folder, "Shop");
                studio.conversation.shown = true;
            });
            self.frame += 1;
            return Ok(Outcome::Running);
        }
        let now = Instant::now();
        let interval = self
            .last
            .map(|previous| now.duration_since(previous).as_secs_f64() * 1000.0);
        // The paint that ended after the previous step belongs to the
        // previous frame: from that step to the end of its paint.
        let cpu = match (self.last, crate::timing::last_paint_end()) {
            (Some(started), Some(ended)) if ended > started => {
                Some(ended.duration_since(started).as_secs_f64() * 1000.0)
            }
            _ => None,
        };
        let previous = self.frame.saturating_sub(1);
        if let Some(interval) = interval {
            if CHAT_SCROLL.contains(&previous) {
                self.scroll.push(interval);
                if let Some(cpu) = cpu {
                    self.scroll_cpu.push(cpu);
                }
            } else if CHAT_STREAM.contains(&previous) {
                self.stream.push(interval);
                if let Some(cpu) = cpu {
                    self.stream_cpu.push(cpu);
                }
            }
        }
        if self.frame == CHAT_LOAD {
            let (entries, results) = chat_fixture();
            self.words = entries
                .iter()
                .map(|entry| match entry {
                    Entry::Operator { text } => text.split_whitespace().count(),
                    Entry::Assistant { parts, .. } => parts
                        .iter()
                        .filter_map(|part| match part {
                            agq_providers::AssistantPart::Text { text } => Some(text),
                            _ => None,
                        })
                        .map(|text| text.split_whitespace().count())
                        .sum(),
                    _ => 0,
                })
                .sum();
            studio.act(cx, |studio| {
                studio.conversation.conversation.entries = entries;
                studio.conversation.results = results
                    .into_iter()
                    .map(|result| (result.tool_use_id.clone(), result))
                    .collect();
                studio.conversation.epoch += 1;
                studio.mark(Dirty::CONVERSATION);
            });
            self.loaded = Some(now);
        } else if self.frame == CHAT_LOAD + 1 {
            // The frame that drew all 200 messages for the first time.
            self.first_frame_ms = self
                .loaded
                .map(|loaded| now.duration_since(loaded).as_secs_f64() * 1000.0);
            self.first_frame_cpu_ms = cpu;
        }
        if CHAT_SCROLL.contains(&self.frame) {
            // Over the message list, just above the message field.
            let field = target::find("Message").ok_or("the message field is not shown")?;
            let over = point(field.center().x, field.origin.y - px(200.0));
            let half = (CHAT_SCROLL.start + CHAT_SCROLL.end) / 2;
            wheel(
                over,
                if self.frame < half { 60.0 } else { -60.0 },
                window,
                cx,
            );
        }
        if CHAT_STREAM.contains(&self.frame) {
            // About 100 tokens a second, at about four characters a token.
            let started = *self.stream_started.get_or_insert(now);
            let due = (now.duration_since(started).as_secs_f64() * 400.0) as usize;
            let streamed = &mut self.streamed_chars;
            studio.act(cx, |studio| {
                let live = &mut studio.conversation.live;
                if !matches!(live.last(), Some(Live::Text(_))) {
                    live.push(Live::Text(String::new()));
                }
                if let Some(Live::Text(text)) = live.last_mut() {
                    const STREAM: &str =
                        "The gateway keeps each short code with its link, and a request for a code reads the table once. ";
                    while *streamed < due {
                        let at = *streamed % STREAM.len();
                        text.push_str(&STREAM[at..at + 1]);
                        *streamed += 1;
                        if streamed.is_multiple_of(600) {
                            text.push_str("\n\n");
                        }
                    }
                }
                studio.mark(Dirty::CONVERSATION);
            });
        }
        self.last = Some(Instant::now());
        if self.frame >= CHAT_END {
            if self.streamed_chars == 0 {
                return Err("Nothing was streamed".into());
            }
            return Ok(Outcome::Complete);
        }
        self.frame += 1;
        Ok(Outcome::Running)
    }

    fn report(&self, app: &Studio, result: &Result<Outcome, String>) -> serde_json::Value {
        json!({
            "format": "agentique-native-chat-v2",
            "passed": result.is_ok(),
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
                "first_frame_cpu_ms": self.first_frame_cpu_ms,
            },
            "scroll": { "frame_interval_ms": self.scroll.summary(), "frame_cpu_ms": self.scroll_cpu.summary() },
            "stream": {
                "tokens_per_second": 100,
                "streamed_chars": self.streamed_chars,
                "frame_interval_ms": self.stream.summary(),
                "frame_cpu_ms": self.stream_cpu.summary(),
            },
            "native_metrics": app.metrics_report(),
            "scope": "The conversation is set in memory (no JSON read); the first frame after it is set parses and lays out every visible message. Element names are inline code spans; the project has no such elements, so they are not links. Scrolling is 90 frames up and 90 down by wheel; streaming appends about 400 characters a second to a live reply. Frame CPU is from the step at the start of a frame to the end of that frame's paint (GPU submission and presentation excluded). Vsync on.",
        })
    }
}
