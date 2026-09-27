//! The Settings view (ROADMAP §3.7, W5.8, R-24; Scenario E): Ctrl+, opens it
//! in place of the Surface. A list of sections with a search box on the left;
//! one column of rows (a label, a one-line description, the control on the
//! right) of at most 720 points on the right. Choices apply at once; a key has
//! its own small form (paste, Test, Save) and is never shown again after
//! saving, only its hint (R-25). The rows come from the settings table
//! (`settings.rs`); what they change is applied by the Studio.

use crate::commands::COMMANDS;
use crate::settings::{self, Allowed, Settings};
use crate::theme::{self, Theme};
use agq_providers::{KeyCheck, KeyStatus, ModelInfo, Provider, Providers, keys};
use eframe::egui::{self, RichText, Stroke};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Providers,
    Assistant,
    Appearance,
    Keyboard,
    About,
}

const SECTIONS: [(Section, &str); 5] = [
    (Section::Providers, "Providers"),
    (Section::Assistant, "Assistant"),
    (Section::Appearance, "Appearance"),
    (Section::Keyboard, "Keyboard"),
    (Section::About, "About"),
];

/// The widest the content column gets (§3.7).
const COLUMN: f32 = 720.0;

/// What Settings changed that the Studio applies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changed {
    pub appearance: bool,
    pub assistant: bool,
}

enum Background<T> {
    Idle,
    Running(Receiver<T>),
    Done(T),
}

impl<T> Background<T> {
    fn start(work: impl FnOnce() -> T + Send + 'static) -> Background<T>
    where
        T: Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(work());
        });
        Background::Running(receiver)
    }

    /// Takes a finished result; true when it just arrived.
    fn poll(&mut self) -> bool {
        if let Background::Running(receiver) = self
            && let Ok(result) = receiver.try_recv()
        {
            *self = Background::Done(result);
            return true;
        }
        false
    }

    fn running(&self) -> bool {
        matches!(self, Background::Running(_))
    }
}

/// One provider's key form and model list.
struct ProviderState {
    /// The key being pasted; cleared once saved (R-25 point 2).
    input: String,
    test: Background<KeyCheck>,
    /// Save once the test says the key works.
    save_after_test: bool,
    models: Background<Result<Vec<ModelInfo>, String>>,
    confirm_remove: bool,
    /// Move the focus to Cancel on the next frame.
    focus_cancel: bool,
    /// "Save anyway (offline)" was pressed.
    save_anyway: bool,
    status: KeyStatus,
    message: Option<String>,
}

impl ProviderState {
    fn new(provider: Provider) -> ProviderState {
        ProviderState {
            input: String::new(),
            test: Background::Idle,
            save_after_test: false,
            models: Background::Idle,
            confirm_remove: false,
            focus_cancel: false,
            save_anyway: false,
            status: agq_providers::key_status(provider),
            message: None,
        }
    }
}

pub struct SettingsView {
    pub open: bool,
    pub section: Section,
    pub search: String,
    pub settings: Settings,
    /// False when the file is in a format this version does not write.
    writable: bool,
    path: PathBuf,
    providers: BTreeMap<Provider, ProviderState>,
    /// Why the last save failed.
    save_error: Option<String>,
    focus_search: bool,
}

impl SettingsView {
    pub fn load(path: PathBuf) -> SettingsView {
        let (settings, writable) = Settings::load(&path);
        SettingsView {
            open: false,
            section: Section::Providers,
            search: String::new(),
            settings,
            writable,
            path,
            providers: BTreeMap::new(),
            save_error: None,
            focus_search: false,
        }
    }

    /// Opens Settings at a section (deep links from errors, §3.7).
    pub fn show(&mut self, section: Section) {
        self.open = true;
        self.section = section;
        self.focus_search = true;
    }

