//! The Assistant's runtime in the Studio (ROADMAP §4.7, §4.10, C-51, C-53):
//! which runtime runs the Conversation's turns, where the Claude Agent
//! runtime's model answers (Anthropic, or DeepSeek's Anthropic-compatible
//! endpoint), the development session it gets in a project with a code
//! repository, and its setup and health check for Settings. The runtime
//! itself is `agq_assistant::claude_agent`; this decides when to use it and
//! says what it needs. Since C-54 also: which Anthropic credential sessions
//! use (an API key, or the Operator's Claude subscription token), whether
//! this computer has a Claude login (never used), and each Orchestrator
//! role's model, resolved from Settings and the credentials.

use crate::studio::{Dirty, Studio};
use agq_assistant::claude_agent::{
    self, ClaudeAgent, Effective, Installation, Login, Node, Verified,
};
use agq_assistant::policy::{Development, Endpoint, Permissions, Place, Policy, Undecided};
use agq_orchestrator::record::RoleModel;
use agq_providers::{Credential, KeyStatus, Provider, Secret};
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

/// Every credential Agentique may hold: each provider's key, and the Claude
/// subscription token.
pub fn credentials() -> impl Iterator<Item = Credential> {
    Provider::ALL
        .into_iter()
        .map(Credential::Key)
        .chain([Credential::ClaudeSubscription])
}

/// Whether a key or token is there.
fn present(status: KeyStatus) -> bool {
    matches!(
        status,
        KeyStatus::Stored | KeyStatus::FromEnvironment { .. }
    )
}

/// Where the Claude Agent runtime's model answers, from the Settings'
/// provider and the keys there are (`status`): Anthropic's API, or
/// DeepSeek's Anthropic-compatible endpoint (C-53). With no provider chosen,
/// Anthropic when an API key or the Claude subscription token is there
/// (C-54), else DeepSeek when its key is.
pub fn model_access(provider: &str, status: &dyn Fn(Credential) -> KeyStatus) -> Option<Endpoint> {
    let present = |credential| present(status(credential));
    let anthropic =
        present(Credential::Key(Provider::Anthropic)) || present(Credential::ClaudeSubscription);
    match provider {
        "deepseek" => Some(Endpoint::deepseek()),
        "anthropic" => None,
        _ if !anthropic && present(Credential::Key(Provider::DeepSeek)) => {
            Some(Endpoint::deepseek())
        }
        _ => None,
    }
}

/// Where the Claude Agent runtime reaches `provider`'s models: Anthropic's
/// own API (`None`), or the provider's Anthropic-compatible endpoint.
pub fn endpoint(provider: Provider) -> Result<Option<Endpoint>, String> {
    match provider {
        Provider::Anthropic => Ok(None),
        Provider::DeepSeek => Ok(Some(Endpoint::deepseek())),
        other => Err(format!(
            "the Claude Agent runtime does not reach {}'s models",
            other.name()
        )),
    }
}

/// A Claude Agent runtime on `credential`, with that credential's secret:
/// a key (and its provider's endpoint), or the Claude subscription token on
/// Anthropic's own API (C-54). Exactly one credential reaches the session.
pub fn runtime_on(
    credential: Credential,
    secret: Secret,
    node: Node,
    installation: Installation,
    data: PathBuf,
    model: Option<String>,
    effort: Option<String>,
) -> Result<ClaudeAgent, String> {
    Ok(match credential {
        Credential::ClaudeSubscription => {
            ClaudeAgent::new(node, installation, data, model, effort, Secret::new(""))
                .with_subscription(secret)
        }
        Credential::Key(provider) => {
            let mut agent = ClaudeAgent::new(node, installation, data, model, effort, secret);
            agent.endpoint = endpoint(provider)?;
            agent
        }
    })
}

