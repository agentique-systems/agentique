//! The docked Panels (§3.2 Panels): on the left a column with the Outline,
//! the Library (C-49) and the Scenarios (C-50) under one tab bar, and on the
//! right a column with the Inspector, the Run (C-50), the Requirements, the
//! History, the Problems and the Objectives (C-53).
pub mod block_preview;
mod evidence;
mod history;
mod inspector;
pub mod library;
mod objectives;
mod outline;
mod problems;
mod requirements;
mod responsibility;
mod reuse;
pub mod run;
pub mod scenarios;

pub use inspector::parse_value;
pub use outline::OutlineView;
pub use reuse::sample as reuse_sample;

use crate::{
    studio::{Dirty, LeftTab, Panel, Studio, StudioEvent},
    ui::{self, ActiveTheme, IconName, r, theme},
    workspace::StudioExt,
};
use agq_language::ElementId;
use agq_studio_scene::SceneTarget;
use gpui::{
    AnimationExt, App, AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
    div, prelude::FluentBuilder, relative,
};

/// The left-hand column: the Outline and the Library.
pub struct LeftColumn {
    studio: Entity<Studio>,
    outline: Entity<OutlineView>,
    library: Entity<library::LibraryView>,
    scenarios: Entity<scenarios::ScenariosView>,
    _subscription: Subscription,
}

impl LeftColumn {
    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe(&studio, |_, _, event: &StudioEvent, cx| {
            if event.0.intersects(Dirty::LAYOUT | Dirty::APPEARANCE) {
                cx.notify();
            }
        });
        LeftColumn {
            outline: cx.new(|cx| OutlineView::new(studio.clone(), window, cx)),
            library: cx.new(|cx| library::LibraryView::new(studio.clone(), window, cx)),
            scenarios: cx.new(|cx| scenarios::ScenariosView::new(studio.clone(), cx)),
            studio,
            _subscription: subscription,
        }
    }

    /// The Library panel (the scripted journeys read its search).
    #[cfg(feature = "automation")]
    pub fn library(&self) -> &Entity<library::LibraryView> {
        &self.library
    }
}

const LEFT_TABS: [(LeftTab, &str, IconName); 3] = [
    (LeftTab::Outline, "Outline", IconName::Outline),
    (LeftTab::Library, "Library", IconName::Library),
    (LeftTab::Scenarios, "Scenarios", IconName::Scenario),
];
const LEFT_SHARE: f32 = 1.0 / LEFT_TABS.len() as f32;

impl Render for LeftColumn {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let tab = self.studio.read(cx).left;
        let index = LEFT_TABS
            .iter()
            .position(|(t, _, _)| *t == tab)
            .unwrap_or(0);
        let body: gpui::AnyView = match tab {
            LeftTab::Outline => self.outline.clone().into(),
            LeftTab::Library => self.library.clone().into(),
            LeftTab::Scenarios => self.scenarios.clone().into(),
        };
        crate::ui::target::regioned(
            "outline",
            div()
                .id("left-column")
                .size_full()
                .flex()
                .flex_col()
                .bg(theme.chrome)
                .role(gpui::Role::Complementary)
                .aria_label("Outline, Library and Scenarios")
                .child(
                    div()
                        .id("left-tabs")
                        .flex_none()
                        .h(r(36.0))
                        .px(r(6.0))
                        .relative()
                        .flex()
                        .items_stretch()
                        .border_b_1()
                        .border_color(theme.separator)
                        .role(gpui::Role::TabList)
                        .children(LEFT_TABS.iter().enumerate().map(
                            |(i, (choice, label, glyph))| {
                                let chosen = *choice == tab;
                                let studio = self.studio.clone();
                                let choice = *choice;
                                div()
                                    .id(("left-tab", i))
                                    .role(gpui::Role::Tab)
                                    .aria_selected(chosen)
                                    .aria_label(*label)
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap(r(4.0))
                                    .text_size(r(theme::text::SM))
                                    .font_weight(theme::MEDIUM)
                                    .text_color(if chosen { theme.text } else { theme.text_muted })
                                    .cursor_pointer()
                                    .hover(|style| style.text_color(theme.text))
                                    .on_click(move |_: &ClickEvent, _, cx| {
                                        studio.act(cx, |studio| {
                                            studio.left = choice;
                                            if choice == LeftTab::Library {
                                                studio.library.focus_search = true;
                                            }
                                            studio.mark(Dirty::LAYOUT);
                                        })
                                    })
                                    .relative()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .px(r(2.0))
                                    .child(div().flex_none().child(
                                        ui::icon(*glyph).size(13.0).color(if chosen {
                                            theme.text_secondary
                                        } else {
                                            theme.text_faint
                                        }),
                                    ))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(*label),
                                    )
                                    .child(ui::target::control(
                                        ui::target::Control::new("tab", *label).selected(chosen),
                                    ))
                            },
                        ))
                        .child(
                            div()
                                .absolute()
                                .bottom_0()
                                .h(gpui::px(2.0))
                                .w(relative(LEFT_SHARE))
                                .px(r(10.0))
                                .child(div().size_full().rounded_full().bg(theme.accent.solid))
                                .with_spring(
                                    "left-tab-mark",
                                    ui::primitives::spring().to(index as f32),
                                    |this, at: f32| this.left(relative(at * LEFT_SHARE)),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .child(body.cached(gpui::StyleRefinement::default().size_full())),
                ),
        )
    }
}