    /// The Assistant's model: the provider, model and effort set here, with
    /// the environment variables winning over them.
    pub fn model_choice(&self) -> agq_assistant::ModelChoice {
        let text = |id: &str| self.text(id);
        let (provider, model, effort) = (
            text("assistant.provider"),
            text("assistant.model"),
            text("assistant.effort"),
        );
        agq_assistant::ModelChoice::configured(Some(&provider), Some(&model), Some(&effort))
    }

    /// Whether the Operator chose any appearance setting; before they do,
    /// the Studio keeps what the session remembers.
    pub fn appearance_chosen(&self) -> bool {
        [
            "appearance.theme",
            "appearance.uiScale",
            "appearance.reducedMotion",
        ]
        .iter()
        .any(|id| self.settings.changed(id))
    }

    /// Sets a value and saves it (the Studio's own commands change
    /// appearance too).
    pub fn set_value(&mut self, id: &str, value: Value) {
        self.set(id, value);
    }

    /// A setting's value as text; empty for "not set".
    pub fn text(&self, id: &str) -> String {
        match self.settings.get(id) {
            Value::String(text) => text,
            other => other.to_string(),
        }
    }

    fn set(&mut self, id: &str, value: Value) {
        if self.settings.set(id, value).is_ok() {
            self.save();
        }
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

    fn provider(&mut self, provider: Provider) -> &mut ProviderState {
        self.providers
            .entry(provider)
            .or_insert_with(|| ProviderState::new(provider))
    }

    /// Draws the view in place of the Surface; returns what the Studio must
    /// apply.
    pub fn ui(&mut self, ui: &mut egui::Ui, theme: Theme) -> Changed {
        let ctx = ui.ctx().clone();
        let mut changed = Changed::default();
        // Finished background work.
        for provider in Provider::ALL {
            let state = self.provider(provider);
            state.models.poll();
            if state.test.poll() {
                let works = matches!(state.test, Background::Done(KeyCheck::Works));
                if state.save_after_test {
                    state.save_after_test = false;
                    if works {
                        changed.assistant |= self.store_key(provider);
                    }
                }
            }
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.search.is_empty() {
                self.open = false;
            } else {
                self.search.clear();
            }
        }
        egui::Panel::left("settings-sections")
            .resizable(false)
            .exact_size(220.0)
            .frame(
                egui::Frame::new()
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Settings")
                        .font(theme::semibold(theme::HEADING))
                        .color(theme.text),
                );
                ui.add_space(theme::SPACE);
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search settings")
                        .desired_width(f32::INFINITY),
                );
                if std::mem::take(&mut self.focus_search) {
                    search.request_focus();
                }
                ui.add_space(theme::SPACE);
                for (section, name) in SECTIONS {
                    let selected = self.section == section && self.search.is_empty();
                    if ui.selectable_label(selected, name).clicked() {
                        self.section = section;
                        self.search.clear();
                    }
                }
                ui.add_space(theme::SPACE_L);
                if ui.button("Close").on_hover_text("Esc").clicked() {
                    self.open = false;
                }
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme.canvas)
                    .inner_margin(egui::Margin::symmetric(24, 16)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_max_width(COLUMN);
                    for problem in self.settings.problems.clone() {
                        notice(ui, theme, &problem);
                    }
                    if let Some(error) = self.save_error.clone() {
                        notice(ui, theme, &error);
                    }
                    let query = self.search.trim().to_lowercase();
                    if query.is_empty() {
                        match self.section {
                            Section::Providers => {
                                changed.assistant |= self.providers_ui(ui, theme, None)
                            }
                            Section::Assistant => {
                                changed.assistant |= self.assistant_ui(ui, theme, None)
                            }
                            Section::Appearance => {
                                changed.appearance |= self.appearance_ui(ui, theme, None)
                            }
                            Section::Keyboard => keyboard_ui(ui, theme, None),
                            Section::About => self.about_ui(ui, theme),
                        }
                    } else {
                        // Search shows matching rows from every section.
                        changed.assistant |= self.providers_ui(ui, theme, Some(&query));
                        changed.assistant |= self.assistant_ui(ui, theme, Some(&query));
                        changed.appearance |= self.appearance_ui(ui, theme, Some(&query));
                        keyboard_ui(ui, theme, Some(&query));
                    }
                });
            });
        changed
    }

    /// Saves the pasted key in the Credential Manager and its hint in the
    /// settings; clears the field. True when the key changed.
    fn store_key(&mut self, provider: Provider) -> bool {
        let key = std::mem::take(&mut self.provider(provider).input);
        let result = keys::store(provider, &key);
        let hint = keys::hint(&key);
        let state = self.provider(provider);
        match result {
            Ok(()) => {
                state.status = KeyStatus::Stored;
                state.message = Some("Saved in Windows Credential Manager.".into());
                state.test = Background::Idle;
                let id = format!("providers.{}.keyHint", provider.id());
                self.set(&id, json!(hint));
                true
            }
            Err(error) => {
                state.message = Some(error.0);
                false
            }
        }
    }

    fn providers_ui(&mut self, ui: &mut egui::Ui, theme: Theme, query: Option<&str>) -> bool {
        let mut changed = false;
        if query.is_none() {
            section_title(ui, theme, "Providers");
        }
        for provider in Provider::ALL {
            let words = [
                provider.name(),
                "key",
                "api key",
                "token",
                "provider",
                "model",
                provider.key_variable(),
            ];
            if query.is_some_and(|query| !matches(query, &words)) {
                continue;
            }
            changed |= self.provider_ui(ui, theme, provider);
            ui.add_space(theme::SPACE_L);
        }
        changed
    }

    fn provider_ui(&mut self, ui: &mut egui::Ui, theme: Theme, provider: Provider) -> bool {
        let mut changed = false;
        let hint = self.text(&format!("providers.{}.keyHint", provider.id()));
        let state = self.provider(provider);
        let status = match &state.status {
            KeyStatus::FromEnvironment { variable } => {
                format!("From environment variable {variable} (a saved key is ignored).")
            }
            KeyStatus::Stored if hint.is_empty() => {
                "Saved in Windows Credential Manager.".to_string()
            }
            KeyStatus::Stored => format!("Saved in Windows Credential Manager ({hint})."),
            KeyStatus::Missing => "No key.".to_string(),
            KeyStatus::Unavailable(reason) => reason.clone(),
        };
        let from_environment = matches!(state.status, KeyStatus::FromEnvironment { .. });
        egui::Frame::new()
            .fill(theme.elevated)
            .stroke(Stroke::new(theme::HAIRLINE, theme.border))
            .corner_radius(theme::RADIUS)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(provider.name()).font(theme::semibold(theme::BODY)).color(theme.text));
                ui.label(RichText::new(status).color(theme.text_secondary));
                if provider == Provider::TypeSafe {
                    ui.label(
                        RichText::new("Jev answers typed questions for fast agents; it is never the Assistant's model.")
                            .font(theme::regular(theme::CAPTION))
                            .color(theme.muted),
                    );
                }
                ui.add_space(theme::SPACE_S);
                ui.add_enabled_ui(!from_environment, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.input)
                                .password(true)
                                .hint_text(format!("Paste a {} key", provider.name()))
                                .desired_width(280.0),
                        );
                        let has_input = !state.input.trim().is_empty();
                        let busy = state.test.running();
                        if ui.add_enabled(has_input && !busy, egui::Button::new("Test")).clicked() {
                            let key = state.input.clone();
                            state.test = Background::start(move || Providers::new().check_key(provider, Some(&key)));
                        }
                        if ui.add_enabled(has_input && !busy, egui::Button::new("Save")).clicked() {
                            // Save runs the test first (R-25 point 3).
                            let key = state.input.clone();
                            state.save_after_test = true;
                            state.test = Background::start(move || Providers::new().check_key(provider, Some(&key)));
                        }
                        if busy {
                            ui.add(egui::Spinner::new().size(12.0));
                        }
                    });
                })
                .response
                .on_disabled_hover_text(format!("Set by {}", provider.key_variable()));
                if let Background::Done(check) = &state.test {
                    let (text, works) = describe(provider, check);
                    ui.label(RichText::new(text).color(if works { theme.green } else { theme.amber }));
                    if !works && !state.input.trim().is_empty() {
                        state.save_anyway = ui
                            .button("Save anyway (offline)")
                            .on_hover_text("Save the key without a successful test")
                            .clicked();
                    }
                }
                if let Some(message) = &state.message {
                    ui.label(RichText::new(message).color(theme.text_secondary));
                }
                // Removing a saved key: a simple confirmation with Cancel
                // focused (§3.7 Danger zone).
                if matches!(state.status, KeyStatus::Stored) {
                    if state.confirm_remove {
                        ui.horizontal(|ui| {
                            ui.label(format!("Remove the saved {} key?", provider.name()));
                            let cancel = ui.button("Cancel");
                            if std::mem::take(&mut state.focus_cancel) {
                                cancel.request_focus();
                            }
                            if cancel.clicked() {
                                state.confirm_remove = false;
                            }
                            if ui.button(RichText::new("Remove key").color(theme.error)).clicked() {
                                state.confirm_remove = false;
                                match keys::remove(provider) {
                                    Ok(()) => {
                                        state.status = agq_providers::key_status(provider);
                                        state.message = Some("The saved key was removed.".into());
                                        changed = true;
                                    }
                                    Err(error) => state.message = Some(error.0),
                                }
                            }
                        });
                    } else if ui.button("Remove key…").clicked() {
                        state.confirm_remove = true;
                        state.focus_cancel = true;
                    }
                }
                // The model list with capability badges and prices (E2).
                ui.add_space(theme::SPACE_S);
                let listing = state.models.running();
                let label = if matches!(state.models, Background::Idle) { "Show models" } else { "Refresh models" };
                if ui.add_enabled(!listing, egui::Button::new(label)).clicked() {
                    state.models = Background::start(move || {
                        Providers::new().list_models(provider).map_err(|error| error.message)
                    });
                }
                match &state.models {
                    Background::Done(Ok(models)) => {
                        for model in models {
                            model_row(ui, theme, model);
                        }
                    }
                    Background::Done(Err(error)) => {
                        ui.label(RichText::new(error).color(theme.amber));
                    }
                    Background::Running(_) => {
                        ui.add(egui::Spinner::new().size(12.0));
                    }
                    Background::Idle => {}
                }
            });
        if std::mem::take(&mut self.provider(provider).save_anyway) {
            changed |= self.store_key(provider);
        }
        changed
    }

    fn assistant_ui(&mut self, ui: &mut egui::Ui, theme: Theme, query: Option<&str>) -> bool {
        let mut changed = false;
        if query.is_none() {
            section_title(ui, theme, "Assistant");
        }
        for id in [
            "assistant.provider",
            "assistant.model",
            "assistant.effort",
            "assistant.showCost",
        ] {
            let row = settings::setting(id).expect("a setting");
            if !visible(row, query) {
                continue;
            }
            let current = self.text(id);
            let default = row.default.value();
            let is_changed = self.settings.changed(id);
            let new_value = setting_row(
                ui,
                theme,
                row.label,
                row.description,
                is_changed,
                &default,
                |ui| {
                    match id {
                        "assistant.provider" => {
                            let mut value = current.clone();
                            egui::ComboBox::from_id_salt(id)
                                .selected_text(if value.is_empty() {
                                    "Automatic".to_string()
                                } else {
                                    value.clone()
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut value,
                                        String::new(),
                                        "Automatic (first provider with a key)",
                                    );
                                    for provider in [
                                        Provider::Anthropic,
                                        Provider::DeepSeek,
                                        Provider::OpenAi,
                                        Provider::OpenRouter,
                                    ] {
                                        ui.selectable_value(
                                            &mut value,
                                            provider.id().to_string(),
                                            provider.name(),
                                        );
                                    }
                                });
                            (value != current).then(|| json!(value))
                        }
                        "assistant.showCost" => {
                            let mut on = self.settings.get(id).as_bool().unwrap_or(true);
                            let before = on;
                            ui.checkbox(&mut on, "");
                            (on != before).then(|| json!(on))
                        }
                        _ => {
                            let mut value = current.clone();
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut value)
                                    .hint_text("Default")
                                    .desired_width(200.0),
                            );
                            // Text commits on Enter or leaving the field (§3.7).
                            (response.lost_focus() && value != current).then(|| json!(value.trim()))
                        }
                    }
                },
            );
            if let Some(value) = new_value {
                self.set(id, value);
                changed = true;
            }
        }
        changed
    }

    fn appearance_ui(&mut self, ui: &mut egui::Ui, theme: Theme, query: Option<&str>) -> bool {
        let mut changed = false;
        if query.is_none() {
            section_title(ui, theme, "Appearance");
        }
        for id in [
            "appearance.theme",
            "appearance.uiScale",
            "appearance.reducedMotion",
        ] {
            let row = settings::setting(id).expect("a setting");
            if !visible(row, query) {
                continue;
            }
            let is_changed = self.settings.changed(id);
            let default = row.default.value();
            let value = self.settings.get(id);
            let new_value = setting_row(
                ui,
                theme,
                row.label,
                row.description,
                is_changed,
                &default,
                |ui| match row.allowed {
                    Allowed::Choice(choices) => {
                        let current = value.as_str().unwrap_or_default().to_string();
                        let mut selected = current.clone();
                        egui::ComboBox::from_id_salt(id)
                            .selected_text(choice_label(&selected))
                            .show_ui(ui, |ui| {
                                for choice in choices {
                                    ui.selectable_value(
                                        &mut selected,
                                        choice.to_string(),
                                        choice_label(choice),
                                    );
                                }
                            });
                        (selected != current).then(|| json!(selected))
                    }
                    Allowed::Number { min, max, step } => {
                        let current = value.as_f64().unwrap_or(min);
                        let mut selected = current;
                        egui::ComboBox::from_id_salt(id)
                            .selected_text(format!("{:.0}%", selected * 100.0))
                            .show_ui(ui, |ui| {
                                let mut scale = min;
                                while scale <= max + 1e-9 {
                                    ui.selectable_value(
                                        &mut selected,
                                        scale,
                                        format!("{:.0}%", scale * 100.0),
                                    );
                                    scale += step;
                                }
                            });
                        ((selected - current).abs() > 1e-9).then(|| json!(selected))
                    }
                    _ => None,
                },
            );
            if let Some(value) = new_value {
                self.set(id, value);
                changed = true;
            }
        }
        changed
    }

    fn about_ui(&mut self, ui: &mut egui::Ui, theme: Theme) {
        section_title(ui, theme, "About");
        ui.label(format!("Agentique Studio {}", env!("CARGO_PKG_VERSION")));
        ui.label(RichText::new("Licensed under Apache-2.0. Fonts: Inter and JetBrains Mono, under the SIL Open Font License 1.1.").color(theme.text_secondary));
        ui.add_space(theme::SPACE);
        ui.label(
            RichText::new(format!("Settings file: {}", self.path.display())).color(theme.muted),
        );
        ui.label(
            RichText::new("Keys are kept in the Windows Credential Manager, never in this file.")
                .color(theme.muted),
        );
    }
}

