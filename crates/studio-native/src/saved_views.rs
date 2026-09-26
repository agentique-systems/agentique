//! Local operator presentation bookmarks. These never enter a model checkpoint
//! or advance a repository branch; the exact saved revision remains explicit.
use crate::{navigation::World, session};
use agq_kernel::ElementId;
use agq_modeling_repository::{BranchId, ProjectId, ProjectRevisionId};
use agq_modeling_view::{ViewDefinition, ViewKind, ViewProjection};
use agq_studio_scene::{Camera2D, LayoutMemory};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io, path::Path};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PanelState {
    pub agent: bool,
    pub explain: bool,
    pub source: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedPresentation {
    pub world: World,
    pub definition: ViewDefinition,
    pub camera: Camera2D,
    pub layout: LayoutMemory,
    pub collapsed: BTreeSet<ElementId>,
    pub expanded: Option<BTreeSet<ElementId>>,
    pub branch: Option<BranchId>,
    #[serde(default)]
    pub panels: PanelState,
    #[serde(default)]
    pub selection: Option<crate::selection::Selection>,
}

impl SavedPresentation {
    pub fn valid(&self) -> bool {
        let expected_kind = match self.world {
            World::System => ViewKind::Architecture,
            World::Requirements => ViewKind::Requirements,
            World::Graph | World::History => ViewKind::SemanticGraph,
        };
        self.definition.version == ViewDefinition::VERSION
            && self.definition.kind == expected_kind
            && self.definition.depth <= 8
            && session::valid_camera(&self.camera)
            && self.layout.bounds.values().all(|bounds| bounds.finite())
    }

    /// Reconcile against an already loaded projection for this exact revision.
    /// Absence means outside this loaded view, never proof of semantic deletion.
    pub fn reconcile(&mut self, projection: &ViewProjection) -> Recovery {
        let ids: BTreeSet<_> = projection
            .nodes
            .iter()
            .flat_map(|node| {
                std::iter::once(node.id).chain(node.features.iter().map(|feature| feature.id))
            })
            .collect();
        let mut recovery = Recovery::default();
        if self.definition.focus.is_some_and(|id| !ids.contains(&id)) {
            self.definition.focus = None;
            recovery.focus_reset = true;
        }
        let before = self.collapsed.len()
            + self.layout.bounds.len()
            + self.expanded.as_ref().map_or(0, BTreeSet::len);
        self.collapsed.retain(|id| ids.contains(id));
        self.layout.bounds.retain(|id, _| ids.contains(id));
        // Hidden IDs are expected to be absent from this projection. Keep them;
        // absence is not sufficient evidence to remove an intentional omission.
        if let Some(expanded) = &mut self.expanded {
            expanded.retain(|id| ids.contains(id));
            if expanded.is_empty() {
                self.expanded = None;
            }
        }
        let after = self.collapsed.len()
            + self.layout.bounds.len()
            + self.expanded.as_ref().map_or(0, BTreeSet::len);
        recovery.omitted_references = before.saturating_sub(after);
        recovery
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Recovery {
    pub focus_reset: bool,
    pub omitted_references: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewBinding {
    pub project: Option<ProjectId>,
    pub revision: ProjectRevisionId,
    pub fixture: Option<String>,
}
impl ViewBinding {
    pub fn same_project(&self, other: &Self) -> bool {
        self.project == other.project && self.fixture == other.fixture
    }

    fn valid(&self) -> bool {
        self.project.is_some() != self.fixture.is_some()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedView {
    pub id: u64,
    pub name: String,
    pub binding: ViewBinding,
    pub presentation: SavedPresentation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedViews {
    version: u32,
    pub views: Vec<SavedView>,
}
impl Default for SavedViews {
    fn default() -> Self {
        Self {
            version: 1,
            views: vec![],
        }
    }
}
impl SavedViews {
    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = match session::read_presentation(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let result: Self = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        result.validate()?;
        Ok(result)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        self.validate()?;
        session::write_presentation(path, &serde_json::to_vec(self).map_err(io::Error::other)?)
    }

    fn validate(&self) -> io::Result<()> {
        let mut ids = BTreeSet::new();
        if self.version != 1
            || self.views.len() > 256
            || self.views.iter().any(|view| {
                !ids.insert(view.id)
                    || view.id == 0
                    || valid_name(&view.name).is_err()
                    || !view.binding.valid()
                    || !view.presentation.valid()
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Saved views have an unsupported format or invalid presentation state",
            ));
        }
        for (index, view) in self.views.iter().enumerate() {
            if self.views[..index].iter().any(|other| {
                other.binding.same_project(&view.binding)
                    && other.name.to_lowercase() == view.name.to_lowercase()
            }) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Duplicate saved view names within one project",
                ));
            }
        }
        Ok(())
    }

    pub fn insert(
        &mut self,
        name: &str,
        binding: ViewBinding,
        presentation: SavedPresentation,
    ) -> Result<u64, String> {
        let name = valid_name(name)?;
        self.ensure_name_available(&name, &binding, None)?;
        if !binding.valid() || !presentation.valid() {
            return Err("Cannot save an invalid or unloaded presentation".into());
        }
        if self.views.len() >= 256 {
            return Err("The local saved-view limit is 256".into());
        }
        let id = self
            .views
            .iter()
            .map(|view| view.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Saved view identity exhausted")?;
        self.views.push(SavedView {
            id,
            name,
            binding,
            presentation,
        });
        Ok(id)
    }

    pub fn rename(&mut self, id: u64, name: &str) -> Result<(), String> {
        let name = valid_name(name)?;
        let binding = &self
            .views
            .iter()
            .find(|view| view.id == id)
            .ok_or("Saved view is no longer available")?
            .binding;
        self.ensure_name_available(&name, binding, Some(id))?;
        self.views
            .iter_mut()
            .find(|view| view.id == id)
            .ok_or("Saved view is no longer available")?
            .name = name;
        Ok(())
    }

    /// Explicitly replace this local bookmark with the current immutable revision
    /// and presentation. This never updates or commits a semantic revision.
    pub fn update(
        &mut self,
        id: u64,
        binding: ViewBinding,
        presentation: SavedPresentation,
    ) -> Result<(), String> {
        if !binding.valid() || !presentation.valid() {
            return Err("Cannot save an invalid presentation".into());
        }
        let view = self
            .views
            .iter_mut()
            .find(|view| view.id == id)
            .ok_or("Saved view is no longer available")?;
        if !view.binding.same_project(&binding) {
            return Err("A saved view cannot move to another project or fixture".into());
        }
        view.binding = binding;
        view.presentation = presentation;
        Ok(())
    }

    fn ensure_name_available(
        &self,
        name: &str,
        binding: &ViewBinding,
        except: Option<u64>,
    ) -> Result<(), String> {
        if self.views.iter().any(|view| {
            Some(view.id) != except
                && view.binding.same_project(binding)
                && view.name.to_lowercase() == name.to_lowercase()
        }) {
            return Err("A view with this name already exists in this project".into());
        }
        Ok(())
    }
}

fn valid_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(
            "Use a view name between 1 and 120 characters, without control characters".into(),
        );
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> (ViewBinding, SavedPresentation) {
        let projection = agq_studio_scene::fixtures::architecture();
        (
            ViewBinding {
                project: None,
                fixture: Some("architecture".into()),
                revision: projection.revision_id,
            },
            SavedPresentation {
                world: World::System,
                definition: ViewDefinition::architecture(),
                camera: Camera2D::default(),
                layout: LayoutMemory::default(),
                collapsed: BTreeSet::new(),
                expanded: None,
                branch: None,
                panels: PanelState::default(),
                selection: None,
            },
        )
    }

    #[test]
    fn save_rename_update_preserve_binding_and_never_move_between_projects() {
        let (binding, presentation) = state();
        let mut views = SavedViews::default();
        let id = views
            .insert(
                "Repository interfaces",
                binding.clone(),
                presentation.clone(),
            )
            .unwrap();
        assert!(
            views
                .insert(
                    "REPOSITORY INTERFACES",
                    binding.clone(),
                    presentation.clone()
                )
                .is_err()
        );
        views.rename(id, "Repository contracts").unwrap();
        let mut other_revision = binding.clone();
        other_revision.revision = ProjectRevisionId::new();
        views
            .update(id, other_revision.clone(), presentation.clone())
            .unwrap();
        assert_eq!(views.views[0].binding, other_revision);
        let mut other_project = binding;
        other_project.fixture = Some("requirements".into());
        assert!(views.update(id, other_project, presentation).is_err());
        assert_eq!(views.views[0].binding, other_revision);
    }

    #[test]
    fn missing_id_recovery_keeps_existing_references_and_never_empties_the_world() {
        let (_, mut presentation) = state();
        let projection = agq_studio_scene::fixtures::architecture();
        let existing = projection.nodes[0].id;
        let absent = ElementId::from_u128(u128::MAX);
        presentation.definition.focus = Some(absent);
        presentation.collapsed = BTreeSet::from([existing, absent]);
        presentation.expanded = Some(BTreeSet::from([absent]));
        let recovered = presentation.reconcile(&projection);
        assert!(recovered.focus_reset);
        assert_eq!(recovered.omitted_references, 2);
        assert_eq!(presentation.collapsed, BTreeSet::from([existing]));
        assert!(presentation.expanded.is_none());
        assert!(presentation.definition.focus.is_none());
    }

    #[test]
    fn corrupt_existing_store_is_an_error_not_an_empty_store_to_overwrite() {
        let path =
            std::env::temp_dir().join(format!("agq-saved-views-{}.json", ProjectRevisionId::new()));
        let (binding, presentation) = state();
        let mut views = SavedViews::default();
        views
            .insert("Part architecture", binding, presentation)
            .unwrap();
        views.save(&path).unwrap();
        assert_eq!(SavedViews::load(&path).unwrap().views.len(), 1);
        std::fs::write(&path, b"interrupted invalid content").unwrap();
        assert!(SavedViews::load(&path).is_err());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"interrupted invalid content"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn saved_view_roundtrip_retains_exact_revision_lens_and_presentation() {
        let path = std::env::temp_dir().join(format!(
            "agq-view-roundtrip-{}.json",
            ProjectRevisionId::new()
        ));
        let (binding, mut presentation) = state();
        let id = agq_studio_scene::fixtures::architecture().nodes[0].id;
        presentation.definition.focus = Some(id);
        presentation.definition.depth = 3;
        presentation.definition.include_standard_library = true;
        presentation.collapsed.insert(id);
        presentation.camera.zoom = 1.75;
        let mut store = SavedViews::default();
        store
            .insert("μs interface view", binding.clone(), presentation.clone())
            .unwrap();
        store.save(&path).unwrap();
        store.rename(1, "Interface view μs").unwrap();
        store.save(&path).unwrap();
        let restored = SavedViews::load(&path).unwrap();
        assert_eq!(restored.views[0].binding, binding);
        assert_eq!(restored.views[0].name, "Interface view μs");
        assert_eq!(
            restored.views[0].presentation.definition,
            presentation.definition
        );
        assert_eq!(restored.views[0].presentation.camera, presentation.camera);
        assert_eq!(
            restored.views[0].presentation.collapsed,
            presentation.collapsed
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn mismatched_lens_and_invalid_camera_never_replace_a_bookmark() {
        let (binding, presentation) = state();
        let mut store = SavedViews::default();
        let id = store
            .insert("Architecture", binding.clone(), presentation.clone())
            .unwrap();
        let mut invalid = presentation.clone();
        invalid.world = World::Requirements;
        assert!(store.update(id, binding.clone(), invalid).is_err());
        let mut invalid = presentation;
        invalid.camera.zoom = f32::INFINITY;
        assert!(store.update(id, binding, invalid).is_err());
        assert_eq!(store.views[0].presentation.world, World::System);
        assert!(store.views[0].presentation.camera.zoom.is_finite());
    }
}
