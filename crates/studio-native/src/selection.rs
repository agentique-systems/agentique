//! Shared revision-scoped selection for the scene, outliner and inspector.
use agq_kernel::ElementId;
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::{SceneTarget, SemanticScene};
use std::collections::BTreeSet;

/// The canvas is one toolkit widget. Toolkit-wide click counts alone cannot
/// distinguish two rapid clicks on different semantic objects.
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

pub struct Selection {
    pub revision: ProjectRevisionId,
    pub targets: BTreeSet<SceneTarget>,
    pub primary: Option<SceneTarget>,
}

impl Selection {
    pub fn new(revision: ProjectRevisionId) -> Self {
        Self {
            revision,
            targets: BTreeSet::new(),
            primary: None,
        }
    }
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
    pub fn element(&self, scene: &SemanticScene) -> Option<ElementId> {
        if self.revision != scene.revision_id {
            return None;
        }
        scene.target_bounds(self.primary.as_ref()?)?;
        match self.primary.as_ref()? {
            SceneTarget::Edge(id) => {
                scene
                    .edges
                    .iter()
                    .find(|e| e.semantic.id == *id)?
                    .semantic
                    .relationship_id
            }
            other => other.element_id(),
        }
    }
    pub fn contains(&self, id: ElementId) -> bool {
        self.targets
            .iter()
            .any(|target| target.element_id() == Some(id))
    }
    /// Surviving identities retain selection, removed identities cannot bind stale inspection.
    pub fn reconcile(&mut self, scene: &SemanticScene) {
        self.revision = scene.revision_id;
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
        let a = SceneTarget::Node(ElementId::from_u128(1));
        let b = SceneTarget::Node(ElementId::from_u128(2));
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
    fn stale_revision_or_missing_target_cannot_become_an_inspector_request() {
        let projection = fixtures::architecture();
        let scene =
            SemanticScene::from_projection(&projection, &SceneOptions::default(), None).unwrap();
        let first = scene.nodes[0].id();
        let mut selection = Selection::new(ProjectRevisionId::new());
        selection.select(SceneTarget::Node(first), false);
        assert_eq!(selection.element(&scene), None);
        selection.reconcile(&scene);
        assert_eq!(selection.element(&scene), Some(first));
        selection.select(SceneTarget::Node(ElementId::from_u128(u128::MAX)), false);
        assert_eq!(selection.element(&scene), None);
        selection.reconcile(&scene);
        assert!(selection.primary.is_none());
    }
}