/// The right-hand column of Panels.
pub struct InspectorColumn {
    studio: Entity<Studio>,
    inspector: inspector::Fields,
    objectives: objectives::Fields,
    _subscription: Subscription,
}

impl InspectorColumn {
    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe(&studio, |_, _, event: &StudioEvent, cx| {
            if event.0.intersects(
                Dirty::MODEL
                    | Dirty::SELECTION
                    | Dirty::APPEARANCE
                    | Dirty::LAYOUT
                    | Dirty::STATUS
                    | Dirty::OVERLAY,
            ) {
                cx.notify();
            }
        });
        InspectorColumn {
            inspector: inspector::Fields::new(studio.clone(), window, cx),
            objectives: objectives::Fields::new(window, cx),
            studio,
            _subscription: subscription,
        }
    }
}

impl InspectorColumn {
    pub(crate) fn objective_fields(&self) -> &objectives::Fields {
        &self.objectives
    }

    pub(crate) fn objective_fields_mut(&mut self) -> &mut objectives::Fields {
        &mut self.objectives
    }
}

const TABS: [(Panel, &str, IconName); 6] = [
    (Panel::Inspector, "Inspector", IconName::Sliders),
    (Panel::Run, "Run", IconName::Play),
    (Panel::Requirements, "Requirements", IconName::Requirements),
    (Panel::History, "History", IconName::Clock),
    (Panel::Problems, "Problems", IconName::Warning),
    (Panel::Objectives, "Objectives", IconName::Agent),
];
const TAB_SHARE: f32 = 1.0 / TABS.len() as f32;
/// Below this width per tab (in unscaled pixels) a tab shows its icon, with
/// its name as the tooltip and accessible name.
const TAB_LABEL_MIN: f32 = 76.0;

