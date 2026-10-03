//! The Library panel (C-49; Scenario H1–H3), beside the Outline: reusable
//! building blocks from the built-in library, the project and My Library.
//! The search finds them as you type and marks what matched; the scope and
//! kind filters narrow them; the arrow keys move through them; the preview
//! shows the selected block's ports, inner parts and connections before it
//! is used. Enter adds a usage of it to the selected part (or, under "What
//! can connect here?", beside the port's part, connected to it); a block can
//! also be dragged onto the Surface. Everything it does goes through the
//! Studio's one change path.
use super::block_preview::BlockPreview;
use crate::{
    palette::kind_icon,
    studio::{Dirty, Studio, StudioEvent},
    ui::{self, ActiveTheme, Button, IconName, Segmented, TextField, icon, r, theme},
    workspace::StudioExt,
};
use agq_language::ElementKind;
use agq_library::{BlockRef, Hit, Preview, Query, Scope};
use gpui::{
    App, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable, Font, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, Render, ScrollStrategy, SharedString,
    StatefulInteractiveElement, Styled, StyledText, Subscription, TextRun, UniformListScrollHandle,
    Window, actions, div, font, prelude::FluentBuilder, uniform_list,
};
use gpui_base::input::{InputEvent, InputState};
use std::rc::Rc;

actions!(library, [Next, Previous, Clear]);

pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", Next, Some("Library")),
        KeyBinding::new("up", Previous, Some("Library")),
        KeyBinding::new("escape", Clear, Some("Library")),
    ]);
}

/// A building block dragged from the Library onto the Surface.
#[derive(Clone, Debug)]
pub struct LibraryDrag {
    pub block: BlockRef,
    pub name: SharedString,
    pub kind: ElementKind,
    pub composite: bool,
}

/// What follows the pointer while a block is dragged: a card-like chip.
pub struct DragGhost {
    drag: LibraryDrag,
}

impl DragGhost {
    pub fn new(drag: LibraryDrag) -> Self {
        DragGhost { drag }
    }
}

impl Render for DragGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .px(r(10.0))
            .py(r(6.0))
            .rounded(r(crate::tokens::radius::CARD))
            .bg(theme.raised.opacity(0.94))
            .border_1()
            .border_color(theme.accent.solid)
            .shadow(theme.shadow_overlay())
            .flex()
            .items_center()
            .gap(r(8.0))
            .text_size(r(theme::text::SM))
            .child(
                icon(if self.drag.composite {
                    IconName::Component
                } else {
                    kind_icon(self.drag.kind)
                })
                .size(14.0)
                .color(theme.accent.text),
            )
            .child(
                div()
                    .font_weight(theme::MEDIUM)
                    .child(self.drag.name.clone()),
            )
            .child(
                div()
                    .text_size(r(theme::text::XS))
                    .text_color(theme.text_muted)
                    .child("Drop to add"),
            )
    }
}

/// The kind filters, each one or more definition kinds, with its icon.
const KINDS: [(&str, IconName, &[ElementKind]); 5] = [
    ("Parts", IconName::Part, &[ElementKind::PartDef]),
    (
        "Ports and interfaces",
        IconName::Port,
        &[
            ElementKind::PortDef,
            ElementKind::InterfaceDef,
            ElementKind::ConnectionDef,
        ],
    ),
    ("Items", IconName::Item, &[ElementKind::ItemDef]),
    (
        "Value types",
        IconName::Attribute,
        &[ElementKind::AttributeDef],
    ),
    (
        "Requirements",
        IconName::Requirement,
        &[ElementKind::RequirementDef],
    ),
];

const ROW: f32 = 46.0;

/// A preview made for a block, My Library's version and the project's
/// revision.
type Shown = (BlockRef, u64, Option<u64>, Option<Rc<Preview>>);

/// What the results were found for: My Library's version, the project's
/// revision, the query, the filters and the port that blocks must fit.
type Key = (
    u64,
    Option<u64>,
    String,
    Option<Scope>,
    Vec<ElementKind>,
    Option<(agq_language::ElementId, agq_language::ElementId)>,
);

