//! The Inspector's sections for the factory loop (C-50, W7.4, W8.2): a
//! part's behaviour, a stand-in's answer, an agent's settings, the
//! scenarios that verify a requirement with their newest results, and the
//! code linked to an element with its drift. Every change goes through the
//! System State or the links file; none of it is SysML text.
use crate::{
    runs::stand_in_features,
    studio::{Dirty, Studio},
    ui::{self, ActiveTheme, Button, IconName, Segmented, Tone, icon, r, theme},
    workspace::StudioExt,
};
use agq_language::{ElementId, ElementKind, Semantics, Tree};
use gpui::{
    AnyElement, App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
};

/// The outcomes a stand-in may give, as the Scenarios library names them.
const OUTCOMES: [&str; 5] = [
    "answer",
    "timeout",
    "invalidOutput",
    "refusal",
    "toolUnavailable",
];

/// The agent settings the Inspector lists, as the Agents library names them.
const AGENT_SETTINGS: [&str; 6] = [
    "mode",
    "model",
    "minConfidence",
    "maxLatencyMs",
    "maxCostPerCallUsd",
    "fallback",
];

pub fn sections(
    studio: &Entity<Studio>,
    element: ElementId,
    editable: bool,
    cx: &App,
) -> Vec<AnyElement> {
    let state = studio.read(cx);
    let Some(project) = state.project.as_ref() else {
        return Vec::new();
    };
    let tree = project.state().tree();
    let mut out = Vec::new();
    if let Some(section) = behaviour(studio, tree, element, cx) {
        out.push(section);
    }
    if let Some(section) = stand_in(studio, tree, element, editable, cx) {
        out.push(section);
    }
    if let Some(section) = agent(studio, tree, element, cx) {
        out.push(section);
    }
    if let Some(section) = evidence(studio, tree, element, cx) {
        out.push(section);
    }
    out.push(implementation(studio, element, editable, cx));
    out
}

/// A clickable row that shows `target` in the Inspector.
fn inspect_row(
    studio: &Entity<Studio>,
    id: impl Into<SharedString>,
    target: Option<ElementId>,
    label: String,
    detail: String,
    glyph: IconName,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let studio = studio.clone();
    div()
        .id(id.into())
        .min_h(r(26.0))
        .px(r(6.0))
        .mx(r(-6.0))
        .flex()
        .items_center()
        .gap(r(8.0))
        .rounded(r(crate::tokens::radius::CONTROL))
        .text_size(r(theme::text::SM))
        .when_some(target, |this, target| {
            this.cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .on_click(move |_: &ClickEvent, _, cx| {
                    studio.act(cx, |studio| {
                        studio.inspected = Some((studio.selection.primary.clone(), target));
                        studio.panel = crate::studio::Panel::Inspector;
                        studio.mark(Dirty::SELECTION | Dirty::LAYOUT);
                    })
                })
        })
        .relative()
        .child(ui::target::target(label.clone()))
        .child(icon(glyph).size(13.0).color(theme.text_muted))
        .child(div().flex_none().text_color(theme.text_muted).child(label))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .font_family(theme::MONO)
                .text_color(theme.text_secondary)
                .child(detail),
        )
        .into_any_element()
}

/// The state machine a part or definition exhibits: its states and the
/// transitions between them, read-only here (the Surface and the Assistant
/// change them).
fn behaviour(
    studio: &Entity<Studio>,
    tree: &Tree,
    element: ElementId,
    cx: &App,
) -> Option<AnyElement> {
    let semantics = Semantics::new(tree);
    let definition = match tree[element].kind {
        ElementKind::PartDef => element,
        ElementKind::Part => semantics.types_of(element).first()?.0,
        _ => return None,
    };
    let holder = tree.get(definition)?;
    let machine = holder
        .children()
        .iter()
        .copied()
        .find(|c| tree[*c].kind == ElementKind::State && tree[*c].exhibit)?;
    let mut rows = Vec::new();
    for (index, child) in tree[machine].children().iter().copied().enumerate() {
        let e = &tree[child];
        match e.kind {
            ElementKind::State => rows.push(inspect_row(
                studio,
                format!("behaviour-state-{index}"),
                Some(child),
                "state".into(),
                tree.effective_name(child).unwrap_or("?").to_string(),
                IconName::CircleDot,
                cx,
            )),
            ElementKind::Transition => {
                let end = |i: usize| {
                    e.ends
                        .get(i)
                        .map(|r| r.last_name().to_string())
                        .unwrap_or_else(|| "?".into())
                };
                let guard = e
                    .guard
                    .as_ref()
                    .map(|g| format!(" if {}", agq_language::print_expression(tree, child, g)))
                    .unwrap_or_default();
                rows.push(inspect_row(
                    studio,
                    format!("behaviour-transition-{index}"),
                    Some(child),
                    "transition".into(),
                    format!("{} → {}{guard}", end(0), end(1)),
                    IconName::ArrowRight,
                    cx,
                ));
            }
            _ => {}
        }
    }
    Some(
        div()
            .flex()
            .flex_col()
            .child(super::group("Behaviour", Some(rows.len()), cx))
            .children(rows)
            .into_any_element(),
    )
}

