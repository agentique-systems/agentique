//! The Studio's window: one workspace around the Surface (§3.2 Shell). A
//! title bar with the project, the views, the command search and the
//! panels' switches; the Outline docked left; the Inspector column and the
//! Conversation docked right, each resizable and collapsible, remembered per
//! project; focus mode hides them all; a status bar; and the overlays
//! (dialogs, the palette, the context menu). The Studio's state lives in one
//! entity; each view is drawn again only for what it shows (`Dirty`).
use crate::{
    commands::{self, CommandId, Run},
    conversation_view::ConversationView,
    dialogs::DialogsView,
    navigation::SurfaceView as View,
    palette::Palette,
    panels::{InspectorColumn, LeftColumn},
    session::Widths,
    settings_view::SettingsView,
    studio::{Dirty, Studio, StudioEvent, System},
    surface::{SurfaceEvent, SurfaceView},
    ui::{self, ActiveTheme, Button, IconName, Menu, MenuItem, Segmented, Tone, icon, r, theme},
    welcome::Welcome,
};
use gpui::{
    AnimationExt, AnyView, App, AppContext, ClickEvent, Context, DismissEvent, DragMoveEvent,
    Empty, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, SharedString, StatefulInteractiveElement, Styled, Subscription,
    Task, Window, WindowAppearance, WindowControlArea, anchored, deferred, div, point,
    prelude::FluentBuilder, px,
};
use std::time::Duration;

impl EventEmitter<StudioEvent> for Studio {}

/// Updating the Studio from a view: the update, then telling the views
/// what changed.
pub trait StudioExt {
    fn act<R>(&self, cx: &mut App, update: impl FnOnce(&mut Studio) -> R) -> R;
}

impl StudioExt for Entity<Studio> {
    fn act<R>(&self, cx: &mut App, update: impl FnOnce(&mut Studio) -> R) -> R {
        self.update(cx, |studio, cx| {
            let result = update(studio);
            let dirty = studio.take_dirty();
            cx.emit(StudioEvent(dirty));
            result
        })
    }
}

pub fn bind(_: &mut App) {}

/// A column docked beside the Surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dock {
    Outline,
    Inspector,
    Conversation,
}

/// A splitter being dragged.
#[derive(Clone)]
struct Resize(Dock);

pub struct Workspace {
    studio: Entity<Studio>,
    focus: FocusHandle,
    surface: Entity<SurfaceView>,
    outline: Entity<LeftColumn>,
    inspector: Entity<InspectorColumn>,
    conversation: Entity<ConversationView>,
    settings: Entity<SettingsView>,
    dialogs: Entity<DialogsView>,
    welcome: Entity<Welcome>,
    gallery: Option<Entity<crate::gallery::Gallery>>,
    palette: Option<(Entity<Palette>, Subscription)>,
    context_menu: Option<(gpui::Point<Pixels>, Entity<Menu>, Subscription)>,
    /// Polls the Assistant's turn and saves the session now and then.
    _ticker: Task<()>,
    _subscriptions: Vec<Subscription>,
    applied: Option<(bool, bool, bool, f32)>,
    /// Whether an overlay was open and the Surface shown, when last seen.
    overlay: Option<(bool, bool)>,
    /// The right column's panel last drawn: when another replaces it, a
    /// field that had the keyboard is gone, and the keyboard goes back to
    /// the middle.
    panel: Option<crate::studio::Panel>,
    was_working: bool,
    /// Ticks since the window opened: the frame count of screens without
    /// the Surface.
    ticks: u64,
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Workspace {
    pub fn new(args: crate::Args, window: &mut Window, cx: &mut Context<Workspace>) -> Workspace {
        let system = System {
            dark: is_dark(window.appearance()),
            reduced_motion: cx.reduce_motion(),
        };
        let gallery = args.fixture.as_deref() == Some("components");
        let studio = cx.new(|_| {
            let mut studio = Studio::new(args, system);
            studio.open_control_endpoint();
            studio
        });
        let surface = cx.new(|cx| SurfaceView::new(studio.clone(), cx));
        let outline = cx.new(|cx| LeftColumn::new(studio.clone(), window, cx));
        // One start form for objectives, in the Conversation or the
        // Objectives panel (C-54).
        let form =
            cx.new(|cx| crate::objective_form::ObjectiveForm::new(studio.clone(), window, cx));
        let inspector = cx.new(|cx| InspectorColumn::new(studio.clone(), form.clone(), window, cx));
        let conversation =
            cx.new(|cx| ConversationView::new(studio.clone(), form.clone(), window, cx));
        let settings = cx.new(|cx| SettingsView::new(studio.clone(), window, cx));
        let dialogs = cx.new(|cx| DialogsView::new(studio.clone(), cx));
        let welcome = cx.new(|cx| Welcome::new(studio.clone(), cx));
        let gallery =
            gallery.then(|| cx.new(|cx| crate::gallery::Gallery::new(studio.clone(), window, cx)));
        let mut subscriptions = vec![
            cx.subscribe_in(
                &studio,
                window,
                |this, _, event: &StudioEvent, window, cx| this.studio_changed(event.0, window, cx),
            ),
            cx.subscribe_in(
                &surface,
                window,
                |this, _, event: &SurfaceEvent, window, cx| match event {
                    SurfaceEvent::ContextMenu(position) => {
                        this.open_context_menu(*position, window, cx)
                    }
                },
            ),
            cx.observe_window_appearance(window, |this, window, cx| {
                let dark = is_dark(window.appearance());
                let reduced = this.studio.read(cx).system.reduced_motion;
                this.studio.act(cx, |studio| {
                    studio.follow_system(System {
                        dark,
                        reduced_motion: reduced,
                    })
                });
            }),
        ];
        subscriptions.push(cx.on_release(|this: &mut Workspace, cx| {
            this.studio.update(cx, |studio, _| {
                // An objective stays where it was, to continue (C-53).
                studio.interrupt_objective(Duration::from_secs(10));
                studio.end_turn();
                studio.save_session();
                studio.control.close_endpoint();
            });
        }));
        // Weak: the ticker must not keep the Studio alive.
        let controlled = studio.downgrade();
        let ticker = cx.spawn_in(window, async move |this, cx| {
            // Agents were busy at the last tick: one more frame takes what
            // they left on screen (the chip, marks) away.
            let mut was_busy = false;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let Ok(()) = this.update_in(cx, |this, window, cx| this.tick(window, cx)) else {
                    break;
                };
                // Agents' observations and actions (C-53), outside any
                // view's update: their input goes through the window as the
                // platform's does, and its handlers update the views.
                let Some(controlled) = controlled.upgrade() else {
                    break;
                };
                let _ = cx.update(|window, cx| {
                    let changed = crate::control::tick(&controlled, window, cx);
                    let busy = controlled.read(cx).control.busy();
                    let was = std::mem::replace(&mut was_busy, busy);
                    if changed || busy || was {
                        controlled.act(cx, |studio| {
                            studio.control.live_marks();
                            studio.control.live_effects();
                            studio.mark(Dirty::STATUS | Dirty::OVERLAY)
                        });
                    }
                });
            }
        });
        let mut workspace = Workspace {
            studio,
            focus: cx.focus_handle(),
            surface,
            outline,
            inspector,
            conversation,
            settings,
            dialogs,
            welcome,
            gallery,
            palette: None,
            context_menu: None,
            _ticker: ticker,
            _subscriptions: subscriptions,
            applied: None,
            overlay: None,
            panel: None,
            was_working: false,
            ticks: 0,
        };
        workspace.apply_appearance(window, cx);
        workspace.focus_center(window, cx);
        workspace
    }