pub struct LibraryView {
    studio: Entity<Studio>,
    search: Entity<InputState>,
    hits: Rc<Vec<Hit>>,
    key: Option<Key>,
    /// The highlighted result.
    cursor: usize,
    scroll: UniformListScrollHandle,
    focus: FocusHandle,
    /// The preview shown, for the block, My Library's version and the
    /// project's revision it was made for.
    preview: Option<Shown>,
    _subscriptions: Vec<Subscription>,
}

impl LibraryView {
    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search blocks: cache, queue, rate limit…")
        });
        let subscriptions = vec![
            cx.subscribe(&studio, |_, _, event: &StudioEvent, cx| {
                if event.0.intersects(
                    Dirty::MODEL
                        | Dirty::LAYOUT
                        | Dirty::APPEARANCE
                        | Dirty::OVERLAY
                        | Dirty::SELECTION,
                ) {
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &search,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => {
                        this.key = None;
                        this.cursor = 0;
                        this.pick(0, cx);
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => this.insert(window, cx),
                    _ => {}
                },
            ),
        ];
        LibraryView {
            studio,
            search,
            hits: Rc::new(Vec::new()),
            key: None,
            cursor: 0,
            scroll: UniformListScrollHandle::new(),
            focus: cx.focus_handle(),
            preview: None,
            _subscriptions: subscriptions,
        }
    }

    /// The search box's text (the scripted journeys).
    #[cfg(feature = "automation")]
    pub fn query(&self, cx: &App) -> String {
        self.search.read(cx).value().to_string()
    }

    /// Finds the blocks for the current query and filters, if anything
    /// changed since.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().trim().to_string();
        self.studio.update(cx, |studio, _| {
            studio.library_index();
        });
        let studio = self.studio.read(cx);
        let library = &studio.library;
        let revision = studio.project.as_ref().map(|p| p.state().revision());
        let key: Key = (
            library.source.version(),
            revision,
            query.clone(),
            library.scope,
            library.kinds.clone(),
            library.fit.as_ref().map(|f| (f.card, f.port)),
        );
        if self.key.as_ref() == Some(&key) {
            return;
        }
        let only: Option<Vec<usize>> = library
            .fit
            .as_ref()
            .map(|f| f.fits.iter().map(|fit| fit.block).collect());
        let mut hits = library.index.search(&Query {
            text: &query,
            scope: library.scope,
            kinds: &library.kinds,
            only: only.as_deref(),
            limit: 0,
        });
        // With nothing typed, the blocks used last come first.
        if query.is_empty() {
            let at = |hit: &Hit| {
                library.index.get(hit.block).and_then(|b| {
                    library
                        .recent
                        .iter()
                        .position(|recent| *recent == b.reference)
                })
            };
            hits.sort_by_key(|hit| at(hit).unwrap_or(usize::MAX));
        }
        // Keep the chosen block highlighted when it is still found.
        if let Some(selected) = &library.selected
            && let Some(at) = hits.iter().position(|h| {
                library
                    .index
                    .get(h.block)
                    .is_some_and(|b| b.reference == *selected)
            })
        {
            self.cursor = at;
        }
        self.cursor = self.cursor.min(hits.len().saturating_sub(1));
        self.hits = Rc::new(hits);
        self.key = Some(key);
    }

    /// The block under the cursor.
    fn current(&self, cx: &App) -> Option<BlockRef> {
        let hit = self.hits.get(self.cursor)?;
        self.studio
            .read(cx)
            .library
            .index
            .get(hit.block)
            .map(|b| b.reference.clone())
    }

    /// Moves the cursor to a result and shows it in the preview.
    fn pick(&mut self, index: usize, cx: &mut Context<Self>) {
        self.cursor = index;
        let block = self.current(cx);
        self.studio.update(cx, |studio, _| {
            studio.library.selected = block;
        });
        self.scroll.scroll_to_item(index, ScrollStrategy::Nearest);
        cx.notify();
    }

    fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let count = self.hits.len();
        if count == 0 {
            return;
        }
        let next = if forward {
            (self.cursor + 1).min(count - 1)
        } else {
            self.cursor.saturating_sub(1)
        };
        self.pick(next, cx);
    }

    /// Adds the highlighted block: into the selected part, or beside the
    /// port's part and connected to it under "What can connect here?".
    fn insert(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(block) = self.current(cx) else {
            return;
        };
        self.studio.act(cx, |studio| insert(studio, block));
    }

    /// Escape: clears the search, then the port filter, then gives the
    /// keyboard back to the Surface.
    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search.read(cx).value().is_empty() {
            self.search
                .update(cx, |state, cx| state.set_value("", window, cx));
            return;
        }
        let fit = self.studio.read(cx).library.fit.is_some();
        self.studio.act(cx, |studio| {
            if fit {
                studio.library.fit = None;
            }
            studio.mark(Dirty::LAYOUT | Dirty::OVERLAY);
        });
        if !fit {
            window.blur(cx);
        }
    }

    /// The preview of the highlighted block, made again only when it, My
    /// Library or the model changed.
    fn preview(&mut self, cx: &mut Context<Self>) -> Option<Rc<Preview>> {
        let block = self.current(cx)?;
        let studio = self.studio.read(cx);
        let version = studio.library.source.version();
        let revision = studio.project.as_ref().map(|p| p.state().revision());
        if let Some((shown, v, rev, preview)) = &self.preview
            && *shown == block
            && *v == version
            && *rev == revision
        {
            return preview.clone();
        }
        let index = studio.library.index.position(&block)?;
        let preview = studio
            .library
            .source
            .preview(
                &studio.library.index,
                index,
                studio.project.as_ref().map(|p| p.state().tree()),
            )
            .map(Rc::new);
        self.preview = Some((block, version, revision, preview.clone()));
        preview
    }
}

