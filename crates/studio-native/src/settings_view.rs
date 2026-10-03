//! The Settings view (ROADMAP §3.7, W5.8, R-24; Scenario E): Ctrl+, shows
//! it in place of the Surface and the Panels. Sections on the left under a
//! search box; one column of rows of at most 720 points on the right, each a
//! label, a one-line description and its control, with a mark and a reset
//! when it differs from its default. Choices apply at once; text commits on
//! Enter or leaving the field. A key has its own small form (paste, Test,
//! Save) and is never shown again after saving, only its hint (R-25). The
//! rows come from the settings table (`settings.rs`).
use crate::{
    commands::COMMANDS,
    settings::{self, Allowed, Section},
    studio::{Dirty, Studio, StudioEvent},
    ui::{
        self, ActiveTheme, Button, Chip, IconName, KeyCaps, Menu, MenuItem, Segmented, Switch,
        TextField, Tone, icon, r, theme,
    },
    workspace::StudioExt,
};
use agq_providers::{KeyCheck, KeyStatus, ModelInfo, Provider, Providers, keys};
use gpui::{
    AnyElement, App, AppContext, ClickEvent, Context, DismissEvent, Entity, FocusHandle, Focusable,
    HighlightStyle, InteractiveElement, IntoElement, KeyBinding, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, StyledText, Subscription, Task, Window,
    actions, anchored, deferred, div, prelude::FluentBuilder,
};
use gpui_base::input::{InputEvent, InputState};
use serde_json::{Value, json};
use std::collections::BTreeMap;

actions!(settings_view, [Close]);

pub fn bind(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Close, Some("Settings"))]);
}

/// The widest the content column gets (§3.7).
const COLUMN: f32 = 720.0;

fn section_icon(section: Section) -> IconName {
    match section {
        Section::Providers => IconName::Key,
        Section::Assistant => IconName::Assistant,
        Section::Appearance => IconName::Palette,
        Section::Keyboard => IconName::Keyboard,
        Section::Projects => IconName::Folder,
        Section::Advanced => IconName::Sliders,
        Section::About => IconName::Info,
    }
}

/// One provider's key form and model list.
struct ProviderState {
    input: Entity<InputState>,
    testing: Option<Task<()>>,
    check: Option<KeyCheck>,
    /// The key the last test ran on; Save stores exactly this key.
    tested: String,
    save_after_test: bool,
    models: Option<Result<Vec<ModelInfo>, String>>,
    listing: Option<Task<()>>,
    confirm_remove: bool,
    status: KeyStatus,
    message: Option<String>,
    _subscription: Subscription,
}

pub struct SettingsView {
    studio: Entity<Studio>,
    focus: FocusHandle,
    search: Entity<InputState>,
    providers: BTreeMap<Provider, ProviderState>,
    /// Text rows being typed into, committed on Enter or leaving them.
    drafts: BTreeMap<&'static str, Entity<InputState>>,
    confirm_reset: bool,
    open_menu: Option<(&'static str, Entity<Menu>, Subscription)>,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Search matches labels, descriptions and synonyms (§3.7: people search in
/// their own words).
pub fn matches(query: &str, words: &[&str]) -> bool {
    query.split_whitespace().all(|term| {
        words
            .iter()
            .any(|word| word.to_lowercase().contains(&term.to_lowercase()))
    })
}

pub fn visible(row: &settings::Setting, query: Option<&str>) -> bool {
    query.is_none_or(|query| {
        let mut words = vec![row.label, row.description];
        words.extend(row.synonyms);
        matches(query, &words)
    })
}

/// What a key test found, in plain words (§2.4 E2), and whether it works.
pub fn describe(provider: Provider, check: &KeyCheck) -> (String, bool) {
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

fn choice_label(choice: &str) -> String {
    match choice {
        "system" => "Follow Windows".into(),
        "light" => "Light".into(),
        "dark" => "Dark".into(),
        "high-contrast" => "High contrast".into(),
        "on" => "On".into(),
        "off" => "Off".into(),
        "" => "Automatic".into(),
        "loop" => "Agentique".into(),
        "claude-agent" => "Claude Agent".into(),
        other => other.to_string(),
    }
}

fn describe_default(default: &Value) -> String {
    match default {
        Value::String(text) => choice_label(text),
        Value::Number(number) => format!("{:.0}%", number.as_f64().unwrap_or(1.0) * 100.0),
        Value::Bool(true) => "on".into(),
        Value::Bool(false) => "off".into(),
        other => other.to_string(),
    }
}

/// Byte ranges of `text` that the query's words match (ASCII case folding,
/// which keeps byte offsets).
fn marked(text: &str, query: Option<&str>) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let lower = text.to_ascii_lowercase();
    for word in query.unwrap_or_default().split_whitespace() {
        let word = word.to_ascii_lowercase();
        let mut from = 0;
        while let Some(at) = lower[from..].find(&word) {
            let start = from + at;
            ranges.push(start..start + word.len());
            from = start + word.len().max(1);
        }
    }
    ranges.sort_by_key(|range| range.start);
    ranges
}

fn highlighted(text: &str, query: Option<&str>, cx: &App) -> AnyElement {
    let ranges = marked(text, query);
    if ranges.is_empty() {
        return SharedString::from(text.to_string()).into_any_element();
    }
    let theme = cx.theme();
    let style = HighlightStyle {
        background_color: Some(theme.accent.solid.opacity(0.25)),
        ..Default::default()
    };
    StyledText::new(text.to_string())
        .with_highlights(ranges.into_iter().map(|range| (range, style)))
        .into_any_element()
}

fn open_in_explorer(path: &std::path::Path) {
    let mut command = std::process::Command::new("explorer");
    if path.is_file() {
        command.arg(format!("/select,{}", path.display()));
    } else {
        command.arg(path);
    }
    let _ = command.spawn();
}

impl SettingsView {
    /// What Settings are searching for (the scripted journeys).
    #[cfg(feature = "automation")]
    pub fn search_text(&self, cx: &App) -> String {
        self.search.read(cx).value().to_string()
    }

    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search settings"));
        let subscriptions = vec![
            cx.subscribe(&studio, |_, _, event: &StudioEvent, cx| {
                if event.0.intersects(
                    Dirty::OVERLAY | Dirty::APPEARANCE | Dirty::CONVERSATION | Dirty::STATUS,
                ) {
                    cx.notify();
                }
            }),
            cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        SettingsView {
            studio,
            focus: cx.focus_handle(),
            search,
            providers: BTreeMap::new(),
            drafts: BTreeMap::new(),
            confirm_reset: false,
            open_menu: None,
            _subscriptions: subscriptions,
        }
    }

    /// Settings has the keyboard: the search box.
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.search.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
    }

    fn close(&mut self, _: &Close, window: &mut Window, cx: &mut Context<Self>) {
        // Escape clears the search first.
        if !self.search.read(cx).value().is_empty() {
            self.search
                .update(cx, |state, cx| state.set_value("", window, cx));
            return;
        }
        if self.confirm_reset || self.providers.values().any(|p| p.confirm_remove) {
            self.confirm_reset = false;
            for state in self.providers.values_mut() {
                state.confirm_remove = false;
            }
            cx.notify();
            return;
        }
        // A pasted key that was not saved is dropped (R-25).
        for state in self.providers.values_mut() {
            state
                .input
                .update(cx, |input, cx| input.set_value("", window, cx));
            state.tested.clear();
        }
        self.studio.act(cx, |studio| {
            studio.settings_open = false;
            studio.mark(Dirty::OVERLAY | Dirty::LAYOUT);
        });
    }

    fn set(&mut self, id: &'static str, value: Value, cx: &mut Context<Self>) {
        self.studio.act(cx, |studio| {
            if studio.settings.set(id, value).is_ok() {
                match id.split('.').next() {
                    Some("appearance") => studio.apply_appearance(),
                    Some("assistant") => studio.apply_runtime_choice(),
                    _ => {}
                }
            }
            studio.mark(Dirty::ALL);
        });
    }

    fn provider(
        &mut self,
        provider: Provider,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> &mut ProviderState {
        if let std::collections::btree_map::Entry::Vacant(_) = self.providers.entry(provider) {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(format!("Paste the {} key", provider.name()))
                    .masked(true)
            });
            let subscription = cx.subscribe_in(
                &input,
                window,
                move |this, _, event: &InputEvent, window, cx| {
                    match event {
                        // A result belongs to the key it tested.
                        InputEvent::Change => {
                            if let Some(state) = this.providers.get_mut(&provider) {
                                state.check = None;
                                state.save_after_test = false;
                                state.message = None;
                            }
                            cx.notify();
                        }
                        InputEvent::PressEnter { .. } => this.test(provider, true, window, cx),
                        _ => {}
                    }
                },
            );
            self.providers.insert(
                provider,
                ProviderState {
                    input,
                    testing: None,
                    check: None,
                    tested: String::new(),
                    save_after_test: false,
                    models: None,
                    listing: None,
                    confirm_remove: false,
                    status: agq_providers::key_status(provider),
                    message: None,
                    _subscription: subscription,
                },
            );
        }
        self.providers.get_mut(&provider).expect("inserted above")
    }

