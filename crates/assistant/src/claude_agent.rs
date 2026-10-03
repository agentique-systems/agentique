//! The Claude Agent runtime (ROADMAP §4.7, §4.10, C-51; part
//! `ClaudeAgentRuntime`): the official Claude Agent SDK runs the turn's loop
//! in a companion process (`claude-agent/`, TypeScript on Node), and the
//! Studio carries out its tool calls exactly as it does the loop's.
//!
//! - The companion's sources are compiled in ([`COMPANION`]), so a build
//!   carries exactly the companion it was built with. They are written once
//!   per version into the runtime folder, beside the packages installed from
//!   the companion's lock file ([`Installation`]).
//! - [`ClaudeAgent`] is the [`Runtime`]: one companion process per turn,
//!   protocol 1 on its standard input and output, the session resumed from
//!   the conversation's last [`Entry::Session`], and an explicit handoff when
//!   it cannot be.
//! - [`find_node`], [`Installation::install`], [`verify`] and [`probe`] are
//!   Settings' setup and health check.
//!
//! The companion gets an environment built from nothing but what it needs;
//! the Anthropic key goes into it and nowhere else (§7.6). What the agent can
//! call is decided in the companion's policy and by the Studio's own checks;
//! neither is an operating-system sandbox.

use crate::conversation::{Conversation, Entry, ToolResult};
use crate::model::{StreamEvent, Usage};
use crate::runtime::Runtime;
use crate::turn::{ToolCall, Toolset, TurnEvent};
use agq_providers::{AssistantPart, ModelRef, Provider, Secret};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// The protocol the companion speaks (`claude-agent/src/protocol.ts`).
pub const PROTOCOL: u64 = 1;

/// The SDK version the lock file pins; checked against the installed one.
pub const SDK_VERSION: &str = "0.3.287";

/// The oldest Node that runs the companion (type stripping).
pub const NODE_MINIMUM: (u64, u64) = (22, 6);

/// The runtime's name in conversation entries.
pub const RUNTIME: &str = "claude-agent";

/// The companion: its package files and sources, as built in.
pub const COMPANION: [(&str, &str); 6] = [
    (
        "package.json",
        include_str!("../../../claude-agent/package.json"),
    ),
    (
        "package-lock.json",
        include_str!("../../../claude-agent/package-lock.json"),
    ),
    (
        "src/main.ts",
        include_str!("../../../claude-agent/src/main.ts"),
    ),
    (
        "src/bridge.ts",
        include_str!("../../../claude-agent/src/bridge.ts"),
    ),
    (
        "src/policy.ts",
        include_str!("../../../claude-agent/src/policy.ts"),
    ),
    (
        "src/protocol.ts",
        include_str!("../../../claude-agent/src/protocol.ts"),
    ),
];

/// How long the companion may take to say it is ready.
const READY: Duration = Duration::from_secs(60);

const STOPPED: &str = "Stopped by the Operator. Changes made so far stay and can be undone.";

/// FNV-1a, as hex: stable folder names for versions of the companion.
fn digest(parts: &[&str]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.as_bytes().iter().chain(b"\0") {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// The digest of what is installed: the package and lock files.
pub fn packages_digest() -> String {
    digest(&[COMPANION[0].1, COMPANION[1].1])
}

/// The digest of the whole companion as built in: what a build manifest
/// records.
pub fn companion_digest() -> String {
    let all: Vec<&str> = COMPANION.iter().flat_map(|(p, t)| [*p, *t]).collect();
    digest(&all)
}

/// Node.js, found on the PATH.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub path: PathBuf,
    pub version: String,
}

impl Node {
    /// npm, beside Node (as the Node installer puts it).
    pub fn npm(&self) -> PathBuf {
        let folder = self.path.parent().unwrap_or(Path::new("."));
        if cfg!(windows) {
            folder.join("npm.cmd")
        } else {
            folder.join("npm")
        }
    }
}

/// Finds Node on the PATH and checks its version.
pub fn find_node() -> Result<Node, String> {
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    let path = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|folder| folder.join(name))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            format!(
                "Node.js was not found on the PATH. Install Node.js {}.{} or later (nodejs.org), then check again.",
                NODE_MINIMUM.0, NODE_MINIMUM.1
            )
        })?;
    let output = hidden(Command::new(&path))
        .arg("--version")
        .output()
        .map_err(|e| format!("Node.js at {} could not run: {e}", path.display()))?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let numbers: Vec<u64> = version
        .trim_start_matches('v')
        .split('.')
        .filter_map(|n| n.parse().ok())
        .collect();
    let (major, minor) = (
        numbers.first().copied().unwrap_or(0),
        numbers.get(1).copied().unwrap_or(0),
    );
    if (major, minor) < NODE_MINIMUM {
        return Err(format!(
            "Node.js {version} is too old: the Claude Agent runtime needs {}.{} or later.",
            NODE_MINIMUM.0, NODE_MINIMUM.1
        ));
    }
    Ok(Node { path, version })
}

