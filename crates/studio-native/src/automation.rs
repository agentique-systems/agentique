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
//! - `h-library --project <new folder>`: Scenario H, the Library (C-49), on
//!   the URL shortener sample: search, keyboard preview, "What can connect
//!   here?", drag onto the Surface, a connection, opening a definition and
//!   going back, a specialisation with an override, usages, a block made
//!   from a selection, My Library into a second project (beside the first,
//!   `<folder>-other`), and the Assistant (a scripted stand-in) reusing a
//!   block where one fits, modelling plainly where none does, and undone.
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
    MouseMoveEvent, MouseUpEvent, PlatformInput, ScrollDelta, ScrollWheelEvent, TouchPhase, Window,
    point, px,
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
    /// Click a port on a card: (card, port).
    ClickPort((&'static str, &'static str)),
    /// Shift+click a card: add it to the selection.
    AddCard(&'static str),
    /// Drag a block's row from the Library onto the card at this path.
    DragBlock(&'static str, &'static str),
    /// Click the link to a building block in the Conversation.
    ClickBlockLink(&'static str),
    /// End the process at once, as a crash would: nothing is closed or saved.
    Crash,
}

impl Action {
    /// Frames the action takes.
    fn ticks(&self) -> u64 {
        match self {
            Self::Idle | Self::Key(_) | Self::Crash => 1,
            Self::Text(text) => text.chars().count() as u64,
            Self::Click(_)
            | Self::ClickCard(_)
            | Self::ClickLink(_)
            | Self::ClickPort(_)
            | Self::AddCard(_)
            | Self::ClickBlockLink(_) => 2,
            Self::Fill(_, text) => 4 + text.chars().count() as u64,
            Self::DragPort(..) | Self::DragBlock(..) => 12,
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
                | Self::ClickPort(_)
                | Self::AddCard(_)
                | Self::DragBlock(..)
                | Self::ClickBlockLink(_)
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
    /// The Library is shown, searching for this.
    LibrarySearch(&'static str),
    /// The Library highlights the block with this name.
    LibraryShows(&'static str),
    /// The element exists (a rename may be open, as after inserting a block).
    Inserted(&'static str, ElementKind),
    /// The Surface shows the inside of this definition, under a breadcrumb.
    Opened(&'static str),
    /// Nothing is opened or focused, and this card is selected.
    BackAt(&'static str),
    /// The palette is open in this mode ("Library", "Library fit", "Usages").
    Palette(&'static str),
    /// This dialog is open ("Specialize", "Extract", "Save", "Override").
    DialogOpen(&'static str),
    /// A redefinition in `owner` (or a part redefinition in it) gives
    /// `feature` this value.
    Overridden(&'static str, &'static str, &'static str),
    /// The project's copy of this block has the Library's content.
    UnchangedCopy(&'static str),
    /// My Library holds this block.
    MyLibraryHas(&'static str),
    /// The project with this top-level package is open.
    PackageOpen(&'static str),
    /// The model's text alone reads back without problems.
    SelfContained,
    /// The last turn called these tools.
    AssistantUsed(&'static [&'static str]),
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

/// Scenario H, the Library (C-49), from the URL shortener sample.
fn library(folder: &Path) -> Vec<Step> {
    const SERVICE: &str = "UrlShortener::UrlShortenerService";
    let other = folder.with_file_name(format!(
        "{}-other",
        folder
            .file_name()
            .map_or("project".into(), |n| n.to_string_lossy().to_string())
    ));
    let settle = |step: Step| Step { settle: 12, ..step };
    let mut steps = vec![
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
        step(
            "Create project opens the sample",
            Action::Click("Create project"),
            Check::Exists("UrlShortener::LinkStore", ElementKind::PartDef),
        ),
        step(
            "select the service",
            Action::ClickCard(SERVICE),
            Check::Selected(SERVICE),
        ),
        // H1: open the Library, search, and preview by keyboard.
        shot(
            "01-library",
            step(
                "Ctrl+Shift+L opens the Library",
                Action::Key("ctrl-shift-l"),
                Check::LibrarySearch(""),
            ),
        ),
        step(
            "type a composite's name",
            Action::Text("gateway"),
            Check::LibrarySearch("gateway"),
        ),
        step(
            "the gateway is found first",
            Action::Idle,
            Check::LibraryShows("Gateway"),
        ),
        // H2: insert by keyboard, with the name ready to edit.
        step(
            "Enter inserts it into the selected part",
            Action::Key("enter"),
            Check::Inserted(
                "UrlShortener::UrlShortenerService::gateway",
                ElementKind::Part,
            ),
        ),
        step(
            "its name is ready to edit",
            Action::Idle,
            Check::RenameDialog,
        ),
        step("type a name", Action::Text("front"), Check::RenameDialog),
        step(
            "Enter names it",
            Action::Key("enter"),
            Check::Exists(
                "UrlShortener::UrlShortenerService::front",
                ElementKind::Part,
            ),
        ),
        // H3: what can connect to the gateway's backend.
        step(
            "select the gateway's backend port",
            Action::ClickPort((
                "UrlShortener::UrlShortenerService::front",
                "Library::Services::Gateway::backend",
            )),
            Check::NoDialog,
        ),
        step(
            "Ctrl+K opens the palette",
            Action::Key("ctrl-k"),
            Check::Palette("Commands"),
        ),
        step(
            "find the command",
            Action::Text("What can connect"),
            Check::Palette("Commands"),
        ),
        shot(
            "02-what-fits",
            settle(step(
                "Enter shows only the blocks that fit",
                Action::Key("enter"),
                Check::Palette("Library fit"),
            )),
        ),
        step(
            "type the composite",
            Action::Text("cached"),
            Check::Palette("Library fit"),
        ),
        shot(
            "03-inserted-and-connected",
            settle(step(
                "Enter inserts it beside the gateway, connected",
                Action::Key("enter"),
                Check::Inserted(
                    "UrlShortener::UrlShortenerService::cachedStore",
                    ElementKind::Part,
                ),
            )),
        ),
        step(
            "connected to the gateway's backend",
            Action::Idle,
            Check::Interface("front.backend", "cachedStore.access"),
        ),
        step("name it", Action::Text("sessions"), Check::RenameDialog),
        step(
            "Enter names it",
            Action::Key("enter"),
            Check::Exists(
                "UrlShortener::UrlShortenerService::sessions",
                ElementKind::Part,
            ),
        ),
        // Drag a block from the Library onto the service.
        settle(step(
            "Shift+1 fits the view",
            Action::Key("shift-1"),
            Check::View("Architecture"),
        )),
        step(
            "show the Library again",
            Action::Key("ctrl-shift-l"),
            Check::LibrarySearch("gateway"),
        ),
        step(
            "search for a queue",
            Action::Fill("Library search", "queue".into()),
            Check::LibraryShows("Queue"),
        ),
        shot(
            "04-dropped",
            settle(step(
                "drag it onto the service",
                Action::DragBlock("Queue", SERVICE),
                Check::Inserted(
                    "UrlShortener::UrlShortenerService::queue",
                    ElementKind::Part,
                ),
            )),
        ),
        step("keep its name", Action::Key("escape"), Check::NoDialog),
        // H4: open the composite's definition, then go back.
        step(
            "select the cached store",
            Action::ClickCard("UrlShortener::UrlShortenerService::sessions"),
            Check::Selected("UrlShortener::UrlShortenerService::sessions"),
        ),
        shot(
            "05-inside",
            settle(step(
                "Enter opens its definition",
                Action::Key("enter"),
                Check::Opened("Library::Storage::CachedStore"),
            )),
        ),
        settle(step(
            "Backspace returns to the usage",
            Action::Key("backspace"),
            Check::BackAt("UrlShortener::UrlShortenerService::sessions"),
        )),
        // H5: specialise it and override one inherited value there.
        step("Ctrl+K", Action::Key("ctrl-k"), Check::Palette("Commands")),
        step(
            "Specialise…",
            Action::Text("Specialise"),
            Check::Palette("Commands"),
        ),
        step(
            "Enter asks for the name",
            Action::Key("enter"),
            Check::DialogOpen("Specialize"),
        ),
        step(
            "name the specialisation",
            Action::Fill("Specialisation name", "SessionStore".into()),
            Check::DialogOpen("Specialize"),
        ),
        shot(
            "06-specialised",
            settle(step(
                "Enter creates it and opens it",
                Action::Key("enter"),
                Check::Opened("UrlShortener::SessionStore"),
            )),
        ),
        step(
            "the usage is typed by it",
            Action::Idle,
            Check::TypedBy(
                "UrlShortener::UrlShortenerService::sessions",
                "SessionStore",
            ),
        ),
        step(
            "override the cache's time to live",
            Action::Click("Override cache.ttlSeconds"),
            Check::DialogOpen("Override"),
        ),
        step(
            "type the value",
            Action::Fill("Override value", "60".into()),
            Check::DialogOpen("Override"),
        ),
        shot(
            "07-overridden",
            step(
                "Enter overrides it here only",
                Action::Key("enter"),
                Check::Overridden("UrlShortener::SessionStore", "ttlSeconds", "60"),
            ),
        ),
        step(
            "the original is unchanged",
            Action::Idle,
            Check::UnchangedCopy("Library::Storage::CachedStore"),
        ),
        settle(step(
            "Backspace goes back",
            Action::Key("backspace"),
            Check::BackAt("UrlShortener::UrlShortenerService::sessions"),
        )),
        // Find the usages of the specialisation.
        step(
            "Shift+F12 finds its usages",
            Action::Key("shift-f12"),
            Check::Palette("Usages"),
        ),
        step("Escape closes them", Action::Key("escape"), Check::NoDialog),
        // H6: a building block from a selection.
        step(
            "select the store",
            Action::ClickCard("UrlShortener::UrlShortenerService::store"),
            Check::Selected("UrlShortener::UrlShortenerService::store"),
        ),
        step(
            "and the click statistics",
            Action::AddCard("UrlShortener::UrlShortenerService::clickStats"),
            Check::NoDialog,
        ),
        step("Ctrl+K", Action::Key("ctrl-k"), Check::Palette("Commands")),
        step(
            "Create building block…",
            Action::Text("Create building block"),
            Check::Palette("Commands"),
        ),
        shot(
            "08-boundary",
            step(
                "Enter shows the proposed block first",
                Action::Key("enter"),
                Check::DialogOpen("Extract"),
            ),
        ),
        step(
            "name it",
            Action::Fill("Block name", "LinkData".into()),
            Check::DialogOpen("Extract"),
        ),
        settle(step(
            "Enter creates it in one change",
            Action::Key("enter"),
            Check::Exists(
                "UrlShortener::UrlShortenerService::linkData",
                ElementKind::Part,
            ),
        )),
        step("nothing broke", Action::Idle, Check::NoProblems),
        // H7: save it to My Library, and use it in another project.
        step("Ctrl+K", Action::Key("ctrl-k"), Check::Palette("Commands")),
        step(
            "Save to My Library…",
            Action::Text("Save to My Library"),
            Check::Palette("Commands"),
        ),
        step(
            "Enter asks for a category",
            Action::Key("enter"),
            Check::DialogOpen("Save"),
        ),
        step(
            "a category",
            Action::Fill("Category", "Links".into()),
            Check::DialogOpen("Save"),
        ),
        step(
            "Enter saves it",
            Action::Key("enter"),
            Check::MyLibraryHas("Library::Links::LinkData"),
        ),
        step(
            "Ctrl+N: another project",
            Action::Key("ctrl-n"),
            Check::DialogNewProject,
        ),
        step(
            "its name",
            Action::Fill("Project name", "Other".into()),
            Check::DialogNewProject,
        ),
        step(
            "its folder",
            Action::Fill("Project folder", other.display().to_string()),
            Check::DialogNewProject,
        ),
        settle(step(
            "Create project",
            Action::Click("Create project"),
            Check::PackageOpen("Other"),
        )),
        step(
            "show the Library",
            Action::Key("ctrl-shift-l"),
            Check::LibrarySearch("queue"),
        ),
        step(
            "search for the saved block",
            Action::Fill("Library search", "linkdata".into()),
            Check::LibraryShows("LinkData"),
        ),
        shot(
            "09-other-project",
            settle(step(
                "Enter inserts it",
                Action::Key("enter"),
                Check::Inserted("Other::linkData", ElementKind::Part),
            )),
        ),
        step("keep its name", Action::Key("escape"), Check::NoDialog),
        step(
            "the project has its own copy and stands alone",
            Action::Idle,
            Check::SelfContained,
        ),
        // H8: the Assistant models plainly where no block fits...
        step(
            "ask for something no block fits",
            Action::Fill("Message", "Add a QR code generator for short links.".into()),
            Check::MessageTyped,
        ),
        Step {
            settle: 30,
            ..step(
                "Enter sends; it searches, finds nothing that fits, and models it",
                Action::Key("enter"),
                Check::AssistantDone("Other::QrCodeGenerator"),
            )
        },
        step(
            "no block was bent to fit",
            Action::Idle,
            Check::Missing("Library::Services"),
        ),
        // ... and reuses a block where one fits.
        step(
            "ask for a rate-limited API",
            Action::Fill(
                "Message",
                "Add a public API that lets each client make 100 requests per minute.".into(),
            ),
            Check::MessageTyped,
        ),
        Step {
            settle: 30,
            ..shot(
                "10-assistant-reuses",
                step(
                    "Enter sends; it searches, reads and uses the block",
                    Action::Key("enter"),
                    Check::AssistantDone("Other::Shop::publicApi"),
                ),
            )
        },
        step(
            "through the Library tools",
            Action::Idle,
            Check::AssistantUsed(&["search_library", "read_library_block", "use_library_block"]),
        ),
        step(
            "typed by the block",
            Action::Idle,
            Check::TypedBy("Other::Shop::publicApi", "RateLimitedApi"),
        ),
        step(
            "a link to the block shows it in the Library",
            Action::ClickBlockLink("built-in:Library::Services::RateLimitedApi"),
            Check::LibraryShows("RateLimitedApi"),
        ),
        shot(
            "11-undone",
            step(
                "undo the Assistant's changes",
                Action::Click("Undo the Assistant's changes"),
                Check::Missing("Other::Shop"),
            ),
        ),
        step(
            "the block's copies went with it",
            Action::Idle,
            Check::Missing("Library::Services::RateLimitedApi"),
        ),
    ];
    // Screens with the Library read well at the end too.
    steps.push(shot(
        "12-end",
        step(
            "the Library at the end",
            Action::Key("ctrl-shift-l"),
            Check::LibrarySearch("linkdata"),
        ),
    ));
    steps
}

/// The Assistant for `h-library`: a scripted stand-in (never the network)
/// that models a QR code generator plainly (no block fits), then reuses the
/// rate-limited API block for a rate-limited API.
pub fn script_library_assistant(app: &mut Studio) {
    use serde_json::{Value, json};
    let text = |text: &str| json!({ "type": "text", "text": text });
    let tool = |id: &str, name: &str, input: Value| json!({ "type": "tool_use", "id": id, "name": name, "input": input });
    let reply = |content: Vec<Value>, stop: &str| agq_assistant::Reply {
        content,
        stop_reason: stop.to_string(),
    };
    let replies = vec![
        reply(
            vec![
                text("I'll check the Library first."),
                tool("s1", "search_library", json!({ "query": "qr code" })),
            ],
            "tool_use",
        ),
        reply(
            vec![
                text(
                    "No building block fits a QR code generator, so I'll model it in the project.",
                ),
                tool(
                    "c1",
                    "apply_changes",
                    json!({
                        "description": "Add a QR code generator",
                        "operations": [
                            { "op": "create", "parent": "Other", "kind": "part def", "name": "QrCodeGenerator",
                              "doc": "Turns a short link into a QR code image." }
                        ]
                    }),
                ),
            ],
            "tool_use",
        ),
        reply(
            vec![text(
                "Modelled `Other::QrCodeGenerator`. No Library block fits it, so it is a plain project definition.",
            )],
            "end_turn",
        ),
        reply(
            vec![tool(
                "s2",
                "search_library",
                json!({ "query": "rate limit" }),
            )],
            "tool_use",
        ),
        reply(
            vec![tool(
                "r1",
                "read_library_block",
                json!({ "block": "built-in:Library::Services::RateLimitedApi" }),
            )],
            "tool_use",
        ),
        reply(
            vec![
                text(
                    "The rate-limited API block fits: a gateway whose requests pass a rate limiter.",
                ),
                tool(
                    "c2",
                    "apply_changes",
                    json!({
                        "description": "Add the shop system",
                        "operations": [ { "op": "create", "parent": "Other", "kind": "part def", "name": "Shop" } ]
                    }),
                ),
            ],
            "tool_use",
        ),
        reply(
            vec![tool(
                "u1",
                "use_library_block",
                json!({
                    "block": "built-in:Library::Services::RateLimitedApi",
                    "parent": "Other::Shop",
                    "name": "publicApi",
                    "values": { "limiter.limitPerMinute": 100 }
                }),
            )],
            "tool_use",
        ),
        reply(
            vec![text(
                "`built-in:Library::Services::RateLimitedApi` matches, so I used it as `Other::Shop::publicApi` with a limit of 100 requests per minute.",
            )],
            "end_turn",
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
    /// What the Library's search box holds.
    pub library_search: String,
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
    /// Wheel turns the current step took to bring its button into view.
    scrolled: u32,
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
                "h-library" => library(&folder),
                "a-reopen" => reopen(),
                other => return Err(format!("no journey is called {other}")),
            },
            index: 0,
            age: 0,
            frames: 0,
            point: None,
            capture: None,
            last_pointer: None,
            scrolled: 0,
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
        // A button below the window (the welcome at 150% or 200%) is
        // scrolled into view first, with the wheel, as the Operator would.
        if self.age == 0
            && let Action::Click(name) = &step.action
            && let Some(bounds) = target::find(name)
        {
            // Above the status bar, which grows with the UI scale; only a
            // button whose middle is hidden is scrolled to.
            let height = window.viewport_size().height - window.rem_size() * 1.75;
            if bounds.center().y > height {
                if self.scrolled >= 40 {
                    return Err(format!(
                        "step {} “{}”: {name} stays below the window",
                        self.index + 1,
                        step.name
                    ));
                }
                self.scrolled += 1;
                let by = (bounds.bottom() - height + window.rem_size()).min(px(240.0));
                window.dispatch_event(
                    PlatformInput::ScrollWheel(ScrollWheelEvent {
                        position: point(bounds.center().x, height * 0.5),
                        delta: ScrollDelta::Pixels(point(px(0.0), -by)),
                        modifiers: Modifiers::default(),
                        touch_phase: TouchPhase::Moved,
                    }),
                    cx,
                );
                return Ok(Outcome::Running);
            }
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
                    self.scrolled = 0;
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
            Action::ClickPort(port) => {
                let at = match self.point {
                    Some((p, _)) => p,
                    None => {
                        let p = port_point(studio.read(cx), *port)?;
                        self.point = Some((p, p));
                        p
                    }
                };
                click(at, tick == 0, window, cx);
            }
            Action::AddCard(path) => {
                let at = match self.point {
                    Some((p, _)) => p,
                    None => {
                        let p = card_point(studio.read(cx), path)?;
                        self.point = Some((p, p));
                        p
                    }
                };
                let shift = Modifiers {
                    shift: true,
                    ..Modifiers::default()
                };
                if tick == 0 {
                    move_to(at, None, window, cx);
                }
                let input = if tick == 0 {
                    PlatformInput::MouseDown(MouseDownEvent {
                        button: MouseButton::Left,
                        position: at,
                        modifiers: shift,
                        click_count: 1,
                        first_mouse: false,
                    })
                } else {
                    PlatformInput::MouseUp(MouseUpEvent {
                        button: MouseButton::Left,
                        position: at,
                        modifiers: shift,
                        click_count: 1,
                    })
                };
                window.dispatch_event(input, cx);
            }
            Action::ClickBlockLink(block) => {
                let at = target::find(&format!("Block {block}"))
                    .ok_or_else(|| format!("the link to {block} is not shown"))?
                    .center();
                click(at, tick == 0, window, cx);
            }
            Action::DragBlock(row, card) => {
                let (a, b) = match self.point {
                    Some(points) => points,
                    None => {
                        let a = target::find(&format!("Block {row}"))
                            .ok_or_else(|| format!("{row} is not listed in the Library"))?
                            .center();
                        let points = (a, card_point(studio.read(cx), card)?);
                        self.point = Some(points);
                        points
                    }
                };
                let last = action.ticks() - 1;
                if tick == 0 {
                    move_to(a, None, window, cx);
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
    let preferred = if node.is_container {
        Point::new(node.bounds.center().x, node.bounds.min.y + 26.0)
    } else {
        Point::new(node.bounds.center().x, node.bounds.min.y + 40.0)
    };
    let error = match screen(app, preferred) {
        Ok(p) => return Ok(p),
        Err(error) => error,
    };
    // Only part of the card is in view (a narrow Surface at 200%): click the
    // part that is, as the Operator would.
    let viewport = target::find("Viewport")
        .ok_or("the Surface is not shown")?
        .dilate(px(-4.0));
    let at = |world: Point| {
        let local = app.camera.world_to_screen(world);
        point(
            viewport.origin.x + px(local.x),
            viewport.origin.y + px(local.y),
        )
    };
    let shown =
        gpui::Bounds::from_corners(at(node.bounds.min), at(node.bounds.max)).intersect(&viewport);
    if shown.size.width < px(12.0) || shown.size.height < px(12.0) {
        return Err(error);
    }
    let y = at(preferred)
        .y
        .max(shown.origin.y + px(4.0))
        .min(shown.bottom() - px(4.0));
    Ok(point(shown.center().x, y))
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
        Check::LibrarySearch(query) => {
            if app.left != crate::studio::LeftTab::Library
                || app.outline_hidden
                || app.panels_hidden
            {
                return fail("the Library is not shown");
            }
            if views.library_search != *query {
                return Err(format!(
                    "the Library searches for {:?}, not {query:?}",
                    views.library_search
                ));
            }
        }
        Check::LibraryShows(name) => {
            let shown = app
                .library
                .selected
                .as_ref()
                .and_then(|block| app.library.index.find(block))
                .map(|block| block.name.clone());
            if shown.as_deref() != Some(*name) {
                return Err(format!("the Library shows {shown:?}, not {name}"));
            }
        }
        Check::Inserted(path, kind) => {
            let id = find(app, path)?;
            if tree(app)?[id].kind != *kind {
                return fail("the element has the wrong kind");
            }
        }
        Check::Opened(path) => {
            let id = find(app, path)?;
            if app.focus != Some(id) || app.drill.is_empty() {
                return Err(format!(
                    "the Surface does not show the inside of {path} (focus {:?})",
                    app.focus
                ));
            }
        }
        Check::BackAt(path) => {
            let id = find(app, path)?;
            if app.focus.is_some() || !app.drill.is_empty() {
                return fail("a definition is still open");
            }
            if app.selection.primary != Some(SceneTarget::Node(id))
                && app.selection.primary != Some(SceneTarget::Container(id))
            {
                return fail("the card it came from is not selected");
            }
        }
        Check::Palette(mode) => {
            let open = views.palette.as_ref().map(|(m, _)| match m {
                PaletteMode::Library { fit: true } => "Library fit",
                PaletteMode::Library { fit: false } => "Library",
                PaletteMode::Usages(_) => "Usages",
                PaletteMode::Elements => "Elements",
                PaletteMode::Commands => "Commands",
            });
            if open != Some(*mode) {
                return Err(format!("the palette shows {open:?}, not {mode}"));
            }
        }
        Check::DialogOpen(kind) => {
            let open = match &app.dialog {
                Some(Dialog::Specialize { .. }) => "Specialize",
                Some(Dialog::ExtractBlock { .. }) => "Extract",
                Some(Dialog::SaveToLibrary { .. }) => "Save",
                Some(Dialog::Override { .. }) => "Override",
                Some(Dialog::LibraryConflict { .. }) => "Conflict",
                Some(_) => "another",
                None => "none",
            };
            if open != *kind {
                return Err(format!("the open dialog is {open}, not {kind}"));
            }
        }
        Check::Overridden(owner, feature, value) => {
            let tree = tree(app)?;
            let owner = find(app, owner)?;
            let found = tree.descendants(owner).into_iter().any(|id| {
                let e = &tree[id];
                !e.redefines.is_empty()
                    && e.redefines[0].last_name() == *feature
                    && e.value.as_ref().is_some_and(|v| v.to_string() == *value)
            });
            if !found {
                return fail("no override gives that value");
            }
            if app.dialog.is_some() {
                return fail("a dialog is still open");
            }
        }
        Check::UnchangedCopy(path) => {
            let id = find(app, path)?;
            let index = &app.library.index;
            let block = index
                .project_block(id)
                .and_then(|i| index.get(i))
                .ok_or("the copy is not in the Library's index")?;
            if block.origin.is_none_or(|o| o.changed) {
                return Err(format!("the copy reads as {}", block.source_label()));
            }
        }
        Check::MyLibraryHas(qualified) => {
            if app.library.source.mine().find(qualified).is_none() {
                return Err(format!("My Library has no {qualified}"));
            }
            let path = app
                .library
                .source
                .mine_path()
                .ok_or("My Library is not kept")?;
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let name = qualified.rsplit("::").next().unwrap_or(qualified);
            if !text.contains(&format!("part def {name}")) {
                return fail("My Library's file does not hold it");
            }
        }
        Check::PackageOpen(name) => {
            find(app, name)?;
            if app.dialog.is_some() {
                return fail("a dialog is still open");
            }
        }
        Check::SelfContained => {
            let tree = tree(app)?;
            let text: String = agq_language::print(tree)
                .into_iter()
                .map(|source| source.text)
                .collect::<Vec<_>>()
                .join("\n");
            let alone = agq_language::parse(&[agq_language::Source::new("alone.sysml", &text)]);
            if let Some(problem) = agq_language::validate(&alone).first() {
                return Err(format!(
                    "the model does not stand alone: {}",
                    problem.message
                ));
            }
        }
        Check::AssistantUsed(names) => {
            if app.conversation.running() {
                return fail("the Assistant is still working");
            }
            let mut called: Vec<String> = Vec::new();
            for entry in app.conversation.conversation.entries.iter().rev() {
                match entry {
                    agq_assistant::Entry::Operator { .. } => break,
                    agq_assistant::Entry::Assistant { parts, .. } => {
                        for part in parts {
                            if let agq_providers::AssistantPart::ToolCall { name, .. } = part {
                                called.push(name.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            for name in *names {
                if !called.iter().any(|c| c == name) {
                    return Err(format!(
                        "the last turn did not call {name} (called {called:?})"
                    ));
                }
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
