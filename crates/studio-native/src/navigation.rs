//! Presentation history contains focus and camera, never model history.
use agq_kernel::ElementId;
use agq_modeling_workspace::ProjectRevisionId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub revision: ProjectRevisionId,
    pub world: World,
    pub focus: Option<ElementId>,
    pub center: [f32; 2],
    pub zoom: f32,
}

#[derive(Default, Debug)]
pub struct Navigation {
    entries: Vec<Location>,
    cursor: usize,
}
impl Navigation {
    pub fn push(&mut self, location: Location) {
        if self.entries.get(self.cursor) == Some(&location) { return; }
        self.entries.truncate(self.cursor + 1);
        self.entries.push(location);
        self.cursor = self.entries.len() - 1;
        if self.entries.len() > 128 {
            self.entries.remove(0);
            self.cursor -= 1;
        }
    }
    pub fn update_camera(&mut self, center: [f32; 2], zoom: f32) {
        if let Some(entry) = self.entries.get_mut(self.cursor) {
            entry.center = center;
            entry.zoom = zoom;
        }
    }
    pub fn back(&mut self) -> Option<Location> {
        if self.cursor == 0 { return None; }
        self.cursor -= 1;
        self.entries.get(self.cursor).cloned()
    }
    pub fn forward(&mut self) -> Option<Location> {
        if self.cursor + 1 >= self.entries.len() { return None; }
        self.cursor += 1;
        self.entries.get(self.cursor).cloned()
    }
}

