//! Development sessions on the Claude Agent runtime (C-53, ROADMAP §4.7):
//! what a session may do ([`Policy`]), where its model answers
//! ([`Endpoint`]), and how a running session is steered ([`Steering`]).
//!
//! The Studio (or the Orchestrator) decides the policy; the companion
//! enforces it in its one pre-tool hook (`claude-agent/src/policy.ts`). A
//! model file is never written with a file tool, whatever else the policy
//! says, so the model changes only through Agentique's tools. These are
//! gates on what the agent can do, not an operating-system sandbox.

use serde::Serialize;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// What a development session may do: sent in protocol 2's `start`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    /// Folders the file tools may read.
    pub read: Vec<PathBuf>,
    /// Folders the file tools may write; each inside a read folder.
    pub write: Vec<PathBuf>,
    /// Paths (relative to the folders; `*` within a name, `**` across
    /// folders) never written by a file tool. The model files are always
    /// among them.
    pub protected: Vec<String>,
    /// Paths never read by a file tool: keys and other secrets.
    pub hidden: Vec<String>,
    /// Whether commands may run at all (trusted-local execution).
    pub commands: bool,
    /// Commands refused even then.
    pub refused_commands: Vec<RefusedCommand>,
    /// Web fetch and search.
    pub network: bool,
    /// MCP servers besides Agentique's whose tools may run.
    pub mcp_servers: Vec<String>,
    /// A call the policy does not decide.
    pub undecided: Undecided,
}

/// What happens to a call the policy does not decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Undecided {
    /// The Studio asks the Operator (the Conversation).
    Ask,
    /// It is refused with the reason (an objective's sessions: nobody is
    /// asked, so the objective's permissions are the whole policy).
    Refuse,
}

/// A command the policy refuses, and the reason the agent reads. The
/// pattern is a regular expression matched case-insensitively by the
/// companion (JavaScript); the standard ones use only syntax that reads the
/// same in Rust, where they are tested.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RefusedCommand {
    pub pattern: String,
    pub reason: String,
}

fn refused(pattern: &str, reason: &str) -> RefusedCommand {
    RefusedCommand {
        pattern: pattern.into(),
        reason: reason.into(),
    }
}

/// The model files, relative to the project: never written with a file
/// tool, so the model changes only through Agentique's tools.
pub const MODEL_FILES: [&str; 4] = [
    "model/*.sysml",
    "model/agentique.json",
    "model/links.json",
    "model/agentique.lock",
];

/// The project's agent configuration: it steers later sessions and its hooks
/// run outside the policy, so a session changes it only when an objective
/// names it.
pub const AGENT_CONFIGURATION: [&str; 3] = [".claude", "CLAUDE.md", "AGENTS.md"];

/// Never read with a file tool: files that hold keys.
pub const SECRET_FILES: [&str; 6] = [
    ".env",
    ".env.*",
    "**/*.pem",
    "**/*.key",
    "**/.git-credentials",
    "**/id_rsa*",
];

/// Where a session works, which decides some of what it may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The Operator's own working copy (the Conversation): commands that
    /// discard work or rewrite history are refused.
    WorkingCopy,
    /// A worktree of its own (an objective's cycle), which nothing else
    /// uses.
    Worktree,
}

/// The standard refused commands. Pushing is refused unless `push`; force
/// pushes, pushes to `main`, merging pull requests, global git
/// configuration, reading key files and writing model files are refused
/// always; discarding work and rewriting history are refused in the
/// Operator's working copy; downloads are refused without the network.
pub fn refused_commands(place: Place, push: bool, network: bool) -> Vec<RefusedCommand> {
    let mut list = vec![
        refused(
            r"\bgit\s+push\b[^\n]*(--force|--force-with-lease|\s-f\b|\s\+)",
            "Force-pushing rewrites shared history; it is never done from Agentique.",
        ),
        refused(
            r"\bgit\s+push\b[^\n]*(\s|:)(refs/heads/)?(main|master)(\s|:|$)",
            "Nothing is pushed to the default branch directly: changes reach it through a reviewed pull request.",
        ),
        refused(
            r"\bgh\s+pr\s+merge\b",
            "Merging is the Orchestrator's, after the checks and an independent review pass.",
        ),
        refused(
            r"\bgh\s+(repo\s+(delete|edit|rename)|api\s+[^\n]*-X\s*(DELETE|PATCH|PUT))",
            "Changing or deleting the repository on GitHub is the Operator's.",
        ),
        refused(
            r"\bgit\s+config\s+[^\n]*--(global|system)\b",
            "Global git configuration is the Operator's.",
        ),
        refused(
            r"(^|[\s/\\])\.env(\.|\s|$)|\.git-credentials|\bid_rsa\b",
            "That file holds keys or credentials; it is not read in a session.",
        ),
        refused(
            r"(>|\btee\b|\bsed\s+-i|\bperl\s+-i|Set-Content|Add-Content|Out-File|\bmv\b|\bmove\b|\bcp\b|\bcopy\b|\brm\b|\bdel\b|Remove-Item|\bgit\s+(checkout|restore)\b)[^\n]*\bmodel[\\/]+([^\s\\/]*\.sysml|agentique\.json|links\.json)",
            "The model changes only through Agentique's tools (apply_changes), never by writing its files.",
        ),
    ];
    if !push {
        list.push(refused(
            r"\bgit\s+push\b",
            "Pushing is not allowed in this session.",
        ));
    }
    if place == Place::WorkingCopy {
        list.push(refused(
            r"\bgit\s+(reset\s+--hard|clean\s+-[a-zA-Z]*f|checkout\s+--\s|checkout\s+\.(\s|$)|restore\s|stash\s+(drop|clear)|branch\s+-D|rebase\b|filter-branch|filter-repo)",
            "That would discard or rewrite work in the Operator's working copy.",
        ));
    }
    if !network {
        list.push(refused(
            r"\b(curl|wget|Invoke-WebRequest|Invoke-RestMethod|iwr|irm)\b",
            "The network is off for this session.",
        ));
    }
    list
}

