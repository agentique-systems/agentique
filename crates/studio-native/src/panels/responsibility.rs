//! The Inspector's "About this part" (ROADMAP §2.8 C1, C-51): for a part
//! def, or a part by its type, what it is and why, what information it owns,
//! its contract, what it may use and what depends on it, where it is
//! implemented and checked, and what changing it affects, all from the model
//! and its links (`agq_implementation::responsibility`, the answer the
//! Assistant's `explain_element` gives too). Each related part opens here
//! without moving the selection or the camera; Back returns.

use super::evidence::inspect_row;
use crate::{
    studio::{Dirty, Studio},
    ui::{self, ActiveTheme, IconName, Tone, r, theme},
    workspace::StudioExt,
};
use agq_implementation::LinkKind;
use agq_language::{ElementId, Tree};
use gpui::{
    AnyElement, App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
};

/// The section, or `None` when the element is not a part.
pub fn section(studio: &Entity<Studio>, element: ElementId, cx: &App) -> Option<AnyElement> {
    let state = studio.read(cx);
    let project = state.project.as_ref()?;
    let tree = project.state().tree();
    let about = state.responsibility(element)?;
    let theme = cx.theme().clone();
    let name = |id: ElementId| tree.effective_name(id).unwrap_or("?").to_string();
    // Navigated away from the selection: offer the way back.
    let selected = state.selection.element(&state.scene);
    let back = selected
        .filter(|selected| *selected != element)
        .map(|selected| (selected, crate::edit::display_name(tree, selected)));
    let rows =
        |label: &'static str, ids: &[ElementId], glyph: IconName, key: &str| -> Vec<AnyElement> {
            ids.iter()
                .enumerate()
                .map(|(index, id)| {
                    inspect_row(
                        studio,
                        format!("about-{key}-{index}"),
                        Some(*id),
                        label.to_string(),
                        name(*id),
                        glyph,
                        cx,
                    )
                })
                .collect()
        };
    let mut body: Vec<AnyElement> = Vec::new();
    body.push(
        div()
            .text_size(r(theme::text::SM))
            .line_height(r(18.0))
            .text_color(theme.text_secondary)
            .child(if about.purpose.is_empty() {
                "The model does not say what this part is for: it has no documentation.".to_string()
            } else {
                about.purpose.clone()
            })
            .into_any_element(),
    );
    // A referential part: it refers to a part that exists elsewhere, and
    // what follows is about its type (C-55).
    if let Some(usage) = &about.usage {
        body.push(
            ui::Banner::new(
                Tone::Info,
                format!(
                    "Refers to {}: `{}` does not contain it. What follows describes its type, {}.",
                    usage.target_text(),
                    usage.name,
                    about.name
                ),
            )
            .into_any_element(),
        );
    }
    if !about.refers.is_empty() {
        body.push(
            super::group("Refers to (not owned)", Some(about.refers.len()), cx).into_any_element(),
        );
        for referential in &about.refers {
            body.push(
                div()
                    .font_family(theme::MONO)
                    .text_size(r(theme::text::SM))
                    .pb(r(4.0))
                    .child(format!(
                        "{} → {}",
                        referential.name,
                        referential
                            .refers_to
                            .as_deref()
                            .unwrap_or("nothing in this configuration")
                    ))
                    .into_any_element(),
            );
        }
    }
    if !about.owns.is_empty() {
        body.push(
            super::group("Information it owns", Some(about.owns.len()), cx).into_any_element(),
        );
        for (feature, doc) in &about.owns {
            body.push(
                div()
                    .flex()
                    .flex_col()
                    .gap(r(2.0))
                    .pb(r(4.0))
                    .child(
                        div()
                            .font_family(theme::MONO)
                            .text_size(r(theme::text::SM))
                            .child(feature.clone()),
                    )
                    .when(!doc.is_empty(), |this| {
                        this.child(
                            div()
                                .text_size(r(theme::text::XS))
                                .text_color(theme.text_muted)
                                .child(doc.clone()),
                        )
                    })
                    .into_any_element(),
            );
        }
    }
    body.push(super::group("Contract", Some(about.contract.len()), cx).into_any_element());
    if about.contract.is_empty() {
        body.push(super::note("No ports are modelled.", cx).into_any_element());
    }
    for port in &about.contract {
        let list = |items: &[String]| {
            if items.is_empty() {
                "nothing".to_string()
            } else {
                items.join(", ")
            }
        };
        body.push(
            div()
                .flex()
                .flex_col()
                .gap(r(2.0))
                .pb(r(6.0))
                .child(
                    div()
                        .font_family(theme::MONO)
                        .text_size(r(theme::text::SM))
                        .child(format!("{} : {}", port.name, port.type_name)),
                )
                .child(
                    div()
                        .text_size(r(theme::text::XS))
                        .text_color(theme.text_muted)
                        .child(format!(
                            "receives {} · sends {}",
                            list(&port.receives),
                            list(&port.sends)
                        )),
                )
                .into_any_element(),
        );
    }
    body.push(super::group("Depends on", Some(about.depends_on.len()), cx).into_any_element());
    if about.depends_on.is_empty() {
        body.push(super::note("Nothing: it may use no other part.", cx).into_any_element());
    }
    body.extend(rows(
        "may use",
        &about.depends_on,
        IconName::Connection,
        "uses",
    ));
    body.push(
        super::group(
            "Used by",
            Some(about.used_by.len() + about.connected.len()),
            cx,
        )
        .into_any_element(),
    );
    if about.used_by.is_empty() && about.connected.is_empty() {
        body.push(super::note("Nothing depends on it.", cx).into_any_element());
    }
    body.extend(rows(
        "depends on it",
        &about.used_by,
        IconName::Connection,
        "used",
    ));
    body.extend(rows(
        "exchanges items",
        &about.connected,
        IconName::Port,
        "connected",
    ));
    body.push(
        super::group("Implemented in", Some(about.implemented_in.len()), cx).into_any_element(),
    );
    if about.implemented_in.is_empty() {
        body.push(
            super::note(
                "No implementation links: the model does not say where its code is.",
                cx,
            )
            .into_any_element(),
        );
    } else {
        let count = |kind: LinkKind| {
            about
                .implemented_in
                .iter()
                .filter(|(k, _)| *k == kind)
                .count()
        };
        let crates: Vec<&str> = about
            .implemented_in
            .iter()
            .filter(|(k, _)| *k == LinkKind::Crate)
            .map(|(_, at)| at.as_str())
            .collect();
        body.push(
            super::note(
                format!(
                    "{}{} function(s) and {} test(s) linked (the Implementation section below opens them).",
                    if crates.is_empty() { String::new() } else { format!("{}; ", crates.join(", ")) },
                    count(LinkKind::Symbol) + count(LinkKind::EntryPoint) + count(LinkKind::Config),
                    count(LinkKind::Test),
                ),
                cx,
            )
            .into_any_element(),
        );
    }
    body.push(
        super::group(
            "Checked by",
            Some(about.requirements.len() + about.scenarios.len()),
            cx,
        )
        .into_any_element(),
    );
    body.extend(rows(
        "keeps",
        &about.requirements,
        IconName::Requirement,
        "requirement",
    ));
    for (index, scenario) in about.scenarios.iter().enumerate() {
        let studio = studio.clone();
        let scenario = *scenario;
        body.push(
            div()
                .id(("about-scenario", index))
                .min_h(r(26.0))
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
                        studio.select_scenario(scenario);
                        studio.mark(Dirty::LAYOUT | Dirty::STATUS);
                    })
                })
                .child(
                    ui::icon(IconName::Scenario)
                        .size(13.0)
                        .color(theme.text_muted),
                )
                .child(
                    div()
                        .flex_none()
                        .text_color(theme.text_muted)
                        .child("scenario"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .font_family(theme::MONO)
                        .text_color(theme.text_secondary)
                        .child(name(scenario)),
                )
                .into_any_element(),
        );
    }
    if about.requirements.is_empty() && about.scenarios.is_empty() {
        body.push(super::note("No requirement or scenario covers it.", cx).into_any_element());
    }
    body.push(super::group("If you change it", None, cx).into_any_element());
    body.push(
        ui::inline_message(
            if about.locked_by.is_some() {
                Tone::Warning
            } else {
                Tone::Neutral
            },
            change_impact(tree, &about),
            cx,
        )
        .into_any_element(),
    );
    Some(
        div()
            .flex()
            .flex_col()
            .child(super::group("About this part", None, cx))
            .when_some(back, |this, (selected, label)| {
                let studio = studio.clone();
                this.child(
                    ui::Button::new("about-back", format!("Back to {label}"))
                        .small()
                        .icon(IconName::ChevronLeft)
                        .on_click(move |_: &ClickEvent, _, cx| {
                            studio.act(cx, |studio| {
                                studio.inspected =
                                    Some((studio.selection.primary.clone(), selected));
                                studio.mark(Dirty::SELECTION | Dirty::LAYOUT);
                            })
                        }),
                )
            })
            .children(body)
            .into_any_element(),
    )
}

/// What changing the part affects, in one sentence.
pub fn change_impact(
    tree: &Tree,
    about: &agq_implementation::responsibility::Responsibility,
) -> String {
    let tests = about
        .implemented_in
        .iter()
        .filter(|(kind, _)| *kind == LinkKind::Test)
        .count();
    let dependents = match about.used_by.len() {
        0 => "Nothing depends on it".to_string(),
        n => format!("{n} part(s) depend on it"),
    };
    let lock = match about.locked_by {
        Some(lock) => format!(
            "; it is locked (by {}), so changing it asks first, and a patch to its code asks again at integration",
            tree.effective_name(lock).unwrap_or("?")
        ),
        None => String::new(),
    };
    format!(
        "{dependents}{lock}. {} scenario(s) and {tests} linked test(s) cover it.",
        about.scenarios.len()
    )
}
