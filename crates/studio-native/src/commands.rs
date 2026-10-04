//! Every Studio action, with its shortcut, for the menus, the command palette
//! and the keyboard. The keystrokes are GPUI's (`ctrl-z`, `shift-1`); they
//! are bound in `bind` below.
use gpui::{Action, App, KeyBinding};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandId {
    Architecture,
    Graph,
    Requirements,
    Fit,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomToSelection,
    GoToElement,
    ShortcutHelp,
    Focus,
    LeaveFocus,
    Collapse,
    Pin,
    Unpin,
    CreatePart,
    CreatePort,
    CreateItem,
    CreateAttribute,
    CreateInterface,
    CreateRequirement,
    Rename,
    Delete,
    Connect,
    MoveTo,
    Lock,
    Undo,
    Redo,
    Checkpoint,
    History,
    NewProject,
    OpenProject,
    DevelopAgentique,
    Palette,
    Theme,
    Contrast,
    ReducedMotion,
    AskAssistant,
    InsertSelection,
    NewConversation,
    ShowConversation,
    Settings,
    HidePanels,
    ShowOutline,
    ShowInspector,
    ShowLibrary,
    InsertFromLibrary,
    ConnectFromLibrary,
    OpenDefinition,
    FindUsages,
    Specialize,
    CreateBlock,
    SaveToLibrary,
    ShowScenarios,
    NewScenario,
    RunScenario,
    StopRun,
    TraceFirst,
    TraceBack,
    TracePlay,
    TraceForward,
    TraceLast,
    CheckImplementation,
    TrustLocal,
    StartObjective,
    MessageObjective,
    PauseObjective,
    StepObjective,
    ResumeObjective,
    StopObjective,
    ContinueObjective,
}