impl Policy {
    /// The policy for development in `folder` (and the folders it may only
    /// read): it reads and writes there, never writes the model files or
    /// the `protected` paths (the project's own, from its links), never
    /// reads key files, runs commands only if `commands` (trusted-local
    /// execution), and refuses the standard commands for its place.
    pub fn development(
        folder: &Path,
        also_read: &[PathBuf],
        protected: &[String],
        place: Place,
        options: Permissions,
    ) -> Policy {
        let mut read = vec![folder.to_path_buf()];
        read.extend(also_read.iter().cloned());
        let mut guarded: Vec<String> = MODEL_FILES
            .iter()
            .chain(AGENT_CONFIGURATION.iter())
            .map(|p| p.to_string())
            .collect();
        guarded.push(".git".into());
        for path in protected {
            if !guarded.contains(path) {
                guarded.push(path.clone());
            }
        }
        Policy {
            read,
            write: vec![folder.to_path_buf()],
            protected: guarded,
            hidden: SECRET_FILES.iter().map(|p| p.to_string()).collect(),
            commands: options.commands,
            refused_commands: refused_commands(place, options.push, options.network),
            network: options.network,
            mcp_servers: options.mcp_servers,
            undecided: options.undecided,
        }
    }

    /// The same policy, writing nothing (a reviewer or an evaluator).
    pub fn read_only(mut self) -> Policy {
        self.write.clear();
        self
    }

    /// The same policy, letting the session write paths an objective names
    /// (its agent configuration, say). The model files stay protected.
    pub fn allowing(mut self, named: &[String]) -> Policy {
        self.protected
            .retain(|p| MODEL_FILES.contains(&p.as_str()) || !named.contains(p));
        self
    }
}

/// The choices a policy is made from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permissions {
    pub commands: bool,
    pub network: bool,
    pub push: bool,
    pub mcp_servers: Vec<String>,
    pub undecided: Undecided,
}

/// An Anthropic-compatible endpoint (C-53): the base URL the SDK talks to,
/// the provider whose key and prices apply, and the model for the SDK's own
/// small tasks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub base_url: String,
    pub provider: agq_providers::Provider,
    pub fast_model: Option<String>,
}

/// DeepSeek's documented Anthropic-compatible endpoint
/// (api-docs.deepseek.com, "Anthropic API").
pub const DEEPSEEK_ENDPOINT: &str = "https://api.deepseek.com/anthropic";

impl Endpoint {
    pub fn deepseek() -> Endpoint {
        Endpoint {
            base_url: DEEPSEEK_ENDPOINT.into(),
            provider: agq_providers::Provider::DeepSeek,
            fast_model: Some("deepseek-flash".into()),
        }
    }
}

/// A development session: where it works and what it may do.
#[derive(Clone, Debug, PartialEq)]
pub struct Development {
    /// The agent's working folder.
    pub cwd: PathBuf,
    pub policy: Policy,
    /// The project's settings to load (`project`, `local`).
    pub setting_sources: Vec<String>,
    /// Subagents besides the SDK's own and the project's, by name, as the
    /// SDK's agent definitions (`description`, `prompt`, `tools`, …).
    pub agents: serde_json::Value,
    /// Agentique's instructions appended to the SDK's development
    /// instructions (true), or the whole system prompt (false).
    pub preset: bool,
}

/// The pause gate (C-53): Pause holds the session at its next tool call,
/// Step lets one through, Run goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Run,
    Pause,
    Step,
}