    /// Takes the running turn's events and saves the session now and then.
    /// Runs every frame-length; does nothing when nothing runs.
    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ticks += 1;
        // Handed over to the launcher (Use this build): end, so it can start
        // the build; the session and the project are saved.
        if self.studio.read(cx).develop.quit {
            if let Some(code) = self.studio.read(cx).develop.exit {
                // Handing over to the supervisor (C-53): the work is stopped and
                // saved, and the exit code tells it which way to go.
                self.studio.update(cx, |studio, _| {
                    let left = studio.stop_work(std::time::Duration::from_secs(20));
                    if !left.is_empty() {
                        eprintln!("Handing over while {} had not ended", left.join(", "));
                    }
                    studio.save_session();
                    studio.control.close_endpoint();
                });
                std::process::exit(code);
            }
            self.studio.update(cx, |studio, _| {
                studio.stop_work(std::time::Duration::from_secs(20));
            });
            cx.quit();
            return;
        }
        let studio = self.studio.read(cx);
        let running = studio.conversation.running();
        let waiting_dialog = studio.conversation.waiting.is_some();
        if running || waiting_dialog {
            let working = self.studio.read(cx).assistant_working();
            self.studio.act(cx, |studio| {
                let before = studio.conversation.conversation.entries.len();
                let live = studio.conversation.live.len();
                let revision = studio.project.as_ref().map(|p| p.state().revision());
                studio.poll_conversation();
                let changed_model =
                    studio.project.as_ref().map(|p| p.state().revision()) != revision;
                let _ = (before, live);
                studio.mark(Dirty::CONVERSATION | Dirty::STATUS);
                if changed_model {
                    studio.mark(Dirty::MODEL | Dirty::SELECTION);
                }
                if studio.dialog.is_some() {
                    studio.mark(Dirty::OVERLAY);
                }
            });
            let now_working = self.studio.read(cx).assistant_working();
            // A turn that ends or stops to ask while the window is in the
            // background flashes the taskbar (§3.4).
            if crate::studio::needs_attention(working, now_working, Some(window.is_window_active()))
            {
                window.request_attention();
            }
            self.was_working = now_working;
        }
        // Runs and implementation checks on their threads, and playback.
        let studio = self.studio.read(cx);
        if studio.runs.running()
            || studio.runs.playing
            || studio.implementation.checking()
            || studio.implementation.task.is_some()
            || studio.runtime.work.is_some()
            || studio.runtime_reading()
            || studio.develop.work.is_some()
        {
            self.studio.act(cx, |studio| {
                let finished = studio.poll_runs()
                    | studio.poll_checks()
                    | studio.poll_task()
                    | studio.poll_runtime()
                    | studio.poll_build();
                let moved = studio.playback_tick();
                if finished || moved {
                    studio.mark(Dirty::MODEL | Dirty::LAYOUT | Dirty::STATUS);
                } else {
                    studio.mark(Dirty::STATUS);
                }
            });
        }
        if self.studio.read(cx).objectives.wants_poll() {
            self.studio.act(cx, |studio| {
                studio.poll_objective();
            });
        }
        self.studio
            .update(cx, |studio, _| studio.save_session_now_and_then());
        self.frames_and_screenshot(window, cx);
    }

    /// `--frames` and `--screenshot`: close after the frames, or after the
    /// window settles and its image is saved, and print the metrics.
    fn frames_and_screenshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let studio = self.studio.read(cx);
        if studio.args.scenario_running() {
            return;
        }
        let frames = studio.args.frames;
        let screenshot = studio.args.screenshot.clone();
        // Screens without the Surface (the start screen, Settings, the
        // gallery) count the workspace's ticks.
        let surface = (studio.project.is_some() || studio.fixture.is_some())
            && !studio.settings_open
            && studio.fixture.as_deref() != Some("components");
        let drawn = if surface {
            studio.frame_number
        } else {
            self.ticks
        };
        let settled = drawn >= 20 && studio.camera_target.is_none();
        let done = match (&screenshot, frames) {
            (_, Some(frames)) => drawn >= frames,
            (Some(_), None) => settled,
            (None, None) => false,
        };
        if !done {
            return;
        }
        if let Some(path) = screenshot {
            save_screenshot(window, &path);
        }
        let report = self.studio.read(cx).metrics_report();
        if let Some(path) = &self.studio.read(cx).args.metrics
            && let Err(error) = std::fs::write(path, report.to_string())
        {
            eprintln!("Cannot write the metrics: {error}");
        }
        println!("{report}");
        cx.quit();
        // GPUI's quit on Windows waits for the message queue to empty, which
        // a window that redraws every tick (the gallery, in a debug build)
        // can put off for a minute, saving the image again each tick. The
        // run's work is written; nothing else is saved in these runs.
        std::process::exit(0);
    }

    fn studio_changed(&mut self, dirty: Dirty, window: &mut Window, cx: &mut Context<Self>) {
        if dirty.intersects(Dirty::APPEARANCE) {
            self.apply_appearance(window, cx);
        }
        if dirty.intersects(Dirty::OVERLAY) {
            self.sync_palette(window, cx);
            let conversation_focus = self.studio.read(cx).conversation.focus_input;
            if conversation_focus {
                self.conversation
                    .update(cx, |view, cx| view.focus_input(window, cx));
            }
        }
        if dirty.intersects(
            Dirty::STATUS | Dirty::LAYOUT | Dirty::MODEL | Dirty::OVERLAY | Dirty::CONVERSATION,
        ) {
            let title = match &self.studio.read(cx).project {
                Some(project) => format!("{} — Agentique Studio", project_name(project.folder())),
                None => "Agentique Studio".to_string(),
            };
            window.set_window_title(&title);
        }
        if dirty.intersects(Dirty::ALL) {
            cx.notify();
        }
        // When a dialog, the palette or Settings closes, or the screen in
        // the middle changes, the keyboard goes back to the middle.
        let studio = self.studio.read(cx);
        let overlay = studio
            .dialog
            .as_ref()
            .is_some_and(|d| !matches!(d, crate::edit::Dialog::Rename { .. }))
            || studio.palette.is_some()
            || studio.settings_open;
        let center = self.surface_shown(cx);
        let panel = studio.panel;
        let was = self.overlay.replace((overlay, center));
        let panel_changed = self.panel.replace(panel).is_some_and(|was| was != panel);
        if !overlay
            && (was.is_some_and(|(was_overlay, _)| was_overlay)
                || was.is_some_and(|(_, was_center)| was_center != center)
                || panel_changed)
            || window.focused(cx).is_none()
        {
            self.focus_center(window, cx);
        }
    }

    /// Whether the Surface is in the middle (not the welcome, Settings or
    /// the gallery).
    fn surface_shown(&self, cx: &App) -> bool {
        let studio = self.studio.read(cx);
        (studio.project.is_some() || studio.fixture.is_some())
            && !studio.settings_open
            && self.gallery.is_none()
    }

    /// Gives the keyboard to the Surface, or to the workspace when the
    /// Surface is not shown, so the Studio's shortcuts work at once.
    fn focus_center(&self, window: &mut Window, cx: &mut App) {
        let focus = if self.surface_shown(cx) {
            self.surface.read(cx).focus_handle(cx)
        } else {
            self.focus.clone()
        };
        window.focus(&focus, cx);
    }

    /// The theme, reduced motion and UI scale Settings asks for.
    fn apply_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let studio = self.studio.read(cx);
        let wanted = (
            studio.appearance.dark,
            studio.appearance.contrast,
            studio.reduced_motion,
            studio.ui_scale(),
        );
        if self.applied == Some(wanted) {
            return;
        }
        self.applied = Some(wanted);
        let (dark, contrast, reduced, scale) = wanted;
        ui::install_theme(dark, contrast, cx);
        cx.set_reduce_motion(reduced);
        window.set_rem_size(px(16.0 * scale));
        cx.refresh_windows();
    }

    fn run(&mut self, action: &Run, window: &mut Window, cx: &mut Context<Self>) {
        let id = action.0;
        self.studio.act(cx, |studio| studio.execute(id));
        match id {
            CommandId::AskAssistant | CommandId::InsertSelection | CommandId::MessageObjective => {
                self.conversation
                    .update(cx, |view, cx| view.focus_input(window, cx));
            }
            CommandId::ShowLibrary => {
                // The Library's search takes the keyboard when it is drawn.
                cx.notify();
            }
            CommandId::Settings | CommandId::ShortcutHelp => {
                if self.studio.read(cx).settings_open {
                    self.settings.update(cx, |view, cx| view.focus(window, cx));
                } else {
                    self.focus_center(window, cx);
                }
            }
            _ => {}
        }
        self.sync_palette(window, cx);
    }

    /// Opens or closes the palette as the Studio asks.
    fn sync_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wanted = self.studio.read(cx).palette;
        match (wanted, &self.palette) {
            (Some(mode), None) => {
                let palette = cx.new(|cx| Palette::new(self.studio.clone(), mode, window, cx));
                let subscription = cx.subscribe_in(
                    &palette,
                    window,
                    |this, palette, _: &DismissEvent, window, cx| {
                        // A row may open the palette again in another mode
                        // ("What can connect here?", "Find usages"): only
                        // this palette's own mode closes.
                        let mode = palette.read(cx).mode();
                        this.palette = None;
                        this.studio.act(cx, |studio| {
                            if studio.palette == Some(mode) {
                                studio.palette = None;
                            }
                            studio.mark(Dirty::OVERLAY);
                        });
                        if this.studio.read(cx).palette.is_some() {
                            this.sync_palette(window, cx);
                        } else {
                            this.focus_center(window, cx);
                        }
                    },
                );
                palette.update(cx, |palette, cx| palette.focus(window, cx));
                self.palette = Some((palette, subscription));
                cx.notify();
            }
            (Some(mode), Some((palette, _))) => {
                palette.update(cx, |palette, cx| palette.set_mode(mode, window, cx));
            }
            (None, Some(_)) => {
                self.palette = None;
                cx.notify();
            }
            (None, None) => {}
        }
    }

    fn open_context_menu(
        &mut self,
        position: gpui::Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let studio = self.studio.read(cx);
        let context = studio.context();
        let shared = studio.shared_definition().and_then(|definition| {
            let tree = studio.project.as_ref()?.state().tree();
            Some(crate::edit::display_name(tree, definition))
        });
        let mut items = Vec::new();
        for id in crate::surface::context_commands(studio.selection.primary.as_ref()) {
            let command = commands::command(*id);
            let label = match (&shared, id) {
                (Some(definition), CommandId::Rename | CommandId::Delete | CommandId::Lock) => {
                    format!("{} in {definition} (shared)", command.label)
                }
                _ => command.label.to_string(),
            };
            let studio = self.studio.clone();
            let id = *id;
            let mut item = MenuItem::action(label, move |_, cx| {
                studio.act(cx, |studio| studio.execute(id))
            })
            .id(format!("menu-{}", crate::control::command_name(id)))
            .shortcut(command.shortcut)
            .disabled(commands::unavailable(id, &context));
            if let Some(glyph) = command_icon(id) {
                item = item.icon(glyph);
            }
            if id == CommandId::Delete {
                item = item.danger();
                items.push(MenuItem::Separator);
            }
            items.push(item);
        }
        let menu = cx.new(|cx| Menu::new(items, cx));
        let subscription =
            cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
                this.context_menu = None;
                this.focus_center(window, cx);
                cx.notify();
            });
        let focus = menu.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.context_menu = Some((position, menu, subscription));
        cx.notify();
    }

    fn resize(
        &mut self,
        event: &DragMoveEvent<Resize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Resize(dock) = event.drag(cx).clone();
        let scale = f32::from(window.rem_size()) / 16.0;
        let x = f32::from(event.event.position.x) / scale;
        let width = f32::from(window.viewport_size().width) / scale;
        let studio = self.studio.read(cx);
        let widths = studio.widths;
        let right_of_inspector = if studio.conversation.shown && studio.project.is_some() {
            widths.conversation
        } else {
            0.0
        };
        let new = match dock {
            Dock::Outline => x,
            Dock::Conversation => width - x,
            Dock::Inspector => width - right_of_inspector - x,
        }
        .clamp(Widths::MIN, Widths::MAX);
        self.studio.act(cx, |studio| {
            match dock {
                Dock::Outline => studio.widths.outline = new,
                Dock::Inspector => studio.widths.inspector = new,
                Dock::Conversation => studio.widths.conversation = new,
            }
            studio.mark(Dirty::LAYOUT);
        });
    }
}

