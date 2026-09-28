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
        id: "assistant.provider",
        label: "Provider",
        description: "The model provider the Assistant uses; empty picks the first provider with a key.",
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

/// The row for `id`.
pub fn setting(id: &str) -> Option<&'static Setting> {
    SETTINGS.iter().find(|setting| setting.id == id)
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
    Appearance,
    Keyboard,
    Projects,
    Advanced,
    About,
}

impl Section {
    pub const ALL: [Section; 7] = [
        Section::Providers,
        Section::Assistant,
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

    /// Takes the appearance a Stage 4 session remembered, once.
    pub fn adopt_appearance(&mut self, dark: bool, contrast: bool, reduced_motion: bool) {
        let theme = match (contrast, dark) {
            (true, _) => "high-contrast",
            (false, true) => "dark",
            (false, false) => "light",
        };
        self.set("appearance.theme", Value::from(theme));
        if reduced_motion {
            self.set("appearance.reducedMotion", Value::from("on"));
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
        for row in SETTINGS {
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
