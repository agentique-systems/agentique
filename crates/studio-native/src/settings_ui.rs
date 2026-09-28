//! The Settings view (ROADMAP §3.7, W5.8, R-24; Scenario E): Ctrl+, opens it
//! in place of the Surface. A list of sections with a search box on the left;
//! one column of rows (a label, a one-line description, the control on the
//! right) of at most 720 points on the right. Choices apply at once; a key has
//! its own small form (paste, Test, Save) and is never shown again after
//! saving, only its hint (R-25). The rows come from the settings table
//! (`settings.rs`); what they change is applied by the Studio.

use crate::commands::COMMANDS;
use crate::settings::{self, Allowed, Settings};
use crate::targets::{Target, record};
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
    Projects,
    Advanced,
    About,
}

const SECTIONS: [(Section, &str); 7] = [
    (Section::Providers, "Providers"),
    (Section::Assistant, "Assistant"),
    (Section::Appearance, "Appearance"),
    (Section::Keyboard, "Keyboard"),
    (Section::Projects, "Projects"),
    (Section::Advanced, "Advanced"),
    (Section::About, "About"),
];

/// The widest the content column gets (§3.7).
const COLUMN: f32 = 720.0;

/// What Settings changed that the Studio applies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changed {
    pub appearance: bool,
    pub assistant: bool,
    /// A project to take off the recent list.
    pub remove_recent: Option<PathBuf>,
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
    /// The key the last test ran on; Save stores exactly this key.
    tested: String,
    /// Save once the test says the key works.
    save_after_test: bool,
    models: Background<Result<Vec<ModelInfo>, String>>,
    confirm_remove: bool,
    /// Move the focus to Cancel on the next frame.
    focus_cancel: bool,
    /// The saved key was just removed: its hint goes too.
    removed: bool,
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
            tested: String::new(),
            save_after_test: false,
            models: Background::Idle,
            confirm_remove: false,
            focus_cancel: false,
            removed: false,
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
    /// Text being typed into a text row; committed on Enter or leaving it.
    drafts: BTreeMap<&'static str, String>,
    /// "Reset all settings" asks first, with Cancel focused once.
    confirm_reset: bool,
    focus_cancel_reset: bool,
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
            drafts: BTreeMap::new(),
            confirm_reset: false,
            focus_cancel_reset: false,
            focus_search: false,
        }
    }

    /// Closes Settings; a pasted key that was not saved is dropped (R-25).
    pub fn close(&mut self) {
        self.open = false;
        for state in self.providers.values_mut() {
            state.input.clear();
            state.tested.clear();
            state.confirm_remove = false;
        }
        self.drafts.clear();
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

    /// Takes the appearance a Stage 4 session remembered, once.
    pub fn adopt_appearance(&mut self, dark: bool, contrast: bool, reduced_motion: bool) {
        let theme = match (contrast, dark) {
            (true, _) => "high-contrast",
            (false, true) => "dark",
            (false, false) => "light",
        };
        self.set("appearance.theme", json!(theme));
        if reduced_motion {
            self.set("appearance.reducedMotion", json!("on"));
        }
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
    pub fn ui(&mut self, ui: &mut egui::Ui, theme: Theme, recent: &[PathBuf]) -> Changed {
        let ctx = ui.ctx().clone();
        let mut changed = Changed::default();
        // Finished background work.
        for provider in Provider::ALL {
            let state = self.provider(provider);
            state.models.poll();
            if state.test.poll() {
                let works = matches!(state.test, Background::Done(KeyCheck::Works));
                // A key that works shows its models at once, listed with that
                // key even before it is saved (E2).
                if works && !state.models.running() {
                    let key = state.tested.clone();
                    state.models = Background::start(move || {
                        Providers::new()
                            .with_key(provider, key)
                            .list_models(provider)
                            .map_err(|error| error.message)
                    });
                }
                if state.save_after_test {
                    state.save_after_test = false;
                    if works {
                        let key = state.tested.clone();
                        changed.assistant |= self.store_key(provider, key);
                    }
                }
            }
        }
        let confirming = self.providers.values().any(|state| state.confirm_remove);
        if !confirming
            && !egui::Popup::is_any_open(&ctx)
            && ctx.input(|input| input.key_pressed(egui::Key::Escape))
        {
            if self.search.is_empty() {
                self.close();
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
                record(&ctx, Target::Field("Search settings"), search.rect);
                if std::mem::take(&mut self.focus_search) {
                    search.request_focus();
                }
                ui.add_space(theme::SPACE);
                for (section, name) in SECTIONS {
                    let selected = self.section == section && self.search.is_empty();
                    let button = ui.selectable_label(selected, name);
                    record(&ctx, Target::Button(name), button.rect);
                    if button.clicked() {
                        self.section = section;
                        self.search.clear();
                    }
                }
                ui.add_space(theme::SPACE_L);
                let close = ui.button("Close").on_hover_text("Esc");
                record(&ctx, Target::Button("Close settings"), close.rect);
                if close.clicked() {
                    self.close();
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
                            Section::Projects => self.projects_ui(ui, theme, recent, &mut changed),
                            Section::Advanced => self.advanced_ui(ui, theme, &mut changed),
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
    fn store_key(&mut self, provider: Provider, key: String) -> bool {
        let result = keys::store(provider, &key);
        let hint = keys::hint(&key);
        let state = self.provider(provider);
        match result {
            Ok(()) => {
                // The key is gone from the form once it is saved (R-25).
                state.input.clear();
                state.tested.clear();
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
                format!("From environment variable {variable} (the saved key is ignored).")
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
                if provider == Provider::Anthropic {
                    ui.label(
                        RichText::new(
                            "The Assistant reads Anthropic's key from ANTHROPIC_API_KEY until Anthropic moves onto the provider layer (W5.7); a key saved here is tested and kept for then.",
                        )
                        .font(theme::regular(theme::CAPTION))
                        .color(theme.muted),
                    );
                }
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
                        let busy = state.test.running();
                        // The key cannot change while it is being tested.
                        let field = ui.add_enabled(
                            !busy,
                            egui::TextEdit::singleline(&mut state.input)
                                .password(true)
                                .hint_text(format!("Paste the {} key", provider.name()))
                                .desired_width(280.0),
                        );
                        if field.changed() {
                            // A result belongs to the key it tested.
                            state.test = Background::Idle;
                            state.save_after_test = false;
                            state.message = None;
                        }
                        let has_input = !state.input.trim().is_empty();
                        if ui.add_enabled(has_input && !busy, egui::Button::new("Test")).clicked() {
                            state.tested = state.input.trim().to_string();
                            let key = state.tested.clone();
                            state.test = Background::start(move || Providers::new().check_key(provider, Some(&key)));
                        }
                        if ui.add_enabled(has_input && !busy, egui::Button::new("Save")).clicked() {
                            // Save runs the test first (R-25 point 3).
                            state.tested = state.input.trim().to_string();
                            let key = state.tested.clone();
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
                                        state.removed = true;
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
        if std::mem::take(&mut self.provider(provider).removed) {
            self.set(&format!("providers.{}.keyHint", provider.id()), json!(""));
        }
        if std::mem::take(&mut self.provider(provider).save_anyway) {
            // The failed test was on this very key: the field resets the
            // result when it changes.
            let key = self.provider(provider).tested.clone();
            changed |= self.store_key(provider, key);
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
            // A model and an effort belong to a chosen provider.
            let automatic = self.text("assistant.provider").is_empty();
            let mut draft = self.drafts.remove(id).unwrap_or_else(|| current.clone());
            let mut typing = false;
            let set_by = match id {
                "assistant.provider" => "AGENTIQUE_PROVIDER",
                "assistant.model" => "AGENTIQUE_MODEL",
                "assistant.effort" => "AGENTIQUE_EFFORT",
                _ => "",
            };
            let text = RowText {
                label: row.label,
                description: row.description,
                query,
                set_by: (!set_by.is_empty() && std::env::var_os(set_by).is_some())
                    .then_some(set_by),
            };
            let new_value = setting_row(ui, theme, text, is_changed, &default, |ui, label| {
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
                            })
                            .response
                            .labelled_by(label);
                        (value != current).then(|| json!(value))
                    }
                    "assistant.showCost" => {
                        let mut on = self.settings.get(id).as_bool().unwrap_or(true);
                        let before = on;
                        ui.checkbox(&mut on, "").labelled_by(label);
                        (on != before).then(|| json!(on))
                    }
                    _ => {
                        let response = ui
                            .add_enabled(
                                !automatic,
                                egui::TextEdit::singleline(&mut draft)
                                    .hint_text("Default")
                                    .desired_width(200.0),
                            )
                            .on_disabled_hover_text("Choose a provider first")
                            .labelled_by(label);
                        typing = response.has_focus();
                        // Text commits on Enter or leaving the field (§3.7).
                        (response.lost_focus() && draft.trim() != current)
                            .then(|| json!(draft.trim()))
                    }
                }
            });
            if typing {
                self.drafts.insert(id, draft);
            }
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
            let text = RowText {
                label: row.label,
                description: row.description,
                query,
                set_by: None,
            };
            let new_value = setting_row(
                ui,
                theme,
                text,
                is_changed,
                &default,
                |ui, label| match row.allowed {
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
                            })
                            .response
                            .labelled_by(label);
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
                            })
                            .response
                            .labelled_by(label);
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

    /// Where new projects go, and the recent projects (§3.7 Projects).
    fn projects_ui(
        &mut self,
        ui: &mut egui::Ui,
        theme: Theme,
        recent: &[PathBuf],
        changed: &mut Changed,
    ) {
        section_title(ui, theme, "Projects");
        let id = "projects.defaultFolder";
        let row = settings::setting(id).expect("a setting");
        let current = self.text(id);
        let mut draft = self.drafts.remove(id).unwrap_or_else(|| current.clone());
        let mut typing = false;
        let text = RowText {
            label: row.label,
            description: row.description,
            query: None,
            set_by: None,
        };
        let new_value = setting_row(
            ui,
            theme,
            text,
            self.settings.changed(id),
            &row.default.value(),
            |ui, label| {
                let response = ui
                    .add(
                        egui::TextEdit::singleline(&mut draft)
                            .hint_text("Agentique in your user folder")
                            .desired_width(260.0),
                    )
                    .labelled_by(label);
                typing = response.has_focus();
                (response.lost_focus() && draft.trim() != current).then(|| json!(draft.trim()))
            },
        );
        if typing {
            self.drafts.insert(id, draft);
        }
        if let Some(value) = new_value {
            self.set(id, value);
        }
        ui.add_space(theme::SPACE);
        ui.label(RichText::new("Recent projects").color(theme.text));
        if recent.is_empty() {
            ui.label(RichText::new("None yet.").color(theme.muted));
        }
        for folder in recent {
            ui.horizontal(|ui| {
                ui.label(RichText::new(folder.display().to_string()).color(theme.text_secondary));
                if ui.small_button("Remove from the list").clicked() {
                    changed.remove_recent = Some(folder.clone());
                }
            });
        }
        ui.add_space(theme::SPACE);
        let data = self.path.with_file_name("projects");
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Each project's conversation is kept in {}.",
                    data.display()
                ))
                .color(theme.muted),
            );
            if ui.small_button("Open folder").clicked() {
                open_in_explorer(&data);
            }
        });
    }

    /// The settings file, and resetting everything (§3.7 Advanced, with its
    /// Danger zone).
    fn advanced_ui(&mut self, ui: &mut egui::Ui, theme: Theme, changed: &mut Changed) {
        section_title(ui, theme, "Advanced");
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Settings file: {}", self.path.display()))
                    .color(theme.text_secondary),
            );
            if ui.small_button("Show in Explorer").clicked() {
                open_in_explorer(&self.path);
            }
        });
        ui.add_space(theme::SPACE_L);
        egui::Frame::new()
            .stroke(Stroke::new(theme::HAIRLINE, theme.error))
            .corner_radius(theme::RADIUS)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Danger zone").font(theme::semibold(theme::BODY)).color(theme.text));
                ui.label(
                    RichText::new("Every setting goes back to its default. Saved keys stay; remove them in Providers. A copy of the file is kept as settings.json.bak.")
                        .color(theme.text_secondary),
                );
                if self.confirm_reset {
                    ui.horizontal(|ui| {
                        let cancel = ui.button("Cancel");
                        if std::mem::take(&mut self.focus_cancel_reset) {
                            cancel.request_focus();
                        }
                        if cancel.clicked() {
                            self.confirm_reset = false;
                        }
                        if ui.button(RichText::new("Reset all settings").color(theme.error)).clicked() {
                            self.confirm_reset = false;
                            if self.reset_all() {
                                changed.appearance = true;
                                changed.assistant = true;
                            }
                        }
                    });
                } else if ui.button("Reset all settings…").clicked() {
                    self.confirm_reset = true;
                    self.focus_cancel_reset = true;
                }
            });
    }

    /// Resets every setting after keeping a copy of the file; false when the
    /// copy could not be made (then nothing is reset).
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
        self.drafts.clear();
        self.save();
        true
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

