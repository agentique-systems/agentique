//! Key caps (§3.2 Primitives): a shortcut shown the same way in buttons,
//! tooltips, menus, the palette and Settings.
use crate::ui::theme::{self, ActiveTheme, r};
use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div};

/// A shortcut such as `Ctrl+Shift+Z` as a row of caps.
#[derive(IntoElement)]
pub struct KeyCaps {
    shortcut: SharedString,
    on_solid: bool,
}

impl KeyCaps {
    pub fn new(shortcut: impl Into<SharedString>) -> KeyCaps {
        KeyCaps {
            shortcut: shortcut.into(),
            on_solid: false,
        }
    }
    /// On a primary or danger button.
    pub fn on_solid(mut self, on_solid: bool) -> KeyCaps {
        self.on_solid = on_solid;
        self
    }
}

/// The keys of a shortcut: `Ctrl+Shift+Z` is `Ctrl`, `Shift`, `Z`; a lone
/// `+` stays a key.
pub fn keys(shortcut: &str) -> Vec<&str> {
    if shortcut == "+" {
        return vec!["+"];
    }
    let mut keys = Vec::new();
    let mut rest = shortcut;
    while let Some(index) = rest.find('+') {
        if index == 0 {
            // "Ctrl++": the key itself is a plus.
            keys.push("+");
            rest = &rest[1..];
            continue;
        }
        keys.push(&rest[..index]);
        rest = &rest[index + 1..];
    }
    if !rest.is_empty() {
        keys.push(rest);
    }
    keys
}

impl RenderOnce for KeyCaps {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (bg, fg, border) = if self.on_solid {
            (
                gpui::hsla(0.0, 0.0, 1.0, 0.16),
                theme.accent.on_solid,
                gpui::hsla(0.0, 0.0, 1.0, 0.12),
            )
        } else {
            (theme.inset, theme.text_muted, theme.separator)
        };
        div()
            .flex()
            .items_center()
            .gap(r(2.0))
            .children(keys(&self.shortcut).into_iter().map(|key| {
                div()
                    .min_w(r(16.0))
                    .h(r(16.0))
                    .px(r(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(r(crate::tokens::radius::TAG))
                    .bg(bg)
                    .border_1()
                    .border_color(border)
                    .text_color(fg)
                    .text_size(r(theme::text::XS))
                    .font_weight(theme::MEDIUM)
                    .line_height(r(16.0))
                    .child(SharedString::from(key.to_string()))
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::keys;

    #[test]
    fn shortcuts_split_into_keys_and_a_plus_stays_a_key() {
        assert_eq!(keys("Ctrl+Shift+Z"), ["Ctrl", "Shift", "Z"]);
        assert_eq!(keys("+"), ["+"]);
        assert_eq!(keys("Ctrl++"), ["Ctrl", "+"]);
        assert_eq!(keys("F2"), ["F2"]);
    }
}
