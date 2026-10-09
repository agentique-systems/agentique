//! The Inspector (§3.2 Panels: label and value rows, inline validation): the
//! selected element's kind, place, name and properties, edited through
//! System State operations; its lock, its problems in plain language, and
//! what it owns. A field commits on Enter or when the Operator leaves it;
//! while the Operator types, the model's changes do not overwrite the field.
use super::InspectorColumn;
use crate::{
    palette::kind_icon,
    studio::{Dirty, Studio},
    ui::{
        self, ActiveTheme, Button, Chip, IconName, Segmented, TextArea, TextField, Tone, icon, r,
        theme,
    },
    workspace::StudioExt,
};
use agq_language::{
    Direction, ElementId, ElementKind, Literal, Multiplicity, Reference, Tree, library,
};
use agq_studio_scene::SceneTarget;
use agq_system_state::Property;
use gpui::{
    AppContext, ClickEvent, Context, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div,
    prelude::FluentBuilder,
};
use gpui_base::input::{InputEvent, InputState, TextareaState};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Name,
    Type,
    Multiplicity,
    /// A literal or an expression (C-50): a feature's value, what a step
    /// sends, a check's condition, a wait's time.
    Value,
    /// A transition's condition (C-50).
    Guard,
    Doc,
}

/// What the Value row is called for an element, if it has one.
fn value_label(e: &agq_language::Element) -> Option<&'static str> {
    Some(match e.kind {
        // The part a referential part refers to (C-55).
        ElementKind::Part | ElementKind::Item if e.referential => "Refers to",
        ElementKind::Attribute | ElementKind::Reference => "Value",
        ElementKind::Send => "Sends",
        ElementKind::AssertConstraint => "Check",
        ElementKind::Accept if e.after => "After (ms)",
        _ => return None,
    })
}

/// Whether a feature's value may be a literal (else only an expression).
fn holds_literal(kind: ElementKind) -> bool {
    matches!(kind, ElementKind::Attribute | ElementKind::Reference)
}

/// The Inspector's fields, kept by the Panels' column.
pub struct Fields {
    studio: Entity<Studio>,
    name: Entity<InputState>,
    type_query: Entity<InputState>,
    multiplicity: Entity<InputState>,
    value: Entity<InputState>,
    guard: Entity<InputState>,
    doc: Entity<TextareaState>,
    /// The element and model revision the fields show.
    loaded: Option<(ElementId, u64)>,
    /// Why the multiplicity typed could not be used (inline validation).
    multiplicity_error: Option<String>,
    /// Why the value or guard typed could not be used.
    value_error: Option<String>,
    guard_error: Option<String>,
    type_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl Field {
    /// The field's control id, for the ones agents can operate.
    fn target(self) -> Option<&'static str> {
        match self {
            Field::Name => Some("Name"),
            Field::Type => Some("Type"),
            Field::Multiplicity => Some("Multiplicity"),
            Field::Value => Some("Value"),
            Field::Guard => Some("Guard"),
            Field::Doc => Some("Docs"),
        }
    }
}

/// The model's current text for a field.
fn current(tree: &Tree, element: ElementId, field: Field) -> String {
    let e = &tree[element];
    match field {
        Field::Name => tree.effective_name(element).unwrap_or("").to_string(),
        Field::Type => {
            let references = if e.kind.is_usage() {
                &e.typed_by
            } else {
                &e.specializes
            };
            references
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        }
        Field::Multiplicity => e.multiplicity.map(|m| m.to_string()).unwrap_or_default(),
        Field::Value => match (&e.value, &e.expression) {
            (Some(literal), _) => literal.to_string(),
            (None, Some(expression)) => agq_language::print_expression(tree, element, expression),
            _ => String::new(),
        },
        Field::Guard => e
            .guard
            .as_ref()
            .map(|g| agq_language::print_expression(tree, element, g))
            .unwrap_or_default(),
        Field::Doc => e
            .children()
            .iter()
            .find(|c| tree[**c].kind == ElementKind::Doc)
            .and_then(|c| tree[*c].text.clone())
            .unwrap_or_default(),
    }
}

