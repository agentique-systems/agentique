//! Native panel routing for opaque, current-revision read capabilities.
use crate::{
    app::StudioApp,
    bridge::{Output, Reply},
    read_lane::PanelRead,
};
use agq_kernel::ElementId;
use agq_studio_platform::{CandidateId, RevisionBinding};

impl StudioApp {
    /// Call immediately after a real projection swap, before another UI action
    /// can queue preparation. Pinning is serialized with all platform operations.
    pub fn pin_current_reader(&mut self) {
        let Some(binding) = self.binding.filter(|binding| {
            self.fixture.is_none() && self.projection.revision_id == binding.revision
        }) else {
            return;
        };
        match self.bridge.pin_reader(binding, self.work_context()) {
            Ok(Some(request)) => {
                self.pending.insert(request);
            }
            Ok(None) => {}
            Err(error) => {
                self.status = format!("Could not open read-only revision access: {error}")
            }
        }
    }

    pub fn request_panel_read(
        &mut self,
        panel: PanelRead,
        binding: RevisionBinding,
        candidate: Option<CandidateId>,
        element: ElementId,
    ) -> u64 {
        if candidate.is_none() && self.fixture.is_none() && self.binding == Some(binding) {
            self.pin_current_reader();
            match self.bridge.read(binding, element, panel) {
                Ok(request) => {
                    self.pending.insert(request);
                    return request;
                }
                Err(error) => {
                    self.status = format!("Current revision read unavailable: {error}");
                    return 0;
                }
            }
        }
        // Candidate objects and historical ghosts retain their existing serial
        // authority path. They cannot be observed through a current reader.
        self.enqueue(Box::new(move |platform| match (candidate, panel) {
            (Some(id), PanelRead::Inspector) => platform
                .inspect_candidate(id, element)
                .map(Output::Inspector),
            (Some(id), PanelRead::Explain) => platform
                .explain_candidate(id, element)
                .map(Output::Explanation),
            (Some(id), PanelRead::Source) => {
                platform.source_candidate(id, element).map(Output::Source)
            }
            (None, PanelRead::Inspector) => {
                platform.inspect(binding, element).map(Output::Inspector)
            }
            (None, PanelRead::Explain) => {
                platform.explain(binding, element).map(Output::Explanation)
            }
            (None, PanelRead::Source) => platform.source(binding, element).map(Output::Source),
        }))
    }

    pub fn receive_reader_pin(&mut self, reply: Reply) {
        let Some(scope) = self.bridge.reader_pin(reply.request) else {
            return;
        };
        if !reply.terminal
            || reply.mutation
            || scope.epoch != reply.epoch
            || !self.bridge.current_epoch(reply.epoch)
            || self.fixture.is_some()
            || self.binding != Some(scope.binding)
            || self.projection.revision_id != scope.binding.revision
            || !reply.context.as_ref().is_some_and(|context| {
                context.fixture.is_none() && context.binding == Some(scope.binding)
            })
        {
            self.bridge.fail_reader(
                reply.request,
                "Reader pin superseded by the displayed context",
            );
            return;
        }
        let result = match reply.result {
            Ok(Output::Reader(reader)) => self.bridge.install_reader(reply.request, reader),
            Err(error) => Err(error),
            _ => Err("Reader pin returned a different result type".into()),
        };
        if let Err(error) = result {
            self.bridge.fail_reader(reply.request, &error);
            self.status = format!("Read-only revision access unavailable: {error}");
        }
    }

