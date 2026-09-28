//! The start screen (D1) and the first run's welcome (R-46, E1). The first
//! run shows three steps: what Agentique is, connecting a model provider (or
//! skipping it), and creating or opening a project, with the URL shortener
//! as a sample. Later starts list the recent projects.
use crate::{
    commands::{CommandId, Run},
    edit::Dialog,
    studio::{Dirty, Studio, StudioEvent},
    ui::{self, ActiveTheme, Button, IconName, Tone, icon, r, theme},
    workspace::StudioExt,
};
use gpui::{
    App, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div,
    prelude::FluentBuilder,
};

pub struct Welcome {
    studio: Entity<Studio>,
    _subscription: Subscription,
}

impl Welcome {
    pub fn new(studio: Entity<Studio>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe(&studio, |_, _, event: &StudioEvent, cx| {
            if event.0.intersects(Dirty::STATUS | Dirty::APPEARANCE | Dirty::OVERLAY | Dirty::CONVERSATION) {
                cx.notify();
            }
        });
        Welcome {
            studio,
            _subscription: subscription,
        }
    }
}

fn run(id: CommandId) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, window, cx| window.dispatch_action(Box::new(Run(id)), cx)
}

/// One numbered step of the welcome.
fn step(number: &'static str, title: &'static str, body: SharedString, actions: Vec<gpui::AnyElement>, done: bool, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .gap(r(14.0))
        .p(r(16.0))
        .rounded(r(crate::tokens::radius::CARD + 2.0))
        .bg(theme.raised)
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .size(r(26.0))
                .flex_none()
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(if done { theme.success.soft } else { theme.accent.soft })
                .text_color(if done { theme.success.text } else { theme.accent.text })
                .text_size(r(theme::text::SM))
                .font_weight(theme::SEMIBOLD)
                .child(if done {
                    icon(IconName::Check).size(14.0).color(theme.success.text).into_any_element()
                } else {
                    number.into_any_element()
                }),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(r(4.0))
                .child(div().text_size(r(theme::text::PROSE)).font_weight(theme::SEMIBOLD).child(title))
                .child(
                    div()
                        .text_size(r(theme::text::SM))
                        .line_height(r(19.0))
                        .text_color(theme.text_secondary)
                        .child(body),
                )
                .when(!actions.is_empty(), |this| {
                    this.child(div().pt(r(8.0)).flex().flex_wrap().gap(r(8.0)).children(actions))
                }),
        )
}

