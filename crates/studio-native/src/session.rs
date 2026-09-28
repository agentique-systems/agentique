//! What the Studio remembers between runs: the last project, recent
//! projects, and per project the view, camera and layout. Presentation only;
//! the model is saved by the project.
use crate::navigation::SurfaceView;
use agq_studio_scene::{Camera2D, LayoutMemory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    /// The project that was open, reopened on start.
    pub project: Option<PathBuf>,
    /// Most recent first.
    #[serde(default)]
    pub recent: Vec<PathBuf>,
    #[serde(default)]
    pub views: BTreeMap<PathBuf, ProjectView>,
    /// Stage 4 kept the appearance here; Settings has it now (read once,
    /// then `appearance_in_settings`).
    pub dark: bool,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    #[serde(default)]
    pub appearance_in_settings: bool,
}

/// How one project was last shown.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ProjectView {
    pub view: SurfaceView,
    pub camera: Option<Camera2D>,
    #[serde(default)]
    pub layouts: BTreeMap<SurfaceView, LayoutMemory>,
    /// Focus mode: the Outline, the Panels and the Conversation hidden.
    #[serde(default)]
    pub panels_hidden: bool,
    /// Single panels collapsed (Ctrl+B, Ctrl+Alt+B, Ctrl+J).
    #[serde(default)]
    pub outline_hidden: bool,
    #[serde(default)]
    pub inspector_hidden: bool,
    #[serde(default)]
    pub conversation_hidden: bool,
}

impl Session {
    pub const VERSION: u32 = 2;

    pub fn load(path: &Path) -> Option<Self> {
        let bytes = read_presentation(path).ok()?;
        let mut session: Self = serde_json::from_slice(&bytes).ok()?;
        if session.version != Self::VERSION {
            return None;
        }
        for view in session.views.values_mut() {
            view.camera = view.camera.filter(valid_camera);
            view.layouts
                .retain(|_, layout| layout.bounds.values().all(|b| b.finite()));
        }
        Some(session)
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        write_presentation(path, &serde_json::to_vec(self)?)
    }
    /// Moves `folder` to the front of the recent projects.
    pub fn remember(&mut self, folder: &Path) {
        self.project = Some(folder.to_path_buf());
        self.recent.retain(|recent| recent != folder);
        self.recent.insert(0, folder.to_path_buf());
        self.recent.truncate(8);
    }
    /// The default location: the user's configuration directory.
    pub fn default_path() -> PathBuf {
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(std::env::temp_dir);
        base.join("Agentique").join("studio-session.json")
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
    let temporary = path.with_extension(format!("json.{}.next", std::process::id()));
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

    #[test]
    fn a_session_round_trips_and_remembers_recent_projects() {
        let directory = std::env::temp_dir().join(format!("agq-session-{}", std::process::id()));
        let path = directory.join("session.json");
        let mut session = Session {
            version: Session::VERSION,
            dark: true,
            ..Default::default()
        };
        session.remember(Path::new("a"));
        session.remember(Path::new("b"));
        session.remember(Path::new("a"));
        assert_eq!(session.recent, [PathBuf::from("a"), PathBuf::from("b")]);
        session.views.insert(
            PathBuf::from("a"),
            ProjectView {
                view: SurfaceView::Graph,
                camera: Some(Camera2D::default()),
                layouts: BTreeMap::new(),
                panels_hidden: false,
                ..Default::default()
            },
        );
        session.save(&path).unwrap();
        let loaded = Session::load(&path).unwrap();
        assert_eq!(loaded.project, Some(PathBuf::from("a")));
        assert_eq!(loaded.views[Path::new("a")].view, SurfaceView::Graph);
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[cfg(test)]
mod safety_tests {
    use super::*;

    fn path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("agq-session-{name}-{}.json", std::process::id()))
    }

    #[test]
    fn oversized_or_future_sessions_are_not_loaded() {
        let file = path("oversized");
        std::fs::File::create(&file)
            .unwrap()
            .set_len(8 * 1024 * 1024 + 1)
            .unwrap();
        assert!(Session::load(&file).is_none());
        let future = Session {
            version: Session::VERSION + 1,
            ..Default::default()
        };
        future.save(&file).unwrap();
        assert!(Session::load(&file).is_none());
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn unsafe_cameras_are_dropped_and_unreadable_sessions_are_not_loaded() {
        let file = path("camera");
        let mut session = Session {
            version: Session::VERSION,
            ..Default::default()
        };
        let camera = Camera2D {
            zoom: Camera2D::MAX_ZOOM + 1.0,
            ..Default::default()
        };
        session.views.insert(
            PathBuf::from("p"),
            ProjectView {
                view: SurfaceView::Graph,
                camera: Some(camera),
                layouts: BTreeMap::new(),
                panels_hidden: false,
                ..Default::default()
            },
        );
        session.save(&file).unwrap();
        let loaded = Session::load(&file).unwrap();
        let view = &loaded.views[Path::new("p")];
        assert!(view.camera.is_none());
        assert_eq!(view.view, SurfaceView::Graph);
        // Geometry that is not a number cannot be read back at all.
        let mut value = serde_json::to_value(&loaded).unwrap();
        value["views"]["p"]["camera"] = serde_json::json!({"center": {"x": null, "y": 0.0}, "zoom": 1.0, "viewport": {"width": 1.0, "height": 1.0}});
        std::fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Session::load(&file).is_none());
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn a_failed_replacement_keeps_the_previous_session() {
        let file = path("replace");
        write_presentation(&file, b"previous").unwrap();
        let oversized = vec![0; PRESENTATION_LIMIT as usize + 1];
        assert!(write_presentation(&file, &oversized).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"previous");
        let _ = std::fs::remove_file(file);
    }
}