    /// A current read is independent of candidate lifecycle and disposable scene
    /// requests, but never independent of runtime, revision, selection or panel.
    pub fn receive_panel_read(&mut self, reply: Reply) {
        let Some(context) = reply.read else { return };
        let latest = match context.panel {
            PanelRead::Inspector => self.inspector_request,
            PanelRead::Explain => self.explanation_request,
            PanelRead::Source => self.source_request,
        };
        if !reply.terminal
            || reply.mutation
            || reply.request != latest
            || reply.epoch != context.scope.epoch
            || !self.bridge.current_epoch(reply.epoch)
            || self.fixture.is_some()
            || self.binding != Some(context.scope.binding)
            || self.selected_context() != Some((context.scope.binding, None, context.element))
            || (context.panel == PanelRead::Explain && !self.show_explain)
            || (context.panel == PanelRead::Source && !self.show_source)
        {
            return;
        }
        match (context.panel, reply.result) {
            (PanelRead::Inspector, Ok(Output::Inspector(inspector)))
                if inspector.revision_id == context.scope.binding.revision
                    && inspector.element.revision_id == inspector.revision_id
                    && inspector.element.id == context.element =>
            {
                self.inspector = Some(inspector)
            }
            (PanelRead::Explain, Ok(Output::Explanation(explanation)))
                if explanation.revision_id == context.scope.binding.revision
                    && explanation.subject_id == context.element =>
            {
                self.explanation = Some(explanation)
            }
            (PanelRead::Source, Ok(Output::Source(source)))
                if source.binding == context.scope.binding && source.element == context.element =>
            {
                self.source = Some(source)
            }
            (_, Err(error)) => self.status = format!("Current revision read failed: {error}"),
            _ => self.status = "Discarded a read response with mismatched semantic identity".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{Candidate, ComparisonMode},
        read_lane::{ReadContext, ReadScope},
    };
    use agq_modeling_repository::{ProjectId, ProjectRevisionId};
    use agq_modeling_view::ElementInspector;
    use agq_studio_platform::{CandidatePhase, SourceProjection};
    use agq_studio_scene::SceneTarget;
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
        let context = eframe::CreationContext::_new_kittest(eframe::egui::Context::default());
        let mut app = StudioApp::new(&context, args).unwrap();
        // Unit-only reply DTOs test routing; they cannot mint a revision reader.
        app.fixture = None;
        app.binding = Some(RevisionBinding {
            project: ProjectId::new(),
            revision: app.projection.revision_id,
        });
        let element = app.projection.nodes[0].id;
        app.selection.select(SceneTarget::Node(element), false);
        app
    }

    fn inspector(app: &StudioApp) -> ElementInspector {
        ElementInspector {
            revision_id: app.projection.revision_id,
            element: app.projection.nodes[0].clone(),
            owner: None,
            effective_types: vec![],
            owned_features: vec![],
            effective_features: vec![],
            specializations: vec![],
            subsettings: vec![],
            redefinitions: vec![],
            relationships: vec![],
            multiplicity: None,
            source: None,
            queries: vec![],
            profile: "Unit routing DTO; not semantic acceptance".into(),
        }
    }

    fn read_reply(app: &StudioApp, request: u64, panel: PanelRead, result: Output) -> Reply {
        Reply {
            request,
            epoch: app.bridge.epoch(),
            context: None,
            read: Some(ReadContext {
                scope: ReadScope {
                    epoch: app.bridge.epoch(),
                    binding: app.binding.unwrap(),
                },
                panel,
                element: app.projection.nodes[0].id,
            }),
            mutation: false,
            terminal: true,
            result: Ok(result),
        }
    }

