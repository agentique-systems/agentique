//! The component gallery (`--fixture components`; ROADMAP §3.1, R-26, §8.5
//! rule 2): every token and component in one place, so drift in the design
//! system is visible at a glance. The colours are the generated dark, light
//! and high-contrast scales; the components draw in the Studio's current
//! theme, which the header switches (a preview: it is not saved). The
//! Surface and the Conversation samples are the real views, not copies.
use crate::{
    conversation_view::ConversationView,
    panels::{
        block_preview::{self, BlockPreview},
        library::{DragGhost, LibraryDrag, LibraryView},
    },
    studio::{Dirty, Studio},
    surface::paint,
    tokens::{self, Colours, HIGH_CONTRAST, INPUTS, Mode, Rgba, Scale},
    ui::{
        self, ActiveTheme, Badge, Banner, Button, Chip, EmptyState, IconName, KeyCaps, Menu,
        MenuItem, Segmented, Switch, TextArea, TextField, Tone, r, theme,
    },
    workspace::StudioExt,
};
use agq_studio_scene::{
    Camera2D, LodController, Scene, SceneLookup, SceneOptions, SceneTarget, Size, SpatialIndex,
    fixtures,
};
use gpui::{
    AnyElement, App, AppContext, Context, Entity, FontWeight, Hsla, InteractiveElement,
    IntoElement, ParentElement, Render, Rgba as GpuiRgba, SharedString, StatefulInteractiveElement,
    Styled, Window, canvas, div, prelude::FluentBuilder,
};
use gpui_base::input::{InputState, TextareaState};
use std::{collections::BTreeMap, rc::Rc};

/// A laid-out sample model for the Surface section.
struct Sample {
    scene: Rc<Scene>,
    spatial: Rc<SpatialIndex>,
    lookup: Rc<SceneLookup>,
    /// A card shown selected, and one shown as just changed by the Assistant.
    selected: Option<SceneTarget>,
    changed: Option<agq_studio_scene::ElementId>,
    /// What the sample frames: a container, or the whole model.
    framed: agq_studio_scene::Rect,
}

impl Sample {
    fn new(
        scene: Scene,
        framed: &str,
        selected_name: Option<&str>,
        changed_name: Option<&str>,
    ) -> Sample {
        let find = |name: &str| {
            scene
                .nodes
                .iter()
                .find(|n| n.semantic.name == name && !n.is_container)
                .map(|n| n.id())
        };
        let selected = selected_name.and_then(find).map(SceneTarget::Node);
        let changed = changed_name.and_then(find);
        let framed = scene
            .nodes
            .iter()
            .find(|n| n.is_container && n.semantic.name == framed)
            .map_or(scene.bounds(), |n| n.bounds.inflate(24.0));
        Sample {
            framed,
            spatial: Rc::new(SpatialIndex::build(&scene)),
            lookup: Rc::new(SceneLookup::build(&scene)),
            scene: Rc::new(scene),
            selected,
            changed,
        }
    }
}

pub struct Gallery {
    studio: Entity<Studio>,
    field: Entity<InputState>,
    invalid: Entity<InputState>,
    search: Entity<InputState>,
    large: Entity<InputState>,
    area: Entity<TextareaState>,
    area_invalid: Entity<TextareaState>,
    menu: Entity<Menu>,
    conversation: Entity<ConversationView>,
    library: Entity<LibraryView>,
    ghost: Entity<DragGhost>,
    /// Block previews with their captions.
    previews: Vec<(&'static str, Rc<agq_library::Preview>)>,
    samples: Vec<(&'static str, Rc<Sample>)>,
    switch_on: bool,
    segment: usize,
}

impl Gallery {
    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("A text field")
                .default_value("LinkStore")
        });
        let invalid = cx.new(|cx| InputState::new(window, cx).default_value("0..*..1"));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        let large = cx.new(|cx| InputState::new(window, cx).placeholder("Search or run a command"));
        let area_invalid = cx.new(|cx| {
            TextareaState::new(window, cx).default_value("Counts clicks per short code\nand")
        });
        let area = cx.new(|cx| {
            let mut state =
                TextareaState::new(window, cx).placeholder("A text area grows with its text");
            state.set_auto_grow(2, 6, cx);
            state
        });
        let menu = cx.new(|cx| {
            Menu::new(
                vec![
                    MenuItem::Header("Edit".into()),
                    MenuItem::action("Rename", |_, _| {})
                        .icon(IconName::Pencil)
                        .shortcut("F2"),
                    MenuItem::action("Move to…", |_, _| {})
                        .icon(IconName::Move)
                        .shortcut("M"),
                    MenuItem::action("Lock", |_, _| {})
                        .icon(IconName::Lock)
                        .shortcut("L")
                        .checked(true),
                    MenuItem::action("Connect", |_, _| {})
                        .icon(IconName::Connection)
                        .disabled(Some("Select two ports first")),
                    MenuItem::Separator,
                    MenuItem::action("Delete", |_, _| {})
                        .icon(IconName::Trash)
                        .shortcut("Del")
                        .danger(),
                ],
                cx,
            )
            .min_width(220.0)
        });
        studio.update(cx, |studio, _| sample_conversation(studio));
        let conversation = cx.new(|cx| ConversationView::new(studio.clone(), window, cx));
        let library = cx.new(|cx| LibraryView::new(studio.clone(), window, cx));
        let ghost = cx.new(|_| {
            DragGhost::new(LibraryDrag {
                block: agq_library::BlockRef::parse("built-in:Library::Storage::CachedStore")
                    .expect("a block reference"),
                name: "CachedStore".into(),
                kind: agq_language::ElementKind::PartDef,
                composite: true,
            })
        });
        let previews = studio.update(cx, |studio, _| {
            studio.library_index();
            let library = &studio.library;
            [
                (
                    "A composite: its boundary ports, its parts and their connections",
                    "built-in:Library::Storage::CachedStore",
                ),
                (
                    "An atomic block: its ports and values",
                    "built-in:Library::Messaging::Queue",
                ),
                ("A requirement", "built-in:Library::Services::FairUse"),
            ]
            .into_iter()
            .filter_map(|(caption, reference)| {
                let block = library
                    .index
                    .position(&agq_library::BlockRef::parse(reference)?)?;
                let preview = library.source.preview(&library.index, block, None)?;
                Some((caption, Rc::new(preview)))
            })
            .collect()
        });
        Gallery {
            studio,
            field,
            invalid,
            search,
            large,
            area,
            area_invalid,
            menu,
            conversation,
            library,
            ghost,
            previews,
            samples: samples(),
            switch_on: true,
            segment: 0,
        }
    }
}

