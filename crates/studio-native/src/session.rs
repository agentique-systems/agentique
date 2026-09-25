//! Disposable, versioned presentation restoration. No candidate or semantic checkpoint.
use crate::navigation::World;
use agq_kernel::ElementId;
use agq_modeling_repository::{ProjectId, ProjectRevisionId};
use agq_studio_scene::{Camera2D, LayoutMemory};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub project: Option<ProjectId>,
    pub revision: ProjectRevisionId,
    pub fixture: Option<String>,
    pub world: World,
    pub focus: Option<ElementId>,
    pub camera: Camera2D,
    pub layout: LayoutMemory,
    pub dark: bool,
    pub high_contrast: bool,
    pub reduced_motion: bool,
}
impl Session {
    pub fn load(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        if bytes.len() > 8 * 1024 * 1024 { return None; }
        let result: Self = serde_json::from_slice(&bytes).ok()?;
        if result.version != 1 || !result.camera.zoom.is_finite()
            || !(Camera2D::MIN_ZOOM..=Camera2D::MAX_ZOOM).contains(&result.camera.zoom)
            || !result.camera.center.x.is_finite() || !result.camera.center.y.is_finite()
            || result.layout.bounds.values().any(|bounds| !bounds.finite()) {
            return None;
        }
        Some(result)
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let temporary = path.with_extension("json.next");
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec(self)?)?;
        file.sync_all()?;
        std::fs::rename(temporary, path)
    }
}
