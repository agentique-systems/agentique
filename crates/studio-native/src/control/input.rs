//! Input given to the window as the platform gives it (pointer, keys,
//! text), for the control interface's actions and the scripted journeys:
//! hit testing, focus, key bindings and text input all run as they do for
//! the Operator.
use gpui::{
    App, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PlatformInput, ScrollDelta, ScrollWheelEvent, TouchPhase, Window,
    point, px,
};

/// A key in GPUI's syntax, pressed and released. As on Windows, a key that
/// types nothing printable (Enter, Escape, a shortcut) inserts no text.
pub fn press(key: &str, window: &mut Window, cx: &mut App) -> Result<(), String> {
    let keystroke = Keystroke::parse(key).map_err(|e| format!("{key}: {e}"))?;
    window.dispatch_event(
        PlatformInput::KeyDown(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        }),
        cx,
    );
    window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke }), cx);
    Ok(())
}

/// Types one character, as the key that produces it.
pub fn type_char(c: char, window: &mut Window, cx: &mut App) {
    let keystroke = Keystroke {
        modifiers: Modifiers {
            shift: c.is_uppercase(),
            ..Modifiers::default()
        },
        key: if c == ' ' {
            "space".into()
        } else {
            c.to_lowercase().to_string()
        },
        key_char: Some(c.to_string()),
    };
    window.dispatch_keystroke(keystroke, cx);
}

/// Half a click: the pointer moves there, then the button goes down (or,
/// still held while it moves, up).
pub fn click(at: gpui::Point<gpui::Pixels>, down: bool, window: &mut Window, cx: &mut App) {
    move_to(at, (!down).then_some(MouseButton::Left), window, cx);
    pointer_button(at, down, window, cx);
}

pub fn move_to(
    at: gpui::Point<gpui::Pixels>,
    pressed: Option<MouseButton>,
    window: &mut Window,
    cx: &mut App,
) {
    window.dispatch_event(
        PlatformInput::MouseMove(MouseMoveEvent {
            position: at,
            pressed_button: pressed,
            modifiers: Modifiers::default(),
        }),
        cx,
    );
}

/// The pointer moves with the button held (a drag), for the journeys.
#[cfg(feature = "automation")]
pub fn drag_to(at: gpui::Point<gpui::Pixels>, window: &mut Window, cx: &mut App) {
    move_to(at, Some(MouseButton::Left), window, cx);
}

pub fn pointer_button(
    at: gpui::Point<gpui::Pixels>,
    down: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let input = if down {
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: at,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        })
    } else {
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: at,
            modifiers: Modifiers::default(),
            click_count: 1,
        })
    };
    window.dispatch_event(input, cx);
}

/// Text typed into the focused field, key by key.
pub fn type_text(text: &str, window: &mut Window, cx: &mut App) {
    for c in text.chars() {
        if c == '\n' {
            let _ = press("enter", window, cx);
        } else {
            type_char(c, window, cx);
        }
    }
}

/// The wheel turned over `at`: `dy` pixels down (negative: up).
pub fn scroll(at: gpui::Point<gpui::Pixels>, dy: f32, window: &mut Window, cx: &mut App) {
    window.dispatch_event(
        PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-dy))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        }),
        cx,
    );
}