/// Why a Claude login on this computer is not used, as Settings say it.
pub fn login_line(login: Option<&Result<Login, String>>) -> String {
    match login {
        Some(Ok(login)) => match login.describe() {
            Some(how) => format!(
                "Claude login on this computer: {how}. Not used: Anthropic does not allow products built on the Claude Agent SDK to offer claude.ai login without its approval. To use Claude models, add an Anthropic API key from the Claude Console, or your own Claude subscription token from `claude setup-token`, in Settings › Providers › Anthropic."
            ),
            None => "No Claude login on this computer (Agentique would not use one: it uses only the keys and token in Settings or the environment).".into(),
        },
        Some(Err(why)) => format!("Whether this computer has a Claude login could not be checked: {why}"),
        None => "Checking whether this computer has a Claude login…".into(),
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
            KeyStatus::Stored => (true, "The Assistant's credential is in the Windows Credential Manager".into()),
            KeyStatus::FromEnvironment { variable } => (true, format!("The Assistant's credential comes from {variable}")),
            KeyStatus::Missing => (false, "No credential for the runtime's model: add an Anthropic API key or your Claude subscription token (from `claude setup-token`) in Settings › Providers › Anthropic, or a DeepSeek key to use DeepSeek's Anthropic-compatible endpoint. This computer's own Claude login is not used.".into()),
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
                    lines.push((
                        true,
                        format!(
                            "Every session uses only the one credential Agentique gives it, checked against what the SDK reports before its first model call (here: {}); one that would use another, such as this computer's own Claude login, stops",
                            effective.credential()
                        ),
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

/// Who pays for a provider's usage, in Settings' words (C-54).
fn billed(credential: Credential) -> String {
    match credential {
        Credential::ClaudeSubscription => "counts against your Claude plan's usage limits; Agentique shows its usage at API prices (API-equivalent)".into(),
        Credential::Key(Provider::Anthropic) => "billed per token to the Anthropic Console account of the key (Agentique cannot see which organisation)".into(),
        Credential::Key(provider) => format!("billed per token to the {} account of the key", provider.name()),
    }
}

impl Studio {
    /// The credentials of the providers the agents use (the Assistant's,
    /// and each role's model and fallback), where each comes from and who
    /// pays, then this computer's Claude login (C-54): Settings' lines,
    /// (fine, text).
    pub fn credential_lines(&self) -> Vec<(bool, String)> {
        let mut providers = vec![
            self.runtime_endpoint()
                .map(|e| e.provider)
                .unwrap_or(Provider::Anthropic),
        ];
        for configured in self.settings.agent_configuration() {
            providers.push(configured.model.model.provider);
            providers.extend(configured.fallback.map(|f| f.model.provider));
        }
        providers.sort();
        providers.dedup();
        let mut lines = Vec::new();
        let mut credentials: Vec<Credential> =
            providers.iter().map(|p| Credential::Key(*p)).collect();
        if providers.contains(&Provider::Anthropic) {
            credentials.insert(1, Credential::ClaudeSubscription);
        }
        for credential in credentials {
            let name = credential.name();
            lines.push(match self.credential(credential) {
                KeyStatus::FromEnvironment { variable } => (
                    true,
                    format!("{name}: from {variable}; {}.", billed(credential)),
                ),
                KeyStatus::Stored => (
                    true,
                    format!(
                        "{name}: saved in the Windows Credential Manager; {}.",
                        billed(credential)
                    ),
                ),
                KeyStatus::Missing => (false, format!("{name}: none (Settings › Providers).")),
                KeyStatus::Unavailable(why) => (
                    false,
                    format!("{name}: the credential store cannot be read ({why})."),
                ),
            });
        }
        if providers.contains(&Provider::Anthropic) {
            lines.push((
                true,
                format!(
                    "Claude Agent runtime sessions on Anthropic use the {}; the roles that call their model directly (escalation, the explorer) and Agentique's own loop need the API key.",
                    match self.anthropic_credential() {
                        Credential::ClaudeSubscription => "Claude subscription token",
                        _ => "API key",
                    }
                ),
            ));
        }
        lines.push((
            !matches!(&self.runtime.login, Some(Err(_))),
            login_line(self.runtime.login.as_ref()),
        ));
        lines
    }
}

/// Setup work running on its own thread.
pub enum Work {
    Check(Receiver<Health>),
    Install(Receiver<Result<Health, String>>, Arc<AtomicBool>),
}

/// Where each credential comes from.
pub type Credentials = std::collections::BTreeMap<Credential, KeyStatus>;

/// Every credential's status, read now (the environment, else the store:
/// each read may wait for the store up to its timeout).
fn read_all() -> Credentials {
    credentials()
        .map(|c| (c, agq_providers::credential_status(c)))
        .collect()
}

#[derive(Default)]
pub struct RuntimeState {
    pub health: Option<Health>,
    pub work: Option<Work>,
    /// The Operator asked to install; the card asks once more.
    pub confirm_install: bool,
    pub message: Option<String>,
    /// Whether this computer has a Claude login, once checked: shown, and
    /// named in a fallback's reason, never used (C-54).
    pub login: Option<Result<Login, String>>,
    /// The check of the Claude login running on its own thread, beside any
    /// other setup work.
    pub login_probe: Option<Receiver<Result<Login, String>>>,
    /// Where each key and the subscription token come from, as last read
    /// (C-54): read on a thread of its own when the Studio starts, when
    /// Settings or the Objectives panel opens and when a key or the token is
    /// saved or removed, and read at once before an objective starts;
    /// drawing never reads the store.
    pub credentials: Credentials,
    /// Node.js as last looked for (on that thread, or by Check), so the
    /// runtime choice never starts `node --version` on the window's thread;
    /// `None` until it is known.
    pub node: Option<Result<Node, String>>,
    /// That reading, running.
    pub reading: Option<Receiver<(Credentials, Result<Node, String>)>>,
    /// The Assistant's route on the Claude Agent runtime, as the runtime
    /// choice last made it: its label and why it is not the configured
    /// one (C-54); `None` on Agentique's own loop.
    pub assistant: Option<(String, Option<String>)>,
}

impl Studio {
    /// The runtime the Settings choose.
    pub fn runtime_setting(&self) -> String {
        match self.settings.text("assistant.runtime").as_str() {
            CLAUDE_AGENT => CLAUDE_AGENT.into(),
            _ => LOOP.into(),
        }
    }

    /// Where `credential` comes from, as last read ([`read_credentials`]
    /// (Self::read_credentials)); never reads the store.
    pub fn credential(&self, credential: Credential) -> KeyStatus {
        self.runtime
            .credentials
            .get(&credential)
            .cloned()
            .unwrap_or(KeyStatus::Missing)
    }

    /// Reads where each key and the token come from again, and looks for
    /// Node.js, on a thread of its own (each store read may wait up to its
    /// timeout); the next tick applies what changed ([`poll_runtime`]
    /// (Self::poll_runtime)). One reading at a time.
    pub fn refresh_credentials(&mut self) {
        if self.runtime.reading.is_some() {
            return;
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send((read_all(), claude_agent::find_node()));
        });
        self.runtime.reading = Some(receiver);
    }

    /// Reads where each key and the token come from at once (before an
    /// objective starts, so its record is exact: an act of the Operator's,
    /// never drawing), and applies a change to the Assistant's runtime.
    pub fn read_credentials_now(&mut self) {
        let read = read_all();
        if read != self.runtime.credentials {
            self.runtime.credentials = read;
            self.apply_runtime_choice();
        }
    }

    /// Takes a finished reading: the runtime choice is made again only
    /// when a credential or Node.js changed. Returns whether anything did.
    pub fn poll_credentials(&mut self) -> bool {
        let Some(receiver) = &self.runtime.reading else {
            return false;
        };
        let (read, node) = match receiver.try_recv() {
            Ok(read) => read,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                self.runtime.reading = None;
                return false;
            }
        };
        self.runtime.reading = None;
        let changed = read != self.runtime.credentials || self.runtime.node.as_ref() != Some(&node);
        if changed {
            self.runtime.credentials = read;
            self.runtime.node = Some(node);
            self.apply_runtime_choice();
            self.mark(Dirty::LAYOUT | Dirty::STATUS);
        }
        changed
    }

    /// Whether setup work beside the runtime's own runs on a thread: the
    /// Claude login's check, or a reading of the credentials.
    pub fn runtime_reading(&self) -> bool {
        self.runtime.login_probe.is_some() || self.runtime.reading.is_some()
    }

    /// Where the Claude Agent runtime's model answers for the Assistant,
    /// from the Settings and the credentials as last read.
    pub fn runtime_endpoint(&self) -> Option<Endpoint> {
        model_access(self.settings.text("assistant.provider").as_str(), &|c| {
            self.credential(c)
        })
    }

    /// Which Anthropic credential the Claude Agent runtime's sessions use
    /// (C-54): the one Settings prefer when both are there (the Claude
    /// subscription token by default), else whichever is there; the
    /// preferred one when neither is.
    pub fn anthropic_credential(&self) -> Credential {
        let key = Credential::Key(Provider::Anthropic);
        let token = Credential::ClaudeSubscription;
        let (first, second) = if self.settings.text("providers.anthropic.credential") == "key" {
            (key, token)
        } else {
            (token, key)
        };
        if present(self.credential(first)) || !present(self.credential(second)) {
            first
        } else {
            second
        }
    }

    /// Starts checking whether this computer has a Claude login, on its own
    /// thread, unless that is known or being checked (Settings and the
    /// Objectives panel show it; C-54). Other setup work goes on beside it.
    pub fn probe_login(&mut self) {
        // A test instance reads nothing of the Operator's credentials.
        if self.runtime.login.is_some()
            || self.runtime.login_probe.is_some()
            || self.safe_mode
            || self.args.test_instance
        {
            return;
        }
        let installation = Installation {
            root: Installation::default_root(),
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        // Node is looked for on the probe's thread too.
        std::thread::spawn(move || {
            let login = claude_agent::find_node().and_then(|node| {
                if installation.installed() {
                    claude_agent::login(&node, &installation)
                } else {
                    Err("the Claude Agent runtime is not installed (Settings › Assistant)".into())
                }
            });
            let _ = sender.send(login);
        });
        self.runtime.login_probe = Some(receiver);
    }

    /// Takes the Claude login's check when it has ended. Returns whether it
    /// did.
    pub fn poll_login(&mut self) -> bool {
        let Some(receiver) = &self.runtime.login_probe else {
            return false;
        };
        let login = match receiver.try_recv() {
            Ok(login) => login,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                Err("the check stopped without a result".to_string())
            }
        };
        self.runtime.login_probe = None;
        self.runtime.login = Some(login);
        self.apply_runtime_choice();
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
        true
    }

    /// Each Orchestrator role's model as Settings configure it and the
    /// credentials allow (C-54), for an objective that explores or not;
    /// or each role it needs that has neither its model nor its fallback,
    /// named with what is missing. Whether this computer has a Claude login
    /// is said when it is known.
    pub fn agent_models(
        &self,
        explore: bool,
    ) -> Result<agq_orchestrator::models::Resolved, Vec<String>> {
        self.resolving(|configured, credentials| {
            agq_orchestrator::models::resolve(configured, credentials, explore)
        })
    }

    /// [`agent_models`](Self::agent_models), role by role, for Settings and
    /// the Objectives panel to show each one before an objective starts.
    pub fn agent_models_each(&self) -> Vec<(&'static str, Result<RoleModel, String>)> {
        self.resolving(agq_orchestrator::models::resolve_each)
    }

    /// Runs a resolution with the roles as Settings configure them, the
    /// credentials as last read, and this computer's Claude login.
    fn resolving<T>(
        &self,
        resolve: impl FnOnce(
            &[agq_orchestrator::models::Configured],
            &agq_orchestrator::models::Credentials,
        ) -> T,
    ) -> T {
        let configured = self.settings.agent_configuration();
        let status = |credential: Credential| self.credential(credential);
        let credentials = agq_orchestrator::models::Credentials {
            status: &status,
            prefer_subscription: self.settings.text("providers.anthropic.credential") != "key",
            login: self.runtime.login.as_ref().and_then(|l| l.as_ref().ok()),
        };
        resolve(&configured, &credentials)
    }

    /// The Assistant's model on the Claude Agent runtime, and why it is not
    /// the configured one when it is not (C-54), as the runtime choice last
    /// made it. `None` on Agentique's own loop.
    pub fn assistant_route(&self) -> Option<(String, Option<String>)> {
        self.runtime.assistant.clone()
    }

    /// [`assistant_route`](Self::assistant_route), worked out.
    fn route_now(&self) -> Option<(String, Option<String>)> {
        if self.runtime_setting() != CLAUDE_AGENT || self.safe_mode {
            return None;
        }
        let configured = self.settings.text("assistant.provider");
        let endpoint = self.runtime_endpoint();
        let credential = match &endpoint {
            Some(endpoint) => Credential::Key(endpoint.provider),
            None => self.anthropic_credential(),
        };
        let (model, _) = self.runtime_model(endpoint.as_ref());
        let model = model.unwrap_or_else(|| "the SDK's default".into());
        let label = match credential {
            Credential::ClaudeSubscription => {
                format!("{model} · Claude subscription (your plan's limits apply)")
            }
            Credential::Key(provider) => format!("{model} · {} key", provider.name()),
        };
        let reason = (configured.is_empty() && endpoint.is_some()).then(|| {
            let login = match &self.runtime.login {
                Some(Ok(login)) => login.describe().map(|how| {
                    format!(
                        "; the Claude login on this computer ({how}) is not used: Anthropic does not allow products built on the Claude Agent SDK to offer claude.ai login without its approval"
                    )
                }),
                _ => None,
            };
            format!(
                "No Anthropic API key or Claude subscription token, so the Assistant runs on DeepSeek's Anthropic-compatible endpoint{}.",
                login.unwrap_or_default()
            )
        });
        Some((label, reason))
    }

    /// The model and effort the Claude Agent runtime's turns use: the
    /// Settings' model and effort when they are the runtime's provider's,
    /// otherwise its default (Sonnet 5.5 on Anthropic since C-54, as the
    /// provider's default; `deepseek-v4-pro` on DeepSeek's endpoint).
    fn runtime_model(&self, endpoint: Option<&Endpoint>) -> (Option<String>, Option<String>) {
        let choice = self.settings.model_choice();
        let provider = endpoint.map(|e| e.provider).unwrap_or(Provider::Anthropic);
        let model = (choice.model.provider == provider)
            .then(|| choice.model.model.clone())
            .filter(|m| !m.is_empty())
            .or_else(|| match endpoint {
                Some(_) => Some(DEEPSEEK_MODEL.to_string()),
                None => Some(Provider::Anthropic.default_model().to_string()),
            });
        let effort = choice
            .effort
            .clone()
            .filter(|e| matches!(e.as_str(), "low" | "medium" | "high" | "xhigh" | "max"));
        (model, effort)
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
        self.runtime.assistant = self.route_now();
        if self.runtime_setting() != CLAUDE_AGENT || self.safe_mode {
            self.conversation.use_choice(choice);
            return;
        }
        let endpoint = self.runtime_endpoint();
        let provider = endpoint
            .as_ref()
            .map(|e| e.provider)
            .unwrap_or(Provider::Anthropic);
        // The one credential its sessions get: the endpoint's provider's
        // key, or the Anthropic credential Settings choose (C-54).
        let credential = match &endpoint {
            Some(_) => Credential::Key(provider),
            None => self.anthropic_credential(),
        };
        // The model and effort chosen for the runtime's provider carry over;
        // otherwise its default.
        let (model, effort) = self.runtime_model(endpoint.as_ref());
        // Node as last looked for, never looked for here (the window's
        // thread); until it is known, a turn waits for it.
        let node = self.runtime.node.clone().unwrap_or_else(|| {
            Err("Agentique is still looking for Node.js; try again in a moment.".into())
        });
        let installation = Installation {
            root: Installation::default_root(),
        };
        let name = provider.name();
        let problem = match (&node, installation.installed(), self.credential(credential)) {
            (Err(why), _, _) => Some(why.clone()),
            (_, false, _) => Some(
                "The Claude Agent runtime is not installed: Settings › Assistant › Install.".into(),
            ),
            (_, _, KeyStatus::Missing) if provider == Provider::Anthropic => Some(
                "The Claude Agent runtime needs an Anthropic API key or your Claude subscription token (from `claude setup-token`): add one in Settings › Providers › Anthropic. Everything else works as usual.".into(),
            ),
            (_, _, KeyStatus::Missing) => Some(format!(
                "The Claude Agent runtime needs a {name} key: add it in Settings › Providers › {name}. Everything else works as usual."
            )),
            (_, _, KeyStatus::Unavailable(why)) => {
                Some(format!("The {} cannot be read: {why}", credential.name()))
            }
            _ => None,
        };
        let data = self.claude_agent_data();
        let through = match (&endpoint, credential) {
            (Some(e), _) => format!(" (through {})", e.provider.name()),
            (None, Credential::ClaudeSubscription) => " (Claude subscription)".into(),
            (None, _) => String::new(),
        };
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
                let agent = match (node.clone(), agq_providers::runtime_credential(credential)) {
                    (Some(node), Ok(Some(secret))) => runtime_on(
                        credential,
                        secret,
                        node,
                        installation.clone(),
                        data.clone(),
                        model.clone(),
                        effort.clone(),
                    ),
                    (_, Err(why)) => {
                        Err(format!("The {} cannot be read: {why}", credential.name()))
                    }
                    _ => Err(
                        "The Claude Agent runtime is not ready: see Settings › Assistant.".into(),
                    ),
                };
                match agent {
                    Ok(mut agent) => {
                        agent.development = inputs.borrow().development.clone();
                        agent.steering = inputs.borrow().steering.clone().unwrap_or_default();
                        Box::new(agent)
                    }
                    Err(why) => Box::new(Unavailable(why)),
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
            env: Vec::new(),
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
        let login = self.poll_login() | self.poll_credentials();
        let Some(work) = &self.runtime.work else {
            return login;
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
            return login;
        };
        self.runtime.work = None;
        match result {
            Ok(health) => {
                self.runtime.message = None;
                if let Some(node) = &health.node {
                    self.runtime.node = Some(node.clone());
                }
                self.runtime.health = Some(health);
            }
            Err(why) => self.runtime.message = Some(why),
        }
        self.apply_runtime_choice();
        // A check (or an installation) looks at the Claude login again.
        self.runtime.login = None;
        self.probe_login();
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

    /// The review of W12.3: what Settings and the Objectives panel draw
    /// (each role's model, the credential lines, the Assistant's route)
    /// comes from the credentials as last read, so drawing reads no store;
    /// reading them again is an act of its own.
    #[test]
    fn drawing_uses_the_credentials_as_last_read() {
        use agq_orchestrator::record::Access;
        use agq_providers::{Credential, KeyStatus, Provider};
        let (mut app, _folder) = studio("credentials-as-read");
        // What the store and environment hold here does not matter: the
        // last read says the subscription token and the DeepSeek and
        // TypeSafe AI keys are there, and nothing else.
        app.runtime.credentials = super::credentials()
            .map(|c| (c, KeyStatus::Missing))
            .collect();
        for credential in [
            Credential::ClaudeSubscription,
            Credential::Key(Provider::DeepSeek),
            Credential::Key(Provider::TypeSafe),
        ] {
            app.runtime
                .credentials
                .insert(credential, KeyStatus::Stored);
        }
        let each = app.agent_models_each();
        let lead = each[0].1.as_ref().unwrap();
        assert_eq!((each[0].0, lead.access), ("lead", Access::Subscription));
        let escalation = &each.iter().find(|(r, _)| *r == "escalation").unwrap().1;
        assert_eq!(
            escalation.as_ref().unwrap().model.to_string(),
            "deepseek/deepseek-v4-pro"
        );
        let lines = app.credential_lines();
        assert!(
            lines.iter().any(|(fine, l)| *fine
                && l.starts_with(
                    "Claude subscription token: saved in the Windows Credential Manager"
                )),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|(fine, l)| !*fine && l.starts_with("Anthropic key: none")),
            "{lines:?}"
        );
        assert_eq!(app.anthropic_credential(), Credential::ClaudeSubscription);
        // Now the last read says nothing is there: so do the roles.
        for status in app.runtime.credentials.values_mut() {
            *status = KeyStatus::Missing;
        }
        assert!(app.agent_models(false).is_err());
        assert!(app.agent_models_each().iter().all(|(_, m)| m.is_err()));
    }

    /// The review of W12.3: credentials are read on a thread of their own,
    /// and the next tick applies the reading only when a credential or
    /// Node.js changed.
    #[test]
    fn a_reading_is_applied_only_when_something_changed() {
        use agq_providers::{Credential, KeyStatus, Provider};
        let (mut app, _folder) = studio("credentials-read-apart");
        let known: super::Credentials = super::credentials()
            .map(|c| (c, KeyStatus::Missing))
            .collect();
        let node = Err::<agq_assistant::claude_agent::Node, String>("no Node here".into());
        app.runtime.credentials = known.clone();
        app.runtime.node = Some(node.clone());
        // The same reading: nothing to apply.
        let (sender, receiver) = std::sync::mpsc::channel();
        app.runtime.reading = Some(receiver);
        assert!(app.runtime_reading());
        assert!(!app.poll_credentials(), "nothing yet");
        sender.send((known.clone(), node.clone())).unwrap();
        assert!(!app.poll_credentials(), "nothing changed");
        assert!(app.runtime.reading.is_none());
        // A key appeared: applied.
        let mut changed = known;
        changed.insert(Credential::Key(Provider::DeepSeek), KeyStatus::Stored);
        let (sender, receiver) = std::sync::mpsc::channel();
        app.runtime.reading = Some(receiver);
        sender.send((changed, node)).unwrap();
        assert!(app.poll_credentials());
        assert_eq!(
            app.credential(Credential::Key(Provider::DeepSeek)),
            KeyStatus::Stored
        );
    }
}
