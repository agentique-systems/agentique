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
    Command {
        id: CommandId::Focus,
        label: "Focus selection",
        shortcut: "F",
        description: "Move into the selected system",
    },
    Command {
        id: CommandId::Dependencies,
        label: "Show dependencies",
        shortcut: "D",
        description: "Create a temporary semantic neighborhood",
    },
    Command {
        id: CommandId::CreatePart,
        label: "Create nested PartUsage",
        shortcut: "",
        description: "Prepare a source-backed candidate for review",
    },
    Command {
        id: CommandId::Graph,
        label: "Open Graph World",
        shortcut: "2",
        description: "Explore typed relationship families",
    },
    Command {
        id: CommandId::System,
        label: "Open System World",
        shortcut: "1",
        description: "Navigate the system architecture",
    },
    Command {
        id: CommandId::Requirements,
        label: "Open Requirements World",
        shortcut: "3",
        description: "Follow requirement and verification context",
    },
    Command {
        id: CommandId::History,
        label: "Open revision history",
        shortcut: "4",
        description: "Inspect immutable design history",
    },
    Command {
        id: CommandId::Explain,
        label: "Explain selection",
        shortcut: "E",
        description: "Inspect provenance and its evidence",
    },
    Command {
        id: CommandId::Source,
        label: "Open source",
        shortcut: "",
        description: "Inspect source in the exact selected revision",
    },
    Command {
        id: CommandId::Compare,
        label: "Compare with parent",
        shortcut: "",
        description: "Show additions, changes and removed ghosts",
    },
    Command {
        id: CommandId::ExpandOutgoing,
        label: "Expand outgoing relationships",
        shortcut: "",
        description: "Add one deliberate neighborhood step",
    },
    Command {
        id: CommandId::ExpandIncoming,
        label: "Expand incoming relationships",
        shortcut: "",
        description: "Follow relationships into the selection",
    },
    Command {
        id: CommandId::Neighbors,
        label: "Select neighbors",
        shortcut: "N",
        description: "Select adjacent elements in the current projection",
    },
    Command {
        id: CommandId::Fit,
        label: "Fit view",
        shortcut: "Home",
        description: "Frame the visible system",
    },
    Command {
        id: CommandId::Home,
        label: "Home",
        shortcut: "",
        description: "Return to the project architecture",
    },
    Command {
        id: CommandId::Back,
        label: "Navigate back",
        shortcut: "Alt+Left",
        description: "Restore previous camera and focus",
    },
    Command {
        id: CommandId::Forward,
        label: "Navigate forward",
        shortcut: "Alt+Right",
        description: "Restore next camera and focus",
    },
    Command {
        id: CommandId::Up,
        label: "Up to owner",
        shortcut: "Alt+Up",
        description: "Navigate through semantic ownership",
    },
    Command {
        id: CommandId::Validate,
        label: "Validate candidate",
        shortcut: "",
        description: "Run the platform validation contract",
    },
    Command {
        id: CommandId::Commit,
        label: "Commit validated candidate",
        shortcut: "",
        description: "Durable compare-and-set of the branch head",
    },
    Command {
        id: CommandId::Cancel,
        label: "Cancel candidate",
        shortcut: "Esc",
        description: "Discard the uncommitted alternate revision",
    },
    Command {
        id: CommandId::Theme,
        label: "Switch light / dark theme",
        shortcut: "",
        description: "Change presentation surfaces",
    },
    Command {
        id: CommandId::Contrast,
        label: "Toggle high contrast",
        shortcut: "",
        description: "Increase focus, edge and text contrast",
    },
    Command {
        id: CommandId::ReducedMotion,
        label: "Toggle reduced motion",
        shortcut: "",
        description: "Use immediate camera transitions",
    },
];

/// UI review state preserves the platform lifecycle; a visual preview cannot be
/// promoted to Validated by a boolean or confused with unresolved durability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CandidateReview {
    #[default]
    None,
    Visual,
    Semantic(agq_studio_platform::CandidatePhase),
}

#[derive(Default)]
pub struct CommandContext {
    pub selected: bool,
    pub can_create: bool,
    pub candidate: CandidateReview,
    pub live: bool,
    pub busy: bool,
}

/// Advisory UI eligibility for the existing reviewed source command. The service
/// independently verifies exact source provenance and owner semantics on proposal.
pub fn can_create_part(
    node: &agq_modeling_view::ViewNode,
    fixture: bool,
    revision: agq_modeling_workspace::ProjectRevisionId,
) -> bool {
    node.revision_id == revision
        && node.origin == agq_modeling_view::ViewOrigin::Authored
        && (fixture || node.source_available)
        && matches!(node.semantic_kind.as_str(), "PartDefinition" | "PartUsage")
}

