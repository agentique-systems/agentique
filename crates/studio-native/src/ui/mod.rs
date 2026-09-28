//! The design system (ROADMAP §3.2, R-26): theme roles and type from the
//! tokens, icons, and the components every view builds from. The component
//! gallery (`--fixture components`) shows each in its states and themes.
pub mod button;
pub mod field;
pub mod icon;
pub mod keycap;
pub mod menu;
pub mod overlay;
pub mod primitives;
pub mod theme;
pub mod tooltip;

pub use button::{Button, Variant};
pub use field::{TextArea, TextField};
pub use icon::{Icon, IconName, icon};
pub use keycap::KeyCaps;
pub use menu::{Menu, MenuItem};
pub use overlay::Dialog;
pub use primitives::{
    Badge, Banner, Chip, EmptyState, Segmented, Switch, Tone, divider, divider_vertical,
    inline_message, section_header, spinner,
};
pub use theme::{ActiveTheme, Theme, r};

/// Installs the theme for the Studio's appearance, for the views and for
/// gpui-base's fields and scroll bars.
pub fn install_theme(dark: bool, contrast: bool, cx: &mut gpui::App) {
    let theme = Theme::new(dark, contrast);
    *gpui_base::Theme::global_mut(cx) = theme.base_theme();
    cx.set_global(theme);
}
