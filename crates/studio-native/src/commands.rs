//! One action vocabulary for menus, palette, keyboard and future agent requests.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandId {
    Home,
    Back,
    Forward,
    Up,
    Focus,
    Fit,
    System,
    Graph,
    Requirements,
    History,
    Dependencies,
    Explain,
    Source,
    Neighbors,
    ExpandOutgoing,
    ExpandIncoming,
    CreatePart,
    Compare,
    Validate,
    Commit,
    Cancel,
    Theme,
    Contrast,
    ReducedMotion,
}

pub struct Command {
    pub id: CommandId,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub description: &'static str,
}

pub const COMMANDS: &[Command] = &[
    Command { id: CommandId::Focus, label: "Focus selection", shortcut: "F", description: "Move into the selected system" },
    Command { id: CommandId::Dependencies, label: "Show dependencies", shortcut: "D", description: "Create a temporary semantic neighborhood" },
    Command { id: CommandId::CreatePart, label: "Create nested PartUsage", shortcut: "", description: "Prepare a source-backed candidate for review" },
    Command { id: CommandId::Graph, label: "Open Graph World", shortcut: "2", description: "Explore typed relationship families" },
    Command { id: CommandId::System, label: "Open System World", shortcut: "1", description: "Navigate the system architecture" },
    Command { id: CommandId::Requirements, label: "Open Requirements World", shortcut: "3", description: "Follow requirement and verification context" },
    Command { id: CommandId::History, label: "Open revision history", shortcut: "4", description: "Inspect immutable design history" },
    Command { id: CommandId::Explain, label: "Explain selection", shortcut: "E", description: "Inspect provenance and its evidence" },
    Command { id: CommandId::Source, label: "Open source", shortcut: "", description: "Inspect source in the exact selected revision" },
    Command { id: CommandId::Compare, label: "Compare with parent", shortcut: "", description: "Show additions, changes and removed ghosts" },
    Command { id: CommandId::ExpandOutgoing, label: "Expand outgoing relationships", shortcut: "", description: "Add one deliberate neighborhood step" },
    Command { id: CommandId::ExpandIncoming, label: "Expand incoming relationships", shortcut: "", description: "Follow relationships into the selection" },
    Command { id: CommandId::Neighbors, label: "Select neighbors", shortcut: "N", description: "Select adjacent elements in the current projection" },
    Command { id: CommandId::Fit, label: "Fit view", shortcut: "Home", description: "Frame the visible system" },
    Command { id: CommandId::Home, label: "Home", shortcut: "", description: "Return to the project architecture" },
    Command { id: CommandId::Back, label: "Navigate back", shortcut: "Alt+Left", description: "Restore previous camera and focus" },
    Command { id: CommandId::Forward, label: "Navigate forward", shortcut: "Alt+Right", description: "Restore next camera and focus" },
    Command { id: CommandId::Up, label: "Up to owner", shortcut: "Alt+Up", description: "Navigate through semantic ownership" },
    Command { id: CommandId::Validate, label: "Validate candidate", shortcut: "", description: "Run the platform validation contract" },
    Command { id: CommandId::Commit, label: "Commit validated candidate", shortcut: "", description: "Durable compare-and-set of the branch head" },
    Command { id: CommandId::Cancel, label: "Cancel candidate", shortcut: "Esc", description: "Discard the uncommitted alternate revision" },
    Command { id: CommandId::Theme, label: "Switch light / dark theme", shortcut: "", description: "Change presentation surfaces" },
    Command { id: CommandId::Contrast, label: "Toggle high contrast", shortcut: "", description: "Increase focus, edge and text contrast" },
    Command { id: CommandId::ReducedMotion, label: "Toggle reduced motion", shortcut: "", description: "Use immediate camera transitions" },
];

#[derive(Default)]
pub struct CommandContext {
    pub selected: bool,
    pub candidate: bool,
    pub validated: bool,
    pub live: bool,
    pub busy: bool,
}

pub fn unavailable(id: CommandId, context: &CommandContext) -> Option<&'static str> {
    use CommandId::*;
    if context.busy && matches!(id, CreatePart | Validate | Commit | Cancel) {
        return Some("A model operation is running");
    }
    match id {
        Focus | Dependencies | Explain | Source | Neighbors | CreatePart | ExpandIncoming | ExpandOutgoing if !context.selected => Some("Select an element first"),
        CreatePart if context.candidate => Some("Review or cancel the existing candidate"),
        Validate if !context.live => Some("Visual fixtures cannot establish semantic validation"),
        Validate | Cancel if !context.candidate => Some("No candidate to review"),
        Commit if !context.live => Some("Install the authenticated runtime to commit model changes"),
        Commit if !context.validated => Some("Validate this candidate before committing"),
        _ => None,
    }
}

pub fn search(query: &str) -> impl Iterator<Item = &'static Command> {
    let words: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    COMMANDS.iter().filter(move |command| {
        let haystack = format!("{} {}", command.label, command.description).to_lowercase();
        words.iter().all(|word| haystack.contains(word))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_cannot_enable_validation_or_commit() {
        let context = CommandContext { candidate: true, validated: true, ..Default::default() };
        assert!(unavailable(CommandId::Validate, &context).is_some());
        assert!(unavailable(CommandId::Commit, &context).is_some());
        assert_eq!(search("nested part").next().unwrap().id, CommandId::CreatePart);
    }
}