/// A command that opens no console window (on Windows; elsewhere there is
/// none to hide).
fn hidden(command: Command) -> Command {
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.creation_flags(0x0800_0000);
        command
    };
    command
}

/// Where the runtime's packages and the companion's sources live:
/// `<root>/claude-agent-<packages digest>/` holds `node_modules` (installed
/// once from the lock file, never changed) and one `src-<digest>` folder per
/// version of the companion's sources. Node finds the packages from any of
/// them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installation {
    pub root: PathBuf,
}

impl Installation {
    /// `%LOCALAPPDATA%\Agentique\runtime`.
    pub fn default_root() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
            .unwrap_or_else(std::env::temp_dir)
            .join("Agentique")
            .join("runtime")
    }

    pub fn folder(&self) -> PathBuf {
        self.root
            .join(format!("{RUNTIME}-{}", &packages_digest()[..12]))
    }

    /// The installed SDK's version, if it is installed.
    pub fn installed_sdk(&self) -> Option<String> {
        let package = self
            .folder()
            .join("node_modules/@anthropic-ai/claude-agent-sdk/package.json");
        let text = std::fs::read_to_string(package).ok()?;
        let value: Value = serde_json::from_str(&text).ok()?;
        value["version"].as_str().map(str::to_string)
    }

    pub fn installed(&self) -> bool {
        self.installed_sdk().as_deref() == Some(SDK_VERSION)
    }

    /// The companion's entry point for this build, written if it is not
    /// there yet (each version once, never changed after).
    pub fn script(&self) -> Result<PathBuf, String> {
        let sources = self
            .folder()
            .join(format!("src-{}", &companion_digest()[..12]));
        let main = sources.join("src/main.ts");
        if main.is_file() {
            return Ok(main);
        }
        let temporary = self.folder().join(format!(
            "src-{}.{}.next",
            &companion_digest()[..12],
            std::process::id()
        ));
        let write = || -> std::io::Result<()> {
            for (path, text) in COMPANION.iter().filter(|(p, _)| p.starts_with("src/")) {
                let file = temporary.join(path);
                std::fs::create_dir_all(file.parent().unwrap_or(&temporary))?;
                std::fs::write(file, text)?;
            }
            std::fs::rename(&temporary, &sources)
        };
        if let Err(error) = write() {
            let _ = std::fs::remove_dir_all(&temporary);
            if !main.is_file() {
                return Err(format!(
                    "The companion could not be written to {}: {error}",
                    sources.display()
                ));
            }
        }
        Ok(main)
    }

    /// Installs the packages the lock file names (`npm ci`, without their
    /// install scripts), downloading them from the npm registry: about
    /// 300 MB, most of it the Claude Code binary. Blocking: run it off the
    /// UI thread. Then checks the binary against the SDK's checksum.
    pub fn install(&self, node: &Node, cancel: &AtomicBool) -> Result<Verified, String> {
        let folder = self.folder();
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        for (path, text) in &COMPANION[..2] {
            std::fs::write(folder.join(path), text).map_err(|e| e.to_string())?;
        }
        let mut child = hidden(Command::new(node.npm()))
            .args([
                "ci",
                "--omit=dev",
                "--ignore-scripts",
                "--no-audit",
                "--no-fund",
            ])
            .current_dir(&folder)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("npm could not start: {e}"))?;
        let stderr = child.stderr.take();
        let errors = std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut stderr) = stderr {
                let _ = std::io::Read::read_to_string(&mut stderr, &mut text);
            }
            text
        });
        let status = loop {
            if cancel.load(Ordering::SeqCst) {
                agq_execution::process::kill_tree(&mut child);
                return Err("The installation was cancelled.".into());
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => return Err(e.to_string()),
            }
        };
        let errors = errors.join().unwrap_or_default();
        if !status.success() {
            return Err(format!(
                "npm could not install the runtime ({status}): {}",
                agq_execution::process::last_lines(&errors, 12)
            ));
        }
        let verified = verify(node, self)?;
        if !verified.checksum_matches {
            return Err(
                "The Claude Code binary does not match the checksum in the SDK's manifest; it is not used."
                    .into(),
            );
        }
        Ok(verified)
    }
}

