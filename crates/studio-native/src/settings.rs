//! Settings (ROADMAP §3.7, §4.9, R-24): the settings table and
//! `settings.json` format 1. The table in code is the one truth for every
//! setting (§8.2): its id, label, description, default, scope, allowed values
//! and validation message. The file holds only the values the Operator
//! changed, never a secret; keys live in the Windows Credential Manager
//! (R-25). This is the interface of §6.2 (interface 2); the Settings view
//! that edits it is W5.8.

use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where a setting applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The whole app (`settings.json`).
    App,
}

/// What values a setting takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Allowed {
    Toggle,
    /// One of these ids.
    Choice(&'static [&'static str]),
    /// A number from `min` to `max` in steps of `step`.
    Number {
        min: f64,
        max: f64,
        step: f64,
    },
    /// Free text; empty means "not set".
    Text,
}

/// One row of the settings table.
#[derive(Clone, Copy, Debug)]
pub struct Setting {
    /// Stable id, `section.name`; the key in `settings.json`.
    pub id: &'static str,
    pub label: &'static str,
    /// One line, shown under the label and searched.
    pub description: &'static str,
    /// Words the Operator may search with that are not in the label or
    /// description ("token" and "key" find the API key).
    pub synonyms: &'static [&'static str],
    pub default: DefaultValue,
    pub scope: Scope,
    pub allowed: Allowed,
    /// Shown when a value is not allowed; the value is kept, not applied.
    pub invalid: &'static str,
}

/// A default value, as it would be written in the file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefaultValue {
    Bool(bool),
    Number(f64),
    Text(&'static str),
}

impl DefaultValue {
    pub fn value(self) -> Value {
        match self {
            DefaultValue::Bool(value) => Value::Bool(value),
            DefaultValue::Number(value) => Value::from(value),
            DefaultValue::Text(value) => Value::from(value),
        }
    }
}