#[cfg(feature = "automation")]
impl Workspace {
    pub fn studio(&self) -> &Entity<Studio> {
        &self.studio
    }

    /// What the scripted journeys check in views that keep their own text.
    pub fn views(&self, cx: &App) -> crate::automation::Views {
        crate::automation::Views {
            dialog_text: self.dialogs.read(cx).first_text(cx),
            settings_search: self.settings.read(cx).search_text(cx),
            palette: self
                .palette
                .as_ref()
                .map(|(palette, _)| palette.read(cx).query(cx)),
            library_search: self.outline.read(cx).library().read(cx).query(cx),
        }
    }
}

fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

fn project_name(folder: &std::path::Path) -> String {
    folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder.display().to_string())
}

/// An icon for commands that have one (menus, the palette).
pub fn command_icon(id: CommandId) -> Option<IconName> {
    use CommandId::*;
    Some(match id {
        CreatePart => IconName::Part,
        CreatePort => IconName::Port,
        CreateItem => IconName::Item,
        CreateAttribute => IconName::Attribute,
        CreateInterface => IconName::Interface,
        CreateRequirement => IconName::Requirement,
        Rename => IconName::Pencil,
        Delete => IconName::Trash,
        Connect => IconName::Connection,
        MoveTo => IconName::Move,
        Lock => IconName::Lock,
        Undo => IconName::Undo,
        Redo => IconName::Redo,
        Checkpoint => IconName::Checkpoint,
        History => IconName::Clock,
        Architecture => IconName::Architecture,
        Graph => IconName::Graph,
        Requirements => IconName::Requirements,
        Fit => IconName::Fit,
        ZoomIn => IconName::ZoomIn,
        ZoomOut => IconName::ZoomOut,
        ZoomToSelection => IconName::Locate,
        GoToElement => IconName::Search,
        ShortcutHelp => IconName::Keyboard,
        Focus => IconName::Focus,
        Collapse => IconName::ChevronsUpDown,
        Pin => IconName::Pin,
        Unpin => IconName::PinOff,
        NewProject => IconName::FolderPlus,
        OpenProject => IconName::FolderOpen,
        DevelopAgentique => IconName::Agent,
        Palette => IconName::Command,
        Theme => IconName::Moon,
        Contrast => IconName::Eye,
        ReducedMotion => IconName::Sliders,
        AskAssistant | NewConversation | ShowConversation => IconName::Conversation,
        InsertSelection => IconName::Enter,
        Settings => IconName::Settings,
        HidePanels => IconName::Maximize,
        ShowOutline => IconName::PanelLeft,
        ShowInspector => IconName::PanelRight,
        LeaveFocus => IconName::ChevronLeft,
        ZoomReset => IconName::Search,
        ShowLibrary | InsertFromLibrary => IconName::Library,
        ConnectFromLibrary => IconName::Port,
        OpenDefinition => IconName::Definition,
        FindUsages => IconName::Search,
        Specialize => IconName::Branch,
        CreateBlock => IconName::Component,
        SaveToLibrary => IconName::Save,
        ShowScenarios | NewScenario => IconName::Scenario,
        RunScenario | TracePlay => IconName::Play,
        StopRun => IconName::Stop,
        TraceFirst => IconName::SkipBack,
        TraceBack => IconName::ChevronLeft,
        TraceForward => IconName::ChevronRight,
        TraceLast => IconName::SkipForward,
        CheckImplementation => IconName::Drift,
        TrustLocal => IconName::Terminal,
        StartObjective | MessageObjective => IconName::Agent,
        PauseObjective => IconName::Pause,
        StepObjective => IconName::SkipForward,
        ResumeObjective | ContinueObjective => IconName::Play,
        StopObjective => IconName::Stop,
    })
}

