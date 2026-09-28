//! The Outline: the cards on the Surface as a tree, parents before children
//! in model order, with their kind, lock and problems; a filter narrows it.
//! A click selects the card and moves the camera to it; a container's arrow
//! collapses or expands it on the Surface.
use crate::{
    palette::kind_icon,
    studio::{Dirty, Studio, StudioEvent},
    ui::{self, ActiveTheme, IconName, TextField, icon, r, theme},
    workspace::StudioExt,
};
use agq_studio_scene::{ElementId, LockMark, NodeCategory, SceneTarget};
use gpui::{
    AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div,
    prelude::FluentBuilder, uniform_list,
};
use gpui_base::input::{InputEvent, InputState};
use std::rc::Rc;

struct Row {
    id: ElementId,
    depth: usize,
    name: SharedString,
    keyword: &'static str,
    category: NodeCategory,
    kind: Option<agq_language::ElementKind>,
    container: bool,
    collapsed: bool,
    lock: LockMark,
    problems: usize,
}

pub struct OutlineView {
    studio: Entity<Studio>,
    filter: Entity<InputState>,
    rows: Rc<Vec<Row>>,
    /// The scene generation and filter the rows were built for.
    built: Option<(u64, String)>,
    _subscriptions: Vec<Subscription>,
}

impl OutlineView {
    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter elements"));
        let subscriptions = vec![
            cx.subscribe(&studio, |_, _, event: &StudioEvent, cx| {
                if event
                    .0
                    .intersects(Dirty::MODEL | Dirty::SELECTION | Dirty::APPEARANCE)
                {
                    cx.notify();
                }
            }),
            cx.subscribe(&filter, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.built = None;
                    cx.notify();
                }
            }),
        ];
        OutlineView {
            studio,
            filter,
            rows: Rc::new(Vec::new()),
            built: None,
            _subscriptions: subscriptions,
        }
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let query = self.filter.read(cx).value().trim().to_string();
        let studio = self.studio.read(cx);
        let key = (studio.scene.generation, query.clone());
        if self.built.as_ref() == Some(&key) {
            return;
        }
        let tree = studio.project.as_ref().map(|p| p.state().tree());
        // Parents before children, siblings in model order.
        let position: std::collections::BTreeMap<_, _> = studio
            .input
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id, i))
            .collect();
        let mut order: Vec<usize> = (0..studio.scene.nodes.len()).collect();
        order.sort_by_key(|i| {
            position
                .get(&studio.scene.nodes[*i].id())
                .copied()
                .unwrap_or(usize::MAX)
        });
        let rows = order
            .into_iter()
            .map(|index| &studio.scene.nodes[index])
            .filter(|node| {
                query.is_empty()
                    || crate::commands::fuzzy_score(&query, &node.semantic.name).is_some()
            })
            .map(|node| Row {
                id: node.id(),
                depth: if query.is_empty() {
                    node.depth.min(8)
                } else {
                    0
                },
                name: node.semantic.name.clone().into(),
                keyword: node.semantic.keyword,
                category: node.category,
                kind: tree.and_then(|t| t.get(node.id())).map(|e| e.kind),
                container: node.is_container,
                collapsed: node.collapsed,
                lock: node.semantic.lock,
                problems: node.semantic.problems,
            })
            .collect();
        self.rows = Rc::new(rows);
        self.built = Some(key);
    }
}

