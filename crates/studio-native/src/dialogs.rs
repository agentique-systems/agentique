//! The Studio's dialogs over the workspace: New project and Open project
//! (with the folder picker of Windows), Create, Checkpoint, Move to and the
//! confirmation of a change to locked elements or a shared definition.
//! Enter confirms, Escape cancels; the first field has the keyboard. Rename
//! is done in place on the Surface.
use crate::{
    edit::{CreateKind, Dialog},
    studio::{Dirty, Studio, StudioEvent},
    ui::{self, ActiveTheme, Button, IconName, Segmented, TextField, icon, r, theme},
    workspace::StudioExt,
};
use agq_language::Parent;
use gpui::{
    App, AppContext, ClickEvent, Context, Entity, Focusable, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, PathPromptOptions, Render, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Window, actions, div, prelude::FluentBuilder,
};
use gpui_base::input::{InputEvent, InputState};
use std::path::PathBuf;

actions!(dialog, [Cancel, Confirm, Next, Previous]);

pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some("Dialog")),
        KeyBinding::new("enter", Confirm, Some("Dialog && !Input")),
        KeyBinding::new("down", Next, Some("Dialog")),
        KeyBinding::new("up", Previous, Some("Dialog")),
    ]);
}

/// Which dialog the fields were made for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    NewProject,
    OpenProject,
    Create,
    Checkpoint,
    MoveTo,
    Confirm,
}

fn kind_of(dialog: &Dialog) -> Option<Kind> {
    Some(match dialog {
        Dialog::NewProject { .. } => Kind::NewProject,
        Dialog::OpenProject { .. } => Kind::OpenProject,
        Dialog::Create { .. } => Kind::Create,
        Dialog::Checkpoint { .. } => Kind::Checkpoint,
        Dialog::MoveTo { .. } => Kind::MoveTo,
        Dialog::Confirm { .. } => Kind::Confirm,
        Dialog::Rename { .. } => return None,
    })
}

