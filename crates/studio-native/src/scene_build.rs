//! Disposable scene construction off the UI thread. The latest request wins;
//! neither worker completion nor a cancelled presentation can change a model.
use agq_kernel::ElementId;
use agq_modeling_view::{RelationshipFamily, ViewProjection};
use agq_studio_scene::{LayoutMemory, SceneLookup, SceneOptions, SemanticScene, SpatialIndex};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, mpsc},
    time::Instant,
};

/// Apply the same disposable visibility policy to both sides of a comparison.
/// Canonical DTOs stay intact; containment already communicates System ownership.
pub fn presentation_projection(
    source: &ViewProjection,
    families: &BTreeSet<RelationshipFamily>,
    expanded: Option<&BTreeSet<ElementId>>,
    system: bool,
) -> ViewProjection {
    let mut projection = source.clone();
    let hidden: BTreeSet<_> = source.view.hidden_elements.iter().copied().collect();
    let visible =
        |id: &ElementId| !hidden.contains(id) && expanded.is_none_or(|ids| ids.contains(id));
    projection.nodes.retain(|node| visible(&node.id));
    for node in &mut projection.nodes {
        node.features.retain(|feature| visible(&feature.id));
    }
    projection.edges.retain(|edge| {
        visible(&edge.source)
            && visible(&edge.target)
            && families.contains(&edge.family)
            && !(system && edge.family == RelationshipFamily::Ownership)
    });
    projection
}

pub struct SceneInput {
    pub projection: ViewProjection,
    pub before: Option<ViewProjection>,
    pub options: SceneOptions,
    pub memory: LayoutMemory,
}

pub struct BuiltScene {
    pub scene: SemanticScene,
    pub spatial: SpatialIndex,
    pub lookup: SceneLookup,
    pub outliner: Vec<usize>,
    pub build_ms: f64,
    pub scene_ms: f64,
    pub index_ms: f64,
}

pub fn build(input: SceneInput) -> Result<BuiltScene, String> {
    let started = Instant::now();
    let mut scene =
        SemanticScene::from_projection(&input.projection, &input.options, Some(&input.memory))
            .map_err(|e| e.to_string())?;
    if let Some(before) = input.before {
        // A comparison must never silently become an ordinary scene when its
        // before-side cannot be constructed.
        let parent = SemanticScene::from_projection(&before, &input.options, Some(&input.memory))
            .map_err(|e| format!("Cannot construct comparison baseline: {e}"))?;
        scene.apply_diff(&parent);
    }
    let scene_ms = started.elapsed().as_secs_f64() * 1000.0;
    let index_started = Instant::now();
    let spatial = SpatialIndex::build(&scene);
    let lookup = SceneLookup::build(&scene);
    let outliner =
        crate::app::hierarchy_order_with_focus(&scene, input.projection.metadata.suggested_focus);
    let index_ms = index_started.elapsed().as_secs_f64() * 1000.0;
    Ok(BuiltScene {
        scene,
        spatial,
        lookup,
        outliner,
        build_ms: started.elapsed().as_secs_f64() * 1000.0,
        scene_ms,
        index_ms,
    })
}

type Completion = (u64, Result<BuiltScene, String>);

pub struct SceneBuilder {
    pending: Arc<Mutex<Option<(u64, SceneInput)>>>,
    wake: mpsc::SyncSender<()>,
    results: mpsc::Receiver<Completion>,
    latest: u64,
    pub busy: bool,
}