impl Render for OutlineView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.rebuild(cx);
        let theme = cx.theme().clone();
        let rows = self.rows.clone();
        let count = rows.len();
        let studio = self.studio.clone();
        let selection = self.studio.read(cx).selection.clone();
        let empty = self.studio.read(cx).scene.nodes.is_empty();
        div()
            .id("outline")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.chrome)
            .role(gpui::Role::Navigation)
            .aria_label("Outline")
            .child(
                div()
                    .flex_none()
                    .h(r(36.0))
                    .px(r(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(theme.separator)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(r(6.0))
                            .text_size(r(theme::text::SM))
                            .font_weight(theme::MEDIUM)
                            .child(icon(IconName::Outline).size(13.0).color(theme.text_faint))
                            .child("Outline"),
                    )
                    .child(
                        div()
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_faint)
                            .child(format!("{count}")),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .px(r(8.0))
                    .pt(r(8.0))
                    .pb(r(4.0))
                    .child(TextField::new(&self.filter).leading(IconName::Search)),
            )
            .child(if empty {
                div()
                    .p(r(16.0))
                    .child(crate::panels::note(
                        "The model is empty. Cards appear here as you create them.",
                        cx,
                    ))
                    .into_any_element()
            } else {
                uniform_list("outline-rows", count, move |range, _, cx| {
                    let theme = cx.theme().clone();
                    range
                        .map(|index| {
                            let row = &rows[index];
                            let selected = selection.contains(row.id);
                            let id = row.id;
                            let container = row.container;
                            let studio = studio.clone();
                            let toggle_studio = studio.clone();
                            let glyph = row.kind.map(kind_icon).unwrap_or(IconName::Part);
                            let colour = match row.category {
                                NodeCategory::Requirement => theme.warning.text,
                                NodeCategory::Definition => theme.info.text,
                                NodeCategory::Package => theme.text_muted,
                                _ => theme.accent.text,
                            };
                            div()
                                .id(("outline-row", index))
                                .role(gpui::Role::TreeItem)
                                .aria_selected(selected)
                                .aria_label(SharedString::from(format!(
                                    "{} {}",
                                    row.keyword, row.name
                                )))
                                .h(r(26.0))
                                .mx(r(6.0))
                                .pl(r(6.0 + 14.0 * row.depth as f32))
                                .pr(r(8.0))
                                .flex()
                                .items_center()
                                .gap(r(6.0))
                                .rounded(r(crate::tokens::radius::CONTROL))
                                .text_size(r(theme::text::SM))
                                .cursor_pointer()
                                .when(selected, |this| {
                                    this.bg(theme.accent.soft).text_color(theme.text)
                                })
                                .when(!selected, |this| {
                                    this.text_color(theme.text_secondary)
                                        .hover(|style| style.bg(theme.hover))
                                })
                                .on_click(move |_: &ClickEvent, _, cx| {
                                    studio.act(cx, |studio| {
                                        let target = if container {
                                            SceneTarget::Container(id)
                                        } else {
                                            SceneTarget::Node(id)
                                        };
                                        studio.select(target.clone(), false);
                                        studio.panel = crate::studio::Panel::Inspector;
                                        studio.frame_target(&target);
                                        studio
                                            .mark(Dirty::SELECTION | Dirty::CAMERA | Dirty::LAYOUT);
                                    })
                                })
                                .child(
                                    div()
                                        .id(("outline-toggle", index))
                                        .w(r(12.0))
                                        .flex_none()
                                        .when(container, |this| {
                                            this.cursor_pointer()
                                                .child(
                                                    icon(if row.collapsed {
                                                        IconName::ChevronRight
                                                    } else {
                                                        IconName::ChevronDown
                                                    })
                                                    .size(12.0)
                                                    .color(theme.text_faint),
                                                )
                                                .on_click(move |_: &ClickEvent, _, cx| {
                                                    cx.stop_propagation();
                                                    toggle_studio.act(cx, |studio| {
                                                        if !studio.collapsed.remove(&id) {
                                                            studio.collapsed.insert(id);
                                                        }
                                                        studio.rebuild();
                                                    })
                                                })
                                        }),
                                )
                                .child(icon(glyph).size(13.0).color(colour.opacity(0.85)))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(row.name.clone()),
                                )
                                .when(row.lock.locked(), |this| {
                                    this.child(icon(IconName::Lock).size(12.0).color(
                                        if row.lock == LockMark::Own {
                                            theme.warning.text
                                        } else {
                                            theme.warning.text.opacity(0.45)
                                        },
                                    ))
                                })
                                .when(row.problems > 0, |this| {
                                    this.child(
                                        ui::Badge::new(row.problems.to_string())
                                            .tone(ui::Tone::Warning),
                                    )
                                })
                        })
                        .collect()
                })
                .flex_1()
                .min_w_0()
                .pb(r(8.0))
                .into_any_element()
            })
    }
}