impl Render for Welcome {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let first_run = studio.session.recent.is_empty();
        let status = studio.status.clone();
        let key = studio.conversation.key_missing.is_none();
        let model = studio.conversation.model_name.clone();
        let recent = studio.session.recent.clone();
        let sample = {
            let studio = self.studio.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                studio.act(cx, |studio| {
                    studio.dialog = Some(Dialog::sample_project());
                    studio.mark(Dirty::OVERLAY);
                })
            }
        };
        let example = {
            let studio = self.studio.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| studio.act(cx, |studio| studio.show_fixture("architecture"))
        };
        let content = if first_run {
            div()
                .flex()
                .flex_col()
                .gap(r(12.0))
                .child(step(
                    "1",
                    "What it is",
                    "Design a system's architecture with an AI Assistant: parts, ports, interfaces and requirements on the Surface, kept as SysML v2 text in git. Every change shows where it happens and can be undone.".into(),
                    Vec::new(),
                    false,
                    cx,
                ))
                .child(step(
                    "2",
                    "Connect a model provider",
                    if key {
                        format!("A key is set: the Assistant uses {model}.").into()
                    } else {
                        "No key yet. You can skip this: everything but the Assistant works without one.".into()
                    },
                    vec![Button::new("welcome-settings", "Open Settings").icon(IconName::Key).shortcut("Ctrl+,").on_click(run(CommandId::Settings)).into_any_element()],
                    key,
                    cx,
                ))
                .child(step(
                    "3",
                    "Create or open a project",
                    "A project is a folder with its model in git. Start empty, open one, or start from the URL shortener of the walkthrough.".into(),
                    vec![
                        Button::new("welcome-new", "New project…").primary().icon(IconName::FolderPlus).on_click(run(CommandId::NewProject)).into_any_element(),
                        Button::new("welcome-open", "Open project…").icon(IconName::FolderOpen).on_click(run(CommandId::OpenProject)).into_any_element(),
                        Button::new("welcome-sample", "Start from the URL shortener").icon(IconName::Parts).on_click(sample).into_any_element(),
                    ],
                    false,
                    cx,
                ))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(r(16.0))
                .child(
                    div()
                        .flex()
                        .gap(r(8.0))
                        .child(Button::new("start-new", "New project…").primary().large().icon(IconName::FolderPlus).shortcut("Ctrl+N").on_click(run(CommandId::NewProject)))
                        .child(Button::new("start-open", "Open project…").large().icon(IconName::FolderOpen).shortcut("Ctrl+O").on_click(run(CommandId::OpenProject))),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(ui::section_header("Recent projects", cx))
                        .children(recent.into_iter().enumerate().map(|(index, folder)| {
                            let studio = self.studio.clone();
                            let name = folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            let path = folder.display().to_string();
                            div()
                                .id(("start-recent", index))
                                .h(r(44.0))
                                .px(r(10.0))
                                .mx(r(-10.0))
                                .flex()
                                .items_center()
                                .gap(r(12.0))
                                .rounded(r(crate::tokens::radius::CARD))
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.hover))
                                .role(gpui::Role::Button)
                                .aria_label(SharedString::from(format!("Open {name}")))
                                .on_click(move |_: &ClickEvent, _, cx| {
                                    let folder = folder.clone();
                                    studio.act(cx, |studio| studio.open_project(&folder))
                                })
                                .child(
                                    div()
                                        .size(r(28.0))
                                        .flex_none()
                                        .rounded(r(7.0))
                                        .bg(theme.raised)
                                        .border_1()
                                        .border_color(theme.border)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(icon(IconName::Folder).size(14.0).color(theme.text_muted)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(div().font_weight(theme::MEDIUM).child(name))
                                        .child(
                                            div()
                                                .text_size(r(theme::text::XS))
                                                .font_family(theme::MONO)
                                                .text_color(theme.text_faint)
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .whitespace_nowrap()
                                                .child(path),
                                        ),
                                )
                                .child(icon(IconName::ArrowRight).size(14.0).color(theme.text_faint))
                        })),
                )
                .into_any_element()
        };
        div()
            .id("welcome")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .items_center()
            .bg(theme.canvas)
            .child(
                div()
                    .w_full()
                    .max_w(r(640.0))
                    .px(r(24.0))
                    .pt(gpui::relative(0.12))
                    .pb(r(40.0))
                    .flex()
                    .flex_col()
                    .gap(r(24.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(r(8.0))
                            .child(
                                div()
                                    .size(r(40.0))
                                    .rounded(r(11.0))
                                    .bg(theme.accent.solid)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .shadow(theme.shadow_small())
                                    .child(icon(IconName::Component).size(22.0).color(theme.accent.on_solid)),
                            )
                            .child(
                                div()
                                    .pt(r(8.0))
                                    .text_size(r(theme::text::XXL))
                                    .font_weight(theme::SEMIBOLD)
                                    .child(if first_run { "Welcome to Agentique" } else { "Agentique" }),
                            )
                            .child(
                                div()
                                    .text_size(r(theme::text::PROSE))
                                    .text_color(theme.text_muted)
                                    .child("Design, simulate and implement systems with an AI Assistant, at the level of their architecture."),
                            ),
                    )
                    .when(!status.is_empty(), |this| this.child(ui::Banner::new(Tone::Warning, status)))
                    .child(content)
                    .child(
                        div().child(
                            Button::new("start-example", "Look at an example (read-only)")
                                .ghost()
                                .icon(IconName::Eye)
                                .on_click(example),
                        ),
                    ),
            )
    }
}
