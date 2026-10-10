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
use agq_simulation::{Mode, RunResult};
use agq_studio_scene::SceneTarget;
use agq_system_state::{Operation, SystemState};
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

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
    /// For a subrequirement: whether the headline counts it with a
    /// requirement that contains it (its outermost container is a
    /// requirement usage, or a requirement def some usage is typed by).
    pub counted: bool,
}

impl Studio {
    /// The Requirements Panel's rows, built once per version of the model,
    /// the links, the kept results and the implementation checks, and kept
    /// for the panel, the Inspector and the Assistant alike.
    pub fn requirements(&self) -> Vec<Row> {
        let key = self.evidence_key();
        if let Some((kept, rows)) = &*self.requirement_rows.borrow()
            && *kept == key
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
        let rows = rows(project.state(), &links, &runs, report);
        *self.requirement_rows.borrow_mut() = Some((key, rows.clone()));
        rows
    }

    /// A requirement's row: its ladder, and for a subrequirement whether
    /// the headline counts it.
    pub fn requirement_row(&self, id: ElementId) -> Option<Row> {
        self.requirements().into_iter().find(|row| row.id == id)
    }

    /// `check_requirements` for the Assistant: the headline and every
    /// ladder, or one, in words.
    pub fn check_requirements(&self, requirement: Option<ElementId>) -> String {
        let Some(project) = &self.project else {
            return "No project is open.".into();
        };
        let ladders: Vec<Ladder> = self
            .requirements()
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

    /// What the rows depend on besides the model: the links, the kept
    /// results and the newest implementation checks. (A change to the code
    /// alone is read when one of these changes, as the Run panel reads it
    /// when a result is shown.)
    fn evidence_key(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.generation.hash(&mut hasher);
        self.runs.saved_generation.hash(&mut hasher);
        self.project
            .as_ref()
            .and_then(|p| p.links())
            .hash(&mut hasher);
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

    /// Every scenario's newest results, as the ladder reads them: an
    /// implementation result is current only while the code is what it ran
    /// (as the Run panel decides when it shows one).
    fn scenario_runs(&self) -> Vec<ScenarioRuns> {
        let store = self.run_store();
        let repository = self.implementation_repository();
        self.scenario_rows_now()
            .into_iter()
            .map(|row| ScenarioRuns {
                scenario: row.id,
                results: row
                    .latest
                    .iter()
                    .map(|(mode, summary, current)| {
                        let current = *current
                            && (*mode != Mode::Implementation
                                || store
                                    .as_ref()
                                    .and_then(|s| s.load(&summary.id))
                                    .is_some_and(|result| {
                                        code_current(&result, repository.as_deref())
                                    }));
                        ScenarioResult {
                            mode: *mode,
                            status: summary.status,
                            tally: summary.tally.clone(),
                            stop: summary.stop,
                            current,
                        }
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

/// Whether an implementation result still describes the code: without a
/// repository to compare with it stands, as in the Run panel.
pub(crate) fn code_current(result: &RunResult, repository: Option<&Path>) -> bool {
    repository.is_none_or(|repository| {
        agq_implementation::harness::code_is_current(result, repository).is_ok()
    })
}

/// The headline over the rows: what the requirement usages stand on, then
/// the subrequirement rows it leaves out: those counted with a requirement
/// that contains them, and those with their own `satisfy` or scenario
/// results, which it does not count (a subrequirement nothing counted
/// contains is in neither).
pub fn rows_headline(rows: &[Row]) -> String {
    let ladders: Vec<Ladder> = rows.iter().map(|row| row.ladder.clone()).collect();
    let nested: Vec<&Row> = rows.iter().filter(|r| r.ladder.nested).collect();
    let own = nested.iter().filter(|r| own_rungs(&r.ladder)).count();
    let counted = nested
        .iter()
        .filter(|r| r.counted && !own_rungs(&r.ladder))
        .count();
    let mut parts = vec![headline(&ladders)];
    match counted {
        0 => {}
        1 => parts.push("1 subrequirement counted with the requirement that contains it".into()),
        n => parts.push(format!(
            "{n} subrequirements counted with the requirements that contain them"
        )),
    }
    match own {
        0 => {}
        1 => parts.push(
            "1 subrequirement with its own satisfy or scenario results, which the headline does not count"
                .into(),
        ),
        k => parts.push(format!(
            "{k} subrequirements with their own satisfy or scenario results, which the headline does not count"
        )),
    }
    parts.join(" · ")
}

/// Whether a subrequirement has rungs of its own: a `satisfy` naming it, or
/// a scenario result verifying it. Calculations marked "within" are its
/// container's and are counted with it; a calculation of its own always has
/// a `satisfy` naming it. (Linked tests are not counted for any
/// requirement's standing.)
pub fn own_rungs(ladder: &Ladder) -> bool {
    !ladder.declared.is_empty()
        || ladder
            .scenarios
            .iter()
            .flat_map(|s| &s.results)
            .any(|r| r.passes() || r.fails() || r.inconclusive())
}

/// What a subrequirement is, said once for the panel and the Inspector.
pub fn subrequirement_note(row: &Row) -> &'static str {
    if own_rungs(&row.ladder) {
        "A subrequirement with its own satisfy or scenario results: they are shown here and the headline does not count them. Calculations marked \u{201c}within\u{201d} are its container's, counted with it."
    } else if row.counted {
        "A subrequirement: evaluated within the requirement that contains it (in a requirement def, within each requirement that uses the def) and counted with it, not on its own."
    } else {
        "A subrequirement no requirement usage contains (its def is not used): nothing counts it yet."
    }
}

/// A row's standing in words: for a subrequirement, whether it is counted
/// with the requirement that contains it, has results of its own, or is
/// not counted at all.
pub fn standing_words(row: &Row) -> String {
    let ladder = &row.ladder;
    if ladder.definition {
        ladder.summary()
    } else if ladder.nested && own_rungs(ladder) {
        format!(
            "{} · a subrequirement with its own satisfy or scenario results, which the headline does not count",
            ladder.label()
        )
    } else if ladder.nested && row.counted {
        format!(
            "{} · a subrequirement, counted with the requirement that contains it",
            ladder.label()
        )
    } else if ladder.nested {
        format!(
            "{} · a subrequirement no requirement usage contains, not counted",
            ladder.label()
        )
    } else {
        ladder.label()
    }
}

/// How a standing is coloured: a violation or a failure is danger, a
/// verification or a calculation that holds is success, a declaration
/// alone is neutral, nothing at all a warning.
pub fn tone(standing: Standing) -> Tone {
    match standing {
        Standing::Violated | Standing::FailingScenario => Tone::Danger,
        Standing::Verified | Standing::Holds => Tone::Success,
        Standing::HoldsPartly | Standing::OnlyDeclared | Standing::Inconclusive => Tone::Neutral,
        Standing::Nothing => Tone::Warning,
    }
}

/// A scenario result's chip in a ladder: the mode and what it says.
pub fn result_chip(result: &ScenarioResult) -> (String, Tone) {
    let tone = match result.word() {
        "passed" => Tone::Success,
        "failed" => Tone::Danger,
        _ => Tone::Neutral,
    };
    (
        format!(
            "{} · {}",
            crate::panels::scenarios::short_mode(result.mode),
            result.word()
        ),
        tone,
    )
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
    let ladders = ladders(tree, state.diagnostics(), Some(links), Some(runs), report);
    // What the headline counts: its requirement usages, then (to a fixed
    // point) the defs a counted usage's type is or specialises, and the
    // usages inside a counted container; a subrequirement is counted when
    // its outermost requirement container is.
    let semantics = agq_language::Semantics::new(tree);
    let requirement_like = |id: ElementId| {
        matches!(
            tree[id].kind,
            ElementKind::Requirement | ElementKind::RequirementDef
        )
    };
    let outermost = |id: ElementId| {
        let mut outermost = id;
        while let Some(owner) = tree[outermost].owner().filter(|o| requirement_like(*o)) {
            outermost = owner;
        }
        outermost
    };
    let usages: Vec<ElementId> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::Requirement)
        .collect();
    let defs: Vec<ElementId> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::RequirementDef && outermost(*id) == *id)
        .collect();
    let mut counted_containers: BTreeSet<ElementId> = ladders
        .iter()
        .filter(|l| !l.definition && !l.nested)
        .map(|l| l.requirement)
        .collect();
    loop {
        let types: Vec<ElementId> = usages
            .iter()
            .filter(|u| counted_containers.contains(&outermost(**u)))
            .flat_map(|u| semantics.types_of(*u))
            .map(|(t, _)| t)
            .collect();
        let more: Vec<ElementId> = defs
            .iter()
            .filter(|d| !counted_containers.contains(*d))
            .filter(|d| types.iter().any(|t| semantics.specializes(*t, **d)))
            .copied()
            .collect();
        if more.is_empty() {
            break;
        }
        counted_containers.extend(more);
    }
    let counted = |id: ElementId| counted_containers.contains(&outermost(id));
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
                counted: ladder.nested && counted(id),
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
    fn a_subrequirement_says_it_is_counted_with_the_requirement_that_contains_it() {
        let text = "package S {
    requirement def Purpose {
        requirement inner;
    }
    requirement purpose : Purpose;
}";
        let state = SystemState::new(parse(&[Source::new("s.sysml", text)]), BTreeSet::new());
        assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
        let rows = rows(&state, &Links::default(), &[], None);
        let words = |name: &str| {
            standing_words(
                rows.iter()
                    .find(|row| row.name.ends_with(name))
                    .unwrap_or_else(|| panic!("no row {name}")),
            )
        };
        assert!(
            words("inner")
                .contains("a subrequirement, counted with the requirement that contains it"),
            "{}",
            words("inner")
        );
        assert!(
            !words("purpose").contains("subrequirement"),
            "{}",
            words("purpose")
        );
        assert!(
            !words("Purpose").contains("subrequirement"),
            "{}",
            words("Purpose")
        );
        // A deliberate change to this assertion (#125's review): the
        // singular now reads as one.
        let headline = rows_headline(&rows);
        assert!(
            headline.ends_with(
                "(of 1) · 1 subrequirement counted with the requirement that contains it"
            ),
            "{headline}"
        );
    }

    /// A subrequirement with its own `satisfy` says so on its row and in
    /// the headline, never that it is counted with its container; one whose
    /// only calculations are its container's ("within") is still counted
    /// with it; one of a def no usage uses is counted by nothing (also in a
    /// model whose other requirements are used).
    #[test]
    fn a_subrequirement_says_whether_and_how_the_headline_counts_it() {
        let text = "package S {
    attribute def Mass;
    part def Kit {
        attribute mass : ScalarValues::Real;
    }
    part light : Kit {
        attribute :>> mass = 5;
    }
    part p;
    requirement def Purpose {
        subject kit : Kit;
        requirement inner;
        requirement other;
        requirement light {
            require constraint { kit.mass <= 10 }
        }
    }
    requirement purpose : Purpose;
    satisfy purpose by light;
    satisfy Purpose::inner by p;
    requirement def Unused {
        requirement nobody;
    }
}";
        let state = SystemState::new(parse(&[Source::new("s.sysml", text)]), BTreeSet::new());
        assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
        let rows = rows(&state, &Links::default(), &[], None);
        let row = |name: &str| {
            rows.iter()
                .find(|row| row.name.ends_with(name))
                .unwrap_or_else(|| panic!("no row {name}"))
        };
        let words = |name: &str| standing_words(row(name));
        assert!(
            words("inner").contains("its own satisfy or scenario results"),
            "{}",
            words("inner")
        );
        assert!(
            !words("inner").contains("counted with"),
            "{}",
            words("inner")
        );
        assert!(
            words("other").contains("counted with the requirement"),
            "{}",
            words("other")
        );
        // Its calculation is the container's: counted with it.
        assert!(
            !row("Purpose::light").ladder.calculated.is_empty(),
            "the container's calculation reaches it"
        );
        assert!(!own_rungs(&row("Purpose::light").ladder));
        assert!(
            words("Purpose::light").contains("counted with the requirement"),
            "{}",
            words("Purpose::light")
        );
        assert!(
            words("nobody").contains("not counted"),
            "{}",
            words("nobody")
        );
        assert!(subrequirement_note(row("nobody")).contains("nothing counts it"));
        assert!(subrequirement_note(row("inner")).contains("container's"));
        let headline = rows_headline(&rows);
        assert!(
            headline.ends_with(
                "2 subrequirements counted with the requirements that contain them · 1 subrequirement with its own satisfy or scenario results, which the headline does not count"
            ),
            "{headline}"
        );
        let unused = "package S {
    requirement def Purpose {
        requirement inner;
    }
}";
        let state = SystemState::new(parse(&[Source::new("s.sysml", unused)]), BTreeSet::new());
        let rows = super::rows(&state, &Links::default(), &[], None);
        let headline = rows_headline(&rows);
        assert!(!headline.contains("subrequirement"), "{headline}");
        // Used through a specialisation, or by a usage nested in a counted
        // requirement: counted.
        for used in [
            "package S {
    requirement def Purpose {
        requirement inner;
    }
    requirement def Sub :> Purpose;
    requirement s : Sub;
}",
            "package S {
    requirement def Purpose {
        requirement inner;
    }
    requirement outer {
        requirement nested : Purpose;
    }
}",
        ] {
            let state = SystemState::new(parse(&[Source::new("s.sysml", used)]), BTreeSet::new());
            assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
            let rows = super::rows(&state, &Links::default(), &[], None);
            let inner = rows
                .iter()
                .find(|r| r.name.ends_with("Purpose::inner"))
                .expect("Purpose::inner");
            assert!(inner.counted, "{used}");
            assert!(
                standing_words(inner).contains("counted with"),
                "{}",
                standing_words(inner)
            );
        }
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
                tally: vec![(agq_simulation::Verdict::Passed, 1)],
                stop: None,
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
        let ladder = app.requirement_row(screened).unwrap().ladder;
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

    #[test]
    fn an_implementation_pass_is_current_only_while_the_code_is_what_it_ran() {
        let folder =
            std::env::temp_dir().join(format!("agq-studio-code-current-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join("lib.rs"),
            "pub fn a() {}
",
        )
        .unwrap();
        agq_execution::git::init_and_commit(&folder, "code").unwrap();
        let digest = agq_execution::git::tree_digest(&folder).unwrap();
        let result = RunResult {
            format: agq_simulation::result::FORMAT,
            id: "run".into(),
            scenario: 1,
            scenario_name: "S".into(),
            scenario_qualified_name: "P::S".into(),
            mode: Mode::Implementation,
            started: "2026-10-10T00:00:00Z".into(),
            wall_ms: 0,
            logical_ms: 0,
            events_processed: 0,
            status: RunStatus::Completed,
            stop: None,
            blockers: Vec::new(),
            checks: Vec::new(),
            verifies: Vec::new(),
            trace: Vec::new(),
            provenance: agq_simulation::result::Provenance {
                runner: agq_simulation::RUNNER.into(),
                model_digest: String::new(),
                model_revision: 0,
                seed: 0,
                implementation: Some(agq_simulation::result::ImplementationProvenance {
                    repository: folder.to_string_lossy().into_owned(),
                    commit: String::new(),
                    dirty: false,
                    tree_digest: digest,
                    harness: String::new(),
                }),
                live: None,
                recordings: None,
                binding: None,
            },
            live: None,
        };
        assert!(code_current(&result, Some(&folder)));
        // The code changes: the pass no longer describes it.
        std::fs::write(
            folder.join("lib.rs"),
            "pub fn b() {}
",
        )
        .unwrap();
        assert!(!code_current(&result, Some(&folder)));
        // Without a repository to compare with, it stands (as in the Run panel).
        assert!(code_current(&result, None));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