/// The settings table: the first rows (§3.7 Appearance and Assistant).
pub const SETTINGS: &[Setting] = &[
    Setting {
        id: "appearance.theme",
        label: "Theme",
        description: "Follow Windows, or always use light, dark or high contrast.",
        synonyms: &["dark", "light", "colour", "color", "mode", "contrast"],
        default: DefaultValue::Text("system"),
        scope: Scope::App,
        allowed: Allowed::Choice(&["system", "light", "dark", "high-contrast"]),
        invalid: "Choose system, light, dark or high-contrast.",
    },
    Setting {
        id: "appearance.uiScale",
        label: "UI scale",
        description: "Size of text and controls, from 100% to 200%.",
        synonyms: &["zoom", "size", "dpi", "bigger", "smaller"],
        default: DefaultValue::Number(1.0),
        scope: Scope::App,
        allowed: Allowed::Number {
            min: 1.0,
            max: 2.0,
            step: 0.25,
        },
        invalid: "Use a scale from 1.0 to 2.0 in steps of 0.25.",
    },
    Setting {
        id: "appearance.reducedMotion",
        label: "Reduced motion",
        description: "Follow Windows, or always or never replace movement with instant changes.",
        synonyms: &["animation", "motion", "accessibility"],
        default: DefaultValue::Text("system"),
        scope: Scope::App,
        allowed: Allowed::Choice(&["system", "on", "off"]),
        invalid: "Choose system, on or off.",
    },
    Setting {
        id: "control.speed",
        label: "Agents' speed in the window",
        description: "How fast agents act where you can see them: observe (typing about 12 characters a second, a pause on each target before a click), fast (a character a frame) or instant.",
        synonyms: &["observer", "watch", "typing", "slow", "agents", "control"],
        default: DefaultValue::Text("observe"),
        scope: Scope::App,
        allowed: Allowed::Choice(&["observe", "fast", "instant"]),
        invalid: "Choose observe, fast or instant.",
    },
    Setting {
        id: "assistant.runtime",
        label: "Runtime",
        description: "What runs the Assistant's turns: Agentique's own loop, with any provider, or the Claude Agent SDK's loop (Anthropic, or DeepSeek's Anthropic-compatible endpoint), which in a project with a code repository is a full development session under the project's permission policy.",
        synonyms: &["sdk", "agent sdk", "claude agent", "loop", "engine"],
        default: DefaultValue::Text("loop"),
        scope: Scope::App,
        allowed: Allowed::Choice(&["loop", "claude-agent"]),
        invalid: "Choose loop or claude-agent.",
    },
    Setting {
        id: "assistant.provider",
        label: "Provider",
        description: "The model provider the Assistant uses; empty picks the first provider with a key. For the Claude Agent runtime, DeepSeek means its Anthropic-compatible endpoint.",
        synonyms: &[
            "anthropic",
            "deepseek",
            "openai",
            "openrouter",
            "claude",
            "ai",
            "llm",
        ],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Choice(&["", "anthropic", "deepseek", "openai", "openrouter"]),
        invalid: "Choose anthropic, deepseek, openai or openrouter, or leave it empty.",
    },
    Setting {
        id: "assistant.model",
        label: "Model",
        description: "The provider's model id; empty uses the provider's default.",
        synonyms: &["llm", "ai", "claude", "deepseek-flash"],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "Enter a model id, or leave it empty.",
    },
    Setting {
        id: "assistant.effort",
        label: "Effort",
        description: "How much the model thinks, among the levels it offers; empty uses its default.",
        synonyms: &["thinking", "reasoning", "speed", "cost"],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "Enter an effort level the model offers, or leave it empty.",
    },
    Setting {
        id: "providers.anthropic.keyHint",
        label: "Anthropic key",
        description: "What Settings shows of the saved key: its prefix and last four characters, never the key.",
        synonyms: &["api key", "token", "secret", "credential"],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "A key hint is text.",
    },
    Setting {
        id: "providers.anthropic.tokenHint",
        label: "Claude subscription token",
        description: "What Settings shows of the saved token (from `claude setup-token`): its prefix and last four characters, never the token.",
        synonyms: &[
            "oauth",
            "setup-token",
            "plan",
            "max",
            "pro",
            "secret",
            "credential",
        ],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "A token hint is text.",
    },
    Setting {
        id: "providers.anthropic.credential",
        label: "Anthropic credential for agents",
        description: "With both an API key and a Claude subscription token, which one the Claude Agent runtime's sessions use: the token counts against your Claude plan's limits, the key is billed per token. Direct calls always need the key.",
        synonyms: &[
            "subscription",
            "oauth",
            "api key",
            "billing",
            "plan",
            "token",
        ],
        default: DefaultValue::Text("subscription"),
        scope: Scope::App,
        allowed: Allowed::Choice(&["subscription", "key"]),
        invalid: "Choose subscription or key.",
    },
    Setting {
        id: "providers.openai.keyHint",
        label: "OpenAI key",
        description: "What Settings shows of the saved key: its prefix and last four characters, never the key.",
        synonyms: &["api key", "token", "secret", "credential"],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "A key hint is text.",
    },
    Setting {
        id: "providers.openrouter.keyHint",
        label: "OpenRouter key",
        description: "What Settings shows of the saved key: its prefix and last four characters, never the key.",
        synonyms: &["api key", "token", "secret", "credential"],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "A key hint is text.",
    },
    Setting {
        id: "providers.deepseek.keyHint",
        label: "DeepSeek key",
        description: "What Settings shows of the saved key: its prefix and last four characters, never the key.",
        synonyms: &["api key", "token", "secret", "credential"],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "A key hint is text.",
    },
    Setting {
        id: "providers.typesafe.keyHint",
        label: "TypeSafe AI key",
        description: "What Settings shows of the saved key: its prefix and last four characters, never the key.",
        synonyms: &["api key", "token", "secret", "credential"],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "A key hint is text.",
    },
    Setting {
        id: "assistant.showCost",
        label: "Show estimated cost",
        description: "Show tokens and estimated cost per turn and per day.",
        synonyms: &["price", "money", "tokens", "usage", "spend"],
        default: DefaultValue::Bool(true),
        scope: Scope::App,
        allowed: Allowed::Toggle,
        invalid: "Choose on or off.",
    },
    Setting {
        id: "projects.defaultFolder",
        label: "Folder for new projects",
        description: "Where New project suggests a folder; empty: Agentique in your user folder.",
        synonyms: &["location", "directory", "path", "workspace"],
        default: DefaultValue::Text(""),
        scope: Scope::App,
        allowed: Allowed::Text,
        invalid: "Enter a folder, or leave it empty.",
    },
];

