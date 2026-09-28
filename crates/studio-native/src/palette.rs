//! The command palette (§3.2 Command; D2, D4): every command with its
//! shortcut, recent ones first, and every element of the model by qualified
//! name, in one keyboard-driven list. Ctrl+K opens both; Ctrl+P ("go to
//! element") opens the elements. Enter runs the highlighted row; a command
//! that cannot run now stays listed with the reason.
use crate::{
    commands::{self, CommandId},
    studio::{Dirty, PaletteMode, Studio},
    ui::{self, ActiveTheme, IconName, KeyCaps, icon, r, theme},
    workspace::StudioExt,
};
use agq_language::{ElementId, ElementKind};
use gpui::{
    App, AppContext, ClickEvent, Context, DismissEvent, ElementId as GpuiId, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, ScrollStrategy,
    SharedString, StatefulInteractiveElement, Styled, Subscription, UniformListScrollHandle, Window,
    actions, div, prelude::FluentBuilder, uniform_list,
};
use gpui_base::input::{Input, InputEvent, InputState};
use std::rc::Rc;

actions!(palette, [Next, Previous, Choose, Close]);

pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", Next, Some("Palette")),
        KeyBinding::new("up", Previous, Some("Palette")),
        KeyBinding::new("enter", Choose, Some("Palette")),
        KeyBinding::new("escape", Close, Some("Palette")),
    ]);
}

#[derive(Clone)]
enum Action {
    Command(CommandId),
    Element(ElementId),
}

#[derive(Clone)]
struct Row {
    action: Action,
    title: SharedString,
    detail: SharedString,
    icon: Option<IconName>,
    shortcut: &'static str,
    reason: Option<&'static str>,
    group: &'static str,
}

const ROW: f32 = 40.0;
const LIST: f32 = 400.0;

pub struct Palette {
    studio: Entity<Studio>,
    mode: PaletteMode,
    input: Entity<InputState>,
    rows: Rc<Vec<Row>>,
    /// The nearest element names when nothing matches (D2).
    nearest: Vec<SharedString>,
    selected: usize,
    scroll: UniformListScrollHandle,
    focus: FocusHandle,
    _subscription: Subscription,
}

impl EventEmitter<DismissEvent> for Palette {}