fn stand_in(
    studio: &Entity<Studio>,
    tree: &Tree,
    element: ElementId,
    editable: bool,
    cx: &App,
) -> Option<AnyElement> {
    let features = stand_in_features(tree, element)?;
    let theme = cx.theme().clone();
    let outcome = features
        .iter()
        .find(|(n, ..)| *n == "outcome")
        .map(|(_, _, text)| text.rsplit("::").next().unwrap_or("").to_string())
        .unwrap_or_default();
    let chosen = OUTCOMES.iter().position(|o| *o == outcome).unwrap_or(0);
    let chooser = studio.clone();
    let mut segmented = Segmented::new("stand-in-outcome", chosen);
    for o in OUTCOMES {
        segmented = segmented.choice(
            None,
            match o {
                "answer" => "answer",
                "timeout" => "timeout",
                "invalidOutput" => "invalid",
                "refusal" => "refusal",
                _ => "no tool",
            },
        );
    }
    let segmented = segmented.on_choose(move |index, _, cx| {
        if !editable {
            return;
        }
        chooser.act(cx, |studio| {
            let text = format!("Scenarios::Outcome::{}", OUTCOMES[index]);
            if let Err(error) = studio.set_stand_in_feature(element, "outcome", &text) {
                studio.status = error;
            }
        })
    });
    let rows: Vec<AnyElement> = features
        .into_iter()
        .filter(|(name, ..)| *name != "outcome")
        .enumerate()
        .map(|(index, (name, child, text))| {
            let add =
                child.is_none() && editable && matches!(name, "latencyMs" | "output" | "call");
            let studio_row = studio.clone();
            div()
                .flex()
                .items_center()
                .gap(r(6.0))
                .child(div().flex_1().min_w_0().child(inspect_row(
                    studio,
                    format!("stand-in-{index}"),
                    child,
                    name.to_string(),
                    if text.is_empty() {
                        "not set".into()
                    } else {
                        text
                    },
                    IconName::Attribute,
                    cx,
                )))
                .when(add, |this| {
                    this.child(
                        Button::new(("stand-in-set", index), "Set")
                            .small()
                            .ghost()
                            .on_click(move |_: &ClickEvent, _, cx| {
                                studio_row.act(cx, |studio| {
                                    let default = match name {
                                        "latencyMs" => "100",
                                        "call" => "1",
                                        _ => "null",
                                    };
                                    match studio.set_stand_in_feature(element, name, default) {
                                        Ok(()) => {
                                            // Show the new feature to type its value.
                                            let tree =
                                                studio.project.as_ref().map(|p| p.state().tree());
                                            let created = tree
                                                .and_then(|t| stand_in_features(t, element))
                                                .and_then(|f| {
                                                    f.into_iter().find(|(n, ..)| *n == name)
                                                })
                                                .and_then(|(_, c, _)| c);
                                            if let Some(created) = created {
                                                studio.inspected = Some((
                                                    studio.selection.primary.clone(),
                                                    created,
                                                ));
                                            }
                                        }
                                        Err(error) => studio.status = error,
                                    }
                                    studio.mark(Dirty::SELECTION | Dirty::MODEL);
                                })
                            }),
                    )
                })
                .into_any_element()
        })
        .collect();
    Some(
        div()
            .flex()
            .flex_col()
            .gap(r(4.0))
            .child(super::group("Stand-in", None, cx))
            .child(
                div()
                    .text_size(r(theme::text::XS))
                    .line_height(r(16.0))
                    .text_color(theme.text_muted)
                    .child("In model and code runs, this answers in place of its target. A live evaluation asks the real model instead."),
            )
            .child(div().pt(r(4.0)).child(segmented))
            .children(rows)
            .into_any_element(),
    )
}