/// Uses a block from the Library panel or the palette.
pub fn insert(studio: &mut Studio, block: BlockRef) {
    match studio.library.fit.clone() {
        Some(filter) => {
            let with = studio
                .library
                .index
                .position(&block)
                .and_then(|i| filter.fits.iter().find(|f| f.block == i))
                .and_then(|f| f.ports.first().cloned());
            let parent = studio.fit_parent(&filter);
            if studio
                .insert_block(
                    block,
                    parent,
                    Some(agq_library::ConnectTo {
                        card: filter.card,
                        port: filter.port,
                        with,
                    }),
                )
                .is_some()
            {
                studio.library.fit = None;
            }
        }
        None => {
            studio.insert_block(block, None, None);
        }
    }
    studio.mark(Dirty::ALL);
}

/// A name with its matched characters marked.
fn marked(name: &str, positions: &[usize], colour: gpui::Hsla, mark: gpui::Hsla) -> StyledText {
    let mut runs = Vec::new();
    let mut start = 0;
    let base: Font = font(theme::SANS);
    let mut strong = font(theme::SANS);
    strong.weight = theme::SEMIBOLD;
    let mut normal = base.clone();
    normal.weight = theme::MEDIUM;
    let chars: Vec<(usize, char)> = name.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let hit = positions.contains(&i);
        let mut j = i;
        while j < chars.len() && positions.contains(&j) == hit {
            j += 1;
        }
        let end = chars.get(j).map_or(name.len(), |(b, _)| *b);
        runs.push(TextRun {
            len: end - start,
            font: if hit { strong.clone() } else { normal.clone() },
            color: if hit { mark } else { colour },
            background_color: None,
            underline: None,
            strikethrough: None,
        });
        start = end;
        i = j;
    }
    StyledText::new(SharedString::from(name.to_string())).with_runs(runs)
}

impl Render for LibraryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Its own region inside the cached left column, so the column
        // painting again does not lose this view's controls (C-53).
        crate::ui::target::regioned("left-body", self.content(window, cx))
    }
}

