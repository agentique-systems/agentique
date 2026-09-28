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
        label: "Leave focus",
        shortcut: "Backspace",
        description: "Show the whole model again",
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
];

/// Further keys for commands that have a shortcut of their own.
const ALIASES: &[(&str, CommandId)] = &[
    ("home", CommandId::Fit),
    ("shift-=", CommandId::ZoomIn),
    ("+", CommandId::ZoomIn),
    ("ctrl-shift-z", CommandId::Redo),
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
}

/// Why a command cannot run now, in plain words; `None` when it can.
pub fn unavailable(id: CommandId, context: &CommandContext) -> Option<&'static str> {
    use CommandId::*;
    match id {
        _ if context.busy
            && !matches!(id, Palette | Theme | Contrast | ReducedMotion | Settings) =>
        {
            Some("Finish or cancel the open dialog first")
        }
        CreatePart | CreatePort | CreateItem | CreateAttribute | CreateInterface
        | CreateRequirement | Rename | Delete | Connect | MoveTo | Lock | Undo | Redo
        | Checkpoint
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
        LeaveFocus if !context.focused => Some("Nothing is focused"),
        AskAssistant | InsertSelection | NewConversation | ShowConversation if !context.project => {
            Some("Open or create a project to work with the Assistant")
        }
        InsertSelection if !context.selected => Some("Select an element first"),
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
            let plain = !key.contains("ctrl-") && !key.contains("alt-");
            assert!(
                !(plain && WHILE_TYPING.contains(&command.id)),
                "{key} would take a key from a text field"
            );
        }
    }
}