    /// Tests the pasted key (and saves it when it works, for Save: R-25
    /// point 3). The test runs no model.
    fn test(
        &mut self,
        provider: Provider,
        save: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = self.provider(provider, window, cx);
        let key = state.input.read(cx).value().trim().to_string();
        if key.is_empty() || state.testing.is_some() {
            return;
        }
        state.tested = key.clone();
        state.save_after_test = save;
        state.check = None;
        let work = cx
            .background_executor()
            .spawn(async move { Providers::new().check_key(provider, Some(&key)) });
        state.testing = Some(cx.spawn_in(window, async move |this, cx| {
            let check = work.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.tested(provider, check, window, cx)
            });
        }));
        cx.notify();
    }

    fn tested(
        &mut self,
        provider: Provider,
        check: KeyCheck,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let works = matches!(check, KeyCheck::Works);
        let state = self.provider(provider, window, cx);
        state.testing = None;
        state.check = Some(check);
        let key = state.tested.clone();
        // A key that works shows its models at once, listed with that key
        // even before it is saved (E2).
        if works {
            self.list_models(provider, Some(key.clone()), window, cx);
        }
        let state = self.provider(provider, window, cx);
        if std::mem::take(&mut state.save_after_test) && works {
            self.store(provider, key, window, cx);
        }
        cx.notify();
    }

    fn list_models(
        &mut self,
        provider: Provider,
        key: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let work = cx.background_executor().spawn(async move {
            let providers = match key {
                Some(key) => Providers::new().with_key(provider, key),
                None => Providers::new(),
            };
            providers
                .list_models(provider)
                .map_err(|error| error.message)
        });
        let task = cx.spawn_in(window, async move |this, cx| {
            let models = work.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(state) = this.providers.get_mut(&provider) {
                    state.listing = None;
                    state.models = Some(models);
                }
                cx.notify();
            });
        });
        self.provider(provider, window, cx).listing = Some(task);
        cx.notify();
    }

    /// Saves the key in the Credential Manager and its hint in the settings;
    /// the key leaves the form once it is saved (R-25 point 2).
    fn store(
        &mut self,
        provider: Provider,
        key: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = keys::store(provider, &key);
        let hint = keys::hint(&key);
        let state = self.provider(provider, window, cx);
        match result {
            Ok(()) => {
                state
                    .input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                state.tested.clear();
                state.status = KeyStatus::Stored;
                state.message = Some("Saved in Windows Credential Manager.".into());
                state.check = None;
                let id: &'static str = match provider {
                    Provider::Anthropic => "providers.anthropic.keyHint",
                    Provider::OpenAi => "providers.openai.keyHint",
                    Provider::OpenRouter => "providers.openrouter.keyHint",
                    Provider::DeepSeek => "providers.deepseek.keyHint",
                    Provider::TypeSafe => "providers.typesafe.keyHint",
                };
                self.set(id, json!(hint), cx);
            }
            Err(error) => state.message = Some(error.0),
        }
        cx.notify();
    }

    fn remove(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.provider(provider, window, cx);
        state.confirm_remove = false;
        match keys::remove(provider) {
            Ok(()) => {
                state.status = agq_providers::key_status(provider);
                state.message = Some("The saved key was removed.".into());
                let id: &'static str = match provider {
                    Provider::Anthropic => "providers.anthropic.keyHint",
                    Provider::OpenAi => "providers.openai.keyHint",
                    Provider::OpenRouter => "providers.openrouter.keyHint",
                    Provider::DeepSeek => "providers.deepseek.keyHint",
                    Provider::TypeSafe => "providers.typesafe.keyHint",
                };
                self.set(id, json!(""), cx);
            }
            Err(error) => state.message = Some(error.0),
        }
        cx.notify();
    }

    fn draft(
        &mut self,
        id: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(state) = self.drafts.get(id) {
            // Show the saved value unless it is being typed in.
            let focused = state.read(cx).focus_handle(cx).is_focused(window);
            let current = self.studio.read(cx).settings.text(id);
            if !focused && state.read(cx).value() != current {
                state.update(cx, |state, cx| state.set_value(current, window, cx));
            }
            return state.clone();
        }
        let current = self.studio.read(cx).settings.text(id);
        let placeholder = match id {
            "assistant.model" => "The provider's default",
            "assistant.effort" => "The model's default",
            "projects.defaultFolder" => "Agentique in your user folder",
            _ => "",
        };
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(current)
        });
        let subscription = cx.subscribe_in(
            &state,
            window,
            move |this, state, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    let text = state.read(cx).value().trim().to_string();
                    if this.studio.read(cx).settings.text(id) != text {
                        this.set(id, json!(text), cx);
                    }
                }
            },
        );
        self._subscriptions.push(subscription);
        self.drafts.insert(id, state.clone());
        state
    }

    fn open_choice(
        &mut self,
        id: &'static str,
        choices: Vec<(String, String)>,
        current: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let this = cx.entity();
        let items = choices
            .into_iter()
            .map(|(value, label)| {
                let this = this.clone();
                let chosen = value == current;
                MenuItem::action(label, move |_, cx| {
                    let value = value.clone();
                    this.update(cx, |view, cx| view.set(id, json!(value), cx));
                })
                .checked(chosen)
            })
            .collect();
        let menu = cx.new(|cx| Menu::new(items, cx).min_width(220.0));
        let subscription = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, _, cx| {
            this.open_menu = None;
            cx.notify();
        });
        let focus = menu.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.open_menu = Some((id, menu, subscription));
        cx.notify();
    }
}

