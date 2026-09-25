//! Native input qualification for disposable local views and process restoration.
//! No application mutation, service writes, or manufactured semantic success.
use crate::{
    Args,
    app::StudioApp,
    automation::{self, ScenarioStatus},
    navigation::World,
    saved_views::{SavedPresentation, SavedView},
    session::Session,
};
use agq_modeling_repository::ContentDigest;
use eframe::egui::{self, Event, Key, Modifiers, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Instant};

const FORMAT: &str = "agentique-native-presentation/1";
const ORIGINAL: &str = "Architecture overview";
const RENAMED: &str = "Platform relationships";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Menu,
    Name,
    Save,
    Rename,
    Update,
    Select(u64),
    Open(u64),
}

#[derive(Clone)]
struct Recorded {
    rect: Rect,
    frame: u64,
}
pub fn record(ctx: &egui::Context, target: Target, rect: Rect) {
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new((FORMAT, target)), Recorded { rect, frame })
    });
}
fn target(ctx: &egui::Context, target: Target) -> Result<Rect, String> {
    ctx.data(|data| data.get_temp::<Recorded>(egui::Id::new((FORMAT, target))))
        .filter(|item| {
            item.rect.is_finite()
                && item.rect.is_positive()
                && ctx.cumulative_frame_nr().saturating_sub(item.frame) <= 2
                && ctx.viewport_rect().contains_rect(item.rect)
        })
        .map(|item| item.rect)
        .ok_or_else(|| format!("Current visible {target:?} widget is unavailable"))
}

pub fn is_scenario(scenario: Option<&str>) -> bool {
    matches!(scenario, Some("presentation" | "presentation-restart"))
}

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Before any worker starts, protect both operator files and previous evidence.
pub fn validate_launch(args: &Args) -> Result<(), String> {
    if !is_scenario(args.scenario.as_deref()) {
        return Ok(());
    }
    let database = args
        .database
        .as_ref()
        .ok_or("Presentation qualification requires an explicit isolated --database")?;
    let session = database.with_extension("native-session.json");
    let views = database.with_extension("native-views.json");
    if !database.is_absolute()
        || args.frames.is_some()
        || args.screenshot.is_some()
        || args.scenario_timeout_seconds == 0
    {
        return Err(
            "Use an absolute isolated database, a nonzero timeout, and no --frames/--screenshot"
                .into(),
        );
    }
    let gallery = args
        .gallery
        .as_ref()
        .ok_or("Presentation qualification requires a fresh --gallery")?;
    if exists(database)
        || exists(&args.scenario_report)
        || exists(gallery)
        || [&session, &views, database].contains(&&args.scenario_report)
        || [&session, &views, database].contains(&gallery)
        || gallery == &args.scenario_report
        || exists(&database.with_extension("views.sqlite"))
        || ["-wal", "-shm"].iter().any(|suffix| {
            let mut path = database.as_os_str().to_os_string();
            path.push(suffix);
            exists(Path::new(&path))
        })
    {
        return Err("Choose unused database, report and gallery paths; prior evidence and model files are preserved".into());
    }
    if args.scenario.as_deref() == Some("presentation") {
        if args.fixture.as_deref() != Some("architecture")
            || !args.no_restore
            || args.restart_report.is_some()
            || exists(&session)
            || exists(&views)
        {
            return Err("First presentation run requires --fixture architecture --no-restore and no existing presentation sidecars/restart report".into());
        }
    } else {
        if args.fixture.is_some() || args.no_restore {
            return Err(
                "Presentation restart requires normal startup: no --fixture and no --no-restore"
                    .into(),
            );
        }
        let report = prior_report(args)?;
        if Path::new(&report.database) != database
            || report.session_digest.as_ref() != Some(&digest(&session)?)
            || report.views_digest.as_ref() != Some(&digest(&views)?)
        {
            return Err("Restart must read the exact isolated paths and presentation bytes from the successful first report".into());
        }
        let restored = Session::load(&session).ok_or("Saved session is invalid")?;
        if restored.project.is_some() || restored.fixture.as_deref() != Some("architecture") {
            return Err(
                "Presentation restart refuses every live project and unknown fixture".into(),
            );
        }
    }
    Ok(())
}

