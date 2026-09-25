//! Shared revision-scoped selection for the scene, outliner and inspector.
use agq_kernel::ElementId;
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::{SceneTarget, SemanticScene};
use std::collections::BTreeSet;

pub struct Selection {
    pub revision: ProjectRevisionId,
    pub targets: BTreeSet<SceneTarget>,
    pub primary: Option<SceneTarget>,
}
impl Selection {
    pub fn new(revision: ProjectRevisionId) -> Self {
        Self { revision, targets: BTreeSet::new(), primary: None }
    }
    pub fn select(&mut self, target: SceneTarget, extend: bool) {
        if !extend { self.targets.clear(); }
        if extend && self.targets.contains(&target) {
            self.targets.remove(&target);
            self.primary = self.targets.last().cloned();
        } else {
            self.targets.insert(target.clone());
            self.primary = Some(target);
        }
    }
    pub fn clear(&mut self) { self.targets.clear(); self.primary = None; }
    pub fn element(&self, scene: &SemanticScene) -> Option<ElementId> {
        match self.primary.as_ref()? {
            SceneTarget::Edge(id) => scene.edges.iter().find(|e| e.semantic.id == *id)?.semantic.relationship_id,
            other => other.element_id(),
        }
    }
    pub fn contains(&self, id: ElementId) -> bool {
        self.targets.iter().any(|target| target.element_id() == Some(id))
    }
    /// Surviving identities retain selection, removed identities cannot bind stale inspection.
    pub fn reconcile(&mut self, scene: &SemanticScene) {
        self.revision = scene.revision_id;
        self.targets.retain(|target| scene.target_bounds(target).is_some());
        if self.primary.as_ref().is_none_or(|target| !self.targets.contains(target)) {
            self.primary = self.targets.last().cloned();
        }
    }
    pub fn replace(&mut self, targets: impl IntoIterator<Item = SceneTarget>) {
        self.targets = targets.into_iter().collect();
        self.primary = self.targets.last().cloned();
    }
}
