//! Disposable scene construction off the UI thread. The latest request wins;
//! neither worker completion nor a cancelled presentation can change a model.
use agq_modeling_view::ViewProjection;
use agq_studio_scene::{LayoutMemory, SceneLookup, SceneOptions, SemanticScene, SpatialIndex};
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Instant,
};

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
    let outliner = crate::app::hierarchy_order(&scene);
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