/// What a key test found, in plain words (§2.4 E2), and whether it works.
fn describe(provider: Provider, check: &KeyCheck) -> (String, bool) {
    let name = provider.name();
    match check {
        KeyCheck::Works => ("Key works.".into(), true),
        KeyCheck::Refused => ("Key refused. Check that it was copied whole.".into(), false),
        KeyCheck::NoAccess => (
            "Key has no access (no balance or no permission).".into(),
            false,
        ),
        KeyCheck::RateLimited => ("Rate limited, try later.".into(), false),
        KeyCheck::Unreachable(why) => (format!("Could not reach {name} ({why})."), false),
        KeyCheck::Missing => ("Paste a key first.".into(), false),
        KeyCheck::Failed(why) => (why.clone(), false),
    }
}

fn model_row(ui: &mut egui::Ui, theme: Theme, model: &ModelInfo) {
    let mut badges = Vec::new();
    if !model.capabilities.tools {
        badges.push("no tools: not for the Assistant".to_string());
    }
    if !model.efforts.is_empty() {
        badges.push(format!("effort {}", model.efforts.join(", ")));
    }
    if matches!(
        model.capabilities.prompt_cache,
        agq_providers::PromptCache::None
    ) && model.capabilities.tools
    {
        badges.push("no prompt caching".into());
    }
    if let Some(window) = model.context_window {
        badges.push(format!("{}k context", window / 1000));
    }
    let price = model
        .price
        .map(|price| {
            format!(
                "≈ ${} / ${} per million tokens (estimate, {})",
                price.input, price.output, price.as_of
            )
        })
        .unwrap_or_else(|| "price not known".into());
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(&model.model.model)
                .font(theme::medium(theme::LABEL))
                .color(if model.capabilities.tools {
                    theme.text
                } else {
                    theme.muted
                }),
        );
        for badge in &badges {
            ui.label(
                RichText::new(badge)
                    .font(theme::regular(theme::CAPTION))
                    .color(theme.text_secondary),
            );
        }
        ui.label(
            RichText::new(price)
                .font(theme::regular(theme::CAPTION))
                .color(theme.muted),
        );
    });
}