/// The Surface samples: the URL shortener with a selected card and one the
/// Assistant just changed, and a comparison with added, changed and removed
/// cards.
fn samples() -> Vec<(&'static str, Rc<Sample>)> {
    let options = SceneOptions::default();
    // As the Architecture view shows them.
    let architecture = |mut input: agq_studio_scene::SceneInput| {
        use agq_studio_scene::EdgeKind;
        input.retain_edges(&[EdgeKind::Connection, EdgeKind::Interface, EdgeKind::Satisfy]);
        input
    };
    let mut samples = Vec::new();
    if let Ok(scene) = Scene::build(&architecture(fixtures::architecture()), &options, None) {
        samples.push((
            "Cards, ports, connections and a container; a selected card, and one the Assistant just changed",
            Rc::new(Sample::new(scene, "UrlShortenerService", Some("store"), Some("api"))),
        ));
    }
    // Inside a definition opened from a specialisation: one part inherited
    // from the general, one overriding what it inherits.
    let mut opened = architecture(fixtures::architecture());
    for node in &mut opened.nodes {
        let (origin, note) = match node.name.as_str() {
            "store" => (
                agq_studio_scene::NodeOrigin::Inherited,
                "· from LinkService",
            ),
            "api" => (agq_studio_scene::NodeOrigin::Override, "· override"),
            _ => continue,
        };
        node.origin = origin;
        node.detail = format!("{} {note}", node.detail);
    }
    if let Ok(scene) = Scene::build(&opened, &options, None) {
        samples.push((
            "Inside an opened specialisation: an inherited part (dashed, dimmed) and an override (accent edge)",
            Rc::new(Sample::new(scene, "UrlShortenerService", None, None)),
        ));
    }
    let (before, after) = fixtures::change_trees();
    let comparison = crate::history::Comparison::of_trees(
        "URL shortener",
        before,
        "edited",
        &after,
        fixtures::change().1,
        false,
    );
    if let Ok(scene) = Scene::comparison(
        &architecture(comparison.before.clone()),
        &architecture(comparison.after.clone()),
        &comparison.changed,
        &options,
        None,
    ) {
        samples.push((
            "What changed: added, changed and removed",
            Rc::new(Sample::new(scene, "UrlShortenerService", None, None)),
        ));
    }
    samples
}

/// A conversation that shows every Conversation component the Studio has.
fn sample_conversation(studio: &mut Studio) {
    use agq_assistant::{Entry, ToolResult};
    use serde_json::json;
    let reply = [
        "The URL shortener is in place:",
        "",
        "- `UrlShortener::api` takes **shorten** and **resolve** requests.",
        "- `UrlShortener::store` keeps the links, *one row per code*.",
        "",
        "| Part | Serves |",
        "|---|---|",
        "| api | shorten, resolve |",
        "| store | links |",
        "",
        "```rust",
        "fn shorten(url: &str) -> String {",
        "    hash(url)",
        "}",
        "```",
    ]
    .join("\n");
    let panel = &mut studio.conversation;
    panel.key_missing = None;
    panel.model_name = "gallery sample".into();
    panel.conversation.entries = vec![
        Entry::Operator {
            text: "Build a URL shortener: an HTTP API, a link store and click statistics.".into(),
        },
        Entry::reply(
            None,
            &[
                json!({ "type": "thinking", "thinking": "The API and the store talk through one port pair; statistics can wait for the Operator's answer." }),
                json!({ "type": "text", "text": "I'll add the API and the link store, joined by an interface." }),
                json!({ "type": "tool_use", "id": "g1", "name": "read_model", "input": {} }),
                json!({ "type": "tool_use", "id": "g2", "name": "apply_changes", "input": { "description": "Add the API and the link store", "operations": [] } }),
                json!({ "type": "tool_use", "id": "g3", "name": "apply_changes", "input": { "description": "Rename the gateway", "operations": [] } }),
            ],
        ),
        Entry::reply(None, &[json!({ "type": "text", "text": reply })]),
        Entry::Notice {
            text: "The provider did not answer in time. Retry, or pick another model.".into(),
        },
    ];
    panel.results = [
        ToolResult {
            tool_use_id: "g1".into(),
            content: "12 elements".into(),
            is_error: false,
            change: None,
        },
        ToolResult {
            tool_use_id: "g2".into(),
            content: "Changed.".into(),
            is_error: false,
            change: None,
        },
        ToolResult {
            tool_use_id: "g3".into(),
            content: "The gateway is locked; the Operator kept it.".into(),
            is_error: true,
            change: None,
        },
    ]
    .into_iter()
    .map(|result| (result.tool_use_id.clone(), result))
    .collect();
    panel.epoch += 1;
}