/// A click handler.
type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
/// A changed setting's default, and what resets it.
type Reset = (Value, OnClick);

/// One row: the label and description on the left, the control on the
/// right, a mark and a reset when it differs from its default (§3.7).
#[allow(clippy::too_many_arguments)]
fn row(
    id: &'static str,
    label: &str,
    description: &str,
    query: Option<&str>,
    set_by: Option<&'static str>,
    changed: bool,
    reset: Option<Reset>,
    control: AnyElement,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .id(SharedString::from(format!("row-{id}")))
        .px(r(16.0))
        .py(r(12.0))
        .flex()
        .items_center()
        .gap(r(16.0))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(r(3.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(6.0))
                        .text_size(r(theme::text::BASE))
                        .font_weight(theme::MEDIUM)
                        .child(highlighted(label, query, cx))
                        .when(changed, |this| {
                            let tooltip = ui::tooltip::text("Changed from the default", None);
                            this.child(
                                div()
                                    .id(SharedString::from(format!("changed-{id}")))
                                    .size(gpui::px(6.0))
                                    .rounded_full()
                                    .bg(theme.accent.solid)
                                    .tooltip(move |window, cx| tooltip(window, cx)),
                            )
                        }),
                )
                .child(
                    div()
                        .text_size(r(theme::text::SM))
                        .line_height(r(17.0))
                        .text_color(theme.text_muted)
                        .child(highlighted(description, query, cx)),
                )
                .when_some(set_by, |this, variable| {
                    this.child(
                        div().pt(r(2.0)).child(
                            Chip::new(format!("Set by {variable}"))
                                .icon(IconName::Lock)
                                .tone(Tone::Info),
                        ),
                    )
                }),
        )
        .when_some(
            reset.filter(|_| set_by.is_none()),
            |this, (default, on_reset)| {
                this.child(
                    Button::icon_only(
                        SharedString::from(format!("reset-{id}")),
                        IconName::Retry,
                        "Reset",
                    )
                    .small()
                    .tooltip(
                        format!("Reset to the default: {}", describe_default(&default)),
                        None,
                    )
                    .on_click(on_reset),
                )
            },
        )
        .child(
            div()
                .flex_none()
                .when(set_by.is_some(), |this| this.opacity(0.5))
                .child(control),
        )
}

impl SettingsView {
    /// The Claude Agent runtime: what it is, whether it is ready, and the
    /// Install and Check actions (ROADMAP §4.7).
    fn runtime_card(&mut self, query: Option<&str>, cx: &mut Context<Self>) -> Option<AnyElement> {
        const TITLE: &str = "Claude Agent runtime";
        const ABOUT: &str = "The Claude Agent SDK runs the Assistant's loop in a companion process, with Agentique's tools only: every model change still goes through the System State, every command through Execution, and every approval through you. Anthropic only; it needs Node.js and an Anthropic API key.";
        if query
            .is_some_and(|q| !matches(q, &[TITLE, ABOUT, "sdk", "node", "install", "claude agent"]))
        {
            return None;
        }
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let busy = studio.runtime.work.is_some();
        let installing = studio.installing_runtime();
        let message = studio.runtime.message.clone();
        let confirm = studio.runtime.confirm_install;
        let safe_mode = studio.safe_mode;
        let key = agq_providers::key_status(
            crate::agent_runtime::model_access(studio.settings.text("assistant.provider").as_str())
                .map(|e| e.provider)
                .unwrap_or(Provider::Anthropic),
        );
        let lines = studio
            .runtime
            .health
            .as_ref()
            .map(|h| h.lines(&key))
            .unwrap_or_default();
        let studio_entity = self.studio.clone();
        let act = move |f: fn(&mut Studio)| {
            let studio = studio_entity.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| studio.act(cx, f)
        };
        let mut rows: Vec<AnyElement> = vec![
            div()
                .px(r(16.0))
                .py(r(12.0))
                .flex()
                .flex_col()
                .gap(r(6.0))
                .child(
                    div()
                        .text_size(r(theme::text::SM))
                        .line_height(r(17.0))
                        .text_color(theme.text_muted)
                        .child(highlighted(ABOUT, query, cx)),
                )
                .child(
                    div()
                        .text_size(r(theme::text::SM))
                        .line_height(r(17.0))
                        .text_color(theme.text_muted)
                        .child("These controls decide which tools the agent can call. They are not an operating-system sandbox: the runtime runs with your rights and reaches Anthropic's API."),
                )
                .into_any_element(),
        ];
        if safe_mode {
            rows.push(
                div()
                    .px(r(16.0))
                    .py(r(8.0))
                    .child(ui::inline_message(
                        Tone::Warning,
                        "Agentique started in safe mode: the Claude Agent runtime is not used until it starts normally.",
                        cx,
                    ))
                    .into_any_element(),
            );
        }
        if !lines.is_empty() {
            rows.push(
                div()
                    .px(r(16.0))
                    .py(r(10.0))
                    .flex()
                    .flex_col()
                    .gap(r(6.0))
                    .children(lines.into_iter().map(|(fine, text)| {
                        div()
                            .flex()
                            .items_start()
                            .gap(r(8.0))
                            .child(
                                icon(if fine {
                                    IconName::Check
                                } else {
                                    IconName::Warning
                                })
                                .size(14.0)
                                .color(if fine {
                                    theme.success.text
                                } else {
                                    theme.warning.text
                                }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(r(theme::text::SM))
                                    .line_height(r(17.0))
                                    .child(text),
                            )
                    }))
                    .into_any_element(),
            );
        }
        if let Some(message) = message {
            rows.push(
                div()
                    .px(r(16.0))
                    .py(r(8.0))
                    .child(ui::inline_message(Tone::Neutral, message, cx))
                    .into_any_element(),
            );
        }
        let mut actions = div()
            .px(r(16.0))
            .py(r(12.0))
            .flex()
            .items_center()
            .gap(r(8.0));
        if confirm {
            actions = actions
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(r(theme::text::SM))
                        .child(format!(
                            "Download the Claude Agent SDK {} and its Claude Code binary (about 300 MB) from the npm registry into {}?",
                            agq_assistant::claude_agent::SDK_VERSION,
                            agq_assistant::claude_agent::Installation::default_root().display()
                        )),
                )
                .child(
                    Button::new("runtime-install-confirm", "Download and install")
                        .primary()
                        .on_click(act(|studio| studio.install_runtime())),
                )
                .child(Button::new("runtime-install-cancel", "Cancel").on_click(act(|studio| {
                    studio.runtime.confirm_install = false;
                    studio.mark(Dirty::LAYOUT);
                })));
        } else if installing {
            actions = actions.child(div().flex_1()).child(
                Button::new("runtime-install-stop", "Cancel the installation")
                    .on_click(act(|studio| studio.cancel_runtime_install())),
            );
        } else {
            actions = actions
                .child(div().flex_1())
                .child(
                    Button::new("runtime-check", "Check")
                        .disabled(busy)
                        .on_click(act(|studio| studio.check_runtime())),
                )
                .child(
                    Button::new("runtime-install", "Install…")
                        .disabled(busy)
                        .on_click(act(|studio| {
                            studio.runtime.confirm_install = true;
                            studio.mark(Dirty::LAYOUT);
                        })),
                );
        }
        rows.push(actions.into_any_element());
        Some(group(Some(TITLE), rows, cx).into_any_element())
    }
}

