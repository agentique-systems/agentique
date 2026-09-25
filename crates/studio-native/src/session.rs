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
    /// Optional extension keeps version-1 sessions readable. Model identity
    /// remains in the surrounding session, never in disposable layout hints.
    #[serde(default)]
    pub presentation: Option<crate::saved_views::SavedPresentation>,
}

impl Session {
    pub fn load(path: &Path) -> Option<Self> {
        let bytes = read_presentation(path).ok()?;
        let result: Self = serde_json::from_slice(&bytes).ok()?;
        if result.version != 1
            || !valid_camera(&result.camera)
            || result.layout.bounds.values().any(|bounds| !bounds.finite())
            || result.presentation.as_ref().is_some_and(|presentation| {
                !presentation.valid()
                    || presentation.world != result.world
                    || presentation.definition.focus != result.focus
            })
        {
            return None;
        }
        Some(result)
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        write_presentation(path, &serde_json::to_vec(self)?)
    }
}

pub fn valid_camera(camera: &Camera2D) -> bool {
    camera.zoom.is_finite()
        && (Camera2D::MIN_ZOOM..=Camera2D::MAX_ZOOM).contains(&camera.zoom)
        && camera.center.x.is_finite()
        && camera.center.y.is_finite()
        && camera.viewport.width.is_finite()
        && camera.viewport.width > 0.0
        && camera.viewport.height.is_finite()
        && camera.viewport.height > 0.0
}

const PRESENTATION_LIMIT: u64 = 8 * 1024 * 1024;

pub fn read_presentation(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > PRESENTATION_LIMIT {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Presentation state exceeds 8 MiB",
        ));
    }
    let mut bytes = Vec::new();
    file.take(PRESENTATION_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > PRESENTATION_LIMIT {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Presentation state exceeds 8 MiB",
        ));
    }
    Ok(bytes)
}

/// Write and flush a same-directory temporary before atomic replacement.
/// On failure the existing presentation is retained; no model files are touched.
pub fn write_presentation(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if bytes.len() as u64 > PRESENTATION_LIMIT {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Presentation state exceeds 8 MiB",
        ));
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("json.{}.next", ProjectRevisionId::new()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SessionFile(std::path::PathBuf);
    impl Drop for SessionFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    fn temporary() -> SessionFile {
        SessionFile(std::env::temp_dir().join(format!(
            "agq-native-session-{}.json",
            ProjectRevisionId::new()
        )))
    }

    #[test]
    fn oversized_or_future_session_is_rejected_without_touching_model_state() {
        let path = temporary();
        std::fs::File::create(&path.0)
            .unwrap()
            .set_len(8 * 1024 * 1024 + 1)
            .unwrap();
        assert!(Session::load(&path.0).is_none());
        let session = Session {
            version: 99,
            project: None,
            revision: ProjectRevisionId::new(),
            fixture: Some("architecture".into()),
            world: World::System,
            focus: None,
            camera: Camera2D::default(),
            layout: LayoutMemory::default(),
            dark: true,
            high_contrast: false,
            reduced_motion: true,
            presentation: None,
        };
        session.save(&path.0).unwrap();
        assert!(Session::load(&path.0).is_none());
    }

    #[test]
    fn session_roundtrip_preserves_presentation_and_rejects_unsafe_camera() {
        let path = temporary();
        let mut session = Session {
            version: 1,
            project: None,
            revision: ProjectRevisionId::new(),
            fixture: Some("architecture".into()),
            world: World::Graph,
            focus: None,
            camera: Camera2D::default(),
            layout: LayoutMemory::default(),
            dark: false,
            high_contrast: true,
            reduced_motion: true,
            presentation: None,
        };
        session.save(&path.0).unwrap();
        let restored = Session::load(&path.0).unwrap();
        assert_eq!(restored.revision, session.revision);
        assert_eq!(restored.world, World::Graph);
        assert!(restored.high_contrast);
        session.camera.zoom = Camera2D::MAX_ZOOM + 1.0;
        session.save(&path.0).unwrap();
        assert!(Session::load(&path.0).is_none());
    }

    #[test]
    fn interrupted_or_oversized_replacement_retains_previous_presentation() {
        let path = temporary();
        write_presentation(&path.0, b"previous presentation").unwrap();
        let oversized = vec![0; PRESENTATION_LIMIT as usize + 1];
        assert!(write_presentation(&path.0, &oversized).is_err());
        assert_eq!(std::fs::read(&path.0).unwrap(), b"previous presentation");
    }

    #[test]
    fn old_sessions_without_extended_presentation_remain_readable() {
        let path = temporary();
        let mut value = serde_json::json!({
            "version": 1,
            "project": null,
            "revision": ProjectRevisionId::new(),
            "fixture": "architecture",
            "world": "System",
            "focus": null,
            "camera": Camera2D::default(),
            "layout": LayoutMemory::default(),
            "dark": true,
            "high_contrast": false,
            "reduced_motion": false
        });
        write_presentation(&path.0, &serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Session::load(&path.0).unwrap().presentation.is_none());
        value["camera"]["viewport"]["width"] = serde_json::json!(-1.0);
        write_presentation(&path.0, &serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Session::load(&path.0).is_none());
    }
}