impl Gate {
    pub fn as_str(self) -> &'static str {
        match self {
            Gate::Run => "run",
            Gate::Pause => "pause",
            Gate::Step => "step",
        }
    }
}

#[derive(Debug)]
struct SteeringState {
    messages: VecDeque<String>,
    gate: Gate,
    /// The gate changed since the running turn last read it.
    changed: bool,
    /// The tool the session is held at, while paused.
    held_at: Option<String>,
}

/// Messages queued into a running session, and its pause gate: shared by
/// whoever steers (the Conversation, the Orchestrator) and the runtime,
/// which reads them while the turn runs.
#[derive(Clone, Debug)]
pub struct Steering(Arc<Mutex<SteeringState>>);

impl Default for Steering {
    fn default() -> Self {
        Steering(Arc::new(Mutex::new(SteeringState {
            messages: VecDeque::new(),
            gate: Gate::Run,
            changed: false,
            held_at: None,
        })))
    }
}

impl Steering {
    /// A message for the running session; the next turn's if none runs.
    pub fn queue(&self, text: &str) {
        if let Ok(mut state) = self.0.lock() {
            state.messages.push_back(text.to_string());
        }
    }

    pub fn set_gate(&self, gate: Gate) {
        if let Ok(mut state) = self.0.lock() {
            state.gate = gate;
            state.changed = true;
            if gate != Gate::Pause {
                state.held_at = None;
            }
        }
    }

    pub fn gate(&self) -> Gate {
        self.0.lock().map(|s| s.gate).unwrap_or(Gate::Run)
    }

    /// The tool the session is held at, while paused.
    pub fn held_at(&self) -> Option<String> {
        self.0.lock().ok().and_then(|s| s.held_at.clone())
    }

    pub(crate) fn note_held(&self, tool: &str) {
        if let Ok(mut state) = self.0.lock() {
            state.held_at = Some(tool.to_string());
        }
    }

    /// What the runtime sends next: queued messages, and the gate if it
    /// changed (or, at a turn's start, if it is not Run).
    pub(crate) fn take(&self, starting: bool) -> (Vec<String>, Option<Gate>) {
        let Ok(mut state) = self.0.lock() else {
            return (Vec::new(), None);
        };
        let messages = state.messages.drain(..).collect();
        let gate = if state.changed || (starting && state.gate != Gate::Run) {
            state.changed = false;
            // A Step already used is Pause.
            let gate = state.gate;
            if gate == Gate::Step {
                state.gate = Gate::Pause;
            }
            Some(gate)
        } else {
            None
        };
        (messages, gate)
    }
}

/// A development session's environment: the Studio's own, without anything
/// that looks like a key, token or secret and without the variables of a
/// Claude Code session that may have started the Studio. The companion
/// filters it again (the one place is its policy).
pub fn development_environment() -> Vec<(String, String)> {
    std::env::vars()
        .filter(|(name, _)| !secret_name(name) && !parent_session_name(name))
        .collect()
}

/// Whether a variable's name looks like a key, token or other secret.
pub fn secret_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    [
        "KEY",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "CREDENTIAL",
        "AUTH",
        "COOKIE",
        "PRIVATE",
    ]
    .iter()
    .any(|word| upper.contains(word))
}

