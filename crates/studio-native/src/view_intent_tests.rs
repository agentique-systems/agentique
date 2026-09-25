//! Native request-order regressions, not accepted-runtime semantic evidence.
//! These tests inject explicit DTOs into the same reply boundary as the worker.
//! The unauthenticated Bridge never executes their submitted closures; its real
//! error replies are deliberately not consumed by these deterministic tests.

use crate::{
    app::{Candidate, ComparisonMode, PendingPreparation, StudioApp},
    bridge::{Output, Reply, WorkContext},
    navigation::World,
};
use agq_modeling_view::{GraphScope, RelationshipFamily, ViewDefinition, ViewProjection};
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_platform::{CandidatePhase, CandidateProjection, RevisionBinding};
use clap::Parser;
use std::{collections::BTreeSet, time::Instant};

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
    let creation = eframe::CreationContext::_new_kittest(eframe::egui::Context::default());
    let mut app = StudioApp::new(&creation, args).unwrap();
    // Exercise native live-request routing, without opening or authenticating a
    // ModelRepository. Fixture DTOs establish only presentation state behavior.
    app.fixture = None;
    app.binding = Some(RevisionBinding {
        project: agq_modeling_repository::ProjectId::new(),
        revision: app.projection.revision_id,
    });
    app.branch = Some(agq_modeling_repository::BranchId::new());
    app.world = World::Graph;
    app.focus = Some(app.projection.nodes[0].id);
    app.families = BTreeSet::from([RelationshipFamily::Ownership, RelationshipFamily::Typing]);
    app.include_standard = false;
    app.projection.view = ViewDefinition {
        depth: 1,
        focus: app.focus,
        relationship_families: app.families.iter().copied().collect(),
        ..ViewDefinition::semantic_graph()
    };
    assert!(app.rebuild_immediate());
    app
}

fn graph(app: &StudioApp, depth: u8, scope: GraphScope) -> ViewDefinition {
    let mut view = app.definition();
    view.depth = depth;
    view.graph_scope = scope;
    view
}

fn mutation(app: &mut StudioApp) -> (u64, WorkContext) {
    let context = app.work_context();
    let request = app
        .bridge
        .work(
            Box::new(|_| panic!("no authenticated service in a view-intent routing test")),
            context.clone(),
            true,
        )
        .unwrap();
    app.pending.insert(request);
    (request, context)
}

fn reply(
    request: u64,
    context: WorkContext,
    mutation: bool,
    result: Result<Output, String>,
) -> Reply {
    Reply {
        request,
        epoch: 0,
        context: Some(context),
        read: None,
        mutation,
        terminal: true,
        result,
    }
}

fn candidate(app: &StudioApp, view: ViewDefinition) -> CandidateProjection {
    let mut projection = app.projection.clone();
    projection.revision_id = ProjectRevisionId::from_u128(0xcade_0004);
    projection.view = view;
    for node in &mut projection.nodes {
        node.revision_id = projection.revision_id;
    }
    for edge in &mut projection.edges {
        edge.revision_id = projection.revision_id;
    }
    CandidateProjection {
        id: serde_json::from_str("\"00000000-0000-0000-0000-000000000094\"").unwrap(),
        base: app.binding.unwrap(),
        phase: CandidatePhase::Working,
        intent: "View-intent routing DTO".into(),
        actor: "unit test".into(),
        changes: serde_json::from_value(serde_json::json!({
            "from": app.projection.revision_id, "to": projection.revision_id,
            "documents": [], "declared": {"added":[],"removed":[],"changed":[]},
            "derived": {"added":[],"removed":[],"changed":[]},
            "relationships_added":[],"relationships_removed":[],"relationships_changed":[],
            "validation_changed":false
        }))
        .unwrap(),
        source_preview: agq_modeling_agent::SourcePreview {
            path: "unit-only.sysml".into(),
            before: String::new(),
            after: "part unitOnly;".into(),
        },
        projection,
    }
}

fn retain_candidate(app: &mut StudioApp) {
    let candidate = candidate(app, app.definition());
    app.candidate = Some(Candidate {
        id: Some(candidate.id),
        phase: Some(candidate.phase),
        intent: candidate.intent,
        actor: candidate.actor,
        before: app.projection.clone(),
        after: candidate.projection,
        source: candidate.source_preview.after,
        review_selection: None,
    });
}