impl SettingsView {
    /// Agentique's own builds (C-51, ROADMAP §4.15): this version, building
    /// the repository's commit, trying a build as a test instance, using one
    /// (the launcher takes over and falls back if it does not start), and
    /// how to recover.
    fn builds_card(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let running = studio.running_build();
        let repository = studio.agentique_repository();
        let head = repository
            .as_ref()
            .and_then(|r| agq_execution::git::head(r).ok())
            .map(|h| h.commit);
        let building = studio.develop.work.is_some();
        let message = studio.develop.message.clone();
        let recovered = studio.develop.recovered.clone();
        let confirm = studio.develop.confirm_use.clone();
        let root = studio.builds_root();
        let builds: Vec<(agq_launcher::Entry, Option<String>)> = studio
            .builds()
            .map(|r| r.builds)
            .unwrap_or_default()
            .into_iter()
            .take(8)
            .map(|entry| {
                let blocker = studio.adoption_blocker(&entry.id);
                (entry, blocker)
            })
            .collect();
        let entity = self.studio.clone();
        let act = move |f: Box<dyn Fn(&mut Studio)>| {
            let studio = entity.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| studio.act(cx, |s| f(s))
        };
        let text = |t: String| {
            div()
                .text_size(r(theme::text::SM))
                .line_height(r(17.0))
                .text_color(theme.text_secondary)
                .child(t)
        };
        let mut rows: Vec<AnyElement> = vec![
            div()
                .px(r(16.0))
                .py(r(12.0))
                .flex()
                .flex_col()
                .gap(r(4.0))
                .child(div().font_weight(theme::MEDIUM).child(match &running {
                    Some(id) => format!("This version: build {id}"),
                    None => "This version was built outside Agentique (not from its builds).".to_string(),
                }))
                .child(text(
                    "Build the repository's integrated commit, try the build as a test instance with its own data, then use it: Agentique restarts in it, and returns to the last known good version if it does not start.".into(),
                ))
                .into_any_element(),
        ];
        if let Some((failed, reason)) = recovered {
            rows.push(
                div()
                    .px(r(16.0))
                    .py(r(8.0))
                    .child(ui::inline_message(
                        Tone::Warning,
                        format!("The build {failed} did not start ({reason}); this is the last known good version."),
                        cx,
                    ))
                    .into_any_element(),
            );
        }
        if let Some(message) = message {
            rows.push(
                div()
                    .px(r(16.0))
                    .py(r(8.0))
                    .child(ui::inline_message(Tone::Neutral, message, cx))
                    .into_any_element(),
            );
        }
        let mut actions = div()
            .px(r(16.0))
            .py(r(10.0))
            .flex()
            .items_center()
            .gap(r(8.0))
            .child(div().flex_1());
        if running.is_none() {
            actions = actions.child(
                Button::new("builds-keep", "Keep this version to return to")
                    .tooltip("Copies this running Agentique into the builds folder as the last known good version", None)
                    .on_click(act(Box::new(|s: &mut Studio| {
                        s.develop.message = Some(match s.keep_this_version() {
                            Ok(id) => format!("Kept this version as {id}."),
                            Err(why) => format!("Not kept: {why}"),
                        });
                        s.mark(Dirty::LAYOUT);
                    }))),
            );
        }
        actions = if building {
            actions.child(
                Button::new("builds-stop", "Stop the build")
                    .on_click(act(Box::new(|s: &mut Studio| s.cancel_build()))),
            )
        } else {
            actions.child(
                Button::new(
                    "builds-build",
                    match &head {
                        Some(commit) => format!("Build {}", &commit[..commit.len().min(10)]),
                        None => "Build".to_string(),
                    },
                )
                .primary()
                .disabled(repository.is_none())
                .tooltip(
                    "A release build of the repository's current commit, in a worktree of its own",
                    None,
                )
                .on_click(act(Box::new(|s: &mut Studio| s.build_agentique()))),
            )
        };
        rows.push(actions.into_any_element());
        for (index, (entry, blocker)) in builds.into_iter().enumerate() {
            let id = entry.id.clone();
            let state = match &entry.state {
                agq_launcher::State::Built => "built".to_string(),
                agq_launcher::State::Started => "started".to_string(),
                agq_launcher::State::Failed { reason } => format!("did not start: {reason}"),
            };
            let current = running.as_deref() == Some(id.as_str());
            let asking = confirm.as_deref() == Some(id.as_str());
            let try_id = id.clone();
            let use_id = id.clone();
            let mut row = div()
                .id(("build-row", index))
                .px(r(16.0))
                .py(r(8.0))
                .flex()
                .items_center()
                .gap(r(10.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_family(theme::MONO)
                                .text_size(r(theme::text::SM))
                                .child(id.clone()),
                        )
                        .child(
                            div()
                                .text_size(r(theme::text::XS))
                                .text_color(theme.text_muted)
                                .child(format!(
                                    "{} · {}{}",
                                    if entry.commit.is_empty() {
                                        "built outside Agentique".to_string()
                                    } else {
                                        format!(
                                            "commit {}",
                                            &entry.commit[..entry.commit.len().min(10)]
                                        )
                                    },
                                    state,
                                    if current { " · running now" } else { "" }
                                )),
                        ),
                );
            if !current {
                row = row
                    .child(
                        Button::new(("build-try", index), "Try")
                            .small()
                            .tooltip("Start it as a test instance: its own data, on its own copy of the repository", None)
                            .on_click(act(Box::new(move |s: &mut Studio| s.try_build(&try_id)))),
                    )
                    .child(if asking {
                        let id = use_id.clone();
                        Button::new(("build-use-confirm", index), "Restart in it")
                            .small()
                            .primary()
                            .tooltip("Saves your session and project, backs up the app data, and restarts Agentique in this build through the launcher", None)
                            .on_click(act(Box::new(move |s: &mut Studio| {
                                if let Err(why) = s.use_build(&id) {
                                    s.develop.message = Some(why);
                                }
                                s.develop.confirm_use = None;
                                s.mark(Dirty::LAYOUT);
                            })))
                    } else {
                        let id = use_id.clone();
                        Button::new(("build-use", index), "Use this build…")
                            .small()
                            .disabled(blocker.is_some())
                            .tooltip(blocker.clone().unwrap_or_else(|| "Restart Agentique in this build".into()), None)
                            .on_click(act(Box::new(move |s: &mut Studio| {
                                s.develop.confirm_use = Some(id.clone());
                                s.mark(Dirty::LAYOUT);
                            })))
                    });
            }
            rows.push(row.into_any_element());
        }
        rows.push(
            div()
                .px(r(16.0))
                .py(r(10.0))
                .child(text(format!(
                    "If a build does not start, the launcher returns to the last known good one by itself. To start that one by hand, in safe mode (without the Claude Agent runtime): {} --recover. Its log: {}.",
                    root.join(agq_launcher::LAUNCHER).display(),
                    root.join("launcher.log").display()
                )))
                .into_any_element(),
        );
        group(Some("Builds"), rows, cx).into_any_element()
    }
}

