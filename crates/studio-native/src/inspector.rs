//! The Inspector Panel: the selected element's properties, edited through
//! System State operations, and its problems in plain language.
use crate::{
    app::StudioApp,
    targets::{Target, record},
};
use agq_language::{
    Direction, ElementId, ElementKind, Literal, Multiplicity, Reference, Tree, library,
};
use agq_studio_scene::SceneTarget;
use agq_system_state::Property;
use eframe::egui::{self, Key, RichText};

/// A committed edit from one of the Inspector's fields.
enum FieldEdit {
    Name(String),
    Property(Property, &'static str),
    Lock,
}

/// The definitions a usage of `kind` can be typed by.
fn type_kinds(kind: ElementKind) -> &'static [ElementKind] {
    use ElementKind::*;
    match kind {
        Part => &[PartDef, ItemDef],
        Port => &[PortDef],
        Item => &[ItemDef],
        Attribute => &[AttributeDef],
        Connection => &[ConnectionDef, InterfaceDef],
        Interface => &[InterfaceDef],
        Requirement => &[RequirementDef],
        Subject => &[PartDef, ItemDef, PortDef],
        PartDef => &[PartDef],
        PortDef => &[PortDef],
        ItemDef => &[ItemDef],
        AttributeDef => &[AttributeDef],
        RequirementDef => &[RequirementDef],
        InterfaceDef => &[InterfaceDef],
        ConnectionDef => &[ConnectionDef],
        _ => &[],
    }
}

/// Definitions to choose from: the project's, then the built-in library's.
/// Every definition a type can be chosen from: the project's, then the
/// built-in library's. Built once per model version.
pub fn type_options(tree: &Tree) -> Vec<(ElementKind, String, Reference)> {
    let mut out = Vec::new();
    for id in tree.walk() {
        if tree[id].kind.is_definition()
            && let Some(name) = tree.effective_name(id)
        {
            out.push((tree[id].kind, name.to_string(), Reference::to(id, name)));
        }
    }
    let library = library();
    for id in library.walk() {
        if library[id].kind.is_definition()
            && let Some(name) = library.effective_name(id)
        {
            let qualified = library.qualified_name(id);
            out.push((
                library[id].kind,
                name.to_string(),
                Reference::new(&qualified),
            ));
        }
    }
    out
}

/// A name that wraps within the panel, on at most three lines.
pub fn inspector_name(ui: &mut egui::Ui, name: &str) -> egui::Response {
    let mut title = egui::text::LayoutJob::simple_singleline(
        name.to_owned(),
        crate::theme::semibold(crate::theme::TITLE),
        ui.visuals().strong_text_color(),
    );
    title.wrap.max_width = ui.available_width();
    title.wrap.max_rows = 3;
    title.wrap.break_anywhere = false;
    title.wrap.overflow_character = Some('…');
    ui.add(egui::Label::new(title).wrap()).on_hover_text(name)
}

pub fn parse_multiplicity(text: &str) -> Result<Option<Multiplicity>, String> {
    let text = text
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    if text.is_empty() {
        return Ok(None);
    }
    let bound = |part: &str| -> Result<Option<u64>, String> {
        match part.trim() {
            "*" => Ok(None),
            number => number
                .parse()
                .map(Some)
                .map_err(|_| format!("“{number}” is not a number or *")),
        }
    };
    let (lower, upper) = match text.split_once("..") {
        Some((lower, upper)) => (bound(lower)?.unwrap_or(0), bound(upper)?),
        None => match bound(text)? {
            None => (0, None),
            Some(n) => (n, Some(n)),
        },
    };
    if upper.is_some_and(|upper| upper < lower) {
        return Err("the upper bound is below the lower bound".into());
    }
    Ok(Some(Multiplicity { lower, upper }))
}