pub struct Command {
    pub id: CommandId,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub description: &'static str,
    /// The keystroke that runs the command when no text field has focus
    /// (GPUI syntax).
    pub key: Option<&'static str>,
}

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::CreatePart,
        label: "Create part",
        shortcut: "P",
        description: "Add a part inside the selected element, or at the top level",
        key: Some("p"),
    },
    Command {
        id: CommandId::CreatePort,
        label: "Create port",
        shortcut: "O",
        description: "Add a port to the selected part or definition",
        key: Some("o"),
    },
    Command {
        id: CommandId::CreateItem,
        label: "Create item",
        shortcut: "I",
        description: "Add an item: something that flows between parts",
        key: Some("i"),
    },
    Command {
        id: CommandId::CreateAttribute,
        label: "Create attribute",
        shortcut: "A",
        description: "Add an attribute (a value) to the selected element",
        key: Some("a"),
    },
    Command {
        id: CommandId::CreateInterface,
        label: "Create interface definition",
        shortcut: "N",
        description: "Add an interface definition with two port ends",
        key: Some("n"),
    },
    Command {
        id: CommandId::CreateRequirement,
        label: "Create requirement",
        shortcut: "R",
        description: "Add a requirement inside the selected element, or at the top level",
        key: Some("r"),
    },
    Command {
        id: CommandId::Rename,
        label: "Rename",
        shortcut: "F2",
        description: "Rename the selected element; references keep pointing at it",
        key: Some("f2"),
    },
    Command {
        id: CommandId::Delete,
        label: "Delete",
        shortcut: "Del",
        description: "Delete the selected elements and everything they own",
        key: Some("delete"),
    },
    Command {
        id: CommandId::Connect,
        label: "Connect",
        shortcut: "C",
        description: "Connect the two selected ports or parts",
        key: Some("c"),
    },
    Command {
        id: CommandId::MoveTo,
        label: "Move to…",
        shortcut: "M",
        description: "Move the selected element into another owner",
        key: Some("m"),
    },
    Command {
        id: CommandId::Lock,
        label: "Lock or unlock",
        shortcut: "L",
        description: "A locked element changes only after you confirm",
        key: Some("l"),
    },
    Command {
        id: CommandId::Undo,
        label: "Undo",
        shortcut: "Ctrl+Z",
        description: "Undo the last change",
        key: Some("ctrl-z"),
    },
    Command {
        id: CommandId::Redo,
        label: "Redo",
        shortcut: "Ctrl+Y",
        description: "Redo the last undone change",
        key: Some("ctrl-y"),
    },
    Command {
        id: CommandId::Checkpoint,
        label: "Checkpoint",
        shortcut: "Ctrl+S",
        description: "Record the current model in the history with a message",
        key: Some("ctrl-s"),
    },
    Command {
        id: CommandId::History,
        label: "Show history",
        shortcut: "H",
        description: "Checkpoints, and what changed between them",
        key: Some("h"),
    },
    Command {
        id: CommandId::Architecture,
        label: "Architecture view",
        shortcut: "1",
        description: "Containment, ports and connections",
        key: Some("1"),
    },
    Command {
        id: CommandId::Graph,
        label: "Graph view",
        shortcut: "2",
        description: "Elements layered by their relationships",
        key: Some("2"),
    },
    Command {
        id: CommandId::Requirements,
        label: "Requirements view",
        shortcut: "3",
        description: "Requirements, what satisfies them and their subjects",
        key: Some("3"),
    },
    Command {
        id: CommandId::Fit,
        label: "Fit to view",
        shortcut: "Shift+1",
        description: "Show the whole model (also Home)",
        key: Some("shift-1"),
    },
    Command {
        id: CommandId::ZoomIn,
        label: "Zoom in",
        shortcut: "+",
        description: "Closer, around the middle of the Surface (also =)",
        key: Some("="),
    },
    Command {
        id: CommandId::ZoomOut,
        label: "Zoom out",
        shortcut: "-",
        description: "Further, around the middle of the Surface",
        key: Some("-"),
    },
    Command {
        id: CommandId::ZoomReset,
        label: "Zoom to 100%",
        shortcut: "Shift+0",
        description: "Cards at their own size",
        key: Some("shift-0"),
    },
    Command {
        id: CommandId::ZoomToSelection,
        label: "Zoom to selection",
        shortcut: "Shift+2",
        description: "Move the camera to the selected element",
        key: Some("shift-2"),
    },
    Command {
        id: CommandId::GoToElement,
        label: "Go to element",
        shortcut: "Ctrl+P",
        description: "Find an element by name and show it",
        key: Some("ctrl-p"),
    },
    Command {
        id: CommandId::ShortcutHelp,
        label: "Keyboard shortcuts",
        shortcut: "?",
        description: "Every command and its shortcut, in Settings",
        key: Some("?"),
    },
    Command {
        id: CommandId::Focus,
        label: "Focus selection",
        shortcut: "F",
        description: "Show only the selected element and what it owns",
        key: Some("f"),
    },
    Command {
        id: CommandId::LeaveFocus,
        label: "Back",
        shortcut: "Backspace",
        description: "Back to what the Surface showed before a definition was opened or an element focused (also Alt+Left)",
        key: Some("backspace"),
    },
    Command {
        id: CommandId::Collapse,
        label: "Collapse or expand",
        shortcut: "X",
        description: "Hide or show what the selected container owns",
        key: Some("x"),
    },
    Command {
        id: CommandId::Pin,
        label: "Pin position",
        shortcut: "",
        description: "Keep this card where it is in the graph view",
        key: None,
    },
    Command {
        id: CommandId::Unpin,
        label: "Unpin position",
        shortcut: "",
        description: "Let the graph layout place this card again",
        key: None,
    },
    Command {
        id: CommandId::NewProject,
        label: "New project…",
        shortcut: "Ctrl+N",
        description: "Create a project in a new folder",
        key: Some("ctrl-n"),
    },
    Command {
        id: CommandId::OpenProject,
        label: "Open project…",
        shortcut: "Ctrl+O",
        description: "Open a project folder",
        key: Some("ctrl-o"),
    },
    Command {
        id: CommandId::DevelopAgentique,
        label: "Develop Agentique",
        shortcut: "",
        description: "Open Agentique's own repository as a project: its architecture, workflows, code and checks",
        key: None,
    },
    Command {
        id: CommandId::Palette,
        label: "Command palette",
        shortcut: "Ctrl+K",
        description: "Find any command or element",
        key: Some("ctrl-k"),
    },
    Command {
        id: CommandId::Theme,
        label: "Switch light or dark theme",
        shortcut: "",
        description: "Presentation only",
        key: None,
    },
    Command {
        id: CommandId::Contrast,
        label: "Switch high contrast",
        shortcut: "",
        description: "Presentation only",
        key: None,
    },
    Command {
        id: CommandId::ReducedMotion,
        label: "Switch reduced motion",
        shortcut: "",
        description: "Camera moves and highlights without animation",
        key: None,
    },
    Command {
        id: CommandId::AskAssistant,
        label: "Ask the Assistant",
        shortcut: "Ctrl+L",
        description: "Write a message in the Conversation",
        key: Some("ctrl-l"),
    },
    Command {
        id: CommandId::InsertSelection,
        label: "Insert selection into the message",
        shortcut: "Ctrl+I",
        description: "Refer to the selected elements in the Conversation",
        key: Some("ctrl-i"),
    },
    Command {
        id: CommandId::NewConversation,
        label: "New conversation",
        shortcut: "",
        description: "Start a new conversation with the Assistant; the model is unaffected",
        key: None,
    },
    Command {
        id: CommandId::ShowConversation,
        label: "Show or hide the conversation",
        shortcut: "Ctrl+J",
        description: "The Conversation column on the right",
        key: Some("ctrl-j"),
    },
    Command {
        id: CommandId::Settings,
        label: "Settings",
        shortcut: "Ctrl+,",
        description: "Keys, the Assistant's model and appearance",
        key: Some("ctrl-,"),
    },
    Command {
        id: CommandId::HidePanels,
        label: "Show or hide the panels",
        shortcut: "Ctrl+\\",
        description: "Focus mode: the Surface takes the whole window; remembered per project",
        key: Some("ctrl-\\"),
    },
    Command {
        id: CommandId::ShowInspector,
        label: "Show or hide the Inspector",
        shortcut: "Ctrl+Alt+B",
        description: "The Inspector, Requirements and History column; remembered per project",
        key: Some("ctrl-alt-b"),
    },
    Command {
        id: CommandId::ShowOutline,
        label: "Show or hide the Outline",
        shortcut: "Ctrl+B",
        description: "The list of elements on the left; remembered per project",
        key: Some("ctrl-b"),
    },
    Command {
        id: CommandId::ShowLibrary,
        label: "Library",
        shortcut: "Ctrl+Shift+L",
        description: "Reusable building blocks: search them, preview them and add them",
        key: Some("ctrl-shift-l"),
    },
    Command {
        id: CommandId::InsertFromLibrary,
        label: "Insert from Library…",
        shortcut: "Shift+A",
        description: "Find a building block and add a usage of it to the selected part",
        key: Some("shift-a"),
    },
    Command {
        id: CommandId::ConnectFromLibrary,
        label: "What can connect here?",
        shortcut: "",
        description: "Building blocks with a port that fits the selected port; add one and connect it",
        key: None,
    },
    Command {
        id: CommandId::OpenDefinition,
        label: "Open definition",
        shortcut: "Enter",
        description: "Show the inside of the selected usage's definition; Backspace goes back (also F12)",
        key: Some("enter"),
    },
    Command {
        id: CommandId::FindUsages,
        label: "Find usages",
        shortcut: "Shift+F12",
        description: "Every usage and specialisation of the selected definition",
        key: Some("shift-f12"),
    },
    Command {
        id: CommandId::Specialize,
        label: "Specialise…",
        shortcut: "",
        description: "A new definition that specialises the selected one, for a variant; the original stays as it is",
        key: None,
    },
    Command {
        id: CommandId::CreateBlock,
        label: "Create building block from selection…",
        shortcut: "",
        description: "Turn the selected parts into a reusable definition and one usage of it",
        key: None,
    },
    Command {
        id: CommandId::SaveToLibrary,
        label: "Save to My Library…",
        shortcut: "",
        description: "Keep the selected definition for use in other projects",
        key: None,
    },
    Command {
        id: CommandId::ShowScenarios,
        label: "Scenarios",
        shortcut: "Ctrl+Shift+R",
        description: "The project's scenarios and their newest results",
        key: Some("ctrl-shift-r"),
    },
    Command {
        id: CommandId::NewScenario,
        label: "New scenario…",
        shortcut: "",
        description: "A scenario for the selected part: what goes in, how its parts answer, what must come out",
        key: None,
    },
    Command {
        id: CommandId::RunScenario,
        label: "Run scenario",
        shortcut: "F5",
        description: "Run the chosen scenario in the chosen mode",
        key: Some("f5"),
    },
    Command {
        id: CommandId::StopRun,
        label: "Stop run",
        shortcut: "Shift+F5",
        description: "Stop the run in progress; it ends as cancelled",
        key: Some("shift-f5"),
    },
    Command {
        id: CommandId::TraceFirst,
        label: "Trace: first event",
        shortcut: "",
        description: "Show the first event of the trace",
        key: None,
    },
    Command {
        id: CommandId::TraceBack,
        label: "Trace: step back",
        shortcut: "[",
        description: "Show the event before; it undoes nothing the run did",
        key: Some("["),
    },
    Command {
        id: CommandId::TracePlay,
        label: "Trace: play or pause",
        shortcut: "\\",
        description: "Play the trace on the Surface, one event at a time",
        key: Some("\\"),
    },
    Command {
        id: CommandId::TraceForward,
        label: "Trace: step forward",
        shortcut: "]",
        description: "Show the next event of the trace",
        key: Some("]"),
    },
    Command {
        id: CommandId::TraceLast,
        label: "Trace: last event",
        shortcut: "",
        description: "Show where the run ended",
        key: None,
    },
    Command {
        id: CommandId::CheckImplementation,
        label: "Check the implementation",
        shortcut: "",
        description: "Module boundaries, contract shapes and linked tests against the model",
        key: None,
    },
    Command {
        id: CommandId::TrustLocal,
        label: "Trusted-local execution…",
        shortcut: "",
        description: "Allow or stop builds, tests and the harness running on this computer for this project",
        key: None,
    },
    // Objectives (C-54): the same commands in the Conversation, the
    // Objectives panel and the palette.
    Command {
        id: CommandId::StartObjective,
        label: "Start as objective…",
        shortcut: "Ctrl+Enter",
        description: "Show the start form in the Conversation with your message as the intent, its budgets, permissions and each role's model; nothing starts until you press Start (Ctrl+Enter in the message)",
        key: None,
    },
    Command {
        id: CommandId::MessageObjective,
        label: "Write to the objective",
        shortcut: "",
        description: "Address your messages to the objective's agents instead of the Assistant; they go into its thread",
        key: None,
    },
    Command {
        id: CommandId::PauseObjective,
        label: "Pause the objective",
        shortcut: "",
        description: "Its agents hold at their next tool call, the Orchestrator before its next phase",
        key: None,
    },
    Command {
        id: CommandId::StepObjective,
        label: "Step the objective",
        shortcut: "",
        description: "Let the paused objective take one tool call, or one phase, then hold again",
        key: None,
    },
    Command {
        id: CommandId::ResumeObjective,
        label: "Resume the objective",
        shortcut: "",
        description: "Let the paused objective go on",
        key: None,
    },
    Command {
        id: CommandId::StopObjective,
        label: "Stop the objective",
        shortcut: "",
        description: "End the objective and its child objectives: their sessions stop, their records stay",
        key: None,
    },
    Command {
        id: CommandId::ContinueObjective,
        label: "Continue the objective",
        shortcut: "",
        description: "Go on with the objective that is not finished, from the phase it reached",
        key: None,
    },
];