impl Render for InspectorColumn {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let panel = studio.panel;
        let problems: usize = studio.problems.values().map(Vec::len).sum::<usize>()
            + studio.drift().values().map(Vec::len).sum::<usize>();
        let index = TABS.iter().position(|(p, _, _)| *p == panel).unwrap_or(0);
        let icons_only = studio.widths.inspector / (TABS.len() as f32) < TAB_LABEL_MIN;
        let running = studio.runs.running();
        let body = match panel {
            Panel::Inspector => self.inspector.render(window, cx).into_any_element(),
            Panel::Run => run::render(&self.studio, cx).into_any_element(),
            Panel::Requirements => requirements::render(&self.studio, cx).into_any_element(),
            Panel::History => history::render(&self.studio, cx).into_any_element(),
            Panel::Problems => problems::render(&self.studio, cx).into_any_element(),
            Panel::Objectives => {
                let studio = self.studio.clone();
                self.objectives.render(&studio, window, cx)
            }
        };
        crate::ui::target::regioned(
            "inspector",
            div()
                .id("inspector-column")
                .size_full()
                .flex()
                .flex_col()
                .bg(theme.chrome)
                .role(gpui::Role::Complementary)
                .aria_label("Panels")
                .child(
                    div()
                        .id("panel-tabs")
                        .flex_none()
                        .h(r(36.0))
                        .px(r(6.0))
                        .relative()
                        .flex()
                        .items_stretch()
                        .border_b_1()
                        .border_color(theme.separator)
                        .role(gpui::Role::TabList)
                        .children(TABS.iter().enumerate().map(|(i, (tab, label, glyph))| {
                            let chosen = *tab == panel;
                            let studio = self.studio.clone();
                            let tab = *tab;
                            div()
                                .id(("panel-tab", i))
                                .role(gpui::Role::Tab)
                                .aria_selected(chosen)
                                .aria_label(*label)
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap(r(5.0))
                                .text_size(r(theme::text::SM))
                                .font_weight(theme::MEDIUM)
                                .text_color(if chosen { theme.text } else { theme.text_muted })
                                .cursor_pointer()
                                .hover(|style| style.text_color(theme.text))
                                .on_click(move |_: &ClickEvent, _, cx| {
                                    studio.act(cx, |studio| {
                                        studio.panel = tab;
                                        studio.mark(Dirty::LAYOUT);
                                    })
                                })
                                .relative()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .when(icons_only, |this| {
                                    this.child(ui::icon(*glyph).size(14.0).color(if chosen {
                                        theme.text_secondary
                                    } else {
                                        theme.text_faint
                                    }))
                                    .tooltip(
                                        move |window, cx| {
                                            ui::tooltip::text(*label, None)(window, cx)
                                        },
                                    )
                                })
                                .when(!icons_only, |this| {
                                    this.child(
                                        div()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(*label),
                                    )
                                })
                                .child(ui::target::control(
                                    ui::target::Control::new("tab", *label).selected(chosen),
                                ))
                                .when(tab == Panel::Run && running, |this| {
                                    this.child(ui::primitives::spinner(
                                        "run-tab-spinner",
                                        10.0,
                                        theme.info.solid,
                                    ))
                                })
                                .when(tab == Panel::Problems && problems > 0, |this| {
                                    this.child(
                                        ui::Badge::new(problems.to_string())
                                            .tone(ui::Tone::Warning),
                                    )
                                })
                        }))
                        // The sliding mark under the chosen tab.
                        .child(
                            div()
                                .absolute()
                                .bottom_0()
                                .h(gpui::px(2.0))
                                .w(relative(TAB_SHARE))
                                .px(r(10.0))
                                .child(div().size_full().rounded_full().bg(theme.accent.solid))
                                .with_spring(
                                    "panel-tab-mark",
                                    ui::primitives::spring().to(index as f32),
                                    |this, at: f32| this.left(relative(at * TAB_SHARE)),
                                ),
                        ),
                )
                .child(div().flex_1().min_w_0().min_h_0().child(body)),
        )
    }
}

/// Selects the element's card, or the nearest card that owns it, and moves
/// the camera there; an element without a card is shown in the Inspector.
pub(crate) fn show(studio: &Entity<Studio>, id: ElementId, cx: &mut App) {
    studio.act(cx, |studio| {
        let Some(tree) = studio.project.as_ref().map(|p| p.state().tree()) else {
            studio.show_element(id);
            return;
        };
        let mut current = Some(id);
        while let Some(element) = current {
            let target = SceneTarget::Node(element);
            if studio.scene.target_bounds(&target).is_some() {
                studio.select(target.clone(), false);
                if element != id {
                    studio.inspected = Some((studio.selection.primary.clone(), id));
                }
                studio.frame_target(&target);
                return;
            }
            current = tree.get(element).and_then(|e| e.owner());
        }
    });
}

/// A heading over a group of rows, with an optional count.
pub(crate) fn group(title: &'static str, count: Option<usize>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .gap(r(6.0))
        .pt(r(14.0))
        .pb(r(6.0))
        .text_size(r(theme::text::XS))
        .font_weight(theme::SEMIBOLD)
        .text_color(theme.text_muted)
        .child(title.to_uppercase())
        .when_some(count, |this, count| {
            this.child(div().text_color(theme.text_faint).child(count.to_string()))
        })
}

/// A quiet line of text for an empty panel.
pub(crate) fn note(text: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .text_size(r(theme::text::SM))
        .line_height(r(18.0))
        .text_color(theme.text_muted)
        .child(text.into())
}