pub fn parse_value(text: &str) -> Option<Literal> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    Some(match text {
        "true" => Literal::Boolean(true),
        "false" => Literal::Boolean(false),
        _ if text.parse::<i64>().is_ok() => Literal::Integer(text.into()),
        _ if text.parse::<f64>().is_ok() && text.contains('.') => Literal::Real(text.into()),
        _ => Literal::String(
            text.trim_start_matches('"')
                .trim_end_matches('"')
                .replace('"', "'"),
        ),
    })
}

/// A single-line field showing `current`; returns the new text when the
/// Operator presses Enter or leaves the field after changing it.
fn text_field(
    ui: &mut egui::Ui,
    key: (&str, ElementId),
    current: &str,
    enabled: bool,
) -> Option<String> {
    let id = ui.make_persistent_id(key);
    let mut buffer = ui
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| current.to_string());
    let response = ui.add_enabled(
        enabled,
        egui::TextEdit::singleline(&mut buffer)
            .margin(crate::theme::INPUT_MARGIN)
            .desired_width(f32::INFINITY),
    );
    let escaped = ui.input(|i| i.key_pressed(Key::Escape));
    let committed =
        (response.lost_focus() && !escaped && buffer != current).then(|| buffer.clone());
    if !response.has_focus() {
        buffer = current.to_string();
    }
    ui.data_mut(|d| d.insert_temp(id, buffer));
    committed
}