fn digest(path: &Path) -> Result<ContentDigest, String> {
    crate::session::read_presentation(path)
        .map(|bytes| ContentDigest::of(&bytes))
        .map_err(|e| e.to_string())
}
fn prior_report(args: &Args) -> Result<Report, String> {
    let path = args
        .restart_report
        .as_ref()
        .ok_or("Presentation restart requires --restart-report")?;
    let report: Report = serde_json::from_slice(
        &crate::session::read_presentation(path).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if report.format != FORMAT
        || report.scenario != "presentation"
        || !report.passed
        || report.final_state.is_none()
        || report.session_digest.is_none()
        || report.views_digest.is_none()
    {
        return Err(
            "Restart requires a successful complete first-process presentation report".into(),
        );
    }
    Ok(report)
}

#[derive(Clone, Serialize, Deserialize)]
struct State {
    revision: String,
    selection: Vec<String>,
    fixture: Option<String>,
    presentation: SavedPresentation,
    dark: bool,
    contrast: bool,
    reduced_motion: bool,
}
impl State {
    fn of(app: &StudioApp) -> Self {
        Self {
            revision: app.projection.revision_id.to_string(),
            selection: app
                .selection
                .targets
                .iter()
                .map(|target| format!("{target:?}"))
                .collect(),
            fixture: app.fixture.clone(),
            presentation: app.capture_presentation(),
            dark: app.theme.dark,
            contrast: app.theme.contrast,
            reduced_motion: app.reduced_motion,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Assertion {
    name: String,
    passed: bool,
    before: State,
    after: State,
    inputs: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Report {
    format: String,
    scenario: String,
    scope: String,
    database: String,
    adapter: String,
    passed: bool,
    failure: Option<String>,
    assertions: Vec<Assertion>,
    gallery: Vec<String>,
    final_state: Option<State>,
    session_digest: Option<ContentDigest>,
    views_digest: Option<ContentDigest>,
}
#[derive(Clone, Copy)]
enum Action {
    Idle,
    Click(Target),
    Edit(&'static str),
    Key(Key),
    FocusPlatform,
    Pan,
    OpenSaved,
}
#[derive(Clone, Copy)]
enum Check {
    Ready,
    Menu,
    Name(&'static str),
    Saved,
    Renamed,
    Closed,
    ClosedPreservingSelection,
    Focused,
    Graph,
    Panned,
    Updated,
    System,
    Opened,
    Session,
    Restart,
}
#[derive(Clone, Copy)]
struct Step {
    name: &'static str,
    action: Action,
    check: Check,
}
fn steps(restart: bool) -> Vec<Step> {
    if restart {
        return vec![Step {
            name: "separate process restores exact saved presentation",
            action: Action::Idle,
            check: Check::Restart,
        }];
    }
    vec![
        Step {
            name: "isolated fixture ready",
            action: Action::Idle,
            check: Check::Ready,
        },
        Step {
            name: "open Local views",
            action: Action::Click(Target::Menu),
            check: Check::Menu,
        },
        Step {
            name: "enter view name",
            action: Action::Edit(ORIGINAL),
            check: Check::Name(ORIGINAL),
        },
        Step {
            name: "save current view through menu",
            action: Action::Click(Target::Save),
            check: Check::Saved,
        },
        Step {
            name: "enter replacement view name",
            action: Action::Edit(RENAMED),
            check: Check::Name(RENAMED),
        },
        Step {
            name: "rename selected view without altering presentation",
            action: Action::Click(Target::Rename),
            check: Check::Renamed,
        },
        Step {
            name: "Escape dismisses views menu and preserves canvas selection",
            action: Action::Key(Key::Escape),
            check: Check::ClosedPreservingSelection,
        },
        Step {
            name: "focus ModelingPlatform with native double-click",
            action: Action::FocusPlatform,
            check: Check::Focused,
        },
        Step {
            name: "switch focused view to Graph",
            action: Action::Key(Key::Num2),
            check: Check::Graph,
        },
        Step {
            name: "pan graph before updating bookmark",
            action: Action::Pan,
            check: Check::Panned,
        },
        Step {
            name: "reopen Local views",
            action: Action::Click(Target::Menu),
            check: Check::Menu,
        },
        Step {
            name: "update selected view from current presentation",
            action: Action::Click(Target::Update),
            check: Check::Updated,
        },
        Step {
            name: "dismiss updated view menu",
            action: Action::Click(Target::Menu),
            check: Check::Closed,
        },
        Step {
            name: "leave bookmarked world",
            action: Action::Key(Key::Num1),
            check: Check::System,
        },
        Step {
            name: "open saved view menu",
            action: Action::Click(Target::Menu),
            check: Check::Menu,
        },
        Step {
            name: "open exact updated view through its row",
            action: Action::OpenSaved,
            check: Check::Opened,
        },
        Step {
            name: "ordinary autosave retains reopened presentation",
            action: Action::Idle,
            check: Check::Session,
        },
    ]
}

#[derive(Clone)]
struct Runner {
    report: Report,
    prior: Option<Report>,
    steps: Vec<Step>,
    index: usize,
    age: u64,
    frame: u64,
    started: Instant,
    origin: Option<f64>,
    before: Option<State>,
    inputs: Vec<String>,
    point: Option<Pos2>,
    saved: Option<SavedView>,
    capture: Option<u64>,
    dirty: bool,
}
impl Runner {
    fn new(app: &StudioApp) -> Result<Self, String> {
        let restart = app.args.scenario.as_deref() == Some("presentation-restart");
        Ok(Self {
            report: Report { format: FORMAT.into(), scenario: app.args.scenario.clone().unwrap_or_default(),
                scope:"Native egui input over explicit architecture fixture. Presentation persistence only; no runtime authentication, semantic validation or durable model acceptance.".into(),
                database:app.config.database.display().to_string(),adapter:app.adapter.clone(),passed:false,failure:None,assertions:vec![],gallery:vec![],final_state:None,session_digest:None,views_digest:None },
            prior: if restart {Some(prior_report(&app.args)?)} else {None}, steps:steps(restart),index:0,age:0,frame:0,
            started:Instant::now(),origin:None,before:None,inputs:vec![],point:None,saved:None,capture:None,dirty:true,
        })
    }
    fn advance(
        &mut self,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<ScenarioStatus, String> {
        if app.fixture.as_deref() != Some("architecture")
            || app.binding.is_some()
            || app.branch.is_some()
            || app.candidate.is_some()
        {
            return Err("Presentation driver refuses non-architecture fixtures, live bindings, branches and candidates".into());
        }
        if self.started.elapsed().as_secs() > app.args.scenario_timeout_seconds.min(180) {
            return Err("Presentation qualification exceeded its wall-time deadline".into());
        }
        self.frame += 1;
        input
            .events
            .retain(|event| matches!(event, Event::Screenshot { .. } | Event::WindowFocused(_)));
        input.focused = true;
        input.modifiers = Modifiers::NONE;
        let origin = *self.origin.get_or_insert(input.time.unwrap_or(0.0));
        input.time = Some(origin + self.frame as f64 / 60.0);
        input.predicted_dt = 1.0 / 60.0;
        if let Some(age) = &mut self.capture {
            *age += 1;
            if let Some(image) = input.events.iter().find_map(|event| {
                if let Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            }) {
                let directory = app.args.gallery.as_ref().ok_or("Gallery path missing")?;
                std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
                let suffix = if self.report.failure.is_some() {
                    "-failed"
                } else {
                    ""
                };
                let path = directory.join(format!("{}{suffix}.png", self.report.scenario));
                let bytes: Vec<_> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                image::save_buffer(
                    &path,
                    &bytes,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                )
                .map_err(|e| e.to_string())?;
                let evidence = serde_json::json!({"scope":self.report.scope,"state":State::of(app),"failure":self.report.failure,"view_name":crate::presentation::local_view_name(ctx),"image_sha256":ContentDigest::of(&std::fs::read(&path).map_err(|e|e.to_string())?),"adapter":app.adapter});
                std::fs::write(
                    path.with_extension("json"),
                    serde_json::to_vec_pretty(&evidence).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                self.report.gallery.push(path.display().to_string());
                self.dirty = true;
                self.capture = None;
                return if let Some(failure) = &self.report.failure {
                    Err(failure.clone())
                } else {
                    Ok(ScenarioStatus::Complete)
                };
            }
            if *age > 120 {
                return Err("Native presentation screenshot did not arrive".into());
            }
            return Ok(ScenarioStatus::Running);
        }
        let step = self.steps[self.index];
        if self.age == 0 {
            self.before = Some(State::of(app));
            self.inputs.clear();
            self.point = None;
        }
        // Separate ordinary clicks so egui cannot interpret save/rename as a double-click.
        if (40..52).contains(&self.age) {
            let start = input.events.len();
            self.inject(step.action, self.age - 40, app, ctx, input)?;
            self.inputs.extend(
                input.events[start..]
                    .iter()
                    .map(|event| format!("frame {}: {event:?}", app.frame_number)),
            );
        }
        if self.age >= 92 {
            let passed = self.check(step.check, app, ctx)?;
            let timeout = if matches!(step.check, Check::Session) {
                1800
            } else {
                300
            };
            if passed || self.age > timeout {
                self.report.assertions.push(Assertion {
                    name: step.name.into(),
                    passed,
                    before: self.before.clone().expect("step snapshot"),
                    after: State::of(app),
                    inputs: self.inputs.clone(),
                });
                self.dirty = true;
                if !passed {
                    self.report.failure = Some(format!(
                        "Presentation assertion failed: {}; status={}",
                        step.name, app.status
                    ));
                    self.capture = Some(0);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                        egui::UserData::default(),
                    ));
                    return Ok(ScenarioStatus::Running);
                }
                self.index += 1;
                self.age = 0;
                if self.index == self.steps.len() {
                    self.report.final_state = Some(State::of(app));
                    self.report.session_digest = Some(digest(&app.session_path)?);
                    self.report.views_digest = Some(digest(
                        &app.config.database.with_extension("native-views.json"),
                    )?);
                    self.capture = Some(0);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                        egui::UserData::default(),
                    ));
                }
                return Ok(ScenarioStatus::Running);
            }
        }
        self.age += 1;
        Ok(ScenarioStatus::Running)
    }
    fn inject(
        &mut self,
        action: Action,
        frame: u64,
        app: &StudioApp,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
    ) -> Result<(), String> {
        let mut click_target = |target_key| -> Result<(), String> {
            if frame < 2 {
                let point = if let Some(point) = self.point {
                    point
                } else {
                    let point = target(ctx, target_key)?.center();
                    self.point = Some(point);
                    point
                };
                automation::click(input, point, frame == 0, Modifiers::NONE);
            }
            Ok(())
        };
        match action {
            Action::Idle => {}
            Action::Click(key) => click_target(key)?,
            Action::OpenSaved => click_target(Target::Open(
                self.saved.as_ref().ok_or("No saved view identity")?.id,
            ))?,
            Action::Edit(text) => match frame {
                0 | 1 => click_target(Target::Name)?,
                3 => automation::key(input, Key::A, Modifiers::COMMAND),
                4 => input.events.push(Event::Text(text.into())),
                _ => {}
            },
            Action::Key(key) if frame == 0 => automation::key(input, key, Modifiers::NONE),
            Action::Key(_) => {}
            Action::FocusPlatform if frame < 4 => {
                let point = if let Some(point) = self.point {
                    point
                } else {
                    let node = app
                        .scene
                        .node(agq_studio_scene::fixtures::id(2))
                        .ok_or("ModelingPlatform fixture node missing")?;
                    let viewport = automation::target(ctx, automation::Target::Viewport)?;
                    let local = app.camera.world_to_screen(agq_studio_scene::Point::new(
                        node.bounds.center().x,
                        node.bounds.min.y + 26.0,
                    ));
                    let point = viewport.min + Vec2::new(local.x, local.y);
                    if !viewport.contains(point) {
                        return Err("ModelingPlatform header is outside viewport".into());
                    }
                    self.point = Some(point);
                    point
                };
                automation::click(input, point, frame % 2 == 0, Modifiers::NONE);
            }
            Action::FocusPlatform => {}
            Action::Pan if frame < 4 => {
                let point = *self.point.get_or_insert(
                    automation::target(ctx, automation::Target::Viewport)?.left_top()
                        + Vec2::new(22.0, 22.0),
                );
                match frame {
                    0 => automation::click(input, point, true, Modifiers::NONE),
                    1 | 2 => input.events.push(Event::PointerMoved(
                        point + Vec2::new(frame as f32 * 40.0, frame as f32 * 24.0),
                    )),
                    _ => automation::click(
                        input,
                        point + Vec2::new(80.0, 48.0),
                        false,
                        Modifiers::NONE,
                    ),
                }
            }
            Action::Pan => {}
        }
        Ok(())
    }
    fn check(
        &mut self,
        check: Check,
        app: &StudioApp,
        ctx: &egui::Context,
    ) -> Result<bool, String> {
        if !app.ready
            || app.scene_builder.busy
            || app.camera_target.is_some()
            || app.fit_pending
            || app.scene.revision_id != app.projection.revision_id
        {
            return Ok(false);
        }
        let before = self.before.as_ref().expect("step snapshot");
        let views = if matches!(
            check,
            Check::Saved | Check::Renamed | Check::Updated | Check::Opened
        ) {
            app.list_local_views()?
        } else {
            vec![]
        };
        Ok(match check {
            Check::Ready => app.world == World::System && !app.scene.nodes.is_empty(),
            Check::Menu => target(ctx, Target::Name).is_ok(),
            Check::Name(expected) => {
                target(ctx, Target::Name).is_ok()
                    && ctx.wants_keyboard_input()
                    && crate::presentation::local_view_name(ctx).as_deref() == Some(expected)
            }
            Check::Closed => target(ctx, Target::Name).is_err(),
            Check::ClosedPreservingSelection => {
                target(ctx, Target::Name).is_err()
                    && State::of(app).selection == before.selection
                    && !before.selection.is_empty()
            }
            Check::Saved => {
                if let [saved] = views.as_slice() {
                    let valid = saved.name == ORIGINAL
                        && saved.binding.project.is_none()
                        && saved.binding.revision == app.projection.revision_id
                        && presentation_equal(&saved.presentation, &before.presentation);
                    if valid {
                        self.saved = Some(saved.clone());
                    }
                    valid
                } else {
                    false
                }
            }
            Check::Renamed => {
                if let (Some(previous), [saved]) = (&self.saved, views.as_slice()) {
                    let valid = saved.id == previous.id
                        && saved.name == RENAMED
                        && saved.binding == previous.binding
                        && presentation_equal(&saved.presentation, &previous.presentation);
                    if valid {
                        self.saved = Some(saved.clone());
                    }
                    valid
                } else {
                    false
                }
            }
            Check::Focused => {
                app.world == World::System && app.focus == Some(agq_studio_scene::fixtures::id(2))
            }
            Check::Graph => {
                app.world == World::Graph
                    && app.focus.is_none()
                    && app
                        .expanded
                        .as_ref()
                        .is_some_and(|ids| ids.contains(&agq_studio_scene::fixtures::id(2)))
            }
            Check::Panned => {
                app.world == World::Graph && app.camera.center != before.presentation.camera.center
            }
            Check::Updated => {
                if let (Some(previous), [saved]) = (&self.saved, views.as_slice()) {
                    let valid = saved.id == previous.id
                        && saved.name == RENAMED
                        && saved.binding == previous.binding
                        && saved.presentation.world == World::Graph
                        && presentation_equal(&saved.presentation, &before.presentation)
                        && !presentation_equal(&saved.presentation, &previous.presentation);
                    if valid {
                        self.saved = Some(saved.clone());
                    }
                    valid
                } else {
                    false
                }
            }
            Check::System => app.world == World::System,
            Check::Opened => self.saved.as_ref().is_some_and(|saved| {
                views.len() == 1
                    && presentation_equal(&app.capture_presentation(), &saved.presentation)
                    && app.projection.revision_id == saved.binding.revision
                    && app.status.starts_with("Opened local view")
            }),
            Check::Session => Session::load(&app.session_path).is_some_and(|session| {
                session.project.is_none()
                    && session.fixture.as_deref() == Some("architecture")
                    && session.revision == app.projection.revision_id
                    && session
                        .presentation
                        .as_ref()
                        .is_some_and(|saved| presentation_equal(saved, &app.capture_presentation()))
            }),
            Check::Restart => {
                let previous = self.prior.as_ref().ok_or("Missing restart report")?;
                let expected = previous
                    .final_state
                    .as_ref()
                    .ok_or("Missing first-process final state")?;
                let actual = State::of(app);
                actual.revision == expected.revision
                    && actual.fixture == expected.fixture
                    && actual.dark == expected.dark
                    && actual.contrast == expected.contrast
                    && actual.reduced_motion == expected.reduced_motion
                    && presentation_equal(&actual.presentation, &expected.presentation)
                    && Some(digest(&app.session_path)?) == previous.session_digest
                    && Some(digest(
                        &app.config.database.with_extension("native-views.json"),
                    )?) == previous.views_digest
            }
        })
    }
}

/// Viewport size is monitor/window state, not a saved camera authority.
fn presentation_equal(left: &SavedPresentation, right: &SavedPresentation) -> bool {
    let mut left = left.clone();
    left.camera.viewport = right.camera.viewport;
    serde_json::to_value(left).ok() == serde_json::to_value(right).ok()
}

pub fn drive(
    app: &StudioApp,
    ctx: &egui::Context,
    input: &mut egui::RawInput,
) -> Result<ScenarioStatus, String> {
    let id = egui::Id::new(FORMAT);
    let mut runner = match ctx.data(|data| data.get_temp::<Runner>(id)) {
        Some(runner) => runner,
        None => Runner::new(app)?,
    };
    let result = runner.advance(app, ctx, input);
    runner.report.passed = matches!(result, Ok(ScenarioStatus::Complete));
    if let Err(error) = &result {
        runner.report.failure = Some(error.clone());
    }
    if runner.dirty || !matches!(result, Ok(ScenarioStatus::Running)) {
        let bytes = serde_json::to_vec_pretty(&runner.report).map_err(|e| e.to_string())?;
        crate::session::write_presentation(&app.args.scenario_report, &bytes)
            .map_err(|e| e.to_string())?;
        runner.dirty = false;
    }
    ctx.data_mut(|data| data.insert_temp(id, runner));
    ctx.request_repaint();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn isolated_args() -> Args {
        let mut args = Args::parse_from(["studio"]);
        let prefix = std::env::temp_dir().join(format!(
            "agentique-presentation-{}",
            agq_modeling_repository::ProjectRevisionId::new()
        ));
        args.database = Some(prefix.with_extension("sqlite"));
        args.scenario_report = prefix.with_extension("report.json");
        args.gallery = Some(prefix.with_extension("gallery"));
        args.scenario = Some("presentation".into());
        args.fixture = Some("architecture".into());
        args.no_restore = true;
        args
    }

    #[test]
    fn presentation_launch_requires_fresh_explicit_isolation_before_autosave() {
        let args = isolated_args();
        validate_launch(&args).unwrap();
        let mut invalid = args.clone();
        invalid.database = None;
        assert!(validate_launch(&invalid).is_err());
        invalid = args.clone();
        invalid.no_restore = false;
        assert!(validate_launch(&invalid).is_err());
        invalid = args.clone();
        invalid.scenario_report = args
            .database
            .as_ref()
            .unwrap()
            .with_extension("native-session.json");
        assert!(validate_launch(&invalid).is_err());
        let views = args
            .database
            .as_ref()
            .unwrap()
            .with_extension("native-views.json");
        std::fs::write(&views, b"existing operator state").unwrap();
        assert!(validate_launch(&args).is_err());
        assert_eq!(std::fs::read(&views).unwrap(), b"existing operator state");
        std::fs::remove_file(&views).unwrap();
    }

    #[test]
    fn restart_cannot_be_requested_as_a_fresh_fixture_launch() {
        let mut args = isolated_args();
        args.scenario = Some("presentation-restart".into());
        assert!(validate_launch(&args).is_err());
        args.fixture = None;
        args.no_restore = false;
        assert!(validate_launch(&args).is_err());
        args.scenario = Some("vertical".into());
        assert!(!is_scenario(args.scenario.as_deref()));
    }
}