pub struct DialogsView {
    studio: Entity<Studio>,
    kind: Option<Kind>,
    /// The dialog's fields: the first has the keyboard.
    first: Option<Entity<InputState>>,
    second: Option<Entity<InputState>>,
    /// The highlighted owner in Move to.
    chosen: usize,
    focus: gpui::FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl DialogsView {
    /// The text of the dialog's first field (the scripted journeys).
    #[cfg(feature = "automation")]
    pub fn first_text(&self, cx: &App) -> Option<String> {
        self.first
            .as_ref()
            .map(|state| state.read(cx).value().to_string())
    }

    pub fn new(studio: Entity<Studio>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe(&studio, |this, studio, event: &StudioEvent, cx| {
            if event.0.intersects(Dirty::OVERLAY | Dirty::MODEL) {
                // The view is not drawn while no dialog is open: forget the
                // closed dialog now, so the next one gets new fields.
                if studio.read(cx).dialog.as_ref().and_then(kind_of).is_none() {
                    this.kind = None;
                    this.first = None;
                    this.second = None;
                    this._subscriptions.truncate(1);
                }
                cx.notify();
            }
        });
        DialogsView {
            studio,
            kind: None,
            first: None,
            second: None,
            chosen: 0,
            focus: cx.focus_handle(),
            _subscriptions: vec![subscription],
        }
    }

    fn input(
        &mut self,
        value: &str,
        placeholder: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(value.to_string())
        });
        self._subscriptions.push(cx.subscribe_in(
            &state,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => this.confirm(window, cx),
                InputEvent::Change => {
                    this.chosen = 0;
                    cx.notify();
                }
                _ => {}
            },
        ));
        state
    }

    /// Makes the fields for the dialog the Studio has open.
    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = self
            .studio
            .read(cx)
            .dialog
            .as_ref()
            .and_then(|d| kind_of(d).map(|k| (k, d)));
        let wanted = dialog.map(|(kind, _)| kind);
        if wanted == self.kind {
            return;
        }
        self._subscriptions.truncate(1);
        self.first = None;
        self.second = None;
        self.chosen = 0;
        self.kind = wanted;
        let values = match self.studio.read(cx).dialog.as_ref() {
            Some(Dialog::NewProject { folder, name, .. }) => {
                Some((name.clone(), Some(folder.clone())))
            }
            Some(Dialog::OpenProject { folder }) => Some((folder.clone(), None)),
            Some(Dialog::Create { name, .. }) => Some((name.clone(), None)),
            Some(Dialog::Checkpoint { message }) => Some((message.clone(), None)),
            Some(Dialog::MoveTo { query, .. }) => Some((query.clone(), None)),
            _ => None,
        };
        if let Some((first, second)) = values {
            let placeholder = match wanted {
                Some(Kind::NewProject) => "NewSystem",
                Some(Kind::OpenProject) => "C:\\Users\\you\\Agentique\\MySystem",
                Some(Kind::Create) => "Name (Enter for a default)",
                Some(Kind::Checkpoint) => "What changed?",
                Some(Kind::MoveTo) => "Find the new owner",
                _ => "",
            };
            let first = self.input(&first, placeholder, window, cx);
            let focus = first.read(cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
            self.first = Some(first);
            if let Some(second) = second {
                self.second = Some(self.input(&second, "Folder", window, cx));
            }
        } else if wanted.is_some() {
            let focus = self.focus.clone();
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
    }

    fn text(field: &Option<Entity<InputState>>, cx: &App) -> String {
        field
            .as_ref()
            .map(|state| state.read(cx).value().trim().to_string())
            .unwrap_or_default()
    }

    fn cancel(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.studio.act(cx, |studio| {
            if matches!(studio.dialog, Some(Dialog::Confirm { .. })) {
                // Cancel and Escape refuse the change.
                studio.answer(false);
            } else {
                studio.dialog = None;
            }
            studio.mark(Dirty::OVERLAY | Dirty::MODEL);
        });
    }

    fn confirm(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let first = Self::text(&self.first, cx);
        let second = Self::text(&self.second, cx);
        let owners = self.owners(cx);
        let chosen = self.chosen;
        self.studio.act(cx, |studio| {
            let Some(dialog) = studio.dialog.take() else {
                return;
            };
            match dialog {
                Dialog::NewProject { sample, .. } => {
                    if first.is_empty() || second.is_empty() {
                        studio.dialog = Some(dialog);
                        return;
                    }
                    let folder = PathBuf::from(&second);
                    if sample {
                        studio.create_sample(&folder, &first);
                    } else {
                        studio.create_project(&folder, &first);
                    }
                }
                Dialog::OpenProject { .. } => {
                    if first.is_empty() {
                        studio.dialog = Some(dialog);
                        return;
                    }
                    studio.open_project(&PathBuf::from(&first));
                }
                Dialog::Create {
                    kind,
                    definition,
                    parent,
                    ..
                } => studio.create(kind, definition, &first, parent),
                Dialog::Checkpoint { .. } => studio.checkpoint(&first),
                Dialog::MoveTo { element, .. } => match owners.get(chosen) {
                    Some((parent, _)) => studio.move_to(element, *parent),
                    None => studio.dialog = Some(dialog),
                },
                dialog @ Dialog::Confirm { .. } => {
                    studio.dialog = Some(dialog);
                    studio.answer(true);
                }
                dialog @ Dialog::Rename { .. } => studio.dialog = Some(dialog),
            }
            studio.mark(Dirty::ALL);
        });
    }

    /// Owners matching Move to's query, best first.
    fn owners(&self, cx: &App) -> Vec<(Parent, String)> {
        let studio = self.studio.read(cx);
        let Some(Dialog::MoveTo { element, .. }) = &studio.dialog else {
            return Vec::new();
        };
        let query = Self::text(&self.first, cx);
        let mut matches: Vec<_> = studio
            .owner_options(*element)
            .into_iter()
            .filter_map(|(parent, label)| {
                crate::commands::fuzzy_score(&query, &label).map(|score| (score, parent, label))
            })
            .collect();
        matches.sort_by_key(|(score, _, _)| *score);
        matches
            .into_iter()
            .take(40)
            .map(|(_, parent, label)| (parent, label))
            .collect()
    }

    /// Fills a folder field from Windows' folder picker.
    fn browse(&mut self, field: bool, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose a folder".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await
                && let Some(path) = paths.into_iter().next()
            {
                let _ = this.update_in(cx, |this, window, cx| {
                    let target = if field {
                        this.second.clone()
                    } else {
                        this.first.clone()
                    };
                    if let Some(state) = target {
                        state.update(cx, |state, cx| {
                            state.set_value(path.display().to_string(), window, cx)
                        });
                    }
                });
            }
        })
        .detach();
    }
}

