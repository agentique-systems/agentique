//! Where controls are drawn, and what they are (C-53, ROADMAP §4.16): every
//! button, field, tab, choice and named row records its bounds and what an
//! agent needs to know about it (role, label, value, enabled, selected,
//! focused) when it is painted. The control interface reads this to tell
//! agents what is on screen and to put their input where the Operator's
//! would go; the scripted journeys (`--features automation`) click and type
//! through the same records.
//!
//! Records are kept per **region**. The docked columns are cached GPUI views:
//! a column that did not change is not painted again, so its records stay;
//! one that is painted again replaces its records; one that is not shown is
//! left out ([`begin_frame`]). Everything else is painted every frame, so its
//! records start afresh each frame. Recording costs one map entry per drawn
//! control and nothing waits for it.
use gpui::{AnyElement, Bounds, IntoElement, Pixels, SharedString, Styled};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// The cached regions: their records last until they are painted again or
/// leave the screen.
pub const CACHED: [&str; 3] = ["outline", "inspector", "conversation"];

/// The region of everything painted every frame that names no region.
pub const ROOT: &str = "root";

/// What an agent knows about one control.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    /// Stable within a screen: the control's element id, or its name.
    pub id: SharedString,
    /// `button`, `field`, `tab`, `option`, `item`, `link`, `dialog`, `area`.
    pub role: &'static str,
    pub label: SharedString,
    pub value: Option<SharedString>,
    pub enabled: bool,
    pub selected: bool,
    pub focused: bool,
}

impl Control {
    pub fn new(role: &'static str, label: impl Into<SharedString>) -> Control {
        let label = label.into();
        Control {
            id: label.clone(),
            role,
            label,
            value: None,
            enabled: true,
            selected: false,
            focused: false,
        }
    }
    pub fn id(mut self, id: impl Into<SharedString>) -> Control {
        self.id = id.into();
        self
    }
    pub fn value(mut self, value: impl Into<SharedString>) -> Control {
        self.value = Some(value.into());
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Control {
        self.enabled = enabled;
        self
    }
    pub fn selected(mut self, selected: bool) -> Control {
        self.selected = selected;
        self
    }
    pub fn focused(mut self, focused: bool) -> Control {
        self.focused = focused;
        self
    }
}

/// A control as it was last drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub control: Control,
    pub bounds: Bounds<Pixels>,
    pub region: &'static str,
}

#[derive(Default)]
struct Registry {
    regions: BTreeMap<&'static str, Vec<Drawn>>,
    /// The regions being painted, innermost last.
    stack: Vec<&'static str>,
    /// The cached regions on screen this frame.
    mounted: Vec<&'static str>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// A frame begins (the workspace renders): records of everything painted
/// every frame start afresh; of the cached regions, those on screen keep
/// theirs until they are painted again.
pub fn begin_frame(mounted: &[&'static str]) {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.regions.retain(|name, _| CACHED.contains(name));
        registry.stack.clear();
        registry.mounted = mounted.to_vec();
    });
}

fn with_canvas(record: impl Fn(Bounds<Pixels>) + 'static) -> AnyElement {
    gpui::canvas(move |bounds, _, _| record(bounds), |_, _, _, _| {})
        .absolute()
        .inset_0()
        .into_any_element()
}

/// Begins a region: put it first in the region's root element (which must
/// be `relative()`), and [`region_end`] last. Painting the region again
/// replaces its records.
pub fn region(name: &'static str) -> AnyElement {
    with_canvas(move |_| {
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry.regions.insert(name, Vec::new());
            registry.stack.push(name);
        })
    })
}

/// `element` as the region `name`, for a cached view's render: the
/// region's records are replaced only when the view is painted again.
pub fn regioned(name: &'static str, element: impl IntoElement) -> gpui::Div {
    use gpui::ParentElement;
    gpui::div()
        .size_full()
        .child(region(name))
        .child(element)
        .child(region_end())
}

/// Ends the region begun last.
pub fn region_end() -> AnyElement {
    with_canvas(|_| {
        REGISTRY.with(|registry| {
            registry.borrow_mut().stack.pop();
        })
    })
}

/// A child that records its parent's bounds as `control`; the parent must
/// be `relative()`.
pub fn control(control: Control) -> AnyElement {
    with_canvas(move |bounds| {
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let region = registry.stack.last().copied().unwrap_or(ROOT);
            registry.regions.entry(region).or_default().push(Drawn {
                control: control.clone(),
                bounds,
                region,
            });
        })
    })
}

/// A named control with nothing more to say (a list row, a tab).
pub fn target(name: impl Into<SharedString>) -> AnyElement {
    control(Control::new("item", name))
}

/// Records bounds measured elsewhere (the Surface's viewport).
pub fn record(name: &str, bounds: Bounds<Pixels>) {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let region = registry.stack.last().copied().unwrap_or(ROOT);
        let name = SharedString::from(name.to_string());
        registry.regions.entry(region).or_default().push(Drawn {
            control: Control::new("area", name),
            bounds,
            region,
        });
    })
}

/// Every control on screen now: the regions painted this frame and the
/// cached regions on screen, in painting order.
pub fn drawn() -> Vec<Drawn> {
    REGISTRY.with(|registry| {
        let registry = registry.borrow();
        registry
            .regions
            .iter()
            .filter(|(name, _)| !CACHED.contains(name) || registry.mounted.contains(name))
            .flat_map(|(_, controls)| controls.iter().cloned())
            .collect()
    })
}

/// Where the control with this id or label was drawn last (the last one
/// drawn when several share it: the topmost).
pub fn find(name: &str) -> Option<Bounds<Pixels>> {
    drawn()
        .into_iter()
        .rev()
        .find(|d| d.control.id == name || d.control.label == name)
        .map(|d| d.bounds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(name: &str) -> Drawn {
        Drawn {
            control: Control::new("button", name.to_string()),
            bounds: Bounds::default(),
            region: ROOT,
        }
    }

    #[test]
    fn records_of_cached_regions_last_while_they_are_on_screen() {
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry
                .regions
                .insert("outline", vec![draw("Outline row")]);
            registry.regions.insert(ROOT, vec![draw("Settings")]);
            registry.regions.insert("dialog", vec![draw("Confirm")]);
        });
        begin_frame(&["outline"]);
        // The root's and the dialog's records start afresh; the outline,
        // cached and on screen, keeps its own.
        let names: Vec<String> = drawn().iter().map(|d| d.control.id.to_string()).collect();
        assert_eq!(names, vec!["Outline row".to_string()]);
        assert!(find("Outline row").is_some());
        begin_frame(&[]);
        assert!(
            drawn().is_empty(),
            "a hidden column's controls are not on screen"
        );
        begin_frame(&["outline"]);
        assert_eq!(
            drawn().len(),
            1,
            "and are back when it is shown again unchanged"
        );
    }
}