/// The models a role's session or call may run on, as `provider/model`:
/// those the capability table knows and the Claude Agent runtime reaches
/// (a test checks both).
const AGENT_MODELS: &[&str] = &[
    "anthropic/claude-opus-5-5",
    "anthropic/claude-sonnet-5-5",
    "anthropic/claude-opus-5",
    "anthropic/claude-haiku-4-5",
    "deepseek/deepseek-v4-pro",
    "deepseek/deepseek-flash",
];
const AGENT_FALLBACKS: &[&str] = &[
    "",
    "anthropic/claude-opus-5-5",
    "anthropic/claude-sonnet-5-5",
    "anthropic/claude-opus-5",
    "anthropic/claude-haiku-4-5",
    "deepseek/deepseek-v4-pro",
    "deepseek/deepseek-flash",
];
/// The known pinned typed-decision models (C-52).
const DECISION_MODELS: &[&str] = &["typesafe/jev-1.13.0"];
/// Every effort level a model of the table offers; empty for its default.
/// A level the chosen model does not offer gives way to its default.
const EFFORTS: &[&str] = &["", "low", "medium", "high", "xhigh", "max"];

/// One row of [`AGENTS`].
const fn agent_row(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    default: &'static str,
    allowed: &'static [&'static str],
) -> Setting {
    Setting {
        id,
        label,
        description,
        synonyms: &["agent", "role", "model", "orchestrator", "objective", "llm"],
        default: DefaultValue::Text(default),
        scope: Scope::App,
        allowed: Allowed::Choice(allowed),
        invalid: "Choose one of the models or levels Settings offer.",
    }
}

