//! The Assistant's runtime in the Studio (ROADMAP §4.7, §4.10, C-51, C-53):
//! which runtime runs the Conversation's turns, where the Claude Agent
//! runtime's model answers (Anthropic, or DeepSeek's Anthropic-compatible
//! endpoint), the development session it gets in a project with a code
//! repository, and its setup and health check for Settings. The runtime
//! itself is `agq_assistant::claude_agent`; this decides when to use it and
//! says what it needs.

use crate::studio::{Dirty, Studio};
use agq_assistant::claude_agent::{self, ClaudeAgent, Effective, Installation, Node, Verified};
use agq_assistant::policy::{Development, Endpoint, Permissions, Place, Policy, Undecided};
use agq_providers::{KeyStatus, Provider};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, TryRecvError};

/// `path` without Windows' verbatim prefix (`\\?\C:\x` is `C:\x`), as the
/// SDK, Git and the session's commands expect it.
pub fn plain(path: &std::path::Path) -> PathBuf {
    let text = path.display().to_string();
    match text.strip_prefix(r"\\?\UNC\") {
        Some(rest) => PathBuf::from(format!(r"\\{rest}")),
        None => PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text)),
    }
}

/// The runtime setting's values.
pub const LOOP: &str = "loop";
pub const CLAUDE_AGENT: &str = "claude-agent";

/// The model a development session uses on DeepSeek's endpoint when the
/// Settings name none (its deliberate model; `deepseek-flash` does the
/// SDK's own small tasks).
pub const DEEPSEEK_MODEL: &str = "deepseek-v4-pro";

