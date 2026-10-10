//! Buttons (§3.2 Primitives): primary, secondary, ghost and danger, at 24, 28
//! and 32 points, with an icon, a shortcut hint and every state (rest, hover,
//! press, focus, disabled, selected). Behaviour (focus, Enter and Space,
//! the accessible role) is gpui-base's.
use crate::ui::{
    icon::{IconName, icon},
    keycap::KeyCaps,
    theme::{self, ActiveTheme, r},
};
use gpui::{
    AnyView, App, BoxShadow, ClickEvent, ElementId, FontWeight, Hsla, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window, div, hsla, point, prelude::FluentBuilder, px,
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Danger,
    /// A ghost button whose label is quieter until hovered (toolbars).
    Subtle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    /// 24 points.
    Small,
    /// 28 points.
    #[default]
    Medium,
    /// 32 points.
    Large,
}

impl Size {
    pub fn height(self) -> f32 {
        match self {
            Size::Small => crate::tokens::space::CONTROL_SM,
            Size::Medium => crate::tokens::space::CONTROL,
            Size::Large => crate::tokens::space::CONTROL_LG,
        }
    }
    fn text(self) -> f32 {
        match self {
            Size::Small => theme::text::SM,
            _ => theme::text::BASE,
        }
    }
    fn icon(self) -> f32 {
        match self {
            Size::Small => 14.0,
            _ => 16.0,
        }
    }
}

type Click = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type Tooltip = Rc<dyn Fn(&mut Window, &mut App) -> AnyView>;

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<IconName>,
    trailing: Option<IconName>,
    shortcut: Option<&'static str>,
    variant: Variant,
    size: Size,
    disabled: bool,
    selected: bool,
    full_width: bool,
    /// Its label may be cut short (an ellipsis) when room is short.
    truncate: bool,
    tooltip: Option<Tooltip>,
    on_click: Option<Click>,
    label_for_screen_readers: Option<SharedString>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
        Button {
            id: id.into(),
            label: Some(label.into()),
            icon: None,
            trailing: None,
            shortcut: None,
            variant: Variant::Secondary,
            size: Size::Medium,
            disabled: false,
            selected: false,
            full_width: false,
            truncate: false,
            tooltip: None,
            on_click: None,
            label_for_screen_readers: None,
        }
    }

    /// A square button showing only an icon; `label` names it for screen
    /// readers and in its tooltip.
    pub fn icon_only(
        id: impl Into<ElementId>,
        name: IconName,
        label: impl Into<SharedString>,
    ) -> Button {
        let label = label.into();
        Button {
            label: None,
            icon: Some(name),
            variant: Variant::Subtle,
            label_for_screen_readers: Some(label.clone()),
            tooltip: Some(crate::ui::tooltip::text(label, None)),
            ..Button::new(id, "")
        }
    }

    /// A fuller name for screen readers (and the journeys) than the
    /// visible label, where the label alone is ambiguous ("Send").
    pub fn accessible_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label_for_screen_readers = Some(label.into());
        self
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }
    pub fn primary(self) -> Self {
        self.variant(Variant::Primary)
    }
    /// Lets its label be cut short with an ellipsis when room is short.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }
    pub fn ghost(self) -> Self {
        self.variant(Variant::Ghost)
    }
    pub fn danger(self) -> Self {
        self.variant(Variant::Danger)
    }
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
    pub fn small(self) -> Self {
        self.size(Size::Small)
    }
    pub fn large(self) -> Self {
        self.size(Size::Large)
    }
    pub fn icon(mut self, name: IconName) -> Self {
        self.icon = Some(name);
        self
    }
    pub fn trailing(mut self, name: IconName) -> Self {
        self.trailing = Some(name);
        self
    }
    /// The shortcut shown in key caps after the label (`Ctrl+K`).
    pub fn shortcut(mut self, shortcut: &'static str) -> Self {
        self.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }
    /// A tooltip with a title and an optional shortcut.
    pub fn tooltip(
        mut self,
        title: impl Into<SharedString>,
        shortcut: Option<&'static str>,
    ) -> Self {
        self.tooltip = Some(crate::ui::tooltip::text(title.into(), shortcut));
        self
    }
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

/// The 2-point focus ring outside a control (§3.5).
pub fn focus_ring(color: Hsla) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color,
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(crate::tokens::stroke::FOCUS),
        inset: false,
    }]
}