/// The Orchestrator's roles (C-54, ROADMAP §4.16 "Models per role"): for
/// each, a model, its effort and a fallback with its effort, resolved before
/// an objective starts (`agq_orchestrator::models`). The Assistant's model
/// is `assistant.provider`, `assistant.model` and `assistant.effort`.
pub const AGENTS: &[Setting] = &[
    agent_row(
        "agents.lead.model",
        "Lead",
        "The lead proposes each cycle's improvement: its provider and model.",
        "anthropic/claude-opus-5-5",
        AGENT_MODELS,
    ),
    agent_row(
        "agents.lead.effort",
        "Lead effort",
        "How much the lead's model thinks; empty uses the model's default.",
        "high",
        EFFORTS,
    ),
    agent_row(
        "agents.lead.fallback",
        "Lead fallback",
        "The model the lead uses when its own has no credential Agentique may use; empty for none.",
        "deepseek/deepseek-v4-pro",
        AGENT_FALLBACKS,
    ),
    agent_row(
        "agents.lead.fallbackEffort",
        "Lead fallback effort",
        "How much the lead's fallback thinks; empty uses its default.",
        "max",
        EFFORTS,
    ),
    agent_row(
        "agents.implementer.model",
        "Implementer",
        "The implementer makes the change in the cycle's worktree: its provider and model.",
        "anthropic/claude-sonnet-5-5",
        AGENT_MODELS,
    ),
    agent_row(
        "agents.implementer.effort",
        "Implementer effort",
        "How much the implementer's model thinks; empty uses the model's default.",
        "high",
        EFFORTS,
    ),
    agent_row(
        "agents.implementer.fallback",
        "Implementer fallback",
        "The model the implementer uses when its own has no credential Agentique may use; empty for none.",
        "deepseek/deepseek-v4-pro",
        AGENT_FALLBACKS,
    ),
    agent_row(
        "agents.implementer.fallbackEffort",
        "Implementer fallback effort",
        "How much the implementer's fallback thinks; empty uses its default.",
        "high",
        EFFORTS,
    ),
    agent_row(
        "agents.reviewer.model",
        "Reviewer",
        "The reviewer judges the change independently: its provider and model.",
        "anthropic/claude-opus-5-5",
        AGENT_MODELS,
    ),
    agent_row(
        "agents.reviewer.effort",
        "Reviewer effort",
        "How much the reviewer's model thinks; empty uses the model's default.",
        "high",
        EFFORTS,
    ),
    agent_row(
        "agents.reviewer.fallback",
        "Reviewer fallback",
        "The model the reviewer uses when its own has no credential Agentique may use; empty for none.",
        "deepseek/deepseek-v4-pro",
        AGENT_FALLBACKS,
    ),
    agent_row(
        "agents.reviewer.fallbackEffort",
        "Reviewer fallback effort",
        "How much the reviewer's fallback thinks; empty uses its default.",
        "max",
        EFFORTS,
    ),
    agent_row(
        "agents.evaluator.model",
        "Evaluator",
        "The evaluator tries the change in a test instance: its provider and model.",
        "anthropic/claude-sonnet-5-5",
        AGENT_MODELS,
    ),
    agent_row(
        "agents.evaluator.effort",
        "Evaluator effort",
        "How much the evaluator's model thinks; empty uses the model's default.",
        "medium",
        EFFORTS,
    ),
    agent_row(
        "agents.evaluator.fallback",
        "Evaluator fallback",
        "The model the evaluator uses when its own has no credential Agentique may use; empty for none.",
        "deepseek/deepseek-v4-pro",
        AGENT_FALLBACKS,
    ),
    agent_row(
        "agents.evaluator.fallbackEffort",
        "Evaluator fallback effort",
        "How much the evaluator's fallback thinks; empty uses its default.",
        "high",
        EFFORTS,
    ),
    agent_row(
        "agents.explorer.model",
        "Explorer",
        "The explorer chooses each step of an exploration, one call at a time (an API key, never the subscription token): its provider and model.",
        "deepseek/deepseek-flash",
        AGENT_MODELS,
    ),
    agent_row(
        "agents.explorer.effort",
        "Explorer effort",
        "How much the explorer's model thinks for a step Jev is unsure of; empty uses the model's default.",
        "low",
        EFFORTS,
    ),
    agent_row(
        "agents.explorer.fallback",
        "Explorer fallback",
        "The model the explorer uses when its own has no key; empty for none.",
        "",
        AGENT_FALLBACKS,
    ),
    agent_row(
        "agents.explorer.fallbackEffort",
        "Explorer fallback effort",
        "How much the explorer's fallback thinks; empty uses its default.",
        "",
        EFFORTS,
    ),
    agent_row(
        "agents.escalation.model",
        "Escalation",
        "The reasoning model a typed decision escalates to when it is unsure, called directly (an API key, never the subscription token).",
        "anthropic/claude-opus-5-5",
        AGENT_MODELS,
    ),
    agent_row(
        "agents.escalation.effort",
        "Escalation effort",
        "How much the escalation model thinks; empty uses the model's default.",
        "high",
        EFFORTS,
    ),
    agent_row(
        "agents.escalation.fallback",
        "Escalation fallback",
        "The model escalation uses when its own has no key; empty for none.",
        "deepseek/deepseek-v4-pro",
        AGENT_FALLBACKS,
    ),
    agent_row(
        "agents.escalation.fallbackEffort",
        "Escalation fallback effort",
        "How much the escalation fallback thinks; empty uses its default.",
        "max",
        EFFORTS,
    ),
    agent_row(
        "agents.decisions.model",
        "Typed decisions",
        "The model of fast typed decisions in operation (Jev, a known pinned version). It takes no effort and has no fallback: when it is unsure, escalation decides.",
        "typesafe/jev-1.13.0",
        DECISION_MODELS,
    ),
];

