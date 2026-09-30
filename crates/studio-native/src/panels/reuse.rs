//! The Inspector's view of reuse (C-49, Scenario H5): for a usage, the
//! definition it is typed by, where that came from, and the values it
//! inherits, each overridable here only; for a definition, where it came
//! from, its usages and specialisations, and what it inherits. It answers
//! "am I changing this one thing, or the definition used everywhere?".
use crate::{
    edit::Dialog,
    library::definition_of,
    palette::kind_icon,
    studio::{Dirty, Studio},
    ui::{self, ActiveTheme, Button, Chip, IconName, Tone, icon, r, theme},
    workspace::StudioExt,
};
use agq_language::{ElementId, ElementKind, Semantics, Tree};
use gpui::{
    AnyElement, App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
};

/// An attribute found through a type or a general, and whether it is
/// overridden here.
struct Inherited {
    /// The path from the owner: `[ttlSeconds]` or `[cache, ttlSeconds]`.
    path: Vec<ElementId>,
    label: String,
    value: String,
    /// The redefinition that overrides it here, if any.
    overridden: Option<ElementId>,
    /// The definition it comes from.
    from: String,
}

/// At most this many inherited values are listed.
const MOST: usize = 14;

/// The inherited attributes of `owner` (a usage or a definition), with
/// those of its inner parts, found by the language's own lookup.
fn inherited(tree: &Tree, owner: ElementId) -> Vec<Inherited> {
    let semantics = Semantics::new(tree);
    let inside = |id: ElementId| {
        let mut current = Some(id);
        while let Some(here) = current {
            if here == owner {
                return true;
            }
            current = tree.get(here).and_then(|e| e.owner());
        }
        false
    };
    let name = |id: ElementId| tree.effective_name(id).unwrap_or("").to_string();
    let origin = |feature: ElementId| {
        tree.get(feature)
            .and_then(|e| e.owner())
            .map(name)
            .unwrap_or_default()
    };
    let value_of = |feature: ElementId| {
        tree.get(feature)
            .and_then(|e| e.value.as_ref())
            .map(|v| format!("= {v}"))
            .unwrap_or_else(|| {
                let types: Vec<String> = semantics
                    .types_of(feature)
                    .iter()
                    .map(|(t, _)| {
                        tree.effective_name(*t)
                            .map(str::to_string)
                            .or_else(|| {
                                agq_language::library()
                                    .effective_name(*t)
                                    .map(str::to_string)
                            })
                            .unwrap_or_default()
                    })
                    .collect();
                format!(": {}", types.join(", "))
            })
    };
    let mut out = Vec::new();
    let visit = |path: Vec<ElementId>, feature: ElementId, out: &mut Vec<Inherited>| {
        let Some(element) = tree.get(feature) else {
            return;
        };
        let here = inside(feature);
        // Own attributes of a definition are changed directly, not listed.
        if here && element.redefines.is_empty() {
            return;
        }
        let labels: Vec<String> = path.iter().map(|p| name(*p)).collect();
        out.push(Inherited {
            label: labels.join("."),
            value: value_of(feature),
            overridden: here.then_some(feature),
            from: origin(feature),
            path,
        });
    };
    for feature in semantics.features(owner) {
        let Some(element) = tree.get(feature) else {
            continue;
        };
        match element.kind {
            ElementKind::Attribute => visit(vec![feature], feature, &mut out),
            ElementKind::Part => {
                for inner in semantics.features(feature) {
                    if tree.get(inner).map(|e| e.kind) == Some(ElementKind::Attribute) {
                        visit(vec![feature, inner], inner, &mut out);
                    }
                }
            }
            _ => {}
        }
    }
    out.truncate(MOST);
    out
}

