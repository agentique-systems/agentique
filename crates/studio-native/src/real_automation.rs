//! Opt-in, two-process native acceptance over authenticated real semantic data.
//!
//! This driver can only inject ordinary input and inspect application responses.
//! It never fabricates projections, mutates a graph, or calls a commit bypass.
use crate::{
    Args,
    app::{ComparisonMode, StudioApp},
    automation::{self, ScenarioStatus},
    navigation::World,
    real_targets::{self, Target},
    selection::Selection,
};
use agq_kernel::ElementId;
use agq_modeling_repository::{
    BranchId, ContentDigest, ProjectId, ProjectRevisionId, RevisionManifest, ValidationState,
};
use agq_modeling_view::{
    ElementInspector, ExplanationNodeKind, RelationshipFamily, ViewEdge, ViewKind, ViewOrigin,
    ViewProjection,
};
use agq_studio_platform::{CandidatePhase, RevisionBinding};
use agq_studio_scene::{DiffMark, NodeCategory, Point, SceneTarget};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, time::Instant};

const FORMAT: &str = "agentique-native-real-acceptance/1";
const PART_NAME: &str = "alphaStudioObserver";
// Coarse non-freeze qualification, separate from the 60 Hz interaction target.
const LONG_PREPARATION_MS: u128 = 1_000;
const MAX_PREPARATION_INPUT_GAP_MS: u128 = 250;

/// Run before StudioApp starts a worker: real acceptance may write only to an
/// explicitly selected fresh database. Restart reads that same isolated database.
pub fn validate_launch(args: &Args) -> Result<(), String> {
    if args.resume_report.is_some() && args.scenario.as_deref() != Some("real") {
        return Err("--resume-report is only valid with --scenario real".into());
    }
    if crate::presentation_automation::is_scenario(args.scenario.as_deref()) {
        return Ok(()); // Its independent launch gate validates isolated paths.
    }
    let real = matches!(args.scenario.as_deref(), Some("real" | "real-restart"));
    if !real {
        return if args.restart_report.is_some() {
            Err("--restart-report is only valid with --scenario real-restart".into())
        } else {
            Ok(())
        };
    }
    if args.fixture.is_some() || !args.no_restore || args.frames.is_some() {
        return Err(
            "Real acceptance requires --no-restore and refuses --fixture or --frames".into(),
        );
    }
    if args.screenshot.is_some() || args.gallery.is_none() {
        return Err(
            "Real acceptance requires --gallery and refuses single-frame --screenshot".into(),
        );
    }
    if args.scenario_timeout_seconds == 0 {
        return Err("Real acceptance needs a nonzero wall-time deadline".into());
    }
    let database = args
        .database
        .as_ref()
        .ok_or("An explicit isolated --database is required")?;
    if !database.is_absolute() {
        return Err("The acceptance database path must be absolute".into());
    }
    if entry_exists(&args.scenario_report) {
        return Err(
            "Choose a new --scenario-report; prior acceptance evidence is preserved".into(),
        );
    }
    if args.gallery.as_ref().is_some_and(|path| entry_exists(path)) {
        return Err("Choose a new --gallery directory; prior captures are preserved".into());
    }
    if args.scenario.as_deref() == Some("real") {
        if args.resume_report.is_some() {
            read_resume(args)?;
        } else if args.restart_report.is_some()
            || entry_exists(database)
            || entry_exists(&database.with_extension("views.sqlite"))
            || entry_exists(&database.with_extension("native-session.json"))
            || ["-wal", "-shm"].iter().any(|suffix| {
                let mut path = database.as_os_str().to_os_string();
                path.push(suffix);
                entry_exists(Path::new(&path))
            })
        {
            return Err("First real run requires a new database with no sidecar presentation state and no restart report".into());
        }
    } else {
        let report = read_restart(args)?;
        if !database.is_file() || Path::new(&report.database) != database {
            return Err(
                "Restart must open the exact existing database from the successful first report"
                    .into(),
            );
        }
    }
    Ok(())
}

fn entry_exists(path: &Path) -> bool {
    // Includes broken symbolic links: a missing target is not a fresh entry.
    std::fs::symlink_metadata(path).is_ok()
}

fn read_restart(args: &Args) -> Result<Report, String> {
    read_restart_evidence(args).map(|(report, _)| report)
}

fn read_report(path: &Path) -> Result<(Report, ContentDigest), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Cannot read prior report: {e}"))?;
    let report =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid first-process report: {e}"))?;
    Ok((report, ContentDigest::of(&bytes)))
}

fn read_restart_evidence(args: &Args) -> Result<(Report, ContentDigest), String> {
    let path = args
        .restart_report
        .as_ref()
        .ok_or("--scenario real-restart requires --restart-report")?;
    let (report, digest) = read_report(path)?;
    if report.format != FORMAT
        || report.scenario != "real"
        || !report.passed
        || report.outcome != "journey_passed_restart_pending"
        || report.restart_verified
        || report.committed.is_none()
        || report.added_element.is_none()
        || report.added_owner.is_none()
        || report.gallery.len() < 8
    {
        return Err("Restart requires a successful real first-process journey with committed identity and complete gallery".into());
    }
    Ok((report, digest))
}

fn read_resume(args: &Args) -> Result<(Report, ContentDigest), String> {
    if args.scenario.as_deref() != Some("real") || args.restart_report.is_some() {
        return Err("Resume requires --scenario real without --restart-report".into());
    }
    let path = args.resume_report.as_ref().ok_or("Resume report missing")?;
    let (report, digest) = read_report(path)?;
    let database = args.database.as_ref().ok_or("Resume database missing")?;
    if !database.is_absolute() || !database.is_file() || Path::new(&report.database) != database {
        return Err(
            "Resume must open the exact existing absolute database from the failed report".into(),
        );
    }
    validate_resume_report(&report)?;
    assert_source_manifest(
        &args.root,
        report.baseline.as_ref().expect("validated baseline"),
    )?;
    Ok((report, digest))
}