/// Saves the window's last frame as a PNG (`--screenshot`, journeys).
#[cfg(feature = "automation")]
pub fn save_screenshot(window: &mut Window, path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match window.render_to_image() {
        Ok(image) => match image.save(path) {
            Ok(()) => println!("Screenshot saved: {}", path.display()),
            Err(error) => eprintln!("Screenshot failed: {error}"),
        },
        Err(error) => eprintln!("Screenshot failed: {error}"),
    }
}

/// Screenshots need GPUI's frame capture, built with `--features automation`.
#[cfg(not(feature = "automation"))]
pub fn save_screenshot(_: &mut Window, path: &std::path::Path) {
    eprintln!(
        "Screenshot not saved to {}: build the Studio with `--features automation` to capture the window",
        path.display()
    );
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let settings_open = studio.settings_open;
        let has_project = studio.project.is_some();
        let fixture = studio.fixture.clone();
        let welcome = !has_project && fixture.is_none();
        let gallery = self.gallery.clone();
        let focus_mode = studio.panels_hidden;
        let widths = studio.widths;
        let show_outline = !focus_mode
            && !studio.outline_hidden
            && !welcome
            && !settings_open
            && gallery.is_none();
        let show_inspector = !focus_mode
            && !studio.inspector_hidden
            && !welcome
            && !settings_open
            && gallery.is_none();
        let show_conversation =
            !focus_mode && studio.conversation.shown && has_project && !settings_open;
        let room = f32::from(window.viewport_size().width) / (f32::from(window.rem_size()) / 16.0);
        let widths = widths.fitted([show_outline, show_inspector, show_conversation], room);
        // Columns shown at start are simply there; one opened later slides in.
        let animate = self.ticks > 30 && !studio.reduced_motion;
        let context_menu = self
            .context_menu
            .as_ref()
            .map(|(position, menu, _)| (*position, menu.clone()));
        let palette = self.palette.as_ref().map(|(palette, _)| palette.clone());
        let dialog_open = studio
            .dialog
            .as_ref()
            .is_some_and(|dialog| !matches!(dialog, crate::edit::Dialog::Rename { .. }));
        let (center, center_region): (AnyView, &'static str) = if let Some(gallery) = gallery {
            (gallery.into(), "gallery")
        } else if settings_open {
            (self.settings.clone().into(), "settings")
        } else if welcome {
            (self.welcome.clone().into(), "welcome")
        } else {
            (self.surface.clone().into(), "surface")
        };
        // What is on screen, for the control interface (C-53): the cached
        // columns shown keep their records until they are painted again.
        let mounted: Vec<&'static str> = [
            (show_outline, "outline"),
            (show_inspector, "inspector"),
            (show_conversation, "conversation"),
        ]
        .into_iter()
        .filter_map(|(shown, name)| shown.then_some(name))
        .collect();
        ui::target::begin_frame(&mounted);
        let now = std::time::Instant::now();
        let marks: Vec<crate::control::Mark> = studio
            .control
            .marks
            .iter()
            .filter(|m| m.until > now)
            .cloned()
            .collect();
        let effects: Vec<crate::control::Effect> = studio
            .control
            .effects
            .iter()
            .filter(|e| e.progress(now) < 1.0)
            .cloned()
            .collect();
        let reduced_motion = studio.reduced_motion;
        let root = div()
            .id("workspace")
            .key_context("Workspace")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.chrome)
            .text_color(theme.text)
            .font_family(theme::SANS)
            .text_size(r(theme::text::BASE))
            .on_action(cx.listener(Self::run))
            .on_drag_move(cx.listener(Self::resize))
            .child(ui::target::region("title"))
            .child(self.title_bar(window, cx))
            .child(ui::target::region_end())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .border_t_1()
                    .border_color(theme.separator)
                    .when(show_outline, |this| {
                        this.child(dock_column(
                            Dock::Outline,
                            widths.outline,
                            self.outline.clone().into(),
                            animate,
                            cx,
                        ))
                        .child(splitter(Dock::Outline, cx))
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .bg(theme.canvas)
                            .child(ui::target::region(center_region))
                            .child(center)
                            .child(ui::target::region_end()),
                    )
                    .when(show_inspector, |this| {
                        this.child(splitter(Dock::Inspector, cx)).child(dock_column(
                            Dock::Inspector,
                            widths.inspector,
                            self.inspector.clone().into(),
                            animate,
                            cx,
                        ))
                    })
                    .when(show_conversation, |this| {
                        this.child(splitter(Dock::Conversation, cx))
                            .child(dock_column(
                                Dock::Conversation,
                                widths.conversation,
                                self.conversation.clone().into(),
                                animate,
                                cx,
                            ))
                    }),
            )
            .child(ui::target::region("status"))
            .child(self.status_bar(cx))
            .child(ui::target::region_end())
            .when(dialog_open, |this| {
                this.child(ui::target::region("dialog"))
                    .child(self.dialogs.clone())
                    .child(ui::target::region_end())
            })
            .when_some(palette, |this, palette| {
                this.child(ui::target::region("palette"))
                    .child(palette)
                    .child(ui::target::region_end())
            })
            .children(agent_marks(&marks, &theme))
            .children(agent_effects(&effects, now, reduced_motion, &theme))
            .when_some(context_menu, |this, (position, menu)| {
                this.child(
                    deferred(
                        anchored()
                            .position(position)
                            .child(div().mt(px(2.0)).child(menu)),
                    )
                    .with_priority(2),
                )
            });
        // The first frame's paint ends the start budget, whatever it shows.
        let root = root.when(self.ticks == 0, |root| {
            let studio = self.studio.clone();
            root.child(
                gpui::canvas(
                    |_, _, _| {},
                    move |_, _, _, cx| {
                        studio.update(cx, |studio, _| {
                            studio.timing.first_paint();
                            studio.mark_ready();
                        })
                    },
                )
                .absolute()
                .size_0(),
            )
        });
        #[cfg(feature = "automation")]
        let root = root.child(crate::timing::paint_end_marker());
        root
    }
}