/// The reuse section for the inspected element, or nothing when it neither
/// is a definition nor has one.
pub fn section(
    studio: &Entity<Studio>,
    element: ElementId,
    editable: bool,
    cx: &mut App,
) -> Option<AnyElement> {
    studio.update(cx, |studio, _| {
        studio.library_index();
    });
    let theme = cx.theme().clone();
    let state = studio.read(cx);
    let tree = state.project.as_ref()?.state().tree();
    let e = tree.get(element)?;
    let definition = definition_of(tree, element)?;
    let is_usage = e.kind.is_usage();
    let index = &state.library.index;
    let block = index.project_block(definition).and_then(|i| index.get(i));
    let source = block
        .map(|b| b.source_label())
        .unwrap_or_else(|| "Project".into());
    let usages = state.usages_of(definition);
    let (usage_count, special_count) = usages.iter().fold((0, 0), |(u, s), id| {
        if tree.get(*id).is_some_and(|e| e.kind.is_definition()) {
            (u, s + 1)
        } else {
            (u + 1, s)
        }
    });
    let definition_name = tree.effective_name(definition).unwrap_or("").to_string();
    let copied = block.and_then(|b| b.origin);
    let list = inherited(tree, element);
    let owner_name = tree.effective_name(element).unwrap_or("").to_string();
    let used = uses(usage_count, special_count);
    let them = if usage_count + special_count == 1 {
        "it"
    } else {
        "all of them"
    };
    let note = match (is_usage, copied) {
        (true, _) if usage_count > 1 => format!(
            "Typed by {definition_name}, which {used} share. Changing {definition_name} changes all of them; override a value here to change only {owner_name}, or specialise it for a variant."
        ),
        (true, _) => format!(
            "Typed by {definition_name}. Changing {definition_name} changes every usage of it; override a value here to change only {owner_name}, or specialise it for a variant."
        ),
        (false, Some(origin)) if !origin.changed => format!(
            "A copy of the {} block{}. Changing it here changes this project only; the Library stays as it is.",
            origin.scope.label(),
            if used.is_empty() {
                String::new()
            } else {
                format!(", used by {used}")
            }
        ),
        (false, Some(origin)) => format!(
            "Changed since it was copied from the {}.",
            origin.scope.label()
        ),
        (false, None) if used.is_empty() => "Not used yet.".to_string(),
        (false, None) => format!("Used by {used}. A change here changes {them}."),
    };
    let open = studio.clone();
    let find = studio.clone();
    let special = studio.clone();
    let save = studio.clone();
    Some(
        div()
            .flex()
            .flex_col()
            .gap(r(6.0))
            .child(super::group(
                if is_usage { "Definition" } else { "Reuse" },
                None,
                cx,
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .child(
                        icon(kind_icon(tree[definition].kind))
                            .size(13.0)
                            .color(theme.info.text),
                    )
                    .child(
                        div()
                            .id("reuse-definition")
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .font_family(theme::MONO)
                            .text_size(r(theme::text::SM))
                            .font_weight(theme::MEDIUM)
                            .text_color(theme.accent.text)
                            .cursor_pointer()
                            .role(gpui::Role::Link)
                            .aria_label(SharedString::from(format!(
                                "Open the definition {definition_name}"
                            )))
                            .on_click({
                                let open = open.clone();
                                move |_: &ClickEvent, _, cx| {
                                    open.act(cx, |studio| studio.open_definition(element))
                                }
                            })
                            .child(definition_name.clone()),
                    )
                    .child(Chip::new(SharedString::from(source)).tone(
                        if copied.is_some() {
                            Tone::Info
                        } else {
                            Tone::Neutral
                        },
                    )),
            )
            .child(
                div()
                    .text_size(r(theme::text::SM))
                    .line_height(r(17.0))
                    .text_color(theme.text_secondary)
                    .child(note),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(r(6.0))
                    .child(
                        Button::new("reuse-open", "Open definition")
                            .small()
                            .icon(IconName::Definition)
                            .tooltip("Show its inside; Backspace goes back", Some("Enter"))
                            .on_click(move |_, _, cx| {
                                open.act(cx, |studio| studio.open_definition(element))
                            }),
                    )
                    .child(
                        Button::new(
                            "reuse-usages",
                            format!("Find usages ({})", usage_count + special_count),
                        )
                        .small()
                        .icon(IconName::Search)
                        .tooltip("Every usage and specialisation", Some("Shift+F12"))
                        .on_click(move |_, _, cx| {
                            find.act(cx, |studio| {
                                studio.find_usages(element);
                                studio.mark(Dirty::OVERLAY);
                            })
                        }),
                    )
                    .child(
                        Button::new("reuse-specialize", "Specialise…")
                            .small()
                            .icon(IconName::Branch)
                            .disabled(!editable)
                            .tooltip(
                                "A variant: a new definition that has everything this one has; the original stays as it is",
                                None,
                            )
                            .on_click(move |_, _, cx| {
                                special.act(cx, |studio| studio.start_specialize(element))
                            }),
                    )
                    .when(!is_usage, |this| {
                        this.child(
                            Button::new("reuse-save", "Save to My Library…")
                                .small()
                                .icon(IconName::Save)
                                .on_click(move |_, _, cx| {
                                    save.act(cx, |studio| studio.start_save(element))
                                }),
                        )
                    }),
            )
            .when(!list.is_empty(), |this| {
                this.child(super::group(
                    if is_usage { "Values from its definition" } else { "Inherited values" },
                    Some(list.len()),
                    cx,
                ))
                .children(list.into_iter().enumerate().map(|(index, item)| {
                    inherited_row(studio, element, index, item, editable, cx)
                }))
            })
            .into_any_element(),
    )
}

/// One inherited value: overridden here (with Reset) or inherited (with
/// Override).
fn inherited_row(
    studio: &Entity<Studio>,
    owner: ElementId,
    index: usize,
    item: Inherited,
    editable: bool,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let studio = studio.clone();
    let overridden = item.overridden;
    let label = item.label.clone();
    let origin = if overridden.is_some() {
        "Overridden here".to_string()
    } else {
        format!("From {}", item.from)
    };
    let tooltip_text = SharedString::from(origin.clone());
    div()
        .id(("inherited", index))
        .h(r(26.0))
        .flex()
        .items_center()
        .gap(r(6.0))
        .text_size(r(theme::text::SM))
        .role(gpui::Role::Group)
        .aria_label(SharedString::from(format!(
            "{} {}, {}",
            item.label, item.value, origin
        )))
        .tooltip(move |window, cx| ui::tooltip::text(tooltip_text.clone(), None)(window, cx))
        .child(
            icon(if overridden.is_some() {
                IconName::Pencil
            } else {
                IconName::Attribute
            })
            .size(12.0)
            .color(if overridden.is_some() {
                theme.accent.text
            } else {
                theme.text_faint
            }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .font_family(theme::MONO)
                .text_color(if overridden.is_some() {
                    theme.text
                } else {
                    theme.text_secondary
                })
                .child(item.label.clone()),
        )
        .child(
            div()
                .flex_none()
                .max_w(r(96.0))
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .font_family(theme::MONO)
                .text_color(if overridden.is_some() {
                    theme.accent.text
                } else {
                    theme.text_muted
                })
                .child(item.value.clone()),
        )
        .child(match overridden {
            Some(redefinition) => Button::new(("reset", index), "Reset")
                .small()
                .ghost()
                .disabled(!editable)
                .tooltip("Take the definition's value again", None)
                .on_click(move |_, _, cx| {
                    let label = label.clone();
                    studio.act(cx, |studio| {
                        let owner_name = studio
                            .project
                            .as_ref()
                            .and_then(|p| {
                                p.state().tree().effective_name(owner).map(str::to_string)
                            })
                            .unwrap_or_default();
                        studio.operation(
                            &format!("Reset {label} in {owner_name}"),
                            agq_system_state::Operation::Delete {
                                element: redefinition,
                            },
                        );
                        studio.mark(Dirty::ALL);
                    })
                })
                .into_any_element(),
            None => {
                let path = item.path.clone();
                let value = item
                    .value
                    .strip_prefix("= ")
                    .map(str::to_string)
                    .unwrap_or_default();
                div()
                    .relative()
                    .child(ui::target::target(format!("Override {}", item.label)))
                    .child(
                        Button::new(("override", index), "Override")
                            .small()
                            .ghost()
                            .disabled(!editable)
                            .tooltip("A value here only; the definition keeps its own", None)
                            .on_click(move |_, _, cx| {
                                let path = path.clone();
                                let value = value.clone();
                                studio.act(cx, |studio| {
                                    studio.dialog = Some(Dialog::Override { owner, path, value });
                                    studio.mark(Dirty::OVERLAY);
                                })
                            }),
                    )
                    .into_any_element()
            }
        })
}

/// The gallery's sample (§8.5 rule 2): a value overridden here and one from
/// the definition, as the Inspector lists them.
pub fn sample(studio: &Entity<Studio>, cx: &App) -> AnyElement {
    let id = ElementId::from_raw;
    let rows = [
        Inherited {
            path: vec![id(1)],
            label: "cache.ttlSeconds".into(),
            value: "= 60".into(),
            overridden: Some(id(2)),
            from: "SessionStore".into(),
        },
        Inherited {
            path: vec![id(3)],
            label: "cache.maxEntries".into(),
            value: ": Positive".into(),
            overridden: None,
            from: "Cache".into(),
        },
    ];
    div()
        .flex()
        .flex_col()
        .gap(r(6.0))
        .child(super::group(
            "Values from its definition",
            Some(rows.len()),
            cx,
        ))
        .children(
            rows.into_iter()
                .enumerate()
                .map(|(index, item)| inherited_row(studio, id(0), index, item, true, cx)),
        )
        .into_any_element()
}

/// "2 usages and 1 specialisation", leaving out what there is none of.
pub(crate) fn uses(usages: usize, specialisations: usize) -> String {
    let mut parts = Vec::new();
    if usages > 0 {
        parts.push(crate::conversation::plural(usages, "usage", "usages"));
    }
    if specialisations > 0 {
        parts.push(crate::conversation::plural(
            specialisations,
            "specialisation",
            "specialisations",
        ));
    }
    parts.join(" and ")
}

#[cfg(test)]
mod tests {
    use super::inherited;
    use agq_language::{Source, parse};

    #[test]
    fn inherited_values_are_found_and_overrides_marked() {
        let tree = parse(&[Source::new(
            "p.sysml",
            "package P {
    part def Cache { attribute ttl : ScalarValues::Natural = 300; }
    part def Store { part cache : Cache; attribute size : ScalarValues::Natural = 5; }
    part def System {
        part s : Store { attribute :>> size = 9; }
    }
}",
        )]);
        let usage = tree.find("P::System::s").unwrap();
        let found = inherited(&tree, usage);
        let labels: Vec<(&str, &str, bool)> = found
            .iter()
            .map(|i| (i.label.as_str(), i.value.as_str(), i.overridden.is_some()))
            .collect();
        assert!(labels.contains(&("size", "= 9", true)), "{labels:?}");
        assert!(
            labels.contains(&("cache.ttl", "= 300", false)),
            "{labels:?}"
        );
        let store = tree.find("P::Store").unwrap();
        // A definition's own attributes are not "inherited".
        assert!(inherited(&tree, store).iter().all(|i| i.label != "size"),);
    }
}
