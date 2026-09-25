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
        let replies: Vec<_> = self.bridge.replies.try_iter().collect();
        self.receive_replies(replies);
    }
    pub fn receive_replies(&mut self, replies: impl IntoIterator<Item = crate::bridge::Reply>) {
        for reply in replies {
            if reply.terminal {
                self.pending.remove(&reply.request);
                self.bridge.complete(reply.request);
            }
            // Project history is independent of the displayed revision/candidate.
            // Its own request fence excludes older refreshes and other projects.
            if matches!(&reply.result, Ok(Output::HistoryRefresh(_)))
                || self
                    .history_request
                    .is_some_and(|(request, _)| request == reply.request)
            {
                if let Some((request, project)) = self.history_request
                    && request == reply.request
                    && self.fixture.is_none()
                    && self.project_id() == Some(project)
                {
                    self.history_request = None;
                    match reply.result {
                        Ok(Output::HistoryRefresh(history)) if history.project.id == project => {
                            self.history = Some(history);
                        }
                        Err(error) => {
                            self.status = format!("Could not refresh design history: {error}")
                        }
                        _ => {}
                    }
                }
                continue;
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
                        let id = candidate.id;
                        if self.binding == Some(candidate.base) && self.fixture.is_none() {
                            // Keep the handle until cancellation is acknowledged;
                            // a failed cancel must remain retryable by the operator.
                            self.candidate = Some(Candidate {
                                id: Some(id),
                                phase: Some(candidate.phase),
                                intent: candidate.intent,
                                actor: candidate.actor,
                                before: self.projection.clone(),
                                after: candidate.projection,
                                source: candidate.source_preview.after,
                                review_selection: None,
                            });
                            self.comparison = ComparisonMode::Current;
                        }
                        self.enqueue_mutation(Box::new(move |platform| {
                            platform.cancel(id)?;
                            Ok(Output::Cancelled)
                        }));
                    }
                    self.status =
                        "Discarding prepared candidate · current revision retained".into();
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
                    if reply.request == self.scene_request
                        && self.show_agent
                        && let Some(activity) = &mut self.agent_activity
                    {
                        activity.error = Some(error.clone());
                        activity.complete = false;
                    }
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
                    if reply.request == self.scene_request
                        && let Some(target) = self.pending_revision.take()
                    {
                        self.revision_retry = Some(target);
                    }
                    if reply.request == self.lifecycle_request {
                        self.lifecycle_request = 0;
                        self.status = format!("Candidate state needs refresh: {}", self.status);
                    }
                    if reply.mutation && self.candidate.is_some() {
                        self.reconcile_candidate_lifecycle();
                    }
                }
                Ok(Output::CandidateLifecycle(candidate))
                    if reply.request == self.lifecycle_request
                        && self.binding == Some(candidate.base)
                        && self
                            .candidate
                            .as_ref()
                            .is_some_and(|current| current.id == Some(candidate.id)) =>
                {
                    if let Some(current) = &mut self.candidate {
                        current.phase = Some(candidate.phase);
                    }
                    self.lifecycle_request = 0;
                    self.lifecycle_unknown = false;
                    self.status = format!("Candidate state refreshed: {:?}", candidate.phase);
                    if candidate.phase == agq_studio_platform::CandidatePhase::Committed
                        && self
                            .committed_receipt
                            .as_ref()
                            .is_none_or(|(project, receipt)| {
                                *project != candidate.base.project
                                    || receipt.revision_id != candidate.projection.revision_id
                            })
                    {
                        // Fetch the retained receipt through the same idempotent
                        // operation; never infer durability from a rendered graph.
                        let id = candidate.id;
                        self.enqueue_mutation(Box::new(move |platform| {
                            platform.commit(id).map(Output::Committed)
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
                        self.pending_revision = None;
                        self.revision_retry = None;
                        self.history_request = None;
                        self.lifecycle_request = 0;
                        self.lifecycle_unknown = false;
                        if self
                            .committed_receipt
                            .as_ref()
                            .is_some_and(|(project, _)| *project != history.project.id)
                        {
                            self.committed_receipt = None;
                        }
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
                            .pending_revision
                            .or(self.binding)
                            .is_some_and(|binding| binding.revision == projection.revision_id) =>
                {
                    let previous_display = self.display_snapshot();
                    self.projection = projection;
                    let previous_candidate =
                        self.pending_revision.and_then(|_| self.candidate.take());
                    let previous_before = self
                        .pending_revision
                        .and_then(|_| self.compare_before.take());
                    if self.pending_revision.is_some() {
                        self.comparison = ComparisonMode::Current;
                    }
                    // No frame can observe a new revision whose scene failed to build.
                    if !self.rebuild_immediate() {
                        if self.pending_revision.take().is_some() {
                            self.candidate = previous_candidate;
                            self.compare_before = previous_before;
                        }
                        self.revision_retry = self.binding.map(|binding| RevisionBinding {
                            revision: self.projection.revision_id,
                            ..binding
                        });
                        self.restore_display(previous_display);
                        self.restore = None;
                        continue;
                    }
                    if let Some(binding) = self.pending_revision.take() {
                        self.binding = Some(binding);
                        self.revision_retry = None;
                        self.show_agent = false;
                        self.dependencies = None;
                        self.agent_activity = None;
                        self.agent_return = None;
                    }
                    self.finish_agent_projection();
                    self.ready = true;
                    self.fit_pending = true;
                    let presentation_ok = self.apply_pending_presentation();
                    self.request_inspection();
                    self.record_location();
                    if presentation_ok {
                        self.status = "Revision restored in process".into();
                    }
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
                    let previous = self.display_snapshot();
                    self.projection = comparison.after;
                    self.compare_before = Some(comparison.before);
                    self.comparison = ComparisonMode::Diff;
                    if !self.rebuild_immediate() {
                        self.restore_display(previous);
                        continue;
                    }
                    self.fit_pending = true;
                    self.request_inspection();
                }
                Ok(Output::Candidate(candidate))
                    if reply.mutation
                        && self.binding == Some(candidate.base)
                        && self.fixture.is_none() =>
                {
                    let previous = self.display_snapshot();
                    let before = self
                        .candidate
                        .as_ref()
                        .filter(|current| current.id == Some(candidate.id))
                        .map(|current| current.before.clone())
                        .unwrap_or_else(|| self.projection.clone());
                    self.candidate = Some(Candidate {
                        review_selection: None,
                        id: Some(candidate.id),
                        phase: Some(candidate.phase),
                        intent: candidate.intent,
                        actor: candidate.actor,
                        before,
                        after: candidate.projection,
                        source: format!(
                            "{}\n\n{}",
                            candidate.source_preview.path, candidate.source_preview.after
                        ),
                    });
                    if self.show_agent {
                        self.expanded = None;
                    }
                    self.show_agent = false;
                    self.dependencies = None;
                    self.agent_activity = None;
                    self.agent_return = None;
                    self.invalidate_inspection();
                    self.scene_request = 0;
                    self.comparison = ComparisonMode::Diff;
                    if !self.rebuild_immediate() {
                        self.restore_candidate_display(previous);
                        continue;
                    }
                    self.request_inspection();
                    self.status = "Candidate revision ready for review".into();
                }
                Ok(Output::CandidateView(candidate, before))
                    if (reply.mutation || reply.request == self.scene_request)
                        && self.binding == Some(candidate.base)
                        && self
                            .candidate
                            .as_ref()
                            .is_some_and(|current| current.id == Some(candidate.id)) =>
                {
                    // Validation is a lifecycle result, independent of a disposable
                    // lens request. Navigation may supersede its view, never its phase.
                    if reply.mutation && candidate.projection.view != self.definition() {
                        if let Some(current) = &mut self.candidate {
                            current.phase = Some(candidate.phase);
                        }
                        self.request_projection();
                        self.status = "Candidate validated · refreshing your current view".into();
                        continue;
                    }
                    let previous = self.display_snapshot();
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
                    if !self.rebuild_immediate() {
                        self.restore_candidate_display(previous);
                        continue;
                    }
                    self.finish_agent_projection();
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
                        self.committed_receipt = Some((binding.project, receipt.clone()));
                        self.pending_revision = Some(RevisionBinding {
                            revision: receipt.revision_id,
                            ..binding
                        });
                    }
                    if let Some(candidate) = &mut self.candidate {
                        candidate.phase = Some(agq_studio_platform::CandidatePhase::Committed);
                    }
                    self.lifecycle_unknown = false;
                    self.lifecycle_request = 0;
                    // Durability and history do not wait for rendering to succeed.
                    self.refresh_history();
                    self.request_projection();
                    self.status = "Candidate durably committed".into();
                }
                Ok(Output::Cancelled) if reply.mutation => {
                    self.candidate = None;
                    self.lifecycle_unknown = false;
                    self.lifecycle_request = 0;
                    self.comparison = ComparisonMode::Current;
                    self.invalidate_inspection();
                    self.scene_request = 0;
                    if !self.rebuild_immediate() {
                        self.ready = false;
                        self.setup_reason = format!(
                            "Candidate cancelled; current revision view failed: {}. Reopen the project to retry.",
                            self.status
                        );
                        continue;
                    }
                    self.request_inspection();
                    self.status = "Uncommitted candidate cancelled".into();
                }
                Ok(_) => {} // Superseded read requests cannot populate another view/selection.
            }
        }
    }
    /// Retain exact candidate lifecycle and source even when its disposable view
    /// cannot be drawn. The operator can still refresh, cancel or reconcile it.
    fn restore_candidate_display(&mut self, previous: crate::app::DisplayState) {
        let error = self.status.clone();
        self.restore_display(previous);
        self.comparison = ComparisonMode::Current;
        self.compare_before = None;
        if self.rebuild_immediate() {
            self.status =
                format!("Candidate retained; showing current revision. View failed: {error}");
            self.request_inspection();
        }
    }
    fn apply_pending_presentation(&mut self) -> bool {
        if let Some(restore) = self.restore.take()
            && restoration_matches(&restore, self.binding, self.world, self.focus)
        {
            let previous = self.display_snapshot();
            if let Some(presentation) = restore.presentation {
                self.apply_saved_presentation(presentation);
                if !self.rebuild_immediate() {
                    self.restore_display(previous);
                    return false;
                }
                return true;
            }
            self.camera = restore.camera;
            self.camera_target = None;
            self.layout = restore.layout;
            // Set the world before rebuilding, so the per-world memory swap
            // cannot discard the newly restored layout.
            self.layout_world = self.world;
            if !self.rebuild_immediate() {
                self.restore_display(previous);
                return false;
            }
            self.fit_pending = false;
        }
        true
    }
    pub fn open_project(&mut self, id: agq_modeling_repository::ProjectId) {
        if !self.allow_context_change() {
            return;
        }
        self.invalidate_inspection();
        self.pending_revision = None;
        self.revision_retry = None;
        self.history_request = None;
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
            "gpu_timestamp_ms": stats.as_ref().map(|s| s.timestamp_ms.summary()),
            "gpu_timestamp_scope": stats.as_ref().map(|s| s.timestamp_status),
            "gpu_timestamp_errors": stats.as_ref().map(|s| s.timestamp_errors),
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
    use crate::bridge::{Reply, WorkContext};
    use agq_studio_platform::{CandidatePhase, CandidateProjection};
    use clap::Parser;

    fn application() -> StudioApp {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let args = crate::Args::parse_from([
            "agq-studio-native",
            "--fixture",
            "architecture",
            "--no-restore",
            "--root",
            root.to_str().unwrap(),
        ]);
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = StudioApp::new(&context, args).unwrap();
        // Reply-order tests exercise live UI routing with explicit test DTOs.
        // They do not open a runtime or establish semantic validation.
        app.fixture = None;
        app.binding = Some(RevisionBinding {
            project: agq_modeling_repository::ProjectId::new(),
            revision: app.projection.revision_id,
        });
        app.branch = Some(agq_modeling_repository::BranchId::new());
        app
    }
    fn candidate(app: &StudioApp, phase: CandidatePhase) -> CandidateProjection {
        let (_, mut projection) = agq_studio_scene::fixtures::revision_diff();
        projection.view = app.definition();
        CandidateProjection {
            id: serde_json::from_str("\"00000000-0000-0000-0000-000000000091\"").unwrap(),
            base: app.binding.unwrap(),
            phase,
            intent: "Add test part".into(),
            actor: "human-operator".into(),
            changes: serde_json::from_value(serde_json::json!({
                "from": app.projection.revision_id, "to": projection.revision_id,
                "documents": [], "declared": {"added":[],"removed":[],"changed":[]},
                "derived": {"added":[],"removed":[],"changed":[]},
                "relationships_added":[],"relationships_removed":[],"relationships_changed":[],
                "validation_changed":false
            }))
            .unwrap(),
            source_preview: agq_modeling_agent::SourcePreview {
                path: "test.sysml".into(),
                before: "".into(),
                after: "part test;".into(),
            },
            projection,
        }
    }
    fn retain(app: &mut StudioApp, phase: CandidatePhase) {
        let candidate = candidate(app, phase);
        app.candidate = Some(Candidate {
            id: Some(candidate.id),
            phase: Some(phase),
            intent: candidate.intent,
            actor: candidate.actor,
            before: app.projection.clone(),
            after: candidate.projection,
            source: candidate.source_preview.after,
            review_selection: None,
        });
    }
    fn reply(
        request: u64,
        context: WorkContext,
        mutation: bool,
        result: Result<Output, String>,
    ) -> Reply {
        Reply {
            request,
            context: Some(context),
            mutation,
            terminal: true,
            result,
        }
    }
    fn history(
        app: &StudioApp,
        head: agq_modeling_workspace::ProjectRevisionId,
    ) -> agq_studio_platform::ProjectHistory {
        let project = app.project_id().unwrap();
        let branch = app.branch.unwrap();
        let metadata = agq_modeling_repository::ResourceMetadata {
            created: "2026-09-25T00:00:00Z".into(),
            name: None,
            description: None,
            alias: vec![],
        };
        agq_studio_platform::ProjectHistory {
            project: agq_modeling_repository::Project {
                id: project,
                name: "State test".into(),
                default_branch: branch,
                metadata: metadata.clone(),
            },
            branches: vec![agq_modeling_repository::Branch {
                id: branch,
                project_id: project,
                name: "main".into(),
                head,
                metadata,
            }],
            revisions: vec![],
        }
    }

    #[test]
    fn validation_completion_survives_agent_dismissal_and_a_superseded_lens() {
        let mut app = application();
        retain(&mut app, CandidatePhase::Working);
        app.remember_agent_return();
        app.show_agent = true;
        let scope = app.work_context();
        let mut validated = candidate(&app, CandidatePhase::Validated);
        validated.projection.view = agq_modeling_view::ViewDefinition::semantic_graph();
        app.scene_request = 700;
        app.dismiss_agent_view();
        let before = app.projection.clone();
        app.receive_replies([reply(
            700,
            scope,
            true,
            Ok(Output::CandidateView(validated, before.clone())),
        )]);
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(CandidatePhase::Validated)
        );
        assert_eq!(app.projection, before);
        assert_eq!(app.world, World::System);
        assert!(!app.show_agent);
    }

    #[test]
    fn unresolved_commit_reconciliation_is_independent_of_disposable_query_ids() {
        let mut app = application();
        retain(&mut app, CandidatePhase::Validated);
        let scope = app.work_context();
        app.receive_replies([reply(
            701,
            scope.clone(),
            true,
            Err("commit acknowledgement lost".into()),
        )]);
        assert!(app.lifecycle_unknown);
        let request = app.lifecycle_request;
        assert_ne!(request, 0);
        app.scene_request = 999;
        app.remember_agent_return();
        app.show_agent = true;
        app.dismiss_agent_view();
        let projection = app.projection.clone();
        let recovered = candidate(&app, CandidatePhase::CommitUnresolved);
        app.receive_replies([reply(
            request,
            scope,
            false,
            Ok(Output::CandidateLifecycle(recovered)),
        )]);
        assert!(!app.lifecycle_unknown);
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(CandidatePhase::CommitUnresolved)
        );
        assert_eq!(app.projection, projection);
        assert_eq!(app.lifecycle_request, 0);
    }

    #[test]
    fn commit_receipt_and_history_survive_failed_revision_loading() {
        let mut app = application();
        retain(&mut app, CandidatePhase::Validated);
        let base = app.binding.unwrap();
        let revision = app.candidate.as_ref().unwrap().after.revision_id;
        let receipt = agq_modeling_repository::CommitReceipt {
            operation_id: agq_modeling_repository::OperationId::new(),
            branch_id: app.branch.unwrap(),
            revision_id: revision,
            replayed: false,
        };
        let scope = app.work_context();
        app.receive_replies([reply(
            702,
            scope.clone(),
            true,
            Ok(Output::Committed(receipt.clone())),
        )]);
        let (history_request, _) = app
            .history_request
            .expect("history starts before rendering succeeds");
        let scene_request = app.scene_request;
        app.receive_replies([reply(
            scene_request,
            scope.clone(),
            false,
            Err("projection unavailable".into()),
        )]);
        assert_eq!(app.binding, Some(base));
        assert_eq!(app.revision_retry.unwrap().revision, revision);
        assert_eq!(app.committed_receipt.as_ref().unwrap().1, receipt);
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(CandidatePhase::Committed)
        );
        // History remains project-scoped even after the operator changes revision.
        app.binding.as_mut().unwrap().revision = agq_modeling_workspace::ProjectRevisionId::new();
        let refreshed = history(&app, revision);
        app.receive_replies([reply(
            history_request,
            scope,
            false,
            Ok(Output::HistoryRefresh(refreshed)),
        )]);
        assert_eq!(app.history.as_ref().unwrap().branches[0].head, revision);
        assert!(app.history_request.is_none());
    }

    #[test]
    fn history_refresh_rejects_superseded_requests_and_other_projects() {
        let mut app = application();
        let base = app.projection.revision_id;
        let scope = app.work_context();
        app.history = Some(history(&app, base));
        app.history_request = Some((802, app.project_id().unwrap()));
        let stale = history(&app, agq_modeling_workspace::ProjectRevisionId::new());
        app.receive_replies([reply(
            801,
            scope.clone(),
            false,
            Ok(Output::HistoryRefresh(stale)),
        )]);
        assert_eq!(app.history.as_ref().unwrap().branches[0].head, base);
        let old_project_history = history(&app, base);
        app.binding.as_mut().unwrap().project = agq_modeling_repository::ProjectId::new();
        app.receive_replies([reply(
            802,
            scope,
            false,
            Ok(Output::HistoryRefresh(old_project_history)),
        )]);
        assert_eq!(app.history.as_ref().unwrap().branches[0].head, base);
        assert_eq!(app.history_request.unwrap().0, 802);
    }

    #[test]
    fn rejected_revision_scene_keeps_binding_projection_and_retry_target_coherent() {
        let mut app = application();
        retain(&mut app, CandidatePhase::Committed);
        let base = app.binding.unwrap();
        let mut invalid = app.candidate.as_ref().unwrap().after.clone();
        invalid.nodes.push(invalid.nodes[0].clone());
        let target = RevisionBinding {
            revision: invalid.revision_id,
            ..base
        };
        app.pending_revision = Some(target);
        app.scene_request = 803;
        let scope = app.work_context();
        app.receive_replies([reply(803, scope, false, Ok(Output::Projection(invalid)))]);
        assert_eq!(app.binding, Some(base));
        assert_eq!(app.projection.revision_id, base.revision);
        assert_eq!(app.scene.revision_id, base.revision);
        assert_eq!(app.revision_retry, Some(target));
        assert!(app.pending_revision.is_none());
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(CandidatePhase::Committed)
        );
    }

    #[test]
    fn rejected_candidate_view_retains_new_lifecycle_but_displays_current_revision() {
        let mut app = application();
        retain(&mut app, CandidatePhase::Working);
        let before = app.projection.clone();
        let mut invalid = candidate(&app, CandidatePhase::Validated);
        invalid
            .projection
            .nodes
            .push(invalid.projection.nodes[0].clone());
        let scope = app.work_context();
        app.receive_replies([reply(
            804,
            scope,
            true,
            Ok(Output::CandidateView(invalid, before.clone())),
        )]);
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(CandidatePhase::Validated)
        );
        assert_eq!(app.comparison, ComparisonMode::Current);
        assert_eq!(app.projection, before);
        assert_eq!(app.scene.revision_id, before.revision_id);
        assert!(app.status.contains("View failed"));
    }

    #[test]
    fn rejected_new_candidate_preserves_cancel_authority_without_publishing_bad_scene() {
        let mut app = application();
        let before = app.projection.clone();
        let mut invalid = candidate(&app, CandidatePhase::Working);
        invalid
            .projection
            .nodes
            .push(invalid.projection.nodes[0].clone());
        let id = invalid.id;
        let scope = app.work_context();
        app.receive_replies([reply(805, scope, true, Ok(Output::Candidate(invalid)))]);
        assert_eq!(app.candidate.as_ref().unwrap().id, Some(id));
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(CandidatePhase::Working)
        );
        assert_eq!(app.comparison, ComparisonMode::Current);
        assert_eq!(app.projection, before);
        assert_eq!(app.scene.revision_id, before.revision_id);
    }

    #[test]
    fn rejected_comparison_keeps_the_previous_display_and_selection() {
        let mut app = application();
        let before = app.projection.clone();
        let selection = app.selection.targets.clone();
        let candidate = candidate(&app, CandidatePhase::Working);
        let mut after = candidate.projection;
        after.nodes.push(after.nodes[0].clone());
        let comparison = agq_studio_platform::ComparisonProjection {
            project: app.project_id().unwrap(),
            before: before.clone(),
            after,
            changes: candidate.changes,
        };
        app.scene_request = 806;
        let scope = app.work_context();
        app.receive_replies([reply(806, scope, false, Ok(Output::Comparison(comparison)))]);
        assert_eq!(app.projection, before);
        assert_eq!(app.scene.revision_id, before.revision_id);
        assert_eq!(app.comparison, ComparisonMode::Current);
        assert_eq!(app.selection.targets, selection);
        assert!(app.status.contains("duplicate element"));
    }

    #[test]
    fn failed_lifecycle_refresh_keeps_mutations_blocked_until_a_matching_retry() {
        let mut app = application();
        retain(&mut app, CandidatePhase::Validated);
        app.lifecycle_unknown = true;
        app.lifecycle_request = 807;
        let scope = app.work_context();
        app.receive_replies([reply(
            807,
            scope.clone(),
            false,
            Err("state query failed".into()),
        )]);
        assert!(app.lifecycle_unknown);
        assert_eq!(app.lifecycle_request, 0);
        assert!(
            crate::commands::unavailable(crate::commands::CommandId::Cancel, &app.context())
                .is_some()
        );
        app.lifecycle_request = 808;
        let stale = candidate(&app, CandidatePhase::Validated);
        app.receive_replies([reply(
            807,
            scope,
            false,
            Ok(Output::CandidateLifecycle(stale)),
        )]);
        assert!(app.lifecycle_unknown);
        assert_eq!(app.lifecycle_request, 808);
    }

    #[test]
    fn mode_change_and_agent_return_reject_bad_disposable_projections() {
        let mut app = application();
        retain(&mut app, CandidatePhase::Validated);
        let before = app.projection.clone();
        let candidate = app.candidate.as_mut().unwrap();
        candidate.after.nodes.push(candidate.after.nodes[0].clone());
        app.change_comparison(ComparisonMode::Diff);
        assert_eq!(app.comparison, ComparisonMode::Current);
        assert_eq!(app.scene.revision_id, before.revision_id);
        app.projection.nodes.push(app.projection.nodes[0].clone());
        app.remember_agent_return();
        app.projection = before.clone();
        app.show_agent = true;
        app.dismiss_agent_view();
        assert_eq!(app.projection, before);
        assert!(
            app.show_agent,
            "failed return leaves the prior coherent view available"
        );
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(CandidatePhase::Validated)
        );
    }

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