    #[test]
    fn current_inspection_completes_while_serial_mutation_is_pending_and_candidate_changes() {
        let mut app = application();
        let pending = app
            .bridge
            .work(
                Box::new(|_| panic!("no authenticated service in routing test")),
                app.work_context(),
                true,
            )
            .unwrap();
        app.pending.insert(pending);
        app.inspector_request = 70;
        app.pending.insert(70);
        let expected = inspector(&app);
        let reply = read_reply(
            &app,
            70,
            PanelRead::Inspector,
            Output::Inspector(expected.clone()),
        );
        let (_, after) = agq_studio_scene::fixtures::revision_diff();
        app.candidate = Some(Candidate {
            id: Some(serde_json::from_str("\"00000000-0000-0000-0000-000000000077\"").unwrap()),
            phase: Some(CandidatePhase::Working),
            intent: "Routing test".into(),
            actor: "unit test".into(),
            before: app.projection.clone(),
            after,
            source: String::new(),
            review_selection: None,
        });
        app.comparison = ComparisonMode::Current;
        app.receive_replies([reply]);
        assert_eq!(app.inspector.as_ref(), Some(&expected));
        assert!(app.bridge.mutation_pending());
        assert!(app.pending.contains(&pending));
        assert!(!app.pending.contains(&70));
        assert_eq!(
            app.candidate.as_ref().unwrap().phase,
            Some(CandidatePhase::Working)
        );
    }

    #[test]
    fn old_runtime_revision_project_selection_or_request_cannot_publish_a_read() {
        for invalid in 0..7 {
            let mut app = application();
            app.inspector_request = 70;
            app.pending.insert(70);
            let mut reply = read_reply(
                &app,
                70,
                PanelRead::Inspector,
                Output::Inspector(inspector(&app)),
            );
            match invalid {
                0 => {
                    reply.epoch += 1;
                    reply.read.as_mut().unwrap().scope.epoch = reply.epoch;
                }
                1 => app.inspector_request = 71,
                2 => {
                    app.selection
                        .select(SceneTarget::Node(app.projection.nodes[1].id), false);
                }
                3 => reply.read.as_mut().unwrap().scope.binding.project = ProjectId::new(),
                4 => reply.read.as_mut().unwrap().scope.binding.revision = ProjectRevisionId::new(),
                5 => reply.mutation = true,
                6 => app.fixture = Some("architecture".into()),
                _ => unreachable!(),
            }
            app.receive_replies([reply]);
            assert!(
                app.inspector.is_none(),
                "Accepted stale read case {invalid}"
            );
            assert!(
                !app.pending.contains(&70),
                "Terminal request leaked in case {invalid}"
            );
        }
    }

    #[test]
    fn candidate_and_diff_selection_cannot_consume_a_current_revision_read() {
        for mode in [ComparisonMode::Candidate, ComparisonMode::Diff] {
            let mut app = application();
            let reply = read_reply(
                &app,
                70,
                PanelRead::Inspector,
                Output::Inspector(inspector(&app)),
            );
            let mut after = app.projection.clone();
            after.revision_id = ProjectRevisionId::new();
            for node in &mut after.nodes {
                node.revision_id = after.revision_id;
            }
            for edge in &mut after.edges {
                edge.revision_id = after.revision_id;
            }
            app.candidate = Some(Candidate {
                id: Some(serde_json::from_str("\"00000000-0000-0000-0000-000000000077\"").unwrap()),
                phase: Some(CandidatePhase::Working),
                intent: "Routing test".into(),
                actor: "unit test".into(),
                before: app.projection.clone(),
                after,
                source: String::new(),
                review_selection: None,
            });
            app.comparison = mode;
            assert!(app.rebuild_immediate());
            assert!(app.selected_context().unwrap().1.is_some());
            app.inspector_request = 70;
            app.pending.insert(70);
            app.receive_replies([reply]);
            assert!(app.inspector.is_none());
            assert!(!app.pending.contains(&70));
        }
    }

