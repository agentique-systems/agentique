//! Scripted UI journeys (`--features automation`). Every action is ordinary
//! input given to the window (pointer, keys, text), through the same
//! dispatch the platform uses; checks read the Studio's state on the
//! following frames.
//!
//! - `a-build --project <new folder>`: creates the project through the New
//!   project dialog, builds part of the URL shortener by hand (parts, ports,
//!   a connection, a rename, a type, a lock, a refused locked change) and
//!   records a checkpoint.
//! - `a-crash --project <same folder>`: makes one more edit, writes its
//!   report, and ends the process at once with exit code 3: the project is
//!   not closed, the session is not saved, nothing runs on exit.
//! - `a-reopen --project <same folder>`: in a new process, checks that
//!   everything is as it was left, and shows what changed since the newest
//!   checkpoint. Run a-build, a-crash, a-reopen.
//! - `a-assistant --project <new folder>`: asks the Assistant (a built-in
//!   scripted model, never the network) to build the URL shortener; answers
//!   its question, follows an element link, refuses a locked change, stops
//!   it and undoes its changes.
//! - `d-daily --project <new folder>`: Scenario D's daily paths on the URL
//!   shortener sample, from the first run's welcome.
//! - `e-settings --project <new folder>`: Scenario E without keys or the
//!   network.
//!
//! Screenshots go only to the `--gallery` directory given on the command line.
use crate::{
    edit::Dialog,
    studio::{PaletteMode, Studio},
    ui::target,
    workspace::Workspace,
};
use agq_language::{ElementId, ElementKind, Tree};
use agq_studio_scene::{Point, SceneTarget};
use gpui::{
    App, Entity, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PlatformInput, Window, point, px,
};
use serde::Serialize;
use std::{
    cell::RefCell,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
enum Action {
    Idle,
    /// A keystroke in GPUI's syntax (`ctrl-s`, `shift-1`, `enter`).
    Key(&'static str),
    Text(&'static str),
    /// Click, select all, type.
    Fill(&'static str, String),
    Click(&'static str),
    /// Click the card of the element at this path.
    ClickCard(&'static str),
    /// Click the link to the element at this path in the Conversation.
    ClickLink(&'static str),
    /// Drag from a port on a card to a port on another card: (card, port).
    DragPort((&'static str, &'static str), (&'static str, &'static str)),
    /// End the process at once, as a crash would: nothing is closed or saved.
    Crash,
}

impl Action {
    /// Frames the action takes.
    fn ticks(&self) -> u64 {
        match self {
            Self::Idle | Self::Key(_) | Self::Crash => 1,
            Self::Text(text) => text.chars().count() as u64,
            Self::Click(_) | Self::ClickCard(_) | Self::ClickLink(_) => 2,
            Self::Fill(_, text) => 4 + text.chars().count() as u64,
            Self::DragPort(..) => 12,
        }
    }
    /// Pointer actions wait for the previous one (`POINTER_PAUSE`).
    fn pointer(&self) -> bool {
        matches!(
            self,
            Self::Click(_)
                | Self::ClickCard(_)
                | Self::ClickLink(_)
                | Self::Fill(..)
                | Self::DragPort(..)
        )
    }
}

#[derive(Clone, Debug)]
enum Check {
    StartScreen,
    DialogNewProject,
    ProjectName(&'static str),
    ProjectOpen,
    CreateDialog,
    RenameDialog,
    CheckpointDialog,
    NoDialog,
    Exists(&'static str, ElementKind),
    Selected(&'static str),
    Port(&'static str, &'static str),
    Interface(&'static str, &'static str),
    TypedBy(&'static str, &'static str),
    Locked(&'static str),
    LockConfirmation,
    Checkpoints(usize),
    Comparison(&'static str),
    NothingUnmatched,
    NothingSelected,
    MoveDialog,
    ProblemShown(&'static str),
    NoProblems,
    MessageTyped,
    Question(&'static str),
    AssistantDone(&'static str),
    Missing(&'static str),
    Stopped,
    SettingsSection(&'static str),
    SettingsSearch(&'static str),
    SettingsClosed,
    View(&'static str),
    /// The palette is open for elements, searching for this.
    PaletteSearch(&'static str),
    PanelsHidden(bool),
}

#[derive(Clone, Debug)]
struct Step {
    name: &'static str,
    action: Action,
    check: Check,
    /// Ticks to wait after the action before checking.
    settle: u64,
    screenshot: Option<&'static str>,
}

fn step(name: &'static str, action: Action, check: Check) -> Step {
    Step {
        name,
        action,
        check,
        settle: 4,
        screenshot: None,
    }
}

fn shot(name: &'static str, step: Step) -> Step {
    Step {
        screenshot: Some(name),
        settle: step.settle.max(8),
        ..step
    }
}

const PACKAGE: &str = "UrlShortener";
const SERVICE: &str = "UrlShortener::UrlShortenerService";

fn create(
    key: &'static str,
    name: &'static str,
    path: &'static str,
    kind: ElementKind,
) -> Vec<Step> {
    vec![
        step(
            "open the create dialog",
            Action::Key(key),
            Check::CreateDialog,
        ),
        step("type the name", Action::Text(name), Check::CreateDialog),
        step("create", Action::Key("enter"), Check::Exists(path, kind)),
    ]
}

fn new_project(folder: &Path) -> Vec<Step> {
    vec![
        step("start screen", Action::Idle, Check::StartScreen),
        step(
            "New project…",
            Action::Click("New project…"),
            Check::DialogNewProject,
        ),
        step(
            "project name",
            Action::Fill("Project name", PACKAGE.into()),
            Check::ProjectName(PACKAGE),
        ),
        step(
            "project folder",
            Action::Fill("Project folder", folder.display().to_string()),
            Check::DialogNewProject,
        ),
        step(
            "Create project",
            Action::Click("Create project"),
            Check::ProjectOpen,
        ),
    ]
}

/// Scenario D's daily paths on the URL shortener sample (D1–D4, D6, D9).
fn daily(folder: &Path) -> Vec<Step> {
    vec![
        step("the welcome", Action::Idle, Check::StartScreen),
        step(
            "start from the URL shortener",
            Action::Click("Start from the URL shortener"),
            Check::ProjectName("UrlShortener"),
        ),
        step(
            "project folder",
            Action::Fill("Project folder", folder.display().to_string()),
            Check::DialogNewProject,
        ),
        shot(
            "01-sample",
            step(
                "Create project opens the sample",
                Action::Click("Create project"),
                Check::Exists("UrlShortener::LinkStore", ElementKind::PartDef),
            ),
        ),
        step(
            "Shift+1 fits the view and stays in this view",
            Action::Key("shift-1"),
            Check::View("Architecture"),
        ),
        step(
            "select the store",
            Action::ClickCard("UrlShortener::UrlShortenerService::store"),
            Check::Selected("UrlShortener::UrlShortenerService::store"),
        ),
        shot(
            "02-zoom-to-selection",
            step(
                "Shift+2 zooms to the selection",
                Action::Key("shift-2"),
                Check::Selected("UrlShortener::UrlShortenerService::store"),
            ),
        ),
        step(
            "Ctrl+P goes to an element",
            Action::Key("ctrl-p"),
            Check::PaletteSearch(""),
        ),
        step(
            "type its name",
            Action::Text("ClickStats"),
            Check::PaletteSearch("ClickStats"),
        ),
        shot(
            "03-go-to-element",
            step(
                "Enter shows it",
                Action::Key("enter"),
                Check::Selected("UrlShortener::ClickStats"),
            ),
        ),
        step(
            "2 shows the Graph view",
            Action::Key("2"),
            Check::View("Graph"),
        ),
        shot(
            "04-requirements",
            step(
                "3 shows the Requirements view",
                Action::Key("3"),
                Check::View("Requirements"),
            ),
        ),
        step(
            "1 goes back to the Architecture view",
            Action::Key("1"),
            Check::View("Architecture"),
        ),
        shot(
            "05-shortcuts",
            step(
                "? lists every shortcut",
                Action::Key("?"),
                Check::SettingsSection("Keyboard"),
            ),
        ),
        step(
            "Escape closes Settings",
            Action::Key("escape"),
            Check::SettingsClosed,
        ),
        shot(
            "06-focus-mode",
            step(
                "Ctrl+\\ hides the panels",
                Action::Key("ctrl-\\"),
                Check::PanelsHidden(true),
            ),
        ),
        step(
            "and shows them again",
            Action::Key("ctrl-\\"),
            Check::PanelsHidden(false),
        ),
        step(
            "Ctrl+S asks for a checkpoint message",
            Action::Key("ctrl-s"),
            Check::CheckpointDialog,
        ),
        step(
            "type the message",
            Action::Text("Daily work"),
            Check::CheckpointDialog,
        ),
        step(
            "Enter records the checkpoint",
            Action::Key("enter"),
            Check::Checkpoints(2),
        ),
    ]
}

/// Scenario E's Settings, without keys or the network.
fn settings(folder: &Path) -> Vec<Step> {
    let mut steps = new_project(folder);
    steps.extend([
        shot(
            "01-providers",
            step(
                "Ctrl+, opens Settings at Providers",
                Action::Key("ctrl-,"),
                Check::SettingsSection("Providers"),
            ),
        ),
        shot(
            "02-search-token",
            step(
                "search finds the keys by a synonym",
                Action::Fill("Search settings", "token".into()),
                Check::SettingsSearch("token"),
            ),
        ),
        shot(
            "03-assistant",
            step(
                "the Assistant section",
                Action::Click("Assistant"),
                Check::SettingsSection("Assistant"),
            ),
        ),
        shot(
            "04-appearance",
            step(
                "the Appearance section",
                Action::Click("Appearance"),
                Check::SettingsSection("Appearance"),
            ),
        ),
        shot(
            "05-keyboard",
            step(
                "the Keyboard section",
                Action::Click("Keyboard"),
                Check::SettingsSection("Keyboard"),
            ),
        ),
        shot(
            "05b-projects",
            step(
                "the Projects section",
                Action::Click("Projects"),
                Check::SettingsSection("Projects"),
            ),
        ),
        shot(
            "05c-advanced",
            step(
                "the Advanced section",
                Action::Click("Advanced"),
                Check::SettingsSection("Advanced"),
            ),
        ),
        shot(
            "06-about",
            step(
                "the About section",
                Action::Click("About"),
                Check::SettingsSection("About"),
            ),
        ),
        step(
            "Escape closes Settings",
            Action::Key("escape"),
            Check::SettingsClosed,
        ),
        step(
            "Ctrl+, opens them again at the last section viewed",
            Action::Key("ctrl-,"),
            Check::SettingsSection("About"),
        ),
        step("Close", Action::Click("Close"), Check::SettingsClosed),
    ]);
    steps
}

fn build_by_hand() -> Vec<Step> {
    let mut steps = Vec::new();
    for (name, path) in [
        ("api", "UrlShortener::api"),
        ("store", "UrlShortener::store"),
        ("stats", "UrlShortener::stats"),
    ] {
        steps.extend(create("p", name, path, ElementKind::Part));
        steps.push(step(
            "clear the selection",
            Action::Key("escape"),
            Check::NothingSelected,
        ));
    }
    for (card, port, name) in [
        ("UrlShortener::api", "UrlShortener::api::storage", "storage"),
        ("UrlShortener::api", "UrlShortener::api::clicks", "clicks"),
        ("UrlShortener::store", "UrlShortener::store::links", "links"),
        (
            "UrlShortener::stats",
            "UrlShortener::stats::clicks",
            "clicks",
        ),
    ] {
        steps.push(step(
            "select the part",
            Action::ClickCard(card),
            Check::Selected(card),
        ));
        steps.extend(create("o", name, port, ElementKind::Port));
    }
    steps.push(step(
        "clear the selection",
        Action::Key("escape"),
        Check::NothingSelected,
    ));
    steps.push(shot(
        "01-parts-and-ports",
        step(
            "drag from api.storage to store.links",
            Action::DragPort(
                ("UrlShortener::api", "UrlShortener::api::storage"),
                ("UrlShortener::store", "UrlShortener::store::links"),
            ),
            Check::Interface("api.storage", "store.links"),
        ),
    ));
    steps.extend([
        step(
            "select stats",
            Action::ClickCard("UrlShortener::stats"),
            Check::Selected("UrlShortener::stats"),
        ),
        step(
            "F2 renames in place",
            Action::Key("f2"),
            Check::RenameDialog,
        ),
        step(
            "select the old name",
            Action::Key("ctrl-a"),
            Check::RenameDialog,
        ),
        step(
            "type the new name",
            Action::Text("statistics"),
            Check::RenameDialog,
        ),
        step(
            "Enter renames",
            Action::Key("enter"),
            Check::Exists("UrlShortener::statistics", ElementKind::Part),
        ),
        step(
            "select store",
            Action::ClickCard("UrlShortener::store"),
            Check::Selected("UrlShortener::store"),
        ),
    ]);
    steps.extend(create(
        "a",
        "capacity",
        "UrlShortener::store::capacity",
        ElementKind::Attribute,
    ));
    steps.extend([
        step(
            "type Natural in the Inspector's type field",
            Action::Fill("Type", "Natural".into()),
            Check::NoDialog,
        ),
        step(
            "Enter sets the type",
            Action::Key("enter"),
            Check::TypedBy("UrlShortener::store::capacity", "Natural"),
        ),
        step(
            "select api",
            Action::ClickCard("UrlShortener::api"),
            Check::Selected("UrlShortener::api"),
        ),
        shot(
            "02-locked",
            step(
                "L locks api",
                Action::Key("l"),
                Check::Locked("UrlShortener::api"),
            ),
        ),
        step(
            "rename the locked part",
            Action::Key("f2"),
            Check::RenameDialog,
        ),
        step(
            "select the old name",
            Action::Key("ctrl-a"),
            Check::RenameDialog,
        ),
        step(
            "type a new name",
            Action::Text("gateway"),
            Check::RenameDialog,
        ),
        shot(
            "03-locked-confirmation",
            step(
                "the change asks for confirmation",
                Action::Key("enter"),
                Check::LockConfirmation,
            ),
        ),
        step(
            "Escape refuses; nothing changes",
            Action::Key("escape"),
            Check::Exists("UrlShortener::api", ElementKind::Part),
        ),
        step(
            "create a port on the locked part",
            Action::Key("o"),
            Check::CreateDialog,
        ),
        step(
            "type the port name",
            Action::Text("shorten"),
            Check::CreateDialog,
        ),
        step(
            "the locked part asks first",
            Action::Key("enter"),
            Check::LockConfirmation,
        ),
        step(
            "Enter confirms the change",
            Action::Key("enter"),
            Check::Exists("UrlShortener::api::shorten", ElementKind::Port),
        ),
        step(
            "clear the selection",
            Action::Key("escape"),
            Check::NothingSelected,
        ),
        step("create another part", Action::Key("p"), Check::CreateDialog),
        step(
            "with a taken name",
            Action::Text("store"),
            Check::CreateDialog,
        ),
        shot(
            "03b-problem-at-element",
            step(
                "the problem shows at the element",
                Action::Key("enter"),
                Check::ProblemShown("store"),
            ),
        ),
        step(
            "Delete removes the new part",
            Action::Key("delete"),
            Check::NoProblems,
        ),
        step(
            "clear the selection",
            Action::Key("escape"),
            Check::NothingSelected,
        ),
    ]);
    steps.extend(create(
        "p",
        "cache",
        "UrlShortener::cache",
        ElementKind::Part,
    ));
    steps.extend([
        step("M moves the part", Action::Key("m"), Check::MoveDialog),
        step(
            "find the new owner",
            Action::Text("store"),
            Check::MoveDialog,
        ),
        step(
            "Enter moves it into store",
            Action::Key("enter"),
            Check::Exists("UrlShortener::store::cache", ElementKind::Part),
        ),
        step(
            "Ctrl+Z undoes the move",
            Action::Key("ctrl-z"),
            Check::Exists("UrlShortener::cache", ElementKind::Part),
        ),
        step(
            "Ctrl+Y redoes it",
            Action::Key("ctrl-y"),
            Check::Exists("UrlShortener::store::cache", ElementKind::Part),
        ),
        step(
            "Ctrl+S asks for a checkpoint message",
            Action::Key("ctrl-s"),
            Check::CheckpointDialog,
        ),
        step(
            "type the message",
            Action::Text("URL shortener skeleton"),
            Check::CheckpointDialog,
        ),
        shot(
            "04-checkpoint",
            step(
                "Enter records the checkpoint",
                Action::Key("enter"),
                Check::Checkpoints(2),
            ),
        ),
    ]);
    steps
}

fn assistant(folder: &Path) -> Vec<Step> {
    let mut steps = new_project(folder);
    steps.extend([
        step(
            "type a request for the Assistant",
            Action::Fill(
                "Message",
                "Build a URL shortener: an HTTP API, a link store and click statistics.".into(),
            ),
            Check::MessageTyped,
        ),
        Step {
            settle: 30,
            ..shot(
                "01-cards-and-question",
                step(
                    "Enter sends; parts appear, then a question",
                    Action::Key("enter"),
                    Check::Question("UrlShortener::UrlShortenerService::store"),
                ),
            )
        },
        Step {
            settle: 30,
            ..shot(
                "02-answered",
                step(
                    "answer with the first option",
                    Action::Click("Option 1"),
                    Check::AssistantDone("UrlShortener::UrlShortenerService::stats"),
                ),
            )
        },
        shot(
            "03-link-selects",
            step(
                "an element link selects the element",
                Action::ClickLink("UrlShortener::UrlShortenerService::store"),
                Check::Selected("UrlShortener::UrlShortenerService::store"),
            ),
        ),
        step(
            "select api",
            Action::ClickCard("UrlShortener::UrlShortenerService::api"),
            Check::Selected("UrlShortener::UrlShortenerService::api"),
        ),
        step(
            "L locks api",
            Action::Key("l"),
            Check::Locked("UrlShortener::UrlShortenerService::api"),
        ),
        step(
            "type a second request",
            Action::Fill("Message", "Add expiring links.".into()),
            Check::MessageTyped,
        ),
        Step {
            settle: 30,
            ..shot(
                "04-locked-asks",
                step(
                    "a change to the locked part asks first",
                    Action::Key("enter"),
                    Check::LockConfirmation,
                ),
            )
        },
        Step {
            settle: 30,
            ..step(
                "Escape refuses; the Assistant goes on and asks",
                Action::Key("escape"),
                Check::Question("UrlShortener::ShortLink::expiresAt"),
            )
        },
        step(
            "the refused change was not made",
            Action::Idle,
            Check::Missing("UrlShortener::UrlShortenerService::api::linkLifetime"),
        ),
        shot(
            "05-stopped",
            step(
                "Stop ends the turn; its work stays",
                Action::Click("Stop"),
                Check::Stopped,
            ),
        ),
        step(
            "the partial work is there",
            Action::Idle,
            Check::Exists("UrlShortener::ShortLink::expiresAt", ElementKind::Attribute),
        ),
        shot(
            "06-undone",
            step(
                "undo the Assistant's changes",
                Action::Click("Undo the Assistant's changes"),
                Check::Missing("UrlShortener::ShortLink::expiresAt"),
            ),
        ),
        step(
            "the first turn's work stays",
            Action::Idle,
            Check::Exists(
                "UrlShortener::UrlShortenerService::stats",
                ElementKind::Part,
            ),
        ),
    ]);
    steps
}

/// The Assistant for `a-assistant`: a scripted stand-in with the replies
/// the journey expects, never the network.
pub fn script_assistant(app: &mut Studio) {
    use serde_json::{Value, json};
    let text = |text: &str| json!({ "type": "text", "text": text });
    let tool = |id: &str, name: &str, input: Value| json!({ "type": "tool_use", "id": id, "name": name, "input": input });
    let reply = |content: Vec<Value>, stop: &str| agq_assistant::Reply {
        content,
        stop_reason: stop.to_string(),
    };
    let changes = |id: &str, description: &str, operations: Value| {
        tool(
            id,
            "apply_changes",
            json!({ "description": description, "operations": operations }),
        )
    };
    let build = json!([
        { "op": "create", "parent": "UrlShortener", "kind": "item def", "name": "ShortLink" },
        { "op": "create", "parent": "UrlShortener::ShortLink", "kind": "attribute", "name": "code", "type": "ScalarValues::String" },
        { "op": "create", "parent": "UrlShortener", "kind": "port def", "name": "LinkStorePort" },
        { "op": "create", "parent": "UrlShortener::LinkStorePort", "kind": "item", "name": "save", "direction": "in", "type": "ShortLink" },
        { "op": "create", "parent": "UrlShortener::LinkStorePort", "kind": "item", "name": "found", "direction": "out", "type": "ShortLink" },
        { "op": "create", "parent": "UrlShortener", "kind": "interface def", "name": "LinkStorage" },
        { "op": "create", "parent": "UrlShortener::LinkStorage", "kind": "port", "name": "client", "type": "~LinkStorePort", "end": true },
        { "op": "create", "parent": "UrlShortener::LinkStorage", "kind": "port", "name": "store", "type": "LinkStorePort", "end": true },
        { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "HttpApi", "doc": "Accepts shorten and resolve requests over HTTP." },
        { "op": "create", "parent": "UrlShortener::HttpApi", "kind": "port", "name": "storage", "type": "~LinkStorePort" },
        { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "LinkStore", "doc": "Keeps short links." },
        { "op": "create", "parent": "UrlShortener::LinkStore", "kind": "port", "name": "links", "type": "LinkStorePort" },
        { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "UrlShortenerService" },
        { "op": "create", "parent": SERVICE, "kind": "part", "name": "api", "type": "HttpApi" },
        { "op": "create", "parent": SERVICE, "kind": "part", "name": "store", "type": "LinkStore" },
        { "op": "connect", "parent": SERVICE, "kind": "interface", "name": "storage",
          "definition": "LinkStorage", "from": "api.storage", "to": "store.links" },
        { "op": "create", "parent": "UrlShortener", "kind": "part", "name": "shortener", "type": "UrlShortenerService" }
    ]);
    let stats = json!([
        { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "ClickStatistics", "doc": "Counts clicks per short link." },
        { "op": "create", "parent": SERVICE, "kind": "part", "name": "stats", "type": "ClickStatistics" }
    ]);
    let finished = [
        "The URL shortener is in place:",
        "",
        "- `UrlShortener::UrlShortenerService::api` takes **shorten** and **resolve** requests.",
        "- `UrlShortener::UrlShortenerService::store` keeps the links; the `storage` interface joins it to the API.",
        "- `UrlShortener::UrlShortenerService::stats` counts clicks, as a *separate* part.",
        "",
        "| Part | Serves |",
        "|---|---|",
        "| api | shorten, resolve |",
        "| store | links |",
        "| stats | clicks |",
        "",
        "Nothing is locked yet: lock the parts you consider settled.",
    ]
    .join("\n");
    let replies = vec![
        reply(
            vec![
                text("I'll read the model first."),
                tool("r1", "read_model", json!({})),
            ],
            "tool_use",
        ),
        reply(
            vec![
                text("An API and a link store, joined by an interface."),
                changes("c1", "Add the API and the link store", build),
            ],
            "tool_use",
        ),
        reply(
            vec![tool(
                "q1",
                "ask_operator",
                json!({
                    "question": "Should click statistics be a separate part, or counted inside the API?",
                    "options": ["A separate part", "Inside the API"]
                }),
            )],
            "tool_use",
        ),
        reply(
            vec![changes("c2", "Add click statistics", stats)],
            "tool_use",
        ),
        reply(vec![text(&finished)], "end_turn"),
        reply(
            vec![
                text("Links need a lifetime. The API is locked, so this asks you first."),
                changes(
                    "c3",
                    "Add a link lifetime to the API",
                    json!([{ "op": "create", "parent": "UrlShortener::UrlShortenerService::api", "kind": "attribute", "name": "linkLifetime" }]),
                ),
            ],
            "tool_use",
        ),
        reply(
            vec![
                text("I'll keep the API as it is and store the expiry with the link."),
                changes(
                    "c4",
                    "Add an expiry time to short links",
                    json!([{ "op": "create", "parent": "UrlShortener::ShortLink", "kind": "attribute", "name": "expiresAt", "type": "ScalarValues::String" }]),
                ),
            ],
            "tool_use",
        ),
        reply(
            vec![tool(
                "q2",
                "ask_operator",
                json!({
                    "question": "Should expired links be deleted, or kept and reported as expired?",
                    "options": ["Delete them", "Keep and report"]
                }),
            )],
            "tool_use",
        ),
    ];
    app.conversation.new_model = crate::conversation::scripted(replies);
    app.conversation.key_missing = None;
    app.conversation.model_name = "scripted stand-in (no network)".into();
}

fn crash() -> Vec<Step> {
    let mut steps = vec![
        step("the project opens", Action::Idle, Check::ProjectOpen),
        step(
            "clear the selection",
            Action::Key("escape"),
            Check::NothingSelected,
        ),
    ];
    steps.extend(create(
        "p",
        "crashEdit",
        "UrlShortener::crashEdit",
        ElementKind::Part,
    ));
    steps.push(step("crash", Action::Crash, Check::ProjectOpen));
    steps
}

fn reopen() -> Vec<Step> {
    vec![
        step("the project opens", Action::Idle, Check::ProjectOpen),
        step(
            "parts",
            Action::Idle,
            Check::Exists("UrlShortener::api", ElementKind::Part),
        ),
        step(
            "renamed part",
            Action::Idle,
            Check::Exists("UrlShortener::statistics", ElementKind::Part),
        ),
        step(
            "ports",
            Action::Idle,
            Check::Port("UrlShortener::store", "links"),
        ),
        step(
            "connection",
            Action::Idle,
            Check::Interface("api.storage", "store.links"),
        ),
        step(
            "type",
            Action::Idle,
            Check::TypedBy("UrlShortener::store::capacity", "Natural"),
        ),
        shot(
            "05-reopened",
            step("lock", Action::Idle, Check::Locked("UrlShortener::api")),
        ),
        step(
            "confirmed, moved and deleted changes",
            Action::Idle,
            Check::Exists("UrlShortener::store::cache", ElementKind::Part),
        ),
        step(
            "the edit made just before the crash is there",
            Action::Idle,
            Check::Exists("UrlShortener::crashEdit", ElementKind::Part),
        ),
        step(
            "every element matched its identity",
            Action::Idle,
            Check::NothingUnmatched,
        ),
        step("no problems left", Action::Idle, Check::NoProblems),
        step("checkpoints", Action::Idle, Check::Checkpoints(2)),
        step(
            "History panel",
            Action::Click("History"),
            Check::Checkpoints(2),
        ),
        step(
            "select the newest checkpoint",
            Action::Click("History 1"),
            Check::Checkpoints(2),
        ),
        shot(
            "06-what-changed",
            step(
                "what changed since then is the edit made before the exit",
                Action::Click("Show changes since then"),
                Check::Comparison("crashEdit"),
            ),
        ),
    ]
}

#[derive(Clone, Serialize)]
struct Report {
    scenario: String,
    passed: bool,
    steps: Vec<String>,
    failure: Option<String>,
    gallery: Vec<String>,
}

/// What the journeys check in views that keep text the Studio does not: the
/// open dialog's first field, the Settings search, the palette's search.
pub struct Views {
    pub dialog_text: Option<String>,
    pub settings_search: String,
    pub palette: Option<(PaletteMode, String)>,
}

pub enum Outcome {
    Running,
    Complete,
}

/// What drives this process: a journey, or one of the benchmarks.
enum Driver {
    Journey(Box<Journey>),
    Stress(Box<crate::stress_automation::Stress>),
    Chat(Box<crate::stress_automation::Chat>),
}

/// Starts the scenario named on the command line, if any. It runs one step
/// at the start of every frame, before the frame is drawn and outside any
/// view's update: input goes through the window as the platform's would.
pub fn start(workspace: Entity<Workspace>, window: &mut Window, cx: &mut App) {
    let studio = workspace.read(cx).studio().clone();
    let Some(scenario) = studio.read(cx).args.scenario.clone() else {
        return;
    };
    let driver = match scenario.as_str() {
        "stress" => Driver::Stress(Box::default()),
        "chat" => Driver::Chat(Box::default()),
        _ => match Journey::new(&scenario, studio.read(cx)) {
            Ok(journey) => Driver::Journey(Box::new(journey)),
            Err(error) => {
                eprintln!("Journey FAILED: {error}");
                std::process::exit(2);
            }
        },
    };
    next_frame(Rc::new(RefCell::new(driver)), workspace, window);
}

fn next_frame(driver: Rc<RefCell<Driver>>, workspace: Entity<Workspace>, window: &mut Window) {
    window.on_next_frame(move |window, cx| {
        let outcome = match &mut *driver.borrow_mut() {
            Driver::Journey(journey) => journey.frame(&workspace, window, cx),
            Driver::Stress(stress) => stress.frame(&workspace, window, cx),
            Driver::Chat(chat) => chat.frame(&workspace, window, cx),
        };
        match outcome {
            // No refresh: that would redraw every view, where input redraws
            // only what it changes (and a measuring Surface asks for frames).
            Outcome::Running => next_frame(driver.clone(), workspace, window),
            Outcome::Complete => cx.quit(),
        }
    });
}

pub struct Journey {
    steps: Vec<Step>,
    index: usize,
    age: u64,
    frames: u64,
    point: Option<(gpui::Point<gpui::Pixels>, gpui::Point<gpui::Pixels>)>,
    capture: Option<(&'static str, u64)>,
    /// When the last pointer action ended: the next waits, so two never
    /// make a double click.
    last_pointer: Option<Instant>,
    /// When the current step's check was first tried.
    checking: Option<Instant>,
    report: Report,
    gallery: Option<std::path::PathBuf>,
    report_path: Option<std::path::PathBuf>,
}

/// Time between pointer actions (the Surface joins clicks 0.35 s apart).
const POINTER_PAUSE: Duration = Duration::from_millis(450);
/// How long a check may keep failing before the journey fails.
const CHECK_PATIENCE: Duration = Duration::from_secs(8);

impl Journey {
    pub fn new(scenario: &str, studio: &Studio) -> Result<Journey, String> {
        let folder = studio
            .args
            .project
            .clone()
            .ok_or("the journeys need --project <folder>")?;
        Ok(Journey {
            steps: match scenario {
                "a-build" => {
                    let mut steps = new_project(&folder);
                    steps.extend(build_by_hand());
                    steps
                }
                "a-crash" => crash(),
                "a-assistant" => assistant(&folder),
                "e-settings" => settings(&folder),
                "d-daily" => daily(&folder),
                "a-reopen" => reopen(),
                other => return Err(format!("no journey is called {other}")),
            },
            index: 0,
            age: 0,
            frames: 0,
            point: None,
            capture: None,
            last_pointer: None,
            checking: None,
            report: Report {
                scenario: scenario.into(),
                passed: false,
                steps: Vec::new(),
                failure: None,
                gallery: Vec::new(),
            },
            gallery: studio.args.gallery.clone(),
            report_path: studio.args.scenario_report.clone(),
        })
    }

    fn write_report(&self) {
        if let Some(path) = &self.report_path
            && let Ok(bytes) = serde_json::to_vec_pretty(&self.report)
            && let Err(error) = std::fs::write(path, bytes)
        {
            eprintln!("Cannot write the report: {error}");
        }
    }

    /// One frame of the journey. Exits the process when a check fails.
    fn frame(
        &mut self,
        workspace: &Entity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> Outcome {
        match self.advance(workspace, window, cx) {
            Ok(Outcome::Running) => Outcome::Running,
            Ok(Outcome::Complete) => {
                self.report.passed = true;
                self.write_report();
                println!(
                    "Journey {} passed: {} steps",
                    self.report.scenario,
                    self.report.steps.len()
                );
                Outcome::Complete
            }
            Err(error) => {
                self.report.failure = Some(error.clone());
                self.write_report();
                // What the window showed when the journey failed.
                if let Some(directory) = &self.gallery
                    && let Ok(image) = window.render_to_image()
                {
                    let _ = std::fs::create_dir_all(directory);
                    let _ =
                        image.save(directory.join(format!("{}-failure.png", self.report.scenario)));
                }
                eprintln!("Journey FAILED: {error}");
                std::process::exit(2);
            }
        }
    }

    fn advance(
        &mut self,
        workspace: &Entity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Outcome, String> {
        let studio = workspace.read(cx).studio().clone();
        self.frames += 1;
        if self.frames > 20_000 {
            return Err("the journey took more than 20,000 frames".into());
        }
        if let Some((name, waited)) = &mut self.capture {
            *waited += 1;
            // A few frames after the check, so the image shows the state.
            if *waited >= 6 {
                let name = *name;
                self.capture = None;
                if let Some(directory) = &self.gallery {
                    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
                    let path = directory.join(format!("{}-{name}.png", self.report.scenario));
                    let image = window
                        .render_to_image()
                        .map_err(|e| format!("screenshot {name}: {e}"))?;
                    image
                        .save(&path)
                        .map_err(|e| format!("Cannot save the screenshot: {e}"))?;
                    self.report.gallery.push(path.display().to_string());
                }
            }
            return Ok(Outcome::Running);
        }
        let Some(step) = self.steps.get(self.index).cloned() else {
            return Ok(Outcome::Complete);
        };
        let pointer = step.action.pointer();
        if self.age == 0
            && pointer
            && self
                .last_pointer
                .is_some_and(|last| last.elapsed() < POINTER_PAUSE)
        {
            return Ok(Outcome::Running);
        }
        let ticks = step.action.ticks();
        if self.age < ticks {
            self.act(&step.action, self.age, &studio, window, cx)
                .map_err(|e| format!("step {} “{}”: {e}", self.index + 1, step.name))?;
            if pointer && self.age + 1 == ticks {
                self.last_pointer = Some(Instant::now());
            }
        }
        if self.age >= ticks + step.settle {
            let checking = *self.checking.get_or_insert_with(Instant::now);
            let views = workspace.read(cx).views(cx);
            match check(&step.check, studio.read(cx), &views) {
                Ok(()) => {
                    self.report
                        .steps
                        .push(format!("{} · {}", self.index + 1, step.name));
                    self.index += 1;
                    self.age = 0;
                    self.point = None;
                    self.checking = None;
                    if let Some(name) = step.screenshot.filter(|_| self.gallery.is_some()) {
                        self.capture = Some((name, 0));
                    }
                    return Ok(Outcome::Running);
                }
                Err(reason) if checking.elapsed() > CHECK_PATIENCE => {
                    return Err(format!(
                        "step {} “{}”: {reason} (status: {})",
                        self.index + 1,
                        step.name,
                        studio.read(cx).status
                    ));
                }
                Err(_) => {}
            }
        }
        self.age += 1;
        Ok(Outcome::Running)
    }

    fn act(
        &mut self,
        action: &Action,
        tick: u64,
        studio: &Entity<Studio>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), String> {
        match action {
            Action::Idle => {}
            Action::Crash => {
                // Report first, then end without closing the project or saving.
                self.report.passed = true;
                self.report
                    .steps
                    .push(format!("{} · crash", self.index + 1));
                self.write_report();
                std::process::exit(3);
            }
            Action::Key(key) => press(key, window, cx)?,
            Action::Text(text) => {
                if let Some(c) = text.chars().nth(tick as usize) {
                    type_char(c, window, cx);
                }
            }
            Action::Click(name) => {
                let at = target::find(name)
                    .ok_or_else(|| format!("{name} is not shown"))?
                    .center();
                click(at, tick == 0, window, cx);
            }
            Action::ClickCard(path) => {
                let at = match self.point {
                    Some((p, _)) => p,
                    None => {
                        let p = card_point(studio.read(cx), path)?;
                        self.point = Some((p, p));
                        p
                    }
                };
                click(at, tick == 0, window, cx);
            }
            Action::ClickLink(path) => {
                let id = find(studio.read(cx), path)?;
                let at = target::find(&format!("Link {}", id.raw()))
                    .ok_or_else(|| format!("the link to {path} is not shown"))?
                    .center();
                click(at, tick == 0, window, cx);
            }
            Action::Fill(name, text) => match tick {
                0 | 1 => {
                    let at = target::find(name)
                        .ok_or_else(|| format!("{name} is not shown"))?
                        .center();
                    click(at, tick == 0, window, cx);
                }
                2 => press("ctrl-a", window, cx)?,
                3 => press("backspace", window, cx)?,
                _ => {
                    if let Some(c) = text.chars().nth((tick - 4) as usize) {
                        type_char(c, window, cx);
                    }
                }
            },
            Action::DragPort(from, to) => {
                let (a, b) = match self.point {
                    Some(points) => points,
                    None => {
                        let points = (
                            port_point(studio.read(cx), *from)?,
                            port_point(studio.read(cx), *to)?,
                        );
                        self.point = Some(points);
                        points
                    }
                };
                let last = action.ticks() - 1;
                if tick == 0 {
                    pointer_button(a, true, window, cx);
                } else if tick < last {
                    let t = tick as f32 / (last - 1) as f32;
                    drag_to(
                        point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t),
                        window,
                        cx,
                    );
                } else {
                    drag_to(b, window, cx);
                    pointer_button(b, false, window, cx);
                }
            }
        }
        Ok(())
    }
}

/// A key in GPUI's syntax, pressed and released. As on Windows, a key that
/// types nothing printable (Enter, Escape, a shortcut) inserts no text.
pub fn press(key: &str, window: &mut Window, cx: &mut App) -> Result<(), String> {
    let keystroke = Keystroke::parse(key).map_err(|e| format!("{key}: {e}"))?;
    window.dispatch_event(
        PlatformInput::KeyDown(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        }),
        cx,
    );
    window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke }), cx);
    Ok(())
}

/// Types one character, as the key that produces it.
fn type_char(c: char, window: &mut Window, cx: &mut App) {
    let keystroke = Keystroke {
        modifiers: Modifiers {
            shift: c.is_uppercase(),
            ..Modifiers::default()
        },
        key: if c == ' ' {
            "space".into()
        } else {
            c.to_lowercase().to_string()
        },
        key_char: Some(c.to_string()),
    };
    window.dispatch_keystroke(keystroke, cx);
}

/// Half a click: the pointer moves there, then the button goes down (or,
/// still held while it moves, up).
fn click(at: gpui::Point<gpui::Pixels>, down: bool, window: &mut Window, cx: &mut App) {
    move_to(at, (!down).then_some(MouseButton::Left), window, cx);
    pointer_button(at, down, window, cx);
}

pub fn move_to(
    at: gpui::Point<gpui::Pixels>,
    pressed: Option<MouseButton>,
    window: &mut Window,
    cx: &mut App,
) {
    window.dispatch_event(
        PlatformInput::MouseMove(MouseMoveEvent {
            position: at,
            pressed_button: pressed,
            modifiers: Modifiers::default(),
        }),
        cx,
    );
}

pub fn drag_to(at: gpui::Point<gpui::Pixels>, window: &mut Window, cx: &mut App) {
    move_to(at, Some(MouseButton::Left), window, cx);
}

pub fn pointer_button(
    at: gpui::Point<gpui::Pixels>,
    down: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let input = if down {
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: at,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        })
    } else {
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: at,
            modifiers: Modifiers::default(),
            click_count: 1,
        })
    };
    window.dispatch_event(input, cx);
}

fn tree(app: &Studio) -> Result<&Tree, String> {
    app.project
        .as_ref()
        .map(|p| p.state().tree())
        .ok_or_else(|| "no project is open".to_string())
}

fn find(app: &Studio, path: &str) -> Result<ElementId, String> {
    tree(app)?
        .find(path)
        .ok_or_else(|| format!("{path} does not exist"))
}

fn screen(app: &Studio, world: Point) -> Result<gpui::Point<gpui::Pixels>, String> {
    let viewport = target::find("Viewport").ok_or("the Surface is not shown")?;
    let local = app.camera.world_to_screen(world);
    let p = point(
        viewport.origin.x + px(local.x),
        viewport.origin.y + px(local.y),
    );
    let inner = viewport.dilate(px(-4.0));
    if !inner.contains(&p) {
        return Err(format!("{p:?} is outside the Surface {viewport:?}"));
    }
    Ok(p)
}

fn card_point(app: &Studio, path: &str) -> Result<gpui::Point<gpui::Pixels>, String> {
    let id = find(app, path)?;
    let node = app
        .lookup
        .node(&app.scene, id)
        .ok_or_else(|| format!("{path} has no card on the Surface"))?;
    let point = if node.is_container {
        Point::new(node.bounds.center().x, node.bounds.min.y + 26.0)
    } else {
        Point::new(node.bounds.center().x, node.bounds.min.y + 40.0)
    };
    screen(app, point)
}

fn port_point(
    app: &Studio,
    (card, port): (&str, &str),
) -> Result<gpui::Point<gpui::Pixels>, String> {
    let (card, port) = (find(app, card)?, find(app, port)?);
    let shown = app
        .lookup
        .port(&app.scene, card, port)
        .ok_or("the port is not shown on the card")?;
    screen(app, shown.position)
}

fn check(check: &Check, app: &Studio, views: &Views) -> Result<(), String> {
    let fail = |message: &str| Err(message.to_string());
    match check {
        Check::StartScreen => {
            if app.project.is_some() {
                return fail("a project is already open");
            }
        }
        Check::DialogNewProject => {
            if !matches!(app.dialog, Some(Dialog::NewProject { .. })) {
                return fail("the New project dialog is not open");
            }
        }
        Check::ProjectName(expected) => {
            if !matches!(app.dialog, Some(Dialog::NewProject { .. }))
                || views.dialog_text.as_deref() != Some(*expected)
            {
                return fail("the project name was not typed");
            }
        }
        Check::ProjectOpen => {
            find(app, PACKAGE)?;
        }
        Check::CreateDialog => {
            if !matches!(app.dialog, Some(Dialog::Create { .. })) {
                return fail("the Create dialog is not open");
            }
        }
        Check::RenameDialog => {
            if !matches!(app.dialog, Some(Dialog::Rename { .. })) {
                return fail("the name is not being edited");
            }
        }
        Check::CheckpointDialog => {
            if !matches!(app.dialog, Some(Dialog::Checkpoint { .. })) {
                return fail("the Checkpoint dialog is not open");
            }
        }
        Check::NoDialog => {
            if app.dialog.is_some() {
                return fail("a dialog is still open");
            }
        }
        Check::Exists(path, kind) => {
            let id = find(app, path)?;
            if tree(app)?[id].kind != *kind {
                return fail("the element has the wrong kind");
            }
            if app.dialog.is_some() {
                return fail("a dialog is still open");
            }
        }
        Check::Selected(path) => {
            let id = find(app, path)?;
            if app.selection.primary != Some(SceneTarget::Node(id))
                && app.selection.primary != Some(SceneTarget::Container(id))
            {
                return Err(format!(
                    "the card is not selected (the selection is {:?})",
                    app.selection.primary
                ));
            }
        }
        Check::NothingSelected => {
            if app.selection.primary.is_some() {
                return fail("something is still selected");
            }
        }
        Check::Port(card, port) => {
            let tree = tree(app)?;
            let card = find(app, card)?;
            if !tree[card].children().iter().any(|c| {
                tree[*c].kind == ElementKind::Port && tree.effective_name(*c) == Some(port)
            }) {
                return fail("the port does not exist");
            }
        }
        Check::Interface(from, to) => {
            let tree = tree(app)?;
            let found = tree.walk().into_iter().any(|id| {
                let e = &tree[id];
                e.kind == ElementKind::Interface
                    && e.ends.len() == 2
                    && e.ends[0].is_linked()
                    && e.ends[1].is_linked()
                    && e.ends[0].to_string() == *from
                    && e.ends[1].to_string() == *to
            });
            if !found {
                return fail("no linked interface connects the two ports");
            }
        }
        Check::TypedBy(path, type_name) => {
            let tree = tree(app)?;
            let id = find(app, path)?;
            if !tree[id]
                .typed_by
                .first()
                .is_some_and(|r| r.is_linked() && r.last_name() == *type_name)
            {
                return fail("the type is not set and linked");
            }
        }
        Check::Locked(path) => {
            let id = find(app, path)?;
            if !app
                .project
                .as_ref()
                .expect("checked by find")
                .state()
                .locks()
                .contains(&id)
            {
                return fail("the element is not locked");
            }
        }
        Check::LockConfirmation => {
            if !matches!(app.dialog, Some(Dialog::Confirm { .. })) {
                return fail("no confirmation was asked");
            }
        }
        Check::Checkpoints(count) => {
            let project = app.project.as_ref().ok_or("no project is open")?;
            let found = project.checkpoints().map_err(|e| e.to_string())?.len();
            if found != *count {
                return Err(format!("{found} checkpoint(s), expected {count}"));
            }
        }
        Check::Comparison(name) => match &app.comparison {
            None => return fail("no comparison is shown"),
            Some(comparison) => {
                let names: Vec<&str> = comparison.created.iter().map(|(_, n)| n.as_str()).collect();
                if names.len() != 1 || !names[0].ends_with(name) {
                    return Err(format!("created since the checkpoint: {names:?}"));
                }
            }
        },
        Check::NothingUnmatched => {
            let project = app.project.as_ref().ok_or("no project is open")?;
            if !project.unmatched().is_empty() {
                return Err(format!("unmatched: {:?}", project.unmatched()));
            }
        }
        Check::MoveDialog => {
            if !matches!(app.dialog, Some(Dialog::MoveTo { .. })) {
                return fail("the Move to dialog is not open");
            }
        }
        Check::ProblemShown(name) => {
            if !app
                .scene
                .nodes
                .iter()
                .any(|n| n.semantic.name == *name && n.semantic.problems > 0)
            {
                return fail("no card with that name shows a problem");
            }
            if app.dialog.is_some() {
                return fail("a dialog is still open");
            }
        }
        Check::NoProblems => {
            let project = app.project.as_ref().ok_or("no project is open")?;
            if let Some(problem) = project.state().diagnostics().first() {
                return Err(format!("a problem remains: {}", problem.message));
            }
        }
        Check::MessageTyped => {
            if app.conversation.input.trim().is_empty() {
                return fail("the message input is empty");
            }
        }
        Check::Question(path) => {
            find(app, path)?;
            if !matches!(
                app.conversation.waiting,
                Some(crate::conversation::Waiting {
                    kind: crate::conversation::WaitingFor::Question { .. },
                    ..
                })
            ) {
                return fail("the Assistant is not asking a question");
            }
        }
        Check::AssistantDone(path) => {
            find(app, path)?;
            if app.conversation.running() || app.dialog.is_some() {
                return fail("the Assistant is still working");
            }
        }
        Check::Missing(path) => {
            if tree(app)?.find(path).is_some() {
                return Err(format!("{path} still exists"));
            }
        }
        Check::SettingsSection(name) => {
            if !app.settings_open {
                return fail("Settings are not open");
            }
            if format!("{:?}", app.settings_section) != *name || !views.settings_search.is_empty() {
                return fail(&format!("Settings do not show {name}"));
            }
        }
        Check::SettingsSearch(query) => {
            if !app.settings_open || views.settings_search != *query {
                return fail(&format!("Settings are not searching for {query}"));
            }
        }
        Check::View(name) => {
            if format!("{:?}", app.view) != *name {
                return fail(&format!("the Surface shows {:?}, not {name}", app.view));
            }
        }
        Check::PanelsHidden(hidden) => {
            if app.panels_hidden != *hidden {
                return fail(&format!(
                    "the panels are not {}",
                    if *hidden { "hidden" } else { "shown" }
                ));
            }
        }
        Check::PaletteSearch(query) => {
            if views
                .palette
                .as_ref()
                .map(|(mode, text)| (*mode, text.as_str()))
                != Some((PaletteMode::Elements, *query))
            {
                return fail(&format!(
                    "the palette is not searching elements for {query:?}"
                ));
            }
        }
        Check::SettingsClosed => {
            if app.settings_open {
                return fail("Settings are still open");
            }
        }
        Check::Stopped => {
            let stopped = matches!(
                app.conversation.conversation.entries.last(),
                Some(agq_assistant::Entry::Notice { text }) if text.starts_with("Stopped")
            );
            if app.conversation.running() || !stopped {
                return fail("the Assistant did not stop");
            }
        }
    }
    Ok(())
}