/// A faint light along the top edge of a raised solid control.
fn highlight(strength: f32) -> BoxShadow {
    BoxShadow {
        color: hsla(0.0, 0.0, 1.0, strength),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: true,
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let height = self.size.height();
        let square = self.label.is_none();
        let (bg, fg, border, hover_bg, hover_fg, press_bg) = match self.variant {
            Variant::Primary => (
                theme.accent.solid,
                theme.accent.on_solid,
                theme.accent.solid,
                theme.accent.solid_hover,
                theme.accent.on_solid,
                theme.accent.solid,
            ),
            Variant::Danger => (
                theme.danger.solid,
                theme.danger.on_solid,
                theme.danger.solid,
                theme.danger.solid_hover,
                theme.danger.on_solid,
                theme.danger.solid,
            ),
            Variant::Secondary => (
                theme.raised,
                theme.text,
                theme.border,
                theme.hover,
                theme.text,
                theme.pressed,
            ),
            Variant::Ghost => (
                gpui::transparent_black(),
                theme.text_secondary,
                gpui::transparent_black(),
                theme.hover,
                theme.text,
                theme.pressed,
            ),
            Variant::Subtle => (
                gpui::transparent_black(),
                theme.text_muted,
                gpui::transparent_black(),
                theme.hover,
                theme.text,
                theme.pressed,
            ),
        };
        let (bg, fg) = if self.selected {
            (theme.accent.soft, theme.accent.text)
        } else {
            (bg, fg)
        };
        let solid = matches!(self.variant, Variant::Primary | Variant::Danger);
        let radius = r(crate::tokens::radius::CONTROL);
        let ring = theme.accent.solid;
        let mut shadows = Vec::new();
        if solid && !self.disabled {
            shadows.push(highlight(if theme.dark { 0.14 } else { 0.22 }));
        }
        let group = SharedString::from(format!("button-{:?}", self.id));
        let hover_icon = if self.disabled { fg } else { hover_fg };
        let truncate = self.truncate;
        let content = div()
            .flex()
            .items_center()
            .gap(r(6.0))
            .when(truncate, |this| this.min_w_0())
            .when_some(self.icon, |this, name| {
                this.child(
                    icon(name)
                        .size(self.size.icon())
                        .color(fg)
                        .hover_color(group.clone(), hover_icon),
                )
            })
            .when_some(self.label.clone(), |this, label| {
                this.child(
                    div()
                        .whitespace_nowrap()
                        .when(truncate, |this| {
                            this.min_w_0().overflow_hidden().text_ellipsis()
                        })
                        .child(label),
                )
            })
            .when_some(self.trailing, |this, name| {
                this.child(icon(name).size(14.0).color(theme.text_muted))
            })
            .when_some(self.shortcut, |this, shortcut| {
                this.child(KeyCaps::new(shortcut).on_solid(solid))
            });
        let name = self.label_for_screen_readers.clone().or(self.label.clone());
        let id_text = self.id.to_string();
        let (disabled, selected) = (self.disabled, self.selected);
        // Its own focus handle, so agents' observations say when it has the
        // focus (Space or Enter then reaches it).
        let focus = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let focused = focus.is_focused(window);
        let mut button = gpui_base::Button::new(self.id)
            .track_focus(&focus)
            .group(group)
            .relative()
            .disabled(self.disabled)
            .selected(self.selected)
            .h(r(height))
            .when(square, |this| this.w(r(height)))
            .when(!square, |this| {
                this.px(r(if self.size == Size::Small { 8.0 } else { 10.0 }))
            })
            .when(self.full_width, |this| this.w_full())
            .when(truncate, |this| this.min_w_0())
            .rounded(radius)
            .bg(bg)
            .text_color(fg)
            .text_size(r(self.size.text()))
            .font_weight(FontWeight(crate::tokens::text::MEDIUM as f32))
            .when(border != gpui::transparent_black(), |this| {
                this.border_1().border_color(border)
            })
            .shadow(shadows.clone())
            .when(!self.disabled, |this| {
                this.cursor_pointer()
                    .hover(|style| style.bg(hover_bg).text_color(hover_fg))
                    .active(|style| style.bg(press_bg))
            })
            .when(self.disabled, |this| this.opacity(0.45))
            .focus_visible(move |style| {
                let mut ring = focus_ring(ring);
                ring.extend(shadows);
                style.shadow(ring)
            })
            .child(content)
            .when_some(name, |this, name| {
                this.child(crate::ui::target::control(
                    crate::ui::target::Control::new("button", name)
                        .id(id_text)
                        .enabled(!disabled)
                        .selected(selected)
                        .focused(focused),
                ))
            });
        if let Some(label) = self.label_for_screen_readers.or(self.label) {
            button = button.accessibility_label(label);
        }
        if let Some(handler) = self.on_click {
            button = button.on_click(move |event, window, cx| handler(event, window, cx));
        }
        if let Some(tooltip) = self.tooltip {
            button = button.tooltip(move |window, cx| tooltip(window, cx));
        }
        button
    }
}
