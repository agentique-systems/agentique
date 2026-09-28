//! Where named controls are drawn, for the scripted journeys
//! (`--features automation`): a control records its bounds when it is
//! painted, and the journey clicks or types there through the window, as
//! the Operator would. Without the feature, nothing is recorded.
use gpui::{AnyElement, IntoElement, SharedString};

#[cfg(feature = "automation")]
thread_local! {
    static TARGETS: std::cell::RefCell<std::collections::HashMap<SharedString, gpui::Bounds<gpui::Pixels>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// A child that records its parent's bounds under `name`; the parent must
/// be `relative()`.
#[cfg(feature = "automation")]
pub fn target(name: impl Into<SharedString>) -> AnyElement {
    use gpui::Styled;
    let name = name.into();
    gpui::canvas(
        move |bounds, _, _| {
            TARGETS.with(|targets| targets.borrow_mut().insert(name.clone(), bounds));
        },
        |_, _, _, _| {},
    )
    .absolute()
    .inset_0()
    .into_any_element()
}

#[cfg(not(feature = "automation"))]
#[inline(always)]
pub fn target(_: impl Into<SharedString>) -> AnyElement {
    gpui::Empty.into_any_element()
}

/// Where `name` was drawn last.
#[cfg(feature = "automation")]
pub fn find(name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
    TARGETS.with(|targets| targets.borrow().get(name).copied())
}

/// Records bounds measured elsewhere (the Surface).
#[cfg(feature = "automation")]
pub fn record(name: &str, bounds: gpui::Bounds<gpui::Pixels>) {
    TARGETS.with(|targets| {
        targets
            .borrow_mut()
            .insert(SharedString::from(name.to_string()), bounds)
    });
}

#[cfg(not(feature = "automation"))]
#[inline(always)]
pub fn record(_: &str, _: gpui::Bounds<gpui::Pixels>) {}