impl LibraryView {
    fn content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        self.refresh(cx);
        // Ctrl+Shift+L and the palette ask for the search box.
        if self.studio.read(cx).library.focus_search {
            self.studio.update(cx, |studio, _| {
                studio.library.focus_search = false;
            });
            let focus = self.search.read(cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
        let preview = self.preview(cx);
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let library = &studio.library;
        let scope = library.scope;
        let kinds = library.kinds.clone();
        let fit = library.fit.as_ref().map(|f| f.label.clone());
        let problems = library.source.problems().len();
        let editable = studio.editable();
        let hits = self.hits.clone();
        let count = hits.len();
        let cursor = self.cursor;
        let recent = library.recent.clone();
        let rows: Rc<Vec<RowData>> = Rc::new(
            hits.iter()
                .filter_map(|hit| {
                    let block = library.index.get(hit.block)?;
                    Some(RowData {
                        reference: block.reference.clone(),
                        name: block.name.clone(),
                        positions: hit.name_positions.clone(),
                        kind: block.kind,
                        composite: block.composite(),
                        source: block.source_label(),
                        category: block.category.join(" › "),
                        summary: block.summary.clone(),
                        ports: block
                            .ports
                            .iter()
                            .map(|p| p.name.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                        recent: recent.contains(&block.reference),
                        problems: block.problems,
                        usable: agq_library::usage_kind(block.kind).is_some(),
                    })
                })
                .collect(),
        );
        let shown = rows.get(cursor).cloned();
        let entity = cx.entity();
        let empty_text = if library.fit.is_some() {
            "No building block has a port that fits. Model what is needed in the project."
        } else if self.search.read(cx).value().trim().is_empty() {
            "No building blocks in this scope yet."
        } else {
            "No building block matches. If nothing fits, model the concept in the project."
        };
        let scope_index = match scope {
            None => 0,
            Some(Scope::BuiltIn) => 1,
            Some(Scope::Project) => 2,
            Some(Scope::Mine) => 3,
        };
        div()
            .id("library")
            .key_context("Library")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.chrome)
            .role(gpui::Role::Region)
            .aria_label("Library of building blocks")
            .on_action(cx.listener(|this, _: &Next, _, cx| this.step(true, cx)))
            .on_action(cx.listener(|this, _: &Previous, _, cx| this.step(false, cx)))
            .on_action(cx.listener(|this, _: &Clear, window, cx| this.clear(window, cx)))
            .child(
                div()
                    .flex_none()
                    .px(r(8.0))
                    .pt(r(8.0))
                    .pb(r(4.0))
                    .relative()
                    .child(
                        TextField::new(&self.search)
                            .leading(IconName::Search)
                            .target("Library search"),
                    ),
            )
            .when_some(fit, |this, label| {
                let studio = self.studio.clone();
                this.child(
                    div()
                        .flex_none()
                        .mx(r(8.0))
                        .mb(r(4.0))
                        .px(r(8.0))
                        .h(r(26.0))
                        .flex()
                        .items_center()
                        .gap(r(6.0))
                        .rounded(r(crate::tokens::radius::CONTROL))
                        .bg(theme.accent.soft)
                        .text_size(r(theme::text::SM))
                        .text_color(theme.accent.text)
                        .child(icon(IconName::Port).size(12.0).color(theme.accent.text))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(format!("Fits {label}")),
                        )
                        .child(
                            Button::icon_only("library-clear-fit", IconName::Close, "Show every block")
                                .small()
                                .on_click(move |_, _, cx| {
                                    studio.act(cx, |studio| {
                                        studio.library.fit = None;
                                        studio.mark(Dirty::LAYOUT);
                                    })
                                }),
                        ),
                )
            })
            .child(
                div()
                    .flex_none()
                    .px(r(8.0))
                    .pb(r(6.0))
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .child(div().flex_1().min_w_0().child({
                        let studio = self.studio.clone();
                        Segmented::new("library-scope", scope_index)
                            .choice(None, "All")
                            .tooltip("Every block", None)
                            .choice(None, "Built-in")
                            .tooltip("The blocks that come with Agentique", None)
                            .choice(None, "Project")
                            .tooltip("This project's own definitions", None)
                            .choice(None, "Mine")
                            .tooltip("My Library: the blocks you saved", None)
                            .on_choose(move |index, _, cx| {
                                studio.act(cx, |studio| {
                                    studio.library.scope = match index {
                                        1 => Some(Scope::BuiltIn),
                                        2 => Some(Scope::Project),
                                        3 => Some(Scope::Mine),
                                        _ => None,
                                    };
                                    studio.mark(Dirty::LAYOUT);
                                })
                            })
                    })),
            )
            .child(
                div()
                    .flex_none()
                    .px(r(8.0))
                    .pb(r(6.0))
                    .flex()
                    .items_center()
                    .gap(r(2.0))
                    .child(
                        div()
                            .mr(r(4.0))
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_faint)
                            .child("Kinds"),
                    )
                    .children(KINDS.iter().enumerate().map(|(i, (label, glyph, of))| {
                        let on = of.iter().all(|k| kinds.contains(k)) && !kinds.is_empty();
                        let studio = self.studio.clone();
                        let of: Vec<ElementKind> = of.to_vec();
                        Button::icon_only(("library-kind", i), *glyph, *label)
                            .small()
                            .selected(on)
                            .on_click(move |_: &ClickEvent, _, cx| {
                                let of = of.clone();
                                studio.act(cx, |studio| {
                                    let kinds = &mut studio.library.kinds;
                                    if of.iter().all(|k| kinds.contains(k)) {
                                        kinds.retain(|k| !of.contains(k));
                                    } else {
                                        for kind in &of {
                                            if !kinds.contains(kind) {
                                                kinds.push(*kind);
                                            }
                                        }
                                    }
                                    studio.mark(Dirty::LAYOUT);
                                })
                            })
                    })),
            )
            .child(ui::divider(cx))
            .child(if count == 0 {
                div()
                    .flex_1()
                    .p(r(16.0))
                    .child(super::note(empty_text, cx))
                    .into_any_element()
            } else {
                uniform_list("library-rows", count, move |range, _, cx| {
                    let theme = cx.theme().clone();
                    range
                        .map(|index| {
                            let row = &rows[index];
                            let current = index == cursor;
                            let entity = entity.clone();
                            let insert_entity = entity.clone();
                            let drag = LibraryDrag {
                                block: row.reference.clone(),
                                name: row.name.clone().into(),
                                kind: row.kind,
                                composite: row.composite,
                            };
                            div()
                                .id(("library-row", index))
                                .role(gpui::Role::ListBoxOption)
                                .aria_selected(current)
                                .aria_label(SharedString::from(format!(
                                    "{}, {}{}, {}. {}",
                                    row.name,
                                    row.kind.keyword(),
                                    if row.composite { ", composite" } else { "" },
                                    row.source,
                                    row.summary
                                )))
                                .h(r(ROW))
                                .mx(r(6.0))
                                .px(r(8.0))
                                .flex()
                                .flex_col()
                                .justify_center()
                                .gap(r(2.0))
                                .rounded(r(crate::tokens::radius::CONTROL))
                                .cursor_pointer()
                                .when(current, |this| this.bg(theme.accent.soft))
                                .when(!current, |this| this.hover(|style| style.bg(theme.hover)))
                                .relative()
                                .child(ui::target::target(format!("Block {}", row.name)))
                                .on_click(move |event: &ClickEvent, window, cx| {
                                    if event.click_count() >= 2 {
                                        insert_entity.update(cx, |view, cx| {
                                            view.pick(index, cx);
                                            view.insert(window, cx)
                                        });
                                    } else {
                                        entity.update(cx, |view, cx| view.pick(index, cx));
                                    }
                                })
                                .when(row.usable, |this| {
                                    this.on_drag(drag, |drag, _, _, cx| {
                                        cx.new(|_| DragGhost::new(drag.clone()))
                                    })
                                })
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(r(6.0))
                                        .child(
                                            icon(if row.composite {
                                                IconName::Component
                                            } else {
                                                kind_icon(row.kind)
                                            })
                                            .size(13.0)
                                            .color(if row.kind == ElementKind::RequirementDef {
                                                theme.warning.text
                                            } else {
                                                theme.info.text
                                            }),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .text_ellipsis()
                                                .text_size(r(theme::text::SM))
                                                .child(marked(
                                                    &row.name,
                                                    &row.positions,
                                                    theme.text,
                                                    theme.accent.text,
                                                )),
                                        )
                                        .when(row.recent, |this| {
                                            this.child(
                                                icon(IconName::Clock)
                                                    .size(11.0)
                                                    .color(theme.text_faint),
                                            )
                                        })
                                        .when(row.problems > 0, |this| {
                                            this.child(
                                                ui::Badge::new(row.problems.to_string())
                                                    .tone(ui::Tone::Warning),
                                            )
                                        })
                                        .child(
                                            div()
                                                .flex_none()
                                                .text_size(r(theme::text::XS))
                                                .text_color(theme.text_faint)
                                                .child(row.source.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .pl(r(19.0))
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .text_size(r(theme::text::XS))
                                        .text_color(theme.text_muted)
                                        .child(
                                            match (row.ports.is_empty(), row.summary.is_empty()) {
                                                (true, _) => row.summary.clone(),
                                                (false, true) => row.ports.clone(),
                                                (false, false) => {
                                                    format!("{} · {}", row.ports, row.summary)
                                                }
                                            },
                                        ),
                                )
                        })
                        .collect()
                })
                .track_scroll(&self.scroll)
                .flex_1()
                .min_h_0()
                .py(r(4.0))
                .into_any_element()
            })
            .when_some(shown, |this, row| {
                this.child(ui::divider(cx))
                    .child(self.details(row, preview, editable, cx))
            })
            .when(problems > 0, |this| {
                this.child(
                    div()
                        .flex_none()
                        .p(r(8.0))
                        .child(ui::inline_message(
                            ui::Tone::Warning,
                            format!(
                                "My Library has {problems} problem(s); blocks with problems cannot be used until they are fixed."
                            ),
                            cx,
                        )),
                )
            })
    }
}