fn validate_resume_report(report: &Report) -> Result<(), String> {
    let baseline = report
        .baseline
        .as_ref()
        .ok_or("Resume report has no validated baseline")?;
    assert_validated(baseline)?;
    let expected = RevisionBinding {
        project: baseline.project_id,
        revision: baseline.revision_id,
    };
    let untouched = |state: &State| {
        !state.mutation_pending
            && state.candidate_revision.is_none()
            && state.candidate_phase.is_none()
            && state.binding.is_none_or(|binding| binding == expected)
            && state
                .branch
                .is_none_or(|branch| Some(branch) == report.branch)
    };
    let safe_steps: Vec<_> = steps(false)
        .into_iter()
        .take_while(|step| !matches!(step.check, Check::CreateDialog))
        .collect();
    if report.format != FORMAT
        || report.scenario != "real"
        || report.outcome != "failed"
        || report.passed
        || report.restart_verified
        || report.failure.as_ref().is_none_or(|e| e.is_empty())
        || report.project != Some(baseline.project_id)
        || report.branch.is_none()
        || report.committed.is_some()
        || report.added_element.is_some()
        || report.added_owner.is_some()
        || report.background_frames != 0
        || report.background_pan_observed
        || report.background_current_inspection.is_some()
        || report.preparation_responsiveness.elapsed_ms.is_some()
        || report.preparation_responsiveness.observed_input_hooks != 0
        || report.assertions.is_empty()
        || report.assertions.len() > safe_steps.len()
        || !report.assertions[0].passed
        || report
            .assertions
            .iter()
            .zip(&safe_steps)
            .any(|(assertion, step)| {
                assertion.name != step.name
                    || !untouched(&assertion.before)
                    || !untouched(&assertion.after)
            })
        || !untouched(&report.last_state)
        || report.last_state.binding != Some(expected)
        || report.last_state.branch != report.branch
        || report.last_state.pending_requests != 0
        || report.last_state.scene_revision != baseline.revision_id
        || report.last_state.selection_revision != baseline.revision_id
        || report.last_state.producer_completeness != "Complete"
    {
        return Err("Resume requires a failed real journey stopped before any candidate or mutation, with an unchanged validated baseline".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct State {
    frame: u64,
    binding: Option<RevisionBinding>,
    branch: Option<BranchId>,
    scene_revision: ProjectRevisionId,
    selection_revision: ProjectRevisionId,
    selected: Vec<String>,
    focus: Option<ElementId>,
    world: String,
    comparison: String,
    candidate_revision: Option<ProjectRevisionId>,
    candidate_phase: Option<String>,
    producer_completeness: String,
    projection_nodes: usize,
    projection_edges: usize,
    scene_nodes: usize,
    scene_edges: usize,
    camera: [f32; 3],
    pending_requests: usize,
    mutation_pending: bool,
    inspector_revision: Option<ProjectRevisionId>,
    inspector_element: Option<ElementId>,
    explanation_revision: Option<ProjectRevisionId>,
    explanation_subject: Option<ElementId>,
    explanation_origin: Option<ViewOrigin>,
    explanation_rule: Option<String>,
    explanation_evidence: Option<usize>,
    agent_view: bool,
    status: String,
}
impl State {
    fn of(app: &StudioApp) -> Self {
        Self {
            frame: app.frame_number,
            binding: app.binding,
            branch: app.branch,
            scene_revision: app.scene.revision_id,
            selection_revision: app.selection.revision,
            selected: app
                .selection
                .targets
                .iter()
                .map(|t| format!("{t:?}"))
                .collect(),
            focus: app.focus,
            world: format!("{:?}", app.world),
            comparison: format!("{:?}", app.comparison),
            candidate_revision: app.candidate.as_ref().map(|c| c.after.revision_id),
            candidate_phase: app
                .candidate
                .as_ref()
                .and_then(|c| c.phase.map(|p| format!("{p:?}"))),
            producer_completeness: app
                .active_projection()
                .metadata
                .producer_completeness
                .clone(),
            projection_nodes: app.active_projection().nodes.len(),
            projection_edges: app.active_projection().edges.len(),
            scene_nodes: app.scene.nodes.len(),
            scene_edges: app.scene.edges.len(),
            camera: [app.camera.center.x, app.camera.center.y, app.camera.zoom],
            pending_requests: app.pending.len(),
            mutation_pending: app.bridge.mutation_pending(),
            inspector_revision: app.inspector.as_ref().map(|i| i.revision_id),
            inspector_element: app.inspector.as_ref().map(|i| i.element.id),
            explanation_revision: app.explanation.as_ref().map(|e| e.revision_id),
            explanation_subject: app.explanation.as_ref().map(|e| e.subject_id),
            explanation_origin: app.explanation.as_ref().map(|e| e.origin),
            explanation_rule: app
                .explanation
                .as_ref()
                .and_then(|e| e.rule_id.map(|id| id.to_string())),
            explanation_evidence: app.explanation.as_ref().map(|e| e.evidence_count),
            agent_view: app.show_agent,
            status: app.status.clone(),
        }
    }
    fn same_capture_context(&self, other: &Self) -> bool {
        self.binding == other.binding
            && self.scene_revision == other.scene_revision
            && self.selection_revision == other.selection_revision
            && self.selected == other.selected
            && self.focus == other.focus
            && self.world == other.world
            && self.comparison == other.comparison
            && self.candidate_revision == other.candidate_revision
            && self.candidate_phase == other.candidate_phase
            && self.explanation_subject == other.explanation_subject
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Assertion {
    name: String,
    passed: bool,
    elapsed_ms: u128,
    since_start_ms: u128,
    before: State,
    after: State,
    inputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Report {
    format: String,
    scenario: String,
    scope: String,
    database: String,
    root: String,
    outcome: String,
    passed: bool,
    restart_verified: bool,
    previous_report_digest: Option<ContentDigest>,
    #[serde(default)]
    resumed_baseline: bool,
    project: Option<ProjectId>,
    branch: Option<BranchId>,
    baseline: Option<RevisionManifest>,
    committed: Option<RevisionManifest>,
    added_element: Option<ElementId>,
    #[serde(default)]
    added_owner: Option<ElementId>,
    added_name: String,
    assertions: Vec<Assertion>,
    gallery: Vec<String>,
    failure: Option<String>,
    elapsed_ms: u128,
    background_frames: u64,
    background_pan_observed: bool,
    #[serde(default)]
    background_current_inspection: Option<BackgroundInspectionEvidence>,
    #[serde(default)]
    preparation_responsiveness: PreparationResponsiveness,
    #[serde(default)]
    engineering_evidence: EngineeringEvidence,
    metrics: serde_json::Value,
    last_state: State,
}

/// Exact responses observed through ordinary Inspector and World interactions.
/// These are review evidence, not an alternate semantic authority.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct EngineeringEvidence {
    repository_interfaces: Option<ElementInspector>,
    inherited_port: Option<ElementInspector>,
    connected_port: Option<ElementInspector>,
    requirement_subject_path: Vec<ViewEdge>,
}

/// Observed native input/query completion while actual candidate work was pending.
/// Timings start at application observations, not OS delivery or photon output.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BackgroundInspectionEvidence {
    binding: RevisionBinding,
    runtime_epoch: u64,
    element: ElementId,
    request: u64,
    selection_started_frame: u64,
    request_observed_frame: u64,
    response_observed_frame: u64,
    selection_to_response_ms: u128,
    request_observed_to_response_ms: u128,
    mutation_pending_at_response: bool,
    inspector: ElementInspector,
}

#[derive(Clone)]
struct BackgroundInspection {
    binding: RevisionBinding,
    runtime_epoch: u64,
    element: ElementId,
    name: String,
    age: u64,
    started: Instant,
    selection_started_frame: u64,
    previous_request: u64,
    request: Option<(u64, u64, Instant)>,
}

fn assess_background_inspection(
    evidence: Option<&BackgroundInspectionEvidence>,
    baseline: RevisionBinding,
    owner: ElementId,
    runtime_epoch: u64,
) -> Result<(), String> {
    let evidence = evidence.ok_or(
        "No different current-revision object was inspected before candidate preparation completed",
    )?;
    if evidence.binding != baseline
        || evidence.runtime_epoch != runtime_epoch
        || evidence.element == owner
        || evidence.request == 0
        || !evidence.mutation_pending_at_response
        || evidence.inspector.revision_id != baseline.revision
        || evidence.inspector.element.revision_id != baseline.revision
        || evidence.inspector.element.id != evidence.element
        || evidence.inspector.element.origin != ViewOrigin::Authored
        || !evidence.inspector.element.source_available
        || evidence.request_observed_frame < evidence.selection_started_frame
        || evidence.response_observed_frame < evidence.request_observed_frame
    {
        return Err("Background Inspector did not prove a different exact baseline object while candidate preparation was still pending".into());
    }
    Ok(())
}

/// Native input-hook cadence during submission through candidate response delivery.
/// These observations do not measure physical input latency or GPU presentation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct PreparationResponsiveness {
    elapsed_ms: Option<u128>,
    maximum_input_hook_gap_ms: u128,
    observed_input_hooks: u64,
    assessment: Option<String>,
}

#[derive(Clone)]
struct PreparationClock {
    started: Instant,
    previous: Instant,
}

impl PreparationClock {
    fn observe(&mut self, now: Instant, evidence: &mut PreparationResponsiveness) {
        evidence.maximum_input_hook_gap_ms = evidence
            .maximum_input_hook_gap_ms
            .max(now.duration_since(self.previous).as_millis());
        evidence.observed_input_hooks += 1;
        self.previous = now;
    }
}

fn assess_preparation(
    evidence: &PreparationResponsiveness,
    background_pan_observed: bool,
) -> Result<&'static str, String> {
    let elapsed = evidence
        .elapsed_ms
        .ok_or("Preparation timing did not finish")?;
    if evidence.observed_input_hooks == 0 {
        return Err("Preparation has no observed native input-hook interval".into());
    }
    if evidence.maximum_input_hook_gap_ms > MAX_PREPARATION_INPUT_GAP_MS {
        return Err(format!(
            "Preparation blocked native input hooks for {} ms; coarse non-freeze limit is {} ms",
            evidence.maximum_input_hook_gap_ms, MAX_PREPARATION_INPUT_GAP_MS
        ));
    }
    if elapsed >= LONG_PREPARATION_MS && !background_pan_observed {
        return Err(format!(
            "Preparation took {elapsed} ms without native panning observed while mutation was pending"
        ));
    }
    Ok(if background_pan_observed {
        "Native camera pan observed during pending mutation; input-hook gaps stayed within 250 ms. This is not a 60 Hz or physical-presentation qualification."
    } else {
        "Preparation completed in under 1000 ms with input-hook gaps within 250 ms; concurrent panning was not observed."
    })
}

fn assert_part_owner(
    projection: &ViewProjection,
    part: ElementId,
    owner: ElementId,
) -> Result<(), String> {
    if projection.nodes.iter().any(|node| {
        node.id == part
            && node.name == PART_NAME
            && node.semantic_kind == "PartUsage"
            && node.owner == Some(owner)
            && node.origin == ViewOrigin::Authored
            && node.source_available
    }) {
        Ok(())
    } else {
        Err("Created part is not source-backed authored content owned by the intended ModelingPlatform identity".into())
    }
}

fn assert_restart_element(
    projection: &ViewProjection,
    focus: Option<ElementId>,
    inspector: Option<&ElementInspector>,
    part: ElementId,
    owner: ElementId,
) -> Result<(), String> {
    if projection.view.kind != ViewKind::Architecture
        || focus != Some(owner)
        || projection.view.focus != Some(owner)
    {
        return Err(
            "Restarted part identity must be checked in its actual focused owner view".into(),
        );
    }
    if projection_named(projection, PART)? != part {
        return Err("Restarted canonical part identity differs".into());
    }
    assert_part_owner(projection, part, owner)?;
    let inspector = inspector.ok_or("Restarted part has no real Inspector response")?;
    if inspector.revision_id != projection.revision_id
        || inspector.element.revision_id != projection.revision_id
        || inspector.element.id != part
        || inspector.element.owner != Some(owner)
        || inspector.owner.as_ref().map(|item| item.id) != Some(owner)
    {
        return Err(
            "Restarted Inspector differs from the committed part, owner or revision".into(),
        );
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct Named {
    name: &'static str,
    kind: &'static str,
}
const PLATFORM: Named = Named {
    name: "ModelingPlatform",
    kind: "PartDefinition",
};
const REPOSITORY: Named = Named {
    name: "ModelRepository",
    kind: "PartDefinition",
};
const AGENT_RUNTIME: Named = Named {
    name: "AgentRuntime",
    kind: "PartDefinition",
};
const PART: Named = Named {
    name: PART_NAME,
    kind: "PartUsage",
};

fn named(app: &StudioApp, named: Named) -> Result<ElementId, String> {
    projection_named(app.active_projection(), named)
}

fn projection_named(projection: &ViewProjection, named: Named) -> Result<ElementId, String> {
    let ids: Vec<_> = projection
        .nodes
        .iter()
        .filter(|n| n.name == named.name && n.semantic_kind == named.kind)
        .map(|n| n.id)
        .collect();
    match ids.as_slice() {
        [id] => Ok(*id),
        _ => Err(format!(
            "Expected one real {} named {}; found {} in the current projection",
            named.kind,
            named.name,
            ids.len()
        )),
    }
}

fn inspector_feature(inspector: &ElementInspector, name: &str) -> Result<ElementId, String> {
    let ids: std::collections::BTreeSet<_> = inspector
        .owned_features
        .iter()
        .chain(&inspector.effective_features)
        .filter(|feature| feature.name == name && feature.semantic_kind == "PortUsage")
        .map(|feature| feature.id)
        .collect();
    if ids.len() == 1 {
        Ok(*ids.first().expect("one feature"))
    } else {
        Err(format!(
            "Expected one canonical Inspector port {name}, found {}",
            ids.len()
        ))
    }
}

fn complete_query(inspector: &ElementInspector, name: &str) -> bool {
    inspector
        .queries
        .iter()
        .any(|query| query.name == name && query.completeness == "Complete")
}

fn requirement_subject_path(projection: &ViewProjection) -> Result<Vec<ViewEdge>, String> {
    let requirement = projection_named(
        projection,
        Named {
            name: "ImmutableRevisions",
            kind: "RequirementDefinition",
        },
    )?;
    let subject = projection
        .nodes
        .iter()
        .filter(|node| node.name == "subjectWorkspace" && node.owner == Some(requirement))
        .collect::<Vec<_>>();
    let [subject] = subject.as_slice() else {
        return Err(
            "ImmutableRevisions has no unique canonical subjectWorkspace in this Requirements view"
                .into(),
        );
    };
    let architecture = projection_named(
        projection,
        Named {
            name: "ProjectWorkspace",
            kind: "PartDefinition",
        },
    )?;
    let mut path = vec![];
    for (kind, source, target) in [
        ("SubjectMembership", requirement, subject.id),
        ("FeatureTyping", subject.id, architecture),
    ] {
        let edges: Vec<_> = projection
            .edges
            .iter()
            .filter(|edge| {
                edge.semantic_kind == kind
                    && edge.source == source
                    && edge.target == target
                    && edge.revision_id == projection.revision_id
                    && edge.relationship_id.is_some()
            })
            .collect();
        let [edge] = edges.as_slice() else {
            return Err(format!(
                "Requirements view lacks one exact canonical {kind} from {source} to {target}"
            ));
        };
        path.push((**edge).clone());
    }
    if path[0].relationship_id == path[1].relationship_id || path[0].id == path[1].id {
        return Err(
            "Requirement subject and typing must retain distinct canonical relationship identities"
                .into(),
        );
    }
    Ok(path)
}

fn platform_connection<'a>(
    projection: &'a ViewProjection,
    inspector: &'a ElementInspector,
) -> Result<&'a ViewEdge, String> {
    let source = projection_named(
        projection,
        Named {
            name: "clientQueries",
            kind: "PortUsage",
        },
    )?;
    let target = projection_named(
        projection,
        Named {
            name: "platformQueries",
            kind: "PortUsage",
        },
    )?;
    let connectors: Vec<_> = projection
        .nodes
        .iter()
        .filter(|node| {
            node.name == "queryConnection"
                && node.origin == ViewOrigin::Authored
                && node.source_available
        })
        .collect();
    let [connector] = connectors.as_slice() else {
        return Err(
            "Focused architecture lacks the unique source-backed queryConnection identity".into(),
        );
    };
    let edges: Vec<_> = inspector
        .relationships
        .iter()
        .filter(|edge| {
            edge.family == RelationshipFamily::Connection
                && edge.relationship_id == Some(connector.id)
                && edge.semantic_kind == connector.semantic_kind
                && edge.revision_id == projection.revision_id
                && ((edge.source == source && edge.target == target)
                    || (edge.source == target && edge.target == source))
                && projection.edges.contains(edge)
        })
        .collect();
    let [edge] = edges.as_slice() else {
        return Err("Port Inspector and focused architecture disagree on the exact queryConnection and its two canonical port endpoints".into());
    };
    if !complete_query(
        inspector,
        &format!("Connection endpoints: queryConnection [{}]", connector.id),
    ) {
        return Err("Port Inspector lacks a complete queryConnection endpoint query".into());
    }
    Ok(edge)
}

fn assert_port_inspector<'a>(
    app: &'a StudioApp,
    expected_id: ElementId,
    name: &'static str,
    owner: Named,
    interface: Named,
) -> Result<&'a ElementInspector, String> {
    let inspector = app.inspector.as_ref().ok_or("Port Inspector unavailable")?;
    let owner = named(app, owner)?;
    // Port definitions are canonical Inspector references, not boundary ports in
    // System World. Do not require a fabricated scene node to prove the type.
    let interfaces: Vec<_> = inspector
        .effective_types
        .iter()
        .filter(|feature| feature.name == interface.name && feature.semantic_kind == interface.kind)
        .collect();
    let revision = app.active_projection().revision_id;
    if app.selected_element() != Some(expected_id)
        || inspector.element.id != expected_id
        || inspector.element.name != name
        || inspector.element.semantic_kind != "PortUsage"
        || inspector.element.origin != ViewOrigin::Authored
        || !inspector.element.source_available
        || inspector.revision_id != revision
        || inspector.element.revision_id != revision
        || inspector.element.owner != Some(owner)
        || inspector.owner.as_ref().map(|owner| owner.id) != Some(owner)
        || interfaces.len() != 1
        || !complete_query(inspector, "Owner")
        || !complete_query(inspector, "Effective types")
        || app
            .lookup
            .port(&app.scene, expected_id)
            .is_none_or(|port| port.owner != owner)
    {
        return Err(format!(
            "Clicked {name} did not retain its canonical port identity, original owner, effective interface and complete query answers"
        ));
    }
    let source = inspector
        .source
        .as_ref()
        .ok_or("Authored port has no source provenance")?;
    if source.start >= source.end
        || !current_manifest(app)?.documents.iter().any(|document| {
            document.document_id == source.document_id
                && document.source_revision_id == source.source_revision_id
                && source.path.as_ref() == Some(&document.path)
        })
    {
        return Err("Port source does not belong to the actual selected revision manifest".into());
    }
    Ok(inspector)
}