/// A card of rows separated by hairlines.
fn group(title: Option<&'static str>, rows: Vec<AnyElement>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(r(8.0))
        .when_some(title, |this, title| {
            this.child(ui::section_header(title, cx))
        })
        .child(
            div()
                .rounded(r(crate::tokens::radius::CARD + 2.0))
                .bg(theme.raised)
                .border_1()
                .border_color(theme.border)
                .flex()
                .flex_col()
                .children(rows.into_iter().enumerate().map(|(index, row)| {
                    div()
                        .when(index > 0, |this| {
                            this.border_t_1().border_color(theme.separator)
                        })
                        .child(row)
                })),
        )
}

impl SettingsView {
    fn setting_row(
        &mut self,
        id: &'static str,
        query: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let setting = settings::setting(id)?;
        if !visible(setting, query) {
            return None;
        }
        let studio = self.studio.read(cx);
        let value = studio.settings.get(id);
        let changed = studio.settings.changed(id);
        let current = studio.settings.text(id);
        let automatic = studio.settings.text("assistant.provider").is_empty();
        let set_by = match id {
            "assistant.provider" => Some("AGENTIQUE_PROVIDER"),
            "assistant.model" => Some("AGENTIQUE_MODEL"),
            "assistant.effort" => Some("AGENTIQUE_EFFORT"),
            _ => None,
        }
        .filter(|variable| std::env::var_os(variable).is_some());
        let default = setting.default.value();
        let this = cx.entity();
        let control: AnyElement = match (id, setting.allowed) {
            ("assistant.provider", _) => {
                let open = self
                    .open_menu
                    .as_ref()
                    .filter(|(open, _, _)| *open == id)
                    .map(|(_, menu, _)| menu.clone());
                let choices: Vec<(String, String)> = std::iter::once((
                    String::new(),
                    "Automatic (first provider with a key)".to_string(),
                ))
                .chain(
                    [
                        Provider::Anthropic,
                        Provider::DeepSeek,
                        Provider::OpenAi,
                        Provider::OpenRouter,
                    ]
                    .into_iter()
                    .map(|p| (p.id().to_string(), p.name().to_string())),
                )
                .collect();
                let label = choices
                    .iter()
                    .find(|(v, _)| *v == current)
                    .map_or(current.clone(), |(_, l)| l.clone());
                let this = this.clone();
                let current_value = current.clone();
                div()
                    .child(
                        Button::new("provider-select", label)
                            .trailing(IconName::ChevronsUpDown)
                            .disabled(set_by.is_some())
                            .on_click(move |_: &ClickEvent, window, cx| {
                                let choices = choices.clone();
                                let current = current_value.clone();
                                this.update(cx, |view, cx| {
                                    view.open_choice(id, choices, current, window, cx)
                                })
                            }),
                    )
                    .when_some(open, |this, menu| {
                        this.child(
                            deferred(
                                anchored()
                                    .snap_to_window()
                                    .child(div().mt(r(32.0)).child(menu)),
                            )
                            .with_priority(3),
                        )
                    })
                    .into_any_element()
            }
            (_, Allowed::Toggle) => {
                let on = value.as_bool().unwrap_or(true);
                Switch::new(
                    SharedString::from(format!("switch-{id}")),
                    on,
                    setting.label,
                )
                .disabled(set_by.is_some())
                .on_toggle(move |on, _, cx| this.update(cx, |view, cx| view.set(id, json!(on), cx)))
                .into_any_element()
            }
            (_, Allowed::Choice(choices)) => {
                let index = choices
                    .iter()
                    .position(|choice| *choice == current)
                    .unwrap_or(0);
                let mut control =
                    Segmented::new(SharedString::from(format!("segments-{id}")), index);
                for choice in choices {
                    control = control.choice(None, choice_label(choice));
                }
                control
                    .on_choose(move |index, _, cx| {
                        let value = choices[index];
                        this.update(cx, |view, cx| view.set(id, json!(value), cx))
                    })
                    .into_any_element()
            }
            (_, Allowed::Number { min, max, step }) => {
                let mut scales = Vec::new();
                let mut scale = min;
                while scale <= max + 1e-9 {
                    scales.push(scale);
                    scale += step;
                }
                let current = value.as_f64().unwrap_or(min);
                let index = scales
                    .iter()
                    .position(|s| (s - current).abs() < 1e-6)
                    .unwrap_or(0);
                let mut control =
                    Segmented::new(SharedString::from(format!("segments-{id}")), index);
                for scale in &scales {
                    control = control.choice(None, format!("{:.0}%", scale * 100.0));
                }
                control
                    .on_choose(move |index, _, cx| {
                        let value = scales[index];
                        this.update(cx, |view, cx| view.set(id, json!(value), cx))
                    })
                    .into_any_element()
            }
            (_, Allowed::Text) => {
                let state = self.draft(id, window, cx);
                let disabled = set_by.is_some()
                    || (automatic && matches!(id, "assistant.model" | "assistant.effort"));
                div()
                    .w(r(240.0))
                    .when(disabled, |this| this.opacity(0.5))
                    .child(TextField::new(&state))
                    .into_any_element()
            }
        };
        let reset_to = default.clone();
        let reset: Option<Reset> = changed.then(|| {
            let this = cx.entity();
            let value = reset_to.clone();
            let on_reset: OnClick = Box::new(move |_, _, cx| {
                let value = value.clone();
                this.update(cx, |view, cx| view.set(id, value, cx))
            });
            (reset_to, on_reset)
        });
        Some(
            row(
                id,
                setting.label,
                setting.description,
                query,
                set_by,
                changed,
                reset,
                control,
                cx,
            )
            .into_any_element(),
        )
    }