/// Opens a folder, or shows a file, in Windows Explorer.
fn open_in_explorer(path: &std::path::Path) {
    let mut command = std::process::Command::new("explorer");
    if path.is_file() {
        command.arg(format!("/select,{}", path.display()));
    } else {
        command.arg(path);
    }
    let _ = command.spawn();
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
/// What a row shows besides its control.
struct RowText<'a> {
    label: &'a str,
    description: &'a str,
    /// The search, whose words are highlighted (§3.7).
    query: Option<&'a str>,
    /// The environment variable that decides this row instead (§3.7: rows
    /// that cannot be changed stay visible, disabled, with the reason).
    set_by: Option<&'static str>,
}

fn setting_row(
    ui: &mut egui::Ui,
    theme: Theme,
    text: RowText,
    is_changed: bool,
    default: &Value,
    control: impl FnOnce(&mut egui::Ui, egui::Id) -> Option<Value>,
) -> Option<Value> {
    let mut result = None;
    let mut label_id = egui::Id::NULL;
    let RowText {
        label,
        description,
        query,
        set_by,
    } = text;
    let reason = set_by.map(|variable| format!("Set by {variable}."));
    let description = match &reason {
        Some(reason) => format!("{reason} {description}"),
        None => description.to_string(),
    };
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 260.0).max(200.0));
            ui.horizontal(|ui| {
                let job = highlighted(label, query, theme::regular(theme::BODY), theme.text, theme);
                label_id = ui.label(job).id;
                if is_changed {
                    ui.label(
                        RichText::new("●")
                            .font(theme::regular(theme::CAPTION))
                            .color(theme.accent),
                    )
                    .on_hover_text("Changed from the default");
                }
            });
            ui.label(highlighted(
                &description,
                query,
                theme::regular(theme::CAPTION),
                theme.muted,
                theme,
            ));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if set_by.is_some() {
                // Shown as it is, not changeable here.
                ui.add_enabled_ui(false, |ui| {
                    let _ = control(ui, label_id);
                })
                .response
                .on_disabled_hover_text(reason.unwrap_or_default());
                return;
            }
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
            // The control is named by its row's label for screen readers
            // (§3.5).
            if let Some(value) = control(ui, label_id) {
                result = Some(value);
            }
        });
    });
    ui.add_space(theme::SPACE);
    result
}