impl Focusable for Palette {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// An element's icon by kind.
pub fn kind_icon(kind: ElementKind) -> IconName {
    use ElementKind::*;
    match kind {
        Package => IconName::Folder,
        Part => IconName::Part,
        PartDef => IconName::Definition,
        Port | PortDef => IconName::Port,
        Item | ItemDef => IconName::Item,
        Attribute | AttributeDef => IconName::Attribute,
        Interface | InterfaceDef => IconName::Interface,
        Connection | ConnectionDef => IconName::Connection,
        Requirement | RequirementDef => IconName::Requirement,
        Satisfy => IconName::Satisfy,
        _ => IconName::Circle,
    }
}

impl Palette {
    pub fn new(studio: Entity<Studio>, mode: PaletteMode, window: &mut Window, cx: &mut Context<Palette>) -> Palette {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(match mode {
                PaletteMode::Commands => "Search commands and elements",
                PaletteMode::Elements => "Go to an element by name",
            })
        });
        let subscription = cx.subscribe_in(&input, window, |palette, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                palette.search(cx);
            }
        });
        let mut palette = Palette {
            studio,
            mode,
            input,
            rows: Rc::new(Vec::new()),
            nearest: Vec::new(),
            selected: 0,
            scroll: UniformListScrollHandle::new(),
            focus: cx.focus_handle(),
            _subscription: subscription,
        };
        palette.search(cx);
        palette
    }

    pub fn set_mode(&mut self, mode: PaletteMode, _: &mut Window, cx: &mut Context<Palette>) {
        if self.mode != mode {
            self.mode = mode;
            self.search(cx);
        }
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Palette>) {
        let focus = self.input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
    }

    fn search(&mut self, cx: &mut Context<Palette>) {
        let query = self.input.read(cx).value().trim().to_string();
        let studio = self.studio.read(cx);
        let mut rows = Vec::new();
        if self.mode == PaletteMode::Commands {
            let context = studio.context();
            let mut found: Vec<_> = commands::search(&query)
                .map(|command| {
                    let reason = commands::unavailable(command.id, &context);
                    let recent = studio.recent_commands.iter().position(|recent| *recent == command.id);
                    (recent, command, reason)
                })
                .collect();
            // With nothing typed, the commands run last come first.
            if query.is_empty() {
                found.sort_by_key(|(recent, _, _)| recent.unwrap_or(usize::MAX));
            }
            for (recent, command, reason) in found {
                rows.push(Row {
                    action: Action::Command(command.id),
                    title: command.label.into(),
                    detail: reason.unwrap_or(command.description).into(),
                    icon: crate::workspace::command_icon(command.id),
                    shortcut: command.shortcut,
                    reason,
                    group: if query.is_empty() && recent.is_some() {
                        "Recent"
                    } else {
                        "Commands"
                    },
                });
            }
        }
        // Elements by qualified name (§3.4: fuzzy matching on qualified names).
        let mut nearest = Vec::new();
        if let Some(project) = &studio.project
            && (!query.is_empty() || self.mode == PaletteMode::Elements)
        {
            let tree = project.state().tree();
            let mut matches: Vec<(usize, ElementId, String, ElementKind)> = Vec::new();
            for id in tree.walk() {
                let Some(name) = tree.effective_name(id) else { continue };
                let element = &tree[id];
                if matches!(element.kind, ElementKind::Doc | ElementKind::Comment) {
                    continue;
                }
                let qualified = tree.qualified_name(id);
                let score = commands::fuzzy_score(&query, name)
                    .map(|score| score / 2)
                    .or_else(|| commands::fuzzy_score(&query, &qualified));
                if let Some(score) = score {
                    matches.push((score, id, qualified, element.kind));
                }
            }
            matches.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.len().cmp(&b.2.len())));
            if matches.is_empty() && !query.is_empty() {
                nearest = nearest_names(&query, tree);
            }
            let limit = if self.mode == PaletteMode::Elements { 2000 } else { 40 };
            for (_, id, qualified, kind) in matches.into_iter().take(limit) {
                let short = qualified.rsplit("::").next().unwrap_or(&qualified).to_string();
                rows.push(Row {
                    action: Action::Element(id),
                    title: short.into(),
                    detail: format!("{} · {qualified}", kind.keyword()).into(),
                    icon: Some(kind_icon(kind)),
                    shortcut: "",
                    reason: None,
                    group: "Elements",
                });
            }
        } else if studio.project.is_none() && !query.is_empty() {
            // A fixture: the cards of the loaded view.
            let mut matches: Vec<_> = studio
                .scene
                .nodes
                .iter()
                .filter_map(|node| commands::fuzzy_score(&query, &node.semantic.name).map(|s| (s, node)))
                .collect();
            matches.sort_by_key(|(score, _)| *score);
            for (_, node) in matches.into_iter().take(40) {
                rows.push(Row {
                    action: Action::Element(node.id()),
                    title: node.semantic.name.clone().into(),
                    detail: node.category.label().into(),
                    icon: Some(IconName::Part),
                    shortcut: "",
                    reason: None,
                    group: "Elements",
                });
            }
        }
        self.selected = rows.iter().position(|row| row.reason.is_none()).unwrap_or(0);
        self.rows = Rc::new(rows);
        self.nearest = nearest.into_iter().map(SharedString::from).collect();
        self.scroll.scroll_to_item(self.selected, ScrollStrategy::Top);
        cx.notify();
    }

    fn step(&mut self, forward: bool, cx: &mut Context<Palette>) {
        let enabled: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.reason.is_none().then_some(index))
            .collect();
        if enabled.is_empty() {
            return;
        }
        let current = enabled.iter().position(|row| *row == self.selected).unwrap_or(0);
        self.selected = enabled[if forward {
            (current + 1) % enabled.len()
        } else {
            (current + enabled.len() - 1) % enabled.len()
        }];
        self.scroll.scroll_to_item(self.selected, ScrollStrategy::Nearest);
        cx.notify();
    }

    fn choose(&mut self, index: usize, cx: &mut Context<Palette>) {
        let Some(row) = self.rows.get(index).cloned() else {
            return;
        };
        if row.reason.is_some() {
            return;
        }
        cx.emit(DismissEvent);
        self.studio.act(cx, |studio| {
            studio.palette = None;
            studio.mark(Dirty::OVERLAY);
            match row.action {
                Action::Command(command) => {
                    studio.recent_commands.retain(|recent| *recent != command);
                    studio.recent_commands.insert(0, command);
                    studio.recent_commands.truncate(5);
                    studio.execute(command);
                }
                Action::Element(id) => {
                    if studio.project.is_some() {
                        studio.reveal(id);
                    } else {
                        studio.show_element(id);
                    }
                }
            }
        });
    }
}