    fn providers(
        &mut self,
        query: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut cards = Vec::new();
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
            cards.push(self.provider_card(provider, query, window, cx));
        }
        cards
    }

    fn provider_card(
        &mut self,
        provider: Provider,
        query: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let hint = self
            .studio
            .read(cx)
            .settings
            .text(&format!("providers.{}.keyHint", provider.id()));
        let theme = cx.theme().clone();
        let this = cx.entity();
        let state = self.provider(provider, window, cx);
        let (status, tone) = match &state.status {
            KeyStatus::FromEnvironment { variable } => (format!("From {variable}"), Tone::Info),
            KeyStatus::Stored if hint.is_empty() => ("Saved".to_string(), Tone::Success),
            KeyStatus::Stored => (format!("Saved · {hint}"), Tone::Success),
            KeyStatus::Missing => ("No key".to_string(), Tone::Neutral),
            KeyStatus::Unavailable(_) => ("Unavailable".to_string(), Tone::Warning),
        };
        let status_line = match &state.status {
            KeyStatus::FromEnvironment { variable } => Some(format!(
                "From environment variable {variable} (the saved key is ignored)."
            )),
            KeyStatus::Stored if hint.is_empty() => {
                Some("Saved in Windows Credential Manager.".to_string())
            }
            KeyStatus::Stored => Some(format!("Saved in Windows Credential Manager ({hint}).")),
            KeyStatus::Missing => None,
            KeyStatus::Unavailable(reason) => Some(reason.clone()),
        };
        let from_environment = matches!(state.status, KeyStatus::FromEnvironment { .. });
        let stored = matches!(state.status, KeyStatus::Stored);
        let busy = state.testing.is_some();
        let listing = state.listing.is_some();
        let input = state.input.clone();
        let has_input = !input.read(cx).value().trim().is_empty();
        let result = state.check.as_ref().map(|check| describe(provider, check));
        let message = state.message.clone();
        let confirm_remove = state.confirm_remove;
        let models = state.models.clone();
        let note = match provider {
            Provider::Anthropic => Some(
                "The Claude Agent runtime uses the key saved here. Agentique's own loop reads Anthropic's key from ANTHROPIC_API_KEY until Anthropic moves onto the provider layer (W5.7).",
            ),
            Provider::TypeSafe => Some(
                "Jev answers typed questions for fast agents; it is never the Assistant's model.",
            ),
            _ => None,
        };
        let test = |save: bool| {
            let this = this.clone();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                this.update(cx, |view, cx| view.test(provider, save, window, cx))
            }
        };
        div()
            .id(SharedString::from(format!("provider-{}", provider.id())))
            .rounded(r(crate::tokens::radius::CARD + 2.0))
            .bg(theme.raised)
            .border_1()
            .border_color(theme.border)
            .p(r(16.0))
            .flex()
            .flex_col()
            .gap(r(10.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(10.0))
                    .child(
                        div()
                            .size(r(28.0))
                            .rounded(r(7.0))
                            .bg(theme.hover)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(r(theme::text::SM))
                            .font_weight(theme::SEMIBOLD)
                            .child(provider.name().chars().next().unwrap_or('?').to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().font_weight(theme::SEMIBOLD).child(highlighted(
                                provider.name(),
                                query,
                                cx,
                            )))
                            .when_some(status_line, |this, line| {
                                this.child(
                                    div()
                                        .text_size(r(theme::text::SM))
                                        .text_color(theme.text_muted)
                                        .child(line),
                                )
                            }),
                    )
                    .child(Chip::new(status).tone(tone)),
            )
            .when_some(note, |this, note| {
                this.child(
                    div()
                        .text_size(r(theme::text::XS))
                        .line_height(r(16.0))
                        .text_color(theme.text_muted)
                        .child(note),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .when(from_environment, |this| this.opacity(0.5))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(TextField::new(&input).leading(IconName::Key)),
                    )
                    .child(
                        Button::new(
                            SharedString::from(format!("test-{}", provider.id())),
                            "Test",
                        )
                        .disabled(!has_input || busy || from_environment)
                        .on_click(test(false)),
                    )
                    .child(
                        Button::new(
                            SharedString::from(format!("save-{}", provider.id())),
                            "Save",
                        )
                        .primary()
                        .disabled(!has_input || busy || from_environment)
                        .on_click(test(true)),
                    )
                    .when(busy, |this| {
                        this.child(ui::spinner(
                            SharedString::from(format!("testing-{}", provider.id())),
                            14.0,
                            theme.text_muted,
                        ))
                    }),
            )
            .when_some(result, |this, (text, works)| {
                let store = cx.entity();
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(8.0))
                        .child(ui::inline_message(
                            if works { Tone::Success } else { Tone::Warning },
                            text,
                            cx,
                        ))
                        .when(!works && has_input, |this| {
                            this.child(
                                Button::new(
                                    SharedString::from(format!("save-anyway-{}", provider.id())),
                                    "Save anyway (offline)",
                                )
                                .small()
                                .ghost()
                                .tooltip("Save the key without a successful test", None)
                                .on_click(
                                    move |_: &ClickEvent, window, cx| {
                                        store.update(cx, |view, cx| {
                                            let key = view
                                                .providers
                                                .get(&provider)
                                                .map(|s| s.tested.clone())
                                                .unwrap_or_default();
                                            view.store(provider, key, window, cx)
                                        })
                                    },
                                ),
                            )
                        }),
                )
            })
            .when_some(message, |this, message| {
                this.child(ui::inline_message(Tone::Neutral, message, cx))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .child({
                        let view = cx.entity();
                        Button::new(
                            SharedString::from(format!("models-{}", provider.id())),
                            if models.is_none() {
                                "Show models"
                            } else {
                                "Refresh models"
                            },
                        )
                        .small()
                        .icon(IconName::Refresh)
                        .disabled(listing)
                        .on_click(move |_: &ClickEvent, window, cx| {
                            view.update(cx, |view, cx| view.list_models(provider, None, window, cx))
                        })
                    })
                    .when(listing, |this| {
                        this.child(ui::spinner(
                            SharedString::from(format!("listing-{}", provider.id())),
                            14.0,
                            theme.text_muted,
                        ))
                    })
                    .child(div().flex_1().min_w_0())
                    .when(stored && !confirm_remove, |this| {
                        let view = cx.entity();
                        this.child(
                            Button::new(
                                SharedString::from(format!("remove-{}", provider.id())),
                                "Remove key…",
                            )
                            .small()
                            .ghost()
                            .on_click(move |_: &ClickEvent, _, cx| {
                                view.update(cx, |view, cx| {
                                    if let Some(state) = view.providers.get_mut(&provider) {
                                        state.confirm_remove = true;
                                    }
                                    cx.notify();
                                })
                            }),
                        )
                    }),
            )
            // Removing a saved key: a simple confirmation with Cancel
            // focused (§3.7 Danger zone).
            .when(confirm_remove, |this| {
                let cancel = cx.entity();
                let remove = cx.entity();
                this.child(
                    div()
                        .p(r(10.0))
                        .rounded(r(crate::tokens::radius::CARD))
                        .border_1()
                        .border_color(theme.danger.border)
                        .flex()
                        .items_center()
                        .gap(r(8.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(r(theme::text::SM))
                                .child(format!("Remove the saved {} key?", provider.name())),
                        )
                        .child(
                            Button::new(
                                SharedString::from(format!("cancel-remove-{}", provider.id())),
                                "Cancel",
                            )
                            .small()
                            .on_click(move |_: &ClickEvent, _, cx| {
                                cancel.update(cx, |view, cx| {
                                    if let Some(state) = view.providers.get_mut(&provider) {
                                        state.confirm_remove = false;
                                    }
                                    cx.notify();
                                })
                            }),
                        )
                        .child(
                            Button::new(
                                SharedString::from(format!("confirm-remove-{}", provider.id())),
                                "Remove key",
                            )
                            .small()
                            .danger()
                            .on_click(
                                move |_: &ClickEvent, window, cx| {
                                    remove.update(cx, |view, cx| view.remove(provider, window, cx))
                                },
                            ),
                        ),
                )
            })
            .when_some(models, |this, models| {
                this.child(match models {
                    Ok(models) => div()
                        .flex()
                        .flex_col()
                        .children(models.iter().map(|model| model_row(model, cx)))
                        .into_any_element(),
                    Err(error) => ui::inline_message(Tone::Warning, error, cx).into_any_element(),
                })
            })
            .into_any_element()
    }
}

