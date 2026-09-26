//! Longer real-project operator journey. Only ordinary native input is injected.
use super::*;

const RENAMED: &str = "alphaStudioObserverRenamed";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Evidence {
    pub elapsed_ms: u128,
    pub completed_cycles: usize,
    pub navigation_restorations: usize,
    pub cancelled_preparations: usize,
    pub validated_renames: usize,
    pub maximum_zoom_anchor_error_world: f32,
    pub binding_observations: usize,
    pub graph_reads_completed_during_preparation: usize,
}

#[derive(Clone, Default)]
pub(super) struct State {
    pub(super) started: Option<Instant>,
    saved: Option<crate::saved_views::SavedPresentation>,
    pan_origin: Option<Point>,
    zoom_anchor: Option<(Point, Point)>,
    preparing_frames: u64,
    cancellation_requested: bool,
    cancel_press: Option<u64>,
    background_graph: bool,
}

#[derive(Clone, Debug)]
pub(super) enum Action {
    Remember,
    PanZoom,
    StandardsOn,
    Head,
    PrepareRename { cancel: bool },
}
impl Action {
    pub(super) fn frames(&self) -> u64 {
        match self {
            Self::Remember => 1,
            Self::PanZoom => 8,
            Self::PrepareRename { .. } => 7,
            _ => 2,
        }
    }
}
#[derive(Clone, Debug)]
pub(super) enum Check {
    Remembered,
    Restored,
    PanZoom,
    Head,
    Diff,
    RenameDialog,
    CancelledPreparation,
    Renamed,
    ValidatedRename,
    CancelledRename,
}

pub(super) fn next_cycle(runner: &mut Runner, app: &StudioApp) -> Result<bool, String> {
    if app.args.soak_seconds == 0 {
        return Ok(false);
    }
    if let Some(started) = runner.soak.started {
        runner.report.soak.elapsed_ms = started.elapsed().as_millis();
        runner.report.soak.completed_cycles += 1;
        if started.elapsed().as_secs() >= app.args.soak_seconds {
            return Ok(false);
        }
    } else {
        runner.soak.started = Some(Instant::now());
    }
    require_head(runner, app)?;
    runner.steps = plan();
    // Keep the first cycle's uncurated captures without overwriting files later.
    if runner.report.soak.completed_cycles > 0 {
        for step in &mut runner.steps {
            step.capture = None;
        }
    }
    runner.index = 0;
    Ok(true)
}