/// Runs a command: the one action every shortcut, menu and palette row
/// dispatches.
#[derive(Clone, PartialEq, Action)]
#[action(namespace = studio, no_json)]
pub struct Run(pub CommandId);

gpui::actions!(
    studio,
    [
        /// Clears the selection on the Surface (Escape).
        Deselect,
        /// Selects the nearest card that way (the arrow keys).
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
    ]
);

/// Commands whose shortcut also works while a text field has the focus:
/// they never take a key a text field needs.
const WHILE_TYPING: &[CommandId] = &[
    CommandId::Palette,
    CommandId::GoToElement,
    CommandId::Settings,
    CommandId::ShowConversation,
    CommandId::AskAssistant,
    CommandId::InsertSelection,
    CommandId::HidePanels,
    CommandId::ShowOutline,
    CommandId::ShowInspector,
    CommandId::NewProject,
    CommandId::OpenProject,
    CommandId::Checkpoint,
    CommandId::ShowLibrary,
    CommandId::ShowScenarios,
    CommandId::RunScenario,
    CommandId::StopRun,
];

/// Further keys for commands that have a shortcut of their own.
pub const ALIASES: &[(&str, CommandId)] = &[
    ("home", CommandId::Fit),
    ("shift-=", CommandId::ZoomIn),
    ("+", CommandId::ZoomIn),
    ("ctrl-shift-z", CommandId::Redo),
    ("alt-left", CommandId::LeaveFocus),
    ("f12", CommandId::OpenDefinition),
];

