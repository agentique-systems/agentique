//! The Assistant's runtime in the Studio (ROADMAP §4.7, §4.10, C-51): which
//! runtime runs the Conversation's turns, and the Claude Agent runtime's
//! setup and health check for Settings. The runtime itself is
//! `agq_assistant::claude_agent`; this decides when to use it and says what
//! it needs.

use crate::studio::{Dirty, Studio};
use agq_assistant::claude_agent::{self, ClaudeAgent, Effective, Installation, Node, Verified};
use agq_providers::{KeyStatus, Provider};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, TryRecvError};

/// The runtime setting's values.
pub const LOOP: &str = "loop";
pub const CLAUDE_AGENT: &str = "claude-agent";

/// What Settings shows about the Claude Agent runtime.
#[derive(Clone, Debug, Default)]
pub struct Health {
    pub node: Option<Result<Node, String>>,
    pub installed: Option<String>,
    pub verified: Option<Result<Verified, String>>,
    pub effective: Option<Result<Effective, String>>,
}

impl Health {
    /// The report in lines: (fine, text).
    pub fn lines(&self, key: &KeyStatus) -> Vec<(bool, String)> {
        let mut lines = Vec::new();
        match &self.node {
            Some(Ok(node)) => lines.push((
                true,
                format!("Node.js {} at {}", node.version, node.path.display()),
            )),
            Some(Err(why)) => lines.push((false, why.clone())),
            None => {}
        }
        match &self.installed {
            Some(version) if version == claude_agent::SDK_VERSION => lines.push((
                true,
                format!("Claude Agent SDK {version} installed in {}", Installation { root: Installation::default_root() }.folder().display()),
            )),
            Some(version) => lines.push((
                false,
                format!("Claude Agent SDK {version} is installed, but this Agentique needs {}: install again.", claude_agent::SDK_VERSION),
            )),
            None => lines.push((
                false,
                format!("The runtime is not installed: Install downloads the Claude Agent SDK {} and its Claude Code binary (about 300 MB) from the npm registry.", claude_agent::SDK_VERSION),
            )),
        }
        match &self.verified {
            Some(Ok(v)) if v.checksum_matches => lines.push((
                true,
                format!(
                    "Claude Code {}: the binary matches the checksum in the SDK's manifest",
                    v.claude_code
                ),
            )),
            Some(Ok(v)) => lines.push((
                false,
                format!(
                    "Claude Code {}: the binary does NOT match the SDK's checksum; it is not used",
                    v.claude_code
                ),
            )),
            Some(Err(why)) => lines.push((false, why.clone())),
            None => {}
        }
        lines.push(match key {
            KeyStatus::Stored => (true, "The Anthropic key is in the Windows Credential Manager".into()),
            KeyStatus::FromEnvironment { variable } => (true, format!("The Anthropic key comes from {variable}")),
            KeyStatus::Missing => (false, "No Anthropic key: add one in Settings › Providers › Anthropic. A claude.ai login is not offered: Anthropic does not allow it for other products.".into()),
            KeyStatus::Unavailable(why) => (false, format!("The credential store cannot be read: {why}")),
        });
        match &self.effective {
            Some(Ok(effective)) => {
                let problems = effective.problems();
                if problems.is_empty() {
                    lines.push((
                        true,
                        format!(
                            "Checked with the SDK itself: the agent can call only Agentique's {} tools, through Agentique's own server, and is never asked for permissions ({}); no settings, hooks, plugins or memory of this machine are loaded",
                            effective.tools.len(),
                            effective.permission_mode
                        ),
                    ));
                    if !effective.skills.is_empty() || !effective.agents.is_empty() {
                        lines.push((
                            true,
                            "Claude Code's own skills and agents are listed by its binary but cannot be used: the Skill and Agent tools are off".into(),
                        ));
                    }
                } else {
                    lines.push((
                        false,
                        format!(
                            "The SDK did not start as configured: {}",
                            problems.join("; ")
                        ),
                    ));
                }
            }
            Some(Err(why)) => lines.push((false, format!("The configuration check failed: {why}"))),
            None => {}
        }
        lines
    }
}

/// Setup work running on its own thread.
pub enum Work {
    Check(Receiver<Health>),
    Install(Receiver<Result<Health, String>>, Arc<AtomicBool>),
}

#[derive(Default)]
pub struct RuntimeState {
    pub health: Option<Health>,
    pub work: Option<Work>,
    /// The Operator asked to install; the card asks once more.
    pub confirm_install: bool,
    pub message: Option<String>,
}

impl Studio {
    /// The runtime the Settings choose.
    pub fn runtime_setting(&self) -> String {
        match self.settings.text("assistant.runtime").as_str() {
            CLAUDE_AGENT => CLAUDE_AGENT.into(),
            _ => LOOP.into(),
        }
    }

    /// Where the Claude Agent runtime keeps its configuration and sessions:
    /// beside the session file, so a test instance has its own.
    pub fn claude_agent_data(&self) -> PathBuf {
        self.session_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("claude-agent")
    }