fn plan() -> Vec<Step> {
    use super::{Action as A, Check as C};
    let step = |name, action, check| Step {
        name,
        action,
        check,
        capture: None,
    };
    let mut steps = vec![
        step(
            "soak returns to current System World",
            A::Palette("Home"),
            C::Home,
        ),
        step(
            "soak selects ModelingPlatform",
            A::Select(PLATFORM),
            C::Selected(PLATFORM),
        ),
        step(
            "soak focuses ModelingPlatform",
            A::Key(Key::F),
            C::FocusedPlatform,
        ),
        step(
            "soak exercises pan and ordered pointer zoom",
            A::Soak(Action::PanZoom),
            C::Soak(Check::PanZoom),
        ),
        step(
            "soak inspects ModelRepository",
            A::Select(REPOSITORY),
            C::RepositoryInterfaces,
        ),
        step(
            "soak remembers exploration state",
            A::Soak(Action::Remember),
            C::Soak(Check::Remembered),
        ),
        step(
            "soak opens Graph World",
            A::Key(Key::Num2),
            C::World(World::Graph),
        ),
        step(
            "soak Back restores focus camera selection and filters",
            A::Palette("Navigate back"),
            C::Soak(Check::Restored),
        ),
        step(
            "soak Forward restores Graph World",
            A::Palette("Navigate forward"),
            C::World(World::Graph),
        ),
        step(
            "soak requests deterministic dependencies",
            A::Palette("Show dependencies"),
            C::Dependencies,
        ),
        step(
            "soak includes adjacent standards",
            A::Soak(Action::StandardsOn),
            C::Standards,
        ),
        step(
            "soak selects derived relationship",
            A::DerivedEdge,
            C::DerivedEdge,
        ),
        step(
            "soak explains real derived evidence",
            A::Key(Key::E),
            C::Explanation,
        ),
        step(
            "soak dismisses Explain",
            A::Key(Key::Escape),
            C::ExplanationClosed,
        ),
        step(
            "soak opens real requirement obligations",
            A::Key(Key::Num3),
            C::World(World::Requirements),
        ),
        step("soak opens real history", A::Key(Key::Num4), C::History),
        step(
            "soak opens graph comparison context",
            A::Key(Key::Num2),
            C::World(World::Graph),
        ),
        step(
            "soak opens graph overview",
            A::Palette("Show loaded graph overview"),
            C::GraphOverview,
        ),
        step(
            "soak compares exact durable parent",
            A::Palette("Compare with parent"),
            C::Soak(Check::Diff),
        ),
        step(
            "soak returns through History",
            A::Key(Key::Num4),
            C::History,
        ),
        step(
            "soak selects durable head",
            A::Soak(Action::Head),
            C::Soak(Check::Head),
        ),
        step(
            "soak returns to architecture before editing",
            A::Palette("Home"),
            C::Home,
        ),
        step(
            "soak selects committed owner",
            A::Select(PLATFORM),
            C::Selected(PLATFORM),
        ),
        step(
            "soak focuses committed owner",
            A::Key(Key::F),
            C::FocusedPlatform,
        ),
        step("soak selects persisted part", A::Select(PART), C::Restart),
        step(
            "soak opens real Rename",
            A::Palette("Rename selected Part"),
            C::Soak(Check::RenameDialog),
        ),
        step(
            "soak cancels preparation while exploring current graph",
            A::Soak(Action::PrepareRename { cancel: true }),
            C::Soak(Check::CancelledPreparation),
        ),
        step(
            "soak restores architecture after cancelled work",
            A::Palette("Home"),
            C::Home,
        ),
        step(
            "soak reselects owner",
            A::Select(PLATFORM),
            C::Selected(PLATFORM),
        ),
        step("soak refocuses owner", A::Key(Key::F), C::FocusedPlatform),
        step(
            "soak reselects same canonical part",
            A::Select(PART),
            C::Restart,
        ),
        step(
            "soak opens Rename again",
            A::Palette("Rename selected Part"),
            C::Soak(Check::RenameDialog),
        ),
        step(
            "soak prepares identity-preserving Rename",
            A::Soak(Action::PrepareRename { cancel: false }),
            C::Soak(Check::Renamed),
        ),
        step(
            "soak validates Rename semantics",
            A::Palette("Validate candidate"),
            C::Soak(Check::ValidatedRename),
        ),
        step(
            "soak cancels validated Rename without moving head",
            A::Palette("Cancel candidate"),
            C::Soak(Check::CancelledRename),
        ),
        step(
            "soak confirms original committed part after cancellation",
            A::Select(PART),
            C::Restart,
        ),
    ];
    steps[14].capture = Some("13-soak-requirements");
    steps[18].capture = Some("14-soak-dense-parent-diff");
    steps[32].capture = Some("15-soak-rename-candidate");
    steps
}