/// One result, ready to draw.
#[derive(Clone)]
struct RowData {
    reference: BlockRef,
    name: String,
    positions: Vec<usize>,
    kind: ElementKind,
    composite: bool,
    source: String,
    category: String,
    summary: String,
    ports: String,
    recent: bool,
    problems: usize,
    usable: bool,
}

impl LibraryView {
    /// The highlighted block's details: purpose, preview, what it exposes
    /// and contains, and what can be done with it.
    fn details(
        &self,
        row: RowData,
        preview: Option<Rc<Preview>>,
        editable: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let block = studio.library.index.find(&row.reference).cloned();
        let project = row.reference.scope == agq_library::Scope::Project;
        let element = block.as_ref().map(|b| b.element);
        let mine = row.reference.scope == agq_library::Scope::Mine;
        let entity = cx.entity();
        let action = |id: &'static str, label: &'static str, glyph: IconName| {
            Button::new(id, label).small().icon(glyph)
        };
        let insert_label = if studio.library.fit.is_some() {
            "Insert and connect"
        } else {
            "Insert"
        };
        div()
            .id("library-details")
            .flex_none()
            .max_h(gpui::relative(0.55))
            .overflow_y_scroll()
            .p(r(10.0))
            .flex()
            .flex_col()
            .gap(r(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(r(theme::text::BASE))
                            .font_weight(theme::SEMIBOLD)
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(row.name.clone()),
                    )
                    .child(ui::Chip::new(SharedString::from(row.source.clone()))),
            )
            .child(
                div()
                    .text_size(r(theme::text::XS))
                    .text_color(theme.text_muted)
                    .font_family(theme::MONO)
                    .child(format!(
                        "{}{}",
                        block.as_ref().map(|b| b.kind_label()).unwrap_or_default(),
                        if row.category.is_empty() {
                            String::new()
                        } else {
                            format!(" · {}", row.category)
                        }
                    )),
            )
            .when_some(
                block
                    .as_ref()
                    .map(|b| b.doc.clone())
                    .filter(|d| !d.is_empty()),
                |this, doc| {
                    this.child(
                        div()
                            .text_size(r(theme::text::SM))
                            .line_height(r(17.0))
                            .text_color(theme.text_secondary)
                            .child(doc),
                    )
                },
            )
            .when_some(preview.clone(), |this, preview| {
                let height = crate::panels::block_preview::height(&preview);
                this.child(BlockPreview::new("library-preview", preview).height(height))
            })
            .when_some(block.as_ref(), |this, block| {
                let facts: Vec<(&str, String)> = [
                    (
                        "Ports",
                        block
                            .ports
                            .iter()
                            .map(|p| format!("{} : {}", p.name, p.type_name))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    (
                        "Parts",
                        block
                            .parts
                            .iter()
                            .map(|p| format!("{} : {}", p.name, p.type_name))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    (
                        "Values",
                        block
                            .attributes
                            .iter()
                            .map(|a| match &a.value {
                                Some(v) => format!("{} = {v}", a.name),
                                None => format!("{} : {}", a.name, a.type_name),
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    (
                        "Needs",
                        block
                            .requirements
                            .iter()
                            .map(|q| q.name.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    (
                        "Used by",
                        if project {
                            crate::panels::reuse::uses(block.usages, block.specializations)
                        } else {
                            String::new()
                        },
                    ),
                ]
                .into_iter()
                .filter(|(_, text)| !text.is_empty())
                .collect();
                this.children(facts.into_iter().map(|(label, text)| {
                    div()
                        .flex()
                        .gap(r(8.0))
                        .text_size(r(theme::text::XS))
                        .child(
                            div()
                                .w(r(52.0))
                                .flex_none()
                                .text_color(theme.text_muted)
                                .child(label),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_family(theme::MONO)
                                .text_color(theme.text_secondary)
                                .child(text),
                        )
                }))
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(r(6.0))
                    .when(row.usable, |this| {
                        let entity = entity.clone();
                        this.child(
                            Button::new("library-insert", insert_label)
                                .small()
                                .primary()
                                .icon(IconName::Plus)
                                .shortcut("Enter")
                                .disabled(!editable)
                                .on_click(move |_, window, cx| {
                                    entity.update(cx, |view, cx| view.insert(window, cx))
                                }),
                        )
                    })
                    .when_some(element.filter(|_| project), |this, element| {
                        let open = self.studio.clone();
                        let usages = self.studio.clone();
                        let special = self.studio.clone();
                        let save = self.studio.clone();
                        this.child(
                            action("library-open", "Open definition", IconName::Definition)
                                .on_click(move |_, _, cx| {
                                    open.act(cx, |studio| studio.open_definition(element))
                                }),
                        )
                        .child(
                            action("library-usages", "Find usages", IconName::Search).on_click(
                                move |_, _, cx| {
                                    usages.act(cx, |studio| {
                                        studio.find_usages(element);
                                        studio.mark(Dirty::OVERLAY);
                                    })
                                },
                            ),
                        )
                        .when(editable, |this| {
                            this.child(
                                action("library-specialize", "Specialise…", IconName::Branch)
                                    .on_click(move |_, _, cx| {
                                        special.act(cx, |studio| studio.start_specialize(element))
                                    }),
                            )
                        })
                        .child(
                            action("library-save", "Save to My Library…", IconName::Save).on_click(
                                move |_, _, cx| save.act(cx, |studio| studio.start_save(element)),
                            ),
                        )
                    })
                    .when(mine, |this| {
                        let studio = self.studio.clone();
                        let reference = row.reference.clone();
                        this.child(
                            Button::new("library-remove", "Remove from My Library")
                                .small()
                                .ghost()
                                .danger()
                                .icon(IconName::Trash)
                                .on_click(move |_, _, cx| {
                                    let reference = reference.clone();
                                    studio.act(cx, |studio| studio.remove_from_library(&reference))
                                }),
                        )
                    }),
            )
            .into_any_element()
    }
}