fn agent(studio: &Entity<Studio>, tree: &Tree, element: ElementId, cx: &App) -> Option<AnyElement> {
    let semantics = Semantics::new(tree);
    let agent = semantics.resolve("Agents::Agent")?;
    let definition = match tree[element].kind {
        ElementKind::PartDef => element,
        ElementKind::Part => semantics.types_of(element).first()?.0,
        _ => return None,
    };
    if definition == agent || !semantics.specializes(definition, agent) {
        return None;
    }
    let theme = cx.theme().clone();
    let features = semantics.features(definition);
    let mut rows = Vec::new();
    for (index, setting) in AGENT_SETTINGS.iter().enumerate() {
        let own = features.iter().copied().find(|f| {
            semantics.name(*f) == Some(*setting)
                || tree
                    .get(*f)
                    .is_some_and(|e| e.redefines.iter().any(|r| r.last_name() == *setting))
        });
        let text = own
            .and_then(|f| tree.get(f).map(|e| (f, e)))
            .map(
                |(f, e)| match (&e.value, &e.expression, e.typed_by.first()) {
                    (Some(v), _, _) => v.to_string(),
                    (None, Some(x), _) => agq_language::print_expression(tree, f, x),
                    (None, None, Some(ty)) => ty.to_string(),
                    _ => "set".into(),
                },
            )
            .unwrap_or_else(|| "default".into());
        rows.push(inspect_row(
            studio,
            format!("agent-setting-{index}"),
            own.filter(|f| tree.contains(*f)),
            setting.to_string(),
            text,
            IconName::Attribute,
            cx,
        ));
    }
    Some(
        div()
            .flex()
            .flex_col()
            .child(super::group("Agent", None, cx))
            .child(
                div()
                    .pb(r(4.0))
                    .text_size(r(theme::text::XS))
                    .line_height(r(16.0))
                    .text_color(theme.text_muted)
                    .child("An answer that is late, invalid or below its minimum confidence counts as a failure: the fallback answers instead, when there is one."),
            )
            .children(rows)
            .into_any_element(),
    )
}

/// The scenarios that verify a requirement, or a scenario's own result.
fn evidence(
    studio: &Entity<Studio>,
    tree: &Tree,
    element: ElementId,
    cx: &App,
) -> Option<AnyElement> {
    let kind = tree[element].kind;
    let theme = cx.theme().clone();
    let in_scenario = tree[element]
        .owner()
        .filter(|o| tree[*o].kind == ElementKind::VerificationDef);
    if let Some(scenario) = in_scenario {
        let entity = studio.clone();
        let name = tree
            .effective_name(scenario)
            .unwrap_or("the scenario")
            .to_string();
        return Some(
            div()
                .flex()
                .flex_col()
                .child(super::group("Scenario", None, cx))
                .child(
                    Button::new("back-to-scenario", format!("Back to {name}"))
                        .icon(IconName::ChevronLeft)
                        .small()
                        .on_click(move |_: &ClickEvent, _, cx| {
                            entity.act(cx, |studio| studio.select_scenario(scenario))
                        }),
                )
                .into_any_element(),
        );
    }
    if kind == ElementKind::VerificationDef {
        let entity = studio.clone();
        return Some(
            div()
                .flex()
                .flex_col()
                .child(super::group("Scenario", None, cx))
                .child(
                    Button::new("open-in-run", "Open in the Run panel")
                        .icon(IconName::Play)
                        .small()
                        .on_click(move |_: &ClickEvent, _, cx| {
                            entity.act(cx, |studio| studio.select_scenario(element))
                        }),
                )
                .into_any_element(),
        );
    }
    if !matches!(kind, ElementKind::Requirement | ElementKind::RequirementDef) {
        return None;
    }
    let rows = studio.read(cx).verifying_scenarios(element);
    Some(
        div()
            .flex()
            .flex_col()
            .gap(r(2.0))
            .child(super::group("Evidence", Some(rows.len()), cx))
            .when(rows.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(r(theme::text::SM))
                        .text_color(theme.text_muted)
                        .child("No scenario verifies this yet. A satisfied requirement is not a verified one."),
                )
            })
            .children(rows.into_iter().enumerate().map(|(index, row)| {
                let entity = studio.clone();
                let id = row.id;
                div()
                    .id(("evidence", index))
                    .px(r(6.0))
                    .mx(r(-6.0))
                    .py(r(5.0))
                    .flex()
                    .flex_col()
                    .gap(r(4.0))
                    .rounded(r(crate::tokens::radius::CONTROL))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_click(move |_: &ClickEvent, _, cx| {
                        entity.act(cx, |studio| studio.select_scenario(id))
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(r(6.0))
                            .child(icon(IconName::Scenario).size(13.0).color(theme.text_muted))
                            .child(div().text_size(r(theme::text::SM)).child(row.name.clone())),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(r(4.0))
                            .when(row.latest.is_empty(), |this| {
                                this.child(ui::Badge::new("not run"))
                            })
                            .children(row.latest.iter().map(|(mode, s, current)| {
                                let (label, tone) = super::scenarios::result_chip(*mode, s.status, s.all_passed, *current);
                                ui::Badge::new(label).tone(tone)
                            })),
                    )
            }))
            .into_any_element(),
    )
}