/// Where the Studio's shortcuts apply: anywhere in the workspace, except
/// while a text field has the focus (GPUI's `!Input` looks at the whole
/// focus path).
pub const SURFACE_KEYS: &str = "Workspace && !Input";
pub const GLOBAL_KEYS: &str = "Workspace";

/// Binds every shortcut.
pub fn bind(cx: &mut App) {
    let mut bindings = Vec::new();
    for command in COMMANDS {
        if let Some(key) = command.key {
            let context = if WHILE_TYPING.contains(&command.id) {
                GLOBAL_KEYS
            } else {
                SURFACE_KEYS
            };
            bindings.push(KeyBinding::new(key, Run(command.id), Some(context)));
        }
    }
    for (key, id) in ALIASES {
        bindings.push(KeyBinding::new(key, Run(*id), Some(SURFACE_KEYS)));
    }
    bindings.extend([
        KeyBinding::new("escape", Deselect, Some(SURFACE_KEYS)),
        KeyBinding::new("left", SelectLeft, Some(SURFACE_KEYS)),
        KeyBinding::new("right", SelectRight, Some(SURFACE_KEYS)),
        KeyBinding::new("up", SelectUp, Some(SURFACE_KEYS)),
        KeyBinding::new("down", SelectDown, Some(SURFACE_KEYS)),
    ]);
    cx.bind_keys(bindings);
}

