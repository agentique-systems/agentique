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
//! left out ([`begin_frame`]). A cached view inside a cached view is its own
//! region, so the outer one painting again does not lose the inner one's
//! records. Everything else is painted every frame, so its records start
//! afresh each frame. A control records its bounds, the part of them that
//! is visible (within its scroll area's mask) and that mask, in painting
//! order, so the topmost of several is the last. Recording costs one map
//! entry per drawn control and nothing waits for it.
use gpui::{AnyElement, Bounds, IntoElement, Pixels, SharedString, Styled, Window};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// The cached regions: their records last until they are painted again or
/// leave the screen.
pub const CACHED: [&str; 4] = ["outline", "left-body", "inspector", "conversation"];

/// Cached regions inside others: on screen when the outer one is.
const NESTED: [(&str, &str); 1] = [("left-body", "outline")];

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
    /// The part of it that is visible; none when its scroll area or column
    /// hides it.
    pub shown: Option<Bounds<Pixels>>,
    /// What its scroll area or column shows: where scrolling reveals it.
    pub clip: Bounds<Pixels>,
    pub region: &'static str,
    /// When it was painted: later is on top.
    pub order: u64,
}

#[derive(Default)]
struct Registry {
    regions: BTreeMap<&'static str, Vec<Drawn>>,
    /// The regions being painted, innermost last.
    stack: Vec<&'static str>,
    /// The cached regions on screen this frame.
    mounted: Vec<&'static str>,
    /// The next control's painting order (it never goes back, so a cached
    /// column's records stay under what is painted after them).
    next: u64,
}

impl Registry {
    fn push(&mut self, control: Control, bounds: Bounds<Pixels>, clip: Bounds<Pixels>) {
        let region = self.stack.last().copied().unwrap_or(ROOT);
        let order = self.next;
        self.next += 1;
        let shown = bounds.intersect(&clip);
        self.regions.entry(region).or_default().push(Drawn {
            control,
            bounds,
            shown: (!shown.is_empty()).then_some(shown),
            clip,
            region,
            order,
        });
    }
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
        let mut mounted = mounted.to_vec();
        for (inner, outer) in NESTED {
            if mounted.contains(&outer) {
                mounted.push(inner);
            }
        }
        registry.mounted = mounted;
    });
}

fn with_canvas(record: impl Fn(Bounds<Pixels>, &Window) + 'static) -> AnyElement {
    gpui::canvas(
        move |bounds, window, _| record(bounds, window),
        |_, _, _, _| {},
    )
    .absolute()
    .inset_0()
    .into_any_element()
}

/// Begins a region: put it first in the region's root element (which must
/// be `relative()`), and [`region_end`] last. Painting the region again
/// replaces its records.
pub fn region(name: &'static str) -> AnyElement {
    with_canvas(move |_, _| {
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
    with_canvas(|_, _| {
        REGISTRY.with(|registry| {
            registry.borrow_mut().stack.pop();
        })
    })
}

/// A child that records its parent's bounds as `control`; the parent must
/// be `relative()`.
pub fn control(control: Control) -> AnyElement {
    with_canvas(move |bounds, window| {
        let clip = window.content_mask().bounds;
        REGISTRY.with(|registry| registry.borrow_mut().push(control.clone(), bounds, clip))
    })
}

/// A named control with nothing more to say (a list row, a tab).
pub fn target(name: impl Into<SharedString>) -> AnyElement {
    control(Control::new("item", name))
}

/// Records bounds measured elsewhere (the Surface's viewport).
pub fn record(name: &str, bounds: Bounds<Pixels>) {
    let control = Control::new("area", SharedString::from(name.to_string()));
    REGISTRY.with(|registry| registry.borrow_mut().push(control, bounds, bounds))
}

/// Overlays above the docked columns and the centre: dialogs, the palette,
/// and on top the menus and popovers painted at the root.
fn layer(region: &str) -> u8 {
    match region {
        ROOT => 3,
        "palette" => 2,
        "dialog" => 1,
        _ => 0,
    }
}

/// Every control on screen now: the regions painted this frame and the
/// cached regions on screen, in painting order (the topmost last).
pub fn drawn() -> Vec<Drawn> {
    REGISTRY.with(|registry| {
        let registry = registry.borrow();
        let mut all: Vec<Drawn> = registry
            .regions
            .iter()
            .filter(|(name, _)| !CACHED.contains(name) || registry.mounted.contains(name))
            .flat_map(|(_, controls)| controls.iter().cloned())
            .collect();
        // Overlays above everything; the docked columns and the centre, which
        // do not overlap, in a fixed order, so which of two same-named
        // controls wins does not depend on which column was painted again.
        all.sort_by_key(|d| {
            (
                layer(d.region),
                (layer(d.region) == 0).then_some(d.region),
                d.order,
            )
        });
        all
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

    fn draw(name: &str, order: u64) -> Drawn {
        Drawn {
            control: Control::new("button", name.to_string()),
            bounds: Bounds::default(),
            shown: None,
            clip: Bounds::default(),
            region: ROOT,
            order,
        }
    }

    #[test]
    fn records_of_cached_regions_last_while_they_are_on_screen() {
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry
                .regions
                .insert("outline", vec![draw("Outline row", 0)]);
            registry.regions.insert(ROOT, vec![draw("Settings", 1)]);
            registry.regions.insert("dialog", vec![draw("Confirm", 2)]);
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

    #[test]
    fn the_topmost_of_controls_sharing_a_name_is_found_whatever_their_regions() {
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry.regions.clear();
            // A dialog (painted last) over the Surface: "dialog" sorts
            // before "surface" by name, but it is on top.
            registry.regions.insert("surface", vec![draw("Delete", 5)]);
            registry.regions.insert("dialog", vec![draw("Delete", 9)]);
            registry.regions.insert("left-body", vec![draw("Row", 1)]);
        });
        begin_frame(&["outline"]);
        let all = drawn();
        assert_eq!(
            all.iter()
                .map(|d| d.control.id.to_string())
                .collect::<Vec<_>>(),
            vec!["Row".to_string()],
            "a frame begins: the nested cached region of a shown column stays"
        );
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry.regions.insert("surface", vec![draw("Delete", 5)]);
            registry.regions.insert("dialog", vec![draw("Delete", 9)]);
        });
        let topmost = drawn()
            .into_iter()
            .rev()
            .find(|d| d.control.id == "Delete")
            .map(|d| d.order);
        assert_eq!(topmost, Some(9), "the dialog's, painted last");
    }
}