/// A docked column; one opened after the start slides in with a short
/// spring (§3.2 motion: 200 ms panels). It is cached, so the Surface's frames
/// do not draw it again.
fn dock_column(dock: Dock, width: f32, view: AnyView, animate: bool, cx: &App) -> impl IntoElement {
    let spring = ui::primitives::spring().to(width);
    let spring = if animate { spring.from(0.0) } else { spring };
    let theme = cx.theme();
    div()
        .id(match dock {
            Dock::Outline => "dock-outline",
            Dock::Inspector => "dock-inspector",
            Dock::Conversation => "dock-conversation",
        })
        .flex_none()
        .h_full()
        .overflow_hidden()
        .bg(theme.chrome)
        .child(view.cached(gpui::StyleRefinement::default().size_full()))
        .with_spring(
            match dock {
                Dock::Outline => "dock-outline-width",
                Dock::Inspector => "dock-inspector-width",
                Dock::Conversation => "dock-conversation-width",
            },
            spring,
            |this, width: f32| this.w(r(width)),
        )
}

/// Where agents acted (C-53): a ring on the control and who did what, for
/// a moment after the action.
fn agent_marks(marks: &[crate::control::Mark], theme: &ui::theme::Theme) -> Vec<gpui::AnyElement> {
    marks
        .iter()
        .map(|mark| {
            let b = mark.bounds;
            div()
                .absolute()
                .left(b.origin.x - px(3.0))
                .top(b.origin.y - px(3.0))
                .w(b.size.width + px(6.0))
                .h(b.size.height + px(6.0))
                .rounded(px(6.0))
                .border_2()
                .border_color(theme.accent.solid)
                .child(
                    div()
                        .absolute()
                        .bottom_full()
                        .left_0()
                        .mb(px(2.0))
                        .px(px(6.0))
                        .py(px(1.0))
                        .rounded(px(4.0))
                        .bg(theme.accent.solid)
                        .text_color(theme.accent.on_solid)
                        .text_size(px(11.0))
                        .whitespace_nowrap()
                        .child(mark.label.clone()),
                )
                .into_any_element()
        })
        .collect()
}

/// Where agents clicked and scrolled (observer mode, C-54): a ripple at a
/// click's point, an arrow at a scroll's, fading out. Under reduced motion
/// the ripple does not grow.
fn agent_effects(
    effects: &[crate::control::Effect],
    now: std::time::Instant,
    reduced_motion: bool,
    theme: &ui::theme::Theme,
) -> Vec<gpui::AnyElement> {
    effects
        .iter()
        .map(|effect| {
            let t = effect.progress(now);
            let fade = 1.0 - t;
            match effect.scroll {
                None => {
                    let size = if reduced_motion {
                        22.0
                    } else {
                        10.0 + 28.0 * t
                    };
                    div()
                        .absolute()
                        .left(effect.at.x - px(size / 2.0))
                        .top(effect.at.y - px(size / 2.0))
                        .size(px(size))
                        .rounded_full()
                        .border_2()
                        .border_color(theme.accent.solid.opacity(fade))
                        .bg(theme.accent.solid.opacity(0.18 * fade))
                        .into_any_element()
                }
                Some(dy) => div()
                    .absolute()
                    .left(effect.at.x - px(14.0))
                    .top(effect.at.y - px(14.0))
                    .size(px(28.0))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme.accent.solid.opacity(0.85 * fade))
                    .child(
                        icon(if dy < 0.0 {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        })
                        .size(16.0)
                        .color(theme.accent.on_solid.opacity(fade)),
                    )
                    .into_any_element(),
            }
        })
        .collect()
}