/// What `main.ts --verify` reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified {
    pub node: String,
    pub sdk: String,
    pub claude_code: String,
    pub checksum_matches: bool,
}

fn companion_output(node: &Node, installation: &Installation, mode: &str) -> Result<Value, String> {
    let script = installation.script()?;
    let output = hidden(Command::new(&node.path))
        .args(["--experimental-strip-types", "--no-warnings"])
        .arg(&script)
        .arg(mode)
        .env_clear()
        .envs(base_environment(node))
        .current_dir(installation.folder())
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("The companion could not start: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .ok_or_else(|| {
            format!(
                "The companion answered nothing: {}",
                agq_execution::process::last_lines(&String::from_utf8_lossy(&output.stderr), 8)
            )
        })?;
    serde_json::from_str(line).map_err(|e| format!("The companion's answer is not JSON: {e}"))
}

/// The installed versions, and the Claude Code binary against the checksum
/// in the SDK's own manifest.
pub fn verify(node: &Node, installation: &Installation) -> Result<Verified, String> {
    if !installation.installed() {
        return Err(format!(
            "The Claude Agent runtime is not installed (SDK {SDK_VERSION})."
        ));
    }
    let value = companion_output(node, installation, "--verify")?;
    Ok(Verified {
        node: value["node"].as_str().unwrap_or_default().into(),
        sdk: value["sdk"].as_str().unwrap_or_default().into(),
        claude_code: value["claudeCode"].as_str().unwrap_or_default().into(),
        checksum_matches: value["checksumMatches"] == true,
    })
}

/// What the agent can actually do, as the SDK itself reported it when it
/// started with the turn's configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effective {
    pub tools: Vec<String>,
    pub mcp_servers: Vec<(String, String)>,
    pub permission_mode: String,
    pub model: String,
    pub claude_code: String,
    pub skills: Vec<String>,
    pub agents: Vec<String>,
}

impl Effective {
    fn from(value: &Value) -> Effective {
        let strings = |field: &str| -> Vec<String> {
            value[field]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        };
        Effective {
            tools: strings("tools"),
            mcp_servers: value["mcpServers"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|s| {
                    (
                        s["name"].as_str().unwrap_or_default().to_string(),
                        s["status"].as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect(),
            permission_mode: value["permissionMode"].as_str().unwrap_or_default().into(),
            model: value["model"].as_str().unwrap_or_default().into(),
            claude_code: value["claudeCode"].as_str().unwrap_or_default().into(),
            skills: strings("skills"),
            agents: strings("agents"),
        }
    }

    /// Problems with what the agent could call: anything but Agentique's
    /// tools, any server but Agentique's, any permission mode but `dontAsk`.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for tool in &self.tools {
            if !tool.starts_with("mcp__agentique__") {
                problems.push(format!("the agent could call `{tool}`"));
            }
        }
        for (server, status) in &self.mcp_servers {
            if server != "agentique" {
                problems.push(format!("an MCP server `{server}` ({status}) was loaded"));
            } else if status != "connected" {
                problems.push(format!("Agentique's tools are {status}, not connected"));
            }
        }
        if self.permission_mode != "dontAsk" {
            problems.push(format!(
                "the permission mode is `{}`, not `dontAsk`",
                self.permission_mode
            ));
        }
        problems
    }
}

/// Starts the SDK exactly as a turn does, with a placeholder key that
/// Anthropic refuses before any model runs (so it costs nothing), and
/// reports what the agent could do. Blocking, up to about a minute.
pub fn probe(node: &Node, installation: &Installation) -> Result<Effective, String> {
    let value = companion_output(node, installation, "--probe")?;
    if value["init"].is_null() {
        return Err("The SDK did not report how it started.".into());
    }
    Ok(Effective::from(&value["init"]))
}

/// The variables every process of the runtime needs on Windows, and Node's
/// folder on the PATH; nothing else of the Studio's environment.
fn base_environment(node: &Node) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = [
        "SystemRoot",
        "SYSTEMROOT",
        "windir",
        "SystemDrive",
        "TEMP",
        "TMP",
        "NUMBER_OF_PROCESSORS",
        "PROCESSOR_ARCHITECTURE",
    ]
    .iter()
    .filter_map(|name| std::env::var(name).ok().map(|v| (name.to_string(), v)))
    .collect();
    let mut path = node
        .path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    if let Ok(root) = std::env::var("SystemRoot") {
        path.push_str(&format!(";{root}\\System32"));
    }
    env.push(("PATH".into(), path));
    env
}