impl Render for DialogsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync(window, cx);
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let Some(dialog) = studio.dialog.as_ref() else {
            return div().into_any_element();
        };
        let entity = cx.entity();
        let cancel_button = {
            let entity = entity.clone();
            Button::new("dialog-cancel", "Cancel").on_click(move |_: &ClickEvent, window, cx| {
                entity.update(cx, |this, cx| this.cancel(window, cx))
            })
        };
        let confirm_button = |label: &'static str, enabled: bool| {
            let entity = entity.clone();
            Button::new("dialog-confirm", label)
                .primary()
                .disabled(!enabled)
                .shortcut("Enter")
                .on_click(move |_: &ClickEvent, window, cx| {
                    entity.update(cx, |this, cx| this.confirm(window, cx))
                })
        };
        let body = match dialog {
            Dialog::NewProject { sample, .. } => {
                let ready = !Self::text(&self.first, cx).is_empty()
                    && !Self::text(&self.second, cx).is_empty();
                let entity = entity.clone();
                ui::Dialog::new("new-project", if *sample { "Start from the URL shortener" } else { "New project" })
                    .description("A project is a folder; the model is saved in it as SysML text, with its history in git.")
                    .child(field("Name", self.first.as_ref().map(|s| TextField::new(s).target("Project name").into_any_element()), cx))
                    .child(field(
                        "Folder",
                        self.second.as_ref().map(|s| {
                            div()
                                .flex()
                                .gap(r(6.0))
                                .child(div().flex_1().min_w_0().child(TextField::new(s).mono().target("Project folder")))
                                .child(Button::new("browse-new", "Browse…").icon(IconName::FolderOpen).on_click(move |_: &ClickEvent, window, cx| {
                                    entity.update(cx, |this, cx| this.browse(true, window, cx))
                                }))
                                .into_any_element()
                        }),
                        cx,
                    ))
                    .footer(cancel_button)
                    .footer(confirm_button("Create project", ready))
                    .into_any_element()
            }
            Dialog::OpenProject { .. } => {
                let recent = studio.session.recent.clone();
                let ready = !Self::text(&self.first, cx).is_empty();
                let browse = entity.clone();
                ui::Dialog::new("open-project", "Open project")
                    .description("A project folder holds its model and history.")
                    .child(field(
                        "Folder",
                        self.first.as_ref().map(|s| {
                            div()
                                .flex()
                                .gap(r(6.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(TextField::new(s).mono().target("Open folder")),
                                )
                                .child(
                                    Button::new("browse-open", "Browse…")
                                        .icon(IconName::FolderOpen)
                                        .on_click(move |_: &ClickEvent, window, cx| {
                                            browse.update(cx, |this, cx| {
                                                this.browse(false, window, cx)
                                            })
                                        }),
                                )
                                .into_any_element()
                        }),
                        cx,
                    ))
                    .when(!recent.is_empty(), |this| {
                        this.child(ui::section_header("Recent projects", cx)).child(
                            div()
                                .flex()
                                .flex_col()
                                .children(recent.into_iter().enumerate().map(|(index, folder)| {
                                    let studio = self.studio.clone();
                                    let name = folder
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_default();
                                    let path = folder.display().to_string();
                                    div()
                                        .id(("recent", index))
                                        .h(r(36.0))
                                        .px(r(8.0))
                                        .mx(r(-8.0))
                                        .flex()
                                        .items_center()
                                        .gap(r(10.0))
                                        .rounded(r(crate::tokens::radius::CONTROL + 2.0))
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.hover))
                                        .on_click(move |_: &ClickEvent, _, cx| {
                                            let folder = folder.clone();
                                            studio.act(cx, |studio| {
                                                studio.dialog = None;
                                                studio.open_project(&folder);
                                            })
                                        })
                                        .child(
                                            icon(IconName::Folder)
                                                .size(14.0)
                                                .color(theme.text_muted),
                                        )
                                        .child(div().font_weight(theme::MEDIUM).child(name))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .whitespace_nowrap()
                                                .text_size(r(theme::text::XS))
                                                .font_family(theme::MONO)
                                                .text_color(theme.text_faint)
                                                .child(path),
                                        )
                                })),
                        )
                    })
                    .footer(cancel_button)
                    .footer(confirm_button("Open", ready))
                    .into_any_element()
            }
            Dialog::Create {
                kind,
                definition,
                parent,
                ..
            } => {
                let owner = match parent {
                    Parent::Element(id) => studio
                        .project
                        .as_ref()
                        .and_then(|p| p.state().tree().effective_name(*id).map(str::to_string))
                        .unwrap_or_else(|| "element".into()),
                    Parent::Document(_) => "the top level".into(),
                };
                let (kind, definition) = (*kind, *definition);
                let usage_kind = kind.element_kind(false).keyword();
                let definition_kind = kind.element_kind(true).keyword();
                let studio = self.studio.clone();
                ui::Dialog::new(
                    "create",
                    format!("Create {}", kind.element_kind(definition).keyword()),
                )
                .description(format!("Inside {owner}"))
                .when(kind != CreateKind::Interface, |this| {
                    this.child(
                        Segmented::new("create-kind", usize::from(definition))
                            .choice(None, usage_kind)
                            .choice(None, definition_kind)
                            .on_choose(move |index, _, cx| {
                                studio.act(cx, |studio| {
                                    if let Some(Dialog::Create { definition, .. }) =
                                        &mut studio.dialog
                                    {
                                        *definition = index == 1;
                                    }
                                    studio.mark(Dirty::OVERLAY);
                                })
                            }),
                    )
                })
                .child(field(
                    "Name",
                    self.first
                        .as_ref()
                        .map(|s| TextField::new(s).target("Name").into_any_element()),
                    cx,
                ))
                .footer(cancel_button)
                .footer(confirm_button("Create", true))
                .into_any_element()
            }
            Dialog::Checkpoint { .. } => ui::Dialog::new("checkpoint", "Checkpoint")
                .description("Record the current model in the history, with a message.")
                .child(field(
                    "Message",
                    self.first.as_ref().map(|s| {
                        TextField::new(s)
                            .target("Checkpoint message")
                            .into_any_element()
                    }),
                    cx,
                ))
                .footer(cancel_button)
                .footer(confirm_button("Record checkpoint", true))
                .into_any_element(),
            Dialog::MoveTo { .. } => {
                let owners = self.owners(cx);
                let chosen = self.chosen;
                ui::Dialog::new("move-to", "Move to…")
                    .description("Choose the element's new owner; references keep pointing at it.")
                    .child(
                        TextField::new(self.first.as_ref().expect("Move to has a field"))
                            .leading(IconName::Search)
                            .target("Owner"),
                    )
                    .child(
                        div()
                            .id("move-to-owners")
                            .max_h(r(280.0))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .children(owners.into_iter().enumerate().map(|(index, (_, label))| {
                                let entity = entity.clone();
                                div()
                                    .id(("owner", index))
                                    .h(r(28.0))
                                    .px(r(8.0))
                                    .flex()
                                    .items_center()
                                    .rounded(r(crate::tokens::radius::CONTROL))
                                    .font_family(theme::MONO)
                                    .text_size(r(theme::text::SM))
                                    .cursor_pointer()
                                    .when(index == chosen, |this| this.bg(theme.accent.soft))
                                    .hover(|style| style.bg(theme.hover))
                                    .on_click(move |_: &ClickEvent, window, cx| {
                                        entity.update(cx, |this, cx| {
                                            this.chosen = index;
                                            this.confirm(window, cx);
                                        })
                                    })
                                    .child(label)
                            })),
                    )
                    .footer(cancel_button)
                    .footer(confirm_button("Move", true))
                    .into_any_element()
            }
            Dialog::Confirm {
                change,
                question,
                locked,
                ..
            } => {
                let title = if locked.is_empty() {
                    "Shared definition"
                } else {
                    "Locked"
                };
                let names: Vec<SharedString> = studio
                    .project
                    .as_ref()
                    .map(|p| {
                        let tree = p.state().tree();
                        locked
                            .iter()
                            .map(|id| SharedString::from(tree.qualified_name(*id)))
                            .collect()
                    })
                    .unwrap_or_default();
                div()
                    .key_context("Dialog")
                    .track_focus(&self.focus)
                    .size_full()
                    .child(
                        ui::Dialog::new("confirm", title)
                            .width(480.0)
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap(r(10.0))
                                    .child(
                                        icon(IconName::Lock).size(16.0).color(theme.warning.text),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .line_height(r(20.0))
                                            .child(question.clone()),
                                    ),
                            )
                            .when(!names.is_empty(), |this| {
                                this.child(div().flex().flex_wrap().gap(r(4.0)).children(
                                    names.into_iter().map(|name| {
                                        ui::Chip::new(name)
                                            .mono()
                                            .icon(IconName::Lock)
                                            .tone(ui::Tone::Warning)
                                    }),
                                ))
                            })
                            .child(
                                div()
                                    .p(r(10.0))
                                    .rounded(r(crate::tokens::radius::CONTROL + 2.0))
                                    .bg(theme.inset)
                                    .border_1()
                                    .border_color(theme.separator)
                                    .text_size(r(theme::text::SM))
                                    .text_color(theme.text_secondary)
                                    .child(change.description.clone()),
                            )
                            .footer(cancel_button)
                            .footer(confirm_button("Change it", true)),
                    )
                    .into_any_element()
            }
            Dialog::Rename { .. } => div().into_any_element(),
        };
        div()
            .id("dialogs")
            .key_context("Dialog")
            .absolute()
            .inset_0()
            .on_action(cx.listener(|this, _: &Cancel, window, cx| this.cancel(window, cx)))
            .on_action(cx.listener(|this, _: &Confirm, window, cx| this.confirm(window, cx)))
            .on_action(cx.listener(|this, _: &Next, _, cx| {
                this.chosen += 1;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Previous, _, cx| {
                this.chosen = this.chosen.saturating_sub(1);
                cx.notify();
            }))
            .child(body)
            .into_any_element()
    }
}

/// A labelled field of a dialog.
fn field(label: &'static str, control: Option<gpui::AnyElement>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(r(6.0))
        .child(
            div()
                .text_size(r(theme::text::SM))
                .font_weight(theme::MEDIUM)
                .text_color(theme.text_secondary)
                .child(label),
        )
        .children(control)
}