fn assert_requested(app: &StudioApp, definition: &ViewDefinition) -> u64 {
    let (request, actual) = app
        .requested_definition
        .as_ref()
        .expect("queued view intent");
    assert_ne!(*request, 0);
    assert_eq!(*request, app.scene_request);
    assert_eq!(actual, definition);
    assert_eq!(&app.definition(), definition);
    assert!(app.deferred_definition.is_none());
    assert!(app.pending.contains(request));
    *request
}

fn with_view(mut projection: ViewProjection, view: &ViewDefinition) -> ViewProjection {
    projection.view = view.clone();
    projection
}

#[test]
fn pending_mutation_coalesces_latest_lens_without_query_or_scene_change_then_flushes_once() {
    let mut app = application();
    let before = app.projection.clone();
    let generation = app.generation;
    let camera = app.camera;
    let (work, context) = mutation(&mut app);
    let a = graph(&app, 2, GraphScope::Neighborhood);
    let b = graph(&app, 5, GraphScope::DependencyNeighborhood);
    app.request_projection_definition(a);
    app.request_projection_definition(b.clone());
    assert_eq!(app.deferred_definition.as_ref(), Some(&b));
    assert_eq!(app.definition(), b);
    assert!(app.requested_definition.is_none());
    assert_eq!(app.pending, BTreeSet::from([work]));
    assert_eq!(app.projection, before);
    assert_eq!(app.generation, generation);
    assert_eq!(app.camera, camera);

    // A failed preparation also releases the latest requested presentation.
    app.receive_replies([reply(
        work,
        context,
        true,
        Err("unit preparation failed".into()),
    )]);
    assert!(!app.bridge.mutation_pending());
    let query = assert_requested(&app, &b);
    assert_eq!(app.pending, BTreeSet::from([query]));
    assert_eq!(app.projection, before);
    assert_eq!(app.generation, generation);
    app.receive_replies(std::iter::empty());
    assert_eq!(assert_requested(&app, &b), query);
    assert_eq!(app.pending, BTreeSet::from([query]));
}

#[test]
fn stale_success_and_error_cannot_clear_newer_requested_view() {
    let mut app = application();
    let before = app.projection.clone();
    let context = app.work_context();
    let a = graph(&app, 2, GraphScope::Neighborhood);
    let b = graph(&app, 4, GraphScope::DependencyNeighborhood);
    app.request_projection_definition(a.clone());
    let old = app.scene_request;
    app.request_projection_definition(b.clone());
    let newest = assert_requested(&app, &b);
    assert_ne!(old, newest);
    app.receive_replies([reply(
        old,
        context.clone(),
        false,
        Ok(Output::Projection(with_view(before.clone(), &a))),
    )]);
    assert_eq!(assert_requested(&app, &b), newest);
    assert_eq!(app.projection, before);
    app.receive_replies([reply(
        old,
        context.clone(),
        false,
        Err("late A error".into()),
    )]);
    assert_eq!(assert_requested(&app, &b), newest);
    assert_eq!(app.projection, before);

    app.receive_replies([reply(
        newest,
        context,
        false,
        Err("matching B error".into()),
    )]);
    assert!(app.requested_definition.is_none());
    assert!(app.deferred_definition.is_none());
    assert_eq!(app.projection, before);
    assert_eq!(app.definition(), before.view);
}

#[test]
fn matching_projection_installs_exact_requested_depth_and_scope_then_retires_intent() {
    let mut app = application();
    let context = app.work_context();
    let desired = graph(&app, 3, GraphScope::DependencyNeighborhood);
    app.request_projection_definition(desired.clone());
    let request = assert_requested(&app, &desired);
    let projected = with_view(app.projection.clone(), &desired);
    app.receive_replies([reply(
        request,
        context,
        false,
        Ok(Output::Projection(projected.clone())),
    )]);
    assert_eq!(app.projection, projected);
    assert_eq!(app.definition(), desired);
    assert!(app.requested_definition.is_none());
    assert!(app.deferred_definition.is_none());
    assert_eq!(app.scene.revision_id, app.binding.unwrap().revision);
}