/// `text` with the words of `query` marked, as a search result shows them.
fn highlighted(
    text: &str,
    query: Option<&str>,
    font: egui::FontId,
    color: egui::Color32,
    theme: Theme,
) -> egui::text::LayoutJob {
    let mut marked = vec![false; text.len()];
    // Case-folded search; only where folding keeps byte offsets (ASCII).
    let lower = text.to_ascii_lowercase();
    for word in query.unwrap_or_default().split_whitespace() {
        let word = word.to_ascii_lowercase();
        let mut from = 0;
        while let Some(at) = lower[from..].find(&word) {
            let start = from + at;
            marked[start..start + word.len()]
                .iter_mut()
                .for_each(|m| *m = true);
            from = start + word.len().max(1);
        }
    }
    let mut job = egui::text::LayoutJob::default();
    let format = |marked: bool| egui::text::TextFormat {
        font_id: font.clone(),
        color,
        background: if marked {
            theme.accent.gamma_multiply(0.25)
        } else {
            egui::Color32::TRANSPARENT
        },
        ..Default::default()
    };
    let mut start = 0;
    for end in 1..=text.len() {
        if !text.is_char_boundary(end) {
            continue;
        }
        let boundary = end == text.len() || marked[end] != marked[start];
        if boundary && text.is_char_boundary(start) {
            job.append(&text[start..end], 0.0, format(marked[start]));
            start = end;
        }
    }
    job
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

    #[test]
    fn every_choice_in_settings_is_named_by_its_row() {
        use eframe::egui::accesskit::Role;
        let (mut app, context, _folder) = studio("settings-names");
        app.settings.show(super::Section::Appearance);
        context.enable_accesskit();
        frame(&mut app, &context, vec![]);
        let output = frame(&mut app, &context, vec![]);
        let combos: Vec<_> = output
            .accesskit_update
            .iter()
            .flat_map(|update| &update.nodes)
            .filter(|(_, node)| node.role() == Role::ComboBox)
            .collect();
        // Theme, UI scale and reduced motion.
        assert_eq!(combos.len(), 3, "{combos:#?}");
        for (_, node) in combos {
            assert!(!node.labelled_by().is_empty(), "{node:?}");
        }
    }

    #[test]
    fn the_model_picker_chooses_settings_provider_and_the_next_turns_model() {
        let (mut app, context, _folder) = studio("model-picker");
        frame(&mut app, &context, vec![]);
        app.settings
            .set_value("assistant.model", serde_json::json!("some-model"));
        app.choose_provider("deepseek");
        assert_eq!(app.settings.text("assistant.provider"), "deepseek");
        // The provider's own defaults, not a model typed for another.
        assert_eq!(app.settings.text("assistant.model"), "");
        if std::env::var_os("AGENTIQUE_PROVIDER").is_none() {
            assert!(
                app.conversation.model_name.starts_with("deepseek-flash"),
                "{}",
                app.conversation.model_name
            );
        }
        app.choose_provider("");
        assert_eq!(app.settings.text("assistant.provider"), "");
    }

    #[test]
    fn projects_and_advanced_reset_remove_recent_and_suggest_the_folder() {
        let (mut app, context, folder) = studio("settings-projects");
        frame(&mut app, &context, vec![]);
        // The folder for new projects is what New project suggests.
        let base = folder.0.join("Work");
        app.settings.set_value(
            "projects.defaultFolder",
            serde_json::json!(base.display().to_string()),
        );
        app.execute(CommandId::NewProject, &context);
        let Some(crate::edit::Dialog::NewProject {
            folder: suggested, ..
        }) = &app.dialog
        else {
            panic!("the New project dialog is open")
        };
        assert!(
            std::path::Path::new(suggested).starts_with(&base),
            "{suggested}"
        );
        app.dialog = None;
        // A project comes off the recent list.
        let recent = app.session.recent[0].clone();
        app.apply_settings(
            &context,
            super::Changed {
                remove_recent: Some(recent.clone()),
                ..Default::default()
            },
        );
        assert!(!app.session.recent.contains(&recent));
        // Resetting keeps a copy of the file first.
        app.settings
            .set_value("appearance.theme", serde_json::json!("dark"));
        assert!(app.settings.reset_all());
        assert_eq!(app.settings.text("appearance.theme"), "system");
        assert!(folder.0.join("settings.json.bak").exists());
        // Ctrl+, opens at the last section viewed.
        app.settings.show(super::Section::Advanced);
        app.settings.close();
        app.execute(CommandId::Settings, &context);
        assert_eq!(app.settings.section, super::Section::Advanced);
    }

    #[test]
    fn the_ui_scale_applies_and_goes_back_to_100_percent() {
        let (mut app, context, _folder) = studio("settings-scale");
        app.settings
            .set_value("appearance.uiScale", serde_json::json!(1.5));
        app.apply_settings(
            &context,
            super::Changed {
                appearance: true,
                ..Default::default()
            },
        );
        frame(&mut app, &context, vec![]);
        assert_eq!(context.zoom_factor(), 1.5);
        // 100% is the default, so the file forgets the value; it still applies.
        app.settings
            .set_value("appearance.uiScale", serde_json::json!(1.0));
        app.apply_settings(
            &context,
            super::Changed {
                appearance: true,
                ..Default::default()
            },
        );
        frame(&mut app, &context, vec![]);
        assert_eq!(context.zoom_factor(), 1.0);
    }

    #[test]
    fn a_stage_4_session_gives_its_appearance_to_settings_once() {
        use clap::Parser;
        let folder =
            std::env::temp_dir().join(format!("agq-settings-adopt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let session_path = folder.join("session.json");
        crate::session::Session {
            version: crate::session::Session::VERSION,
            dark: false,
            high_contrast: true,
            reduced_motion: true,
            ..Default::default()
        }
        .save(&session_path)
        .unwrap();
        let start = || {
            let args = crate::Args::parse_from([
                "studio",
                "--no-restore",
                "--session",
                session_path.to_str().unwrap(),
            ]);
            let creation = eframe::CreationContext::_new_kittest(egui::Context::default());
            crate::app::StudioApp::new(&creation, args)
        };
        let mut app = start();
        assert_eq!(app.settings.text("appearance.theme"), "high-contrast");
        assert_eq!(app.settings.text("appearance.reducedMotion"), "on");
        assert!(app.theme.contrast && app.reduced_motion);
        // Once: after "Follow Windows" is chosen, a restart keeps it.
        app.settings
            .set_value("appearance.theme", serde_json::json!("system"));
        app.save_session();
        drop(app);
        let again = start();
        assert_eq!(again.settings.text("appearance.theme"), "system");
        drop(again);
        let _ = std::fs::remove_dir_all(&folder);
    }
}

#[cfg(test)]
mod highlight_tests {
    use super::*;

    #[test]
    fn search_words_are_marked_in_labels_and_descriptions() {
        let theme = Theme::default();
        let job = highlighted(
            "Show tokens and estimated cost",
            Some("cost token"),
            theme::regular(theme::BODY),
            theme.text,
            theme,
        );
        let marked: Vec<&str> = job
            .sections
            .iter()
            .filter(|section| section.format.background != egui::Color32::TRANSPARENT)
            .map(|section| &job.text[section.byte_range.start.0..section.byte_range.end.0])
            .collect();
        assert_eq!(marked, ["token", "cost"]);
        assert_eq!(job.text, "Show tokens and estimated cost");
        // Nothing searched: nothing marked, text whole.
        let plain = highlighted(
            "Théme",
            None,
            theme::regular(theme::BODY),
            theme.text,
            theme,
        );
        assert_eq!(plain.text, "Théme");
    }
}
