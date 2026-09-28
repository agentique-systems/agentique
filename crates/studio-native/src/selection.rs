//! What the Operator has selected, by identity. Survives edits while the
//! selected elements exist.
use agq_studio_scene::{ElementId, Scene, SceneTarget};
use std::collections::BTreeSet;

/// The Surface is one element; the window's click count cannot tell two
/// quick clicks on different cards apart, so double clicks are checked here.
#[derive(Default)]
pub struct CanvasClicks {
    last: Option<(u64, SceneTarget, [f32; 2], f64)>,
}
impl CanvasClicks {
    pub fn click(
        &mut self,
        generation: u64,
        target: Option<&SceneTarget>,
        position: [f32; 2],
        time: f64,
    ) -> bool {
        let same = self.last.as_ref().is_some_and(
            |(previous_generation, previous_target, previous_position, previous_time)| {
                *previous_generation == generation
                    && Some(previous_target) == target
                    && (0.0..=0.35).contains(&(time - previous_time))
                    && (position[0] - previous_position[0])
                        .hypot(position[1] - previous_position[1])
                        <= 6.0
            },
        );
        self.last = target.map(|target| (generation, target.clone(), position, time));
        same
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selection {
    pub targets: BTreeSet<SceneTarget>,
    pub primary: Option<SceneTarget>,
}

impl Selection {
    pub fn select(&mut self, target: SceneTarget, extend: bool) {
        if !extend {
            self.targets.clear();
        }
        if extend && self.targets.contains(&target) {
            self.targets.remove(&target);
            self.primary = self.targets.last().cloned();
        } else {
            self.targets.insert(target.clone());
            self.primary = Some(target);
        }
    }
    pub fn clear(&mut self) {
        self.targets.clear();
        self.primary = None;
    }
    /// The model element of the primary selection: a card, a port, or the
    /// element an edge stands for.
    pub fn element(&self, scene: &Scene) -> Option<ElementId> {
        match self.primary.as_ref()? {
            SceneTarget::Edge(id) => {
                scene
                    .edges
                    .iter()
                    .find(|e| e.semantic.id == *id)?
                    .semantic
                    .element
            }
            other => other.element_id(),
        }
    }
    /// The model elements of every selected target.
    pub fn elements(&self, scene: &Scene) -> Vec<ElementId> {
        self.targets
            .iter()
            .filter_map(|target| match target {
                SceneTarget::Edge(id) => {
                    scene
                        .edges
                        .iter()
                        .find(|e| e.semantic.id == *id)?
                        .semantic
                        .element
                }
                other => other.element_id(),
            })
            .collect()
    }
    pub fn contains(&self, id: ElementId) -> bool {
        self.targets
            .iter()
            .any(|target| target.element_id() == Some(id))
    }
    /// Keeps the targets that still exist in the new scene.
    pub fn reconcile(&mut self, scene: &Scene) {
        self.targets
            .retain(|target| scene.target_bounds(target).is_some());
        if self
            .primary
            .as_ref()
            .is_none_or(|target| !self.targets.contains(target))
        {
            self.primary = self.targets.last().cloned();
        }
    }
    pub fn replace(&mut self, targets: impl IntoIterator<Item = SceneTarget>) {
        self.targets = targets.into_iter().collect();
        self.primary = self.targets.last().cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_studio_scene::{SceneOptions, fixtures};

    #[test]
    fn double_click_requires_same_object_context_and_pointer_location() {
        let mut clicks = CanvasClicks::default();
        let a = SceneTarget::Node(fixtures::id(1));
        let b = SceneTarget::Node(fixtures::id(2));
        assert!(!clicks.click(1, Some(&a), [10.0, 10.0], 0.0));
        assert!(!clicks.click(1, Some(&b), [10.0, 10.0], 0.1));
        assert!(!clicks.click(2, Some(&b), [10.0, 10.0], 0.2));
        assert!(!clicks.click(2, Some(&b), [90.0, 10.0], 0.3));
        assert!(clicks.click(2, Some(&b), [91.0, 10.0], 0.4));
        assert!(!clicks.click(2, None, [91.0, 10.0], 0.5));
        assert!(!clicks.click(2, Some(&b), [91.0, 10.0], 0.6));
        assert!(!clicks.click(2, Some(&b), [91.0, 10.0], 1.2));
    }

    #[test]
    fn selection_survives_while_the_element_exists() {
        let scene =
            Scene::build(&fixtures::architecture(), &SceneOptions::default(), None).unwrap();
        let first = scene.nodes[0].id();
        let mut selection = Selection::default();
        selection.select(SceneTarget::Node(first), false);
        selection.reconcile(&scene);
        assert_eq!(selection.element(&scene), Some(first));
        selection.select(SceneTarget::Node(fixtures::id(u64::MAX >> 20)), false);
        selection.reconcile(&scene);
        assert!(selection.primary.is_none());
    }
}