/// The Claude Agent runtime for one conversation or task.
pub struct ClaudeAgent {
    pub node: Node,
    pub installation: Installation,
    /// The runtime's own folder: its configuration and sessions
    /// (`config`), its home (`home`) and its working folder (`work`).
    pub data: PathBuf,
    /// The model id, or `None` for the SDK's default.
    pub model: Option<String>,
    pub effort: Option<String>,
    key: Secret,
    /// What the SDK reported when the last turn started.
    pub effective: Option<Effective>,
    /// Another companion script, for tests of the protocol (a scripted
    /// stand-in); `None` runs the companion as built in.
    pub script: Option<PathBuf>,
}

impl ClaudeAgent {
    pub fn new(
        node: Node,
        installation: Installation,
        data: PathBuf,
        model: Option<String>,
        effort: Option<String>,
        key: Secret,
    ) -> ClaudeAgent {
        ClaudeAgent {
            node,
            installation,
            data,
            model,
            effort,
            key,
            effective: None,
            script: None,
        }
    }

    fn model_ref(&self, model: &str) -> ModelRef {
        ModelRef::new(Provider::Anthropic, model)
    }
}

/// The SDK session a new turn continues: the conversation's last session of
/// this runtime, if every turn since was this runtime's. `None` means a new
/// session, with the visible history handed over. The companion forks the
/// session it continues, so each turn's session holds exactly the turns up
/// to it, and a conversation cut back by an edit or a retry continues from
/// the right one.
pub fn session_to_resume(conversation: &Conversation) -> Option<String> {
    let entries = &conversation.entries;
    // The Operator's message for this turn is the last entry.
    let current = entries
        .iter()
        .rposition(|e| matches!(e, Entry::Operator { .. }))?;
    let session = entries[..current]
        .iter()
        .rposition(|e| matches!(e, Entry::Session { runtime, .. } if runtime == RUNTIME))?;
    // Every turn since that session must have been this runtime's: its
    // Operator message followed at once by a session entry.
    let mut turn_starts = entries[session..current]
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Entry::Operator { .. }))
        .map(|(i, _)| i + session);
    let all_ours = turn_starts.all(|i| {
        matches!(entries.get(i + 1), Some(Entry::Session { runtime, .. }) if runtime == RUNTIME)
    });
    let id = match &entries[session] {
        Entry::Session { id, .. } => id.clone(),
        _ => return None,
    };
    (all_ours && !id.is_empty()).then_some(id)
}

/// The visible history before the Operator's latest message, as text for a
/// new session: what was said and done, never hidden reasoning. The most
/// recent part is kept when it is long.
pub fn handover_text(conversation: &Conversation, limit: usize) -> String {
    let entries = &conversation.entries;
    let end = entries
        .iter()
        .rposition(|e| matches!(e, Entry::Operator { .. }))
        .unwrap_or(entries.len());
    let mut lines = Vec::new();
    for entry in &entries[conversation.transcript.min(end)..end] {
        match entry {
            Entry::Operator { text } => lines.push(format!("Operator: {text}")),
            Entry::Assistant { parts, .. } => {
                for part in parts {
                    match part {
                        AssistantPart::Text { text } if !text.trim().is_empty() => {
                            lines.push(format!("Assistant: {text}"))
                        }
                        AssistantPart::ToolCall { name, input, .. } => {
                            lines.push(format!("Assistant called {name} with {input}"))
                        }
                        _ => {}
                    }
                }
            }
            Entry::ToolResults { results } => {
                for result in results {
                    let mut text: String = result.content.chars().take(400).collect();
                    if result.content.chars().count() > 400 {
                        text.push('…');
                    }
                    lines.push(format!(
                        "{}: {text}",
                        if result.is_error {
                            "Tool error"
                        } else {
                            "Tool result"
                        }
                    ));
                }
            }
            Entry::Notice { text } => lines.push(format!("Notice: {text}")),
            Entry::Session { .. } | Entry::Other(_) => {}
        }
    }
    let mut text = lines.join("\n");
    if text.chars().count() > limit {
        let skip = text.chars().count() - limit;
        text = format!(
            "[… earlier history left out]\n{}",
            text.chars().skip(skip).collect::<String>()
        );
    }
    text
}