    #[test]
    fn stale_reader_pin_cannot_replace_or_fail_a_newer_revision_pin() {
        let mut app = application();
        let old_context = app.work_context();
        app.pin_current_reader();
        let old = *app.pending.first().unwrap();
        let mut projection = app.projection.clone();
        projection.revision_id = ProjectRevisionId::new();
        for node in &mut projection.nodes {
            node.revision_id = projection.revision_id;
        }
        for edge in &mut projection.edges {
            edge.revision_id = projection.revision_id;
        }
        app.projection = projection;
        app.binding.as_mut().unwrap().revision = app.projection.revision_id;
        assert!(app.rebuild_immediate());
        app.pin_current_reader();
        let newest = *app.pending.last().unwrap();
        assert_ne!(old, newest);
        let expected = app.bridge.reader_pin(newest).unwrap();
        let status = app.status.clone();
        app.receive_replies([Reply {
            request: old,
            epoch: app.bridge.epoch(),
            context: Some(old_context),
            read: None,
            mutation: false,
            terminal: true,
            result: Err("Old pin failure must not poison the new reader".into()),
        }]);
        assert!(app.bridge.reader_pin(old).is_none());
        assert_eq!(app.bridge.reader_pin(newest), Some(expected));
        assert_eq!(app.status, status);
        assert!(!app.pending.contains(&old));
        assert!(app.pending.contains(&newest));
    }

    #[test]
    fn runtime_open_discards_a_reader_pin_completion_before_authentication_finishes() {
        let mut app = application();
        let old_context = app.work_context();
        let old_epoch = app.bridge.epoch();
        app.pin_current_reader();
        let old = *app.pending.first().unwrap();
        let missing = std::env::temp_dir().join(format!(
            "absent-reader-test-runtime-{}",
            ProjectRevisionId::new()
        ));
        let mut config =
            agq_studio_platform::NativeConfig::for_root(missing.clone(), Some(missing.clone()))
                .unwrap();
        config.runtime.bundle = Some(missing.join("absent.agq-runtime"));
        app.bridge.open(config, None).unwrap();
        assert!(!app.bridge.current_epoch(old_epoch));
        let status = app.status.clone();
        app.receive_replies([Reply {
            request: old,
            epoch: old_epoch,
            context: Some(old_context),
            read: None,
            mutation: false,
            terminal: true,
            result: Err("Old pin failure must not poison the new runtime".into()),
        }]);
        assert!(app.bridge.reader_pin(old).is_none());
        assert_eq!(app.status, status);
        assert!(!app.pending.contains(&old));
    }

    #[test]
    fn source_read_checks_payload_identity_and_dismissed_panel() {
        for invalid in 0..3 {
            let mut app = application();
            app.show_source = invalid != 0;
            app.source_request = 80;
            app.pending.insert(80);
            let mut source = SourceProjection {
                binding: app.binding.unwrap(),
                element: app.projection.nodes[0].id,
                path: "unit-only.sysml".into(),
                source: "part test;".into(),
                start: 0,
                end: 10,
            };
            if invalid == 1 {
                source.binding.revision = ProjectRevisionId::new();
            }
            if invalid == 2 {
                source.element = app.projection.nodes[1].id;
            }
            let reply = read_reply(&app, 80, PanelRead::Source, Output::Source(source));
            app.receive_replies([reply]);
            assert!(app.source.is_none());
            assert!(!app.pending.contains(&80));
        }
    }

    #[test]
    fn failed_reader_pin_terminates_waiting_panel_request_without_fallback_authority() {
        let mut app = application();
        let binding = app.binding.unwrap();
        let element = app.projection.nodes[0].id;
        // No platform was opened; the actual serial worker must refuse the pin.
        let request = app.request_panel_read(PanelRead::Inspector, binding, None, element);
        app.inspector_request = request;
        assert_ne!(request, 0);
        let pin = app
            .bridge
            .replies
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(pin.result.is_err());
        assert!(app.bridge.reader_pin(pin.request).is_some());
        app.receive_replies([pin]);
        let failed_read = app
            .bridge
            .replies
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(failed_read.request, request);
        assert!(failed_read.result.is_err());
        app.receive_replies([failed_read]);
        assert!(app.pending.is_empty());
        assert!(app.inspector.is_none());
        assert!(app.status.contains("Authenticated runtime is not open"));
        assert_eq!(app.binding, Some(binding));
        assert!(app.candidate.is_none());
    }
}
