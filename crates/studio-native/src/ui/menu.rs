//! Menus (§3.2 Command: context menu with the palette's actions): a list of
//! actions with their icons and shortcuts, driven by the keyboard (Up, Down,
//! Enter, Escape) and the pointer. The same menu serves the context menu on
//! the Surface, the model picker and every drop-down.
use crate::ui::{
    icon::{IconName, icon},
    keycap::KeyCaps,
    theme::{self, ActiveTheme, r},
};
use gpui::{
    App, ClickEvent, Context, DismissEvent, ElementId, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, actions, div, prelude::FluentBuilder, px,
};
use std::rc::Rc;

actions!(menu, [SelectNext, SelectPrevious, Confirm, Cancel]);

const CONTEXT: &str = "Menu";

pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", SelectNext, Some(CONTEXT)),
        KeyBinding::new("up", SelectPrevious, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ]);
}

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

pub enum MenuItem {
    Action {
        label: SharedString,
        icon: Option<IconName>,
        shortcut: Option<&'static str>,
        checked: bool,
        /// Why it cannot run now; shown instead of running it.
        disabled: Option<&'static str>,
        danger: bool,
        handler: Handler,
    },
    Header(SharedString),
    Separator,
    /// A line of explanation that cannot be chosen.
    Note(SharedString),
}

impl MenuItem {
    pub fn action(
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> MenuItem {
        MenuItem::Action {
            label: label.into(),
            icon: None,
            shortcut: None,
            checked: false,
            disabled: None,
            danger: false,
            handler: Rc::new(handler),
        }
    }
    pub fn icon(mut self, name: IconName) -> MenuItem {
        if let MenuItem::Action { icon, .. } = &mut self {
            *icon = Some(name);
        }
        self
    }
    pub fn shortcut(mut self, keys: &'static str) -> MenuItem {
        if let MenuItem::Action { shortcut, .. } = &mut self {
            *shortcut = (!keys.is_empty()).then_some(keys);
        }
        self
    }
    pub fn checked(mut self, value: bool) -> MenuItem {
        if let MenuItem::Action { checked, .. } = &mut self {
            *checked = value;
        }
        self
    }
    pub fn disabled(mut self, reason: Option<&'static str>) -> MenuItem {
        if let MenuItem::Action { disabled, .. } = &mut self {
            *disabled = reason;
        }
        self
    }
    pub fn danger(mut self) -> MenuItem {
        if let MenuItem::Action { danger, .. } = &mut self {
            *danger = true;
        }
        self
    }
    fn selectable(&self) -> bool {
        matches!(self, MenuItem::Action { disabled: None, .. })
    }
}

pub struct Menu {
    items: Vec<MenuItem>,
    selected: Option<usize>,
    focus: FocusHandle,
    min_width: f32,
}

impl EventEmitter<DismissEvent> for Menu {}

impl Focusable for Menu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Menu {
    pub fn new(items: Vec<MenuItem>, cx: &mut Context<Menu>) -> Menu {
        Menu {
            items,
            selected: None,
            focus: cx.focus_handle(),
            min_width: 200.0,
        }
    }

    pub fn min_width(mut self, width: f32) -> Menu {
        self.min_width = width;
        self
    }

    fn step(&mut self, forward: bool, cx: &mut Context<Menu>) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let mut index = match (self.selected, forward) {
            (Some(index), true) => (index + 1) % count,
            (Some(index), false) => (index + count - 1) % count,
            (None, true) => 0,
            (None, false) => count - 1,
        };
        for _ in 0..count {
            if self.items[index].selectable() {
                self.selected = Some(index);
                cx.notify();
                return;
            }
            index = if forward {
                (index + 1) % count
            } else {
                (index + count - 1) % count
            };
        }
    }

    fn run(&mut self, index: usize, window: &mut Window, cx: &mut Context<Menu>) {
        if let Some(MenuItem::Action {
            handler,
            disabled: None,
            ..
        }) = self.items.get(index)
        {
            let handler = handler.clone();
            cx.emit(DismissEvent);
            handler(window, cx);
        }
    }
}