/// Where the Claude Agent runtime's model answers, from the Settings'
/// provider and the keys there are: Anthropic's API, or DeepSeek's
/// Anthropic-compatible endpoint (C-53). With no provider chosen, Anthropic
/// when its key is there, else DeepSeek when its key is.
pub fn model_access(provider: &str) -> Option<Endpoint> {
    let has = |p: Provider| {
        matches!(
            agq_providers::key_status(p),
            KeyStatus::Stored | KeyStatus::FromEnvironment { .. }
        )
    };
    match provider {
        "deepseek" => Some(Endpoint::deepseek()),
        "anthropic" => None,
        _ if !has(Provider::Anthropic) && has(Provider::DeepSeek) => Some(Endpoint::deepseek()),
        _ => None,
    }
}

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
            KeyStatus::Stored => (true, "The model's key is in the Windows Credential Manager".into()),
            KeyStatus::FromEnvironment { variable } => (true, format!("The model's key comes from {variable}")),
            KeyStatus::Missing => (false, "No key for the runtime's model: add an Anthropic key in Settings › Providers › Anthropic, or a DeepSeek key to use DeepSeek's Anthropic-compatible endpoint. A claude.ai login is not offered: Anthropic does not allow it for other products.".into()),
            KeyStatus::Unavailable(why) => (false, format!("The credential store cannot be read: {why}")),
        });
        match &self.effective {
            Some(Ok(effective)) => {
                let problems = effective.problems(None);
                if problems.is_empty() {
                    lines.push((
                        true,
                        format!(
                            "Checked with the SDK itself: outside a code repository the agent can call only Agentique's {} tools, through Agentique's own server, and is never asked for permissions ({}); no settings, hooks, plugins or memory of this machine are loaded",
                            effective.tools.len(),
                            effective.permission_mode
                        ),
                    ));
                    lines.push((
                        true,
                        "In a project with a code repository it is a development session (C-53): the SDK's own tools (files, commands, subagents, skills) under the project's permission policy, the model files protected, commands only with trusted-local execution, and the model's key kept out of its commands".into(),
                    ));
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
        let endpoint = model_access(self.settings.text("assistant.provider").as_str());
        let provider = endpoint
            .as_ref()
            .map(|e| e.provider)
            .unwrap_or(Provider::Anthropic);
        // The model and effort chosen for the runtime's provider carry over;
        // otherwise its default (the SDK's for Anthropic).
        let model = (choice.model.provider == provider)
            .then(|| choice.model.model.clone())
            .filter(|m| !m.is_empty())
            .or_else(|| endpoint.as_ref().map(|_| DEEPSEEK_MODEL.to_string()));
        let effort = choice
            .effort
            .clone()
            .filter(|e| matches!(e.as_str(), "low" | "medium" | "high" | "xhigh" | "max"));
        let node = claude_agent::find_node();
        let installation = Installation {
            root: Installation::default_root(),
        };
        let name = provider.name();
        let problem = match (
            &node,
            installation.installed(),
            agq_providers::key_status(provider),
        ) {
            (Err(why), _, _) => Some(why.clone()),
            (_, false, _) => Some(
                "The Claude Agent runtime is not installed: Settings › Assistant › Install.".into(),
            ),
            (_, _, KeyStatus::Missing) => Some(format!(
                "The Claude Agent runtime needs a {name} key: add it in Settings › Providers › {name}. Everything else works as usual."
            )),
            (_, _, KeyStatus::Unavailable(why)) => {
                Some(format!("The {name} key cannot be read: {why}"))
            }
            _ => None,
        };
        let data = self.claude_agent_data();
        let through = endpoint
            .as_ref()
            .map(|e| format!(" (through {})", e.provider.name()))
            .unwrap_or_default();
        let label = match &model {
            Some(model) => format!("Claude Agent · {model}{through}"),
            None => format!("Claude Agent{through}"),
        };
        let node = node.ok();
        let inputs = self.conversation.inputs.clone();
        self.conversation.use_runtime(
            label,
            problem,
            Box::new(move || -> Box<dyn agq_assistant::Runtime> {
                match (node.clone(), agq_providers::runtime_key(provider)) {
                    (Some(node), Ok(Some(key))) => {
                        let mut agent = ClaudeAgent::new(
                            node,
                            installation.clone(),
                            data.clone(),
                            model.clone(),
                            effort.clone(),
                            key,
                        );
                        agent.endpoint = endpoint.clone();
                        agent.development = inputs.borrow().development.clone();
                        agent.steering = inputs.borrow().steering.clone().unwrap_or_default();
                        Box::new(agent)
                    }
                    (_, result) => Box::new(Unavailable(match result {
                        Err(why) => format!("The {} key cannot be read: {why}", provider.name()),
                        _ => "The Claude Agent runtime is not ready: see Settings › Assistant."
                            .into(),
                    })),
                }
            }),
        );
    }

    /// The development session the Conversation's next turn gets (C-53): in
    /// a project whose code repository is known from its implementation
    /// links (Agentique's own repository has them), the SDK's own tools work there under
    /// the project's permission policy: read and write in the repository,
    /// never the model files, the agent configuration or the paths the links
    /// protect; commands only with trusted-local execution; no pushing from
    /// the Conversation; anything else asked of the Operator. `None` leaves
    /// Agentique's tools only.
    pub fn conversation_development(&self) -> Option<Development> {
        if self.runtime_setting() != CLAUDE_AGENT || self.safe_mode {
            return None;
        }
        let project = self.project.as_ref()?;
        // Code is known only through the implementation links (Agentique's
        // own repository has them); a modelling project's folder is no
        // place for file tools.
        project.links()?;
        // Plain paths: a verbatim `\\?\` path is the same folder, but the
        // SDK and the session's commands read the plain form.
        let repository = plain(&self.implementation_repository()?);
        let choice = self
            .implementation
            .choice
            .unwrap_or_else(|| self.execution_file_choice());
        let protected = self
            .implementation_links()
            .map(|links| links.protected)
            .unwrap_or_default();
        let folder = project.folder();
        let folder = plain(
            &folder
                .canonicalize()
                .unwrap_or_else(|_| folder.to_path_buf()),
        );
        let also_read: Vec<PathBuf> = if folder.starts_with(&repository) {
            Vec::new()
        } else {
            vec![folder]
        };
        Some(Development {
            cwd: repository.clone(),
            policy: Policy::development(
                &repository,
                &also_read,
                &protected,
                Place::WorkingCopy,
                Permissions {
                    commands: choice.trusted,
                    network: choice.network,
                    push: false,
                    mcp_servers: Vec::new(),
                    undecided: Undecided::Ask,
                },
            ),
            setting_sources: vec!["project".into()],
            agents: serde_json::json!({}),
            preset: true,
        })
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

#[cfg(test)]
mod tests {
    use crate::edit::app_tests::studio;
    use crate::studio::Sample;
    use agq_assistant::policy::{MODEL_FILES, Undecided};

    /// A project with its code (the screening sample "And its code") gets a
    /// development session on the Claude Agent runtime: it works in the code
    /// repository, reads the model folder, never writes model files or the
    /// links' protected paths, runs commands only once trusted-local
    /// execution is on, and asks about anything else. On the loop, in safe
    /// mode or without a code repository it gets none.
    #[test]
    fn a_project_with_code_gets_a_development_session_within_its_permissions() {
        let (mut app, folder) = studio("development-session");
        assert!(
            app.conversation_development().is_none(),
            "the loop has none"
        );
        app.settings
            .set("assistant.runtime", super::CLAUDE_AGENT.into())
            .unwrap();
        assert!(
            app.conversation_development().is_none(),
            "a project without a code repository has none"
        );
        app.create_sample(
            &folder.0.join("Shortener"),
            "Shortener",
            Sample::ScreeningWithCode,
        );
        let code = super::plain(&folder.0.join("Shortener-code").canonicalize().unwrap());
        assert!(
            !code.display().to_string().starts_with(r"\\?\"),
            "plain paths for the SDK"
        );
        let development = app
            .conversation_development()
            .expect("a development session");
        assert_eq!(development.cwd, code);
        assert_eq!(development.policy.write, vec![code.clone()]);
        assert!(
            development.policy.read.len() == 2,
            "{:?}",
            development.policy.read
        );
        for path in MODEL_FILES {
            assert!(development.policy.protected.contains(&path.to_string()));
        }
        assert!(
            !development.policy.commands,
            "nothing runs before trusted-local execution"
        );
        assert_eq!(development.policy.undecided, Undecided::Ask);
        assert_eq!(development.setting_sources, vec!["project".to_string()]);
        let mut choice = app.execution_choice();
        choice.trusted = true;
        app.set_execution_choice(choice);
        assert!(app.conversation_development().unwrap().policy.commands);
        app.safe_mode = true;
        assert!(
            app.conversation_development().is_none(),
            "safe mode has none"
        );
    }
}