pub(super) fn inject(
    runner: &mut Runner,
    action: &Action,
    frame: u64,
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
) -> Result<(), String> {
    match action {
        Action::Remember => runner.soak.saved = Some(app.capture_presentation()),
        Action::Head => {
            let revision = runner
                .report
                .committed
                .as_ref()
                .ok_or("Soak requires committed restart evidence")?
                .revision_id;
            click(
                input,
                automation::target(ctx, automation::Target::HistoryRevision(revision))?.center(),
                frame == 0,
            );
        }
        Action::StandardsOn => {
            if !app.include_standard {
                click(
                    input,
                    real_targets::target(ctx, Target::Standards)?.center(),
                    frame == 0,
                );
            }
        }
        Action::PanZoom => {
            let viewport = automation::target(ctx, automation::Target::Viewport)?;
            let start = background_pan_start(viewport)?;
            match frame {
                0 => {
                    runner.soak.pan_origin = Some(app.camera.center);
                    click(input, start, true);
                }
                1 | 2 => input.events.push(Event::PointerMoved(
                    start + Vec2::new(frame as f32 * 35.0, frame as f32 * 20.0),
                )),
                3 => click(input, start + Vec2::new(70.0, 40.0), false),
                4 | 6 => {
                    let pointer = viewport.min + viewport.size() * Vec2::new(0.7, 0.38);
                    if frame == 4 {
                        let local =
                            Point::new(pointer.x - viewport.left(), pointer.y - viewport.top());
                        runner.soak.zoom_anchor = Some((local, app.camera.screen_to_world(local)));
                    }
                    input.events.push(Event::PointerMoved(pointer));
                    input.events.push(Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: Vec2::new(0.0, if frame == 4 { 45.0 } else { -45.0 }),
                        modifiers: Modifiers::NONE,
                    });
                    input.events.push(Event::PointerMoved(viewport.center()));
                }
                _ => {}
            }
        }
        Action::PrepareRename { .. } => match frame {
            0 | 1 => {
                if frame == 0 {
                    runner.soak.preparing_frames = 0;
                    runner.soak.cancellation_requested = false;
                    runner.soak.cancel_press = None;
                    runner.soak.background_graph = false;
                }
                click(
                    input,
                    automation::target(ctx, automation::Target::CandidateName)?.center(),
                    frame == 0,
                );
            }
            2 => key(input, Key::A, Modifiers::COMMAND),
            3 => input.events.push(Event::Text(RENAMED.into())),
            5 | 6 => {
                if app.new_part_name != RENAMED {
                    return Err("Rename did not retain native text input".into());
                }
                click(
                    input,
                    automation::target(ctx, automation::Target::CandidatePrepare)?.center(),
                    frame == 5,
                );
            }
            _ => {}
        },
    }
    Ok(())
}

pub(super) fn background(
    runner: &mut Runner,
    action: &Action,
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
) -> Result<(), String> {
    if !matches!(action, Action::PrepareRename { cancel: true }) || app.preparation.is_none() {
        return Ok(());
    }
    require_head(runner, app)?;
    assert_bound(app)?;
    runner.report.soak.binding_observations += 1;
    runner.soak.preparing_frames += 1;
    let frame = runner.soak.preparing_frames;
    let start = input.events.len();
    if frame == 30 {
        key(input, Key::Num2, Modifiers::NONE);
    }
    if frame > 30
        && app.world == World::Graph
        && app.active_projection().view.kind == ViewKind::SemanticGraph
        && app.requested_definition.is_none()
        && app.bridge.mutation_pending()
        && !runner.soak.background_graph
    {
        runner.soak.background_graph = true;
        runner.report.soak.graph_reads_completed_during_preparation += 1;
    }
    if (60..=64).contains(&frame) {
        let viewport = automation::target(ctx, automation::Target::Viewport)?;
        let point = background_pan_start(viewport)?;
        match frame {
            60 => click(input, point, true),
            61 | 62 => input.events.push(Event::PointerMoved(
                point + Vec2::new((frame - 60) as f32 * 35.0, (frame - 60) as f32 * 20.0),
            )),
            63 => click(input, point + Vec2::new(70.0, 40.0), false),
            _ => {}
        }
    }
    if frame >= 120 && runner.soak.background_graph {
        let press = *runner.soak.cancel_press.get_or_insert(frame);
        if frame <= press + 1 {
            click(
                input,
                automation::target(ctx, automation::Target::CancelPreparation)?.center(),
                frame == press,
            );
        }
    }
    if app
        .preparation
        .as_ref()
        .is_some_and(|preparation| preparation.cancelled)
    {
        runner.soak.cancellation_requested = true;
    }
    runner.events.extend(
        input.events[start..]
            .iter()
            .map(|event| format!("Background soak input: {event:?}")),
    );
    Ok(())
}

fn require_head(runner: &Runner, app: &StudioApp) -> Result<(), String> {
    let expected = runner
        .report
        .committed
        .as_ref()
        .ok_or("Soak requires committed evidence")?;
    if current_manifest(app)? != expected
        || app.history.as_ref().is_none_or(|history| {
            !history.branches.iter().any(|branch| {
                Some(branch.id) == runner.report.branch && branch.head == expected.revision_id
            })
        })
    {
        return Err("Soak changed the durable head or selected the wrong revision".into());
    }
    Ok(())
}