pub fn command(id: CommandId) -> &'static Command {
    COMMANDS
        .iter()
        .find(|command| command.id == id)
        .expect("every command is registered")
}

/// What a command's availability depends on.
#[derive(Clone, Copy, Debug, Default)]
pub struct CommandContext {
    /// A dialog is waiting for the Operator.
    pub busy: bool,
    /// A project is open (the Conversation belongs to it).
    pub project: bool,
    /// A project is open (fixtures are read-only).
    pub editable: bool,
    /// Something is selected.
    pub selected: bool,
    /// Two ports or cards are selected.
    pub pair: bool,
    /// The selection is a card that can own members.
    pub container: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub graph_view: bool,
    pub focused: bool,
    /// A port is selected.
    pub port: bool,
    /// The selection is a definition, or a usage typed by one.
    pub definition: bool,
    /// One or more parts are selected, and nothing else.
    pub parts: bool,
    /// A scenario is chosen.
    pub scenario: bool,
    /// A run is in progress.
    pub running: bool,
    /// A result with a trace is shown.
    pub trace: bool,
    /// An objective is shown (running, or the last one), running, paused
    /// (or about to be), or not finished and not running (C-54).
    pub objective: bool,
    pub objective_running: bool,
    pub objective_paused: bool,
    pub objective_unfinished: bool,
}

/// Why a command cannot run now, in plain words; `None` when it can.
pub fn unavailable(id: CommandId, context: &CommandContext) -> Option<&'static str> {
    use CommandId::*;
    match id {
        _ if context.busy
            && !matches!(
                id,
                Palette
                    | Theme
                    | Contrast
                    | ReducedMotion
                    | Settings
                    | PauseObjective
                    | StepObjective
                    | ResumeObjective
                    | StopObjective
            ) =>
        {
            Some("Finish or cancel the open dialog first")
        }
        CreatePart | CreatePort | CreateItem | CreateAttribute | CreateInterface
        | CreateRequirement | Rename | Delete | Connect | MoveTo | Lock | Undo | Redo
        | Checkpoint | InsertFromLibrary | ConnectFromLibrary | Specialize | CreateBlock
            if !context.editable =>
        {
            Some("Open or create a project to edit")
        }
        ZoomToSelection if !context.selected => Some("Select an element first"),
        CreatePort | CreateAttribute if !context.container => {
            Some("Select the part or definition to add it to")
        }
        Rename | Delete | MoveTo | Lock | Focus | Collapse if !context.selected => {
            Some("Select an element first")
        }
        Connect if !context.pair => Some("Select two ports or two parts (Shift+click)"),
        Undo if !context.can_undo => Some("Nothing to undo"),
        Redo if !context.can_redo => Some("Nothing to redo"),
        Pin | Unpin if !context.graph_view || !context.selected => {
            Some("Select a card in the graph view")
        }
        LeaveFocus if !context.focused => Some("Nothing is open or focused"),
        ConnectFromLibrary if !context.port => Some("Select a port first"),
        OpenDefinition | FindUsages | Specialize | SaveToLibrary if !context.definition => {
            Some("Select a definition, or a usage of one")
        }
        SaveToLibrary if !context.project => Some("Open a project first"),
        CreateBlock if !context.parts => Some("Select one or more parts that share an owner"),
        AskAssistant | InsertSelection | NewConversation | ShowConversation if !context.project => {
            Some("Open or create a project to work with the Assistant")
        }
        InsertSelection if !context.selected => Some("Select an element first"),
        NewScenario if !context.editable => Some("Open or create a project to edit"),
        ShowScenarios | CheckImplementation | TrustLocal if !context.project => {
            Some("Open a project first")
        }
        RunScenario if !context.scenario => Some("Choose a scenario in the Scenarios tab"),
        RunScenario if context.running => Some("A run is in progress"),
        StopRun if !context.running => Some("Nothing is running"),
        TraceFirst | TraceBack | TracePlay | TraceForward | TraceLast if !context.trace => {
            Some("Run a scenario first")
        }
        StartObjective if context.objective_running || context.objective_unfinished => {
            Some("An objective is not finished: one runs at a time")
        }
        MessageObjective if !context.objective => Some("No objective is shown"),
        MessageObjective if !context.project => {
            Some("Open or create a project to use the Conversation")
        }
        PauseObjective if !context.objective_running => Some("No objective is running"),
        PauseObjective if context.objective_paused => Some("The objective is paused"),
        StepObjective | ResumeObjective if !context.objective_paused => {
            Some("The objective is not paused")
        }
        StopObjective if !context.objective_running && !context.objective_unfinished => {
            Some("No objective is running")
        }
        ContinueObjective if !context.objective_unfinished => {
            Some("No objective waits to continue")
        }
        _ => None,
    }
}