#[test]
fn candidate_pair_uses_requested_graph_scope_instead_of_old_active_depth() {
    let mut app = application();
    retain_candidate(&mut app);
    app.comparison = ComparisonMode::Candidate;
    assert!(app.rebuild_immediate());
    let context = app.work_context();
    let desired = graph(&app, 4, GraphScope::DependencyNeighborhood);
    let old_view = app.active_projection().view.clone();
    app.request_projection_definition(desired.clone());
    let request = assert_requested(&app, &desired);
    assert_eq!(app.active_projection().view, old_view);
    let before = with_view(app.projection.clone(), &desired);
    let after = candidate(&app, desired.clone());
    let revision = after.projection.revision_id;
    app.receive_replies([reply(
        request,
        context,
        false,
        Ok(Output::CandidateView(after, before)),
    )]);
    assert_eq!(app.candidate.as_ref().unwrap().before.view, desired);
    assert_eq!(app.candidate.as_ref().unwrap().after.view, desired);
    assert_eq!(app.projection.view, desired);
    assert_eq!(app.definition(), desired);
    assert_eq!(app.scene.revision_id, revision);
    assert_eq!(app.comparison, ComparisonMode::Candidate);
    assert!(app.requested_definition.is_none());
    assert!(app.deferred_definition.is_none());
}

#[test]
fn agent_dependency_command_accepts_the_exact_shared_platform_lens() {
    let mut app = application();
    let subject = app.scene.nodes[0].id();
    app.selection
        .select(agq_studio_scene::SceneTarget::Node(subject), false);
    let context = app.work_context();
    app.execute(
        crate::commands::CommandId::Dependencies,
        &eframe::egui::Context::default(),
    );
    // This constructor is also used by StudioPlatform::dependencies, including
    // its name. The full ViewDefinition fence is deliberately retained.
    let returned = ViewDefinition::dependency_neighborhood(
        subject,
        app.families.iter().copied().collect(),
        2,
        app.include_standard,
    );
    let (request, requested) = app.requested_definition.clone().unwrap();
    assert_eq!(requested, returned);
    assert_eq!(requested.name, "Dependency neighborhood");
    let projection = with_view(app.projection.clone(), &returned);
    app.receive_replies([reply(
        request,
        context,
        false,
        Ok(Output::Projection(projection)),
    )]);
    assert_eq!(app.projection.view, returned);
    assert!(app.agent_activity.as_ref().unwrap().complete);
    assert!(app.dependencies.is_some());
    assert!(app.requested_definition.is_none());
}

#[test]
fn same_request_wrong_scope_depth_or_name_cannot_replace_current_projection() {
    for wrong_field in ["scope", "depth", "name"] {
        let mut app = application();
        let before = app.projection.clone();
        let generation = app.generation;
        let context = app.work_context();
        let desired = graph(&app, 4, GraphScope::DependencyNeighborhood);
        app.request_projection_definition(desired.clone());
        let request = assert_requested(&app, &desired);
        let mut wrong = desired;
        match wrong_field {
            "scope" => wrong.graph_scope = GraphScope::Neighborhood,
            "depth" => wrong.depth = 1,
            "name" => wrong.name = "Another view".into(),
            _ => unreachable!(),
        }
        let payload = with_view(before.clone(), &wrong);
        app.receive_replies([reply(
            request,
            context,
            false,
            Ok(Output::Projection(payload)),
        )]);
        assert_eq!(app.projection, before);
        assert_eq!(app.generation, generation);
        assert!(app.requested_definition.is_none());
        assert!(app.deferred_definition.is_none());
    }
}

#[test]
fn coherent_candidate_pair_with_wrong_scope_is_requeried_without_publishing_it() {
    let mut app = application();
    retain_candidate(&mut app);
    app.comparison = ComparisonMode::Candidate;
    assert!(app.rebuild_immediate());
    let before = app.projection.clone();
    let prior_after = app.candidate.as_ref().unwrap().after.clone();
    let generation = app.generation;
    let desired = graph(&app, 4, GraphScope::DependencyNeighborhood);
    app.request_projection_definition(desired.clone());
    let request = assert_requested(&app, &desired);
    let mut wrong = desired.clone();
    wrong.graph_scope = GraphScope::Neighborhood;
    let wrong_before = with_view(before.clone(), &wrong);
    let wrong_after = candidate(&app, wrong);
    let context = app.work_context();
    app.receive_replies([reply(
        request,
        context,
        false,
        Ok(Output::CandidateView(wrong_after, wrong_before)),
    )]);
    assert_eq!(app.projection, before);
    assert_eq!(app.candidate.as_ref().unwrap().after, prior_after);
    assert_eq!(app.generation, generation);
    let replacement = assert_requested(&app, &desired);
    assert_ne!(replacement, request);
    assert_eq!(app.pending, BTreeSet::from([replacement]));
}