/// The element names closest to a query that matched nothing, by edit
/// distance (D2: a name with no match shows the nearest names).
fn nearest_names(query: &str, tree: &agq_language::Tree) -> Vec<String> {
    let query = query.to_lowercase();
    let distance = |a: &str, b: &str| {
        let b: Vec<char> = b.chars().collect();
        let mut row: Vec<usize> = (0..=b.len()).collect();
        for (i, ca) in a.chars().enumerate() {
            let mut previous = row[0];
            row[0] = i + 1;
            for (j, cb) in b.iter().enumerate() {
                let current = row[j + 1];
                row[j + 1] = (current + 1).min(row[j] + 1).min(previous + usize::from(ca != *cb));
                previous = current;
            }
        }
        row[b.len()]
    };
    let mut names: Vec<(usize, String)> = tree
        .walk()
        .into_iter()
        .filter_map(|id| tree.effective_name(id).map(|name| name.to_string()))
        .map(|name| (distance(&query, &name.to_lowercase()), name))
        .collect();
    names.sort();
    names.dedup_by(|a, b| a.1 == b.1);
    names.into_iter().take(3).map(|(_, name)| name).collect()
}

impl Render for Palette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let rows = self.rows.clone();
        let selected = self.selected;
        let count = rows.len();
        let entity = cx.entity();
        let mode_chip = (self.mode == PaletteMode::Elements).then(|| {
            ui::Chip::new("Elements").icon(IconName::Search).tone(ui::Tone::Accent)
        });
        let list_height = (count as f32 * ROW).min(LIST) + 8.0;
        let nearest = self.nearest.clone();
        let panel = div()
            .id("palette")
            .key_context("Palette")
            .track_focus(&self.focus)
            .role(gpui::Role::Dialog)
            .aria_label("Command palette")
            .occlude()
            .w(r(640.0))
            .rounded(r(crate::tokens::radius::DIALOG))
            .bg(theme.overlay)
            .border_1()
            .border_color(theme.border)
            .shadow(theme.shadow_overlay())
            .overflow_hidden()
            .on_action(cx.listener(|palette, _: &Next, _, cx| palette.step(true, cx)))
            .on_action(cx.listener(|palette, _: &Previous, _, cx| palette.step(false, cx)))
            .on_action(cx.listener(|palette, _: &Choose, _, cx| palette.choose(palette.selected, cx)))
            .on_action(cx.listener(|_, _: &Close, _, cx| cx.emit(DismissEvent)))
            .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(DismissEvent)))
            .child(
                div()
                    .h(r(52.0))
                    .px(r(16.0))
                    .flex()
                    .items_center()
                    .gap(r(10.0))
                    .text_size(r(theme::text::LG))
                    .child(icon(IconName::Search).size(16.0).color(theme.text_muted))
                    .child(div().flex_1().child(Input::new(&self.input)))
                    .when_some(mode_chip, |this, chip| this.child(chip)),
            )
            .child(ui::divider(cx))
            .child(if count == 0 {
                div()
                    .py(r(28.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(r(8.0))
                    .text_color(theme.text_muted)
                    .child("No command or element matches")
                    .when(!nearest.is_empty(), |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(r(6.0))
                                .text_size(r(theme::text::SM))
                                .child("Nearest names:")
                                .children(nearest.into_iter().map(|name| ui::Chip::new(name).mono())),
                        )
                    })
                    .into_any_element()
            } else {
                div()
                    .h(r(list_height))
                    .px(r(6.0))
                    .py(r(4.0))
                    .child(
                        uniform_list("palette-rows", count, move |range, _, cx| {
                            let theme = cx.theme().clone();
                            range
                                .map(|index| {
                                    let row = &rows[index];
                                    let current = index == selected;
                                    let enabled = row.reason.is_none();
                                    let first_of_group = index == 0 || rows[index - 1].group != row.group;
                                    let entity = entity.clone();
                                    div()
                                        .id(GpuiId::NamedInteger("palette-row".into(), index as u64))
                                        .role(gpui::Role::ListBoxOption)
                                        .aria_selected(current)
                                        .aria_label(row.title.clone())
                                        .h(r(ROW))
                                        .px(r(10.0))
                                        .flex()
                                        .items_center()
                                        .gap(r(10.0))
                                        .rounded(r(crate::tokens::radius::CONTROL + 2.0))
                                        .when(current && enabled, |this| this.bg(theme.accent.soft))
                                        .when(enabled && !current, |this| this.hover(|style| style.bg(theme.hover)))
                                        .when(enabled, |this| {
                                            this.cursor_pointer().on_click(move |_: &ClickEvent, _, cx| {
                                                entity.update(cx, |palette, cx| palette.choose(index, cx))
                                            })
                                        })
                                        .child(
                                            div()
                                                .size(r(24.0))
                                                .flex_none()
                                                .rounded(r(6.0))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .bg(if current { theme.raised } else { theme.hover.opacity(0.6) })
                                                .border_1()
                                                .border_color(theme.separator)
                                                .when_some(row.icon, |this, glyph| {
                                                    this.child(icon(glyph).size(14.0).color(if enabled {
                                                        if current { theme.accent.text } else { theme.text_secondary }
                                                    } else {
                                                        theme.text_faint
                                                    }))
                                                }),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .flex()
                                                .items_baseline()
                                                .gap(r(8.0))
                                                .child(
                                                    div()
                                                        .flex_none()
                                                        .text_size(r(theme::text::BASE))
                                                        .font_weight(theme::MEDIUM)
                                                        .text_color(if enabled { theme.text } else { theme.text_faint })
                                                        .child(row.title.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .overflow_hidden()
                                                        .whitespace_nowrap()
                                                        .text_ellipsis()
                                                        .text_size(r(theme::text::SM))
                                                        .text_color(theme.text_muted)
                                                        .child(row.detail.clone()),
                                                ),
                                        )
                                        .when(first_of_group, |this| {
                                            this.child(
                                                div()
                                                    .text_size(r(theme::text::XS))
                                                    .text_color(theme.text_faint)
                                                    .child(row.group),
                                            )
                                        })
                                        .when(!row.shortcut.is_empty(), |this| this.child(KeyCaps::new(row.shortcut)))
                                })
                                .collect()
                        })
                        .track_scroll(&self.scroll)
                        .h_full(),
                    )
                    .into_any_element()
            })
            .child(ui::divider(cx))
            .child(
                div()
                    .h(r(34.0))
                    .px(r(16.0))
                    .flex()
                    .items_center()
                    .gap(r(14.0))
                    .text_size(r(theme::text::XS))
                    .text_color(theme.text_muted)
                    .children([("↑ ↓", "move"), ("Enter", "run"), ("Esc", "close")].into_iter().map(
                        |(keys, what)| {
                            div()
                                .flex()
                                .items_center()
                                .gap(r(6.0))
                                .child(KeyCaps::new(keys))
                                .child(what)
                        },
                    ))
                    .child(div().flex_1())
                    .child(match self.mode {
                        PaletteMode::Commands => "Commands and elements",
                        PaletteMode::Elements => "Elements, by qualified name",
                    }),
            );
        div()
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .pt(gpui::relative(0.12))
            .child(ui::overlay::entrance("palette-entrance", div().child(panel)))
    }
}

#[cfg(test)]
mod tests {
    use super::nearest_names;

    #[test]
    fn a_name_with_no_match_offers_the_nearest_names() {
        let tree = agq_studio_scene::fixtures::tree(agq_studio_scene::fixtures::URL_SHORTENER);
        let nearest = nearest_names("linkstor", &tree);
        assert_eq!(nearest.first().map(String::as_str), Some("LinkStore"), "{nearest:?}");
    }
}