impl Render for Menu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let selected = self.selected;
        let min_width = self.min_width;
        div()
            .id("menu")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .role(gpui::Role::Menu)
            .on_action(cx.listener(|menu, _: &SelectNext, _, cx| menu.step(true, cx)))
            .on_action(cx.listener(|menu, _: &SelectPrevious, _, cx| menu.step(false, cx)))
            .on_action(cx.listener(|menu, _: &Confirm, window, cx| {
                if let Some(index) = menu.selected {
                    menu.run(index, window, cx);
                }
            }))
            .on_action(cx.listener(|_, _: &Cancel, _, cx| cx.emit(DismissEvent)))
            .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(DismissEvent)))
            .min_w(r(min_width))
            .max_w(r(420.0))
            .p(r(4.0))
            .rounded(r(crate::tokens::radius::MENU))
            .bg(theme.overlay)
            .border_1()
            .border_color(theme.border)
            .shadow(theme.shadow_overlay())
            .font_family(theme::SANS)
            .text_size(r(theme::text::BASE))
            .flex()
            .flex_col()
            .children(self.items.iter().enumerate().map(|(index, item)| {
                match item {
                    MenuItem::Separator => div()
                        .my(r(4.0))
                        .mx(r(-4.0))
                        .h(theme::hairline())
                        .bg(theme.separator)
                        .into_any_element(),
                    MenuItem::Header(title) => div()
                        .px(r(8.0))
                        .pt(r(6.0))
                        .pb(r(2.0))
                        .text_size(r(theme::text::XS))
                        .font_weight(theme::SEMIBOLD)
                        .text_color(theme.text_muted)
                        .child(title.to_uppercase())
                        .into_any_element(),
                    MenuItem::Note(text) => div()
                        .px(r(8.0))
                        .py(r(4.0))
                        .text_size(r(theme::text::SM))
                        .text_color(theme.text_muted)
                        .child(text.clone())
                        .into_any_element(),
                    MenuItem::Action {
                        label,
                        icon: glyph,
                        shortcut,
                        checked,
                        disabled,
                        danger,
                        ..
                    } => {
                        let highlighted = selected == Some(index);
                        let enabled = disabled.is_none();
                        let text = if !enabled {
                            theme.text_faint
                        } else if *danger {
                            theme.danger.text
                        } else {
                            theme.text
                        };
                        div()
                            .id(ElementId::NamedInteger("menu-item".into(), index as u64))
                            .role(gpui::Role::MenuItem)
                            .aria_label(label.clone())
                            .h(r(28.0))
                            .px(r(8.0))
                            .flex()
                            .items_center()
                            .gap(r(8.0))
                            .rounded(r(crate::tokens::radius::CONTROL))
                            .text_color(text)
                            .when(highlighted && enabled, |this| this.bg(theme.hover))
                            .when(enabled, |this| {
                                this.cursor_pointer()
                                    .hover(|style| style.bg(theme.hover))
                                    .on_click(cx.listener(
                                        move |menu, _: &ClickEvent, window, cx| {
                                            menu.run(index, window, cx)
                                        },
                                    ))
                            })
                            .when_some(*disabled, |this, reason| {
                                let tooltip = crate::ui::tooltip::text(reason, None);
                                this.tooltip(move |window, cx| tooltip(window, cx))
                            })
                            .child(
                                div()
                                    .w(r(16.0))
                                    .flex()
                                    .justify_center()
                                    .when_some(*glyph, |this, name| {
                                        this.child(icon(name).size(14.0).color(if enabled {
                                            theme.text_muted
                                        } else {
                                            theme.text_faint
                                        }))
                                    })
                                    .when(*checked && glyph.is_none(), |this| {
                                        this.child(
                                            icon(IconName::Check)
                                                .size(14.0)
                                                .color(theme.accent.text),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .whitespace_nowrap()
                                    .child(label.clone()),
                            )
                            .when(*checked && glyph.is_some(), |this| {
                                this.child(
                                    icon(IconName::Check).size(14.0).color(theme.accent.text),
                                )
                            })
                            .when_some(*shortcut, |this, keys| {
                                this.child(div().pl(px(12.0)).child(KeyCaps::new(keys)))
                            })
                            .into_any_element()
                    }
                }
            }))
    }
}