#[derive(Clone, Debug)]
enum Action {
    OpenProject,
    Select(Named),
    InspectFeature(&'static str),
    Palette(&'static str),
    Key(Key),
    Standards,
    DerivedEdge,
    Prepare,
    Mode(ComparisonMode),
    HistoryBaseline,
}
impl Action {
    fn frames(&self) -> u64 {
        match self {
            Self::Select(_) | Self::Palette(_) => 12,
            Self::Prepare => 7,
            Self::Key(_) => 1,
            _ => 2,
        }
    }
}

#[derive(Clone, Debug)]
enum Check {
    Baseline,
    Selected(Named),
    FocusedPlatform,
    RepositoryInterfaces,
    FocusedRepository,
    InheritedPort,
    ConnectedPort,
    Home,
    World(World),
    GraphOverview,
    Dependencies,
    Standards,
    DerivedEdge,
    Explanation,
    ExplanationClosed,
    History,
    ParentDiff,
    CreateDialog,
    CandidateWorking,
    Mode(ComparisonMode),
    Validated,
    Committed,
    RestartRevision,
    Restart,
}

#[derive(Clone, Debug)]
struct Step {
    name: &'static str,
    action: Action,
    check: Check,
    capture: Option<&'static str>,
}
fn steps(restart: bool) -> Vec<Step> {
    let step = |name, action, check, capture| Step {
        name,
        action,
        check,
        capture,
    };
    if restart {
        return vec![
            step(
                "restart opens the same durable real project",
                Action::OpenProject,
                Check::RestartRevision,
                Some("10-restarted-system-world"),
            ),
            step(
                "restart selects the committed part's actual ModelingPlatform owner",
                Action::Select(PLATFORM),
                Check::Selected(PLATFORM),
                None,
            ),
            step(
                "restart enters ModelingPlatform through native keyboard input",
                Action::Key(Key::F),
                Check::FocusedPlatform,
                Some("10a-restarted-part-owner"),
            ),
            step(
                "restart inspects the same committed canonical element",
                Action::Select(PART),
                Check::Restart,
                Some("11-restarted-committed-element"),
            ),
            step(
                "restart confirms durable revision history",
                Action::Key(Key::Num4),
                Check::History,
                Some("12-restarted-history"),
            ),
        ];
    }
    vec![
        step(
            "open authenticated Agentique with Validated semantic closure",
            Action::OpenProject,
            Check::Baseline,
            Some("01-system-world"),
        ),
        step(
            "select ModelingPlatform through the semantic outliner",
            Action::Select(PLATFORM),
            Check::Selected(PLATFORM),
            None,
        ),
        step(
            "focus ModelingPlatform through native keyboard input",
            Action::Key(Key::F),
            Check::FocusedPlatform,
            Some("02-focused-subsystem"),
        ),
        step(
            "inspect a platform port and its actual query connection",
            Action::InspectFeature("clientQueries"),
            Check::ConnectedPort,
            Some("02a-platform-port"),
        ),
        step(
            "select ModelRepository inside the focused ModelingPlatform",
            Action::Select(REPOSITORY),
            Check::RepositoryInterfaces,
            None,
        ),
        step(
            "enter ModelRepository without leaving its containing subsystem",
            Action::Key(Key::F),
            Check::FocusedRepository,
            Some("02b-focused-repository"),
        ),
        step(
            "inspect the original inherited repositoryRevisions port",
            Action::InspectFeature("repositoryRevisions"),
            Check::InheritedPort,
            Some("02c-repository-port"),
        ),
        step(
            "select repository as the dependency inquiry subject",
            Action::Select(REPOSITORY),
            Check::Selected(REPOSITORY),
            None,
        ),
        step(
            "agent shows revision-bound dependencies",
            Action::Palette("Show dependencies"),
            Check::Dependencies,
            Some("07-agent-view"),
        ),
        step(
            "open Graph World over real relationships",
            Action::Key(Key::Num2),
            Check::World(World::Graph),
            Some("03-graph-world"),
        ),
        step(
            "include authenticated adjacent standard dependencies",
            Action::Standards,
            Check::Standards,
            None,
        ),
        step(
            "select an actual derived relationship by hit-tested geometry",
            Action::DerivedEdge,
            Check::DerivedEdge,
            None,
        ),
        step(
            "Explain renders actual derived causal evidence",
            Action::Key(Key::E),
            Check::Explanation,
            Some("05-explain"),
        ),
        step(
            "dismiss Explain",
            Action::Key(Key::Escape),
            Check::ExplanationClosed,
            None,
        ),
        step(
            "open real Requirements World",
            Action::Key(Key::Num3),
            Check::World(World::Requirements),
            Some("04-requirements-world"),
        ),
        step(
            "open immutable real design history",
            Action::Key(Key::Num4),
            Check::History,
            None,
        ),
        step(
            "open Graph World to compare the added Agent Fabric",
            Action::Key(Key::Num2),
            Check::World(World::Graph),
            None,
        ),
        step(
            "show the complete authored graph including the added agent architecture",
            Action::Palette("Show loaded graph overview"),
            Check::GraphOverview,
            None,
        ),
        step(
            "compare actual parent revision with the same complete graph lens",
            Action::Palette("Compare with parent"),
            Check::ParentDiff,
            Some("06-history-diff"),
        ),
        step(
            "select current revision in History to leave comparison",
            Action::Key(Key::Num4),
            Check::History,
            None,
        ),
        step(
            "restore current revision atomically",
            Action::HistoryBaseline,
            Check::Baseline,
            None,
        ),
        step(
            "return to System World before direct manipulation",
            Action::Key(Key::Num1),
            Check::Home,
            None,
        ),
        step(
            "select nested part owner",
            Action::Select(PLATFORM),
            Check::Selected(PLATFORM),
            None,
        ),
        step(
            "open Create Part through contextual palette",
            Action::Palette("Create: nested Part"),
            Check::CreateDialog,
            None,
        ),
        step(
            "prepare real source-backed candidate while current remains responsive",
            Action::Prepare,
            Check::CandidateWorking,
            None,
        ),
        step(
            "review Candidate revision",
            Action::Mode(ComparisonMode::Candidate),
            Check::Mode(ComparisonMode::Candidate),
            None,
        ),
        step(
            "select real candidate part",
            Action::Select(PART),
            Check::Selected(PART),
            Some("08-candidate"),
        ),
        step(
            "review immutable Current revision",
            Action::Mode(ComparisonMode::Current),
            Check::Mode(ComparisonMode::Current),
            None,
        ),
        step(
            "return to Candidate revision",
            Action::Mode(ComparisonMode::Candidate),
            Check::Mode(ComparisonMode::Candidate),
            None,
        ),
        step(
            "review exact candidate difference",
            Action::Mode(ComparisonMode::Diff),
            Check::Mode(ComparisonMode::Diff),
            Some("09-candidate-diff"),
        ),
        step(
            "validate the retained semantic candidate",
            Action::Palette("Validate candidate"),
            Check::Validated,
            None,
        ),
        step(
            "commit only the validated candidate",
            Action::Palette("Commit validated candidate"),
            Check::Committed,
            None,
        ),
        step(
            "observe committed revision in immutable History",
            Action::Key(Key::Num4),
            Check::History,
            Some("10-committed-history"),
        ),
    ]
}

#[derive(Clone)]
struct Capture {
    name: &'static str,
    state: State,
    started: Instant,
}

#[derive(Clone)]
struct ReviewSelection {
    selection: Selection,
    focus: Option<ElementId>,
}
impl ReviewSelection {
    fn of(app: &StudioApp) -> Self {
        Self {
            selection: app.selection.clone(),
            focus: app.focus,
        }
    }
}

fn assert_candidate_review_selection(
    before: &ReviewSelection,
    after: &ReviewSelection,
    revision: ProjectRevisionId,
    added: ElementId,
    explicitly_selected: Option<ElementId>,
) -> Result<(), String> {
    if before.focus != after.focus || after.selection.revision != revision {
        return Err("Candidate mode lost its focus or selection revision".into());
    }
    if let Some(selected) = explicitly_selected {
        if selected != added
            || after
                .selection
                .primary
                .as_ref()
                .and_then(SceneTarget::element_id)
                != Some(added)
            || after
                .selection
                .primary
                .as_ref()
                .is_none_or(|target| !after.selection.targets.contains(target))
        {
            return Err(
                "Explicitly selected candidate part was lost while switching review modes".into(),
            );
        }
    } else if before.selection.revision != revision
        || before.selection.targets != after.selection.targets
        || before.selection.primary != after.selection.primary
    {
        return Err("Initial Candidate review did not preserve the actual prior selection".into());
    }
    Ok(())
}

#[derive(Clone)]
struct Runner {
    report: Report,
    steps: Vec<Step>,
    index: usize,
    age: u64,
    started: Instant,
    step_started: Instant,
    before: Option<State>,
    point: Option<Pos2>,
    events: Vec<String>,
    capture: Option<Capture>,
    dirty: bool,
    last_write: Instant,
    background_pan: Option<(u8, Pos2, Point)>,
    background_pan_ready_frame: Option<u64>,
    candidate_id: Option<agq_studio_platform::CandidateId>,
    candidate_revision: Option<ProjectRevisionId>,
    preparation_clock: Option<PreparationClock>,
    inspected_feature: Option<ElementId>,
    background_inspection: Option<BackgroundInspection>,
    review_selection_before: Option<ReviewSelection>,
    /// Set only after the explicit part selection and real Inspector check pass.
    explicitly_selected_candidate_part: Option<ElementId>,
}
impl Runner {
    fn new(app: &StudioApp) -> Result<Self, String> {
        let restart = app.args.scenario.as_deref() == Some("real-restart");
        let prior = if restart {
            Some(read_restart_evidence(&app.args)?)
        } else if app.args.resume_report.is_some() {
            Some(read_resume(&app.args)?)
        } else {
            None
        };
        let previous_report_digest = prior.as_ref().map(|(_, digest)| *digest);
        let previous = prior.map(|(report, _)| report);
        Ok(Self {
            report: Report {
                format: FORMAT.into(), scenario: app.args.scenario.clone().unwrap(),
                scope: "Actual native input, authenticated runtime, real models/agentique and ordinary in-process service workers. First process alone does not establish durable restart or overall alpha acceptance.".into(),
                database: app.config.database.display().to_string(), root: app.config.root.display().to_string(),
                outcome: "running".into(), passed: false, restart_verified: false,
                previous_report_digest,
                resumed_baseline: app.args.resume_report.is_some(),
                project: previous.as_ref().and_then(|p| p.project),
                branch: previous.as_ref().and_then(|p| p.branch),
                baseline: previous.as_ref().and_then(|p| p.baseline.clone()),
                committed: previous.as_ref().and_then(|p| p.committed.clone()),
                added_element: previous.as_ref().and_then(|p| p.added_element),
                added_owner: previous.as_ref().and_then(|p| p.added_owner),
                added_name: PART_NAME.into(), assertions: vec![], gallery: vec![], failure: None,
                elapsed_ms: 0, background_frames: 0, background_pan_observed: false,
                background_current_inspection: None,
                preparation_responsiveness: PreparationResponsiveness::default(),
                engineering_evidence: EngineeringEvidence::default(),
                metrics: serde_json::Value::Null, last_state: State::of(app),
            },
            steps: steps(restart), index: 0, age: 0, started: Instant::now(), step_started: Instant::now(),
            before: None, point: None, events: vec![], capture: None, dirty: true, last_write: Instant::now(),
            background_pan: None, candidate_id: None, candidate_revision: None,
            background_pan_ready_frame: None,
            preparation_clock: None,
            inspected_feature: None,
            background_inspection: None,
            review_selection_before: None,
            explicitly_selected_candidate_part: None,
        })
    }

    fn advance(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<ScenarioStatus, String> {
        if app.fixture.is_some() || app.args.fixture.is_some() {
            return Err("Real runner refuses every fixture and fixture service fallback".into());
        }
        // Observe before inspecting pending state: a synchronous regression may
        // finish the whole mutation between two hooks and must retain that gap.
        if let Some(clock) = &mut self.preparation_clock {
            let now = Instant::now();
            clock.observe(now, &mut self.report.preparation_responsiveness);
            if app.candidate.is_some() && !app.bridge.mutation_pending() {
                self.report.preparation_responsiveness.elapsed_ms =
                    Some(now.duration_since(clock.started).as_millis());
                self.preparation_clock = None;
            }
        }
        if self.started.elapsed().as_secs() > app.args.scenario_timeout_seconds {
            return Err(format!(
                "Real acceptance exceeded {} wall seconds at step {}",
                app.args.scenario_timeout_seconds,
                self.index + 1
            ));
        }
        input
            .events
            .retain(|e| matches!(e, Event::Screenshot { .. } | Event::WindowFocused(_)));
        input.focused = true;
        input.modifiers = Modifiers::NONE;
        if self.capture.is_some() {
            self.receive_capture(app, input)?;
            return Ok(ScenarioStatus::Running);
        }
        if self.index == self.steps.len() {
            return Ok(ScenarioStatus::Complete);
        }
        let step = self.steps[self.index].clone();
        if self.before.is_none() {
            if !idle(app) {
                return Ok(ScenarioStatus::Running);
            }
            if matches!(step.action, Action::OpenProject) && app.projects.is_empty() {
                if app.frame_number > 2 {
                    return Err(format!(
                        "Authenticated bootstrap produced no project: {}",
                        app.setup_reason
                    ));
                }
                return Ok(ScenarioStatus::Running);
            }
            if !matches!(step.action, Action::OpenProject) {
                assert_bound(app)?;
            }
            self.before = Some(State::of(app));
            self.review_selection_before =
                matches!(step.action, Action::Mode(_)).then(|| ReviewSelection::of(app));
            self.step_started = Instant::now();
            self.age = 0;
            self.point = None;
            self.events.clear();
        }
        // Distinct clicks are separated in ordinary native time. This prevents
        // accidental outliner double-click focus across unrelated steps.
        const LEAD: u64 = 30;
        if self.age >= LEAD && self.age < LEAD + step.action.frames() {
            let start = input.events.len();
            self.inject(&step.action, self.age - LEAD, app, ctx, input)?;
            self.events
                .extend(input.events[start..].iter().map(|e| format!("{e:?}")));
        }
        if matches!(step.action, Action::Prepare) && app.bridge.mutation_pending() {
            self.background_input(app, ctx, input)?;
        }
        self.age += 1;
        if self.age < LEAD + step.action.frames() + 18 || !idle(app) {
            return Ok(ScenarioStatus::Running);
        }
        if self.background_pan.is_some() {
            self.background_input(app, ctx, input)?;
            return Ok(ScenarioStatus::Running);
        }
        assert_bound(app)?;
        let result = self.check(&step.check, app, ctx);
        self.report.assertions.push(Assertion {
            name: step.name.into(),
            passed: result.is_ok(),
            elapsed_ms: self.step_started.elapsed().as_millis(),
            since_start_ms: self.started.elapsed().as_millis(),
            before: self.before.take().expect("step initialized"),
            after: State::of(app),
            inputs: self.events.clone(),
        });
        self.dirty = true;
        result.map_err(|error| {
            format!(
                "Step {} '{}': {error}; status={}",
                self.index + 1,
                step.name,
                app.status
            )
        })?;
        self.index += 1;
        if let Some(name) = step.capture {
            self.capture = Some(Capture {
                name,
                state: State::of(app),
                started: Instant::now(),
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                name.to_string(),
            )));
        }
        Ok(ScenarioStatus::Running)
    }

    fn receive_capture(&mut self, app: &StudioApp, input: &egui::RawInput) -> Result<(), String> {
        let capture = self.capture.as_ref().expect("pending capture");
        if !capture.state.same_capture_context(&State::of(app)) || !idle(app) {
            return Err(format!(
                "Semantic or presentation context changed while capturing {}",
                capture.name
            ));
        }
        let image = input.events.iter().find_map(|e| match e {
            Event::Screenshot {
                user_data, image, ..
            } if user_data
                .data
                .as_ref()
                .and_then(|v| v.downcast_ref::<String>())
                .is_some_and(|name| name == capture.name) =>
            {
                Some(image)
            }
            _ => None,
        });
        if let Some(image) = image {
            let directory = app
                .args
                .gallery
                .as_ref()
                .ok_or("Real gallery directory missing")?;
            std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
            let path = directory.join(format!("{}.png", capture.name));
            if path.exists() {
                return Err(format!("Refusing to overwrite capture {}", path.display()));
            }
            let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            image::save_buffer(
                &path,
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            )
            .map_err(|e| e.to_string())?;
            let evidence = serde_json::json!({
                "format": "agentique-native-real-gallery/1", "semantic_data": "real authenticated models/agentique",
                "fixture": null, "checkpoint": capture.name, "state": capture.state,
                "baseline": self.report.baseline, "committed": self.report.committed,
                "engineering_evidence": self.report.engineering_evidence,
                // Preserve the actual revision-bound view and disposable
                // geometry for independent layout diagnosis after this process.
                "projection": app.active_projection(),
                "layout": app.scene.memory(),
                "image_size": image.size, "image_digest": ContentDigest::of(&std::fs::read(&path).map_err(|e| e.to_string())?),
                "adapter": app.adapter, "metrics": app.metrics_report(),
            });
            std::fs::write(
                path.with_extension("json"),
                serde_json::to_vec_pretty(&evidence).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            self.report.gallery.push(path.display().to_string());
            self.capture = None;
            self.dirty = true;
        } else if capture.started.elapsed().as_secs() > 30 {
            return Err(format!(
                "Native screenshot {} was not delivered within 30 seconds",
                capture.name
            ));
        }
        Ok(())
    }

    fn inject(
        &mut self,
        action: &Action,
        frame: u64,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<(), String> {
        match action {
            Action::OpenProject => {
                let project = app
                    .projects
                    .iter()
                    .find(|p| {
                        self.report
                            .project
                            .map_or(p.name == "Agentique", |id| p.id == id)
                    })
                    .ok_or("Expected real Agentique project is absent")?;
                click(
                    input,
                    real_targets::target(ctx, Target::Project(project.id))?.center(),
                    frame == 0,
                );
            }
            Action::Select(wanted) => outliner_input(
                app,
                ctx,
                input,
                frame,
                wanted.name,
                matches!(frame, 8 | 9)
                    .then(|| named(app, *wanted))
                    .transpose()?,
            )?,
            Action::InspectFeature(name) => {
                let point = if let Some(point) = self.point {
                    point
                } else {
                    let inspector = app
                        .inspector
                        .as_ref()
                        .ok_or("No real Inspector to navigate from")?;
                    let id = inspector_feature(inspector, name)?;
                    self.inspected_feature = Some(id);
                    let point = real_targets::target(ctx, Target::InspectorElement(id))?.center();
                    self.point = Some(point);
                    point
                };
                click(input, point, frame == 0);
            }
            Action::Palette(query) => match frame {
                0 => key(input, Key::K, Modifiers::COMMAND),
                5 | 6 => click(
                    input,
                    automation::target(ctx, automation::Target::PaletteInput)?.center(),
                    frame == 5,
                ),
                7 => key(input, Key::A, Modifiers::COMMAND),
                8 => input.events.push(Event::Text((*query).into())),
                10 => {
                    if app.palette_query != *query {
                        return Err("Palette did not retain native text input".into());
                    }
                    key(input, Key::Enter, Modifiers::NONE);
                }
                _ => {}
            },
            Action::Key(k) => key(input, *k, Modifiers::NONE),
            Action::Standards => click(
                input,
                real_targets::target(ctx, Target::Standards)?.center(),
                frame == 0,
            ),
            Action::DerivedEdge => {
                let point = if let Some(point) = self.point {
                    point
                } else {
                    let point = derived_point(app, ctx)?;
                    self.point = Some(point);
                    point
                };
                click(input, point, frame == 0);
            }
            Action::Prepare => match frame {
                0 | 1 => click(
                    input,
                    automation::target(ctx, automation::Target::CandidateName)?.center(),
                    frame == 0,
                ),
                2 => key(input, Key::A, Modifiers::COMMAND),
                3 => input.events.push(Event::Text(PART_NAME.into())),
                5 | 6 => {
                    if frame == 5 && app.new_part_name != PART_NAME {
                        return Err("Candidate name did not retain native text input".into());
                    }
                    if frame == 6 {
                        let now = Instant::now();
                        self.preparation_clock = Some(PreparationClock {
                            started: now,
                            previous: now,
                        });
                        self.report.preparation_responsiveness =
                            PreparationResponsiveness::default();
                    }
                    click(
                        input,
                        automation::target(ctx, automation::Target::CandidatePrepare)?.center(),
                        frame == 5,
                    );
                }
                _ => {}
            },
            Action::Mode(mode) => {
                let target = match mode {
                    ComparisonMode::Current => Target::ComparisonCurrent,
                    ComparisonMode::Candidate => Target::ComparisonCandidate,
                    ComparisonMode::Diff => Target::ComparisonDiff,
                };
                click(
                    input,
                    real_targets::target(ctx, target)?.center(),
                    frame == 0,
                );
            }
            Action::HistoryBaseline => {
                let revision = self
                    .report
                    .baseline
                    .as_ref()
                    .ok_or("No retained baseline")?
                    .revision_id;
                click(
                    input,
                    automation::target(ctx, automation::Target::HistoryRevision(revision))?
                        .center(),
                    frame == 0,
                );
            }
        }
        Ok(())
    }

    fn background_input(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<(), String> {
        if app.bridge.mutation_pending() {
            self.report.background_frames += 1;
            if app.binding.map(|b| b.revision)
                != self.report.baseline.as_ref().map(|m| m.revision_id)
                || app.scene.revision_id
                    != self
                        .report
                        .baseline
                        .as_ref()
                        .ok_or("Baseline absent")?
                        .revision_id
            {
                return Err(
                    "Candidate reconstruction replaced the current world before completion".into(),
                );
            }
        }
        if self.report.background_pan_observed {
            return self.background_inspection_input(app, ctx, input);
        }
        if self.background_pan.is_none() {
            // The Prepare-release pass still contains the centered dialog.
            // egui resolves a new press against the previous pass's widgets.
            // Let that hit data expire without stopping the preparation clock.
            if app.create_dialog || app.palette || app.camera_target.is_some() {
                self.background_pan_ready_frame = None;
                return Ok(());
            }
            let ready_frame = *self
                .background_pan_ready_frame
                .get_or_insert(app.frame_number);
            if app.frame_number < ready_frame + 2 {
                return Ok(());
            }
            let rect = automation::target(ctx, automation::Target::Viewport)?;
            self.background_pan = Some((0, background_pan_start(rect)?, app.camera.center));
            self.events.push(format!("Background pan begins at frame {} after dialog settled at frame {ready_frame}, camera {:?}", app.frame_number, app.camera.center));
        }
        let first_event = input.events.len();
        let (frame, point, before) = self.background_pan.as_mut().expect("initialized pan");
        match *frame {
            0 => click(input, *point, true),
            1 | 2 => input.events.push(Event::PointerMoved(
                *point + Vec2::new(35.0 * *frame as f32, 20.0 * *frame as f32),
            )),
            3 => click(input, *point + Vec2::new(70.0, 40.0), false),
            _ => {
                if app.camera.center.distance(*before) < 1.0 {
                    return Err(
                        "Native pan did not move the current camera during background preparation"
                            .into(),
                    );
                }
                self.report.background_pan_observed = app.bridge.mutation_pending();
                self.events.push(format!(
                    "Background native pan: camera {:?} -> {:?}, mutation_pending={}",
                    before,
                    app.camera.center,
                    app.bridge.mutation_pending()
                ));
                self.background_pan = None;
                return Ok(());
            }
        }
        *frame += 1;
        self.events
            .extend(input.events[first_event..].iter().map(|event| {
                format!(
                    "Background pan input at frame {}: {event:?}",
                    app.frame_number
                )
            }));
        Ok(())
    }

    fn background_inspection_input(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<(), String> {
        if self.report.background_current_inspection.is_some() || !app.bridge.mutation_pending() {
            return Ok(());
        }
        if self.background_inspection.is_none() {
            let owner = self
                .report
                .added_owner
                .ok_or("No retained create intent owner")?;
            // Choose an actual source-backed part in this loaded scene. The
            // overview's intentional depth may omit other named subsystems.
            let node = app.active_projection().nodes.iter().find(|node| {
                node.id != owner && node.origin == ViewOrigin::Authored && node.source_available
                    && matches!(node.semantic_kind.as_str(), "PartDefinition" | "PartUsage")
                    && !node.name.is_empty() && app.scene.node(node.id).is_some()
            }).ok_or("Loaded current architecture has no other authored part to inspect during preparation")?;
            self.background_inspection = Some(BackgroundInspection {
                binding: app
                    .binding
                    .ok_or("Current binding disappeared during preparation")?,
                runtime_epoch: app.bridge.epoch(),
                element: node.id,
                name: node.name.clone(),
                age: 0,
                started: Instant::now(),
                selection_started_frame: app.frame_number,
                previous_request: app.inspector_request,
                request: None,
            });
        }
        let observation = self
            .background_inspection
            .as_mut()
            .expect("initialized inspection");
        if app.binding != Some(observation.binding)
            || app.scene.revision_id != observation.binding.revision
            || app.bridge.epoch() != observation.runtime_epoch
        {
            return Err("Current revision changed during background Inspector input".into());
        }
        if observation.age < 12 {
            let start = input.events.len();
            outliner_input(
                app,
                ctx,
                input,
                observation.age,
                &observation.name,
                Some(observation.element),
            )?;
            self.events.extend(
                input.events[start..]
                    .iter()
                    .map(|event| format!("Background Inspector input: {event:?}")),
            );
            observation.age += 1;
        }
        if observation.age >= 10
            && app.selected_element() == Some(observation.element)
            && app.inspector_request != 0
            && app.inspector_request != observation.previous_request
        {
            observation.request.get_or_insert((
                app.inspector_request,
                app.frame_number,
                Instant::now(),
            ));
        }
        if let Some((request, request_observed_frame, started)) = observation.request
            && app.inspector_request == request
            && app.selected_element() == Some(observation.element)
            && let Some(inspector) = app.inspector.as_ref().filter(|inspector| {
                inspector.element.id == observation.element
                    && inspector.revision_id == observation.binding.revision
            })
        {
            self.report.background_current_inspection = Some(BackgroundInspectionEvidence {
                binding: observation.binding,
                runtime_epoch: observation.runtime_epoch,
                element: observation.element,
                request,
                selection_started_frame: observation.selection_started_frame,
                request_observed_frame,
                response_observed_frame: app.frame_number,
                selection_to_response_ms: observation.started.elapsed().as_millis(),
                request_observed_to_response_ms: started.elapsed().as_millis(),
                mutation_pending_at_response: app.bridge.mutation_pending(),
                inspector: inspector.clone(),
            });
            self.events.push(format!("Background Inspector received request {request} for {} at baseline {} while candidate preparation remained pending", observation.element, observation.binding.revision));
            self.background_inspection = None;
            self.dirty = true;
        }
        Ok(())
    }

    fn check(&mut self, check: &Check, app: &StudioApp, ctx: &egui::Context) -> Result<(), String> {
        let require = |ok: bool, reason: &str| if ok { Ok(()) } else { Err(reason.to_string()) };
        match check {
            Check::Baseline => {
                let manifest = current_manifest(app)?;
                assert_validated(manifest)?;
                // Resume never substitutes a prior source proof for this process.
                assert_self_model_sources(app, manifest)?;
                if let Some(baseline) = &self.report.baseline {
                    require(
                        manifest == baseline
                            && app.comparison == ComparisonMode::Current
                            && app.binding.map(|binding| binding.project) == self.report.project
                            && app.branch == self.report.branch
                            && app.history.as_ref().is_some_and(|history| {
                                history.branches.iter().any(|branch| {
                                    Some(branch.id) == self.report.branch
                                        && branch.head == baseline.revision_id
                                })
                            }),
                        "Baseline revision or comparison mode changed",
                    )?;
                } else {
                    self.report.project = Some(manifest.project_id);
                    self.report.branch = app.branch;
                    self.report.baseline = Some(manifest.clone());
                }
                require(
                    app.gpu_stats.lock().is_ok_and(|s| s.draw_calls > 0),
                    "Native GPU scene has not drawn",
                )
            }
            Check::Selected(wanted) => {
                let id = named(app, *wanted)?;
                require(
                    app.selected_element() == Some(id)
                        && app.inspector.as_ref().is_some_and(|i| {
                            i.element.id == id && i.revision_id == app.scene.revision_id
                        }),
                    "Selection and real Inspector are not bound to the chosen canonical object",
                )?;
                if wanted.name == PART_NAME {
                    assert_part_owner(
                        app.active_projection(),
                        id,
                        self.report
                            .added_owner
                            .ok_or("Created part owner was not retained")?,
                    )?;
                    if let Some(expected) = self.report.added_element {
                        require(
                            id == expected,
                            "Created canonical identity changed across candidate/restart",
                        )?;
                    } else {
                        self.report.added_element = Some(id);
                    }
                    if app.comparison != ComparisonMode::Current
                        && app.candidate.as_ref().is_some_and(|candidate| {
                            candidate.after.revision_id == app.scene.revision_id
                        })
                    {
                        self.explicitly_selected_candidate_part = Some(id);
                    }
                }
                Ok(())
            }
            Check::FocusedPlatform => require(
                app.world == World::System
                    && app.active_projection().view.kind == ViewKind::Architecture
                    && app.focus == Some(named(app, PLATFORM)?)
                    && app.active_projection().view.focus == app.focus
                    && app.scene.node(named(app, REPOSITORY)?).is_some(),
                "Focused ModelingPlatform must expose the real ModelRepository for navigation without returning Home",
            ),
            Check::RepositoryInterfaces => {
                self.check(&Check::FocusedPlatform, app, ctx)?;
                self.check(&Check::Selected(REPOSITORY), app, ctx)?;
                let inspector = app
                    .inspector
                    .as_ref()
                    .ok_or("Repository Inspector unavailable")?;
                let inherited = inspector_feature(inspector, "repositoryRevisions")?;
                require(
                    complete_query(inspector, "Effective features")
                        && complete_query(inspector, "Owned features")
                        && inspector
                            .effective_features
                            .iter()
                            .any(|feature| feature.id == inherited)
                        && !inspector
                            .owned_features
                            .iter()
                            .any(|feature| feature.id == inherited),
                    "Repository Inspector must identify repositoryRevisions as an effective, inherited canonical port",
                )?;
                self.report.engineering_evidence.repository_interfaces = Some(inspector.clone());
                Ok(())
            }
            Check::FocusedRepository => {
                self.check(&Check::Selected(REPOSITORY), app, ctx)?;
                let inspector = app
                    .inspector
                    .as_ref()
                    .ok_or("Repository Inspector unavailable")?;
                let inherited = inspector_feature(inspector, "repositoryRevisions")?;
                let previous = self
                    .report
                    .engineering_evidence
                    .repository_interfaces
                    .as_ref()
                    .ok_or("Inherited interface observation missing")?;
                require(
                    app.world == World::System
                        && app.active_projection().view.kind == ViewKind::Architecture
                        && app.focus == Some(named(app, REPOSITORY)?)
                        && app.active_projection().view.focus == app.focus
                        && inherited == inspector_feature(previous, "repositoryRevisions")?
                        && app.lookup.port(&app.scene, inherited).is_some()
                        && real_targets::target(ctx, Target::InspectorElement(inherited)).is_ok(),
                    "Entering ModelRepository must expose its original inherited port through a visible ordinary Inspector link",
                )
            }
            Check::InheritedPort => {
                let inherited = inspector_feature(
                    self.report
                        .engineering_evidence
                        .repository_interfaces
                        .as_ref()
                        .ok_or("Inherited interface observation missing")?,
                    "repositoryRevisions",
                )?;
                require(
                    self.inspected_feature == Some(inherited)
                        && app.focus == Some(named(app, REPOSITORY)?),
                    "Inherited port click changed canonical identity or left focused ModelRepository",
                )?;
                let inspector = assert_port_inspector(
                    app,
                    inherited,
                    "repositoryRevisions",
                    Named {
                        name: "Repository",
                        kind: "PartDefinition",
                    },
                    Named {
                        name: "ModelRevision",
                        kind: "PortDefinition",
                    },
                )?;
                self.report.engineering_evidence.inherited_port = Some(inspector.clone());
                Ok(())
            }
            Check::ConnectedPort => {
                self.check(&Check::FocusedPlatform, app, ctx)?;
                let id = self
                    .inspected_feature
                    .ok_or("No ordinary platform port click was recorded")?;
                let inspector = assert_port_inspector(
                    app,
                    id,
                    "clientQueries",
                    PLATFORM,
                    Named {
                        name: "SemanticQuery",
                        kind: "PortDefinition",
                    },
                )?;
                let edge = platform_connection(app.active_projection(), inspector)?;
                require(
                    app.scene.edges.iter().any(|item| item.semantic == *edge)
                        && real_targets::target(
                            ctx,
                            Target::InspectorRelationship(edge.id.clone()),
                        )
                        .is_ok(),
                    "Actual port connection must be visible in the scene and ordinary Inspector",
                )?;
                self.report.engineering_evidence.connected_port = Some(inspector.clone());
                Ok(())
            }
            Check::Home => require(
                app.world == World::System
                    && app.focus.is_none()
                    && app.comparison == ComparisonMode::Current,
                "System World did not return to the current architecture",
            ),
            Check::World(world) => {
                require(app.world == *world, "Requested World did not open")?;
                let expected_kind = match world {
                    World::System => ViewKind::Architecture,
                    World::Graph | World::History => ViewKind::SemanticGraph,
                    World::Requirements => ViewKind::Requirements,
                };
                require(
                    app.active_projection().view.kind == expected_kind,
                    "Requested World retained a projection from a different World",
                )?;
                if *world == World::Requirements {
                    require(
                        app.active_projection().view.kind == ViewKind::Requirements
                            && app
                                .scene
                                .nodes
                                .iter()
                                .any(|n| n.category == NodeCategory::Requirement),
                        "Requirements World has no actual modeled requirements",
                    )?;
                    let path = requirement_subject_path(app.active_projection())?;
                    require(
                        path.iter()
                            .all(|edge| app.scene.edges.iter().any(|item| item.semantic == *edge)),
                        "Requirements scene omits a canonical edge of ImmutableRevisions -> subjectWorkspace -> ProjectWorkspace",
                    )?;
                    self.report.engineering_evidence.requirement_subject_path = path;
                }
                require(
                    !app.scene.nodes.is_empty() && !app.scene.edges.is_empty(),
                    "Real World has no inspectable semantic relationships",
                )
            }
            Check::Dependencies => {
                let projection = app.active_projection();
                let root = named(app, REPOSITORY)?;
                require(
                    app.world == World::Graph
                        && app.show_agent
                        && app.focus == Some(root)
                        && projection.view.focus == Some(root)
                        && projection.view.kind == ViewKind::SemanticGraph
                        && !projection.edges.is_empty()
                        && app.agent_activity.as_ref().is_some_and(|activity| {
                            activity.root == root
                                && activity.root_name == REPOSITORY.name
                                && activity.revision == projection.revision_id
                                && activity.complete
                                && activity.error.is_none()
                                && !activity.fixture
                                && activity.result_count == projection.nodes.len()
                        })
                        && app.dependencies.as_ref().is_some_and(|targets| {
                            *targets == projection.nodes.iter().map(|node| node.id).collect()
                        }),
                    "Agent action did not return the completed revision-bound ModelRepository dependency view and matching overlay",
                )
            }
            Check::GraphOverview => {
                require(
                    app.world == World::Graph
                        && app.focus.is_none()
                        && app.active_projection().view.focus.is_none(),
                    "Graph overview retained a neighborhood focus",
                )?;
                let id = named(app, AGENT_RUNTIME)?;
                require(
                    app.scene.node(id).is_some(),
                    "Graph overview omits the Agent Fabric added by bootstrap",
                )
            }
            Check::Standards => require(
                app.include_standard
                    && app.active_projection().view.include_standard_library
                    && app.scene.edges.iter().any(|e| {
                        e.semantic.origin == ViewOrigin::Derived
                            && e.semantic.relationship_id.is_some()
                    }),
                "No derived relationship from authenticated standards is visible",
            ),
            Check::DerivedEdge => require(
                app.selection.primary.as_ref().is_some_and(|t| match t {
                    SceneTarget::Edge(id) => app.lookup.edge(&app.scene, id).is_some_and(|e| {
                        e.semantic.origin == ViewOrigin::Derived
                            && e.semantic.relationship_id.is_some()
                    }),
                    _ => false,
                }),
                "Pointer did not select a canonical derived relationship",
            ),
            Check::Explanation => {
                self.check(&Check::DerivedEdge, app, ctx)?;
                let evidence = app
                    .explanation
                    .as_ref()
                    .ok_or("Real explanation worker returned no evidence")?;
                require(
                    app.show_explain
                        && evidence.subject_id
                            == app.selected_element().ok_or("No explanation subject")?
                        && evidence.origin == ViewOrigin::Derived
                        && evidence.rule_id.is_some()
                        && evidence.evidence_count > 0
                        && !evidence.edges.is_empty()
                        && evidence
                            .nodes
                            .iter()
                            .any(|n| n.kind == ExplanationNodeKind::Rule)
                        && automation::target(ctx, automation::Target::ExplainWindow).is_ok(),
                    "Explain lacks actual derived rule/evidence or a native visible window",
                )
            }
            Check::ExplanationClosed => require(!app.show_explain, "Explain did not dismiss"),
            Check::History => {
                require(app.world == World::History, "History World did not open")?;
                let manifest = current_manifest(app)?;
                assert_validated(manifest)?;
                let expected = self
                    .report
                    .committed
                    .as_ref()
                    .or(self.report.baseline.as_ref())
                    .ok_or("No expected durable revision")?;
                require(
                    manifest == expected,
                    "History is not showing the expected durable revision",
                )
            }
            Check::ParentDiff => {
                let manifest = current_manifest(app)?;
                require(
                    app.comparison == ComparisonMode::Diff
                        && app.compare_before.as_ref().map(|p| p.revision_id)
                            == manifest.parent_revision_id
                        && app
                            .compare_before
                            .as_ref()
                            .is_some_and(|before| before.view == app.projection.view)
                        && app
                            .scene
                            .node(named(app, AGENT_RUNTIME)?)
                            .is_some_and(|node| node.diff == DiffMark::Added),
                    "Parent difference lacks exact parent, matching lens, or the actual added AgentRuntime",
                )
            }
            Check::CreateDialog => {
                let owner = named(app, PLATFORM)?;
                require(
                    app.create_dialog
                        && !app.palette
                        && app.selected_element() == Some(owner)
                        && app.part_edit_ready(),
                    "Create Part dialog did not open with the selected ModelingPlatform's installed owner view ready for preparation",
                )?;
                self.report.added_owner = Some(owner);
                Ok(())
            }
            Check::CandidateWorking => {
                let candidate = app
                    .candidate
                    .as_ref()
                    .ok_or("No real candidate was returned")?;
                assess_background_inspection(
                    self.report.background_current_inspection.as_ref(),
                    agq_studio_platform::RevisionBinding {
                        project: self
                            .report
                            .project
                            .ok_or("Baseline project was not retained")?,
                        revision: candidate.before.revision_id,
                    },
                    self.report
                        .added_owner
                        .ok_or("Create intent owner was not retained")?,
                    app.bridge.epoch(),
                )?;
                require(
                    candidate.id.is_some()
                        && candidate.phase == Some(CandidatePhase::Working)
                        && candidate.before.revision_id
                            == self
                                .report
                                .baseline
                                .as_ref()
                                .ok_or("No baseline")?
                                .revision_id
                        && candidate.after.revision_id != candidate.before.revision_id
                        && candidate.source.contains(&format!("part {PART_NAME};")),
                    "Working candidate lacks actual identity/source/base binding",
                )?;
                require(
                    current_manifest(app)?.revision_id == candidate.before.revision_id,
                    "Working candidate advanced durable history",
                )?;
                self.candidate_id = candidate.id;
                self.candidate_revision = Some(candidate.after.revision_id);
                let matches: Vec<_> = candidate
                    .after
                    .nodes
                    .iter()
                    .filter(|node| node.name == PART_NAME && node.semantic_kind == "PartUsage")
                    .collect();
                let [added] = matches.as_slice() else {
                    return Err("Candidate must contain exactly one new named PartUsage".into());
                };
                assert_part_owner(
                    &candidate.after,
                    added.id,
                    self.report
                        .added_owner
                        .ok_or("Create intent owner was not retained")?,
                )?;
                require(
                    !candidate.before.nodes.iter().any(|node| {
                        node.id == added.id
                            || (node.name == PART_NAME && node.semantic_kind == "PartUsage")
                    }),
                    "Created canonical part or reserved test name already exists in the same baseline lens",
                )?;
                self.report.added_element = Some(added.id);
                self.report.preparation_responsiveness.assessment = Some(
                    assess_preparation(
                        &self.report.preparation_responsiveness,
                        self.report.background_pan_observed,
                    )?
                    .into(),
                );
                Ok(())
            }
            Check::Mode(mode) => {
                let candidate = app
                    .candidate
                    .as_ref()
                    .ok_or("Candidate disappeared during review")?;
                require(
                    candidate.id == self.candidate_id
                        && Some(candidate.after.revision_id) == self.candidate_revision
                        && candidate.before.view == candidate.after.view
                        && app.comparison == *mode,
                    "Candidate identity, matching lens, or review mode is wrong",
                )?;
                let expected = if *mode == ComparisonMode::Current {
                    candidate.before.revision_id
                } else {
                    candidate.after.revision_id
                };
                require(
                    app.scene.revision_id == expected,
                    "Candidate mode mixed revisions",
                )?;
                let before = self
                    .review_selection_before
                    .as_ref()
                    .ok_or("Review mode has no pre-input selection evidence")?;
                require(
                    app.focus == before.focus,
                    "Review mode changed the engineering focus",
                )?;
                let added = self
                    .report
                    .added_element
                    .ok_or("Created part identity was not retained")?;
                if *mode == ComparisonMode::Current {
                    require(
                        self.explicitly_selected_candidate_part == Some(added)
                            && app.scene.node(added).is_none()
                            && app.selected_element() != Some(added),
                        "Candidate-only object leaked into Current or was never explicitly selected",
                    )?;
                } else {
                    assert_candidate_review_selection(
                        before,
                        &ReviewSelection::of(app),
                        candidate.after.revision_id,
                        added,
                        self.explicitly_selected_candidate_part,
                    )?;
                }
                if *mode == ComparisonMode::Diff {
                    require(
                        app.scene
                            .nodes
                            .iter()
                            .any(|n| n.semantic.name == PART_NAME && n.diff == DiffMark::Added),
                        "Added part is absent from semantic difference",
                    )?;
                }
                Ok(())
            }
            Check::Validated => require(
                app.candidate.as_ref().is_some_and(|c| {
                    c.id == self.candidate_id
                        && Some(c.after.revision_id) == self.candidate_revision
                        && c.phase == Some(CandidatePhase::Validated)
                }),
                "Candidate was not validated by the real platform",
            ),
            Check::Committed => {
                let manifest = current_manifest(app)?;
                assert_validated(manifest)?;
                require(
                    app.candidate.is_none()
                        && Some(manifest.revision_id) == self.candidate_revision
                        && manifest.parent_revision_id
                            == self.report.baseline.as_ref().map(|m| m.revision_id)
                        && app.comparison == ComparisonMode::Current,
                    "Commit did not durably advance exactly the validated candidate",
                )?;
                require(
                    app.history.as_ref().is_some_and(|h| {
                        h.branches.iter().any(|b| {
                            Some(b.id) == self.report.branch && b.head == manifest.revision_id
                        })
                    }),
                    "Branch history did not confirm the committed head",
                )?;
                require(
                    Some(named(app, PART)?) == self.report.added_element,
                    "Committed projection lost or changed the reviewed canonical part identity",
                )?;
                assert_part_owner(
                    app.active_projection(),
                    named(app, PART)?,
                    self.report
                        .added_owner
                        .ok_or("Committed part owner was not retained")?,
                )?;
                self.report.committed = Some(manifest.clone());
                Ok(())
            }
            Check::RestartRevision => {
                let manifest = current_manifest(app)?;
                assert_validated(manifest)?;
                require(
                    self.report.committed.as_ref() == Some(manifest)
                        && app.branch == self.report.branch
                        && app.binding.map(|b| b.project) == self.report.project,
                    "Reauthenticated durable revision/receipt differs from the first process",
                )?;
                require(
                    app.history.as_ref().is_some_and(|h| {
                        h.branches.iter().any(|b| {
                            Some(b.id) == self.report.branch && b.head == manifest.revision_id
                        })
                    }),
                    "Restarted branch head differs from committed revision",
                )
            }
            Check::Restart => {
                // The fresh --no-restore overview is intentionally shallow.
                // Verify durable authority on open, then repeat that check here
                // after ordinary selection/focus exposes the nested object.
                self.check(&Check::RestartRevision, app, ctx)?;
                self.check(&Check::FocusedPlatform, app, ctx)?;
                assert_restart_element(
                    app.active_projection(),
                    app.focus,
                    app.inspector.as_ref(),
                    self.report
                        .added_element
                        .ok_or("Restarted part identity was not retained")?,
                    self.report
                        .added_owner
                        .ok_or("Restarted part owner was not retained")?,
                )?;
                self.check(&Check::Selected(PART), app, ctx)
            }
        }
    }
}

fn idle(app: &StudioApp) -> bool {
    app.pending.is_empty()
        && !app.bridge.mutation_pending()
        && !app.scene_builder.busy
        // Setup and History do not render the canvas that consumes fit_pending.
        && (!app.ready
            || app.world == World::History
            || (!app.fit_pending && app.camera_target.is_none()))
}

fn assert_bound(app: &StudioApp) -> Result<(), String> {
    let binding = app.binding.ok_or("No real repository revision is bound")?;
    let projection = app.active_projection();
    if !app.ready
        || app.fixture.is_some()
        || app.branch.is_none()
        || app.projection.revision_id != binding.revision
        || app.scene.revision_id != projection.revision_id
        || app.selection.revision != app.scene.revision_id
        || projection
            .nodes
            .iter()
            .any(|n| n.revision_id != projection.revision_id)
        || projection
            .edges
            .iter()
            .any(|e| e.revision_id != projection.revision_id)
        || projection.metadata.producer_completeness != "Complete"
    {
        return Err("Real view has incomplete closure or mixed project/projection/scene/selection revision bindings".into());
    }
    if let Some(inspector) = &app.inspector
        && !app.selected_context().is_some_and(|(b, _, id)| {
            b.revision == inspector.revision_id && id == inspector.element.id
        })
    {
        return Err("Inspector is stale or bound to another selection/revision".into());
    }
    if let Some(explanation) = &app.explanation
        && !app.selected_context().is_some_and(|(b, _, id)| {
            b.revision == explanation.revision_id && id == explanation.subject_id
        })
    {
        return Err("Explain is stale or bound to another selection/revision".into());
    }
    Ok(())
}

fn current_manifest(app: &StudioApp) -> Result<&RevisionManifest, String> {
    let binding = app.binding.ok_or("No durable binding")?;
    app.history
        .as_ref()
        .filter(|h| h.project.id == binding.project)
        .and_then(|h| {
            h.revisions
                .iter()
                .find(|m| m.revision_id == binding.revision)
        })
        .ok_or_else(|| "Bound revision has no matching actual repository manifest".into())
}
fn assert_validated(manifest: &RevisionManifest) -> Result<(), String> {
    if matches!(manifest.validation, ValidationState::Validated(_)) {
        Ok(())
    } else {
        Err("Actual durable revision is Working, not Validated".into())
    }
}
fn assert_self_model_sources(app: &StudioApp, manifest: &RevisionManifest) -> Result<(), String> {
    assert_source_manifest(&app.config.root, manifest)
}

fn assert_source_manifest(root: &Path, manifest: &RevisionManifest) -> Result<(), String> {
    let entries = std::fs::read_dir(root.join("models/agentique")).map_err(|e| e.to_string())?;
    let mut expected = BTreeMap::new();
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "sysml") {
            expected.insert(
                path.file_name()
                    .ok_or("Invalid source filename")?
                    .to_string_lossy()
                    .to_string(),
                ContentDigest::of(&std::fs::read(path).map_err(|e| e.to_string())?),
            );
        }
    }
    let actual: BTreeMap<_, _> = manifest
        .documents
        .iter()
        .map(|document| (document.path.clone(), document.content_digest))
        .collect();
    if expected.is_empty() || actual.len() != manifest.documents.len() || actual != expected {
        return Err(
            "Repository source identities do not exactly match current models/agentique/*.sysml"
                .into(),
        );
    }
    Ok(())
}

fn key(input: &mut egui::RawInput, key: Key, mut modifiers: Modifiers) {
    if modifiers.command {
        modifiers.ctrl = !cfg!(target_os = "macos");
        modifiers.mac_cmd = cfg!(target_os = "macos");
    }
    input.modifiers = modifiers;
    for pressed in [true, false] {
        input.events.push(Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        });
    }
}
fn outliner_input(
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    frame: u64,
    name: &str,
    element: Option<ElementId>,
) -> Result<(), String> {
    match frame {
        0 | 1 => click(
            input,
            real_targets::target(ctx, Target::ExplorerSearch)?.center(),
            frame == 0,
        ),
        2 => key(input, Key::A, Modifiers::COMMAND),
        3 => input.events.push(Event::Text(name.into())),
        8 | 9 => {
            if app.search != name {
                return Err("Explorer did not retain native text input".into());
            }
            let element = element.ok_or("No canonical object for native Explorer input")?;
            if !app
                .active_projection()
                .nodes
                .iter()
                .any(|node| node.id == element && node.name == name)
            {
                return Err(
                    "Explorer target is absent from the current semantic projection".into(),
                );
            }
            click(
                input,
                real_targets::target(ctx, Target::ExplorerElement(element))?.center(),
                frame == 8,
            );
        }
        _ => {}
    }
    Ok(())
}

fn click(input: &mut egui::RawInput, position: Pos2, pressed: bool) {
    input.events.push(Event::PointerMoved(position));
    input.events.push(Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}
fn screen(app: &StudioApp, viewport: Rect, world: Point) -> Option<Pos2> {
    let point = app.camera.world_to_screen(world);
    let point = viewport.min + Vec2::new(point.x, point.y);
    viewport.shrink(4.0).contains(point).then_some(point)
}
fn derived_point(app: &StudioApp, ctx: &egui::Context) -> Result<Pos2, String> {
    let viewport = automation::target(ctx, automation::Target::Viewport)?;
    for edge in app.scene.edges.iter().filter(|e| {
        e.semantic.origin == ViewOrigin::Derived && e.semantic.relationship_id.is_some()
    }) {
        for pair in edge.points.windows(2) {
            for t in [0.5, 0.25, 0.75] {
                let p = Point::new(
                    pair[0].x + (pair[1].x - pair[0].x) * t,
                    pair[0].y + (pair[1].y - pair[0].y) * t,
                );
                if app.spatial.hit_test(p, 6.0 / app.camera.zoom)
                    == Some(SceneTarget::Edge(edge.semantic.id.clone()))
                    && let Some(point) = screen(app, viewport, p)
                {
                    return Ok(point);
                }
            }
        }
    }
    Err("No visible unoccluded actual derived relationship can be selected; no semantic shortcut substituted".into())
}

pub fn drive(
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    report_path: &Path,
) -> Result<ScenarioStatus, String> {
    let id = egui::Id::new("agentique-native-real-scenario");
    let mut runner = match ctx.data(|data| data.get_temp::<Runner>(id)) {
        Some(runner) => runner,
        None => Runner::new(app)?,
    };
    let result = runner.advance(app, ctx, input);
    runner.report.elapsed_ms = runner.started.elapsed().as_millis();
    runner.report.last_state = State::of(app);
    match &result {
        Ok(ScenarioStatus::Complete) => {
            runner.report.passed = true;
            runner.report.restart_verified = runner.report.scenario == "real-restart";
            runner.report.outcome = if runner.report.restart_verified {
                "passed"
            } else {
                "journey_passed_restart_pending"
            }
            .into();
            runner.report.metrics = app.metrics_report();
        }
        Err(error) => {
            runner.report.outcome = "failed".into();
            runner.report.failure = Some(error.clone());
        }
        Ok(ScenarioStatus::Running) => {}
    }
    if runner.dirty
        || runner.last_write.elapsed().as_secs() >= 10
        || !matches!(result, Ok(ScenarioStatus::Running))
    {
        if let Some(parent) = report_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(
            report_path,
            serde_json::to_vec_pretty(&runner.report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        runner.dirty = false;
        runner.last_write = Instant::now();
    }
    ctx.data_mut(|data| data.insert_temp(id, runner));
    if matches!(result, Ok(ScenarioStatus::Running)) {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
    result
}

fn background_pan_start(rect: egui::Rect) -> Result<Pos2, String> {
    if !rect.is_finite() || rect.width() < 104.0 || rect.height() < 88.0 {
        return Err("Viewport is too small for the complete background pan gesture".into());
    }
    Ok(rect.left_top() + Vec2::splat(24.0))
}

#[cfg(test)]
#[path = "background_pan_tests.rs"]
mod background_pan_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn requirement_architecture_acceptance_requires_both_canonical_hops() {
        let mut projection = agq_studio_scene::fixtures::architecture();
        projection.nodes.truncate(3);
        let requirement = projection.nodes[0].id;
        let subject = projection.nodes[1].id;
        let architecture = projection.nodes[2].id;
        projection.nodes[0].name = "ImmutableRevisions".into();
        projection.nodes[0].semantic_kind = "RequirementDefinition".into();
        projection.nodes[1].name = "subjectWorkspace".into();
        projection.nodes[1].owner = Some(requirement);
        projection.nodes[2].name = "ProjectWorkspace".into();
        projection.nodes[2].semantic_kind = "PartDefinition".into();
        projection.edges.truncate(2);
        projection.edges[0].semantic_kind = "SubjectMembership".into();
        projection.edges[0].source = requirement;
        projection.edges[0].target = subject;
        projection.edges[1].semantic_kind = "FeatureTyping".into();
        projection.edges[1].source = subject;
        projection.edges[1].target = architecture;
        assert_eq!(
            requirement_subject_path(&projection).unwrap(),
            projection.edges
        );
        // Presentation family classification does not erase exact metaclass identity.
        projection.edges[0].family = RelationshipFamily::Requirement;
        assert!(requirement_subject_path(&projection).is_ok());
        for invalid in 0..7 {
            let mut changed = projection.clone();
            match invalid {
                0 => {
                    changed.edges.pop();
                }
                1 => changed.edges[1].source = requirement, // invented direct shortcut
                2 => changed.edges[1].relationship_id = None,
                3 => changed.edges[1].relationship_id = changed.edges[0].relationship_id,
                4 => changed.edges[0].semantic_kind = "OwningMembership".into(),
                5 => changed.nodes[1].owner = Some(architecture),
                6 => changed.edges[1].revision_id = ProjectRevisionId::new(),
                _ => unreachable!(),
            }
            assert!(
                requirement_subject_path(&changed).is_err(),
                "Accepted invalid requirement path {invalid}"
            );
        }
    }

    #[test]
    fn port_connection_acceptance_rejects_wrong_identity_endpoint_or_incomplete_query() {
        let mut projection = agq_studio_scene::fixtures::architecture();
        projection.nodes.truncate(3);
        for (node, name) in
            projection
                .nodes
                .iter_mut()
                .zip(["clientQueries", "platformQueries", "queryConnection"])
        {
            node.name = name.into();
            node.semantic_kind = if name == "queryConnection" {
                "InterfaceUsage"
            } else {
                "PortUsage"
            }
            .into();
            node.origin = ViewOrigin::Authored;
            node.source_available = true;
        }
        let connector = projection.nodes[2].id;
        let mut edge = projection.edges[0].clone();
        edge.family = RelationshipFamily::Connection;
        edge.semantic_kind = "InterfaceUsage".into();
        edge.source = projection.nodes[0].id;
        edge.target = projection.nodes[1].id;
        edge.relationship_id = Some(connector);
        projection.edges = vec![edge.clone()];
        let inspector = ElementInspector {
            revision_id: projection.revision_id,
            element: projection.nodes[0].clone(),
            owner: None,
            effective_types: vec![],
            owned_features: vec![],
            effective_features: vec![],
            feature_provenance: Default::default(),
            specializations: vec![],
            subsettings: vec![],
            redefinitions: vec![],
            relationships: vec![edge.clone()],
            multiplicity: None,
            source: None,
            queries: vec![agq_modeling_view::QuerySummary {
                name: format!("Connection endpoints: queryConnection [{connector}]"),
                completeness: "Complete".into(),
                diagnostics: vec![],
                positive_dependency_count: 0,
                search_dependency_count: 0,
            }],
            profile: "Test-only counterexample; never runtime acceptance".into(),
        };
        assert_eq!(platform_connection(&projection, &inspector).unwrap(), &edge);
        for invalid in 0..5 {
            let mut changed = inspector.clone();
            let mut changed_projection = projection.clone();
            match invalid {
                0 => changed.relationships[0].relationship_id = Some(projection.nodes[0].id),
                1 => changed.relationships[0].target = connector,
                2 => changed.queries[0].completeness = "Incomplete".into(),
                3 => changed.queries[0].name = "Connection endpoints: unrelated".into(),
                4 => changed.relationships[0].family = RelationshipFamily::Typing,
                _ => unreachable!(),
            }
            // Even two agreeing DTOs must not substitute a different relationship.
            changed_projection.edges = changed.relationships.clone();
            assert!(
                platform_connection(&changed_projection, &changed).is_err(),
                "Accepted invalid connection {invalid}"
            );
        }
    }

    fn resume_fixture() -> (Args, Report) {
        let mut args = isolated_args();
        args.root = args.database.as_ref().unwrap().with_extension("root");
        let sources = args.root.join("models/agentique");
        std::fs::create_dir_all(&sources).unwrap();
        let source = "part def ResumeTest;\n";
        std::fs::write(sources.join("ResumeTest.sysml"), source).unwrap();
        let mut fixture_args = args.clone();
        fixture_args.fixture = Some("architecture".into());
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let app = StudioApp::new(&context, fixture_args).unwrap();
        let mut report = Runner::new(&app).unwrap().report;
        let project = ProjectId::new();
        let branch = BranchId::new();
        let revision = ProjectRevisionId::new();
        let digest = ContentDigest::of(source.as_bytes());
        let manifest: RevisionManifest = serde_json::from_value(serde_json::json!({
            "format_version": 1, "project_id": project, "revision_id": revision,
            "parent_revision_id": null,
            "metadata": {"created": "2026-09-25T00:00:00Z", "name": null, "description": null, "alias": []},
            "documents": [{"document_id": ProjectId::new(), "path": "ResumeTest.sysml", "language": "SysMl", "source_revision_id": ProjectRevisionId::new(), "content_digest": digest}],
            "accepted_publications": {"kerml": digest, "sysml": digest},
            "checkpoint_digest": digest,
            "validation": {"Validated": {"acceptance_contract": "test-only", "source_binding": digest, "semantic_digest": digest, "semantic_context": digest, "closure_digest": digest}},
            "semantic_cache": null
        })).unwrap();
        report.outcome = "failed".into();
        report.failure = Some("Native screenshot was not delivered".into());
        report.project = Some(project);
        report.branch = Some(branch);
        report.baseline = Some(manifest);
        let mut state = State::of(&app);
        state.binding = Some(RevisionBinding { project, revision });
        state.branch = Some(branch);
        state.scene_revision = revision;
        state.selection_revision = revision;
        state.producer_completeness = "Complete".into();
        report.assertions.push(Assertion {
            name: steps(false)[0].name.into(),
            passed: true,
            elapsed_ms: 1,
            since_start_ms: 1,
            before: State::of(&app),
            after: state.clone(),
            inputs: vec![],
        });
        report.last_state = state;
        args.resume_report = Some(
            args.database
                .as_ref()
                .unwrap()
                .with_extension("failed.json"),
        );
        // Launch preflight checks paths/source evidence only. Ordinary authenticated
        // History must independently check the real database manifest before edits.
        std::fs::write(args.database.as_ref().unwrap(), b"isolated path sentinel").unwrap();
        std::fs::write(
            args.resume_report.as_ref().unwrap(),
            serde_json::to_vec(&report).unwrap(),
        )
        .unwrap();
        (args, report)
    }

    fn remove_resume_fixture(args: &Args) {
        std::fs::remove_dir_all(&args.root).unwrap();
        std::fs::remove_file(args.database.as_ref().unwrap()).unwrap();
        std::fs::remove_file(args.resume_report.as_ref().unwrap()).unwrap();
    }

    #[test]
    fn explicit_resume_repeats_real_steps_and_digests_the_exact_checked_report() {
        let (mut args, _) = resume_fixture();
        assert!(validate_launch(&args).is_ok());
        let (_, digest) = read_resume(&args).unwrap();
        assert_eq!(
            digest,
            ContentDigest::of(&std::fs::read(args.resume_report.as_ref().unwrap()).unwrap())
        );
        let predecessor = args.resume_report.take();
        assert!(validate_launch(&args).unwrap_err().contains("new database"));
        args.resume_report = predecessor;
        args.scenario = Some("real-restart".into());
        assert!(validate_launch(&args).unwrap_err().contains("only valid"));
        args.scenario = Some("presentation".into());
        assert!(validate_launch(&args).is_err());
        remove_resume_fixture(&args);
    }

    #[test]
    fn resume_refuses_any_recorded_candidate_mutation_or_nonfailed_predecessor() {
        let (args, original) = resume_fixture();
        for mutation in 0..10 {
            let mut report = original.clone();
            match mutation {
                0 => report.outcome = "running".into(),
                1 => report.passed = true,
                2 => {
                    report.added_element =
                        Some(agq_studio_scene::fixtures::architecture().nodes[0].id)
                }
                3 => {
                    report.added_owner =
                        Some(agq_studio_scene::fixtures::architecture().nodes[0].id)
                }
                4 => report.committed = report.baseline.clone(),
                5 => report.last_state.candidate_revision = Some(ProjectRevisionId::new()),
                6 => report.last_state.mutation_pending = true,
                7 => {
                    report.assertions[0].name =
                        "prepare real source-backed candidate while current remains responsive"
                            .into()
                }
                8 => report.baseline.as_mut().unwrap().validation = ValidationState::Working,
                9 => report.assertions[0].before.candidate_phase = Some("Cancelled".into()),
                _ => unreachable!(),
            }
            assert!(
                validate_resume_report(&report).is_err(),
                "Unsafe predecessor case {mutation}"
            );
        }
        remove_resume_fixture(&args);
    }

    #[test]
    fn resume_requires_matching_database_project_branch_and_current_source_population() {
        let (mut args, original) = resume_fixture();
        for mismatch in 0..3 {
            let mut report = original.clone();
            match mismatch {
                0 => report.project = Some(ProjectId::new()),
                1 => report.branch = Some(BranchId::new()),
                2 => {
                    report.last_state.binding.as_mut().unwrap().revision = ProjectRevisionId::new()
                }
                _ => unreachable!(),
            }
            assert!(validate_resume_report(&report).is_err());
        }
        let database = args.database.take();
        args.database = Some(args.root.join("another.sqlite"));
        assert!(
            read_resume(&args)
                .unwrap_err()
                .contains("exact existing absolute database")
        );
        args.database = database;
        let sources = args.root.join("models/agentique");
        std::fs::write(sources.join("Added.sysml"), "part def Extra;").unwrap();
        assert!(
            read_resume(&args)
                .unwrap_err()
                .contains("source identities")
        );
        std::fs::remove_file(sources.join("Added.sysml")).unwrap();
        std::fs::write(sources.join("ResumeTest.sysml"), "part def Changed;").unwrap();
        assert!(
            read_resume(&args)
                .unwrap_err()
                .contains("source identities")
        );
        remove_resume_fixture(&args);
    }

    #[test]
    fn preparation_cannot_hide_a_frozen_ui_behind_few_observed_frames() {
        let started = Instant::now();
        let mut clock = PreparationClock {
            started,
            previous: started,
        };
        let mut evidence = PreparationResponsiveness::default();
        // A whole synchronous operation finishes before the next input hook.
        clock.observe(started + std::time::Duration::from_secs(5), &mut evidence);
        evidence.elapsed_ms = Some(5_000);
        assert_eq!(evidence.observed_input_hooks, 1);
        assert!(
            assess_preparation(&evidence, false)
                .unwrap_err()
                .contains("5000 ms")
        );
        assert!(
            assess_preparation(&evidence, true).is_err(),
            "A prior pan does not excuse a later freeze"
        );

        // Observed frame count never decides whether a long operation needs pan proof.
        evidence.maximum_input_hook_gap_ms = 20;
        assert!(
            assess_preparation(&evidence, false)
                .unwrap_err()
                .contains("without native panning")
        );
        assert!(assess_preparation(&evidence, true).is_ok());
        evidence.elapsed_ms = Some(100);
        assert!(
            assess_preparation(&evidence, false)
                .unwrap()
                .contains("not observed")
        );
    }

    #[test]
    fn background_read_gate_rejects_late_or_wrong_identity_even_after_successful_pan() {
        let mut projection = agq_studio_scene::fixtures::architecture();
        projection.nodes[0].source_available = true;
        let baseline = RevisionBinding {
            project: ProjectId::new(),
            revision: projection.revision_id,
        };
        let owner = projection.nodes[1].id;
        let evidence = BackgroundInspectionEvidence {
            binding: baseline,
            runtime_epoch: 3,
            element: projection.nodes[0].id,
            request: 42,
            selection_started_frame: 100,
            request_observed_frame: 110,
            response_observed_frame: 113,
            selection_to_response_ms: 220,
            request_observed_to_response_ms: 48,
            mutation_pending_at_response: true,
            inspector: ElementInspector {
                revision_id: projection.revision_id,
                element: projection.nodes[0].clone(),
                owner: None,
                effective_types: vec![],
                owned_features: vec![],
                effective_features: vec![],
                feature_provenance: Default::default(),
                specializations: vec![],
                subsettings: vec![],
                redefinitions: vec![],
                relationships: vec![],
                multiplicity: None,
                source: None,
                queries: vec![],
                profile: "Counterexample only; never real-runtime evidence".into(),
            },
        };
        assert!(assess_background_inspection(Some(&evidence), baseline, owner, 3).is_ok());
        assert!(assess_background_inspection(None, baseline, owner, 3).is_err());
        for invalid in 0..7 {
            let mut changed = evidence.clone();
            match invalid {
                0 => changed.mutation_pending_at_response = false,
                1 => changed.binding.revision = ProjectRevisionId::new(),
                2 => changed.runtime_epoch += 1,
                3 => changed.element = owner,
                4 => changed.inspector.element.id = owner,
                5 => changed.request = 0,
                6 => changed.response_observed_frame = changed.request_observed_frame - 1,
                _ => unreachable!(),
            }
            assert!(
                assess_background_inspection(Some(&changed), baseline, owner, 3).is_err(),
                "Accepted invalid concurrent Inspector evidence {invalid}"
            );
        }
    }

    #[test]
    fn created_identity_and_name_do_not_substitute_for_canonical_owner() {
        let mut projection = agq_studio_scene::fixtures::architecture();
        let owner = projection.nodes[0].id;
        let other_owner = projection.nodes[1].id;
        let part = &mut projection.nodes[2];
        let id = part.id;
        part.name = PART_NAME.into();
        part.semantic_kind = "PartUsage".into();
        part.owner = Some(owner);
        part.origin = ViewOrigin::Authored;
        part.source_available = true;
        assert!(assert_part_owner(&projection, id, owner).is_ok());
        projection.nodes[2].owner = Some(other_owner);
        assert!(assert_part_owner(&projection, id, owner).is_err());
        projection.nodes[2].owner = Some(owner);
        projection.nodes[2].origin = ViewOrigin::Derived;
        assert!(assert_part_owner(&projection, id, owner).is_err());
    }

    #[test]
    fn review_gate_tracks_existing_selection_then_explicit_part_through_real_app_modes() {
        // Exercise production presentation transitions on a labeled fixture;
        // this neither constructs nor accepts a semantic candidate.
        let mut args = isolated_args();
        args.fixture = Some("architecture".into());
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = StudioApp::new(&context, args).unwrap();
        let owner = named(&app, PLATFORM).unwrap();
        app.select(SceneTarget::Node(owner), false);
        app.open_part_edit(crate::commands::CommandId::CreatePart);
        assert!(app.part_edit_ready());
        let observed = app
            .scene
            .nodes
            .iter()
            .find(|node| node.id() != owner)
            .unwrap()
            .id();
        app.select(SceneTarget::Node(observed), false);
        app.new_part_name = PART_NAME.into();
        app.prepare_part();
        let revision = app.candidate.as_ref().unwrap().after.revision_id;
        let added = projection_named(&app.candidate.as_ref().unwrap().after, PART).unwrap();
        assert_eq!(app.selected_element(), Some(observed));
        let before = ReviewSelection::of(&app);
        app.change_comparison(ComparisonMode::Candidate);
        let after = ReviewSelection::of(&app);
        assert!(assert_candidate_review_selection(&before, &after, revision, added, None).is_ok());
        assert!(
            assert_candidate_review_selection(&before, &after, revision, added, Some(added))
                .is_err(),
            "The created part has not been explicitly selected yet"
        );
        app.select(SceneTarget::Node(added), false);
        app.change_comparison(ComparisonMode::Current);
        assert!(app.scene.node(added).is_none());
        assert_ne!(app.selected_element(), Some(added));
        let before_return = ReviewSelection::of(&app);
        app.change_comparison(ComparisonMode::Candidate);
        assert!(
            assert_candidate_review_selection(
                &before_return,
                &ReviewSelection::of(&app),
                revision,
                added,
                Some(added)
            )
            .is_ok()
        );
        let before_diff = ReviewSelection::of(&app);
        app.change_comparison(ComparisonMode::Diff);
        assert!(
            assert_candidate_review_selection(
                &before_diff,
                &ReviewSelection::of(&app),
                revision,
                added,
                Some(added)
            )
            .is_ok()
        );
        assert_eq!(app.focus, Some(owner));
    }

    #[test]
    fn review_gate_rejects_lost_substituted_stale_or_reordered_selection_and_focus() {
        let revision = ProjectRevisionId::new();
        let observed = ElementId::from_u128(501);
        let second = ElementId::from_u128(502);
        let added = ElementId::from_u128(503);
        let mut before = ReviewSelection {
            selection: Selection::new(revision),
            focus: Some(ElementId::from_u128(500)),
        };
        before.selection.select(SceneTarget::Node(observed), false);
        before.selection.select(SceneTarget::Port(second), true);
        assert!(assert_candidate_review_selection(&before, &before, revision, added, None).is_ok());
        for invalid in 0..5 {
            let mut after = before.clone();
            match invalid {
                0 => after.selection.clear(),
                1 => after.selection.select(SceneTarget::Node(added), false),
                2 => after.selection.revision = ProjectRevisionId::new(),
                3 => after.selection.primary = Some(SceneTarget::Node(observed)),
                4 => after.focus = None,
                _ => unreachable!(),
            }
            assert!(
                assert_candidate_review_selection(&before, &after, revision, added, None).is_err(),
                "Accepted initial review selection corruption {invalid}"
            );
        }
        let mut restored = before.clone();
        restored.selection.select(SceneTarget::Node(added), false);
        assert!(
            assert_candidate_review_selection(&before, &restored, revision, added, Some(added))
                .is_ok()
        );
        restored.selection.targets.clear();
        assert!(
            assert_candidate_review_selection(&before, &restored, revision, added, Some(added))
                .is_err()
        );
        assert!(
            assert_candidate_review_selection(&before, &before, revision, added, Some(observed))
                .is_err()
        );
    }

    #[test]
    fn restart_plan_navigates_before_enforcing_nested_identity_and_keeps_history_proof() {
        let plan = steps(true);
        assert_eq!(plan.len(), 5);
        assert!(matches!(plan[0].action, Action::OpenProject));
        assert!(matches!(plan[0].check, Check::RestartRevision));
        assert!(matches!(
            plan[1].action,
            Action::Select(Named {
                name: "ModelingPlatform",
                kind: "PartDefinition"
            })
        ));
        assert!(matches!(
            plan[1].check,
            Check::Selected(Named {
                name: "ModelingPlatform",
                kind: "PartDefinition"
            })
        ));
        assert!(matches!(plan[2].action, Action::Key(Key::F)));
        assert!(matches!(plan[2].check, Check::FocusedPlatform));
        assert!(matches!(
            plan[3].action,
            Action::Select(Named {
                name: PART_NAME,
                kind: "PartUsage"
            })
        ));
        assert!(matches!(plan[3].check, Check::Restart));
        assert!(matches!(plan[4].action, Action::Key(Key::Num4)));
        assert!(matches!(plan[4].check, Check::History));
        assert!(
            steps(false)
                .iter()
                .all(|step| !matches!(step.check, Check::RestartRevision | Check::Restart))
        );
    }

    #[test]
    fn restart_identity_gate_rejects_shallow_views_changed_ids_owners_and_stale_inspectors() {
        // Explicit DTO counterexamples qualify the gate, not real semantics.
        let mut projection = agq_studio_scene::fixtures::architecture();
        let owner = projection.nodes[0].id;
        let other = projection.nodes[1].id;
        let part = &mut projection.nodes[2];
        let id = part.id;
        part.name = PART_NAME.into();
        part.semantic_kind = "PartUsage".into();
        part.owner = Some(owner);
        part.origin = ViewOrigin::Authored;
        part.source_available = true;
        projection.view.kind = ViewKind::Architecture;
        projection.view.focus = Some(owner);
        let inspector = ElementInspector {
            revision_id: projection.revision_id,
            element: projection.nodes[2].clone(),
            owner: Some(agq_modeling_view::FeatureSummary {
                id: owner,
                name: "ModelingPlatform".into(),
                semantic_kind: "PartDefinition".into(),
            }),
            effective_types: vec![],
            owned_features: vec![],
            effective_features: vec![],
            feature_provenance: Default::default(),
            specializations: vec![],
            subsettings: vec![],
            redefinitions: vec![],
            relationships: vec![],
            multiplicity: None,
            source: None,
            queries: vec![],
            profile: "Counterexample only; never real-runtime evidence".into(),
        };
        assert!(
            assert_restart_element(&projection, Some(owner), Some(&inspector), id, owner).is_ok()
        );
        for invalid in 0..12 {
            let mut changed = projection.clone();
            let mut inspection = inspector.clone();
            let mut focus = Some(owner);
            match invalid {
                0 => changed.nodes.retain(|node| node.id != id),
                1 => focus = None,
                2 => changed.view.focus = None,
                3 => changed.nodes[2].id = other,
                4 => changed.nodes[2].owner = Some(other),
                5 => inspection.element.id = other,
                6 => inspection.element.owner = Some(other),
                7 => inspection.owner.as_mut().unwrap().id = other,
                8 => inspection.revision_id = ProjectRevisionId::new(),
                9 => inspection.element.revision_id = ProjectRevisionId::new(),
                10 => changed.nodes[2].origin = ViewOrigin::Derived,
                11 => changed.view.kind = ViewKind::SemanticGraph,
                _ => unreachable!(),
            }
            assert!(
                assert_restart_element(&changed, focus, Some(&inspection), id, owner).is_err(),
                "Accepted invalid restart identity case {invalid}"
            );
        }
        assert!(assert_restart_element(&projection, Some(owner), None, id, owner).is_err());
        assert!(
            assert_restart_element(&projection, Some(owner), Some(&inspector), other, owner)
                .is_err()
        );
        assert!(
            assert_restart_element(&projection, Some(owner), Some(&inspector), id, other).is_err()
        );
    }

    fn isolated_args() -> Args {
        let unique = format!(
            "agq-real-launch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        Args::try_parse_from([
            "agq-studio-native",
            "--scenario",
            "real",
            "--no-restore",
            "--database",
            base.with_extension("sqlite").to_str().unwrap(),
            "--scenario-report",
            base.with_extension("json").to_str().unwrap(),
            "--gallery",
            base.to_str().unwrap(),
        ])
        .unwrap()
    }

    #[test]
    fn real_launch_refuses_fixture_default_database_and_session_restoration() {
        let mut args = isolated_args();
        assert!(validate_launch(&args).is_ok());
        args.fixture = Some("architecture".into());
        assert!(validate_launch(&args).unwrap_err().contains("fixture"));
        args.fixture = None;
        args.no_restore = false;
        assert!(validate_launch(&args).is_err());
        args.no_restore = true;
        args.database = None;
        assert!(
            validate_launch(&args)
                .unwrap_err()
                .contains("isolated --database")
        );
    }

    #[test]
    fn real_launch_preserves_existing_database_bytes_before_any_worker_starts() {
        let args = isolated_args();
        let database = args.database.as_ref().unwrap();
        std::fs::write(database, b"existing repository sentinel").unwrap();
        let result = validate_launch(&args);
        let bytes = std::fs::read(database).unwrap();
        std::fs::remove_file(database).unwrap();
        assert!(result.unwrap_err().contains("new database"));
        assert_eq!(bytes, b"existing repository sentinel");
    }

    #[test]
    fn restart_cannot_proceed_from_missing_or_invalid_first_report() {
        let mut args = isolated_args();
        args.scenario = Some("real-restart".into());
        assert!(
            validate_launch(&args)
                .unwrap_err()
                .contains("restart-report")
        );
        let path = args
            .database
            .as_ref()
            .unwrap()
            .with_extension("previous.json");
        args.restart_report = Some(path.clone());
        std::fs::write(&path, br#"{"format":"fixture","passed":true}"#).unwrap();
        let result = validate_launch(&args);
        std::fs::remove_file(path).unwrap();
        assert!(result.unwrap_err().contains("Invalid first-process report"));
    }
}