/// What the latest agent action was for and how it ended, under the agents
/// chip for a few seconds (observer mode, C-54): the agent and its goal,
/// its decision (the reason it gave) and the outcome.
fn agent_outcome(outcome: &crate::control::Outcome, theme: &ui::theme::Theme) -> gpui::AnyElement {
    let line = |text: String, colour: gpui::Hsla| {
        div()
            .min_w_0()
            .overflow_hidden()
            .text_ellipsis()
            .whitespace_nowrap()
            .text_color(colour)
            .child(text)
    };
    let (glyph, colour, result) = match &outcome.refused {
        None if !outcome.ended => (
            IconName::Agent,
            theme.text_secondary,
            format!("Doing: {}", outcome.what),
        ),
        None => (
            IconName::CircleCheck,
            theme.success.text,
            format!("Done: {}", outcome.what),
        ),
        Some(reason) => (
            IconName::CircleX,
            theme.danger.text,
            format!("Refused: {reason}"),
        ),
    };
    div()
        .id("agents-outcome")
        .w(r(340.0))
        .p(r(10.0))
        .flex()
        .flex_col()
        .gap(r(3.0))
        .rounded(r(crate::tokens::radius::MENU))
        .bg(theme.overlay)
        .border_1()
        .border_color(theme.border)
        .shadow(theme.shadow_overlay())
        .text_size(r(theme::text::XS))
        .role(gpui::Role::Status)
        .child(
            div()
                .flex()
                .items_center()
                .gap(r(6.0))
                .child(icon(IconName::Agent).size(12.0).color(theme.accent.solid))
                .child(
                    div()
                        .font_weight(theme::SEMIBOLD)
                        .text_color(theme.text)
                        .child(outcome.agent.clone()),
                )
                .when(!outcome.goal.is_empty(), |this| {
                    this.child(line(format!("· {}", outcome.goal), theme.text_secondary))
                }),
        )
        .when(!outcome.why.is_empty(), |this| {
            this.child(line(
                format!("Decision: {}", outcome.why),
                theme.text_secondary,
            ))
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap(r(6.0))
                .child(icon(glyph).size(12.0).color(colour))
                .child(line(result, colour)),
        )
        .into_any_element()
}

/// The hairline between the Surface and a docked column; drag it to resize
/// the column.
fn splitter(dock: Dock, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .id(match dock {
            Dock::Outline => "splitter-outline",
            Dock::Inspector => "splitter-inspector",
            Dock::Conversation => "splitter-conversation",
        })
        .flex_none()
        .w(px(1.0))
        .h_full()
        .bg(theme.separator)
        .relative()
        .child(
            div()
                .id(match dock {
                    Dock::Outline => "splitter-outline-hit",
                    Dock::Inspector => "splitter-inspector-hit",
                    Dock::Conversation => "splitter-conversation-hit",
                })
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(-3.0))
                .w(px(7.0))
                .cursor_col_resize()
                .hover(|style| style.bg(theme.accent.solid.opacity(0.35)))
                .on_drag(Resize(dock), |_, _, _, cx| cx.new(|_| Empty)),
        )
}