pub fn unavailable(id: CommandId, context: &CommandContext) -> Option<&'static str> {
    use CommandId::*;
    if context.busy && matches!(id, CreatePart | Validate | Commit | Cancel) {
        return Some("A model operation is running");
    }
    match id {
        Compare if context.candidate != CandidateReview::None => {
            Some("Review or cancel the candidate before comparing durable revisions")
        }
        Focus | Dependencies | Explain | Source | Neighbors | CreatePart | ExpandIncoming
        | ExpandOutgoing
            if !context.selected =>
        {
            Some("Select an element first")
        }
        CreatePart if context.candidate != CandidateReview::None => {
            Some("Review or cancel the existing candidate")
        }
        CreatePart if !context.can_create => {
            Some("Select an authored part with source to create a nested part")
        }
        Validate if !context.live => Some("Visual fixtures cannot establish semantic validation"),
        Validate | Commit | Cancel if context.candidate == CandidateReview::None => {
            Some("No candidate to review")
        }
        Validate | Cancel
            if context.candidate
                == CandidateReview::Semantic(
                    agq_studio_platform::CandidatePhase::CommitUnresolved,
                ) =>
        {
            Some("Commit acknowledgement is unresolved; retry the same commit")
        }
        Validate | Cancel | Commit
            if context.candidate
                == CandidateReview::Semantic(agq_studio_platform::CandidatePhase::Committed) =>
        {
            Some("Candidate is already durably committed")
        }
        Validate
            if context.candidate
                != CandidateReview::Semantic(agq_studio_platform::CandidatePhase::Working) =>
        {
            Some("This candidate is not awaiting validation")
        }
        Commit if !context.live => {
            Some("Install the authenticated runtime to commit model changes")
        }
        Commit
            if !matches!(
                context.candidate,
                CandidateReview::Semantic(
                    agq_studio_platform::CandidatePhase::Validated
                        | agq_studio_platform::CandidatePhase::CommitUnresolved
                )
            ) =>
        {
            Some("Validate this candidate before committing")
        }
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
        let context = CommandContext {
            candidate: CandidateReview::Semantic(agq_studio_platform::CandidatePhase::Validated),
            ..Default::default()
        };
        assert!(unavailable(CommandId::Validate, &context).is_some());
        assert!(unavailable(CommandId::Commit, &context).is_some());
        assert_eq!(
            search("nested part").next().unwrap().id,
            CommandId::CreatePart
        );
    }

    #[test]
    fn unknown_acknowledgement_exposes_only_commit_retry_among_mutations() {
        use agq_studio_platform::CandidatePhase;
        let mut context = CommandContext {
            selected: true,
            can_create: true,
            candidate: CandidateReview::Semantic(CandidatePhase::CommitUnresolved),
            live: true,
            busy: false,
        };
        assert!(unavailable(CommandId::Commit, &context).is_none());
        for command in [
            CommandId::Validate,
            CommandId::Cancel,
            CommandId::CreatePart,
            CommandId::Compare,
        ] {
            assert!(unavailable(command, &context).is_some());
        }
        context.busy = true;
        assert!(unavailable(CommandId::Commit, &context).is_some());
    }

    #[test]
    fn only_validated_semantic_candidate_can_commit_and_visual_preview_can_cancel() {
        use agq_studio_platform::CandidatePhase;
        for (candidate, can_commit) in [
            (CandidateReview::None, false),
            (CandidateReview::Visual, false),
            (CandidateReview::Semantic(CandidatePhase::Working), false),
            (CandidateReview::Semantic(CandidatePhase::Validated), true),
            (CandidateReview::Semantic(CandidatePhase::Committed), false),
        ] {
            let context = CommandContext {
                candidate,
                live: true,
                ..Default::default()
            };
            assert_eq!(
                unavailable(CommandId::Commit, &context).is_none(),
                can_commit
            );
            assert_eq!(
                unavailable(CommandId::Compare, &context).is_none(),
                candidate == CandidateReview::None,
                "Durable comparisons cannot be shadowed by a retained candidate"
            );
        }
        let context = CommandContext {
            candidate: CandidateReview::Visual,
            ..Default::default()
        };
        assert!(unavailable(CommandId::Cancel, &context).is_none());
    }

    #[test]
    fn source_edit_eligibility_excludes_requirements_derived_standard_and_old_ghosts() {
        let fixture = agq_studio_scene::fixtures::architecture();
        let mut node = fixture
            .nodes
            .iter()
            .find(|n| n.semantic_kind == "PartUsage")
            .unwrap()
            .clone();
        assert!(can_create_part(&node, true, fixture.revision_id));
        assert!(!can_create_part(&node, false, fixture.revision_id));
        node.source_available = true;
        assert!(can_create_part(&node, false, fixture.revision_id));
        for kind in ["RequirementDefinition", "PortUsage", "CustomPartUsage"] {
            node.semantic_kind = kind.into();
            assert!(!can_create_part(&node, true, fixture.revision_id));
        }
        node.semantic_kind = "PartDefinition".into();
        node.origin = agq_modeling_view::ViewOrigin::Derived;
        assert!(!can_create_part(&node, true, fixture.revision_id));
        node.origin = agq_modeling_view::ViewOrigin::Standard;
        assert!(!can_create_part(&node, true, fixture.revision_id));
        node.origin = agq_modeling_view::ViewOrigin::Authored;
        assert!(!can_create_part(
            &node,
            true,
            agq_modeling_workspace::ProjectRevisionId::new()
        ));
    }
}