pub fn search(query: &str) -> impl Iterator<Item = &'static Command> {
    let mut found: Vec<_> = COMMANDS
        .iter()
        .filter_map(|command| {
            let haystack = format!("{} {}", command.label, command.description);
            fuzzy_score(query, &haystack).map(|score| (score, command))
        })
        .collect();
    found.sort_by_key(|(score, _)| *score);
    found.into_iter().map(|(_, command)| command)
}

pub fn fuzzy_score(query: &str, text: &str) -> Option<usize> {
    let query = query.trim().to_lowercase();
    let text = text.to_lowercase();
    if query.is_empty() {
        return Some(0);
    }
    if let Some(index) = text.find(&query) {
        return Some(index);
    }
    let mut cursor = 0;
    let chars: Vec<_> = text.chars().collect();
    let mut penalty = 1000;
    for needle in query.chars().filter(|character| !character.is_whitespace()) {
        let position = chars[cursor..]
            .iter()
            .position(|character| *character == needle)?;
        penalty += position;
        cursor += position + 1;
    }
    Some(penalty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_finds_abbreviations_and_exact_matches_first() {
        assert_eq!(search("grph").next().unwrap().id, CommandId::Graph);
        assert_eq!(
            search("create port").next().unwrap().id,
            CommandId::CreatePort
        );
        assert!(fuzzy_score("μs", "周期 μs").is_some());
        assert!(fuzzy_score("missing", "Part").is_none());
    }

    #[test]
    fn edits_need_a_project_and_the_right_selection() {
        let fixture = CommandContext::default();
        assert!(unavailable(CommandId::CreatePart, &fixture).is_some());
        let project = CommandContext {
            editable: true,
            ..Default::default()
        };
        assert!(unavailable(CommandId::CreatePart, &project).is_none());
        assert!(unavailable(CommandId::CreatePort, &project).is_some());
        assert!(unavailable(CommandId::Connect, &project).is_some());
        assert!(unavailable(CommandId::Undo, &project).is_some());
    }

    #[test]
    fn every_shortcut_is_unique() {
        let keys: Vec<_> = COMMANDS
            .iter()
            .filter_map(|c| c.key)
            .chain(ALIASES.iter().map(|(key, _)| *key))
            .collect();
        for (i, a) in keys.iter().enumerate() {
            assert!(!keys[i + 1..].contains(a), "{a:?} is bound twice");
        }
    }

    #[test]
    fn every_shortcut_parses_and_single_keys_never_fire_while_typing() {
        for command in COMMANDS {
            let Some(key) = command.key else { continue };
            assert!(gpui::Keystroke::parse(key).is_ok(), "{key}");
            // A function key types nothing, so it may work while typing.
            let base = key.rsplit('-').next().unwrap_or(key);
            let function_key = base.len() > 1
                && base.starts_with('f')
                && base[1..].chars().all(|c| c.is_ascii_digit());
            let plain = !key.contains("ctrl-") && !key.contains("alt-") && !function_key;
            assert!(
                !(plain && WHILE_TYPING.contains(&command.id)),
                "{key} would take a key from a text field"
            );
        }
    }
}