fn keyboard_ui(ui: &mut egui::Ui, theme: Theme, query: Option<&str>) {
    if query.is_none() {
        section_title(ui, theme, "Keyboard");
    }
    for command in COMMANDS {
        if query.is_some_and(|query| {
            !matches(
                query,
                &[command.label, command.description, command.shortcut],
            )
        }) {
            continue;
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(command.label).color(theme.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !command.shortcut.is_empty() {
                    ui.label(
                        RichText::new(command.shortcut)
                            .font(theme::medium(theme::LABEL))
                            .color(theme.text_secondary),
                    );
                }
            });
        });
    }
}

fn section_title(ui: &mut egui::Ui, theme: Theme, title: &str) {
    ui.label(
        RichText::new(title)
            .font(theme::semibold(theme::TITLE))
            .color(theme.text),
    );
    ui.add_space(theme::SPACE);
}

fn notice(ui: &mut egui::Ui, theme: Theme, text: &str) {
    egui::Frame::new()
        .fill(theme.elevated)
        .stroke(Stroke::new(theme::HAIRLINE, theme.amber))
        .corner_radius(theme::RADIUS)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(text).color(theme.text_secondary));
        });
    ui.add_space(theme::SPACE);
}

/// One row: the label and description on the left, the control on the
/// right, a changed marker with a reset to the default (§3.7). Returns the
/// new value when the Operator changed or reset it.
fn setting_row(
    ui: &mut egui::Ui,
    theme: Theme,
    label: &str,
    description: &str,
    is_changed: bool,
    default: &Value,
    control: impl FnOnce(&mut egui::Ui) -> Option<Value>,
) -> Option<Value> {
    let mut result = None;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 260.0).max(200.0));
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).color(theme.text));
                if is_changed {
                    ui.label(
                        RichText::new("●")
                            .font(theme::regular(theme::CAPTION))
                            .color(theme.accent),
                    )
                    .on_hover_text("Changed from the default");
                }
            });
            ui.label(
                RichText::new(description)
                    .font(theme::regular(theme::CAPTION))
                    .color(theme.muted),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if is_changed
                && ui
                    .small_button("Reset")
                    .on_hover_text(format!(
                        "Reset to the default: {}",
                        describe_default(default)
                    ))
                    .clicked()
            {
                result = Some(default.clone());
            }
            if let Some(value) = control(ui) {
                result = Some(value);
            }
        });
    });
    ui.add_space(theme::SPACE);
    result
}