/// Agentique's tools as the companion sends them to the SDK.
fn protocol_tools(definitions: &Value) -> Vec<Value> {
    definitions
        .as_array()
        .into_iter()
        .flatten()
        .map(|definition| {
            let name = definition["name"].as_str().unwrap_or_default();
            json!({
                "name": name,
                "description": definition["description"],
                "inputSchema": definition["input_schema"],
                "readOnly": crate::tools::read_only(name),
            })
        })
        .collect()
}

/// An SDK message's content blocks as Agentique's own: tool names without
/// the MCP prefix, and thinking kept for the Operator to read but without
/// its signature, so it is never sent back to a model.
fn blocks(content: &Value) -> Vec<Value> {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|block| match block["type"].as_str()? {
            "text" => Some(json!({ "type": "text", "text": block["text"] })),
            "tool_use" => Some(json!({
                "type": "tool_use",
                "id": block["id"],
                "name": block["name"].as_str().unwrap_or_default().trim_start_matches("mcp__agentique__"),
                "input": block["input"],
            })),
            "thinking" => Some(json!({ "type": "thinking", "thinking": block["thinking"] })),
            _ => None,
        })
        .collect()
}

/// A running companion: its input, its messages as they arrive, and what it
/// wrote to its error output.
struct Companion {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    stderr: std::sync::Arc<std::sync::Mutex<String>>,
}