pub(super) fn check(runner: &mut Runner, check: &Check, app: &StudioApp) -> Result<(), String> {
    require_head(runner, app)?;
    let require = |value, message: &str| {
        if value {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    };
    match check {
        Check::Remembered => require(runner.soak.saved.is_some(), "Exploration snapshot absent"),
        Check::Head => require(
            app.comparison == ComparisonMode::Current,
            "History did not leave comparison",
        ),
        Check::Restored => {
            let saved = runner
                .soak
                .saved
                .as_ref()
                .ok_or("Exploration snapshot absent")?;
            require(
                app.world == saved.world
                    && app.focus == saved.definition.focus
                    && app.camera.center.distance(saved.camera.center) < 0.01
                    && (app.camera.zoom - saved.camera.zoom).abs() < 0.001
                    && Some(&app.selection) == saved.selection.as_ref()
                    && app.families.iter().copied().collect::<Vec<_>>()
                        == saved.definition.relationship_families
                    && app.include_standard == saved.definition.include_standard_library,
                "Back lost world/focus/camera/selection/filters",
            )?;
            runner.report.soak.navigation_restorations += 1;
            Ok(())
        }
        Check::PanZoom => {
            let (pointer, before) = runner
                .soak
                .zoom_anchor
                .ok_or("Zoom anchor was not observed")?;
            let after = app.camera.screen_to_world(pointer);
            let error = before.distance(after);
            runner.report.soak.maximum_zoom_anchor_error_world = runner
                .report
                .soak
                .maximum_zoom_anchor_error_world
                .max(error);
            require(
                error <= 0.025
                    && runner
                        .soak
                        .pan_origin
                        .is_some_and(|before| before.distance(app.camera.center) > 1.0),
                "Pan or zoom anchor failed in real model",
            )
        }
        Check::Diff => require(
            app.comparison == ComparisonMode::Diff
                && app.compare_before.as_ref().map(|before| before.revision_id)
                    == current_manifest(app)?.parent_revision_id
                && app
                    .compare_before
                    .as_ref()
                    .is_some_and(|before| before.view == app.projection.view)
                && runner.report.added_element.is_some_and(|id| {
                    app.scene
                        .node(id)
                        .is_some_and(|node| node.diff == DiffMark::Added)
                }),
            "Real parent diff lost added part or exact revision/lens",
        ),
        Check::RenameDialog => require(
            app.create_dialog
                && app.part_edit_ready()
                && app.selected_element() == runner.report.added_element
                && app.new_part_name == PART_NAME,
            "Rename dialog did not bind the persisted part",
        ),
        Check::CancelledPreparation => {
            require(
                runner.soak.cancellation_requested
                    && app.preparation.is_none()
                    && app.candidate.is_none()
                    && app.comparison == ComparisonMode::Current,
                "In-flight cancellation did not discard candidate while preserving current",
            )?;
            runner.report.soak.cancelled_preparations += 1;
            Ok(())
        }
        Check::Renamed | Check::ValidatedRename => {
            let candidate = app.candidate.as_ref().ok_or("Rename candidate absent")?;
            let id = runner
                .report
                .added_element
                .ok_or("Committed part identity absent")?;
            require(
                candidate.id.is_some()
                    && candidate.before.revision_id == current_manifest(app)?.revision_id
                    && candidate.after.revision_id != candidate.before.revision_id
                    && candidate
                        .before
                        .nodes
                        .iter()
                        .any(|node| node.id == id && node.name == PART_NAME)
                    && candidate
                        .after
                        .nodes
                        .iter()
                        .any(|node| node.id == id && node.name == RENAMED)
                    && candidate.source.contains(RENAMED),
                "Rename changed identity or lost exact source/base binding",
            )?;
            if matches!(check, Check::ValidatedRename) {
                require(
                    candidate.phase == Some(CandidatePhase::Validated),
                    "Rename did not validate",
                )?;
                runner.report.soak.validated_renames += 1;
            } else {
                require(
                    candidate.phase == Some(CandidatePhase::Working),
                    "Rename is not Working",
                )?;
            }
            Ok(())
        }
        Check::CancelledRename => require(
            app.candidate.is_none()
                && app.comparison == ComparisonMode::Current
                && runner.report.added_element.is_some_and(|id| {
                    app.projection
                        .nodes
                        .iter()
                        .any(|node| node.id == id && node.name == PART_NAME)
                }),
            "Cancelled Rename changed the durable design",
        ),
    }
}
