//! The Requirements Panel's content: each requirement with its subject, its
//! problems and its evidence ladder (C-55): what declares it satisfied (a
//! claim), what the model calculates, which scenarios verify it and what
//! its linked tests say, kept apart. The Inspector and the Assistant's
//! `check_requirements` read the same ladders.
use crate::studio::{Dirty, Studio};
use crate::ui::Tone;
use agq_implementation::requirements::{
    Ladder, ScenarioResult, ScenarioRuns, Standing, describe, headline, ladders,
};
use agq_implementation::{CheckReport, Links};
use agq_language::{Element, ElementId, ElementKind, Parent, Reference};
use agq_studio_scene::SceneTarget;
use agq_system_state::{Operation, SystemState};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct Row {
    pub id: ElementId,
    pub keyword: &'static str,
    pub name: String,
    pub doc: Option<String>,
    pub subjects: Vec<String>,
    pub problems: Vec<String>,
    /// Its evidence, kept apart.
    pub ladder: Ladder,
}

impl Studio {
    /// The Requirements Panel's rows for the current model, built once per
    /// version of the model, the kept results and the implementation checks.
    pub fn requirements(&mut self) -> Vec<Row> {
        let key = self.evidence_key();
        if self
            .requirement_rows
            .as_ref()
            .is_none_or(|(kept, _)| *kept != key)
        {
            let rows = self.requirement_rows_now();
            self.requirement_rows = Some((key, rows));
        }
        self.requirement_rows
            .as_ref()
            .map(|(_, rows)| rows.clone())
            .unwrap_or_default()
    }

    /// The rows, read now (or from the kept rows when they are still right).
    pub fn requirement_rows_now(&self) -> Vec<Row> {
        if let Some((kept, rows)) = &self.requirement_rows
            && *kept == self.evidence_key()
        {
            return rows.clone();
        }
        let Some(project) = &self.project else {
            return Vec::new();
        };
        let links = self.implementation_links().unwrap_or_default();
        let runs = self.scenario_runs();
        let report = self.implementation.report.as_ref().map(|report| {
            let current = self
                .implementation
                .freshness
                .as_ref()
                .is_some_and(agq_simulation::Freshness::is_current);
            (report, current)
        });
        rows(project.state(), &links, &runs, report)
    }

    /// One requirement's ladder, for the Inspector.
    pub fn requirement_ladder(&self, id: ElementId) -> Option<Ladder> {
        self.requirement_rows_now()
            .into_iter()
            .find(|row| row.id == id)
            .map(|row| row.ladder)
    }

    /// `check_requirements` for the Assistant: the headline and every
    /// ladder, or one, in words.
    pub fn check_requirements(&self, requirement: Option<ElementId>) -> String {
        let Some(project) = &self.project else {
            return "No project is open.".into();
        };
        let ladders: Vec<Ladder> = self
            .requirement_rows_now()
            .into_iter()
            .map(|row| row.ladder)
            .collect();
        describe(project.state().tree(), &ladders, requirement)
    }

    /// Shows or hides a requirement's ladder in the Requirements panel.
    pub fn toggle_requirement(&mut self, id: ElementId) {
        if !self.requirements_open.remove(&id) {
            self.requirements_open.insert(id);
        }
        self.mark(Dirty::LAYOUT);
    }

    /// What the rows depend on besides the model: the kept results and
    /// the newest implementation checks.
    fn evidence_key(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.generation.hash(&mut hasher);
        self.runs.saved_generation.hash(&mut hasher);
        self.implementation
            .report
            .as_ref()
            .map(|r| r.id.as_str())
            .hash(&mut hasher);
        self.implementation
            .freshness
            .as_ref()
            .map(agq_simulation::Freshness::is_current)
            .hash(&mut hasher);
        hasher.finish()
    }

    /// Every scenario's newest results, as the ladder reads them.
    fn scenario_runs(&self) -> Vec<ScenarioRuns> {
        self.scenario_rows_now()
            .into_iter()
            .map(|row| ScenarioRuns {
                scenario: row.id,
                results: row
                    .latest
                    .iter()
                    .map(|(mode, summary, current)| ScenarioResult {
                        mode: *mode,
                        status: summary.status,
                        all_passed: summary.all_passed,
                        current: *current,
                    })
                    .collect(),
            })
            .collect()
    }