    /// Applies the Settings' runtime and model to the Conversation's next
    /// turns.
    pub fn apply_runtime_choice(&mut self) {
        let choice = self.settings.model_choice();
        if self.runtime_setting() != CLAUDE_AGENT || self.safe_mode {
            self.conversation.use_choice(choice);
            return;
        }
        // The model and effort chosen for Anthropic carry over; otherwise
        // the SDK's default model.
        let model = (choice.model.provider == Provider::Anthropic)
            .then(|| choice.model.model.clone())
            .filter(|m| !m.is_empty());
        let effort = choice
            .effort
            .clone()
            .filter(|e| matches!(e.as_str(), "low" | "medium" | "high" | "xhigh" | "max"));
        let node = claude_agent::find_node();
        let installation = Installation {
            root: Installation::default_root(),
        };
        let problem = match (&node, installation.installed(), agq_providers::key_status(Provider::Anthropic)) {
            (Err(why), _, _) => Some(why.clone()),
            (_, false, _) => Some("The Claude Agent runtime is not installed: Settings › Assistant › Install.".into()),
            (_, _, KeyStatus::Missing) => Some("The Claude Agent runtime needs an Anthropic key: add it in Settings › Providers › Anthropic. Everything else works as usual.".into()),
            (_, _, KeyStatus::Unavailable(why)) => Some(format!("The Anthropic key cannot be read: {why}")),
            _ => None,
        };
        let data = self.claude_agent_data();
        let label = match &model {
            Some(model) => format!("Claude Agent · {model}"),
            None => "Claude Agent".into(),
        };
        let node = node.ok();
        self.conversation.use_runtime(
            label,
            problem,
            Box::new(move || -> Box<dyn agq_assistant::Runtime> {
                match (node.clone(), agq_providers::claude_agent_key()) {
                    (Some(node), Ok(Some(key))) => Box::new(ClaudeAgent::new(
                        node,
                        installation.clone(),
                        data.clone(),
                        model.clone(),
                        effort.clone(),
                        key,
                    )),
                    (_, result) => Box::new(Unavailable(match result {
                        Err(why) => format!("The Anthropic key cannot be read: {why}"),
                        _ => "The Claude Agent runtime is not ready: see Settings › Assistant."
                            .into(),
                    })),
                }
            }),
        );
    }

    /// Checks the Claude Agent runtime on its own thread: Node, the
    /// installed packages, the binary's checksum, and what the SDK reports
    /// when it starts with the turn's configuration (at no cost).
    pub fn check_runtime(&mut self) {
        if self.runtime.work.is_some() {
            return;
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(health());
        });
        self.runtime.work = Some(Work::Check(receiver));
        self.runtime.message = Some("Checking the Claude Agent runtime…".into());
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
    }

    /// Installs the Claude Agent runtime's packages (the Operator confirmed
    /// the download), then checks it.
    pub fn install_runtime(&mut self) {
        if self.runtime.work.is_some() {
            return;
        }
        self.runtime.confirm_install = false;
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let installation = Installation {
                root: Installation::default_root(),
            };
            let result = claude_agent::find_node()
                .and_then(|node| installation.install(&node, &flag).map(|_| health()));
            let _ = sender.send(result);
        });
        self.runtime.work = Some(Work::Install(receiver, cancel));
        self.runtime.message =
            Some("Installing the Claude Agent runtime from the npm registry…".into());
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
    }

    /// Whether an installation is running (it can be cancelled).
    pub fn installing_runtime(&self) -> bool {
        matches!(self.runtime.work, Some(Work::Install(..)))
    }

    /// Cancels the installation: npm's process tree is ended.
    pub fn cancel_runtime_install(&mut self) {
        if let Some(Work::Install(_, cancel)) = &self.runtime.work {
            cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            self.runtime.message = Some("Cancelling the installation…".into());
            self.mark(Dirty::LAYOUT);
        }
    }

    /// Takes finished setup work. Returns whether anything changed.
    pub fn poll_runtime(&mut self) -> bool {
        let Some(work) = &self.runtime.work else {
            return false;
        };
        let finished = match work {
            Work::Check(receiver) => match receiver.try_recv() {
                Ok(health) => Some(Ok(health)),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("The check stopped without a result.".to_string()))
                }
            },
            Work::Install(receiver, _) => match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("The installation stopped without a result.".to_string()))
                }
            },
        };
        let Some(result) = finished else {
            return false;
        };
        self.runtime.work = None;
        match result {
            Ok(health) => {
                self.runtime.message = None;
                self.runtime.health = Some(health);
            }
            Err(why) => self.runtime.message = Some(why),
        }
        self.apply_runtime_choice();
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
        true
    }
}

/// The whole health check (blocking: Node, a checksum and the SDK's start).
pub fn health() -> Health {
    let node = claude_agent::find_node();
    let installation = Installation {
        root: Installation::default_root(),
    };
    let installed = installation.installed_sdk();
    let (verified, effective) = match (&node, installed.is_some()) {
        (Ok(node), true) => {
            let verified = claude_agent::verify(node, &installation);
            let effective = match &verified {
                Ok(v) if v.checksum_matches => Some(claude_agent::probe(node, &installation)),
                _ => None,
            };
            (Some(verified), effective)
        }
        _ => (None, None),
    };
    Health {
        node: Some(node),
        installed,
        verified,
        effective,
    }
}

/// A runtime that cannot start: its turn says why and ends.
struct Unavailable(String);

impl agq_assistant::Runtime for Unavailable {
    fn model(&self) -> Option<agq_providers::ModelRef> {
        None
    }

    fn label(&self) -> String {
        "Claude Agent".into()
    }

    fn run(
        &mut self,
        conversation: &mut agq_assistant::Conversation,
        _: &agq_assistant::turn::Toolset,
        _: usize,
        _: &mut dyn FnMut(&agq_assistant::ToolCall) -> agq_assistant::ToolResult,
        on_event: &mut dyn FnMut(agq_assistant::TurnEvent),
        _: &AtomicBool,
    ) {
        let entry = agq_assistant::Entry::Notice {
            text: self.0.clone(),
        };
        conversation.entries.push(entry.clone());
        on_event(agq_assistant::TurnEvent::Entry(entry));
    }
}