impl SceneBuilder {
    pub fn new(ctx: eframe::egui::Context) -> std::io::Result<Self> {
        let pending = Arc::new(Mutex::new(None::<(u64, SceneInput)>));
        let worker_pending = pending.clone();
        let (wake, receiver) = mpsc::sync_channel(1);
        let (sender, results) = mpsc::channel();
        std::thread::Builder::new()
            .name("agentique-scene".into())
            .spawn(move || {
                while receiver.recv().is_ok() {
                    let work = worker_pending.lock().ok().and_then(|mut slot| slot.take());
                    if let Some((ticket, input)) = work {
                        if sender.send((ticket, build(input))).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                }
            })?;
        Ok(Self {
            pending,
            wake,
            results,
            latest: 0,
            busy: false,
        })
    }

    /// Invalidates an in-flight build when a smaller synchronous view supersedes it.
    pub fn invalidate(&mut self) {
        self.latest += 1;
        self.busy = false;
        if let Ok(mut pending) = self.pending.lock() {
            *pending = None;
        }
    }

    pub fn request(&mut self, input: SceneInput) -> Result<(), String> {
        self.latest += 1;
        *self.pending.lock().map_err(|e| e.to_string())? = Some((self.latest, input));
        match self.wake.try_send(()) {
            Ok(()) | Err(mpsc::TrySendError::Full(())) => {
                self.busy = true;
                Ok(())
            }
            Err(mpsc::TrySendError::Disconnected(())) => Err("Scene worker unavailable".into()),
        }
    }

    pub fn poll(&mut self) -> Option<Result<BuiltScene, String>> {
        let mut current = None;
        while let Ok((ticket, result)) = self.results.try_recv() {
            if ticket == self.latest {
                current = Some(result);
                self.busy = false;
            }
        }
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_port_cannot_return_through_its_owners_feature_summaries() {
        let mut projection = agq_studio_scene::fixtures::architecture();
        let port = projection
            .nodes
            .iter()
            .flat_map(|node| &node.features)
            .find(|feature| feature.semantic_kind == "PortUsage")
            .unwrap()
            .id;
        projection.view.hidden_elements.push(port);
        let canonical = projection.clone();
        let filtered = presentation_projection(
            &projection,
            &RelationshipFamily::all().into_iter().collect(),
            None,
            true,
        );
        let built = build(SceneInput {
            projection: filtered,
            before: None,
            options: SceneOptions::default(),
            memory: LayoutMemory::default(),
        })
        .unwrap();
        assert!(
            built
                .scene
                .ports
                .iter()
                .all(|candidate| candidate.id != port)
        );
        assert_eq!(projection, canonical);
    }

    #[test]
    fn comparison_filters_do_not_fabricate_removed_ownership_or_erase_real_removed_links() {
        let before = agq_studio_scene::fixtures::architecture();
        let mut after = before.clone();
        let removed = after
            .edges
            .iter()
            .find(|edge| edge.family != RelationshipFamily::Ownership)
            .unwrap()
            .id
            .clone();
        after.edges.retain(|edge| edge.id != removed);
        let canonical_before = serde_json::to_value(&before).unwrap();
        let families = RelationshipFamily::all().into_iter().collect();
        let filtered_before = presentation_projection(&before, &families, None, true);
        let filtered_after = presentation_projection(&after, &families, None, true);
        assert_eq!(filtered_before.nodes, before.nodes);
        let built = build(SceneInput {
            projection: filtered_after,
            before: Some(filtered_before),
            options: SceneOptions::default(),
            memory: LayoutMemory::default(),
        })
        .unwrap();
        assert!(
            built
                .scene
                .edges
                .iter()
                .all(|edge| edge.semantic.family != RelationshipFamily::Ownership)
        );
        assert_eq!(
            built
                .scene
                .edges
                .iter()
                .filter(|edge| edge.diff == agq_studio_scene::DiffMark::Removed)
                .map(|edge| edge.semantic.id.as_str())
                .collect::<Vec<_>>(),
            vec![removed.as_str()]
        );
        assert_eq!(serde_json::to_value(&before).unwrap(), canonical_before);
        let graph = presentation_projection(&before, &families, None, false);
        assert_eq!(graph.edges, before.edges);
    }

    #[test]
    fn stale_scene_cannot_replace_newer_context() {
        let (sender, results) = mpsc::channel();
        let (wake, _) = mpsc::sync_channel(1);
        let mut worker = SceneBuilder {
            pending: Arc::new(Mutex::new(None)),
            wake,
            results,
            latest: 2,
            busy: true,
        };
        sender.send((1, Err("old revision".into()))).unwrap();
        assert!(worker.poll().is_none());
        assert!(worker.busy);
        sender.send((2, Err("current failure".into()))).unwrap();
        assert_eq!(
            worker.poll().unwrap().err().as_deref(),
            Some("current failure")
        );
        assert!(!worker.busy);
        worker.invalidate();
        sender.send((2, Err("late duplicate".into()))).unwrap();
        assert!(worker.poll().is_none());
    }
}