impl Companion {
    fn start(agent: &ClaudeAgent) -> Result<Companion, String> {
        let script = match &agent.script {
            Some(script) => script.clone(),
            None => agent.installation.script()?,
        };
        let mut env = base_environment(&agent.node);
        env.push(("ANTHROPIC_API_KEY".into(), agent.key.expose().to_string()));
        let mut child = hidden(Command::new(&agent.node.path))
            .args(["--experimental-strip-types", "--no-warnings"])
            .arg(&script)
            .env_clear()
            .envs(env)
            .current_dir(agent.data.join("work"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("The Claude Agent runtime could not start: {e}"))?;
        let stdout = child.stdout.take().expect("piped");
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let stderr = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = stderr.clone();
        let mut err = child.stderr.take().expect("piped");
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = std::io::Read::read_to_string(&mut err, &mut text);
            if let Ok(mut s) = sink.lock() {
                s.push_str(&text);
            }
        });
        Ok(Companion {
            stdin: child.stdin.take(),
            child,
            lines,
            stderr,
        })
    }

    fn send(&mut self, message: &Value) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("the runtime's input is closed")?;
        writeln!(stdin, "{message}")
            .and_then(|_| stdin.flush())
            .map_err(|e| format!("the runtime stopped reading: {e}"))
    }

    fn errors(&self) -> String {
        self.stderr.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Drop for Companion {
    fn drop(&mut self) {
        self.stdin = None;
        // A turn that ended on its own exits by itself; give it a moment.
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        agq_execution::process::kill_tree(&mut self.child);
    }
}

impl Runtime for ClaudeAgent {
    fn model(&self) -> Option<ModelRef> {
        self.model.as_deref().map(|m| self.model_ref(m))
    }

    fn label(&self) -> String {
        match &self.model {
            Some(model) => format!("Claude Agent · {model}"),
            None => "Claude Agent".into(),
        }
    }

    fn run(
        &mut self,
        conversation: &mut Conversation,
        toolset: &Toolset,
        max_calls: usize,
        execute: &mut dyn FnMut(&ToolCall) -> ToolResult,
        on_event: &mut dyn FnMut(TurnEvent),
        stop: &AtomicBool,
    ) {
        let mut turn = AgentTurn {
            conversation,
            on_event,
            pending: Vec::new(),
            results: Vec::new(),
            author: self.model(),
            text: String::new(),
        };
        let Some(Entry::Operator { text: prompt }) = turn
            .conversation
            .entries
            .iter()
            .rev()
            .find(|e| matches!(e, Entry::Operator { .. }))
            .cloned()
        else {
            turn.notice("Write a message for the Assistant to answer.");
            return;
        };
        for folder in ["config", "home", "work"] {
            if let Err(error) = std::fs::create_dir_all(self.data.join(folder)) {
                turn.notice(&format!(
                    "The Claude Agent runtime's folder could not be made: {error}"
                ));
                return;
            }
        }
        let resume = session_to_resume(turn.conversation);
        let message = match &resume {
            Some(_) => format!(
                "{prompt}\n\n(Agentique: this continues your earlier session. The model and the code may have changed since; read them again before you change anything.)"
            ),
            None => {
                let history = handover_text(turn.conversation, 24_000);
                if history.trim().is_empty() {
                    prompt.clone()
                } else {
                    format!(
                        "Earlier in this conversation (the visible history, given as text; nothing hidden carried over):\n\n{history}\n\nThe Operator now writes:\n\n{prompt}"
                    )
                }
            }
        };
        let mut companion = match Companion::start(self) {
            Ok(companion) => companion,
            Err(error) => {
                turn.notice(&error);
                return;
            }
        };
        // Ready, in this protocol.
        match companion.lines.recv_timeout(READY) {
            Ok(line) => {
                let ready: Value = serde_json::from_str(&line).unwrap_or_default();
                if ready["type"] != "ready" || ready["protocol"] != PROTOCOL {
                    turn.notice(&format!(
                        "The Claude Agent runtime speaks another protocol ({line}); this Agentique speaks protocol {PROTOCOL}."
                    ));
                    return;
                }
            }
            Err(_) => {
                turn.notice(&format!(
                    "The Claude Agent runtime did not start: {}",
                    agq_execution::process::last_lines(&companion.errors(), 8)
                ));
                return;
            }
        }
        let start = json!({
            "type": "start",
            "options": {
                "prompt": message,
                "systemPrompt": toolset.system,
                "tools": protocol_tools(&toolset.definitions),
                "model": self.model,
                "effort": self.effort,
                "resume": resume,
                "maxTurns": max_calls,
                "cwd": self.data.join("work"),
                "configDir": self.data.join("config"),
                "home": self.data.join("home"),
            }
        });
        if let Err(error) = companion.send(&start) {
            turn.notice(&format!(
                "The Claude Agent runtime did not take the turn: {error}"
            ));
            return;
        }
        let mut interrupted = false;
        let mut interrupted_at: Option<std::time::Instant> = None;
        let mut ended = false;
        loop {
            if stop.load(Ordering::SeqCst) && !interrupted {
                interrupted = true;
                let _ = companion.send(&json!({ "type": "interrupt" }));
            }
            // An interrupted runtime gets a few seconds to end the turn; then
            // the companion is ended with its process tree.
            if interrupted {
                let since = *interrupted_at.get_or_insert_with(std::time::Instant::now);
                if since.elapsed() > Duration::from_secs(15) {
                    turn.notice("The Claude Agent runtime did not stop in time; it was ended.");
                    break;
                }
            }
            let line = match companion.lines.recv_timeout(Duration::from_millis(25)) {
                Ok(line) => line,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                turn.notice(&format!(
                    "The Claude Agent runtime wrote something that is not protocol {PROTOCOL}; the turn ends."
                ));
                break;
            };
            match message["type"].as_str().unwrap_or_default() {
                "init" => {
                    let effective = Effective::from(&message);
                    let problems = effective.problems();
                    let session = message["sessionId"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    if !effective.model.is_empty() {
                        turn.author = Some(self.model_ref(&effective.model));
                    }
                    turn.add(Entry::Session {
                        runtime: RUNTIME.into(),
                        id: session,
                        event: if resume.is_some() {
                            "resumed"
                        } else {
                            "started"
                        }
                        .into(),
                    });
                    self.effective = Some(effective);
                    if !problems.is_empty() {
                        // Fail closed: the SDK did not start as configured.
                        let _ = companion.send(&json!({ "type": "interrupt" }));
                        turn.notice(&format!(
                            "The Claude Agent runtime did not start as Agentique configures it ({}); the turn was stopped before anything ran.",
                            problems.join("; ")
                        ));
                        interrupted = true;
                    }
                }
                "text" => {
                    let text = message["text"].as_str().unwrap_or_default().to_string();
                    turn.text.push_str(&text);
                    turn.stream(StreamEvent::Text(text));
                }
                "thinking" => {
                    turn.stream(StreamEvent::Thinking(
                        message["text"].as_str().unwrap_or_default().to_string(),
                    ));
                }
                "assistant" => {
                    turn.flush_results("Not run: the runtime went on without it.");
                    let content = blocks(&message["content"]);
                    if !content.is_empty() {
                        for block in content.iter().filter(|b| b["type"] == "tool_use") {
                            let id = block["id"].as_str().unwrap_or_default().to_string();
                            turn.stream(StreamEvent::ToolCallStarted {
                                id: id.clone(),
                                name: block["name"].as_str().unwrap_or_default().to_string(),
                            });
                            turn.stream(StreamEvent::ToolInput {
                                id: id.clone(),
                                json: block["input"].to_string(),
                            });
                            turn.pending.push(id);
                        }
                        let author = turn.author.clone();
                        turn.add(Entry::reply(author, &content));
                        turn.text.clear();
                    }
                }
                "tool_call" => {
                    let call_id = message["call"].as_str().unwrap_or_default().to_string();
                    let id = message["toolUseId"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("call-{call_id}"));
                    let call = ToolCall {
                        id: id.clone(),
                        name: message["name"].as_str().unwrap_or_default().to_string(),
                        input: message["input"].clone(),
                    };
                    let result = if stop.load(Ordering::SeqCst) || interrupted {
                        ToolResult::error("Not run: the turn was stopped.")
                    } else {
                        execute(&call)
                    };
                    let result = ToolResult {
                        tool_use_id: id.clone(),
                        ..result
                    };
                    let _ = companion.send(&json!({
                        "type": "tool_result",
                        "call": call_id,
                        "content": result.content,
                        "isError": result.is_error,
                    }));
                    turn.finished(result);
                }
                "result" => {
                    turn.flush_results("Not run: the turn ended before this call.");
                    let usage = &message["usage"];
                    turn.stream(StreamEvent::Usage(Usage {
                        input_tokens: usage["inputTokens"].as_u64().unwrap_or(0),
                        cache_creation_input_tokens: usage["cacheWriteTokens"]
                            .as_u64()
                            .unwrap_or(0),
                        cache_read_input_tokens: usage["cacheReadTokens"].as_u64().unwrap_or(0),
                        output_tokens: usage["outputTokens"].as_u64().unwrap_or(0),
                    }));
                    if message["isError"] == true && !interrupted {
                        let errors: Vec<String> = message["errors"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|e| e.as_str().map(str::to_string))
                            .collect();
                        turn.notice(&match message["subtype"].as_str().unwrap_or_default() {
                            "error_max_turns" => format!(
                                "Paused after {max_calls} steps in one turn. Send a message to let the Assistant continue."
                            ),
                            other => format!(
                                "The Claude Agent runtime ended the turn with an error ({other}){}",
                                if errors.is_empty() { String::new() } else { format!(": {}", errors.join("; ")) }
                            ),
                        });
                    }
                    ended = true;
                }
                "error" => {
                    let text = message["message"].as_str().unwrap_or_default();
                    match message["kind"].as_str().unwrap_or_default() {
                        "auth" => turn.notice(
                            "Anthropic refused the API key. Check it in Settings › Providers › Anthropic (or ANTHROPIC_API_KEY); everything else works as usual.",
                        ),
                        "interrupted" => turn.notice(STOPPED),
                        kind => turn.notice(&format!("The Claude Agent runtime failed ({kind}): {text}")),
                    }
                    ended = true;
                }
                // `ready` again, `retry`, `log`: diagnostics, not shown.
                _ => {}
            }
        }
        turn.flush_results("Not run: the runtime stopped.");
        if !turn.text.trim().is_empty() {
            // Text that streamed in before the runtime stopped stays.
            let text = json!({ "type": "text", "text": turn.text.trim_end() });
            let author = turn.author.clone();
            turn.add(Entry::reply(author, &[text]));
        }
        if !ended {
            if interrupted {
                turn.notice(STOPPED);
            } else {
                let status = companion.child.try_wait().ok().flatten();
                turn.notice(&format!(
                    "The Claude Agent runtime stopped unexpectedly{}. {}",
                    status.map(|s| format!(" ({s})")).unwrap_or_default(),
                    agq_execution::process::last_lines(&companion.errors(), 6)
                ));
            }
        }
        let _ = companion.send(&json!({ "type": "close" }));
    }
}

/// One turn's bookkeeping: tool calls in the reply being answered, their
/// results, and text streamed since the last reply.
struct AgentTurn<'a> {
    conversation: &'a mut Conversation,
    on_event: &'a mut dyn FnMut(TurnEvent),
    /// Tool uses of the last reply not yet answered.
    pending: Vec<String>,
    results: Vec<ToolResult>,
    author: Option<ModelRef>,
    text: String,
}

impl AgentTurn<'_> {
    fn add(&mut self, entry: Entry) {
        self.conversation.entries.push(entry.clone());
        (self.on_event)(TurnEvent::Entry(entry));
    }

    fn notice(&mut self, text: &str) {
        self.add(Entry::Notice { text: text.into() });
    }

    fn stream(&mut self, event: StreamEvent) {
        (self.on_event)(TurnEvent::Stream(event));
    }

    fn finished(&mut self, result: ToolResult) {
        self.pending.retain(|id| *id != result.tool_use_id);
        (self.on_event)(TurnEvent::ToolFinished(result.clone()));
        self.results.push(result);
    }

    /// The last reply's results as one entry; calls it never got are not run.
    fn flush_results(&mut self, why: &str) {
        for id in std::mem::take(&mut self.pending) {
            let result = ToolResult {
                tool_use_id: id,
                ..ToolResult::error(why)
            };
            (self.on_event)(TurnEvent::ToolFinished(result.clone()));
            self.results.push(result);
        }
        if !self.results.is_empty() {
            let results = std::mem::take(&mut self.results);
            self.add(Entry::ToolResults { results });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_providers::{Reasoning, ReasoningPart};

    fn reasoning(text: &str) -> AssistantPart {
        AssistantPart::Reasoning(Reasoning {
            id: None,
            parts: vec![ReasoningPart::Text {
                text: text.into(),
                signature: None,
            }],
        })
    }

    fn operator(text: &str) -> Entry {
        Entry::Operator { text: text.into() }
    }

    fn session(id: &str) -> Entry {
        Entry::Session {
            runtime: RUNTIME.into(),
            id: id.into(),
            event: "started".into(),
        }
    }

    fn said(text: &str) -> Entry {
        Entry::reply(None, &[json!({ "type": "text", "text": text })])
    }

    #[test]
    fn a_session_is_resumed_only_when_every_turn_since_was_its_own() {
        let mut conversation = Conversation {
            entries: vec![operator("one"), session("s1"), said("ok"), operator("two")],
            ..Conversation::default()
        };
        assert_eq!(session_to_resume(&conversation).as_deref(), Some("s1"));
        // A turn on the loop since: hand over instead.
        conversation.entries = vec![
            operator("one"),
            session("s1"),
            said("ok"),
            operator("two"),
            said("from the loop"),
            operator("three"),
        ];
        assert_eq!(session_to_resume(&conversation), None);
        // A new conversation: a new session.
        conversation.entries = vec![operator("first")];
        assert_eq!(session_to_resume(&conversation), None);
    }

    #[test]
    fn the_handover_gives_what_was_visible_and_nothing_hidden() {
        let conversation = Conversation {
            entries: vec![
                operator("Rename the store."),
                Entry::Assistant {
                    model: Some(ModelRef::new(Provider::DeepSeek, "deepseek-flash")),
                    parts: vec![
                        reasoning("secret thoughts"),
                        AssistantPart::Text {
                            text: "Renamed.".into(),
                        },
                    ],
                },
                operator("Now add a cache."),
            ],
            ..Conversation::default()
        };
        let text = handover_text(&conversation, 10_000);
        assert!(text.contains("Operator: Rename the store."), "{text}");
        assert!(text.contains("Assistant: Renamed."), "{text}");
        assert!(!text.contains("secret thoughts"), "{text}");
        assert!(
            !text.contains("Now add a cache"),
            "the current message is not history"
        );
    }

    #[test]
    fn blocks_drop_signatures_and_the_mcp_prefix() {
        let content = json!([
            { "type": "thinking", "thinking": "considering", "signature": "sig" },
            { "type": "tool_use", "id": "toolu_1", "name": "mcp__agentique__read_model", "input": {} },
            { "type": "server_tool_use", "id": "x" }
        ]);
        let blocks = blocks(&content);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].get("signature").is_none());
        assert_eq!(blocks[1]["name"], "read_model");
    }

    #[test]
    fn the_embedded_companion_is_the_pinned_one() {
        let package: Value = serde_json::from_str(COMPANION[0].1).unwrap();
        assert_eq!(
            package["dependencies"]["@anthropic-ai/claude-agent-sdk"],
            SDK_VERSION
        );
        assert!(COMPANION[2].1.contains("PROTOCOL"));
        let protocol = COMPANION[5].1;
        assert!(protocol.contains(&format!("export const PROTOCOL = {PROTOCOL};")));
    }

    #[test]
    fn effective_configuration_problems_are_named() {
        let effective = Effective {
            tools: vec!["mcp__agentique__read_model".into(), "Bash".into()],
            mcp_servers: vec![
                ("agentique".into(), "connected".into()),
                ("other".into(), "connected".into()),
            ],
            permission_mode: "default".into(),
            ..Effective::default()
        };
        let problems = effective.problems();
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].contains("`Bash`"));
    }
}