    /// The selected part or item a requirement can be satisfied by.
    pub fn satisfying_part(&self) -> Option<ElementId> {
        let tree = self.project.as_ref()?.state().tree();
        self.selected_card().filter(|id| {
            tree.get(*id)
                .is_some_and(|e| matches!(e.kind, ElementKind::Part | ElementKind::Item))
        })
    }

    /// Selects an element that has a card and moves the camera to it.
    pub fn show_element(&mut self, id: ElementId) {
        let target = SceneTarget::Node(id);
        if self.scene.target_bounds(&target).is_some() {
            self.select(target.clone(), false);
            self.frame_target(&target);
        }
    }

    /// Adds `satisfy requirement by part` next to the requirement.
    pub fn satisfy(&mut self, requirement: ElementId, part: ElementId) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        let Some(owner) = tree.get(requirement).and_then(Element::owner) else {
            self.status = "A top-level requirement cannot hold a satisfy relationship".into();
            return;
        };
        // Both ends are linked by identity; the printed text names them so
        // that they resolve back to the same elements from the satisfy's owner.
        let mut element = Element::new(ElementKind::Satisfy);
        element.target = Some(Reference::to(
            requirement,
            tree.effective_name(requirement).unwrap_or(""),
        ));
        element.by = Some(Reference::to(part, tree.effective_name(part).unwrap_or("")));
        let description = format!(
            "{} satisfies {}",
            tree.effective_name(part).unwrap_or("part"),
            tree.effective_name(requirement).unwrap_or("requirement")
        );
        self.operation(
            &description,
            Operation::Create {
                parent: Parent::Element(owner),
                element: Box::new(element),
            },
        );
    }
}

/// The headline over the rows: what the requirement usages stand on.
pub fn rows_headline(rows: &[Row]) -> String {
    let ladders: Vec<Ladder> = rows.iter().map(|row| row.ladder.clone()).collect();
    headline(&ladders)
}

/// How a standing is coloured: a violation or a failure is danger, a
/// verification or a calculation that holds is success, a declaration
/// alone is neutral, nothing at all a warning.
pub fn tone(standing: Standing) -> Tone {
    match standing {
        Standing::Violated | Standing::FailingScenario => Tone::Danger,
        Standing::Verified | Standing::Holds => Tone::Success,
        Standing::OnlyDeclared => Tone::Neutral,
        Standing::Nothing => Tone::Warning,
    }
}

