//! Presentation history contains focus and camera, never model history.
use agq_kernel::ElementId;
use agq_modeling_workspace::ProjectRevisionId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum World {
    #[default]
    System,
    Graph,
    Requirements,
    History,
}
impl World {
    pub fn title(self) -> &'static str {
        match self {
            Self::System => "System World",
            Self::Graph => "Graph World",
            Self::Requirements => "Requirements World",
            Self::History => "Design history",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Location {
    pub revision: ProjectRevisionId,
    pub world: World,
    pub focus: Option<ElementId>,
    pub center: [f32; 2],
    pub zoom: f32,
    pub presentation: crate::saved_views::SavedPresentation,
}

#[derive(Default, Debug)]
pub struct Navigation {
    entries: Vec<Location>,
    cursor: usize,
}
impl Navigation {
    pub fn latest_for_revision(&self, revision: ProjectRevisionId) -> Option<Location> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.revision == revision && entry.world != World::History)
            .cloned()
    }
    /// Layouts referenced by Back/Forward must retain the coordinate system
    /// their cameras use. Navigation itself is bounded to 128 entries.
    pub fn retained_views(&self) -> std::collections::BTreeSet<(World, Option<ElementId>)> {
        self.entries
            .iter()
            .map(|entry| (entry.world, entry.focus))
            .collect()
    }
    pub fn push(&mut self, location: Location) {
        if self.update_current(location.clone()) {
            return;
        }
        self.entries.truncate(self.cursor + 1);
        self.entries.push(location);
        self.cursor = self.entries.len() - 1;
        if self.entries.len() > 128 {
            self.entries.remove(0);
            self.cursor -= 1;
        }
    }
    /// Refresh the current visit without creating a second visit or discarding
    /// Forward history when an asynchronous projection finishes restoration.
    pub fn update_current(&mut self, location: Location) -> bool {
        if let Some(entry) = self.entries.get_mut(self.cursor)
            && entry.revision == location.revision
            && entry.world == location.world
            && entry.focus == location.focus
        {
            *entry = location;
            true
        } else {
            false
        }
    }
    pub fn update_camera(&mut self, center: [f32; 2], zoom: f32) {
        if let Some(entry) = self.entries.get_mut(self.cursor) {
            entry.center = center;
            entry.zoom = zoom;
            entry.presentation.camera.center = agq_studio_scene::Point::new(center[0], center[1]);
            entry.presentation.camera.zoom = zoom;
        }
    }
    pub fn back(&mut self) -> Option<Location> {
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        self.entries.get(self.cursor).cloned()
    }
    pub fn forward(&mut self) -> Option<Location> {
        if self.cursor + 1 >= self.entries.len() {
            return None;
        }
        self.cursor += 1;
        self.entries.get(self.cursor).cloned()
    }
}