impl Workspace {
    fn title_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let project = studio.project.as_ref().map(|p| project_name(p.folder()));
        let fixture = studio.fixture.clone();
        let view_index = match studio.view {
            View::Architecture => 0,
            View::Graph => 1,
            View::Requirements => 2,
        };
        let outline_shown = !studio.panels_hidden && !studio.outline_hidden;
        let inspector_shown = !studio.panels_hidden && !studio.inspector_hidden;
        let conversation_shown = !studio.panels_hidden && studio.conversation.shown;
        let settings_open = studio.settings_open;
        let surface_shown = (studio.project.is_some() || studio.fixture.is_some())
            && !settings_open
            && self.gallery.is_none();
        let saved = studio.saved.clone();
        let uncommitted = studio.history.uncommitted.unwrap_or(false);
        let maximized = window.is_maximized();
        let run = |studio: Entity<Studio>, id: CommandId| {
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                window.dispatch_action(Box::new(Run(id)), cx);
                let _ = &studio;
            }
        };
        let studio_entity = self.studio.clone();
        let shortcut = |id| Some(commands::command(id).shortcut);
        let height = 40.0;
        // What agents are doing (C-53): the latest action, and the gate.
        let agents_paused = studio.agents_paused();
        let agents_stopped = studio.control.gate == crate::control::Gate::Stop;
        let agents_shown = studio.control.busy()
            || agents_paused
            || (studio.conversation.running() && studio.conversation.steerable);
        let agent_activity = if agents_stopped {
            "Agents stopped".to_string()
        } else {
            studio
                .control
                .activity()
                .map(str::to_string)
                .or_else(|| studio.conversation.phase.map(|p| format!("Assistant: {p}")))
                .unwrap_or_else(|| "Agents".to_string())
        };
        let held = studio.control.held();
        // The action being carried out, or how the latest ended for a few
        // seconds (C-54), under the chip.
        let outcome = studio.control.shown();
        // The window moves by the bar's empty stretches only: Windows asks
        // which area is under the pointer and GPUI answers with the first
        // window-control area there, so a drag area around the buttons
        // would take their presses (and min, max and close their own).
        let drag = || {
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .window_control_area(WindowControlArea::Drag)
        };
        // Two parts (W13.7): what gives way when room is short (the
        // project, the agents' chip, the views, the search), shrinking and
        // then cut at its end; and what never does (the panels' switches,
        // Settings, and the window's own buttons), so no state of the bar
        // pushes them out of the window.
        let gives_way = div()
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .items_center()
            .gap(r(8.0))
            .child(div().w(r(4.0)).h_full().window_control_area(WindowControlArea::Drag))
            // The project, and how its saving stands: its name is cut short
            // before anything else gives way.
            .child(
                div()
                    .flex()
                    .min_w(r(72.0))
                    .overflow_hidden()
                    .items_center()
                    .gap(r(8.0))
                    .child(brand_mark(&theme).window_control_area(WindowControlArea::Drag))
                    .child(match &project {
                        Some(name) => Button::new("project", name.clone())
                            .ghost()
                            .trailing(IconName::ChevronDown)
                            .tooltip("Open another project", Some("Ctrl+O"))
                            .on_click(run(studio_entity.clone(), CommandId::OpenProject))
                            .into_any_element(),
                        None => div()
                            .window_control_area(WindowControlArea::Drag)
                            .text_size(r(theme::text::BASE))
                            .font_weight(theme::MEDIUM)
                            .text_color(theme.text_secondary)
                            .child(match &fixture {
                                Some(name) => format!("Example · {name}"),
                                None => "Agentique".to_string(),
                            })
                            .into_any_element(),
                    })
                    .when(project.is_some(), |this| {
                        this.child(save_state(&saved, uncommitted, &theme))
                    }),
            )
            .child(drag())
            // Agents at work (C-53): what the latest one did, Pause or
            // Resume, and Step.
            .when(agents_shown, |this| {
                let studio = self.studio.clone();
                this.child(
                    div()
                        .id("agents")
                        .flex()
                        .items_center()
                        .gap(r(4.0))
                        // Never narrower than its icon and buttons (Pause
                        // and Stop; Resume, Step and Stop when paused; Resume
                        // when stopped), measured from the font: only what
                        // the latest agent did shortens (W13.7: at a small
                        // window its buttons covered the views).
                        .max_w(r(460.0))
                        .min_w(r(if agents_stopped {
                            128.0
                        } else if agents_paused {
                            240.0
                        } else {
                            180.0
                        }))
                        .px(r(8.0))
                        .h(r(26.0))
                        .rounded(r(crate::tokens::radius::CONTROL))
                        .border_1()
                        .border_color(if agents_stopped {
                            theme.danger.solid
                        } else if agents_paused {
                            theme.warning.solid
                        } else {
                            theme.accent.solid
                        })
                        .role(gpui::Role::Status)
                        .aria_label(agent_activity.clone())
                        .child(icon(IconName::Agent).size(13.0).color(theme.accent.solid))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(r(theme::text::XS))
                                .text_color(theme.text_secondary)
                                .child(if held > 0 {
                                    format!("{agent_activity} · {held} waiting")
                                } else {
                                    agent_activity.clone()
                                }),
                        )
                        .child(if agents_paused {
                            Button::new("agents-resume", "Resume")
                                .small()
                                .ghost()
                                .icon(IconName::Play)
                                .tooltip("Let agents go on", None)
                                .on_click({
                                    let studio = studio.clone();
                                    move |_: &ClickEvent, _, cx| {
                                        studio.act(cx, |studio| studio.resume_agents())
                                    }
                                })
                                .into_any_element()
                        } else {
                            Button::new("agents-pause", "Pause")
                                .small()
                                .ghost()
                                .icon(IconName::Pause)
                                .tooltip("Hold agents at their next action or tool call", None)
                                .on_click({
                                    let studio = studio.clone();
                                    move |_: &ClickEvent, _, cx| {
                                        studio.act(cx, |studio| studio.pause_agents())
                                    }
                                })
                                .into_any_element()
                        })
                        .when(agents_paused && !agents_stopped, |this| {
                            this.child(
                                Button::new("agents-step", "Step")
                                    .small()
                                    .ghost()
                                    .tooltip(
                                        "Let one action, typed character or tool call through, then hold again",
                                        None,
                                    )
                                    .on_click({
                                        let studio = studio.clone();
                                        move |_: &ClickEvent, _, cx| {
                                            studio.act(cx, |studio| studio.step_agents())
                                        }
                                    }),
                            )
                        })
                        .when(!agents_stopped, |this| {
                            this.child(
                                Button::new("agents-stop", "Stop")
                                    .small()
                                    .ghost()
                                    .icon(IconName::Stop)
                                    .tooltip(
                                        "End agents' action at once and refuse their actions until you resume them",
                                        None,
                                    )
                                    .on_click({
                                        let studio = studio.clone();
                                        move |_: &ClickEvent, _, cx| {
                                            studio.act(cx, |studio| studio.stop_agents())
                                        }
                                    }),
                            )
                        })
                        .when_some(outcome, |this, outcome| {
                            let card = agent_outcome(&outcome, &theme);
                            // Hung from the chip's lower right corner.
                            this.relative().child(
                                div().absolute().top_full().right_0().mt(r(6.0)).child(
                                    deferred(
                                        anchored()
                                            .anchor(gpui::Anchor::TopRight)
                                            .snap_to_window()
                                            .child(card),
                                    )
                                    .with_priority(1),
                                ),
                            )
                        }),
                )
            })
            // The views of the Surface.
            .when(surface_shown, |this| {
                let studio = self.studio.clone();
                // Icons only where the window is narrow (W13.7: at the
                // smallest window the labels pushed the window's own
                // buttons out of it).
                let room = f32::from(window.viewport_size().width)
                    / (f32::from(window.rem_size()) / 16.0);
                this.child(
                    Segmented::new("views", view_index)
                        .compact(room < 1200.0)
                        .choice(Some(IconName::Architecture), "Architecture")
                        .tooltip(
                            "Containment, ports and connections",
                            shortcut(CommandId::Architecture),
                        )
                        .choice(Some(IconName::Graph), "Graph")
                        .tooltip(
                            "Elements layered by their relationships",
                            shortcut(CommandId::Graph),
                        )
                        .choice(Some(IconName::Requirements), "Requirements")
                        .tooltip(
                            "Requirements, what satisfies them and their subjects",
                            shortcut(CommandId::Requirements),
                        )
                        .on_choose(move |index, _, cx| {
                            let id = [
                                CommandId::Architecture,
                                CommandId::Graph,
                                CommandId::Requirements,
                            ][index];
                            studio.act(cx, |studio| studio.execute(id));
                        }),
                )
            })
            .when(settings_open, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(8.0))
                        .text_size(r(theme::text::BASE))
                        .font_weight(theme::MEDIUM)
                        .child(icon(IconName::Settings).size(14.0).color(theme.text_muted))
                        .child("Settings"),
                )
            })
            .child(drag())
            // Search.
            .child(
                div()
                    .id("search")
                    .relative()
                    .flex()
                    .items_center()
                    .gap(r(8.0))
                    // It may shrink to its icon when room is short (W13.7:
                    // at the smallest window it pushed the window's own
                    // buttons out of it).
                    .w(r(260.0))
                    .min_w(r(36.0))
                    .overflow_hidden()
                    .h(r(28.0))
                    .px(r(10.0))
                    .rounded(r(crate::tokens::radius::CONTROL))
                    .bg(theme.inset)
                    .border_1()
                    .border_color(theme.separator)
                    .text_color(theme.text_faint)
                    .text_size(r(theme::text::SM))
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .border_color(theme.border)
                            .text_color(theme.text_muted)
                    })
                    .role(gpui::Role::Button)
                    .aria_label("Search or run a command")
                    .on_click(run(studio_entity.clone(), CommandId::Palette))
                    .child(ui::target::control(
                        ui::target::Control::new("button", "Search or run a command")
                            .id("title-search"),
                    ))
                    .child(icon(IconName::Search).size(14.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child("Search or run a command"),
                    )
                    .child(ui::KeyCaps::new("Ctrl+K")),
            );
        div()
            .id("title-bar")
            .flex_none()
            .w_full()
            .h(r(height))
            .flex()
            .items_center()
            .gap(r(8.0))
            .bg(theme.chrome)
            .child(gives_way)
            // The panels' switches and Settings.
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(r(2.0))
                    .child(
                        Button::icon_only("toggle-outline", IconName::PanelLeft, "Outline")
                            .selected(outline_shown)
                            .tooltip("Show or hide the Outline", shortcut(CommandId::ShowOutline))
                            .on_click(run(studio_entity.clone(), CommandId::ShowOutline)),
                    )
                    .child(
                        Button::icon_only("toggle-inspector", IconName::PanelRight, "Inspector")
                            .selected(inspector_shown)
                            .tooltip(
                                "Show or hide the Inspector",
                                shortcut(CommandId::ShowInspector),
                            )
                            .on_click(run(studio_entity.clone(), CommandId::ShowInspector)),
                    )
                    .child(
                        Button::icon_only(
                            "toggle-conversation",
                            IconName::Conversation,
                            "Conversation",
                        )
                        .selected(conversation_shown)
                        .tooltip(
                            "Show or hide the Conversation",
                            shortcut(CommandId::ShowConversation),
                        )
                        .on_click(run(studio_entity.clone(), CommandId::ShowConversation)),
                    )
                    .child(
                        Button::icon_only("open-settings", IconName::Settings, "Settings")
                            .selected(settings_open)
                            .tooltip("Settings", shortcut(CommandId::Settings))
                            .on_click(run(studio_entity.clone(), CommandId::Settings)),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .h_full()
                    .child(window_controls(maximized, &theme)),
            )
    }

    fn status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let problems: usize = studio.problems.values().map(Vec::len).sum();
        let locks = studio
            .project
            .as_ref()
            .map_or(0, |p| p.state().locks().len());
        // A running objective's phase and spend lead the status line (C-53).
        let status = match studio.objective_status() {
            Some(objective) if studio.status.is_empty() => objective,
            Some(objective) => format!("{objective} — {}", studio.status),
            None => studio.status.clone(),
        };
        let working = studio.conversation.running();
        let waiting = studio.conversation.waiting.is_some();
        let view = studio.view.title();
        let zoom = studio.camera.zoom;
        let comparing = studio.comparison.is_some();
        let studio_entity = self.studio.clone();
        div()
            .id("status-bar")
            .flex_none()
            .h(r(26.0))
            .px(r(10.0))
            .flex()
            .items_center()
            .gap(r(12.0))
            .border_t_1()
            .border_color(theme.separator)
            .bg(theme.chrome)
            .text_size(r(theme::text::SM))
            .text_color(theme.text_muted)
            .role(gpui::Role::Status)
            .child(
                div()
                    .id("status-line")
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .aria_label(SharedString::from(status.clone()))
                    .child(status),
            )
            .when(working, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(6.0))
                        .text_color(theme.info.text)
                        .child(if waiting {
                            icon(IconName::Question).size(12.0).into_any_element()
                        } else {
                            ui::spinner("assistant-working", 12.0, theme.info.text)
                                .into_any_element()
                        })
                        .child(if waiting {
                            "Waiting for you"
                        } else {
                            "Assistant working"
                        }),
                )
            })
            .when(comparing, |this| {
                this.child(
                    ui::Chip::new("Comparing")
                        .icon(IconName::Compare)
                        .tone(Tone::Info),
                )
            })
            .child(
                div()
                    .id("status-problems")
                    .relative()
                    .flex()
                    .items_center()
                    .gap(r(5.0))
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.text))
                    .text_color(if problems > 0 {
                        theme.warning.text
                    } else {
                        theme.text_muted
                    })
                    .role(gpui::Role::Button)
                    .aria_label(SharedString::from(format!("{problems} problems")))
                    .child(ui::target::control(
                        ui::target::Control::new(
                            "button",
                            format!("{problems} problems: show them"),
                        )
                        .id("status-problems"),
                    ))
                    .on_click(move |_, _, cx| {
                        studio_entity.act(cx, |studio| {
                            studio.panel = crate::studio::Panel::Problems;
                            studio.inspector_hidden = false;
                            studio.panels_hidden = false;
                        })
                    })
                    .child(
                        icon(if problems > 0 {
                            IconName::Warning
                        } else {
                            IconName::CircleCheck
                        })
                        .size(12.0),
                    )
                    .child(if problems == 1 {
                        "1 problem".to_string()
                    } else {
                        format!("{problems} problems")
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(5.0))
                    .child(icon(IconName::Lock).size(12.0))
                    .child(format!("{locks} locked")),
            )
            .child(div().child(format!("{view} · {:.0}%", zoom * 100.0)))
    }
}