fn parent_session_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper == "CLAUDECODE"
        || ["CLAUDE_CODE_", "CLAUDE_AGENT_SDK_", "ANTHROPIC_", "OTEL_"]
            .iter()
            .any(|prefix| upper.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refusal(place: Place, push: bool, network: bool, command: &str) -> Option<String> {
        refused_commands(place, push, network)
            .into_iter()
            .find(|r| {
                regex::RegexBuilder::new(&r.pattern)
                    .case_insensitive(true)
                    .build()
                    .expect("a pattern Rust and JavaScript both read")
                    .is_match(command)
            })
            .map(|r| r.reason)
    }

    #[test]
    fn ordinary_development_commands_are_not_refused() {
        for command in [
            "cargo test --workspace",
            "cargo clippy -p agq-launcher --all-targets -- -D warnings",
            "git status && git diff --stat",
            "git commit -m \"Fix the gap\"",
            "git log --oneline -5",
            "python tools/check_architecture.py",
            "gh pr view 97 --json state",
            "gh pr checks 97",
            "rg -n model crates/history",
            "cat crates/history/src/lib.rs > /tmp/copy.rs",
            "node --test claude-agent/test/*.test.ts",
        ] {
            assert_eq!(
                refusal(Place::WorkingCopy, false, false, command),
                None,
                "{command}"
            );
        }
        assert_eq!(
            refusal(
                Place::Worktree,
                true,
                true,
                "git push -u origin agentique/objective-3"
            ),
            None
        );
        assert_eq!(
            refusal(Place::Worktree, true, true, "git reset --hard HEAD~1"),
            None,
            "a cycle's own worktree may be reset"
        );
    }

    #[test]
    fn the_standard_refusals_hold_whatever_the_spelling() {
        let refused = |command: &str| refusal(Place::Worktree, true, true, command);
        assert!(refused("git push --force origin agentique/x").is_some());
        assert!(refused("git push -f").is_some());
        assert!(refused("git push origin +agentique/x").is_some());
        assert!(refused("git push origin main").is_some());
        assert!(refused("GIT PUSH origin HEAD:main").is_some());
        assert!(refused("git push origin main:main").is_some());
        assert!(refused("git push origin agentique/main-fix").is_none());
        assert!(refused("gh pr merge 12 --squash --admin").is_some());
        assert!(refused("gh repo delete agentique-systems/agentique --yes").is_some());
        assert!(refused("git config --global user.email x").is_some());
        assert!(refused("cat .env").is_some());
        assert!(refused("type C:\\work\\.env").is_some());
        assert!(refused("sed -i 's/a/b/' model/Agentique.sysml").is_some());
        assert!(refused("echo x > model\\links.json").is_some());
        assert!(refused("git checkout main -- model/agentique.json").is_some());
        // Reading the model's text is fine.
        assert!(refused("grep -n Orchestrator model/Agentique.sysml").is_none());
        // Without push, any push; in the Operator's working copy, discarding.
        assert!(refusal(Place::Worktree, false, true, "git push -u origin x").is_some());
        assert!(refusal(Place::WorkingCopy, true, true, "git reset --hard").is_some());
        assert!(refusal(Place::WorkingCopy, true, true, "git clean -fdx").is_some());
        assert!(refusal(Place::WorkingCopy, true, true, "git stash drop").is_some());
        assert!(refusal(Place::Worktree, true, false, "curl https://example.com").is_some());
    }

    #[test]
    fn a_development_policy_always_protects_the_model_and_hides_keys() {
        let policy = Policy::development(
            Path::new("C:\\work\\agentique"),
            &[],
            &["standards".into(), "model/*.sysml".into()],
            Place::WorkingCopy,
            Permissions {
                commands: true,
                network: true,
                push: false,
                mcp_servers: Vec::new(),
                undecided: Undecided::Ask,
            },
        );
        for path in MODEL_FILES.iter().chain(AGENT_CONFIGURATION.iter()) {
            assert!(policy.protected.contains(&path.to_string()), "{path}");
        }
        let named = policy
            .clone()
            .allowing(&["CLAUDE.md".into(), "model/*.sysml".into()]);
        assert!(!named.protected.contains(&"CLAUDE.md".to_string()));
        assert!(named.protected.contains(&"model/*.sysml".to_string()));
        assert!(policy.protected.contains(&"standards".to_string()));
        assert_eq!(
            policy
                .protected
                .iter()
                .filter(|p| *p == "model/*.sysml")
                .count(),
            1
        );
        assert!(policy.hidden.contains(&".env".to_string()));
        let json = serde_json::to_value(&policy).unwrap();
        assert_eq!(json["undecided"], "ask");
        assert!(json["refusedCommands"].as_array().unwrap().len() > 5);
        assert!(policy.clone().read_only().write.is_empty());
    }

    #[test]
    fn steering_hands_over_messages_once_and_a_step_becomes_a_pause() {
        let steering = Steering::default();
        assert_eq!(steering.take(true), (Vec::new(), None));
        steering.queue("Also update the README.");
        steering.set_gate(Gate::Step);
        let (messages, gate) = steering.take(false);
        assert_eq!(messages, vec!["Also update the README.".to_string()]);
        assert_eq!(gate, Some(Gate::Step));
        assert_eq!(steering.gate(), Gate::Pause);
        assert_eq!(steering.take(false), (Vec::new(), None));
        // A new turn starts paused if the gate is not Run.
        assert_eq!(steering.take(true), (Vec::new(), Some(Gate::Pause)));
    }

    #[test]
    fn secrets_and_a_parent_session_never_pass() {
        for name in [
            "DEEPSEEK_API_KEY",
            "GITHUB_TOKEN",
            "gh_token",
            "AWS_SECRET_ACCESS_KEY",
            "NPM_AUTH",
        ] {
            assert!(secret_name(name), "{name}");
        }
        for name in ["PATH", "USERPROFILE", "CARGO_HOME", "TEMP", "ComSpec"] {
            assert!(!secret_name(name) && !parent_session_name(name), "{name}");
        }
        for name in [
            "CLAUDECODE",
            "CLAUDE_CODE_ENTRYPOINT",
            "ANTHROPIC_BASE_URL",
            "OTEL_EXPORTER",
        ] {
            assert!(parent_session_name(name), "{name}");
        }
    }
}
