//! The Studio's icons: Lucide (ISC, with MIT for the icons derived from
//! Feather; `assets/icons/LICENSE-LUCIDE`), drawn from vectors at 16 points
//! with a 1.75 stroke, used sparingly (§3.2 Iconography). Each icon is
//! compiled in; `Assets` serves them and the fonts to GPUI.
use crate::ui::theme::r;
use gpui::{
    App, AssetSource, Hsla, InteractiveElement, IntoElement, RenderOnce, Result, SharedString,
    Styled, Window, svg,
};
use std::borrow::Cow;

macro_rules! icons {
    ($($variant:ident => $file:literal),* $(,)?) => {
        /// An icon of the set.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum IconName {
            $($variant),*
        }

        impl IconName {
            /// Every icon, in the order of the set.
            pub const ALL: &'static [IconName] = &[$(IconName::$variant),*];

            pub fn path(self) -> &'static str {
                match self {
                    $(IconName::$variant => concat!("icons/", $file, ".svg")),*
                }
            }
        }

        fn icon_bytes(path: &str) -> Option<&'static [u8]> {
            match path {
                $(concat!("icons/", $file, ".svg") => Some(include_bytes!(concat!("../../assets/icons/", $file, ".svg"))),)*
                _ => None,
            }
        }

        #[cfg(test)]
        const ALL: &[IconName] = &[$(IconName::$variant),*];
    };
}

icons! {
    Search => "search",
    Command => "command",
    Settings => "settings",
    Sliders => "sliders-horizontal",
    PanelLeft => "panel-left",
    PanelRight => "panel-right",
    Maximize => "maximize-2",
    Minimize => "minimize-2",
    Fit => "scan",
    Locate => "locate",
    ZoomIn => "zoom-in",
    ZoomOut => "zoom-out",
    Plus => "plus",
    Minus => "minus",
    Close => "x",
    Check => "check",
    ChevronRight => "chevron-right",
    ChevronDown => "chevron-down",
    ChevronUp => "chevron-up",
    ChevronLeft => "chevron-left",
    ChevronsUpDown => "chevrons-up-down",
    ArrowUp => "arrow-up",
    ArrowRight => "arrow-right",
    Enter => "corner-down-left",
    Stop => "square",
    Copy => "copy",
    Pencil => "pencil",
    Retry => "rotate-ccw",
    Undo => "undo-2",
    Redo => "redo-2",
    Trash => "trash",
    Lock => "lock",
    Unlock => "lock-open",
    Warning => "triangle-alert",
    Alert => "circle-alert",
    CircleCheck => "circle-check",
    CircleX => "circle-x",
    CircleDot => "circle-dot",
    Circle => "circle",
    Info => "info",
    Clock => "clock",
    Checkpoint => "git-commit-horizontal",
    Branch => "git-branch",
    Compare => "git-compare-arrows",
    Part => "box",
    Parts => "boxes",
    Port => "plug",
    Item => "package",
    Attribute => "variable",
    Interface => "cable",
    Requirement => "clipboard-check",
    Folder => "folder",
    FolderOpen => "folder-open",
    FolderPlus => "folder-plus",
    File => "file-text",
    Connection => "link-2",
    Graph => "network",
    Architecture => "layers",
    Outline => "list-tree",
    Requirements => "list-checks",
    Conversation => "message-square",
    Assistant => "sparkle",
    Thinking => "brain",
    Tool => "wrench",
    Eye => "eye",
    Keyboard => "keyboard",
    Palette => "palette",
    Sun => "sun",
    Moon => "moon",
    Monitor => "monitor",
    Refresh => "refresh-cw",
    External => "external-link",
    Key => "key-round",
    Model => "cpu",
    Cost => "coins",
    More => "ellipsis",
    Grip => "grip-vertical",
    Focus => "focus",
    Map => "map",
    Send => "arrow-up",
    Definition => "square-dashed",
    Component => "component",
    Satisfy => "shield-check",
    Move => "move",
    Pin => "pin",
    PinOff => "pin-off",
    Question => "circle-question-mark",
    List => "list",
    Code => "code-xml",
    Table => "table-2",
    Help => "circle-question-mark",
    Save => "save",
    Loader => "loader-circle",
}

/// Serves the icons and the fonts to GPUI.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(icon_bytes(path).map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

/// The fonts of the type scale, compiled in (§3.2 Type).
pub fn fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        // Inter at the three weights the tokens use, instanced from Inter
        // Variable: GPUI's text system on Windows does not select weights of
        // a variable font.
        Cow::Borrowed(include_bytes!("../../assets/fonts/Inter-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/Inter-Medium.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/Inter-SemiBold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/JetBrainsMonoNL-Regular.ttf").as_slice()),
    ]
}

/// An icon at a size, in a colour (the current text colour when not given).
#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    size: f32,
    color: Option<Hsla>,
    /// A colour while the pointer is over the named group (a button).
    hover: Option<(SharedString, Hsla)>,
}

impl Icon {
    pub fn new(name: IconName) -> Icon {
        Icon {
            name,
            size: 16.0,
            color: None,
            hover: None,
        }
    }
    /// Size in points (12, 14, 16 or 20).
    pub fn size(mut self, points: f32) -> Icon {
        self.size = points;
        self
    }
    pub fn color(mut self, color: Hsla) -> Icon {
        self.color = Some(color);
        self
    }
    /// The colour while the pointer is over `group`.
    pub fn hover_color(mut self, group: impl Into<SharedString>, color: Hsla) -> Icon {
        self.hover = Some((group.into(), color));
        self
    }
}

pub fn icon(name: IconName) -> Icon {
    Icon::new(name)
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // An svg does not inherit the text colour: it is always given one.
        let color = self
            .color
            .unwrap_or_else(|| crate::ui::ActiveTheme::theme(cx).text_secondary);
        let element = svg()
            .path(self.name.path())
            .size(r(self.size))
            .flex_none()
            .text_color(color);
        match self.hover {
            Some((group, hover)) => {
                element.group_hover(group, move |style| style.text_color(hover))
            }
            None => element,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_compiled_in() {
        for icon in ALL {
            assert!(icon_bytes(icon.path()).is_some(), "{icon:?}");
        }
    }
}