/// The rows of the Requirements Panel: every requirement and requirement
/// def in document order, with its ladder.
pub fn rows(
    state: &SystemState,
    links: &Links,
    runs: &[ScenarioRuns],
    report: Option<(&CheckReport, bool)>,
) -> Vec<Row> {
    let tree = state.tree();
    let ladders = ladders(tree, state.diagnostics(), links, runs, report);
    ladders
        .into_iter()
        .map(|ladder| {
            let id = ladder.requirement;
            let e = &tree[id];
            let children = || e.children().iter().map(|c| &tree[*c]);
            Row {
                id,
                keyword: e.kind.keyword(),
                name: crate::edit::display_path(tree, id),
                doc: children()
                    .find(|c| c.kind == ElementKind::Doc)
                    .and_then(|c| c.text.clone()),
                subjects: children()
                    .filter(|c| c.kind == ElementKind::Subject)
                    .map(|c| {
                        c.typed_by
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .collect(),
                problems: tree
                    .descendants(id)
                    .into_iter()
                    .flat_map(|d| state.diagnostics_for(d).map(|p| p.message.clone()))
                    .collect(),
                ladder,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_language::{Source, parse};
    use agq_simulation::{Mode, RunStatus};
    use std::collections::BTreeSet;

    fn state() -> SystemState {
        let text = "package D {
    private import ScalarValues::*;
    part def Drone { attribute mass : Real; }
    requirement def Light {
        subject s : Drone;
        attribute limit : Real;
        require constraint { s.mass <= limit }
    }
    part scout : Drone { attribute :>> mass = 900; }
    part hauler : Drone { attribute :>> mass = 4000; }
    requirement scoutLight : Light { attribute :>> limit = 1000; }
    requirement haulerLight : Light { attribute :>> limit = 1000; }
    requirement pleasant { doc /* It is pleasant to fly. */ }
    requirement flies;
    satisfy scoutLight by scout;
    satisfy haulerLight by hauler;
    satisfy pleasant by scout;
    verification def Flight { subject drone : Drone; objective { verify flies; } }
}";
        let state = SystemState::new(parse(&[Source::new("d.sysml", text)]), BTreeSet::new());
        assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
        state
    }

    fn standing(rows: &[Row], name: &str) -> Standing {
        rows.iter()
            .find(|row| row.name.ends_with(name))
            .unwrap_or_else(|| panic!("no row {name}"))
            .ladder
            .standing()
    }

    #[test]
    fn the_headline_counts_what_each_requirement_stands_on_never_a_declaration_as_more() {
        let state = state();
        let rows = rows(&state, &Links::default(), &[], None);
        assert_eq!(rows.len(), 5, "the def and four usages");
        assert_eq!(standing(&rows, "scoutLight"), Standing::Holds);
        assert_eq!(standing(&rows, "haulerLight"), Standing::Violated);
        // Satisfied by declaration only: never shown as more than declared.
        assert_eq!(standing(&rows, "pleasant"), Standing::OnlyDeclared);
        assert_eq!(standing(&rows, "flies"), Standing::Nothing);
        assert_eq!(
            rows_headline(&rows),
            "1 violated by calculation · 1 holds by calculation · 1 only declared · 1 with no evidence (of 4)"
        );
        assert_eq!(tone(Standing::OnlyDeclared), Tone::Neutral);
        assert_eq!(tone(Standing::Violated), Tone::Danger);
        let pleasant = rows.iter().find(|r| r.name.ends_with("pleasant")).unwrap();
        assert!(
            pleasant
                .ladder
                .summary()
                .contains("declared: satisfy by scout"),
            "{}",
            pleasant.ladder.summary()
        );
        assert!(
            pleasant
                .ladder
                .summary()
                .starts_with("calculated for scout: not evaluable"),
            "{}",
            pleasant.ladder.summary()
        );
    }

    #[test]
    fn a_current_passing_scenario_verifies_and_an_outdated_one_does_not() {
        let state = state();
        let scenario = state.tree().find("D::Flight").unwrap();
        let result = |current: bool| ScenarioRuns {
            scenario,
            results: vec![ScenarioResult {
                mode: Mode::Model,
                status: RunStatus::Completed,
                all_passed: true,
                current,
            }],
        };
        let rows_now = rows(&state, &Links::default(), &[result(true)], None);
        assert_eq!(standing(&rows_now, "flies"), Standing::Verified);
        let rows_then = rows(&state, &Links::default(), &[result(false)], None);
        assert_eq!(standing(&rows_then, "flies"), Standing::Nothing);
    }

    #[test]
    fn the_studio_reads_kept_results_into_the_ladder() {
        let (mut app, folder) = crate::edit::app_tests::studio("requirement-ladder");
        app.create_sample(
            &folder.0.join("Screening"),
            crate::studio::SAMPLE_NAME,
            crate::studio::Sample::Screening,
        );
        // Informal requirements, declared satisfied: claims only.
        let before = app.check_requirements(None);
        assert!(
            before.starts_with("Requirements: 2 only declared (of 2)."),
            "{before}"
        );
        assert_eq!(rows_headline(&app.requirements()), "2 only declared (of 2)");
        let tree = app.project.as_ref().unwrap().state().tree();
        let allowed = tree.find("UrlShortener::ShortenAllowed").unwrap();
        let screened = tree.find("UrlShortener::screenedLinks").unwrap();
        app.select_scenario(allowed);
        app.start_run(Mode::Model);
        let started = std::time::Instant::now();
        while !app.poll_runs() {
            assert!(started.elapsed().as_secs() < 20, "the run finishes");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ladder = app.requirement_ladder(screened).unwrap();
        assert_eq!(ladder.standing(), Standing::Verified);
        let after = app.check_requirements(Some(screened));
        assert!(
            after.starts_with(
                "UrlShortener::screenedLinks: verified by a current passing scenario."
            ),
            "{after}"
        );
        assert!(
            after.contains("Scenario UrlShortener::ShortenAllowed verifies it: Model execution: passed (current)."),
            "{after}"
        );
        assert!(
            after.contains("Declared (a claim, not evidence): satisfy by shortener.api."),
            "{after}"
        );
        // The panel's rows follow the kept result without a model change.
        assert_eq!(
            rows_headline(&app.requirements()),
            "1 verified by a current passing scenario · 1 only declared (of 2)"
        );
    }
}