/// Agentique's mark: a small tile in the accent colour.
fn brand_mark(theme: &ui::Theme) -> gpui::Div {
    div()
        .size(r(20.0))
        .rounded(r(6.0))
        .bg(theme.accent.solid)
        .flex()
        .items_center()
        .justify_center()
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0.0, 0.0, 1.0, 0.18),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        }])
        .child(
            icon(IconName::Component)
                .size(13.0)
                .color(theme.accent.on_solid),
        )
}

/// How the project's saving stands: saved, not saved (why), and whether it
/// changed since the last checkpoint.
fn save_state(
    saved: &Result<(), String>,
    uncommitted: bool,
    theme: &ui::Theme,
) -> impl IntoElement {
    let (glyph, text, colour, tooltip) = match saved {
        Ok(()) if uncommitted => (
            IconName::CircleDot,
            "Saved · not checkpointed",
            theme.text_muted,
            "Every change is saved; Ctrl+S records a checkpoint in the history".to_string(),
        ),
        Ok(()) => (
            IconName::CircleCheck,
            "Saved",
            theme.text_muted,
            "Every change is saved, and the history has it".to_string(),
        ),
        Err(reason) => (
            IconName::Alert,
            "Not saved",
            theme.danger.text,
            format!("The last change was not saved: {reason}"),
        ),
    };
    let tooltip = ui::tooltip::text(tooltip, None);
    div()
        .id("save-state")
        .flex()
        .items_center()
        .gap(r(5.0))
        .px(r(6.0))
        .h(r(22.0))
        .rounded(r(crate::tokens::radius::CONTROL))
        .text_size(r(theme::text::XS))
        .text_color(colour)
        .role(gpui::Role::Status)
        .aria_label(text)
        .tooltip(move |window, cx| tooltip(window, cx))
        .child(icon(glyph).size(12.0))
        .child(text)
}

/// Minimize, maximize or restore, and close, drawn by the Studio since the
/// title bar is its own; Windows handles them through their areas.
fn window_controls(maximized: bool, theme: &ui::Theme) -> impl IntoElement {
    let control = |id: &'static str,
                   label: &'static str,
                   area: WindowControlArea,
                   glyph: &'static str,
                   danger: bool| {
        let hover = if danger {
            gpui::rgb(0xc42b1c).into()
        } else {
            theme.hover
        };
        div()
            .id(id)
            .window_control_area(area)
            .relative()
            .role(gpui::Role::Button)
            .aria_label(label)
            .w(r(46.0))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.text_secondary)
            .font_family("Segoe MDL2 Assets")
            .text_size(px(10.0))
            .hover(move |style| {
                let style = style.bg(hover);
                if danger {
                    style.text_color(gpui::white())
                } else {
                    style
                }
            })
            .child(ui::target::control(
                ui::target::Control::new("button", label).id(id),
            ))
            .child(glyph)
    };
    div()
        .flex()
        .h_full()
        .ml(r(4.0))
        .child(control(
            "window-minimize",
            "Minimize",
            WindowControlArea::Min,
            "\u{E921}",
            false,
        ))
        .child(control(
            "window-maximize",
            if maximized { "Restore" } else { "Maximize" },
            WindowControlArea::Max,
            if maximized { "\u{E923}" } else { "\u{E922}" },
            false,
        ))
        .child(control(
            "window-close",
            "Close",
            WindowControlArea::Close,
            "\u{E8BB}",
            true,
        ))
}