/// The code linked to an element, what the checks found there, and the
/// controls to link, open and check it.
fn implementation(
    studio: &Entity<Studio>,
    element: ElementId,
    editable: bool,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let state = studio.read(cx);
    let links = state.implementation_links().unwrap_or_default();
    let linked: Vec<agq_implementation::Link> =
        links.for_element(element).into_iter().cloned().collect();
    let drift = state.drift().remove(&element).unwrap_or_default();
    let checking = state.implementation.checking();
    let outdated = matches!(
        state.implementation.freshness,
        Some(agq_simulation::Freshness::Outdated(_))
    );
    let link_entity = studio.clone();
    let check_entity = studio.clone();
    let implementable = state.project.as_ref().is_some_and(|p| {
        matches!(
            p.state().tree()[element].kind,
            ElementKind::PartDef | ElementKind::Part
        )
    });
    let tasks = if implementable {
        state.tasks_for(element)
    } else {
        Vec::new()
    };
    let active = state
        .implementation
        .task
        .as_ref()
        .filter(|t| t.element == element)
        .map(|t| {
            (
                t.job.clone(),
                t.started.elapsed().as_secs(),
                t.progress
                    .iter()
                    .rev()
                    .take(4)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        });
    let implement_entity = studio.clone();
    div()
        .flex()
        .flex_col()
        .gap(r(2.0))
        .child(super::group("Implementation", Some(linked.len()), cx))
        .when(linked.is_empty(), |this| {
            this.child(
                div()
                    .text_size(r(theme::text::SM))
                    .text_color(theme.text_muted)
                    .child("No code is linked to this element."),
            )
        })
        .children(linked.into_iter().enumerate().map(|(index, link)| {
            let open = studio.clone();
            let remove = studio.clone();
            let path = link.path.clone();
            let symbol = link.symbol.clone();
            let shown = match &link.symbol {
                Some(symbol) => format!("{}#{symbol}", link.path),
                None => link.path.clone(),
            };
            div()
                .flex()
                .items_center()
                .gap(r(6.0))
                .min_h(r(28.0))
                .child(ui::Badge::new(link.kind.label()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .font_family(theme::MONO)
                        .text_size(r(theme::text::SM))
                        .text_color(theme.text_secondary)
                        .child(shown),
                )
                .child({
                    let path = path.clone();
                    Button::icon_only(("link-open", index), IconName::External, "Open in the editor")
                        .ghost()
                        .small()
                        .tooltip("Open in the editor", None)
                        .on_click(move |_: &ClickEvent, _, cx| {
                            let path = path.clone();
                            open.act(cx, |studio| studio.open_in_editor(&path, None))
                        })
                })
                .when(editable, |this| {
                    this.child(
                        Button::icon_only(("link-remove", index), IconName::Drift, "Remove the link")
                            .ghost()
                            .small()
                            .tooltip("Remove the link", None)
                            .on_click(move |_: &ClickEvent, _, cx| {
                                let path = path.clone();
                                let symbol = symbol.clone();
                                remove.act(cx, |studio| studio.unlink_code(element, &path, symbol.as_deref()))
                            }),
                    )
                })
                .into_any_element()
        }))
        .when(!drift.is_empty(), |this| {
            this.child(
                div()
                    .pt(r(6.0))
                    .flex()
                    .flex_col()
                    .gap(r(6.0))
                    .children(drift.into_iter().map(|check| {
                        let (tone, glyph) = super::run::verdict_look(check.verdict);
                        let (_, colour, _) = tone.colours(&theme);
                        div()
                            .flex()
                            .items_start()
                            .gap(r(8.0))
                            .child(div().pt(r(2.0)).child(icon(glyph).size(13.0).color(colour)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(r(2.0))
                                    .child(
                                        div()
                                            .text_size(r(theme::text::SM))
                                            .font_weight(theme::MEDIUM)
                                            .child(format!("Drift: {}", check.name)),
                                    )
                                    .child(
                                        div()
                                            .text_size(r(theme::text::XS))
                                            .line_height(r(16.0))
                                            .text_color(theme.text_secondary)
                                            .child(check.message.clone()),
                                    )
                                    .children(check.details.iter().take(4).map(|d| {
                                        div()
                                            .text_size(r(theme::text::XS))
                                            .font_family(theme::MONO)
                                            .text_color(theme.text_muted)
                                            .child(d.clone())
                                    })),
                            )
                    })),
            )
        })
        .when(outdated, |this| {
            this.child(ui::inline_message(
                Tone::Neutral,
                "The last implementation checks are outdated: the model, the links or the code changed.",
                cx,
            ))
        })
        .when_some(active.clone(), |this, (_, seconds, progress)| {
            let stop = studio.clone();
            this.child(
                div()
                    .pt(r(8.0))
                    .flex()
                    .flex_col()
                    .gap(r(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(r(6.0))
                            .child(ui::primitives::spinner("task-spinner", 12.0, theme.info.solid))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(r(theme::text::SM))
                                    .child(format!("The worker is working · {}:{:02}", seconds / 60, seconds % 60)),
                            )
                            .child(Button::new("task-stop", "Stop").small().on_click(move |_: &ClickEvent, _, cx| {
                                stop.act(cx, |studio| studio.stop_task())
                            })),
                    )
                    .children(progress.into_iter().map(|line| {
                        div()
                            .text_size(r(theme::text::XS))
                            .font_family(theme::MONO)
                            .text_color(theme.text_muted)
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(line)
                    })),
            )
        })
        .children(tasks.into_iter().take(4).enumerate().filter(|(_, job)| active.as_ref().is_none_or(|(id, ..)| *id != job.id)).map(|(index, job)| {
            let review = studio.clone();
            let id = job.id.clone();
            let reviewable = matches!(
                job.state,
                agq_execution::jobs::JobState::WaitingForYou | agq_execution::jobs::JobState::Interrupted
            );
            div()
                .pt(r(6.0))
                .flex()
                .items_center()
                .gap(r(6.0))
                .child(icon(IconName::Patch).size(13.0).color(theme.text_muted))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(r(theme::text::SM))
                                .child(format!("{} · {}", job.title, match job.state {
                                    agq_execution::jobs::JobState::WaitingForYou => "ready for review",
                                    other => other.label(),
                                })),
                        )
                        .when_some(job.outcome.clone(), |this, outcome| {
                            this.child(
                                div()
                                    .text_size(r(theme::text::XS))
                                    .text_color(theme.text_muted)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(outcome),
                            )
                        }),
                )
                .when(reviewable, |this| {
                    this.child(Button::new(("task-review", index), "Review").small().primary().on_click(move |_: &ClickEvent, _, cx| {
                        let id = id.clone();
                        review.act(cx, |studio| studio.review_task(&id))
                    }))
                })
        }))
        .child(
            div()
                .pt(r(8.0))
                .flex()
                .flex_wrap()
                .gap(r(6.0))
                .when(editable && implementable && active.is_none(), |this| {
                    this.child(
                        Button::new("implement", "Implement with the Assistant…")
                            .small()
                            .icon(IconName::Patch)
                            .on_click(move |_: &ClickEvent, _, cx| {
                                implement_entity.act(cx, |studio| {
                                    studio.dialog = Some(crate::edit::Dialog::Implement {
                                        element,
                                        instructions: String::new(),
                                    });
                                    studio.mark(Dirty::OVERLAY);
                                })
                            }),
                    )
                })
                .when(editable, |this| {
                    this.child(
                        Button::new("link-code", "Link code…")
                            .small()
                            .icon(IconName::Connection)
                            .on_click(move |_: &ClickEvent, _, cx| {
                                link_entity.act(cx, |studio| {
                                    studio.dialog = Some(crate::edit::Dialog::LinkCode {
                                        element,
                                        kind: 0,
                                        path: String::new(),
                                        symbol: String::new(),
                                    });
                                    studio.mark(Dirty::OVERLAY);
                                })
                            }),
                    )
                })
                .child(
                    Button::new("check-implementation", if checking { "Checking…" } else { "Check the implementation" })
                        .small()
                        .icon(IconName::Drift)
                        .disabled(checking)
                        .on_click(move |_: &ClickEvent, _, cx| {
                            check_entity.act(cx, |studio| studio.execute(crate::commands::CommandId::CheckImplementation))
                        }),
                ),
        )
        .into_any_element()
}