/// The row for `id`.
pub fn setting(id: &str) -> Option<&'static Setting> {
    SETTINGS
        .iter()
        .chain(AGENTS)
        .find(|setting| setting.id == id)
}

/// Whether `value` is allowed for `setting`.
pub fn allowed(setting: &Setting, value: &Value) -> bool {
    match setting.allowed {
        Allowed::Toggle => value.is_boolean(),
        Allowed::Choice(choices) => value.as_str().is_some_and(|text| choices.contains(&text)),
        Allowed::Number { min, max, step } => value.as_f64().is_some_and(|number| {
            (min..=max).contains(&number) && ((number - min) / step).fract().abs() < 1e-9
        }),
        Allowed::Text => value.is_string(),
    }
}

/// `settings.json`: the changed values, and what could not be read.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Changed values by setting id; a default is never stored.
    values: BTreeMap<String, Value>,
    /// Entries of the file that are not applied (unknown ids, values not
    /// allowed), kept so saving never drops the Operator's text.
    kept: Map<String, Value>,
    /// Plain-words problems with the file, for Settings to show.
    pub problems: Vec<String>,
}

pub const FORMAT: u64 = 1;

impl Settings {
    /// The default location: `%APPDATA%\Agentique\settings.json`.
    pub fn default_path() -> PathBuf {
        crate::session::Session::default_path().with_file_name("settings.json")
    }

    /// Every setting back to its default (entries this version does not
    /// know are kept, as always).
    pub fn reset_all(&mut self) {
        self.values.clear();
    }

    pub fn empty() -> Settings {
        Settings {
            values: BTreeMap::new(),
            kept: Map::new(),
            problems: Vec::new(),
        }
    }

    /// Reads the file; a missing file is all defaults. A file of another
    /// format or one that cannot be read gives defaults and a problem, and
    /// `writable` is false so it is never overwritten.
    pub fn load(path: &Path) -> (Settings, bool) {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return (Settings::empty(), true);
            }
            Err(error) => {
                let mut settings = Settings::empty();
                settings.problems.push(format!(
                    "settings.json could not be read ({error}); defaults are used."
                ));
                return (settings, false);
            }
        };
        Settings::parse(&text)
    }

    /// Reads `settings.json` text; see [`load`](Self::load).
    pub fn parse(text: &str) -> (Settings, bool) {
        let mut settings = Settings::empty();
        let Ok(Value::Object(mut map)) = serde_json::from_str::<Value>(text) else {
            settings
                .problems
                .push("settings.json is not a JSON object; defaults are used and the file is left as it is.".into());
            return (settings, false);
        };
        match map.remove("format").and_then(|format| format.as_u64()) {
            Some(FORMAT) => {}
            other => {
                settings.problems.push(format!(
                    "settings.json has format {}, which this version does not read; defaults are used and the file is left as it is.",
                    other.map_or("none".to_string(), |format| format.to_string())
                ));
                return (settings, false);
            }
        }
        for (id, value) in map {
            match setting(&id) {
                Some(row) if allowed(row, &value) => {
                    if value != row.default.value() {
                        settings.values.insert(id, value);
                    }
                }
                Some(row) => {
                    settings
                        .problems
                        .push(format!("{}: {}", row.label, row.invalid));
                    settings.kept.insert(id, value);
                }
                None => {
                    settings.problems.push(format!(
                        "settings.json: `{id}` is not a setting; it is kept but not used."
                    ));
                    settings.kept.insert(id, value);
                }
            }
        }
        (settings, true)
    }

    /// The value of `id`: the changed value or the default.
    pub fn get(&self, id: &str) -> Value {
        self.values
            .get(id)
            .cloned()
            .or_else(|| setting(id).map(|row| row.default.value()))
            .unwrap_or(Value::Null)
    }

    /// Whether `id` differs from its default (Settings shows a marker and a
    /// reset control).
    pub fn changed(&self, id: &str) -> bool {
        self.values.contains_key(id)
    }

    /// Sets `id`; a value equal to the default removes it. A value that is
    /// not allowed is refused with the row's message.
    pub fn set(&mut self, id: &str, value: Value) -> Result<(), &'static str> {
        let row = setting(id).ok_or("Not a setting.")?;
        if !allowed(row, &value) {
            return Err(row.invalid);
        }
        self.kept.remove(id);
        if value == row.default.value() {
            self.values.remove(id);
        } else {
            self.values.insert(id.to_string(), value);
        }
        Ok(())
    }

    /// The file's text: the format, then the changed values and the kept
    /// entries, sorted by id.
    pub fn to_text(&self) -> String {
        let mut map = Map::new();
        map.insert("format".into(), Value::from(FORMAT));
        let mut entries: BTreeMap<&String, &Value> = self.values.iter().collect();
        entries.extend(self.kept.iter());
        for (id, value) in entries {
            map.insert(id.clone(), value.clone());
        }
        serde_json::to_string_pretty(&Value::Object(map)).expect("settings are plain JSON")
    }

    /// Writes the file atomically (temporary file, then rename).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, self.to_text())?;
        std::fs::rename(temporary, path)
    }
}