impl Fields {
    pub fn new(
        studio: Entity<Studio>,
        window: &mut Window,
        cx: &mut Context<InspectorColumn>,
    ) -> Fields {
        let input =
            |placeholder: &'static str, window: &mut Window, cx: &mut Context<InspectorColumn>| {
                cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
            };
        let name = input("Name", window, cx);
        let type_query = input("Type to find a definition", window, cx);
        let multiplicity = input("1, 0..1, 0..*", window, cx);
        let value = input("A number, true, false, text or an expression", window, cx);
        let guard = input("A condition, such as attempts < 3", window, cx);
        let doc = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx).placeholder("What this element is for");
            state.set_auto_grow(2, 8, cx);
            state
        });
        let mut subscriptions = Vec::new();
        for (state, field) in [
            (&name, Field::Name),
            (&type_query, Field::Type),
            (&multiplicity, Field::Multiplicity),
            (&value, Field::Value),
            (&guard, Field::Guard),
        ] {
            subscriptions.push(cx.subscribe_in(
                state,
                window,
                move |column, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } | InputEvent::Blur => {
                        // What an agent typed here commits as its change,
                        // whoever ends the edit (C-53).
                        let studio = column.inspector.studio.clone();
                        let began = studio.update(cx, |studio, _| {
                            studio.control.begin_typed_commit(field.target())
                        });
                        column.inspector.commit(field, window, cx);
                        studio.update(cx, |studio, _| studio.control.end_typed_commit(began));
                    }
                    InputEvent::Change if field == Field::Type => cx.notify(),
                    _ => {}
                },
            ));
        }
        subscriptions.push(cx.subscribe_in(
            &doc,
            window,
            |column, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Blur) {
                    // What an agent typed here commits as its change (C-53).
                    let studio = column.inspector.studio.clone();
                    let began = studio.update(cx, |studio, _| {
                        studio.control.begin_typed_commit(Field::Doc.target())
                    });
                    column.inspector.commit(Field::Doc, window, cx);
                    studio.update(cx, |studio, _| studio.control.end_typed_commit(began));
                }
            },
        ));
        Fields {
            studio,
            name,
            type_query,
            multiplicity,
            value,
            guard,
            doc,
            loaded: None,
            multiplicity_error: None,
            value_error: None,
            guard_error: None,
            type_error: None,
            _subscriptions: subscriptions,
        }
    }

    fn text(&self, field: Field, cx: &gpui::App) -> String {
        match field {
            Field::Name => self.name.read(cx).value().to_string(),
            Field::Type => self.type_query.read(cx).value().to_string(),
            Field::Multiplicity => self.multiplicity.read(cx).value().to_string(),
            Field::Value => self.value.read(cx).value().to_string(),
            Field::Guard => self.guard.read(cx).value().to_string(),
            Field::Doc => self.doc.read(cx).value().to_string(),
        }
    }

    fn focused(&self, field: Field, window: &Window, cx: &gpui::App) -> bool {
        match field {
            Field::Name => self.name.read(cx).focus_handle(cx).is_focused(window),
            Field::Type => self.type_query.read(cx).focus_handle(cx).is_focused(window),
            Field::Multiplicity => self
                .multiplicity
                .read(cx)
                .focus_handle(cx)
                .is_focused(window),
            Field::Value => self.value.read(cx).focus_handle(cx).is_focused(window),
            Field::Guard => self.guard.read(cx).focus_handle(cx).is_focused(window),
            Field::Doc => self.doc.read(cx).focus_handle(cx).is_focused(window),
        }
    }

    fn set(&self, field: Field, text: String, window: &mut Window, cx: &mut gpui::App) {
        match field {
            Field::Name => self.name.update(cx, |s, cx| s.set_value(text, window, cx)),
            Field::Type => self
                .type_query
                .update(cx, |s, cx| s.set_value(text, window, cx)),
            Field::Multiplicity => self
                .multiplicity
                .update(cx, |s, cx| s.set_value(text, window, cx)),
            Field::Value => self.value.update(cx, |s, cx| s.set_value(text, window, cx)),
            Field::Guard => self.guard.update(cx, |s, cx| s.set_value(text, window, cx)),
            Field::Doc => self.doc.update(cx, |s, cx| s.set_value(text, window, cx)),
        }
    }

    /// Shows the model's values: all of them for another element, and the
    /// fields not being typed in after a change of the model.
    fn sync(&mut self, element: ElementId, window: &mut Window, cx: &mut Context<InspectorColumn>) {
        let studio = self.studio.read(cx);
        let Some(project) = &studio.project else {
            return;
        };
        let revision = project.state().revision();
        let tree = project.state().tree();
        if self.loaded == Some((element, revision)) || !tree.contains(element) {
            return;
        }
        let other = self.loaded.is_none_or(|(loaded, _)| loaded != element);
        let values: Vec<(Field, String)> = [
            Field::Name,
            Field::Type,
            Field::Multiplicity,
            Field::Value,
            Field::Guard,
            Field::Doc,
        ]
        .into_iter()
        .map(|field| (field, current(tree, element, field)))
        .collect();
        for (field, text) in values {
            if other || !self.focused(field, window, cx) {
                self.set(field, text, window, cx);
                // What an agent typed there is gone with it (C-53).
                if let Some(target) = field.target() {
                    self.studio
                        .update(cx, |studio, _| studio.control.forget_typed(target));
                }
            }
        }
        if other {
            self.multiplicity_error = None;
            self.type_error = None;
            self.value_error = None;
            self.guard_error = None;
        }
        self.loaded = Some((element, revision));
    }

    /// Applies what the Operator typed in a field, if it changed.
    fn commit(&mut self, field: Field, window: &mut Window, cx: &mut Context<InspectorColumn>) {
        let Some((element, _)) = self.loaded else {
            return;
        };
        let text = self.text(field, cx);
        let studio = self.studio.read(cx);
        let Some(project) = &studio.project else {
            return;
        };
        let tree = project.state().tree();
        if !tree.contains(element) || !studio.editable() || studio.dialog.is_some() {
            return;
        }
        let before = current(tree, element, field);
        if text.trim() == before.trim() {
            if field == Field::Type {
                self.type_error = None;
            }
            return;
        }
        let usage = tree[element].kind.is_usage();
        match field {
            Field::Name => {
                self.studio.act(cx, |studio| studio.rename(element, &text));
            }
            Field::Multiplicity => match parse_multiplicity(&text) {
                Ok(value) => {
                    self.multiplicity_error = None;
                    self.studio.act(cx, |studio| {
                        studio.set_property(element, Property::Multiplicity(value), "multiplicity")
                    });
                }
                Err(reason) => {
                    self.multiplicity_error = Some(format!("Not applied: {reason}"));
                    cx.notify();
                }
            },
            Field::Value => {
                let e = &tree[element];
                let kind = e.kind;
                let had_expression = e.expression.is_some();
                let had_literal = e.value.is_some();
                let text = text.trim().to_string();
                // A number, true, false or quoted text is a value; anything
                // else is an expression (a name, `new T(...)`, `a + 1`).
                let literal = parse_value(&text).filter(|l| {
                    holds_literal(kind)
                        && (!matches!(l, Literal::String(_)) || text.starts_with('"'))
                });
                let mut properties = Vec::new();
                if text.is_empty() {
                    if had_literal {
                        properties.push(Property::Value(None));
                    }
                    if had_expression {
                        properties.push(Property::Expression(None));
                    }
                } else if literal.is_some()
                    || (holds_literal(kind)
                        && !had_expression
                        && agq_language::parse_expression(&text).is_err())
                {
                    properties.push(Property::Value(literal.or_else(|| parse_value(&text))));
                    if had_expression {
                        properties.push(Property::Expression(None));
                    }
                } else {
                    match agq_language::parse_expression(&text) {
                        Ok(expression) => {
                            properties.push(Property::Expression(Some(expression)));
                            if had_literal {
                                properties.push(Property::Value(None));
                            }
                        }
                        Err(reason) => {
                            self.value_error = Some(format!("Not applied: {reason}"));
                            cx.notify();
                            return;
                        }
                    }
                }
                self.value_error = None;
                let name = tree
                    .effective_name(element)
                    .unwrap_or("element")
                    .to_string();
                let operations = properties
                    .into_iter()
                    .map(|property| agq_system_state::Operation::Set { element, property })
                    .collect();
                self.studio.act(cx, |studio| {
                    studio.submit(agq_system_state::Change::new(
                        agq_system_state::Actor::Operator,
                        &format!("Set the value of {name}"),
                        operations,
                    ));
                });
            }
            Field::Guard => {
                let text = text.trim().to_string();
                let parsed = if text.is_empty() {
                    Ok(None)
                } else {
                    agq_language::parse_expression(&text).map(Some)
                };
                match parsed {
                    Ok(guard) => {
                        self.guard_error = None;
                        self.studio.act(cx, |studio| {
                            studio.set_property(element, Property::Guard(guard), "guard")
                        });
                    }
                    Err(reason) => {
                        self.guard_error = Some(format!("Not applied: {reason}"));
                        cx.notify();
                    }
                }
            }
            Field::Doc => {
                let text = text.trim();
                self.studio.act(cx, |studio| {
                    studio.set_property(
                        element,
                        Property::Doc((!text.is_empty()).then(|| text.to_string())),
                        "documentation",
                    )
                });
            }
            Field::Type => {
                let chosen = if text.trim().is_empty() {
                    Some(Vec::new())
                } else {
                    self.matches(element, &text, cx)
                        .first()
                        .map(|(_, reference)| vec![reference.clone()])
                };
                match chosen {
                    Some(references) => {
                        self.type_error = None;
                        self.choose_type(element, references, usage, cx);
                    }
                    None => {
                        self.type_error = Some(format!("No definition matches “{}”", text.trim()));
                        cx.notify();
                    }
                }
            }
        }
        let _ = window;
    }

    fn choose_type(
        &mut self,
        element: ElementId,
        references: Vec<Reference>,
        usage: bool,
        cx: &mut Context<InspectorColumn>,
    ) {
        let (property, what) = if usage {
            (Property::TypedBy(references), "type")
        } else {
            (Property::Specializes(references), "specialisation")
        };
        self.studio
            .act(cx, |studio| studio.set_property(element, property, what));
    }

    /// Definitions matching the type field, best first.
    fn matches(
        &self,
        element: ElementId,
        query: &str,
        cx: &mut Context<InspectorColumn>,
    ) -> Vec<(String, Reference)> {
        self.studio.update(cx, |studio, _| {
            let Some(project) = &studio.project else {
                return Vec::new();
            };
            let tree = project.state().tree();
            let kinds = type_kinds(tree[element].kind);
            if studio
                .type_options
                .as_ref()
                .is_none_or(|(generation, _)| *generation != studio.generation)
            {
                studio.type_options = Some((studio.generation, type_options(tree)));
            }
            let mut matches: Vec<_> = studio
                .type_options
                .as_ref()
                .map(|(_, all)| {
                    all.iter()
                        .filter(|(kind, _, _)| kinds.contains(kind))
                        .filter_map(|(_, name, reference)| {
                            crate::commands::fuzzy_score(query, name)
                                .map(|score| (score, name.clone(), reference.clone()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            matches.sort_by_key(|(score, name, _)| (*score, name.len()));
            matches
                .into_iter()
                .map(|(_, name, reference)| (name, reference))
                .collect()
        })
    }

    pub fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<InspectorColumn>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let Some(project) = &studio.project else {
            return fixture_inspector(&self.studio, cx).into_any_element();
        };
        let tree = project.state().tree();
        let Some(element) = studio.inspected_element().filter(|id| tree.contains(*id)) else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p(r(20.0))
                .child(
                    ui::EmptyState::new(
                        IconName::Sliders,
                        "Nothing selected",
                        "Select an element on the Surface or in the Outline to see and change its properties.",
                    )
                    .hint("↑ ↓ ← →", "move between cards")
                    .hint("Ctrl+P", "go to an element"),
                )
                .into_any_element();
        };
        self.sync(element, window, cx);
        let can_edit = {
            let studio = self.studio.read(cx);
            studio.editable() && studio.dialog.is_none()
        };
        let about = super::responsibility::section(&self.studio, element, cx);
        let reuse = super::reuse::section(&self.studio, element, can_edit, cx);
        let factory = super::evidence::sections(&self.studio, element, can_edit, cx);
        let studio = self.studio.read(cx);
        let project = studio.project.as_ref().expect("checked above");
        let state = project.state();
        let tree = state.tree();
        let e = &tree[element];
        let kind = e.kind;
        let editable = studio.editable() && studio.dialog.is_none();
        let owner = e
            .owner()
            .map(|owner| crate::edit::display_path(tree, owner));
        let name = crate::edit::display_name(tree, element);
        let shared = studio
            .shared_definition()
            .map(|d| crate::edit::display_name(tree, d));
        let lock = state
            .lock_of(element)
            .map(|lock| (lock == element, tree.qualified_name(lock)));
        let locked_here = state.locks().contains(&element);
        let kinds = type_kinds(kind);
        let usage = kind.is_usage();
        let has_multiplicity = usage && kind != ElementKind::Subject;
        let has_direction = matches!(
            kind,
            ElementKind::Port | ElementKind::Item | ElementKind::Attribute
        );
        let direction = e.direction;
        let value_row = value_label(e);
        // A part or item contains what it holds or refers to it (C-55), by
        // the language's rule: written `ref`, and so is every part it
        // redefines. Directed, `end` and package-level usages always refer.
        let has_usage = matches!(kind, ElementKind::Part | ElementKind::Item);
        let written = e.referential;
        // A usage owned by a package has no featuring type: it is a part of
        // the model, with nothing to choose here.
        let package_level = e
            .owner()
            .is_none_or(|o| tree[o].kind == ElementKind::Package);
        let always = has_usage && !package_level && (e.direction.is_some() || e.is_end);
        let (effective, bound, inherited) = if has_usage {
            let semantics = agq_language::Semantics::new(tree);
            let holder = semantics.value_holder(element);
            let inherited = holder.filter(|h| *h != element).and_then(|h| {
                let value = tree.get(h)?.expression.as_ref()?;
                Some(agq_language::print_expression(tree, h, value))
            });
            (
                semantics.referential(element) == Some(true),
                holder.is_some(),
                inherited,
            )
        } else {
            (false, false, None)
        };
        let composite_after_all = written && !effective;
        let not_bound = written && effective && !bound;
        let chip = match (written && effective, kind) {
            (true, ElementKind::Part) => "ref part",
            (true, ElementKind::Item) => "ref item",
            _ => kind.keyword(),
        };
        let has_guard = kind == ElementKind::Transition;
        let via = e.via.as_ref().map(ToString::to_string);
        let namespace = kind.is_namespace();
        // Problems at the element and at what it owns without a card.
        let mut problems: Vec<String> = Vec::new();
        for id in tree.descendants(element) {
            if let Some(messages) = studio.problems.get(&id)
                && (id == element || studio.lookup.node(&studio.scene, id).is_none())
            {
                problems.extend(messages.iter().cloned());
            }
        }
        let members: Vec<(ElementId, ElementKind, bool, String)> = e
            .children()
            .iter()
            .filter(|c| tree[**c].kind.is_namespace() || tree[**c].kind == ElementKind::Satisfy)
            .map(|c| {
                (
                    *c,
                    tree[*c].kind,
                    tree[*c].referential,
                    tree.effective_name(*c)
                        .map_or_else(|| crate::edit::display_name(tree, *c), str::to_string),
                )
            })
            .collect();
        let type_focused = self.focused(Field::Type, window, cx);
        let type_matches = if type_focused && !kinds.is_empty() {
            let query = self.text(Field::Type, cx);
            self.matches(element, &query, cx)
                .into_iter()
                .take(8)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let studio_entity = self.studio.clone();
        let column = cx.entity();
        div()
            .id("inspector")
            .size_full()
            .overflow_y_scroll()
            .px(r(14.0))
            .pb(r(20.0))
            .text_size(r(theme::text::BASE))
            .flex()
            .flex_col()
            // What it is and where.
            .child(
                div()
                    .pt(r(14.0))
                    .flex()
                    .flex_col()
                    .gap(r(6.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(r(6.0))
                            .child(Chip::new(chip).icon(kind_icon(kind)).tone(match kind {
                                ElementKind::Requirement | ElementKind::RequirementDef => Tone::Warning,
                                k if k.is_definition() => Tone::Info,
                                _ => Tone::Accent,
                            }))
                            .when_some(lock.clone(), |this, (own, _)| {
                                this.child(Chip::new(if own { "Locked" } else { "Locked with owner" }).icon(IconName::Lock).tone(Tone::Warning))
                            }),
                    )
                    .child(
                        div()
                            .text_size(r(theme::text::LG))
                            .line_height(r(22.0))
                            .font_weight(theme::SEMIBOLD)
                            .line_clamp(3)
                            .child(name),
                    )
                    .when_some(owner, |this, owner| {
                        this.child(
                            div()
                                .text_size(r(theme::text::SM))
                                .text_color(theme.text_muted)
                                .font_family(theme::MONO)
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(format!("in {owner}")),
                        )
                    })
                    .when_some(shared, |this, definition| {
                        this.child(ui::Banner::new(
                            Tone::Info,
                            format!("Defined in {definition}: changing it changes {definition} and every part typed by it."),
                        ))
                    })
                    .when_some(lock.filter(|(own, _)| !own), |this, (_, with)| {
                        this.child(ui::inline_message(Tone::Warning, format!("Locked with {with}; changes ask first"), cx))
                    }),
            )
            .children(about)
            .child(super::group("Properties", None, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(r(8.0))
                    .when(namespace, |this| this.child(row("Name", TextField::new(&self.name).target("Name"), cx)))
                    .when(!kinds.is_empty(), |this| {
                        let column = column.clone();
                        this.child(row(
                            if usage { "Type" } else { "Specializes" },
                            div()
                                .flex()
                                .flex_col()
                                .gap(r(4.0))
                                .child(TextField::new(&self.type_query).invalid(self.type_error.is_some()).target("Type"))
                                .when_some(self.type_error.clone(), |this, error| {
                                    this.child(ui::inline_message(Tone::Danger, error, cx))
                                })
                                .when(!type_matches.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .p(r(3.0))
                                            .rounded(r(crate::tokens::radius::CONTROL + 2.0))
                                            .bg(theme.overlay)
                                            .border_1()
                                            .border_color(theme.border)
                                            .shadow(theme.shadow_small())
                                            .children(type_matches.into_iter().enumerate().map(|(index, (name, reference))| {
                                                let column = column.clone();
                                                div()
                                                    .id(("type-match", index))
                                                    .relative()
                                                    .child(ui::target::control(
                                                        ui::target::Control::new("option", name.clone())
                                                            .id(format!("type-match-{index}"))
                                                            .selected(index == 0),
                                                    ))
                                                    .h(r(26.0))
                                                    .px(r(8.0))
                                                    .flex()
                                                    .items_center()
                                                    .rounded(r(crate::tokens::radius::CONTROL))
                                                    .font_family(theme::MONO)
                                                    .text_size(r(theme::text::SM))
                                                    .cursor_pointer()
                                                    .when(index == 0, |this| this.bg(theme.hover))
                                                    .hover(|style| style.bg(theme.hover))
                                                    .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                                                        let reference = reference.clone();
                                                        column.update(cx, |column, cx| {
                                                            column.inspector.type_error = None;
                                                            column.inspector.choose_type(element, vec![reference], usage, cx);
                                                            column.inspector.loaded = None;
                                                        });
                                                        window.blur(cx);
                                                    })
                                                    .child(name)
                                            })),
                                    )
                                }),
                            cx,
                        ))
                    })
                    .when(has_multiplicity, |this| {
                        this.child(row(
                            "Multiplicity",
                            div()
                                .flex()
                                .flex_col()
                                .gap(r(4.0))
                                .child(TextField::new(&self.multiplicity).mono().invalid(self.multiplicity_error.is_some()).target("Multiplicity"))
                                .when_some(self.multiplicity_error.clone(), |this, error| {
                                    this.child(ui::inline_message(Tone::Danger, error, cx))
                                }),
                            cx,
                        ))
                    })
                    .when(has_usage && always, |this| {
                        this.child(row(
                            "Usage",
                            div().pt(r(6.0)).child(super::note("Referential: directed and `end` usages always are.", cx)),
                            cx,
                        ))
                    })
                    .when(has_usage && !always && !package_level, |this| {
                        let studio = studio_entity.clone();
                        this.child(row(
                            "Usage",
                            div()
                                .flex()
                                .flex_col()
                                .gap(r(4.0))
                                .child(
                                    Segmented::new("usage", usize::from(written))
                                        .choice(None, "composite")
                                        .choice(None, "ref")
                                        .on_choose(move |index, _, cx| {
                                            if !editable {
                                                return;
                                            }
                                            studio.act(cx, |studio| {
                                                studio.set_property(element, Property::Referential(index == 1), "usage")
                                            });
                                        }),
                                )
                                .when(composite_after_all, |this| {
                                    this.child(super::note("Composite all the same: it redefines a composite part, which it shares values with. Declare that one `ref` where it is declared.", cx))
                                }),
                            cx,
                        ))
                    })
                    .when(has_direction, |this| {
                        let studio = studio_entity.clone();
                        let index = match direction {
                            None => 0,
                            Some(Direction::In) => 1,
                            Some(Direction::Out) => 2,
                            Some(Direction::InOut) => 3,
                        };
                        this.child(row(
                            "Direction",
                            Segmented::new("direction", index)
                                .choice(None, "none")
                                .choice(None, "in")
                                .choice(None, "out")
                                .choice(None, "inout")
                                .on_choose(move |index, _, cx| {
                                    if !editable {
                                        return;
                                    }
                                    let value = [None, Some(Direction::In), Some(Direction::Out), Some(Direction::InOut)][index];
                                    studio.act(cx, |studio| {
                                        studio.set_property(element, Property::Direction(value), "direction")
                                    });
                                }),
                            cx,
                        ))
                    })
                    .when_some(value_row, |this, label| {
                        this.child(row(
                            label,
                            div()
                                .flex()
                                .flex_col()
                                .gap(r(4.0))
                                .child(TextField::new(&self.value).mono().invalid(self.value_error.is_some()).target("Value"))
                                .when(not_bound, |this| {
                                    this.child(super::note("Not bound: the part it refers to is not identified in this model. Name it here, or give it a stand-in in a scenario.", cx))
                                })
                                .when_some(inherited.clone(), |this, value| {
                                    this.child(super::note(format!("Bound where it is declared: = {value}"), cx))
                                })
                                .when_some(self.value_error.clone(), |this, error| {
                                    this.child(ui::inline_message(Tone::Danger, error, cx))
                                }),
                            cx,
                        ))
                    })
                    .when(has_guard, |this| {
                        this.child(row(
                            "Guard",
                            div()
                                .flex()
                                .flex_col()
                                .gap(r(4.0))
                                .child(TextField::new(&self.guard).mono().invalid(self.guard_error.is_some()).target("Guard"))
                                .when_some(self.guard_error.clone(), |this, error| {
                                    this.child(ui::inline_message(Tone::Danger, error, cx))
                                }),
                            cx,
                        ))
                    })
                    .when_some(via, |this, via| {
                        this.child(row(
                            "Via",
                            div()
                                .pt(r(6.0))
                                .font_family(theme::MONO)
                                .text_size(r(theme::text::SM))
                                .text_color(theme.text_secondary)
                                .child(via),
                            cx,
                        ))
                    })
                    .when(namespace, |this| {
                        this.child(row("Docs", TextArea::new(&self.doc).target("Docs"), cx))
                    }),
            )
            .child(
                div().pt(r(10.0)).flex().gap(r(6.0)).child(
                    Button::new("inspector-lock", if locked_here { "Unlock" } else { "Lock" })
                        .icon(if locked_here { IconName::Unlock } else { IconName::Lock })
                        .small()
                        .disabled(!editable)
                        .tooltip("A locked element changes only after you confirm", Some("L"))
                        .on_click({
                            let studio = studio_entity.clone();
                            move |_: &ClickEvent, _, cx| {
                                studio.act(cx, |studio| {
                                    let name = studio
                                        .project
                                        .as_ref()
                                        .and_then(|p| p.state().tree().effective_name(element).map(str::to_string))
                                        .unwrap_or_default();
                                    let operation = if locked_here {
                                        agq_system_state::Operation::Unlock { element }
                                    } else {
                                        agq_system_state::Operation::Lock { element }
                                    };
                                    let verb = if locked_here { "Unlock" } else { "Lock" };
                                    studio.operation(&format!("{verb} {name}"), operation);
                                })
                            }
                        }),
                ),
            )
            .children(reuse)
            .children(factory)
            .child(super::group("Problems", Some(problems.len()), cx))
            .child(if problems.is_empty() {
                div()
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .text_size(r(theme::text::SM))
                    .text_color(theme.success.text)
                    .child(icon(IconName::CircleCheck).size(13.0).color(theme.success.text))
                    .child("No problems")
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap(r(6.0))
                    .children(problems.into_iter().map(|problem| {
                        div()
                            .flex()
                            .items_start()
                            .gap(r(8.0))
                            .text_size(r(theme::text::SM))
                            .line_height(r(18.0))
                            .text_color(theme.text_secondary)
                            .child(div().pt(r(2.0)).child(icon(IconName::Warning).size(13.0).color(theme.warning.text)))
                            .child(div().flex_1().min_w_0().child(problem))
                    }))
                    .into_any_element()
            })
            .when(!members.is_empty(), |this| {
                this.child(super::group("Owns", Some(members.len()), cx)).child(
                    div().flex().flex_col().children(members.into_iter().enumerate().map(|(index, (id, kind, referential, label))| {
                        let studio = studio_entity.clone();
                        div()
                            .id(("owns", index))
                            .h(r(26.0))
                            .px(r(6.0))
                            .mx(r(-6.0))
                            .flex()
                            .items_center()
                            .gap(r(8.0))
                            .rounded(r(crate::tokens::radius::CONTROL))
                            .text_size(r(theme::text::SM))
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .on_click(move |_: &ClickEvent, _, cx| {
                                studio.act(cx, |studio| {
                                    let target = if kind == ElementKind::Port {
                                        SceneTarget::Port(element, id)
                                    } else {
                                        SceneTarget::Node(id)
                                    };
                                    if studio.scene.target_bounds(&target).is_some() {
                                        studio.select(target.clone(), false);
                                        studio.frame_target(&target);
                                    } else {
                                        // No card of its own (an attribute or item line).
                                        studio.inspected = Some((studio.selection.primary.clone(), id));
                                    }
                                    studio.mark(Dirty::SELECTION | Dirty::CAMERA);
                                })
                            })
                            .child(icon(kind_icon(kind)).size(13.0).color(theme.text_muted))
                            .child(div().text_color(theme.text_muted).child(if referential { format!("ref {}", kind.keyword()) } else { kind.keyword().to_string() }))
                            .child(div().flex_1().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().font_family(theme::MONO).text_color(theme.text_secondary).child(label))
                    })),
                )
            })
            .into_any_element()
    }
}

/// A label and its control (§3.2 Inspector row).
fn row(label: &'static str, control: impl IntoElement, cx: &gpui::App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_start()
        .gap(r(10.0))
        .child(
            div()
                .w(r(84.0))
                .flex_none()
                .pt(r(6.0))
                .text_size(r(theme::text::SM))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(div().flex_1().min_w_0().child(control))
}

/// A read-only example: what the selected card shows.
fn fixture_inspector(studio: &Entity<Studio>, cx: &gpui::App) -> impl IntoElement {
    let theme = cx.theme();
    let studio = studio.read(cx);
    let node = match &studio.selection.primary {
        Some(SceneTarget::Node(id) | SceneTarget::Container(id)) => {
            studio.lookup.node(&studio.scene, *id)
        }
        _ => None,
    };
    div()
        .p(r(14.0))
        .flex()
        .flex_col()
        .gap(r(8.0))
        .child(ui::Banner::new(
            Tone::Neutral,
            "An example, opened read-only. Create or open a project to edit.",
        ))
        .when_some(node, |this, node| {
            this.child(Chip::new(node.semantic.keyword))
                .child(
                    div()
                        .text_size(r(theme::text::LG))
                        .font_weight(theme::SEMIBOLD)
                        .child(SharedString::from(node.semantic.name.clone())),
                )
                .when(!node.semantic.detail.is_empty(), |this| {
                    this.child(
                        div()
                            .font_family(theme::MONO)
                            .text_size(r(theme::text::SM))
                            .text_color(theme.text_muted)
                            .child(SharedString::from(node.semantic.detail.clone())),
                    )
                })
                .children(node.semantic.ports.iter().map(|port| {
                    div()
                        .flex()
                        .items_center()
                        .gap(r(6.0))
                        .text_size(r(theme::text::SM))
                        .child(icon(IconName::Port).size(13.0).color(theme.text_muted))
                        .child(SharedString::from(port.name.clone()))
                }))
        })
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