#[test]
fn preparation_reissues_deferred_lens_as_candidate_pair_in_the_new_candidate_context() {
    let mut app = application();
    let before = app.projection.clone();
    let prepared = candidate(&app, app.definition());
    let id = prepared.id;
    let (work, context) = mutation(&mut app);
    let desired = graph(&app, 5, GraphScope::DependencyNeighborhood);
    app.request_projection_definition(desired.clone());
    app.receive_replies([reply(
        work,
        context.clone(),
        true,
        Ok(Output::Candidate(prepared)),
    )]);
    assert_eq!(app.candidate.as_ref().unwrap().id, Some(id));
    assert_ne!(app.work_context(), context);
    assert_eq!(app.projection, before);
    assert_eq!(app.comparison, ComparisonMode::Current);
    let query = assert_requested(&app, &desired);
    assert_eq!(app.pending, BTreeSet::from([query]));
    let paired = candidate(&app, desired.clone());
    let paired_before = with_view(before, &desired);
    let new_context = app.work_context();
    app.receive_replies([reply(
        query,
        new_context,
        false,
        Ok(Output::CandidateView(paired, paired_before)),
    )]);
    assert_eq!(app.candidate.as_ref().unwrap().before.view, desired);
    assert_eq!(app.candidate.as_ref().unwrap().after.view, desired);
    assert!(app.requested_definition.is_none());
    assert!(app.deferred_definition.is_none());
}

#[test]
fn cancellation_acknowledgement_flushes_desired_view_against_current_without_candidate() {
    let mut app = application();
    retain_candidate(&mut app);
    let before = app.projection.clone();
    let (work, context) = mutation(&mut app);
    let desired = graph(&app, 3, GraphScope::DependencyNeighborhood);
    app.request_projection_definition(desired.clone());
    app.receive_replies([reply(work, context, true, Ok(Output::Cancelled))]);
    assert!(app.candidate.is_none());
    assert_eq!(app.comparison, ComparisonMode::Current);
    assert_eq!(app.projection, before);
    assert!(!app.bridge.mutation_pending());
    let query = assert_requested(&app, &desired);
    assert_eq!(app.pending, BTreeSet::from([query]));
}

#[test]
fn cancelled_preparation_waits_for_cancel_ack_before_flushing_deferred_view() {
    let mut app = application();
    let before = app.projection.clone();
    let prepared = candidate(&app, app.definition());
    let (work, context) = mutation(&mut app);
    app.preparation = Some(PendingPreparation {
        request: work,
        started: Instant::now(),
        cancelled: true,
        intent: "Cancelled unit-only preparation".into(),
    });
    let desired = graph(&app, 4, GraphScope::DependencyNeighborhood);
    app.request_projection_definition(desired.clone());
    app.receive_replies([reply(work, context, true, Ok(Output::Candidate(prepared)))]);
    assert!(app.bridge.mutation_pending());
    assert!(app.candidate.is_some());
    assert_eq!(app.deferred_definition.as_ref(), Some(&desired));
    assert!(app.requested_definition.is_none());
    assert_eq!(app.projection, before);
    assert_eq!(app.pending.len(), 1);
    let cancel = *app.pending.first().unwrap();
    assert_ne!(cancel, work);
    let cancel_context = app.work_context();
    app.receive_replies([reply(cancel, cancel_context, true, Ok(Output::Cancelled))]);
    assert!(app.candidate.is_none());
    assert!(!app.bridge.mutation_pending());
    let query = assert_requested(&app, &desired);
    assert_eq!(app.pending, BTreeSet::from([query]));
    assert_eq!(app.projection, before);
}
