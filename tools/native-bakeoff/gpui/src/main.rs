use gpui::{
    App, Application, Bounds, Context, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, PathBuilder, ScrollWheelEvent, Window, WindowBounds, WindowOptions, canvas,
    div, point, prelude::*, px, quad, rgb, size,
};
use std::time::Instant;
struct Bakeoff {
    selected: usize,
    zoom: f32,
    pan: [f32; 2],
    drag: Option<[f32; 2]>,
    dark: bool,
    dialog: bool,
    menu: bool,
    focus: FocusHandle,
    frames: usize,
    started: Instant,
    bench: bool,
}
impl Render for Bakeoff {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.frames += 1;
        if self.bench {
            window.request_animation_frame();
            if self.frames == 60 {
                self.started = Instant::now();
            }
            if self.frames == 420 {
                println!(
                    "warmup_frames=60 frames={} elapsed_ms={:.2} delivered_fps={:.2}",
                    self.frames - 60,
                    self.started.elapsed().as_secs_f64() * 1000.,
                    (self.frames - 60) as f64 / self.started.elapsed().as_secs_f64()
                );
                cx.quit();
            }
        }
        let selected = self.selected;
        let zoom = self.zoom;
        let pan = self.pan;
        let fg = if self.dark { 0xdbe5ec } else { 0x162530 };
        let bg = if self.dark { 0x101720 } else { 0xedf1f4 };
        let mut root = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(bg))
            .text_color(rgb(fg))
            .text_size(px(14.))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => {
                        this.dialog = false;
                        this.menu = false
                    }
                    "t" => this.dark = !this.dark,
                    "f" => {
                        this.zoom = 1.;
                        this.pan = [
                            40. - (this.selected % 50) as f32 * 240.,
                            40. - (this.selected / 50) as f32 * 120.,
                        ];
                    }
                    _ => {}
                }
                cx.notify();
            }))
            .child(
                div()
                    .h(px(50.))
                    .flex()
                    .items_center()
                    .px(px(16.))
                    .gap(px(24.))
                    .child("AGENTIQUE / native framework bakeoff")
                    .child("System World")
                    .child(
                        div()
                            .id("theme")
                            .cursor_pointer()
                            .child("Theme [T]")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.dark = !this.dark;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("dialog")
                            .cursor_pointer()
                            .child("Project…")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.dialog = true;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .w(px(220.))
                            .flex_shrink_0()
                            .p(px(12.))
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child("PROJECT")
                            .child(
                                gpui::uniform_list(
                                    "outliner",
                                    1000,
                                    cx.processor(
                                        move |this, range: std::ops::Range<usize>, _, cx| {
                                            range
                                                .map(|i| {
                                                    div()
                                                        .id(i)
                                                        .p(px(6.))
                                                        .cursor_pointer()
                                                        .bg(rgb(if i == this.selected {
                                                            0x326d77
                                                        } else {
                                                            bg
                                                        }))
                                                        .child(format!("Subsystem {i:04}"))
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                this.selected = i;
                                                                cx.notify();
                                                            },
                                                        ))
                                                })
                                                .collect::<Vec<_>>()
                                        },
                                    ),
                                )
                                .flex_1(),
                            ),
                    )
                    .child(
                        div()
                            .id("viewport")
                            .flex_1()
                            .overflow_hidden()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                    this.drag = Some([
                                        f32::from(event.position.x),
                                        f32::from(event.position.y),
                                    ]);
                                    let x = (f32::from(event.position.x) - 220. - this.pan[0])
                                        / this.zoom;
                                    let y = (f32::from(event.position.y) - 50. - this.pan[1])
                                        / this.zoom;
                                    let col = (x / 240.).floor() as usize;
                                    let row = (y / 120.).floor() as usize;
                                    if col < 50 && row < 20 {
                                        this.selected = row * 50 + col;
                                    }
                                    cx.notify();
                                }),
                            )
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(|this, _, _, cx| {
                                    this.menu = !this.menu;
                                    cx.notify();
                                }),
                            )
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                let current =
                                    [f32::from(event.position.x), f32::from(event.position.y)];
                                if event.pressed_button == Some(MouseButton::Left) {
                                    if let Some(previous) = this.drag {
                                        this.pan[0] += current[0] - previous[0];
                                        this.pan[1] += current[1] - previous[1];
                                        cx.notify();
                                    }
                                    this.drag = Some(current);
                                } else {
                                    this.drag = None;
                                }
                            }))
                            .on_scroll_wheel(cx.listener(
                                |this, event: &ScrollWheelEvent, _, cx| {
                                    let old = this.zoom;
                                    this.zoom = (this.zoom
                                        * (f32::from(event.delta.pixel_delta(px(20.)).y) * 0.003)
                                            .exp())
                                    .clamp(0.03, 3.);
                                    let pointer = [
                                        f32::from(event.position.x) - 220.,
                                        f32::from(event.position.y) - 50.,
                                    ];
                                    this.pan = [
                                        pointer[0] - (pointer[0] - this.pan[0]) * this.zoom / old,
                                        pointer[1] - (pointer[1] - this.pan[1]) * this.zoom / old,
                                    ];
                                    cx.notify();
                                },
                            ))
                            .child(
                                canvas(
                                    |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        let node = |id: usize| {
                                            point(
                                                bounds.origin.x
                                                    + px((id % 50) as f32 * 240. * zoom
                                                        + 110. * zoom
                                                        + pan[0]),
                                                bounds.origin.y
                                                    + px((id / 50) as f32 * 120. * zoom
                                                        + 44. * zoom
                                                        + pan[1]),
                                            )
                                        };
                                        let mut line = PathBuilder::stroke(px(1.6));
                                        for i in 0..2000 {
                                            let a = i % 1000;
                                            let b = (a + if i >= 1000 { 50 } else { 1 }) % 1000;
                                            line.move_to(node(a));
                                            line.line_to(node(b));
                                        }
                                        if let Ok(path) = line.build() {
                                            window.paint_path(path, rgb(0x3d5264));
                                        }
                                        for id in 0..1000 {
                                            let p = node(id);
                                            window.paint_quad(quad(
                                                Bounds {
                                                    origin: point(
                                                        p.x - px(103. * zoom),
                                                        p.y - px(38. * zoom),
                                                    ),
                                                    size: size(px(206. * zoom), px(76. * zoom)),
                                                },
                                                px(0.),
                                                rgb(if id == selected {
                                                    0x409faf
                                                } else {
                                                    0x213340
                                                }),
                                                px(0.),
                                                gpui::transparent_black(),
                                                Default::default(),
                                            ));
                                        }
                                    },
                                )
                                .size_full(),
                            ),
                    )
                    .child(
                        div()
                            .w(px(260.))
                            .flex_shrink_0()
                            .p(px(18.))
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                            .child(format!("Subsystem {selected:04}"))
                            .child("Part definition")
                            .child("IDENTITY")
                            .child("Authored visual fixture")
                            .child("Revision: fixture-v1")
                            .child("STRUCTURE")
                            .child("2 outgoing relationships")
                            .child("Agentique / Architecture")
                            .child("TEXT QUALITY")
                            .child("Ångström · Δpressure · 模型"),
                    ),
            )
            .child(
                div()
                    .h(px(25.))
                    .px(px(12.))
                    .child("VISUAL FIXTURE · 1,000 nodes · 2,000 edges · GPUI 0.2.2 / D3D11"),
            );
        if self.menu {
            root = root.child(
                div()
                    .absolute()
                    .left(px(420.))
                    .top(px(150.))
                    .w(px(230.))
                    .p(px(16.))
                    .bg(rgb(0x304251))
                    .child("Focus selection [F]")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.zoom = 1.;
                            this.pan = [
                                40. - (this.selected % 50) as f32 * 240.,
                                40. - (this.selected / 50) as f32 * 120.,
                            ];
                            this.menu = false;
                            cx.notify();
                        }),
                    ),
            );
        }
        if self.dialog {
            root = root.child(
                div()
                    .absolute()
                    .left(px(450.))
                    .top(px(300.))
                    .w(px(440.))
                    .h(px(200.))
                    .p(px(24.))
                    .bg(rgb(0x304251))
                    .flex()
                    .flex_col()
                    .gap(px(24.))
                    .child("Project details — deterministic visual fixture")
                    .child(
                        div()
                            .id("close")
                            .cursor_pointer()
                            .child("Close [Escape]")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.dialog = false;
                                cx.notify();
                            })),
                    ),
            );
        }
        root
    }
}
fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Agentique framework bakeoff — GPUI".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus);
                    Bakeoff {
                        selected: 42,
                        zoom: 0.065,
                        pan: [10., 30.],
                        drag: None,
                        dark: true,
                        dialog: false,
                        menu: false,
                        focus,
                        frames: 0,
                        started: Instant::now(),
                        bench: std::env::args().any(|a| a == "--bench"),
                    }
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