fn describe_default(default: &Value) -> String {
    match default {
        Value::String(text) if text.is_empty() => "automatic".into(),
        Value::String(text) => choice_label(text),
        Value::Number(number) => format!("{:.0}%", number.as_f64().unwrap_or(1.0) * 100.0),
        Value::Bool(true) => "on".into(),
        Value::Bool(false) => "off".into(),
        other => other.to_string(),
    }
}

fn choice_label(choice: &str) -> String {
    match choice {
        "system" => "Follow Windows".into(),
        "light" => "Light".into(),
        "dark" => "Dark".into(),
        "high-contrast" => "High contrast".into(),
        "on" => "On".into(),
        "off" => "Off".into(),
        other => other.to_string(),
    }
}

fn visible(row: &settings::Setting, query: Option<&str>) -> bool {
    query.is_none_or(|query| {
        let mut words = vec![row.label, row.description];
        words.extend(row.synonyms);
        matches(query, &words)
    })
}

/// Search matches labels, descriptions and synonyms (§3.7: people search in
/// their own words).
fn matches(query: &str, words: &[&str]) -> bool {
    query
        .split_whitespace()
        .all(|term| words.iter().any(|word| word.to_lowercase().contains(term)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SETTINGS;

    #[test]
    fn search_finds_descriptions_and_synonyms() {
        let key = settings::setting("providers.deepseek.keyHint").unwrap();
        assert!(visible(key, Some("api key")));
        assert!(visible(key, Some("token")));
        let theme = settings::setting("appearance.theme").unwrap();
        assert!(visible(theme, Some("dark")));
        assert!(!visible(theme, Some("api key")));
        assert!(SETTINGS.iter().any(|row| visible(row, Some("effort"))));
    }

    #[test]
    fn key_tests_are_described_in_plain_words() {
        assert_eq!(
            describe(Provider::DeepSeek, &KeyCheck::Works),
            ("Key works.".to_string(), true)
        );
        assert!(!describe(Provider::DeepSeek, &KeyCheck::Refused).1);
    }
}

#[cfg(test)]
mod studio_tests {
    use crate::commands::CommandId;
    use crate::edit::app_tests::{frame, studio};
    use eframe::egui;

    fn press(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers,
            })
            .collect()
    }

    #[test]
    fn ctrl_comma_opens_settings_in_place_of_the_surface_and_escape_closes_them() {
        let (mut app, context, _folder) = studio("settings-open");
        frame(&mut app, &context, vec![]);
        frame(
            &mut app,
            &context,
            press(egui::Key::Comma, egui::Modifiers::COMMAND),
        );
        assert!(app.settings.open);
        // Editing commands wait while Settings are open.
        app.execute(CommandId::CreatePart, &context);
        assert!(app.dialog.is_none(), "{}", app.status);
        frame(&mut app, &context, vec![]);
        frame(
            &mut app,
            &context,
            press(egui::Key::Escape, egui::Modifiers::NONE),
        );
        assert!(!app.settings.open);
    }

    #[test]
    fn the_appearance_commands_change_the_setting_and_it_holds() {
        let (mut app, context, folder) = studio("settings-theme");
        let dark = app.theme.dark;
        app.execute(CommandId::Theme, &context);
        assert_eq!(app.theme.dark, !dark);
        let expected = if dark { "light" } else { "dark" };
        assert_eq!(app.settings.text("appearance.theme"), expected);
        app.execute(CommandId::ReducedMotion, &context);
        assert!(app.reduced_motion);
        // The file beside the session holds them, never a key.
        let saved = std::fs::read_to_string(folder.0.join("settings.json")).unwrap();
        assert!(saved.contains(expected), "{saved}");
        assert!(
            saved.contains("\"appearance.reducedMotion\": \"on\""),
            "{saved}"
        );
    }
}
