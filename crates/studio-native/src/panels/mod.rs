//! The docked Panels (§3.2 Panels): the Outline on the left, and on the right
//! a column with the Inspector, the Requirements, the History and the
//! Problems under one tab bar.
mod history;
mod inspector;
mod outline;
mod problems;
mod requirements;

pub use outline::OutlineView;

use crate::{
    studio::{Dirty, Panel, Studio, StudioEvent},
    ui::{self, ActiveTheme, IconName, r, theme},
    workspace::StudioExt,
};
use agq_language::ElementId;
use agq_studio_scene::SceneTarget;
use gpui::{
    AnimationExt, App, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div,
    prelude::FluentBuilder, relative,
};

/// The right-hand column of Panels.
pub struct InspectorColumn {
    studio: Entity<Studio>,
    inspector: inspector::Fields,
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
            studio,
            _subscription: subscription,
        }
    }
}

const TABS: [(Panel, &str, IconName); 4] = [
    (Panel::Inspector, "Inspector", IconName::Sliders),
    (Panel::Requirements, "Requirements", IconName::Requirements),
    (Panel::History, "History", IconName::Clock),
    (Panel::Problems, "Problems", IconName::Warning),
];

impl Render for InspectorColumn {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let panel = studio.panel;
        let problems: usize = studio.problems.values().map(Vec::len).sum();
        let index = TABS.iter().position(|(p, _, _)| *p == panel).unwrap_or(0);
        let body = match panel {
            Panel::Inspector => self.inspector.render(window, cx).into_any_element(),
            Panel::Requirements => requirements::render(&self.studio, cx).into_any_element(),
            Panel::History => history::render(&self.studio, cx).into_any_element(),
            Panel::Problems => problems::render(&self.studio, cx).into_any_element(),
        };
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
                    .children(TABS.iter().enumerate().map(|(i, (tab, label, _))| {
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
                            .child(*label)
                            .child(ui::target::target(*label))
                            .when(tab == Panel::Problems && problems > 0, |this| {
                                this.child(
                                    ui::Badge::new(problems.to_string()).tone(ui::Tone::Warning),
                                )
                            })
                    }))
                    // The sliding mark under the chosen tab.
                    .child(
                        div()
                            .absolute()
                            .bottom_0()
                            .h(gpui::px(2.0))
                            .w(relative(0.25))
                            .px(r(10.0))
                            .child(div().size_full().rounded_full().bg(theme.accent.solid))
                            .with_spring(
                                "panel-tab-mark",
                                ui::primitives::spring().to(index as f32),
                                |this, at: f32| this.left(relative(at * 0.25)),
                            ),
                    ),
            )
            .child(div().flex_1().min_w_0().min_h_0().child(body))
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
