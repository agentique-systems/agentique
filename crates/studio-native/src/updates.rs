use crate::{
    app::{Candidate, ComparisonMode, StudioApp},
    bridge::Output,
    navigation::{Navigation, World},
};
use agq_studio_platform::RevisionBinding;
use eframe::egui;
use std::time::Duration;

impl StudioApp {
    pub fn receive(&mut self) {
        while let Ok(reply) = self.bridge.replies.try_recv() {
            if reply.terminal {
                self.pending.remove(&reply.request);
                self.bridge.complete(reply.request);
            }
            if reply.terminal
                && self
                    .preparation
                    .as_ref()
                    .is_some_and(|p| p.request == reply.request)
            {
                let preparation = self.preparation.take().expect("matching preparation");
                if preparation.cancelled {
                    if let Ok(Output::Candidate(candidate)) = reply.result {
                        self.enqueue_mutation(Box::new(move |platform| {
                            platform.cancel(candidate.id)?;
                            Ok(Output::Cancelled)
                        }));
                    }
                    self.status =
                        "Candidate preparation cancelled · current revision retained".into();
                    continue;
                }
            }
            if let Some(context) = &reply.context {
                if !context.matches(&self.work_context()) {
                    if reply.mutation {
                        self.status="A model operation completed for an earlier context; reopen that project's history to reconcile it".into();
                    }
                    continue;
                }
            } else if !self.bridge.current_open(reply.request) {
                continue;
            }
            match reply.result {
                Err(error) => {
                    self.setup_reason = error.clone();
                    self.status = error;
                    // A disposable saved focus may no longer be usable. Retry
                    // its authenticated revision once with the same World and
                    // no focus; a failed restoration never exposes fixtures.
                    if reply.request == self.scene_request && self.restore.take().is_some() {
                        self.focus = None;
                        self.request_projection();
                        continue;
                    }
                    if reply.mutation
                        && let Some(id) = self.candidate.as_ref().and_then(|c| c.id)
                    {
                        let view = self.definition();
                        self.scene_request = self.enqueue(Box::new(move |p| {
                            crate::bridge::candidate_view(p, id, &view)
                        }));
                    }
                }
                Ok(Output::Progress(phase)) => {
                    self.setup_reason = format!("{phase:?}");
                    self.status = self.setup_reason.clone();
                }
                Ok(Output::Ready(projects)) => {
                    self.projects = projects;
                    if self.fixture.is_none()
                        && let Some(project) = self
                            .restore
                            .as_ref()
                            .and_then(|s| s.project)
                            .filter(|id| self.projects.iter().any(|p| p.id == *id))
                    {
                        self.open_project(project);
                    } else {
                        self.setup_reason = "Runtime authenticated. Choose a project.".into();
                    }
                }
                Ok(Output::History(history)) if reply.request == self.project_request => {
                    let preferred_branch = self
                        .restore
                        .as_ref()
                        .filter(|session| session.project == Some(history.project.id))
                        .and_then(|session| session.presentation.as_ref())
                        .and_then(|presentation| presentation.branch)
                        .filter(|id| history.branches.iter().any(|branch| branch.id == *id))
                        .unwrap_or(history.project.default_branch);
                    let branch = history
                        .branches
                        .iter()
                        .find(|b| b.id == preferred_branch)
                        .cloned();
                    if let Some(branch) = branch {
                        let revision = self
                            .restore
                            .as_ref()
                            .filter(|s| s.project == Some(history.project.id))
                            .map(|s| s.revision)
                            .filter(|id| history.revisions.iter().any(|r| r.revision_id == *id))
                            .unwrap_or(branch.head);
                        self.binding = Some(RevisionBinding {
                            project: history.project.id,
                            revision,
                        });
                        self.branch = Some(branch.id);
                        self.fixture = None;
                        self.ready = false;
                        self.setup_reason = "Loading the selected project revision".into();
                        let restored = self.restore.as_ref().filter(|session| {
                            session.project == Some(history.project.id)
                                && session.revision == revision
                        });
                        self.focus = restored.and_then(|session| session.focus);
                        self.world = restored.map_or(World::System, |session| session.world);
                        self.history = Some(history);
                        self.navigation = Navigation::default();
                        self.candidate = None;
                        self.compare_before = None;
                        self.comparison = ComparisonMode::Current;
                        self.layout = Default::default();
                        self.layouts.clear();
                        self.selection.clear();
                        self.invalidate_inspection();
                        self.expanded = None;
                        self.dependencies = None;
                        self.collapsed.clear();
                        if let Some(saved) = self
                            .restore
                            .as_ref()
                            .and_then(|session| session.presentation.as_ref())
                        {
                            self.families = saved
                                .definition
                                .relationship_families
                                .iter()
                                .copied()
                                .collect();
                            self.include_standard = saved.definition.include_standard_library;
                            self.collapsed = saved.collapsed.clone();
                            self.expanded = saved.expanded.clone();
                        }
                        self.fit_pending = true;
                        self.request_projection();
                    }
                }
                Ok(Output::Projection(projection))
                    if reply.request == self.scene_request
                        && self
                            .binding
                            .is_some_and(|binding| binding.revision == projection.revision_id) =>
                {
                    self.projection = projection;
                    self.ready = true;
                    self.rebuild();
                    self.fit_pending = true;
                    self.apply_pending_presentation();
                    self.request_inspection();
                    self.record_location();
                    self.status = "Revision restored in process".into();
                }
                Ok(Output::Inspector(inspector))
                    if reply.request == self.inspector_request
                        && self
                            .selected_context()
                            .is_some_and(|(binding, _, element)| {
                                binding.revision == inspector.revision_id
                                    && element == inspector.element.id
                            }) =>
                {
                    self.inspector = Some(inspector)
                }
                Ok(Output::Explanation(explanation))
                    if reply.request == self.explanation_request
                        && self
                            .selected_context()
                            .is_some_and(|(binding, _, element)| {
                                binding.revision == explanation.revision_id
                                    && element == explanation.subject_id
                            }) =>
                {
                    self.explanation = Some(explanation)
                }
                Ok(Output::Source(source))
                    if reply.request == self.source_request
                        && self
                            .selected_context()
                            .is_some_and(|(binding, _, element)| {
                                binding == source.binding && element == source.element
                            }) =>
                {
                    self.source = Some(source)
                }
                Ok(Output::Comparison(comparison)) if reply.request == self.scene_request => {
                    self.projection = comparison.after;
                    self.compare_before = Some(comparison.before);
                    self.comparison = ComparisonMode::Diff;
                    self.rebuild();
                    self.fit_pending = true;
                    self.request_inspection();
                }
                Ok(Output::Candidate(candidate))
                    if reply.mutation
                        && self.binding == Some(candidate.base)
                        && self.fixture.is_none() =>
                {
                    let before = self
                        .candidate
                        .as_ref()
                        .filter(|current| current.id == Some(candidate.id))
                        .map(|current| current.before.clone())
                        .unwrap_or_else(|| self.projection.clone());
                    self.candidate = Some(Candidate {
                        id: Some(candidate.id),
                        phase: Some(candidate.phase),
                        before,
                        after: candidate.projection,
                        source: format!(
                            "{}\n\n{}",
                            candidate.source_preview.path, candidate.source_preview.after
                        ),
                    });
                    self.invalidate_inspection();
                    self.scene_request = 0;
                    self.comparison = ComparisonMode::Diff;
                    self.rebuild();
                    self.fit_pending = true;
                    self.request_inspection();
                    self.status = "Candidate revision ready for review".into();
                }
                Ok(Output::CandidateView(candidate, before))
                    if reply.request == self.scene_request
                        && self.binding == Some(candidate.base)
                        && self
                            .candidate
                            .as_ref()
                            .is_some_and(|current| current.id == Some(candidate.id)) =>
                {
                    if let Some(current) = &mut self.candidate {
                        current.before = before;
                        current.after = candidate.projection;
                        current.phase = Some(candidate.phase);
                    }
                    if reply.mutation {
                        self.comparison = ComparisonMode::Diff;
                        self.status =
                            "Candidate validated; review its revision-bound difference".into();
                    }
                    self.invalidate_inspection();
                    self.rebuild();
                    self.fit_pending = true;
                    self.apply_pending_presentation();
                    self.request_inspection();
                }
                Ok(Output::Committed(receipt))
                    if reply.mutation
                        && self.branch == Some(receipt.branch_id)
                        && reply.context.as_ref().is_some_and(|scope| {
                            scope.binding == self.binding && scope.candidate.is_some()
                        }) =>
                {
                    if let Some(binding) = reply.context.as_ref().and_then(|scope| scope.binding) {
                        self.binding = Some(RevisionBinding {
                            revision: receipt.revision_id,
                            ..binding
                        });
                    }
                    self.candidate = None;
                    self.compare_before = None;
                    self.comparison = ComparisonMode::Current;
                    self.request_projection();
                    if let Some(binding) = self.binding {
                        self.enqueue(Box::new(move |p| {
                            p.history(binding.project).map(Output::HistoryRefresh)
                        }));
                    }
                    self.status = "Candidate durably committed".into();
                }
                Ok(Output::HistoryRefresh(history))
                    if self.project_id() == Some(history.project.id) =>
                {
                    self.history = Some(history)
                }
                Ok(Output::Cancelled) if reply.mutation => {
                    self.candidate = None;
                    self.comparison = ComparisonMode::Current;
                    self.invalidate_inspection();
                    self.scene_request = 0;
                    self.rebuild();
                    self.request_inspection();
                    self.status = "Uncommitted candidate cancelled".into();
                }
                Ok(_) => {} // Superseded read requests cannot populate another view/selection.
            }
        }
    }
    fn apply_pending_presentation(&mut self) {
        if let Some(restore) = self.restore.take()
            && restoration_matches(&restore, self.binding, self.world, self.focus)
        {
            if let Some(presentation) = restore.presentation {
                self.apply_saved_presentation(presentation);
                self.rebuild();
                return;
            }
            self.camera = restore.camera;
            self.camera_target = None;
            self.layout = restore.layout;
            // Set the world before rebuilding, so the per-world memory swap
            // cannot discard the newly restored layout.
            self.layout_world = self.world;
            self.rebuild();
            self.fit_pending = false;
        }
    }
    pub fn open_project(&mut self, id: agq_modeling_repository::ProjectId) {
        if !self.allow_context_change() {
            return;
        }
        self.invalidate_inspection();
        self.scene_request = 0;
        self.project_request = self.enqueue(Box::new(move |platform| {
            platform.history(id).map(Output::History)
        }));
    }
    pub fn metrics_report(&self) -> serde_json::Value {
        let stats = self.gpu_stats.lock().ok();
        let frames = self.timing.frame_summary();
        serde_json::json!({
            "fixture": self.fixture,
            "adapter": self.adapter,
            "frame_count": self.frame_number,
            "frame_interval_median_ms": frames.median,
            "frame_interval_p95_ms": frames.p95,
            "frame_intervals_ms": frames,
            "warmup_frame_intervals_discarded": self.timing.discarded_frame_intervals(),
            "scene_build_ms": self.timing.scene_ms,
            "scene_layout_routing_diff_ms": self.timing.layout_ms,
            "scene_index_lookup_outliner_ms": self.timing.index_ms,
            "scene_background_pending": self.scene_builder.busy,
            "input_pipeline": self.timing.latency_report(),
            "layout_included_in_scene_build": true,
            "hit_test_us": self.timing.hit_summary(),
            "handled_input_to_next_ui_update_ms": {
                "pan": self.timing.input_summary(crate::timing::InputKind::Pan),
                "zoom": self.timing.input_summary(crate::timing::InputKind::Zoom),
                "selection": self.timing.input_summary(crate::timing::InputKind::Selection),
            },
            "gpu_upload_cpu_ms": stats.as_ref().filter(|s| s.uploads > 0).map(|s| s.upload_ms),
            "gpu_uploaded_bytes": stats.as_ref().map(|s| s.uploaded_bytes),
            "gpu_upload_count": stats.as_ref().map(|s| s.uploads),
            "gpu_instances": stats.as_ref().map(|s| s.instances),
            "scene_draw_calls": stats.as_ref().map(|s| s.draw_calls),
            "visible_nodes": self.timing.visible_nodes,
            "total_nodes": self.scene.nodes.len(),
            "total_edges": self.scene.edges.len(),
            "gpu_timestamp_ms": null,
            "physical_input_to_photon_ms": null,
            "note": "Native wgpu frames with vsync. CPU update intervals retain the latest 240 samples after 60 warmup intervals. Input spans start inside the viewport gesture handler and end at the next UI update; they exclude OS input delivery and do not measure presentation. GPU upload is CPU submission time for the last upload. Null means unmeasured."
        })
    }
    pub fn capture(&mut self, ctx: &egui::Context) {
        if self.args.screenshot.is_some() && !self.capture_requested && self.frame_number >= 20 {
            self.capture_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let screenshot = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot
            && let Some(path) = &self.args.screenshot
        {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            match image::save_buffer(
                path,
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            ) {
                Ok(()) => {
                    self.capture_done = true;
                    println!("Native surface captured: {}", path.display());
                }
                Err(error) => {
                    self.status = format!("Screenshot failed: {error}");
                    eprintln!("{}", self.status);
                    self.capture_done = true;
                }
            }
        }
        if self.args.screenshot.is_some() && !self.capture_done {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        let finished = self
            .args
            .frames
            .is_some_and(|frames| self.frame_number >= frames)
            || (self.args.frames.is_none() && self.args.screenshot.is_some() && self.capture_done);
        if finished
            && self.args.scenario.is_none()
            && (self.args.screenshot.is_none() || self.capture_done)
        {
            // Close is asynchronous; subsequent updates must not overwrite the
            // chosen measurement or emit a second benchmark record.
            let emitted = egui::Id::new("native-benchmark-metrics-emitted");
            if ctx
                .data(|data| data.get_temp::<bool>(emitted))
                .unwrap_or(false)
            {
                return;
            }
            let report = self.metrics_report();
            if let Some(path) = &self.args.metrics {
                let write_result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
                    Ok(())
                })();
                if let Err(error) = write_result {
                    eprintln!("Cannot persist native benchmark metrics: {error}");
                    std::process::exit(2);
                }
            }
            println!("{report}");
            ctx.data_mut(|data| data.insert_temp(emitted, true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn restoration_matches(
    session: &crate::session::Session,
    binding: Option<RevisionBinding>,
    world: World,
    focus: Option<agq_kernel::ElementId>,
) -> bool {
    binding.is_some_and(|binding| {
        session.project == Some(binding.project)
            && session.revision == binding.revision
            && session.world == world
            && session.focus == focus
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_restore_requires_exact_revision_world_and_focus() {
        let binding = RevisionBinding {
            project: agq_modeling_repository::ProjectId::new(),
            revision: agq_modeling_workspace::ProjectRevisionId::new(),
        };
        let focus = Some(agq_kernel::ElementId::from_u128(42));
        let mut session = crate::session::Session {
            version: 1,
            project: Some(binding.project),
            revision: binding.revision,
            fixture: None,
            world: World::Graph,
            focus,
            camera: agq_studio_scene::Camera2D::default(),
            layout: Default::default(),
            dark: true,
            high_contrast: false,
            reduced_motion: false,
            presentation: None,
        };
        assert!(restoration_matches(
            &session,
            Some(binding),
            World::Graph,
            focus
        ));
        assert!(!restoration_matches(
            &session,
            Some(binding),
            World::System,
            focus
        ));
        assert!(!restoration_matches(
            &session,
            Some(binding),
            World::Graph,
            None
        ));
        assert!(!restoration_matches(&session, None, World::Graph, focus));
        session.revision = agq_modeling_workspace::ProjectRevisionId::new();
        assert!(!restoration_matches(
            &session,
            Some(binding),
            World::Graph,
            focus
        ));
        session.revision = binding.revision;
        session.project = Some(agq_modeling_repository::ProjectId::new());
        assert!(!restoration_matches(
            &session,
            Some(binding),
            World::Graph,
            focus
        ));
    }
}