/// The sections of the Settings view (§3.7), in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Providers,
    Assistant,
    /// The Orchestrator's roles and their models (C-54).
    Agents,
    Appearance,
    Keyboard,
    Projects,
    Advanced,
    About,
}

impl Section {
    pub const ALL: [Section; 8] = [
        Section::Providers,
        Section::Assistant,
        Section::Agents,
        Section::Appearance,
        Section::Keyboard,
        Section::Projects,
        Section::Advanced,
        Section::About,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Providers => "Providers",
            Section::Assistant => "Assistant",
            Section::Agents => "Agents",
            Section::Appearance => "Appearance",
            Section::Keyboard => "Keyboard",
            Section::Projects => "Projects",
            Section::Advanced => "Advanced",
            Section::About => "About",
        }
    }
}

/// The Studio's settings: the values, where they are saved and whether this
/// version may write that file. What they change is applied by the Studio.
pub struct SettingsStore {
    pub settings: Settings,
    /// False when the file is in a format this version does not write.
    writable: bool,
    path: PathBuf,
    /// Why the last save failed.
    pub save_error: Option<String>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> SettingsStore {
        let (settings, writable) = Settings::load(&path);
        SettingsStore {
            settings,
            writable,
            path,
            save_error: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The Assistant's model: the provider, model and effort set here, with
    /// the environment variables winning over them.
    pub fn model_choice(&self) -> agq_assistant::ModelChoice {
        let (provider, model, effort) = (
            self.text("assistant.provider"),
            self.text("assistant.model"),
            self.text("assistant.effort"),
        );
        agq_assistant::ModelChoice::configured(Some(&provider), Some(&model), Some(&effort))
    }

    /// The Orchestrator's roles as these settings configure them (C-54): a
    /// model and effort each, and a fallback when one is set.
    pub fn agent_configuration(&self) -> Vec<agq_orchestrator::models::Configured> {
        use agq_orchestrator::models::{Choice, Configured, ROLES};
        // A row a role does not have (typed decisions have no effort and no
        // fallback) is not set.
        let value = |id: &str| {
            setting(id)
                .map(|_| self.text(id))
                .filter(|text| !text.is_empty())
        };
        let choice = |model: String, effort: String| {
            agq_providers::ModelRef::parse(&value(&model)?).map(|model| Choice {
                model,
                effort: value(&effort),
            })
        };
        ROLES
            .iter()
            .filter_map(|(role, _)| {
                Some(Configured {
                    role: role.to_string(),
                    model: choice(
                        format!("agents.{role}.model"),
                        format!("agents.{role}.effort"),
                    )?,
                    fallback: choice(
                        format!("agents.{role}.fallback"),
                        format!("agents.{role}.fallbackEffort"),
                    ),
                })
            })
            .collect()
    }

    /// Takes the appearance a Stage 4 session remembered, once.
    pub fn adopt_appearance(&mut self, dark: bool, contrast: bool, reduced_motion: bool) {
        let theme = match (contrast, dark) {
            (true, _) => "high-contrast",
            (false, true) => "dark",
            (false, false) => "light",
        };
        // Both are values of the table's own choices, so neither is refused.
        let _ = self.set("appearance.theme", Value::from(theme));
        if reduced_motion {
            let _ = self.set("appearance.reducedMotion", Value::from("on"));
        }
    }

    /// A setting's value as text; empty for "not set".
    pub fn text(&self, id: &str) -> String {
        match self.settings.get(id) {
            Value::String(text) => text,
            other => other.to_string(),
        }
    }

    pub fn get(&self, id: &str) -> Value {
        self.settings.get(id)
    }

    pub fn changed(&self, id: &str) -> bool {
        self.settings.changed(id)
    }

    /// Sets a value and saves it; a value the table does not allow is
    /// refused with its message.
    pub fn set(&mut self, id: &str, value: Value) -> Result<(), &'static str> {
        self.settings.set(id, value)?;
        self.save();
        Ok(())
    }

    /// Resets every setting to its default, keeping `settings.json.bak`
    /// first (§3.7 Danger zone). True when the settings were reset.
    pub fn reset_all(&mut self) -> bool {
        if self.path.exists()
            && let Err(error) = std::fs::copy(&self.path, self.path.with_extension("json.bak"))
        {
            self.save_error = Some(format!(
                "Nothing was reset: settings.json could not be copied first ({error})."
            ));
            return false;
        }
        self.settings.reset_all();
        self.save();
        true
    }

    fn save(&mut self) {
        if !self.writable {
            self.save_error = Some(
                "settings.json is in a format this version does not write; changes last until Agentique closes.".into(),
            );
            return;
        }
        self.save_error = self
            .settings
            .save(&self.path)
            .err()
            .map(|error| format!("settings.json could not be saved ({error})."));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_row_is_complete_and_its_default_allowed() {
        let mut ids = std::collections::BTreeSet::new();
        for row in SETTINGS.iter().chain(AGENTS) {
            assert!(ids.insert(row.id), "{} twice", row.id);
            assert!(row.id.contains('.'), "{}", row.id);
            assert!(
                !row.label.is_empty() && !row.description.is_empty() && !row.invalid.is_empty()
            );
            assert!(
                allowed(row, &row.default.value()),
                "{}: default not allowed",
                row.id
            );
        }
    }

    #[test]
    fn only_changed_values_are_stored_and_bad_ones_are_kept_not_applied() {
        let (mut settings, writable) = Settings::parse(
            r#"{ "format": 1, "appearance.theme": "dark", "appearance.uiScale": 1.3, "editor.font": "x", "assistant.showCost": true }"#,
        );
        assert!(writable);
        assert_eq!(settings.get("appearance.theme"), json!("dark"));
        // Not allowed: the default applies, the text is kept.
        assert_eq!(settings.get("appearance.uiScale"), json!(1.0));
        assert_eq!(settings.problems.len(), 2, "{:?}", settings.problems);
        // Equal to the default: not stored.
        assert!(!settings.changed("assistant.showCost"));
        settings.set("appearance.theme", json!("system")).unwrap();
        assert!(!settings.changed("appearance.theme"));
        assert_eq!(
            settings.set("appearance.reducedMotion", json!("sometimes")),
            Err("Choose system, on or off.")
        );
        let text = settings.to_text();
        assert!(text.contains("\"format\": 1"));
        assert!(text.contains("editor.font") && text.contains("1.3"));
        assert!(!text.contains("appearance.theme"));
    }

    /// C-54: every role has its four rows; the models offered are the
    /// capability table's, able to do the role's work; and the defaults
    /// configure each role as the ROADMAP says.
    #[test]
    fn the_agents_rows_offer_the_tables_models_and_default_to_c_54() {
        use agq_orchestrator::models::{Kind, ROLES};
        use agq_providers::{ModelRef, capabilities, price};
        for (role, kind) in ROLES {
            // Typed decisions have a model only: no effort, no fallback.
            let fields: &[&str] = if kind == Kind::Decisions {
                &["model"]
            } else {
                &["model", "effort", "fallback", "fallbackEffort"]
            };
            assert_eq!(
                AGENTS
                    .iter()
                    .filter(|row| row.id.starts_with(&format!("agents.{role}.")))
                    .count(),
                fields.len(),
                "{role}"
            );
            for field in fields {
                let id = format!("agents.{role}.{field}");
                let row = setting(&id).unwrap_or_else(|| panic!("{id}"));
                let Allowed::Choice(choices) = row.allowed else {
                    panic!("{id} is a choice");
                };
                for choice in choices.iter().filter(|c| !c.is_empty()) {
                    if field.ends_with("ffort") {
                        continue;
                    }
                    let model = ModelRef::parse(choice).unwrap();
                    let caps = capabilities(&model);
                    assert!(price(&model).is_some(), "{choice}");
                    assert!(
                        match kind {
                            Kind::Session => caps.agent_runtime && caps.tools,
                            Kind::Direct => caps.chat,
                            Kind::Decisions => caps.decisions,
                        },
                        "{id}: {choice}"
                    );
                }
            }
        }
        let store = SettingsStore::load(std::env::temp_dir().join("agq-no-settings-here.json"));
        let configured = store.agent_configuration();
        assert_eq!(configured.len(), 7);
        let shown: Vec<String> = configured
            .iter()
            .map(|c| {
                format!(
                    "{} {} {} -> {}",
                    c.role,
                    c.model.model,
                    c.model.effort.as_deref().unwrap_or("-"),
                    c.fallback
                        .as_ref()
                        .map(|f| format!("{} {}", f.model, f.effort.as_deref().unwrap_or("-")))
                        .unwrap_or_else(|| "none".into())
                )
            })
            .collect();
        assert_eq!(
            shown,
            [
                "lead anthropic/claude-opus-5-5 high -> deepseek/deepseek-v4-pro max",
                "implementer anthropic/claude-sonnet-5-5 high -> deepseek/deepseek-v4-pro high",
                "reviewer anthropic/claude-opus-5-5 high -> deepseek/deepseek-v4-pro max",
                "evaluator anthropic/claude-sonnet-5-5 medium -> deepseek/deepseek-v4-pro high",
                // Low since the W13.7 repair's measurement (ROADMAP §7.6).
                "explorer deepseek/deepseek-flash low -> none",
                "escalation anthropic/claude-opus-5-5 high -> deepseek/deepseek-v4-pro max",
                "decisions typesafe/jev-1.13.0 - -> none",
            ]
        );
        assert_eq!(store.text("providers.anthropic.credential"), "subscription");
    }

    #[test]
    fn another_format_is_refused_and_never_overwritten() {
        let (settings, writable) =
            Settings::parse(r#"{ "format": 2, "appearance.theme": "dark" }"#);
        assert!(!writable);
        assert_eq!(settings.get("appearance.theme"), json!("system"));
        assert_eq!(settings.problems.len(), 1);
    }

    #[test]
    fn saving_and_loading_round_trips() {
        let folder = std::env::temp_dir().join(format!("agq-settings-{}", std::process::id()));
        let path = folder.join("settings.json");
        let mut settings = Settings::empty();
        settings
            .set("assistant.provider", json!("deepseek"))
            .unwrap();
        settings.save(&path).unwrap();
        let (loaded, writable) = Settings::load(&path);
        assert!(writable);
        assert_eq!(loaded.get("assistant.provider"), json!("deepseek"));
        let _ = std::fs::remove_dir_all(folder);
    }
}
