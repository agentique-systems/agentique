use eframe::{egui, egui_wgpu};
use egui_wgpu::wgpu;
#[path = "../../gpu.rs"]
mod gpu;
use std::time::Instant;
struct SceneCallback(gpu::View);
impl egui_wgpu::CallbackTrait for SceneCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        resources.get::<gpu::Gpu>().unwrap().update(queue, self.0);
        vec![]
    }
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        resources.get::<gpu::Gpu>().unwrap().paint(pass);
    }
}
struct App {
    selected: usize,
    zoom: f32,
    pan: egui::Vec2,
    dark: bool,
    dialog: bool,
    search: String,
    frames: usize,
    started: Instant,
    samples: Vec<f64>,
    bench: bool,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = cc.wgpu_render_state.as_ref().unwrap();
        println!("adapter={:?}", state.adapter.get_info());
        state
            .renderer
            .write()
            .callback_resources
            .insert(gpu::Gpu::new(&state.device, state.target_format));
        Self {
            selected: 42,
            zoom: 0.065,
            pan: egui::vec2(10., 30.),
            dark: true,
            dialog: false,
            search: String::new(),
            frames: 0,
            started: Instant::now(),
            samples: vec![],
            bench: std::env::args().any(|a| a == "--bench"),
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let begin = Instant::now();
        ctx.set_visuals(if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.dialog = false;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.dialog = true;
        }
        egui::TopBottomPanel::top("top")
            .exact_height(50.)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.strong("AGENTIQUE / native framework bakeoff");
                    ui.separator();
                    ui.label("System World");
                    if ui.button("Theme").clicked() {
                        self.dark = !self.dark;
                    }
                    if ui.button("Project…").clicked() {
                        self.dialog = true;
                    }
                });
            });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(format!(
                "VISUAL FIXTURE · 1,000 nodes · 2,000 edges · egui 0.33.3 / wgpu 27 · {:.0}%",
                self.zoom * 100.
            ));
        });
        egui::SidePanel::left("outliner")
            .default_width(220.)
            .show(ctx, |ui| {
                ui.heading("Project");
                ui.text_edit_singleline(&mut self.search);
                egui::ScrollArea::vertical().show_rows(ui, 24., 1000, |ui, rows| {
                    for id in rows {
                        if ui
                            .selectable_label(self.selected == id, format!("◈ Subsystem {id:04}"))
                            .clicked()
                        {
                            self.selected = id;
                        }
                    }
                });
            });
        egui::SidePanel::right("inspector")
            .default_width(260.)
            .show(ctx, |ui| {
                ui.heading(format!("Subsystem {:04}", self.selected));
                ui.label("Part definition");
                ui.separator();
                ui.label("IDENTITY");
                ui.label("Origin: authored visual fixture");
                ui.label("Revision: fixture-v1");
                ui.separator();
                ui.label("STRUCTURE");
                ui.label("2 outgoing relationships");
                ui.label("Agentique / Architecture");
                ui.separator();
                ui.label("TEXT QUALITY");
                ui.label("Ångström · Δpressure · 模型");
                ui.label("LongEngineeringSubsystemWithTechnicalNames");
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            if response.dragged() {
                self.pan += response.drag_delta();
            }
            if response.hovered() {
                let delta = ctx.input(|i| i.smooth_scroll_delta.y);
                if delta != 0. {
                    let old = self.zoom;
                    self.zoom = (self.zoom * (delta * 0.003).exp()).clamp(0.03, 3.);
                    let p = response.hover_pos().unwrap() - rect.min;
                    self.pan = p - (p - self.pan) * self.zoom / old;
                }
            }
            if response.clicked() {
                let p =
                    (response.interact_pointer_pos().unwrap() - rect.min - self.pan) / self.zoom;
                let col = (p.x / 240.).floor() as usize;
                let row = (p.y / 120.).floor() as usize;
                if col < 50 && row < 20 {
                    self.selected = row * 50 + col;
                }
            }
            response.context_menu(|ui| {
                if ui.button("Focus selection").clicked() {
                    self.zoom = 1.;
                    self.pan = egui::vec2(
                        40. - (self.selected % 50) as f32 * 240.,
                        40. - (self.selected / 50) as f32 * 120.,
                    );
                    ui.close();
                }
                if ui.button("Project details…").clicked() {
                    self.dialog = true;
                    ui.close();
                }
            });
            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                rect,
                SceneCallback(gpu::View {
                    size: [rect.width(), rect.height()],
                    zoom: self.zoom,
                    selected: self.selected as f32,
                    pan: [self.pan.x, self.pan.y],
                    dark: 1.,
                    pad: 0.,
                }),
            ));
            if self.zoom > 0.35 {
                for id in 0..1000 {
                    let p = rect.min
                        + self.pan
                        + egui::vec2((id % 50) as f32 * 240. + 14., (id / 50) as f32 * 120. + 22.)
                            * self.zoom;
                    if rect.contains(p) {
                        ui.painter().text(
                            p,
                            egui::Align2::LEFT_TOP,
                            format!("Subsystem {id:04}"),
                            egui::FontId::proportional(13.),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
        });
        egui::Window::new("Project details")
            .open(&mut self.dialog)
            .show(ctx, |ui| {
                ui.label("Deterministic visual fixture — no semantic acceptance");
                ui.text_edit_singleline(&mut self.search);
            });
        self.samples.push(begin.elapsed().as_secs_f64() * 1000.);
        self.frames += 1;
        if self.bench {
            ctx.request_repaint();
            if self.frames == 360 {
                self.samples.sort_by(f64::total_cmp);
                println!(
                    "frames={} elapsed_ms={:.2} delivered_fps={:.2} cpu_ui_p50_ms={:.3} cpu_ui_p95_ms={:.3}",
                    self.frames,
                    self.started.elapsed().as_secs_f64() * 1000.,
                    self.frames as f64 / self.started.elapsed().as_secs_f64(),
                    self.samples[180],
                    self.samples[342]
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "Agentique framework bakeoff — egui",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default().with_inner_size([1440., 900.]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