/// A model: its id, capability badges and list price, marked an estimate (E2).
fn model_row(model: &ModelInfo, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let mut badges: Vec<(String, Tone)> = Vec::new();
    if !model.capabilities.tools {
        badges.push(("no tools: not for the Assistant".into(), Tone::Warning));
    }
    if !model.efforts.is_empty() {
        badges.push((
            format!("effort {}", model.efforts.join(", ")),
            Tone::Neutral,
        ));
    }
    if matches!(
        model.capabilities.prompt_cache,
        agq_providers::PromptCache::None
    ) && model.capabilities.tools
    {
        badges.push(("no prompt caching".into(), Tone::Neutral));
    }
    if let Some(window) = model.context_window {
        badges.push((format!("{}k context", window / 1000), Tone::Neutral));
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
    div()
        .py(r(6.0))
        .border_t_1()
        .border_color(theme.separator)
        .flex()
        .flex_col()
        .gap(r(4.0))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(r(6.0))
                .child(
                    div()
                        .font_family(theme::MONO)
                        .text_size(r(theme::text::SM))
                        .text_color(if model.capabilities.tools {
                            theme.text
                        } else {
                            theme.text_muted
                        })
                        .child(model.model.model.clone()),
                )
                .children(
                    badges
                        .into_iter()
                        .map(|(badge, tone)| Chip::new(badge).tone(tone)),
                ),
        )
        .child(
            div()
                .text_size(r(theme::text::XS))
                .text_color(theme.text_faint)
                .child(price),
        )
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let query_text = self.search.read(cx).value().trim().to_lowercase();
        let query = (!query_text.is_empty()).then_some(query_text.as_str());
        let studio = self.studio.read(cx);
        let section = studio.settings_section;
        let save_error = studio.settings.save_error.clone();
        let problems = studio.settings.settings.problems.clone();
        let recent = studio.session.recent.clone();
        let path = studio.settings.path().to_path_buf();
        let studio_entity = self.studio.clone();
        let mut content: Vec<AnyElement> = Vec::new();
        let sections: Vec<Section> = if query.is_some() {
            vec![
                Section::Providers,
                Section::Assistant,
                Section::Appearance,
                Section::Keyboard,
            ]
        } else {
            vec![section]
        };
        for section in sections {
            let block: Option<AnyElement> = match section {
                Section::Providers => {
                    let cards = self.providers(query, window, cx);
                    (!cards.is_empty()).then(|| div().flex().flex_col().gap(r(12.0)).children(cards).into_any_element())
                }
                Section::Assistant => {
                    let rows: Vec<AnyElement> = ["assistant.runtime", "assistant.provider", "assistant.model", "assistant.effort", "assistant.showCost"]
                        .into_iter()
                        .filter_map(|id| self.setting_row(id, query, window, cx))
                        .collect();
                    let card = self.runtime_card(query, cx);
                    (!rows.is_empty() || card.is_some()).then(|| {
                        div()
                            .flex()
                            .flex_col()
                            .gap(r(16.0))
                            .when(!rows.is_empty(), |this| this.child(group(None, rows, cx)))
                            .children(card)
                            .into_any_element()
                    })
                }
                Section::Appearance => {
                    let rows: Vec<AnyElement> = ["appearance.theme", "appearance.uiScale", "appearance.reducedMotion"]
                        .into_iter()
                        .filter_map(|id| self.setting_row(id, query, window, cx))
                        .collect();
                    (!rows.is_empty()).then(|| group(None, rows, cx).into_any_element())
                }
                Section::Keyboard => {
                    let rows: Vec<AnyElement> = COMMANDS
                        .iter()
                        .filter(|command| query.is_none_or(|query| matches(query, &[command.label, command.description, command.shortcut])))
                        .map(|command| {
                            div()
                                .px(r(16.0))
                                .py(r(8.0))
                                .flex()
                                .items_center()
                                .gap(r(12.0))
                                .when_some(crate::workspace::command_icon(command.id), |this, glyph| {
                                    this.child(icon(glyph).size(14.0).color(theme.text_muted))
                                })
                                .child(
                                    div()
                                        .flex_1().min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(div().text_size(r(theme::text::BASE)).child(highlighted(command.label, query, cx)))
                                        .child(div().text_size(r(theme::text::XS)).text_color(theme.text_muted).child(highlighted(command.description, query, cx))),
                                )
                                .when(!command.shortcut.is_empty(), |this| this.child(KeyCaps::new(command.shortcut)))
                                .into_any_element()
                        })
                        .collect();
                    (!rows.is_empty()).then(|| group(if query.is_some() { Some("Keyboard") } else { None }, rows, cx).into_any_element())
                }
                Section::Projects => {
                    let mut rows: Vec<AnyElement> = self.setting_row("projects.defaultFolder", None, window, cx).into_iter().collect();
                    let recent_rows: Vec<AnyElement> = if recent.is_empty() {
                        vec![div().px(r(16.0)).py(r(12.0)).text_color(theme.text_muted).child("None yet.").into_any_element()]
                    } else {
                        recent
                            .iter()
                            .enumerate()
                            .map(|(index, folder)| {
                                let studio = studio_entity.clone();
                                let remove = folder.clone();
                                div()
                                    .px(r(16.0))
                                    .py(r(8.0))
                                    .flex()
                                    .items_center()
                                    .gap(r(10.0))
                                    .child(icon(IconName::Folder).size(14.0).color(theme.text_muted))
                                    .child(div().flex_1().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().font_family(theme::MONO).text_size(r(theme::text::SM)).child(folder.display().to_string()))
                                    .child(Button::new(("remove-recent", index), "Remove from the list").small().ghost().on_click(move |_: &ClickEvent, _, cx| {
                                        let remove = remove.clone();
                                        studio.act(cx, |studio| {
                                            studio.session.recent.retain(|recent| *recent != remove);
                                            studio.save_session();
                                        })
                                    }))
                                    .into_any_element()
                            })
                            .collect()
                    };
                    let data = path.with_file_name("projects");
                    rows.push(
                        div()
                            .px(r(16.0))
                            .py(r(12.0))
                            .flex()
                            .items_center()
                            .gap(r(10.0))
                            .child(div().flex_1().min_w_0().text_size(r(theme::text::SM)).text_color(theme.text_muted).child(format!("Each project's conversation is kept in {}.", data.display())))
                            .child(Button::new("open-data", "Open folder").small().icon(IconName::External).on_click(move |_: &ClickEvent, _, _| open_in_explorer(&data)))
                            .into_any_element(),
                    );
                    Some(
                        div()
                            .flex()
                            .flex_col()
                            .gap(r(20.0))
                            .child(group(None, rows, cx))
                            .child(group(Some("Recent projects"), recent_rows, cx))
                            .into_any_element(),
                    )
                }
                Section::Advanced => {
                    let file = path.clone();
                    let confirm = self.confirm_reset;
                    let this = cx.entity();
                    let open = this.clone();
                    let cancel = this.clone();
                    Some(
                        div()
                            .flex()
                            .flex_col()
                            .gap(r(20.0))
                            .child(group(
                                None,
                                vec![div()
                                    .px(r(16.0))
                                    .py(r(12.0))
                                    .flex()
                                    .items_center()
                                    .gap(r(10.0))
                                    .child(
                                        div()
                                            .flex_1().min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(div().font_weight(theme::MEDIUM).child("Settings file"))
                                            .child(div().text_size(r(theme::text::SM)).font_family(theme::MONO).text_color(theme.text_muted).child(file.display().to_string())),
                                    )
                                    .child(Button::new("show-settings-file", "Show in Explorer").small().icon(IconName::External).on_click(move |_: &ClickEvent, _, _| open_in_explorer(&file)))
                                    .into_any_element()],
                                cx,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(r(8.0))
                                    .child(ui::section_header("Danger zone", cx))
                                    .child(
                                        div()
                                            .p(r(16.0))
                                            .rounded(r(crate::tokens::radius::CARD + 2.0))
                                            .border_1()
                                            .border_color(theme.danger.border)
                                            .flex()
                                            .flex_col()
                                            .gap(r(10.0))
                                            .child(div().font_weight(theme::SEMIBOLD).child("Reset all settings"))
                                            .child(div().text_size(r(theme::text::SM)).text_color(theme.text_secondary).child(
                                                "Every setting goes back to its default. Saved keys stay; remove them in Providers. A copy of the file is kept as settings.json.bak.",
                                            ))
                                            .child(if confirm {
                                                div()
                                                    .flex()
                                                    .gap(r(8.0))
                                                    .child(Button::new("cancel-reset", "Cancel").on_click(move |_: &ClickEvent, _, cx| {
                                                        cancel.update(cx, |view, cx| {
                                                            view.confirm_reset = false;
                                                            cx.notify();
                                                        })
                                                    }))
                                                    .child(Button::new("confirm-reset", "Reset all settings").danger().on_click(move |_: &ClickEvent, _, cx| {
                                                        this.update(cx, |view, cx| {
                                                            view.confirm_reset = false;
                                                            view.studio.act(cx, |studio| {
                                                                if studio.settings.reset_all() {
                                                                    studio.apply_appearance();
                                                                    studio.apply_runtime_choice();
                                                                }
                                                                studio.mark(Dirty::ALL);
                                                            });
                                                            cx.notify();
                                                        })
                                                    }))
                                                    .into_any_element()
                                            } else {
                                                div()
                                                    .child(Button::new("reset-all", "Reset all settings…").danger().on_click(move |_: &ClickEvent, _, cx| {
                                                        open.update(cx, |view, cx| {
                                                            view.confirm_reset = true;
                                                            cx.notify();
                                                        })
                                                    }))
                                                    .into_any_element()
                                            }),
                                    ),
                            )
                            .into_any_element(),
                    )
                }
                Section::About => Some(
                    div().flex().flex_col().gap(r(16.0)).child(group(
                        None,
                        vec![
                            div().px(r(16.0)).py(r(12.0)).flex().flex_col().gap(r(4.0))
                                .child(div().font_weight(theme::SEMIBOLD).child(format!("Agentique Studio {}", env!("CARGO_PKG_VERSION"))))
                                .child(div().text_size(r(theme::text::SM)).text_color(theme.text_muted).child("Drawn with GPUI (a pinned snapshot of Zed's GPUI and gpui-base)."))
                                .into_any_element(),
                            div().px(r(16.0)).py(r(12.0)).text_size(r(theme::text::SM)).text_color(theme.text_secondary)
                                .child("Licensed under Apache-2.0. Fonts: Inter and JetBrains Mono, under the SIL Open Font License 1.1. Icons: Lucide, under the ISC License.")
                                .into_any_element(),
                            div().px(r(16.0)).py(r(12.0)).text_size(r(theme::text::SM)).text_color(theme.text_muted)
                                .child("Keys are kept in the Windows Credential Manager, never in the settings file.")
                                .into_any_element(),
                        ],
                        cx,
                    ))
                    .child(self.builds_card(cx))
                    .into_any_element(),
                ),
            };
            if let Some(block) = block {
                if query.is_some() {
                    content.push(ui::section_header(section.title(), cx).into_any_element());
                }
                content.push(block);
            }
        }
        let empty = query.is_some() && content.is_empty();
        div()
            .id("settings")
            .key_context("Settings")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::close))
            .size_full()
            .flex()
            .bg(theme.canvas)
            .role(gpui::Role::Main)
            .aria_label("Settings")
            // The sections, with search on top.
            .child(
                div()
                    .w(r(248.0))
                    .h_full()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(r(2.0))
                    .p(r(12.0))
                    .bg(theme.chrome)
                    .border_r_1()
                    .border_color(theme.separator)
                    .child(div().pb(r(8.0)).child(TextField::new(&self.search).leading(IconName::Search).target("Search settings")))
                    .children(Section::ALL.into_iter().map(|item| {
                        let chosen = item == section && query.is_none();
                        let studio = self.studio.clone();
                        let search = self.search.clone();
                        div()
                            .id(SharedString::from(format!("section-{}", item.title())))
                            .h(r(30.0))
                            .px(r(10.0))
                            .flex()
                            .items_center()
                            .gap(r(10.0))
                            .rounded(r(crate::tokens::radius::CONTROL + 1.0))
                            .text_size(r(theme::text::BASE))
                            .role(gpui::Role::Tab)
                            .aria_selected(chosen)
                            .aria_label(item.title())
                            .cursor_pointer()
                            .when(chosen, |this| this.bg(theme.accent.soft).text_color(theme.text).font_weight(theme::MEDIUM))
                            .when(!chosen, |this| this.text_color(theme.text_secondary).hover(|style| style.bg(theme.hover)))
                            .on_click(move |_: &ClickEvent, window, cx| {
                                search.update(cx, |state, cx| state.set_value("", window, cx));
                                studio.act(cx, |studio| {
                                    studio.settings_section = item;
                                    studio.mark(Dirty::OVERLAY);
                                })
                            })
                            .relative()
                            .child(icon(section_icon(item)).size(15.0).color(if chosen { theme.accent.text } else { theme.text_muted }))
                            .child(item.title())
                            .child(ui::target::target(item.title()))
                    }))
                    .child(div().flex_1().min_w_0())
                    .child(
                        Button::new("close-settings", "Close")
                            .ghost()
                            .full_width()
                            .shortcut("Esc")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close(&Close, window, cx))),
                    ),
            )
            // The rows.
            .child(
                div()
                    .id("settings-content")
                    .flex_1().min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .w_full()
                            .max_w(r(COLUMN))
                            .px(r(28.0))
                            .pt(r(28.0))
                            .pb(r(48.0))
                            .flex()
                            .flex_col()
                            .gap(r(20.0))
                            .when(query.is_none(), |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(r(4.0))
                                        .child(div().text_size(r(theme::text::XL)).font_weight(theme::SEMIBOLD).child(section.title()))
                                        .child(div().text_size(r(theme::text::SM)).text_color(theme.text_muted).child(match section {
                                            Section::Providers => "Keys for the model providers, kept in the Windows Credential Manager; each is tested before it is saved.",
                                            Section::Assistant => "Which model the Assistant uses, and what it shows.",
                                            Section::Appearance => "Theme, scale and motion. Choices apply at once.",
                                            Section::Keyboard => "Every command and its shortcut.",
                                            Section::Projects => "Where new projects go, and the projects you opened.",
                                            Section::Advanced => "The settings file, and resetting everything.",
                                            Section::About => "Version and licences.",
                                        })),
                                )
                            })
                            .children(problems.into_iter().map(|problem| ui::Banner::new(Tone::Warning, problem).into_any_element()))
                            .when_some(save_error, |this, error| this.child(ui::Banner::new(Tone::Danger, error)))
                            .when(empty, |this| {
                                this.child(ui::EmptyState::new(IconName::Search, "No setting matches", "Search matches labels, descriptions and the words people use for them."))
                            })
                            .children(content),
                    ),
            )
    }
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

    #[test]
    fn search_words_are_marked_in_labels_and_descriptions() {
        assert_eq!(
            marked("Theme of the Studio", Some("the")),
            vec![0..3, 9..12]
        );
        assert!(marked("Theme", None).is_empty());
    }
}
