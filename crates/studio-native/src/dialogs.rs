//! The Studio's dialogs over the workspace: New project and Open project
//! (with the folder picker of Windows), Create, Checkpoint, Move to, the
//! confirmation of a change to locked elements or a shared definition, and
//! the Library's (C-49): Specialise, Create building block from selection,
//! Save to My Library, Override here, and a conflict when using a block.
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
    Specialize,
    ExtractBlock,
    SaveToLibrary,
    Override,
    LibraryConflict,
    NewScenario,
    TrustLocal,
    ConfirmLive,
    LinkCode,
    Implement,
    ReviewTask,
}

fn kind_of(dialog: &Dialog) -> Option<Kind> {
    Some(match dialog {
        Dialog::NewProject { .. } => Kind::NewProject,
        Dialog::OpenProject { .. } => Kind::OpenProject,
        Dialog::Create { .. } => Kind::Create,
        Dialog::Checkpoint { .. } => Kind::Checkpoint,
        Dialog::MoveTo { .. } => Kind::MoveTo,
        Dialog::Confirm { .. } => Kind::Confirm,
        Dialog::Specialize { .. } => Kind::Specialize,
        Dialog::ExtractBlock { .. } => Kind::ExtractBlock,
        Dialog::SaveToLibrary { .. } => Kind::SaveToLibrary,
        Dialog::Override { .. } => Kind::Override,
        Dialog::LibraryConflict { .. } => Kind::LibraryConflict,
        Dialog::NewScenario { .. } => Kind::NewScenario,
        Dialog::TrustLocal => Kind::TrustLocal,
        Dialog::ConfirmLive => Kind::ConfirmLive,
        Dialog::LinkCode { .. } => Kind::LinkCode,
        Dialog::Implement { .. } => Kind::Implement,
        Dialog::ReviewTask { .. } => Kind::ReviewTask,
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
            Some(Dialog::Specialize { name, .. }) => Some((name.clone(), None)),
            Some(Dialog::ExtractBlock {
                definition, usage, ..
            }) => Some((definition.clone(), Some(usage.clone()))),
            Some(Dialog::SaveToLibrary { category, .. }) => Some((category.clone(), None)),
            Some(Dialog::Override { value, .. }) => Some((value.clone(), None)),
            Some(Dialog::NewScenario { name, .. }) => Some((name.clone(), None)),
            Some(Dialog::LinkCode { path, symbol, .. }) => {
                Some((path.clone(), Some(symbol.clone())))
            }
            Some(Dialog::Implement { instructions, .. }) => Some((instructions.clone(), None)),
            _ => None,
        };
        if let Some((first, second)) = values {
            let placeholder = match wanted {
                Some(Kind::NewProject) => "NewSystem",
                Some(Kind::OpenProject) => "C:\\Users\\you\\Agentique\\MySystem",
                Some(Kind::Create) => "Name (Enter for a default)",
                Some(Kind::Checkpoint) => "What changed?",
                Some(Kind::MoveTo) => "Find the new owner",
                Some(Kind::Specialize) => "SessionStore",
                Some(Kind::ExtractBlock) => "Backend",
                Some(Kind::SaveToLibrary) => "Storage",
                Some(Kind::Override) => "60",
                Some(Kind::NewScenario) => "ShortenAllowed",
                Some(Kind::LinkCode) => "src/store.rs",
                Some(Kind::Implement) => "Anything else the worker should know (optional)",
                _ => "",
            };
            let first = self.input(&first, placeholder, window, cx);
            let focus = first.read(cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
            self.first = Some(first);
            if let Some(second) = second {
                let placeholder = match wanted {
                    Some(Kind::ExtractBlock) => "backend (Enter for a default)",
                    Some(Kind::LinkCode) => "LinkStore (optional)",
                    _ => "Folder",
                };
                self.second = Some(self.input(&second, placeholder, window, cx));
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
            } else if matches!(studio.dialog, Some(Dialog::Implement { .. })) {
                studio.dialog = None;
                studio.answer_proposal(Err("The Operator did not start the task.".into()));
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
                    if let Some(sample) = sample {
                        studio.create_sample(&folder, &first, sample);
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
                Dialog::Specialize {
                    definition, usage, ..
                } => {
                    if first.is_empty() {
                        studio.dialog = Some(dialog);
                        return;
                    }
                    studio.specialize(definition, &first, usage);
                }
                Dialog::ExtractBlock { ref extraction, .. } => {
                    if first.is_empty() || !extraction.blockers.is_empty() {
                        studio.dialog = Some(dialog);
                        return;
                    }
                    let extraction = extraction.clone();
                    if !studio.extract(&extraction, &first, &second) {
                        studio.dialog = Some(Dialog::ExtractBlock {
                            extraction,
                            definition: first,
                            usage: second,
                        });
                    }
                }
                Dialog::SaveToLibrary {
                    definition,
                    conflicts,
                    ..
                } => {
                    if first.is_empty() {
                        studio.dialog = Some(Dialog::SaveToLibrary {
                            definition,
                            category: first,
                            conflicts,
                        });
                        return;
                    }
                    if let Err(conflicts) =
                        studio.save_to_library(definition, &first, !conflicts.is_empty())
                    {
                        studio.dialog = Some(Dialog::SaveToLibrary {
                            definition,
                            category: first,
                            conflicts,
                        });
                    }
                }
                Dialog::Override {
                    owner, ref path, ..
                } => match crate::panels::parse_value(&first) {
                    Some(value) => {
                        let path = path.clone();
                        studio.override_feature(owner, &path, agq_library::Override::Value(value));
                    }
                    None => studio.dialog = Some(dialog),
                },
                Dialog::LibraryConflict { request, .. } => {
                    studio.resolve_conflict(request, agq_library::Resolution::Rename)
                }
                Dialog::NewScenario { subject, .. } => {
                    if first.is_empty() {
                        studio.dialog = Some(dialog);
                        return;
                    }
                    studio.create_scenario(subject, &first);
                }
                Dialog::TrustLocal => {
                    let mut choice = studio.execution_choice();
                    choice.trusted = true;
                    studio.set_execution_choice(choice);
                }
                Dialog::ConfirmLive => {
                    studio.runs.confirm_live = false;
                    studio.start_run(agq_simulation::Mode::Live);
                }
                Dialog::Implement { element, .. } => {
                    let started = studio.start_implementation(element, &first);
                    if let Err(why) = &started {
                        studio.status = why.clone();
                    }
                    studio.answer_proposal(started);
                }
                Dialog::ReviewTask {
                    ref job, ref diff, ..
                } => {
                    let (job, diff) = (job.clone(), diff.clone());
                    if let Err(problem) = studio.integrate_task(&job) {
                        studio.dialog = Some(Dialog::ReviewTask {
                            job,
                            problem: Some(problem),
                            diff,
                        });
                    }
                }
                Dialog::LinkCode { element, kind, .. } => {
                    if first.is_empty() {
                        studio.dialog = Some(dialog);
                        return;
                    }
                    let kind = agq_implementation::LinkKind::ALL[kind.min(8)];
                    let symbol = (!second.is_empty()).then_some(second.as_str());
                    studio.link_code(element, kind, &first, symbol);
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
                let sample = *sample;
                let chooser = self.studio.clone();
                ui::Dialog::new("new-project", if sample.is_some() { "Start from the URL shortener" } else { "New project" })
                    .description("A project is a folder; the model is saved in it as SysML text, with its history in git.")
                    .when_some(sample, |this, sample| {
                        use crate::studio::Sample;
                        this.child(
                            Segmented::new("sample-kind", usize::from(sample == Sample::Screening))
                                .choice(None, "The URL shortener")
                                .choice(None, "With AI screening and scenarios")
                                .on_choose(move |index, _, cx| {
                                    chooser.act(cx, |studio| {
                                        if let Some(Dialog::NewProject { sample, .. }) = &mut studio.dialog {
                                            *sample = Some(if index == 1 { Sample::Screening } else { Sample::UrlShortener });
                                        }
                                        studio.mark(Dirty::OVERLAY);
                                    })
                                }),
                        )
                    })
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
                shared,
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
                            .when_some(*shared, |this, (_, usage)| {
                                let studio = self.studio.clone();
                                this.footer(
                                    Button::new("confirm-specialize", "Specialise instead")
                                        .on_click(move |_: &ClickEvent, _, cx| {
                                            studio.act(cx, |studio| {
                                                studio.dialog = None;
                                                studio.start_specialize(usage);
                                                studio.mark(Dirty::ALL);
                                            })
                                        }),
                                )
                            })
                            .footer(confirm_button(
                                if shared.is_some() {
                                    "Change the definition"
                                } else {
                                    "Change it"
                                },
                                true,
                            )),
                    )
                    .into_any_element()
            }
            Dialog::Specialize {
                definition, usage, ..
            } => {
                let tree = studio.project.as_ref().map(|p| p.state().tree());
                let name = |id: agq_language::ElementId| {
                    tree.and_then(|t| t.effective_name(id).map(str::to_string))
                        .unwrap_or_default()
                };
                let general = name(*definition);
                let mut description = format!(
                    "A new definition that specialises {general}: it has everything {general} has, and you can override or add to it without changing {general} or its other usages."
                );
                if let Some(usage) = usage {
                    description.push_str(&format!(" {} will use it.", name(*usage)));
                }
                let ready = !Self::text(&self.first, cx).is_empty();
                ui::Dialog::new("specialize", format!("Specialise {general}"))
                    .width(480.0)
                    .description(description)
                    .child(field(
                        "Name",
                        self.first.as_ref().map(|s| {
                            TextField::new(s)
                                .target("Specialisation name")
                                .into_any_element()
                        }),
                        cx,
                    ))
                    .footer(cancel_button)
                    .footer(confirm_button("Specialise", ready))
                    .into_any_element()
            }
            Dialog::ExtractBlock { extraction, .. } => {
                let tree = studio.project.as_ref().map(|p| p.state().tree());
                let name = |id: agq_language::ElementId| {
                    tree.and_then(|t| t.effective_name(id).map(str::to_string))
                        .unwrap_or_default()
                };
                let parts: Vec<String> = extraction.parts.iter().map(|p| name(*p)).collect();
                let blocked = !extraction.blockers.is_empty();
                let ready = !Self::text(&self.first, cx).is_empty() && !blocked;
                ui::Dialog::new("extract", "Create building block from selection")
                    .width(560.0)
                    .description(format!(
                        "The selected parts become a reusable definition, and one usage of it takes their place in {}. They keep their identity; connections from outside pass through the new definition's ports. One change: undo reverts it.",
                        name(extraction.owner)
                    ))
                    .child(block_list("Parts", parts, IconName::Part, cx))
                    .child(block_list(
                        "Ports it will expose",
                        extraction
                            .boundary
                            .iter()
                            .map(|b| {
                                format!(
                                    "{} → {}.{} ({} connection{})",
                                    b.name,
                                    name(b.part),
                                    name(b.port),
                                    b.connections.len(),
                                    if b.connections.len() == 1 { "" } else { "s" }
                                )
                            })
                            .collect(),
                        IconName::Port,
                        cx,
                    ))
                    .when(!extraction.internal.is_empty(), |this| {
                        this.child(ui::inline_message(
                            ui::Tone::Neutral,
                            format!(
                                "{} connection(s) between the parts move inside with them.",
                                extraction.internal.len()
                            ),
                            cx,
                        ))
                    })
                    .children(extraction.blockers.iter().map(|blocker| {
                        ui::inline_message(ui::Tone::Warning, blocker.clone(), cx)
                    }))
                    .child(field(
                        "Definition name",
                        self.first
                            .as_ref()
                            .map(|s| TextField::new(s).target("Block name").into_any_element()),
                        cx,
                    ))
                    .child(field(
                        "Usage name",
                        self.second
                            .as_ref()
                            .map(|s| TextField::new(s).target("Usage name").into_any_element()),
                        cx,
                    ))
                    .footer(cancel_button)
                    .footer(confirm_button("Create building block", ready))
                    .into_any_element()
            }
            Dialog::SaveToLibrary {
                definition,
                conflicts,
                ..
            } => {
                let tree = studio.project.as_ref().map(|p| p.state().tree());
                let title = tree
                    .and_then(|t| t.effective_name(*definition).map(str::to_string))
                    .unwrap_or_default();
                let replacing = !conflicts.is_empty();
                ui::Dialog::new("save-to-library", format!("Save {title} to My Library"))
                    .width(500.0)
                    .description("My Library keeps it, with the definitions it needs, for use in any project. A project that uses it gets its own copy, so changing My Library later changes no project.")
                    .child(field(
                        "Category",
                        self.first
                            .as_ref()
                            .map(|s| TextField::new(s).target("Category").into_any_element()),
                        cx,
                    ))
                    .children(conflicts.iter().map(|conflict| {
                        ui::inline_message(ui::Tone::Warning, format!("My Library: {conflict}"), cx)
                    }))
                    .footer(cancel_button)
                    .footer(confirm_button(
                        if replacing { "Replace in My Library" } else { "Save" },
                        true,
                    ))
                    .into_any_element()
            }
            Dialog::Override { owner, path, .. } => {
                let tree = studio.project.as_ref().map(|p| p.state().tree());
                let name = |id: agq_language::ElementId| {
                    tree.and_then(|t| t.effective_name(id).map(str::to_string))
                        .unwrap_or_default()
                };
                let feature: Vec<String> = path.iter().map(|f| name(*f)).collect();
                let valid = crate::panels::parse_value(&Self::text(&self.first, cx)).is_some();
                ui::Dialog::new("override", format!("Override {}", feature.join(".")))
                    .width(460.0)
                    .description(format!(
                        "A value for {} in {} only: the definition and its other usages keep theirs.",
                        feature.join("."),
                        name(*owner)
                    ))
                    .child(field(
                        "Value",
                        self.first
                            .as_ref()
                            .map(|s| TextField::new(s).mono().target("Override value").into_any_element()),
                        cx,
                    ))
                    .footer(cancel_button)
                    .footer(confirm_button("Override", valid))
                    .into_any_element()
            }
            Dialog::LibraryConflict { conflicts, .. } => {
                let use_existing = {
                    let studio = self.studio.clone();
                    Button::new("conflict-use-existing", "Use the project's").on_click(
                        move |_: &ClickEvent, _, cx| {
                            studio.act(cx, |studio| {
                                if let Some(Dialog::LibraryConflict { request, .. }) =
                                    studio.dialog.take()
                                {
                                    studio.resolve_conflict(
                                        request,
                                        agq_library::Resolution::UseExisting,
                                    );
                                }
                                studio.mark(Dirty::ALL);
                            })
                        },
                    )
                };
                div()
                    .key_context("Dialog")
                    .track_focus(&self.focus)
                    .size_full()
                    .child(
                        ui::Dialog::new("library-conflict", "Already in the project")
                            .width(540.0)
                            .description("The project already has definitions with these names, with other content. Nothing is overwritten.")
                            .children(conflicts.iter().map(|conflict| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(r(4.0))
                                    .child(ui::Chip::new(SharedString::from(conflict.qualified_name.clone())).mono().tone(ui::Tone::Warning))
                                    .child(
                                        div()
                                            .text_size(r(theme::text::SM))
                                            .text_color(theme.text_secondary)
                                            .child(conflict.difference.clone()),
                                    )
                            }))
                            .footer(cancel_button)
                            .footer(use_existing)
                            .footer(confirm_button("Copy under another name", true)),
                    )
                    .into_any_element()
            }
            Dialog::NewScenario { subject, .. } => {
                let subject = studio
                    .project
                    .as_ref()
                    .and_then(|p| {
                        p.state()
                            .tree()
                            .effective_name(*subject)
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                let ready = !Self::text(&self.first, cx).is_empty();
                ui::Dialog::new("new-scenario", "New scenario")
                    .width(460.0)
                    .description(format!(
                        "A scenario about {subject}: what goes in, how its parts or agents answer, and what must come out. Add those in the Run panel."
                    ))
                    .child(field(
                        "Name",
                        self.first.as_ref().map(|s| {
                            TextField::new(s).target("Scenario name").into_any_element()
                        }),
                        cx,
                    ))
                    .footer(cancel_button)
                    .footer(confirm_button("Create scenario", ready))
                    .into_any_element()
            }
            Dialog::TrustLocal => {
                let choice = studio.implementation.choice.unwrap_or_default();
                let isolation = studio.isolation();
                let studio_entity = self.studio.clone();
                let off = self.studio.clone();
                div()
                    .key_context("Dialog")
                    .track_focus(&self.focus)
                    .size_full()
                    .child(
                        ui::Dialog::new("trust-local", "Trusted-local execution")
                            .width(520.0)
                            .description("Runs the project's builds, tests and harness on this computer, as you.")
                            .child(plain_list(
                                "What Agentique does",
                                vec![
                                    "Runs only Cargo: build, check, test, run, metadata".into(),
                                    "In the code folder, each copy with its own target folder".into(),
                                    "Without API keys, tokens or passwords in the environment".into(),
                                    "Offline, unless you allow the network below".into(),
                                    "Stops a process tree at its time limit or when you stop it".into(),
                                ],
                                IconName::CircleCheck,
                                cx,
                            ))
                            .child(
                                div()
                                    .p(r(10.0))
                                    .rounded(r(crate::tokens::radius::CONTROL + 2.0))
                                    .bg(theme.inset)
                                    .border_1()
                                    .border_color(theme.separator)
                                    .text_size(r(theme::text::SM))
                                    .line_height(r(18.0))
                                    .text_color(theme.text_secondary)
                                    .child(format!("{isolation} Code that runs can do anything your account can: turn this on only for code you trust.")),
                            )
                            .child(
                                ui::Switch::new("trust-network", choice.network, "Allow Cargo to use the network (to fetch dependencies)")
                                    .on_toggle(move |on, _, cx| {
                                        studio_entity.act(cx, |studio| {
                                            let mut choice = studio.execution_choice();
                                            choice.network = on;
                                            studio.implementation.choice = Some(choice);
                                            studio.mark(Dirty::OVERLAY);
                                        })
                                    }),
                            )
                            .footer(cancel_button)
                            .when(choice.trusted, |this| {
                                this.footer(Button::new("trust-off", "Turn off").on_click(
                                    move |_: &ClickEvent, _, cx| {
                                        off.act(cx, |studio| {
                                            studio.dialog = None;
                                            let mut choice = studio.execution_choice();
                                            choice.trusted = false;
                                            studio.set_execution_choice(choice);
                                            studio.mark(Dirty::ALL);
                                        })
                                    },
                                ))
                            })
                            .footer(confirm_button(
                                if choice.trusted { "Keep it on" } else { "Allow on this computer" },
                                true,
                            )),
                    )
                    .into_any_element()
            }
            Dialog::ConfirmLive => {
                let choice = crate::live::live_choice(studio);
                let has_key = crate::live::live_model(studio).is_some();
                let calls = studio.runs.selected.and_then(|scenario| {
                    let store = studio.run_store()?;
                    let result = store.latest(scenario.raw(), agq_simulation::Mode::Model)?;
                    Some(
                        result
                            .trace
                            .iter()
                            .filter(|e| e.kind == agq_simulation::EventKind::AgentCalled)
                            .count(),
                    )
                });
                let samples = agq_simulation::Request::new(agq_simulation::Mode::Live)
                    .samples
                    .max(5);
                let model = choice
                    .as_ref()
                    .map(|(p, m)| format!("{}/{m}", p.id()))
                    .unwrap_or_else(|| "no model".into());
                let calls_line = match calls {
                    Some(n) => format!(
                        "About {} model call(s): {samples} samples × {n} agent call(s) in the model run",
                        samples as usize * n.max(1)
                    ),
                    None => format!(
                        "{samples} samples; run the scenario in model mode first to see how many calls each makes"
                    ),
                };
                div()
                    .key_context("Dialog")
                    .track_focus(&self.focus)
                    .size_full()
                    .child(
                        ui::Dialog::new("confirm-live", "Evaluate live")
                            .width(500.0)
                            .description("Agents answer from a real model instead of the scenario's stand-ins. Each call is sent to the provider and costs money.")
                            .child(plain_list(
                                "What will be sent",
                                vec![
                                    format!("Model: {model}"),
                                    calls_line,
                                    "Each agent's instructions and the item it is asked about".into(),
                                    "The cost is estimated from the provider's usage; it is shown with the result".into(),
                                ],
                                IconName::Agent,
                                cx,
                            ))
                            .when(!has_key, |this| {
                                this.child(ui::inline_message(
                                    ui::Tone::Warning,
                                    "No key is set for this provider: add one in Settings › Providers.",
                                    cx,
                                ))
                            })
                            .footer(cancel_button)
                            .footer(confirm_button("Evaluate live", has_key)),
                    )
                    .into_any_element()
            }
            Dialog::LinkCode { element, kind, .. } => {
                let name = studio
                    .project
                    .as_ref()
                    .and_then(|p| {
                        p.state()
                            .tree()
                            .effective_name(*element)
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                let chosen = *kind;
                let chooser = self.studio.clone();
                let mut kinds = Segmented::new("link-kind", chosen);
                for k in agq_implementation::LinkKind::ALL.iter().take(5) {
                    kinds = kinds.choice(None, k.label());
                }
                let ready = !Self::text(&self.first, cx).is_empty();
                ui::Dialog::new("link-code", format!("Link code to {name}"))
                    .width(520.0)
                    .description("A path in the code folder, and optionally a type, function or test in it. Links are kept with the model in model/links.json.")
                    .child(kinds.on_choose(move |index, _, cx| {
                        chooser.act(cx, |studio| {
                            if let Some(Dialog::LinkCode { kind, .. }) = &mut studio.dialog {
                                *kind = index;
                            }
                            studio.mark(Dirty::OVERLAY);
                        })
                    }))
                    .child(field(
                        "Path",
                        self.first.as_ref().map(|s| {
                            TextField::new(s).mono().target("Link path").into_any_element()
                        }),
                        cx,
                    ))
                    .child(field(
                        "Symbol",
                        self.second.as_ref().map(|s| {
                            TextField::new(s).mono().target("Link symbol").into_any_element()
                        }),
                        cx,
                    ))
                    .footer(cancel_button)
                    .footer(confirm_button("Link", ready))
                    .into_any_element()
            }
            Dialog::Implement { element, .. } => {
                let name = studio
                    .project
                    .as_ref()
                    .and_then(|p| {
                        p.state()
                            .tree()
                            .effective_name(*element)
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                let blocker = studio.task_blocker();
                let model = studio.conversation.model_name.clone();
                let repository = studio
                    .implementation_repository()
                    .map(|r| r.display().to_string())
                    .unwrap_or_else(|| "no code folder".into());
                ui::Dialog::new("implement", format!("Implement {name} with the Assistant"))
                    .width(560.0)
                    .description("A worker implements it from the model in a worktree of its own. Nothing reaches your code until you review the patch and integrate it.")
                    .child(plain_list(
                        "What happens",
                        vec![
                            format!("A worktree of {repository} on a new branch; your working copy is left alone"),
                            "The worker reads the model's contracts and scenarios, writes code there and links it".into(),
                            format!("It builds and checks at most {} times, and stops sooner when it makes no progress", agq_assistant::worker::MAX_ROUNDS),
                            "If the model is wrong or not enough, it asks you instead of working around it".into(),
                            "The Studio then checks the worktree itself and shows you the patch".into(),
                            format!("Model: {model}; it costs what its calls cost"),
                        ],
                        IconName::Patch,
                        cx,
                    ))
                    .child(field(
                        "Instructions",
                        self.first.as_ref().map(|s| {
                            TextField::new(s).target("Task instructions").into_any_element()
                        }),
                        cx,
                    ))
                    .when_some(blocker.clone(), |this, why| {
                        this.child(ui::inline_message(ui::Tone::Warning, why, cx))
                    })
                    .footer(cancel_button)
                    .footer(confirm_button("Start the task", blocker.is_none()))
                    .into_any_element()
            }
            Dialog::ReviewTask { job, problem, diff } => {
                let Some((found, outcome)) = studio.task_outcome(job) else {
                    return div().into_any_element();
                };
                let verification = &outcome.verification;
                let failures = verification.failures();
                let discard = {
                    let studio = self.studio.clone();
                    let job = job.clone();
                    Button::new("task-discard", "Discard").danger().on_click(
                        move |_: &ClickEvent, _, cx| {
                            let job = job.clone();
                            studio.act(cx, |studio| {
                                studio.dialog = None;
                                studio.discard_task(&job);
                                studio.mark(Dirty::ALL);
                            })
                        },
                    )
                };
                let later = Button::new("task-later", "Later").on_click({
                    let entity = entity.clone();
                    move |_: &ClickEvent, window, cx| {
                        entity.update(cx, |this, cx| this.cancel(window, cx))
                    }
                });
                let theme = theme.clone();
                div()
                    .key_context("Dialog")
                    .track_focus(&self.focus)
                    .size_full()
                    .child(
                        ui::Dialog::new("review-task", format!("Review: {}", found.title))
                            .width(760.0)
                            .description(format!(
                                "{} file(s) changed in {}; the worker took {} round(s) of checks.",
                                outcome.files.len(),
                                outcome.worktree,
                                outcome.rounds
                            ))
                            .child(
                                div()
                                    .id("review-body")
                                    .max_h(r(460.0))
                                    .overflow_y_scroll()
                                    .flex()
                                    .flex_col()
                                    .gap(r(12.0))
                                    .child(ui::Banner::new(
                                        if failures == 0 { ui::Tone::Success } else { ui::Tone::Warning },
                                        if failures == 0 {
                                            "The Studio checked the worktree itself: it builds and nothing it checked fails.".to_string()
                                        } else {
                                            format!("The Studio checked the worktree itself: {failures} failing or not verified.")
                                        },
                                    ))
                                    .when_some(outcome.notice.clone(), |this, notice| {
                                        this.child(ui::inline_message(ui::Tone::Neutral, notice, cx))
                                    })
                                    .when_some(outcome.summary.clone(), |this, summary| {
                                        this.child(plain_list("The worker says (its own words)", vec![summary], IconName::Agent, cx))
                                    })
                                    .when(!outcome.contract_requests.is_empty(), |this| {
                                        this.child(plain_list(
                                            "It asks you to change the model",
                                            outcome.contract_requests.clone(),
                                            IconName::Warning,
                                            cx,
                                        ))
                                    })
                                    .child(plain_list(
                                        "What the Studio checked",
                                        verification.describe().lines().map(str::to_string).collect(),
                                        IconName::Requirement,
                                        cx,
                                    ))
                                    .when(!outcome.proposed.is_empty(), |this| {
                                        this.child(plain_list(
                                            "Links it proposes (added to the model when you integrate)",
                                            outcome
                                                .proposed
                                                .iter()
                                                .map(|l| format!("{}: {} {}{}", l.name, l.kind.label(), l.path, l.symbol.as_ref().map(|s| format!("#{s}")).unwrap_or_default()))
                                                .collect(),
                                            IconName::Connection,
                                            cx,
                                        ))
                                    })
                                    .child(plain_list(
                                        "Files",
                                        outcome
                                            .files
                                            .iter()
                                            .map(|(path, status, added, removed)| format!("{path} ({status}, +{added} −{removed})"))
                                            .collect(),
                                        IconName::Patch,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .p(r(10.0))
                                            .rounded(r(crate::tokens::radius::CONTROL + 2.0))
                                            .bg(theme.inset)
                                            .border_1()
                                            .border_color(theme.separator)
                                            .font_family(theme::MONO)
                                            .text_size(r(theme::text::XS))
                                            .line_height(r(16.0))
                                            .text_color(theme.text_secondary)
                                            .whitespace_normal()
                                            .children(diff.lines().map(|line| {
                                                let colour = match line.chars().next() {
                                                    Some('+') => theme.success.text,
                                                    Some('-') => theme.danger.text,
                                                    _ => theme.text_secondary,
                                                };
                                                div().text_color(colour).child(line.to_string())
                                            })),
                                    ),
                            )
                            .when_some(problem.clone(), |this, problem| {
                                this.child(ui::inline_message(ui::Tone::Danger, problem, cx))
                            })
                            .footer(discard)
                            .footer(later)
                            .footer(confirm_button("Integrate", !outcome.files.is_empty())),
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

/// A short labelled list in a dialog, one row per entry.
fn block_list(
    label: &'static str,
    rows: Vec<String>,
    glyph: IconName,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(r(4.0))
        .child(
            div()
                .text_size(r(theme::text::SM))
                .font_weight(theme::MEDIUM)
                .text_color(theme.text_secondary)
                .child(label),
        )
        .when(rows.is_empty(), |this| {
            this.child(
                div()
                    .text_size(r(theme::text::SM))
                    .text_color(theme.text_faint)
                    .child("None"),
            )
        })
        .children(rows.into_iter().map(|row| {
            div()
                .flex()
                .items_center()
                .gap(r(6.0))
                .text_size(r(theme::text::SM))
                .font_family(theme::MONO)
                .child(icon(glyph).size(12.0).color(theme.text_muted))
                .child(row)
        }))
}

/// A short labelled list of sentences in a dialog, wrapping.
fn plain_list(
    label: &'static str,
    rows: Vec<String>,
    glyph: IconName,
    cx: &App,
) -> impl IntoElement {
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
        .children(rows.into_iter().map(|row| {
            div()
                .flex()
                .items_start()
                .gap(r(8.0))
                .text_size(r(theme::text::SM))
                .line_height(r(18.0))
                .child(
                    div()
                        .pt(r(3.0))
                        .child(icon(glyph).size(12.0).color(theme.text_muted)),
                )
                .child(div().flex_1().min_w_0().child(row))
        }))
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