fn colour(rgba: Rgba) -> Hsla {
    GpuiRgba {
        r: rgba.0 as f32 / 255.0,
        g: rgba.1 as f32 / 255.0,
        b: rgba.2 as f32 / 255.0,
        a: rgba.3 as f32 / 255.0,
    }
    .into()
}

/// A titled section.
fn section(
    title: &'static str,
    note: Option<&'static str>,
    body: impl IntoElement,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(r(12.0))
        .pt(r(28.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(r(2.0))
                .child(
                    div()
                        .text_size(r(theme::text::LG))
                        .font_weight(theme::SEMIBOLD)
                        .child(title),
                )
                .when_some(note, |this, note| {
                    this.child(
                        div()
                            .text_size(r(theme::text::SM))
                            .text_color(theme.text_muted)
                            .child(note),
                    )
                }),
        )
        .child(body)
        .into_any_element()
}

/// A control with its label beside it.
fn labelled(control: impl IntoElement, label: &'static str, theme: &ui::Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(r(8.0))
        .text_size(r(theme::text::SM))
        .text_color(theme.text_secondary)
        .child(control)
        .child(label)
        .into_any_element()
}

/// A labelled sample in a row of samples.
fn sample(label: impl Into<SharedString>, body: impl IntoElement, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(r(6.0))
        .child(body)
        .child(
            div()
                .text_size(r(theme::text::XS))
                .text_color(theme.text_faint)
                .child(label.into()),
        )
        .into_any_element()
}

/// A row of samples that wraps.
fn row() -> gpui::Div {
    div().flex().flex_wrap().items_start().gap(r(20.0))
}

/// A framed area on the canvas colour, like the Studio's middle.
fn stage(cx: &App) -> gpui::Div {
    let theme = cx.theme();
    div()
        .p(r(16.0))
        .rounded(r(tokens::radius::CARD + 2.0))
        .bg(theme.canvas)
        .border_1()
        .border_color(theme.separator)
}

fn scale_row(name: &'static str, scale: Scale, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .gap(r(3.0))
        .child(
            div()
                .w(r(64.0))
                .text_size(r(theme::text::XS))
                .text_color(theme.text_muted)
                .child(name),
        )
        .children((1..=12).map(|step| {
            div()
                .w(r(26.0))
                .h(r(20.0))
                .rounded(r(3.0))
                .bg(colour(scale.step(step)))
                .border_1()
                .border_color(theme.separator.opacity(0.5))
        }))
        .into_any_element()
}

fn palette(name: &'static str, colours: Colours, cx: &App) -> AnyElement {
    let background = colour(colours.neutral.step(1));
    div()
        .flex()
        .flex_col()
        .gap(r(4.0))
        .child(
            div()
                .text_size(r(theme::text::SM))
                .font_weight(theme::MEDIUM)
                .child(name),
        )
        .child(scale_row("neutral", colours.neutral, cx))
        .child(scale_row("accent", colours.accent, cx))
        .child(scale_row("success", colours.success, cx))
        .child(scale_row("warning", colours.warning, cx))
        .child(scale_row("danger", colours.danger, cx))
        .child(scale_row("info", colours.info, cx))
        .child(
            div()
                .mt(r(4.0))
                .px(r(10.0))
                .py(r(6.0))
                .rounded(r(tokens::radius::CONTROL))
                .bg(background)
                .flex()
                .gap(r(12.0))
                .text_size(r(theme::text::SM))
                .child(
                    div()
                        .text_color(colour(colours.neutral.step(12)))
                        .child("Text 12"),
                )
                .child(
                    div()
                        .text_color(colour(colours.neutral.step(11)))
                        .child("secondary 11"),
                )
                .child(
                    div()
                        .text_color(colour(colours.accent.step(11)))
                        .child("accent 11"),
                )
                .child(
                    div()
                        .px(r(6.0))
                        .rounded(r(tokens::radius::TAG))
                        .bg(colour(colours.accent.step(9)))
                        .text_color(gpui::white())
                        .child("solid 9"),
                ),
        )
        .into_any_element()
}

impl Gallery {
    fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let (dark, contrast, reduced) = (theme.dark, theme.contrast, studio.reduced_motion);
        let set = |change: fn(&mut Studio)| {
            let studio = self.studio.clone();
            move |cx: &mut App| {
                studio.act(cx, |studio| {
                    change(studio);
                    studio.mark(Dirty::APPEARANCE);
                })
            }
        };
        let to_dark = set(|s| s.appearance.dark = true);
        let to_light = set(|s| s.appearance.dark = false);
        let contrast_toggle = set(|s| s.appearance.contrast = !s.appearance.contrast);
        let motion_toggle = set(|s| s.reduced_motion = !s.reduced_motion);
        div()
            .flex()
            .flex_wrap()
            .items_end()
            .justify_between()
            .gap(r(16.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(r(4.0))
                    .child(div().text_size(r(theme::text::XXL)).font_weight(theme::SEMIBOLD).child("Design system"))
                    .child(
                        div()
                            .text_size(r(theme::text::BASE))
                            .text_color(theme.text_muted)
                            .child("Every token and component of the Studio, in the theme chosen here. Switching it here previews it; it is not saved."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(16.0))
                    .child(
                        Segmented::new("gallery-theme", if dark { 0 } else { 1 })
                            .choice(Some(IconName::Moon), "Dark")
                            .choice(Some(IconName::Sun), "Light")
                            .on_choose(move |choice, _, cx| if choice == 0 { to_dark(cx) } else { to_light(cx) }),
                    )
                    .child(labelled(
                        Switch::new("gallery-contrast", contrast, "High contrast").on_toggle(move |_, _, cx| contrast_toggle(cx)),
                        "High contrast",
                        &theme,
                    ))
                    .child(labelled(
                        Switch::new("gallery-motion", reduced, "Reduced motion").on_toggle(move |_, _, cx| motion_toggle(cx)),
                        "Reduced motion",
                        &theme,
                    )),
            )
            .into_any_element()
    }

    fn tokens(&self, cx: &App) -> Vec<AnyElement> {
        let theme = cx.theme().clone();
        let palettes = row()
            .gap(r(28.0))
            .child(palette("Dark", Colours::generate(INPUTS, Mode::Dark), cx))
            .child(palette("Light", Colours::generate(INPUTS, Mode::Light), cx))
            .child(palette(
                "High contrast, dark",
                Colours::generate(HIGH_CONTRAST, Mode::Dark),
                cx,
            ))
            .child(palette(
                "High contrast, light",
                Colours::generate(HIGH_CONTRAST, Mode::Light),
                cx,
            ));
        let weights: [(FontWeight, &str); 3] = [
            (theme::REGULAR, "400"),
            (theme::MEDIUM, "500"),
            (theme::SEMIBOLD, "600"),
        ];
        let type_scale = div()
            .flex()
            .flex_col()
            .gap(r(6.0))
            .children(
                [
                    (tokens::text::XS, "XS"),
                    (tokens::text::SM, "SM"),
                    (tokens::text::BASE, "BASE"),
                    (tokens::text::PROSE, "PROSE"),
                    (tokens::text::LG, "LG"),
                    (tokens::text::XL, "XL"),
                    (tokens::text::XXL, "XXL"),
                ]
                .into_iter()
                .map(|(size, name)| {
                    div()
                        .flex()
                        .items_baseline()
                        .gap(r(24.0))
                        .child(
                            div()
                                .w(r(96.0))
                                .text_size(r(theme::text::XS))
                                .text_color(theme.text_faint)
                                .child(format!("{name} · {size} px")),
                        )
                        .children(weights.iter().map(|(weight, label)| {
                            div()
                                .text_size(r(size))
                                .font_weight(*weight)
                                .child(format!("Link store {label}"))
                        }))
                }),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(r(24.0))
                    .child(
                        div()
                            .w(r(96.0))
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_faint)
                            .child("Mono · names"),
                    )
                    .child(
                        div()
                            .font_family(theme::MONO)
                            .text_size(r(theme::text::BASE))
                            .child("part store : LinkStore [1];"),
                    ),
            );
        let spacing =
            row()
                .items_end()
                .gap(r(10.0))
                .children(tokens::space::GRID.iter().skip(1).map(|step| {
                    sample(
                        format!("{step}"),
                        div()
                            .w(r(*step))
                            .h(r(24.0))
                            .rounded(r(2.0))
                            .bg(theme.accent.solid.opacity(0.6)),
                        cx,
                    )
                }));
        let radii = row().children(
            [
                ("tag", tokens::radius::TAG),
                ("control", tokens::radius::CONTROL),
                ("card", tokens::radius::CARD),
                ("menu", tokens::radius::MENU),
                ("dialog", tokens::radius::DIALOG),
                ("full", tokens::radius::FULL),
            ]
            .into_iter()
            .map(|(name, radius)| {
                let shape = div()
                    .h(r(56.0))
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.border_strong);
                // Full is half the height: a pill.
                let shape = if radius.is_finite() {
                    shape.w(r(56.0)).rounded(r(radius))
                } else {
                    shape.w(r(96.0)).rounded_full()
                };
                let label = if radius.is_finite() {
                    format!("{name} {radius}")
                } else {
                    format!("{name}: half the height")
                };
                sample(label, shape, cx)
            }),
        );
        let heights = row().children(
            [
                ("row, compact", tokens::space::ROW_COMPACT),
                ("row", tokens::space::ROW),
                ("row, comfortable", tokens::space::ROW_COMFORTABLE),
                ("control, small", tokens::space::CONTROL_SM),
                ("control", tokens::space::CONTROL),
                ("control, large", tokens::space::CONTROL_LG),
                ("panel padding", tokens::space::PANEL_PADDING),
            ]
            .into_iter()
            .map(|(name, height)| {
                sample(
                    format!("{name} {height}"),
                    div()
                        .w(r(96.0))
                        .h(r(height))
                        .rounded(r(tokens::radius::CONTROL))
                        .border_1()
                        .border_color(theme.border_strong),
                    cx,
                )
            }),
        );
        let motion = div()
            .flex()
            .flex_col()
            .gap(r(4.0))
            .text_size(r(theme::text::SM))
            .text_color(theme.text_secondary)
            .children(
                [
                    (
                        "instant (keyboard-invoked, frequent)",
                        tokens::motion::INSTANT_MS,
                    ),
                    ("press", tokens::motion::PRESS_MS),
                    ("hover in", tokens::motion::HOVER_IN_MS),
                    ("hover out", tokens::motion::HOVER_OUT_MS),
                    ("panel", tokens::motion::PANEL_MS),
                    ("camera", tokens::motion::CAMERA_MS),
                    ("reduced-motion fade", tokens::motion::REDUCED_FADE_MS),
                    ("change highlight hold", tokens::motion::CHANGE_HOLD_MS),
                    ("change highlight fade", tokens::motion::CHANGE_FADE_MS),
                ]
                .into_iter()
                .map(|(name, ms)| div().child(format!("{name}: {ms} ms"))),
            )
            .children(
                [
                    ("decelerate, arriving", tokens::motion::DECELERATE),
                    ("accelerate, leaving", tokens::motion::ACCELERATE),
                    ("easy ease, moving", tokens::motion::EASY_EASE),
                ]
                .into_iter()
                .map(|(name, (x1, y1, x2, y2))| {
                    div().child(format!("{name}: cubic-bezier({x1}, {y1}, {x2}, {y2})"))
                }),
            )
            .child(div().child(format!(
                "spring: stiffness {}, damping {} (critically damped, about 200 ms)",
                tokens::motion::SPRING_STIFFNESS,
                tokens::motion::SPRING_DAMPING
            )));
        vec![
            section(
                "Colours",
                Some(
                    "Generated in OKLCH from a base hue, an accent hue and a contrast; steps 1–12 by Radix roles.",
                ),
                palettes,
                cx,
            ),
            section(
                "Type",
                Some(
                    "Inter (400, 500, 600) for the interface, JetBrains Mono for names and values.",
                ),
                type_scale,
                cx,
            ),
            section(
                "Spacing, sizes and radii",
                Some("A 4-point grid; every size scales with the UI scale."),
                div()
                    .flex()
                    .flex_col()
                    .gap(r(16.0))
                    .child(spacing)
                    .child(heights)
                    .child(radii),
                cx,
            ),
            section(
                "Motion",
                Some(
                    "Reduced motion turns moves into jumps; springs follow the pointer and choices.",
                ),
                motion,
                cx,
            ),
        ]
    }

    fn primitives(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme().clone();
        let variants = [
            ("primary", ui::Variant::Primary),
            ("secondary", ui::Variant::Secondary),
            ("ghost", ui::Variant::Ghost),
            ("subtle", ui::Variant::Subtle),
            ("danger", ui::Variant::Danger),
        ];
        let buttons = div()
            .flex()
            .flex_col()
            .gap(r(12.0))
            .children(variants.iter().map(|(name, variant)| {
                row()
                    .items_center()
                    .gap(r(12.0))
                    .child(
                        div()
                            .w(r(72.0))
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_faint)
                            .child(*name),
                    )
                    .child(
                        Button::new(SharedString::from(format!("b-{name}-s")), "Small")
                            .variant(*variant)
                            .small(),
                    )
                    .child(
                        Button::new(SharedString::from(format!("b-{name}-m")), "Medium")
                            .variant(*variant)
                            .icon(IconName::Plus),
                    )
                    .child(
                        Button::new(SharedString::from(format!("b-{name}-l")), "Large")
                            .variant(*variant)
                            .large(),
                    )
                    .child(
                        Button::new(SharedString::from(format!("b-{name}-k")), "Save")
                            .variant(*variant)
                            .shortcut("Ctrl+S"),
                    )
                    .child(
                        Button::new(SharedString::from(format!("b-{name}-t")), "More")
                            .variant(*variant)
                            .trailing(IconName::ChevronDown),
                    )
                    .child(
                        Button::new(SharedString::from(format!("b-{name}-sel")), "Selected")
                            .variant(*variant)
                            .selected(true),
                    )
                    .child(
                        Button::new(SharedString::from(format!("b-{name}-d")), "Disabled")
                            .variant(*variant)
                            .disabled(true),
                    )
                    .child(
                        Button::icon_only(
                            SharedString::from(format!("b-{name}-i")),
                            IconName::Settings,
                            "Settings",
                        )
                        .variant(*variant),
                    )
            }))
            .child(
                row()
                    .items_center()
                    .gap(r(12.0))
                    .child(
                        div()
                            .w(r(72.0))
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_faint)
                            .child("focus"),
                    )
                    .child(
                        div()
                            .px(r(10.0))
                            .h(r(28.0))
                            .flex()
                            .items_center()
                            .rounded(r(tokens::radius::CONTROL))
                            .bg(theme.raised)
                            .border_1()
                            .border_color(theme.border)
                            .shadow(ui::button::focus_ring(theme.accent.solid))
                            .text_size(r(theme::text::BASE))
                            .child("Focus ring, 2 px outside"),
                    ),
            );
        let fields = row()
            .child(sample(
                "text field",
                div().w(r(220.0)).child(TextField::new(&self.field)),
                cx,
            ))
            .child(sample(
                "with an icon",
                div()
                    .w(r(220.0))
                    .child(TextField::new(&self.search).leading(IconName::Search)),
                cx,
            ))
            .child(sample(
                "invalid",
                div()
                    .w(r(220.0))
                    .flex()
                    .flex_col()
                    .gap(r(4.0))
                    .child(TextField::new(&self.invalid).mono().invalid(true))
                    .child(ui::inline_message(
                        Tone::Danger,
                        "Not a multiplicity: use 1, 0..1 or 0..*",
                        cx,
                    )),
                cx,
            ))
            .child(sample(
                "text area",
                div().w(r(260.0)).child(TextArea::new(&self.area)),
                cx,
            ))
            .child(sample(
                "text area, invalid",
                div()
                    .w(r(260.0))
                    .child(TextArea::new(&self.area_invalid).invalid(true)),
                cx,
            ))
            .child(sample(
                "large, with a trailing key cap",
                div().w(r(320.0)).child(
                    TextField::new(&self.large)
                        .large()
                        .leading(IconName::Search)
                        .trailing(KeyCaps::new("Ctrl+K")),
                ),
                cx,
            ));
        let entity = cx.entity();
        let toggled = entity.clone();
        let chosen = entity.clone();
        let controls = row()
            .child(sample(
                "toggle",
                Switch::new("g-switch", self.switch_on, "Show estimated cost").on_toggle(
                    move |on, _, cx| {
                        toggled.update(cx, |this, cx| {
                            this.switch_on = on;
                            cx.notify();
                        })
                    },
                ),
                cx,
            ))
            .child(sample(
                "toggle, disabled",
                Switch::new("g-switch-off", false, "Not available").disabled(true),
                cx,
            ))
            .child(sample(
                "segmented control",
                Segmented::new("g-seg", self.segment)
                    .choice(Some(IconName::Architecture), "Architecture")
                    .choice(Some(IconName::Graph), "Graph")
                    .choice(Some(IconName::Requirements), "Requirements")
                    .on_choose(move |index, _, cx| {
                        chosen.update(cx, |this, cx| {
                            this.segment = index;
                            cx.notify();
                        })
                    }),
                cx,
            ))
            .child(sample(
                "menu (select, context menu)",
                div().w(r(240.0)).child(self.menu.clone()),
                cx,
            ));
        let tones = [
            ("neutral", Tone::Neutral),
            ("accent", Tone::Accent),
            ("success", Tone::Success),
            ("warning", Tone::Warning),
            ("danger", Tone::Danger),
            ("info", Tone::Info),
        ];
        let tokens_row = div()
            .flex()
            .flex_col()
            .gap(r(12.0))
            .child(
                row().items_center().gap(r(8.0)).children(
                    tones
                        .iter()
                        .map(|(name, tone)| Chip::new(*name).tone(*tone)),
                ),
            )
            .child(
                row()
                    .items_center()
                    .gap(r(8.0))
                    .child(
                        Chip::new("UrlShortener::api")
                            .mono()
                            .icon(IconName::Lock)
                            .tone(Tone::Warning),
                    )
                    .child(Chip::new("part").icon(IconName::Part).tone(Tone::Accent))
                    .children(
                        tones
                            .iter()
                            .map(|(name, tone)| Badge::new(name.len().to_string()).tone(*tone)),
                    )
                    .child(KeyCaps::new("Ctrl+Shift+K"))
                    .child(KeyCaps::new("F2")),
            )
            .child(
                row()
                    .items_start()
                    .gap(r(16.0))
                    .child(div().child(cx.new(|_| {
                        ui::tooltip::Tooltip::new("Fit the whole model").shortcut(Some("Shift+1"))
                    })))
                    .child(div().child(cx.new(|_| {
                        ui::tooltip::Tooltip::new("Lock")
                            .shortcut(Some("L"))
                            .detail("A locked element changes only after you confirm.")
                    }))),
            );
        let feedback = div()
            .flex()
            .flex_col()
            .gap(r(12.0))
            .max_w(r(720.0))
            .child(
                Banner::new(Tone::Warning, "No Anthropic key is set. Add one in Settings to work with the Assistant.")
                    .action(Button::new("g-banner", "Open Settings").small().icon(IconName::Key)),
            )
            .child(Banner::new(Tone::Info, "The Surface shows the later checkpoint. Close the comparison to edit."))
            .child(Banner::new(Tone::Danger, "The project could not be saved: the folder is read-only."))
            .children(tones.iter().skip(2).map(|(name, tone)| ui::inline_message(*tone, format!("An inline message, {name}"), cx)))
            .child(
                row()
                    .items_center()
                    .gap(r(20.0))
                    .child(sample("spinner", ui::spinner("g-spin", 16.0, theme.accent.solid), cx))
                    .child(sample("progress", div().w(r(160.0)).child(ui::primitives::progress(Some(0.62), cx)), cx))
                    .child(sample("progress, unknown", div().w(r(160.0)).child(ui::primitives::progress(None, cx)), cx))
                    .child(sample(
                        "skeleton",
                        div()
                            .w(r(200.0))
                            .flex()
                            .flex_col()
                            .gap(r(6.0))
                            .child(ui::primitives::skeleton("g-sk1", gpui::relative(1.0), cx))
                            .child(ui::primitives::skeleton("g-sk2", gpui::relative(0.7), cx)),
                        cx,
                    )),
            )
            .child(
                stage(cx).flex().justify_center().child(
                    EmptyState::new(IconName::Conversation, "Ask the Assistant", "Tell it what to build or change. Every change shows on the Surface and can be undone.")
                        .hint("Ctrl+L", "write here")
                        .action(Button::new("g-empty", "Open a project…").icon(IconName::FolderOpen)),
                ),
            );
        let dialog = stage(cx).relative().h(r(250.0)).overflow_hidden().child(
            ui::Dialog::new("g-dialog", "Locked")
                .top(28.0)
                .width(420.0)
                .description("api is locked. Change it anyway?")
                .child(
                    Chip::new("UrlShortener::api")
                        .mono()
                        .icon(IconName::Lock)
                        .tone(Tone::Warning),
                )
                .footer(Button::new("g-cancel", "Cancel"))
                .footer(Button::new("g-ok", "Change it").primary().shortcut("Enter")),
        );
        vec![
            section(
                "Buttons",
                Some("Hover and press show under the pointer; keyboard focus draws the ring."),
                buttons,
                cx,
            ),
            section("Fields", None, fields, cx),
            section("Choices and menus", None, controls, cx),
            section("Chips, badges, key caps and tooltips", None, tokens_row, cx),
            section("Messages, progress and empty states", None, feedback, cx),
            section(
                "Dialog",
                Some("Over a dimmed window, with the keyboard in its first field."),
                dialog,
                cx,
            ),
        ]
    }

    fn icons(&self, cx: &App) -> AnyElement {
        let theme = cx.theme().clone();
        let body = row().gap(r(6.0)).children(IconName::ALL.iter().map(|name| {
            div()
                .w(r(104.0))
                .h(r(64.0))
                .rounded(r(tokens::radius::CONTROL))
                .border_1()
                .border_color(theme.separator)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(r(6.0))
                .child(ui::icon(*name).size(18.0).color(theme.text_secondary))
                .child(
                    div()
                        .text_size(r(theme::text::XS))
                        .text_color(theme.text_faint)
                        .child(format!("{name:?}")),
                )
        }));
        section(
            "Icons",
            Some("Lucide, 1.75-point strokes, in the text colour of where they sit."),
            body,
            cx,
        )
    }

    fn surface(&self, cx: &App) -> AnyElement {
        let theme = cx.theme().clone();
        let body = div()
            .flex()
            .flex_col()
            .gap(r(16.0))
            .children(self.samples.iter().map(|(title, sample)| {
                let sample = sample.clone();
                let caption = theme.text_faint;
                let theme = theme.clone();
                div()
                    .flex()
                    .flex_col()
                    .gap(r(6.0))
                    .child(
                        div()
                            .h(r(560.0))
                            .w_full()
                            .rounded(r(tokens::radius::CARD + 2.0))
                            .overflow_hidden()
                            .border_1()
                            .border_color(theme.separator)
                            .child(
                                canvas(
                                    move |bounds, window, _| {
                                        let mut camera = Camera2D {
                                            viewport: Size::new(
                                                f32::from(bounds.size.width),
                                                f32::from(bounds.size.height),
                                            ),
                                            ..Camera2D::default()
                                        };
                                        camera.fit(sample.framed, 24.0);
                                        let mut lod = LodController::default();
                                        let level = lod.update(camera.zoom);
                                        let mut selection = crate::selection::Selection::default();
                                        if let Some(target) = sample.selected.clone() {
                                            selection.select(target, false);
                                        }
                                        let highlights: BTreeMap<_, _> = sample
                                            .changed
                                            .map(|id| {
                                                (id, (0.0, agq_system_state::Actor::Assistant))
                                            })
                                            .into_iter()
                                            .collect();
                                        paint::Frame {
                                            scene: sample.scene.clone(),
                                            spatial: sample.spatial.clone(),
                                            lookup: sample.lookup.clone(),
                                            camera,
                                            lod: level,
                                            selection,
                                            hovered: None,
                                            inspected: None,
                                            highlights,
                                            gesture: None,
                                            compatible: Default::default(),
                                            // A still frame: highlights hold at full strength.
                                            reduced_motion: true,
                                            theme: theme.clone(),
                                            ui_scale: f32::from(window.rem_size()) / 16.0,
                                        }
                                    },
                                    |bounds, frame, window, cx| {
                                        paint::paint(&frame, bounds, window, cx);
                                    },
                                )
                                .size_full(),
                            ),
                    )
                    .child(
                        div()
                            .text_size(r(theme::text::XS))
                            .text_color(caption)
                            .child(*title),
                    )
            }));
        section(
            "Surface",
            Some(
                "The Surface's own drawing: category marks, ports by direction (hollow until connected), relationship labels, the selection ring, change marks and an Assistant highlight.",
            ),
            body,
            cx,
        )
    }

    fn conversation(&self, cx: &App) -> AnyElement {
        let theme = cx.theme();
        section(
            "Conversation",
            Some(
                "The Conversation's own view on a sample: messages, thinking, tool cards by state, a notice with Retry, Markdown with a table and code.",
            ),
            div()
                .h(r(720.0))
                .w(r(460.0))
                .rounded(r(tokens::radius::CARD + 2.0))
                .overflow_hidden()
                .border_1()
                .border_color(theme.separator)
                .bg(theme.chrome)
                .child(self.conversation.clone()),
            cx,
        )
    }

    fn library(&self, cx: &App) -> AnyElement {
        let theme = cx.theme().clone();
        let caption = |text: &'static str| {
            div()
                .text_size(r(theme::text::XS))
                .text_color(theme.text_faint)
                .child(text)
        };
        let panel = div()
            .w(r(280.0))
            .flex()
            .flex_col()
            .gap(r(6.0))
            .child(
                div()
                    .w(r(280.0))
                    .h(r(660.0))
                    .flex_none()
                    .rounded(r(tokens::radius::CARD + 2.0))
                    .overflow_hidden()
                    .border_1()
                    .border_color(theme.separator)
                    .bg(theme.chrome)
                    .child(self.library.clone()),
            )
            .child(caption(
                "The Library panel: search, scopes, kinds, results and the selected block",
            ));
        let previews =
            div()
                .flex()
                .flex_col()
                .gap(r(12.0))
                .children(
                    self.previews
                        .iter()
                        .enumerate()
                        .map(|(index, (text, preview))| {
                            div()
                                .w(r(260.0))
                                .flex()
                                .flex_col()
                                .gap(r(6.0))
                                .child(
                                    BlockPreview::new(
                                        SharedString::from(format!("g-preview-{index}")),
                                        preview.clone(),
                                    )
                                    .height(block_preview::height(preview)),
                                )
                                .child(caption(text))
                                .into_any_element()
                        }),
                );
        let crumbs = stage(cx)
            .relative()
            .h(r(64.0))
            .w(r(360.0))
            .child(crate::surface::breadcrumb(
                vec!["demo".into(), "sessions : CachedStore".into()],
                self.studio.clone(),
                cx,
            ));
        let values = stage(cx)
            .w(r(320.0))
            .child(crate::panels::reuse_sample(&self.studio, cx));
        let others = div()
            .flex()
            .flex_col()
            .gap(r(16.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(r(6.0))
                    .child(stage(cx).child(div().flex().child(self.ghost.clone())))
                    .child(caption("What follows the pointer while a block is dragged")),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(r(6.0))
                    .child(crumbs)
                    .child(caption("Where the Surface is: Back, the whole model, each definition opened")),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(r(6.0))
                    .child(values)
                    .child(caption("Inherited values in the Inspector: overridden here, and from the definition")),
            );
        section(
            "Library",
            Some(
                "Building blocks (C-49): the real Library panel on the built-in blocks, structural previews, the drag ghost, the breadcrumb and inherited values.",
            ),
            row()
                .gap(r(24.0))
                .items_start()
                .child(panel)
                .child(previews)
                .child(others),
            cx,
        )
    }

    fn coverage(&self, cx: &App) -> AnyElement {
        let theme = cx.theme().clone();
        let mut groups: Vec<(&str, Vec<&tokens::Component>)> = Vec::new();
        for component in tokens::COMPONENTS {
            match groups
                .iter_mut()
                .find(|(group, _)| *group == component.group)
            {
                Some((_, list)) => list.push(component),
                None => groups.push((component.group, vec![component])),
            }
        }
        let shared = div()
            .text_size(r(theme::text::SM))
            .text_color(theme.text_secondary)
            .child(format!(
                "Every component shows these the same way: {}.",
                tokens::STATES.join(", ")
            ));
        let work = div()
            .text_size(r(theme::text::SM))
            .text_color(theme.text_secondary)
            .child(format!(
                "Work, on the Surface, in the Panels and in the Conversation: {}.",
                tokens::WORK_STATES.join(", ")
            ));
        let body = row()
            .gap(r(28.0))
            .children(groups.into_iter().map(|(group, list)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(r(3.0))
                    .min_w(r(180.0))
                    .child(
                        div()
                            .text_size(r(theme::text::SM))
                            .font_weight(theme::MEDIUM)
                            .pb(r(4.0))
                            .child(group),
                    )
                    .children(list.into_iter().map(|component| {
                        let detail: Vec<&str> = component
                            .variants
                            .iter()
                            .chain(component.states)
                            .copied()
                            .collect();
                        div()
                            .flex()
                            .gap(r(6.0))
                            .text_size(r(theme::text::XS))
                            .child(div().text_color(theme.text_secondary).child(component.name))
                            .when(!detail.is_empty(), |this| {
                                this.child(
                                    div().text_color(theme.text_faint).child(detail.join(", ")),
                                )
                            })
                    }))
            }));
        let body = div()
            .flex()
            .flex_col()
            .gap(r(12.0))
            .child(shared)
            .child(work)
            .child(body);
        section(
            "Component list",
            Some(
                "§3.2's components and their variants and states (the tokens module). Plan, lock and note cards, queued messages, toasts, checkboxes and radios arrive with the work that first uses them.",
            ),
            body,
            cx,
        )
    }
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let header = self.header(cx);
        let tokens = self.tokens(cx);
        let primitives = self.primitives(cx);
        let icons = self.icons(cx);
        let surface = self.surface(cx);
        let conversation = self.conversation(cx);
        let library = self.library(cx);
        let coverage = self.coverage(cx);
        div()
            .id("gallery")
            .size_full()
            .overflow_y_scroll()
            .bg(theme.chrome)
            .child(
                div()
                    .max_w(r(1240.0))
                    .mx_auto()
                    .px(r(40.0))
                    .py(r(32.0))
                    .flex()
                    .flex_col()
                    .child(header)
                    .children(tokens)
                    .children(primitives)
                    .child(icons)
                    .child(surface)
                    .child(conversation)
                    .child(library)
                    .child(coverage)
                    .child(div().h(r(48.0))),
            )
    }
}