impl StudioApp {
    pub fn inspector_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let Some(project) = &self.project else {
            self.fixture_inspector(ui);
            return;
        };
        let state = project.state();
        let tree = state.tree();
        let Some(element) = self.inspected_element().filter(|id| tree.contains(*id)) else {
            ui.label(crate::app::muted(
                "Select an element on the Surface to see and change its properties.",
                theme,
            ));
            self.problems_list(ui);
            return;
        };
        let e = &tree[element];
        // Nothing is edited while a dialog waits for an answer.
        let editable = self.editable() && self.dialog.is_none();
        let mut edits = Vec::new();
        let mut member = None;
        ui.label(
            RichText::new(e.kind.keyword().to_uppercase())
                .font(crate::theme::semibold(crate::theme::CAPTION))
                .color(theme.accent),
        );
        if let Some(owner) = e.owner() {
            ui.label(crate::app::muted(
                format!("in {}", crate::edit::display_path(tree, owner)),
                theme,
            ));
        }
        inspector_name(ui, &crate::edit::display_name(tree, element));
        if let Some(definition) = self.shared_definition() {
            ui.label(
                RichText::new(format!(
                    "Defined in {}: changing it changes {} and every part typed by it",
                    crate::edit::display_name(tree, definition),
                    crate::edit::display_name(tree, definition),
                ))
                .color(theme.violet),
            );
        }
        match state.lock_of(element) {
            Some(lock) if lock == element => {
                ui.label(RichText::new("Locked: changes ask for confirmation").color(theme.amber));
            }
            Some(lock) => {
                ui.label(
                    RichText::new(format!("Locked with {}", tree.qualified_name(lock)))
                        .color(theme.amber),
                );
            }
            None => {}
        }
        theme.section(ui, "NAME");
        if e.kind.is_namespace()
            && let Some(name) = text_field(
                ui,
                ("Name", element),
                tree.effective_name(element).unwrap_or(""),
                editable,
            )
        {
            edits.push(FieldEdit::Name(name));
        }
        let kinds = type_kinds(e.kind);
        if !kinds.is_empty() {
            let usage = e.kind.is_usage();
            theme.section(ui, if usage { "TYPE" } else { "SPECIALIZES" });
            let references = if usage { &e.typed_by } else { &e.specializes };
            let current = references
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            if self
                .type_options
                .as_ref()
                .is_none_or(|(generation, _)| *generation != self.generation)
            {
                self.type_options = Some((self.generation, type_options(tree)));
            }
            let options: Vec<(String, Reference)> = self
                .type_options
                .as_ref()
                .map(|(_, all)| {
                    all.iter()
                        .filter(|(kind, _, _)| kinds.contains(kind))
                        .map(|(_, name, reference)| (name.clone(), reference.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let (edit, message) = type_picker(ui, element, &current, &options, editable, usage);
            edits.extend(edit);
            if let Some(message) = message {
                self.status = message;
            }
        }
        if e.kind.is_usage() && e.kind != ElementKind::Subject {
            theme.section(ui, "MULTIPLICITY");
            let current = e.multiplicity.map(|m| m.to_string()).unwrap_or_default();
            if let Some(text) = text_field(ui, ("Multiplicity", element), &current, editable) {
                match parse_multiplicity(&text) {
                    Ok(value) => edits.push(FieldEdit::Property(
                        Property::Multiplicity(value),
                        "multiplicity",
                    )),
                    Err(reason) => self.status = format!("Multiplicity: {reason}"),
                }
            }
        }
        if matches!(
            e.kind,
            ElementKind::Port | ElementKind::Item | ElementKind::Attribute
        ) {
            theme.section(ui, "DIRECTION");
            let mut direction = e.direction;
            ui.add_enabled_ui(editable, |ui| {
                ui.horizontal(|ui| {
                    for (value, label) in [
                        (None, "none"),
                        (Some(Direction::In), "in"),
                        (Some(Direction::Out), "out"),
                        (Some(Direction::InOut), "inout"),
                    ] {
                        ui.selectable_value(&mut direction, value, label);
                    }
                });
            });
            if direction != e.direction {
                edits.push(FieldEdit::Property(
                    Property::Direction(direction),
                    "direction",
                ));
            }
        }
        if matches!(e.kind, ElementKind::Attribute | ElementKind::Reference) {
            theme.section(ui, "VALUE");
            let current = e
                .value
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            if let Some(text) = text_field(ui, ("Value", element), &current, editable) {
                edits.push(FieldEdit::Property(
                    Property::Value(parse_value(&text)),
                    "value",
                ));
            }
        }
        if e.kind.is_namespace() {
            theme.section(ui, "DOCUMENTATION");
            let current = e
                .children()
                .iter()
                .find(|c| tree[**c].kind == ElementKind::Doc)
                .and_then(|c| tree[*c].text.clone())
                .unwrap_or_default();
            let id = ui.make_persistent_id(("Doc", element));
            let mut buffer = ui
                .data(|d| d.get_temp::<String>(id))
                .unwrap_or_else(|| current.clone());
            let response = ui.add_enabled(
                editable,
                egui::TextEdit::multiline(&mut buffer)
                    .margin(crate::theme::INPUT_MARGIN)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            if response.lost_focus() && buffer.trim() != current.trim() {
                let text = buffer.trim();
                edits.push(FieldEdit::Property(
                    Property::Doc((!text.is_empty()).then(|| text.to_string())),
                    "documentation",
                ));
            }
            if !response.has_focus() {
                buffer = current;
            }
            ui.data_mut(|d| d.insert_temp(id, buffer));
        }
        theme.section(ui, "LOCK");
        let locked_here = state.locks().contains(&element);
        let lock = ui.add_enabled(
            editable,
            egui::Button::new(if locked_here { "Unlock" } else { "Lock" }),
        );
        record(ui.ctx(), Target::Button("Lock"), lock.rect);
        if lock.clicked() {
            edits.push(FieldEdit::Lock);
        }
        // Problems at the element and at what it owns without a card.
        let mut problems: Vec<String> = Vec::new();
        for id in tree.descendants(element) {
            if let Some(messages) = self.problems.get(&id)
                && (id == element || self.lookup.node(&self.scene, id).is_none())
            {
                problems.extend(messages.iter().cloned());
            }
        }
        theme.section(ui, "PROBLEMS");
        if problems.is_empty() {
            ui.label(RichText::new("No problems").color(theme.green));
        }
        for problem in problems {
            ui.label(RichText::new(format!("• {problem}")).color(theme.amber));
        }
        let members: Vec<(ElementId, String)> = e
            .children()
            .iter()
            .filter(|c| tree[**c].kind.is_namespace() || tree[**c].kind == ElementKind::Satisfy)
            .map(|c| {
                (
                    *c,
                    format!(
                        "{} {}",
                        tree[*c].kind.keyword(),
                        tree.effective_name(*c)
                            .map_or_else(|| tree.qualified_name(*c), str::to_string)
                    ),
                )
            })
            .collect();
        if !members.is_empty() {
            theme.section(ui, "OWNS");
            for (id, label) in members {
                if ui.add(egui::Button::new(label).frame(false)).clicked() {
                    member = Some(if tree[id].kind == ElementKind::Port {
                        SceneTarget::Port(element, id)
                    } else {
                        SceneTarget::Node(id)
                    });
                }
            }
        }
        if let Some(target) = member {
            if self.scene.target_bounds(&target).is_some() {
                self.select(target.clone(), false);
                self.frame_target(&target);
            } else if let Some(id) = target.element_id() {
                // No card of its own (an attribute or item line): inspect it here.
                self.inspected = Some((self.selection.primary.clone(), id));
            }
        }
        for edit in edits {
            match edit {
                FieldEdit::Name(name) => self.rename(element, &name),
                FieldEdit::Property(property, what) => self.set_property(element, property, what),
                FieldEdit::Lock => {
                    let operation = if locked_here {
                        agq_system_state::Operation::Unlock { element }
                    } else {
                        agq_system_state::Operation::Lock { element }
                    };
                    let name = self
                        .project
                        .as_ref()
                        .and_then(|p| p.state().tree().effective_name(element).map(str::to_string))
                        .unwrap_or_default();
                    let verb = if locked_here { "Unlock" } else { "Lock" };
                    self.operation(&format!("{verb} {name}"), operation);
                }
            }
        }
    }

    /// Every problem in the model, in plain language; click to select.
    pub fn problems_list(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let Some(project) = &self.project else { return };
        let state = project.state();
        let tree = state.tree();
        theme.section(ui, &format!("PROBLEMS · {}", state.diagnostics().len()));
        if state.diagnostics().is_empty() {
            ui.label(RichText::new("No problems").color(theme.green));
            return;
        }
        let mut chosen = None;
        for diagnostic in state.diagnostics().iter().take(50) {
            let label = format!(
                "{}\n{}",
                tree.get(diagnostic.element).map_or_else(String::new, |_| {
                    crate::edit::display_path(tree, diagnostic.element)
                }),
                diagnostic.message
            );
            if ui
                .add(egui::Button::new(RichText::new(label).color(theme.amber)).frame(false))
                .clicked()
            {
                chosen = Some(diagnostic.element);
            }
        }
        if let Some(mut id) = chosen {
            // Select the element's card, or the nearest card that owns it.
            loop {
                let target = SceneTarget::Node(id);
                if self.scene.target_bounds(&target).is_some() {
                    self.select(target.clone(), false);
                    self.frame_target(&target);
                    break;
                }
                match self
                    .project
                    .as_ref()
                    .and_then(|p| p.state().tree().get(id))
                    .and_then(|e| e.owner())
                {
                    Some(owner) => id = owner,
                    None => break,
                }
            }
        }
    }

    fn fixture_inspector(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        ui.label(crate::app::muted(
            "An example opened read-only. Create or open a project to edit.",
            theme,
        ));
        if let Some(SceneTarget::Node(id) | SceneTarget::Container(id)) = &self.selection.primary
            && let Some(node) = self.lookup.node(&self.scene, *id)
        {
            theme.section(ui, &node.semantic.keyword.to_uppercase());
            ui.label(RichText::new(&node.semantic.name).size(16.0).strong());
            if !node.semantic.detail.is_empty() {
                ui.label(crate::app::muted(&node.semantic.detail, theme));
            }
            for port in &node.semantic.ports {
                ui.label(format!("port {}", port.name));
            }
        }
    }
}

/// A searchable type field: type a name, pick a match, Enter takes the
/// best match; an empty field clears the type.
fn type_picker(
    ui: &mut egui::Ui,
    element: ElementId,
    current: &str,
    options: &[(String, Reference)],
    editable: bool,
    usage: bool,
) -> (Option<FieldEdit>, Option<String>) {
    let id = ui.make_persistent_id(("Type", element));
    let mut buffer = ui
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| current.to_string());
    let response = ui.add_enabled(
        editable,
        egui::TextEdit::singleline(&mut buffer)
            .margin(crate::theme::INPUT_MARGIN)
            .hint_text("Type to find a definition")
            .desired_width(f32::INFINITY),
    );
    record(ui.ctx(), Target::Field("Type"), response.rect);
    let property = |references: Vec<Reference>| {
        if usage {
            Property::TypedBy(references)
        } else {
            Property::Specializes(references)
        }
    };
    let what = if usage { "type" } else { "specialisation" };
    let mut matches: Vec<_> = options
        .iter()
        .filter_map(|(name, reference)| {
            crate::commands::fuzzy_score(&buffer, name).map(|score| (score, name, reference))
        })
        .collect();
    matches.sort_by_key(|(score, name, _)| (*score, name.len()));
    let mut chosen = None;
    let mut message = None;
    if response.has_focus() || ui.data(|d| d.get_temp::<bool>(id.with("open")).unwrap_or(false)) {
        for (_, name, reference) in matches.iter().take(8) {
            if ui
                .add(egui::Button::new(format!("   {name}")).frame(false))
                .clicked()
            {
                chosen = Some(property(vec![(*reference).clone()]));
            }
        }
    }
    let focused = response.has_focus();
    ui.data_mut(|d| d.insert_temp(id.with("open"), focused));
    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    if enter && buffer.trim() != current {
        chosen = if buffer.trim().is_empty() {
            Some(property(Vec::new()))
        } else {
            match matches.first() {
                Some((_, _, reference)) => Some(property(vec![(*reference).clone()])),
                None => {
                    message = Some(format!("No definition matches “{}”", buffer.trim()));
                    None
                }
            }
        };
    }
    if !response.has_focus() {
        buffer = current.to_string();
    }
    ui.data_mut(|d| d.insert_temp(id, buffer));
    (
        chosen.map(|property| FieldEdit::Property(property, what)),
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplicities_and_values_parse_as_written() {
        assert_eq!(
            parse_multiplicity("1").unwrap(),
            Some(Multiplicity {
                lower: 1,
                upper: Some(1)
            })
        );
        assert_eq!(
            parse_multiplicity("[0..*]").unwrap(),
            Some(Multiplicity {
                lower: 0,
                upper: None
            })
        );
        assert_eq!(parse_multiplicity("").unwrap(), None);
        assert!(parse_multiplicity("5..2").is_err());
        assert!(parse_multiplicity("many").is_err());
        assert_eq!(parse_value("42"), Some(Literal::Integer("42".into())));
        assert_eq!(parse_value("true"), Some(Literal::Boolean(true)));
        assert_eq!(parse_value("\"a\""), Some(Literal::String("a".into())));
        assert_eq!(parse_value(""), None);
    }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn very_long_names_stay_within_the_panel_and_three_lines() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let name = format!(
            "Architecture::{}::NestedPart",
            "VeryLongNamespaceWithoutWordBreaks".repeat(30)
        );
        for width in [214.0, 254.0, 380.0] {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::Panel::right("inspector-name")
                    .exact_size(width)
                    .resizable(false)
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        let response = inspector_name(ui, &name);
                        assert!(response.rect.width() <= width + 1.0);
                        assert!(response.rect.height() <= crate::theme::TITLE * 4.0);
                    });
            })
            .textures_delta
            .clear();
        }
    }
}
