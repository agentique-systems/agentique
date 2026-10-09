//! A requirement's evidence, kept apart by what each piece can claim
//! (C-55, ROADMAP §4.14 "What a result may claim"), strongest first when
//! it is summed up, and never stronger than it is:
//!
//! - declared: the `satisfy` relationships that name it, a claim and not
//!   evidence;
//! - calculated from the model: its constraints evaluated on the modelled
//!   configuration of each satisfying feature (an analysis of the model,
//!   not a test of a built system);
//! - scenarios: the verification defs whose objective verifies it, with
//!   their newest result per mode and whether it is still current; only a
//!   failed check, or a stop the model's own behaviour caused, is a
//!   failure, and anything undecided is inconclusive;
//! - implementation: the tests linked to it and their last outcome.
//!
//! Everything is computed from the model, the links and the kept results
//! when asked: the Requirements panel, the Inspector and the Assistant's
//! `check_requirements` read the same answer.

use crate::CheckReport;
use crate::checks::CheckKind;
use crate::links::{LinkKind, Links};
use agq_language::{Diagnostic, ElementId, ElementKind, Role, Tree, printed_reference};
use agq_simulation::requirements::{Evaluation, Status, evaluate_satisfies};
use agq_simulation::{Mode, RunStatus, StopReason, Verdict};

/// The newest result of a scenario in one mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioResult {
    pub mode: Mode,
    pub status: RunStatus,
    /// How many checks ended with each verdict.
    pub tally: Vec<(Verdict, usize)>,
    /// Why it stopped, when it stopped early.
    pub stop: Option<StopReason>,
    /// Whether it still describes the model (and, for an implementation
    /// run, the code) as they are now.
    pub current: bool,
}

/// What a scenario result says about the requirements it verifies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Every check passed, and it has checks.
    Passed,
    /// A check failed, or the model's own behaviour stopped it: why.
    Failed(String),
    /// Nothing decided: a limit, the harness, a missing recording, a
    /// budget, no checks, or checks not decided. Why.
    Inconclusive(String),
    /// A walkthrough: it shows the steps and verifies nothing.
    Shown,
}

/// Stops the model's own behaviour causes: a failure of the model, not of
/// the run.
const MODEL_STOPS: [StopReason; 3] = [
    StopReason::UnhandledMessage,
    StopReason::AmbiguousTransition,
    StopReason::MissingBehaviour,
];

impl ScenarioResult {
    fn count(&self, verdict: Verdict) -> usize {
        self.tally
            .iter()
            .filter(|(v, _)| *v == verdict)
            .map(|(_, n)| n)
            .sum()
    }

    /// What it says, whether or not it is current.
    pub fn outcome(&self) -> Outcome {
        if self.mode == Mode::Walkthrough || self.status == RunStatus::Walkthrough {
            return Outcome::Shown;
        }
        let failed = self.count(Verdict::Failed);
        if failed > 0 {
            return Outcome::Failed(format!("{failed} check(s) failed"));
        }
        if self.status == RunStatus::Stopped
            && let Some(stop) = self.stop.filter(|s| MODEL_STOPS.contains(s))
        {
            return Outcome::Failed(format!(
                "the model's behaviour stopped it ({})",
                stop.code()
            ));
        }
        let checks: usize = self.tally.iter().map(|(_, n)| n).sum();
        let passed = self.count(Verdict::Passed);
        match self.status {
            RunStatus::Completed if checks > 0 && passed == checks => Outcome::Passed,
            RunStatus::Completed if checks == 0 => Outcome::Inconclusive("it has no checks".into()),
            RunStatus::Completed => Outcome::Inconclusive(format!(
                "{} of its {checks} check(s) were not decided",
                checks - passed
            )),
            RunStatus::Stopped => Outcome::Inconclusive(format!(
                "it stopped before deciding ({})",
                self.stop.map_or("no reason recorded", StopReason::code)
            )),
            RunStatus::Blocked => Outcome::Inconclusive("it could not start".into()),
            RunStatus::Cancelled => Outcome::Inconclusive("it was cancelled".into()),
            RunStatus::Walkthrough => Outcome::Shown,
        }
    }

    /// It verified: every check passed, and it is current.
    pub fn passes(&self) -> bool {
        self.current && self.outcome() == Outcome::Passed
    }

    /// It found a failure, and it is current.
    pub fn fails(&self) -> bool {
        self.current && matches!(self.outcome(), Outcome::Failed(_))
    }

    /// It is current and decided nothing.
    pub fn inconclusive(&self) -> bool {
        self.current && matches!(self.outcome(), Outcome::Inconclusive(_))
    }

    /// One word for a chip: passed, failed, inconclusive, shown, outdated.
    pub fn word(&self) -> &'static str {
        if !self.current {
            return "outdated";
        }
        match self.outcome() {
            Outcome::Passed => "passed",
            Outcome::Failed(_) => "failed",
            Outcome::Inconclusive(_) => "inconclusive",
            Outcome::Shown => "shown",
        }
    }

    /// `Model execution: passed (current)`.
    pub fn describe(&self) -> String {
        let outcome = match self.outcome() {
            Outcome::Passed => "passed".to_string(),
            Outcome::Failed(why) => format!("failed: {why}"),
            Outcome::Inconclusive(why) => format!("inconclusive: {why}"),
            Outcome::Shown => "a walkthrough, which verifies nothing".to_string(),
        };
        let currency = if self.current {
            "current"
        } else {
            "outdated: it no longer describes the model or the code"
        };
        format!("{}: {outcome} ({currency})", self.mode.label())
    }
}

/// A scenario's newest results, one per mode (none: never run).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioRuns {
    pub scenario: ElementId,
    pub results: Vec<ScenarioResult>,
}

/// One rung of the ladder: a `satisfy` that names the requirement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declared {
    pub satisfy: ElementId,
    /// The satisfying feature, as written.
    pub by: String,
}

/// A calculation from the model: the requirement evaluated on the
/// configuration of a satisfying feature, or, for a subrequirement, within
/// the requirement that contains it.
#[derive(Clone, Debug, PartialEq)]
pub struct Calculated {
    pub evaluation: Evaluation,
    /// For a subrequirement: the satisfied requirement it was evaluated in,
    /// and its qualified name.
    pub within: Option<(ElementId, String)>,
}

/// A scenario that verifies the requirement and its newest results.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    pub scenario: ElementId,
    pub name: String,
    pub results: Vec<ScenarioResult>,
}

/// A test linked to the requirement and its last outcome, if it ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Test {
    pub location: String,
    /// The verdict, its message and whether the checks are still current;
    /// `None`: not run.
    pub outcome: Option<(Verdict, String, bool)>,
}

/// Everything there is to say about one requirement.
#[derive(Clone, Debug, PartialEq)]
pub struct Ladder {
    pub requirement: ElementId,
    pub name: String,
    /// A requirement def: a template whose usages carry the evidence.
    pub definition: bool,
    /// A subrequirement (owned by a requirement or requirement def):
    /// evaluated within what contains it, and counted with it.
    pub nested: bool,
    pub declared: Vec<Declared>,
    pub calculated: Vec<Calculated>,
    pub scenarios: Vec<Scenario>,
    pub tests: Vec<Test>,
    /// Whether the kept scenario results were available to read.
    pub runs_known: bool,
    /// Whether the implementation links were available to read.
    pub links_known: bool,
}

/// The strongest thing that can be said of a requirement usage, in the
/// order the headline counts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Standing {
    /// A calculation from the model shows a required constraint false.
    Violated,
    /// A current result of a scenario that verifies it failed.
    FailingScenario,
    /// A current result of a scenario that verifies it passed.
    Verified,
    /// Every calculation from the model that applies holds.
    Holds,
    /// Some calculations hold, others could not be calculated.
    HoldsPartly,
    /// Only `satisfy` says so: nothing calculated or verified concludes.
    OnlyDeclared,
    /// Only scenario results that decide nothing.
    Inconclusive,
    /// Nothing says anything about it yet.
    Nothing,
}

impl Standing {
    pub const ALL: [Standing; 8] = [
        Standing::Violated,
        Standing::FailingScenario,
        Standing::Verified,
        Standing::Holds,
        Standing::HoldsPartly,
        Standing::OnlyDeclared,
        Standing::Inconclusive,
        Standing::Nothing,
    ];

    /// What a row says, strongest evidence first.
    pub fn label(self) -> &'static str {
        match self {
            Standing::Violated => "violated by calculation",
            Standing::FailingScenario => "fails a current scenario",
            Standing::Verified => "verified by a current passing scenario",
            Standing::Holds => "holds by calculation",
            Standing::HoldsPartly => "holds for some calculations",
            Standing::OnlyDeclared => "only declared",
            Standing::Inconclusive => "only inconclusive scenario results",
            Standing::Nothing => "no evidence",
        }
    }

    /// The headline's words for `count` requirements.
    pub fn counted(self, count: usize) -> String {
        let words = match (self, count) {
            (Standing::Violated, _) => "violated by calculation",
            (Standing::FailingScenario, 1) => "fails a current scenario",
            (Standing::FailingScenario, _) => "fail a current scenario",
            (Standing::Verified, _) => "verified by a current passing scenario",
            (Standing::Holds, 1) => "holds by calculation",
            (Standing::Holds, _) => "hold by calculation",
            (Standing::HoldsPartly, 1) => "holds for some calculations",
            (Standing::HoldsPartly, _) => "hold for some calculations",
            (Standing::OnlyDeclared, _) => "only declared",
            (Standing::Inconclusive, _) => "with only inconclusive scenario results",
            (Standing::Nothing, _) => "with no evidence",
        };
        format!("{count} {words}")
    }
}

impl Ladder {
    /// The calculations that hold and those that apply (an evaluation
    /// whose assumptions are not met claims nothing and is left out).
    pub fn calculations(&self) -> (usize, usize) {
        let applying: Vec<Status> = self
            .calculated
            .iter()
            .map(|c| c.evaluation.status)
            .filter(|s| *s != Status::AssumptionsNotMet)
            .collect();
        let holding = applying.iter().filter(|s| **s == Status::Holds).count();
        (holding, applying.len())
    }

    /// The strongest thing that can be said, a violation or failure first.
    pub fn standing(&self) -> Standing {
        let results = || self.scenarios.iter().flat_map(|s| &s.results);
        let (holding, applying) = self.calculations();
        if self
            .calculated
            .iter()
            .any(|c| c.evaluation.status == Status::Violated)
        {
            Standing::Violated
        } else if results().any(ScenarioResult::fails) {
            Standing::FailingScenario
        } else if results().any(ScenarioResult::passes) {
            Standing::Verified
        } else if holding > 0 && holding == applying {
            Standing::Holds
        } else if holding > 0 {
            Standing::HoldsPartly
        } else if !self.declared.is_empty() {
            Standing::OnlyDeclared
        } else if results().any(ScenarioResult::inconclusive) {
            Standing::Inconclusive
        } else {
            Standing::Nothing
        }
    }

    /// The standing in words, with how many calculations hold when only
    /// some do.
    pub fn label(&self) -> String {
        match self.standing() {
            Standing::HoldsPartly => {
                let (holding, applying) = self.calculations();
                format!("holds for {holding} of {applying} calculations")
            }
            other => other.label().to_string(),
        }
    }

    /// The ladder in words, each rung on its own lines, for the Assistant
    /// (and anyone reading text).
    pub fn describe(&self) -> String {
        let mut lines = Vec::new();
        if self.definition {
            lines.push(format!(
                "{} (requirement def): a definition; its usages are satisfied, calculated and verified.",
                self.name
            ));
            return lines.join("\n");
        }
        lines.push(format!("{}: {}.", self.name, self.label()));
        if self.nested {
            lines.push(
                "  It is a subrequirement: it is evaluated within the requirement that contains it (in a requirement def, within each requirement that uses the def) and counted with it."
                    .into(),
            );
        }
        if self.declared.is_empty() {
            lines.push("  Declared: nothing satisfies it.".into());
        } else {
            let by: Vec<String> = self
                .declared
                .iter()
                .map(|d| format!("satisfy by {}", d.by))
                .collect();
            lines.push(format!(
                "  Declared (a claim, not evidence): {}.",
                by.join("; ")
            ));
        }
        if self.calculated.is_empty() {
            lines.push(
                "  Calculated from the model: nothing (no satisfy binds its subject).".into(),
            );
        } else {
            lines.push(
                "  Calculated from the model (its constraints on the modelled configuration; not a test of a built system):"
                    .into(),
            );
            for calculated in &self.calculated {
                if let Some((_, within)) = &calculated.within {
                    lines.push(format!("    within {within} (a subrequirement):"));
                }
                for line in calculated.evaluation.describe().lines() {
                    lines.push(format!("    {line}"));
                }
            }
        }
        if self.scenarios.is_empty() {
            lines.push("  Scenarios: none verifies it.".into());
        } else {
            for scenario in &self.scenarios {
                let results = if !self.runs_known {
                    "its results are not available here (the Studio keeps them)".to_string()
                } else if scenario.results.is_empty() {
                    "not run".to_string()
                } else {
                    scenario
                        .results
                        .iter()
                        .map(ScenarioResult::describe)
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                lines.push(format!(
                    "  Scenario {} verifies it: {results}.",
                    scenario.name
                ));
            }
        }
        if !self.links_known {
            lines.push(
                "  Implementation: the implementation links are not available here (the Studio reads them).".into(),
            );
        } else if self.tests.is_empty() {
            lines.push("  Implementation: no tests are linked to it.".into());
        } else {
            for test in &self.tests {
                let outcome = match &test.outcome {
                    None => "not run".to_string(),
                    Some((verdict, message, current)) => format!(
                        "{} ({}){}",
                        verdict.label(),
                        if *current { "current" } else { "outdated" },
                        if message.is_empty() {
                            String::new()
                        } else {
                            format!(": {message}")
                        }
                    ),
                };
                lines.push(format!("  Linked test {}: {outcome}.", test.location));
            }
        }
        lines.join("\n")
    }

    /// The ladder in one line, strongest rung first: what calculated it,
    /// what verified it, what is only declared.
    pub fn summary(&self) -> String {
        if self.definition {
            return "a definition: its usages carry the evidence".into();
        }
        let mut parts = Vec::new();
        for calculated in &self.calculated {
            let evaluation = &calculated.evaluation;
            parts.push(format!(
                "calculated for {}: {}",
                evaluation.subject,
                evaluation.status.label()
            ));
        }
        for scenario in &self.scenarios {
            if !self.runs_known {
                parts.push(format!("{}: results not available here", scenario.name));
                continue;
            }
            let best = scenario
                .results
                .iter()
                .find(|r| r.fails())
                .or_else(|| scenario.results.iter().find(|r| r.passes()))
                .or(scenario.results.first());
            parts.push(match best {
                Some(result) => format!("{}: {}", scenario.name, result.describe()),
                None => format!("{}: not run", scenario.name),
            });
        }
        if !self.declared.is_empty() {
            let by: Vec<&str> = self.declared.iter().map(|d| d.by.as_str()).collect();
            parts.push(format!("declared: satisfy by {}", by.join(", ")));
        }
        if parts.is_empty() {
            return "nothing satisfies or verifies it yet".into();
        }
        parts.join(" · ")
    }
}

/// The headline over requirement usages, subrequirements left to the
/// requirements that contain them: how many stand where, the strongest
/// first, leaving out what no requirement is.
pub fn headline(ladders: &[Ladder]) -> String {
    let usages: Vec<&Ladder> = ladders
        .iter()
        .filter(|l| !l.definition && !l.nested)
        .collect();
    if usages.is_empty() {
        return "No requirement usages yet: a requirement def is satisfied and verified through its usages.".into();
    }
    let parts: Vec<String> = Standing::ALL
        .iter()
        .map(|standing| {
            (
                *standing,
                usages.iter().filter(|l| l.standing() == *standing).count(),
            )
        })
        .filter(|(_, count)| *count > 0)
        .map(|(standing, count)| standing.counted(count))
        .collect();
    format!("{} (of {})", parts.join(" · "), usages.len())
}

/// The ladder of every requirement (definitions too, marked), in document
/// order. `diagnostics` are the model's (`agq_language::validate`, or the
/// System State's); `links` the implementation links and `runs` the kept
/// scenario results, `None` where the caller cannot read them; `report`
/// the newest implementation checks and whether they are current.
pub fn ladders(
    tree: &Tree,
    diagnostics: &[Diagnostic],
    links: Option<&Links>,
    runs: Option<&[ScenarioRuns]>,
    report: Option<(&CheckReport, bool)>,
) -> Vec<Ladder> {
    let evaluations = evaluate_satisfies(tree, diagnostics);
    let satisfies: Vec<agq_language::ElementId> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::Satisfy)
        .collect();
    let mut out = Vec::new();
    for id in tree.walk() {
        let kind = tree[id].kind;
        if !matches!(kind, ElementKind::Requirement | ElementKind::RequirementDef) {
            continue;
        }
        let declared = satisfies
            .iter()
            .copied()
            .filter(|s| tree[*s].target.as_ref().and_then(|t| t.target()) == Some(id))
            .map(|satisfy| Declared {
                satisfy,
                by: tree[satisfy]
                    .by
                    .as_ref()
                    .map(|by| printed_reference(tree, satisfy, Role::By, by).to_string())
                    .unwrap_or_else(|| "nothing (no `by`)".into()),
            })
            .collect();
        let mut calculated = Vec::new();
        for evaluation in &evaluations {
            for (depth, found) in evaluation.walk().into_iter().enumerate() {
                if found.requirement == id {
                    calculated.push(Calculated {
                        evaluation: found.clone(),
                        within: (depth > 0).then(|| {
                            (
                                evaluation.requirement,
                                tree.qualified_name(evaluation.requirement),
                            )
                        }),
                    });
                }
            }
        }
        let scenarios = tree
            .walk()
            .into_iter()
            .filter(|s| tree[*s].kind == ElementKind::VerificationDef && verifies(tree, *s, id))
            .map(|scenario| Scenario {
                scenario,
                name: tree.qualified_name(scenario),
                results: runs
                    .unwrap_or_default()
                    .iter()
                    .find(|r| r.scenario == scenario)
                    .map(|r| r.results.clone())
                    .unwrap_or_default(),
            })
            .collect();
        let tests = links
            .map(|links| links.for_element(id))
            .unwrap_or_default()
            .into_iter()
            .filter(|link| link.kind == LinkKind::Test)
            .map(|link| {
                let location = link.location();
                let outcome = report.and_then(|(report, current)| {
                    report
                        .checks
                        .iter()
                        .find(|c| {
                            c.kind == CheckKind::LinkedTest
                                && c.elements.contains(&id.raw())
                                && c.location.as_deref() == Some(location.as_str())
                        })
                        .map(|c| (c.verdict, c.message.clone(), current))
                });
                Test { location, outcome }
            })
            .collect();
        let mut owner = tree[id].owner();
        let mut nested = false;
        while let Some(o) = owner {
            nested |= matches!(
                tree[o].kind,
                ElementKind::RequirementDef | ElementKind::Requirement
            );
            owner = tree[o].owner();
        }
        out.push(Ladder {
            requirement: id,
            name: tree.qualified_name(id),
            definition: kind == ElementKind::RequirementDef,
            nested: kind == ElementKind::Requirement && nested,
            declared,
            calculated,
            scenarios,
            tests,
            runs_known: runs.is_some(),
            links_known: links.is_some(),
        });
    }
    out
}

/// Whether a scenario's objective verifies `requirement`.
fn verifies(tree: &Tree, scenario: ElementId, requirement: ElementId) -> bool {
    tree[scenario].children().iter().any(|objective| {
        tree[*objective].kind == ElementKind::Objective
            && tree[*objective]
                .children()
                .iter()
                .any(|v| tree[*v].target.as_ref().and_then(|t| t.target()) == Some(requirement))
    })
}

/// What `check_requirements` answers: the headline, then the ladder of
/// each requirement (or only of `only`).
pub fn describe(tree: &Tree, ladders: &[Ladder], only: Option<ElementId>) -> String {
    let mut lines = Vec::new();
    match only {
        None => {
            if ladders.is_empty() {
                return "The model has no requirements yet.".into();
            }
            lines.push(format!("Requirements: {}.", headline(ladders)));
            lines.push(
                "Each requirement keeps apart what is declared (satisfy: a claim, not evidence), calculated from the model, verified by a scenario, and tested in code.".into(),
            );
            if ladders.iter().any(|l| !l.runs_known || !l.links_known) {
                lines.push(
                    "Kept scenario results and implementation links are not available here: the scenario and implementation rungs say nothing about them.".into(),
                );
            }
        }
        Some(id) if !ladders.iter().any(|l| l.requirement == id) => {
            return format!("`{}` is not a requirement.", tree.qualified_name(id));
        }
        Some(_) => {}
    }
    for ladder in ladders
        .iter()
        .filter(|l| only.is_none_or(|id| l.requirement == id))
    {
        lines.push(ladder.describe());
    }
    lines.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_language::{Source, parse};

    const MODEL: &str = "package D {
    private import ScalarValues::*;
    part def Battery { attribute mass : Real; }
    part def Drone { part battery : Battery; attribute mass : Real = battery.mass + 500; }
    requirement def MassLimit {
        subject s : Drone;
        attribute limit : Real;
        require constraint { s.mass <= limit }
    }
    part light : Drone { part :>> battery { attribute :>> mass = 1000; } }
    part heavy : Drone { part :>> battery { attribute :>> mass = 9000; } }
    part unknown : Drone;
    requirement lightMass : MassLimit { attribute :>> limit = 2000; }
    requirement heavyMass : MassLimit { attribute :>> limit = 2000; }
    requirement verifiedMass : MassLimit { attribute :>> limit = 2000; }
    requirement wordsOnly { doc /* It shall be pleasant to fly. */ }
    requirement unsatisfied : MassLimit;
    requirement mixed : MassLimit { attribute :>> limit = 2000; }
    requirement def Group {
        subject s : Drone;
        requirement part1 : MassLimit { attribute :>> limit = 2000; }
    }
    requirement group : Group;
    requirement outer : MassLimit {
        requirement inner : MassLimit { attribute :>> limit = 2000; }
    }
    satisfy outer by light;
    satisfy lightMass by light;
    satisfy heavyMass by heavy;
    satisfy wordsOnly by light;
    satisfy mixed by light;
    satisfy mixed by unknown;
    satisfy group by light;
    satisfy group by heavy;
    verification def FlyLight {
        subject drone : Drone;
        objective { verify verifiedMass; verify unsatisfied; }
    }
}";

    fn tree() -> Tree {
        let tree = parse(&[Source::new("d.sysml", MODEL)]);
        assert_eq!(agq_language::validate(&tree), []);
        tree
    }

    fn ladder<'a>(ladders: &'a [Ladder], tree: &Tree, name: &str) -> &'a Ladder {
        let id = tree.find(&format!("D::{name}")).unwrap();
        ladders.iter().find(|l| l.requirement == id).unwrap()
    }

    fn standing(ladders: &[Ladder], tree: &Tree, name: &str) -> Standing {
        ladder(ladders, tree, name).standing()
    }

    fn result(
        status: RunStatus,
        tally: &[(Verdict, usize)],
        stop: Option<StopReason>,
    ) -> ScenarioResult {
        ScenarioResult {
            mode: Mode::Model,
            status,
            tally: tally.to_vec(),
            stop,
            current: true,
        }
    }

    fn runs(tree: &Tree, result: ScenarioResult) -> Vec<ScenarioRuns> {
        vec![ScenarioRuns {
            scenario: tree.find("D::FlyLight").unwrap(),
            results: vec![result],
        }]
    }

    #[test]
    fn each_requirement_stands_on_its_strongest_evidence_and_no_further() {
        let tree = tree();
        let passed = result(RunStatus::Completed, &[(Verdict::Passed, 2)], None);
        let runs = runs(&tree, passed.clone());
        let ladders = ladders(&tree, &[], Some(&Links::default()), Some(&runs), None);
        assert_eq!(standing(&ladders, &tree, "lightMass"), Standing::Holds);
        assert_eq!(standing(&ladders, &tree, "heavyMass"), Standing::Violated);
        assert_eq!(
            standing(&ladders, &tree, "verifiedMass"),
            Standing::Verified
        );
        // A satisfy of a requirement with nothing to calculate is a claim.
        assert_eq!(
            standing(&ladders, &tree, "wordsOnly"),
            Standing::OnlyDeclared
        );
        // One satisfying feature holds, the other cannot be calculated.
        assert_eq!(standing(&ladders, &tree, "mixed"), Standing::HoldsPartly);
        assert_eq!(
            ladder(&ladders, &tree, "mixed").label(),
            "holds for 1 of 2 calculations"
        );
        // A def's subrequirement within two usages: one holds, one violated.
        assert_eq!(
            standing(&ladders, &tree, "Group::part1"),
            Standing::Violated
        );
        assert!(ladder(&ladders, &tree, "Group::part1").nested);
        // A usage's own subrequirement is counted with it, not apart.
        let inner = ladder(&ladders, &tree, "outer::inner");
        assert!(inner.nested);
        assert_eq!(inner.standing(), Standing::Holds);
        assert_eq!(standing(&ladders, &tree, "outer"), Standing::OnlyDeclared);
        assert_eq!(
            headline(&ladders),
            "2 violated by calculation · 2 verified by a current passing scenario · 1 holds by calculation · 1 holds for some calculations · 2 only declared (of 8)"
        );
        // An outdated pass verifies nothing.
        let outdated = super::tests::runs(
            &tree,
            ScenarioResult {
                current: false,
                ..passed
            },
        );
        let again = super::ladders(&tree, &[], Some(&Links::default()), Some(&outdated), None);
        assert_eq!(standing(&again, &tree, "verifiedMass"), Standing::Nothing);
    }

    #[test]
    fn only_a_failed_check_or_the_models_own_stop_is_a_failure() {
        let tree = tree();
        let standing_with = |result: ScenarioResult| {
            let runs = runs(&tree, result);
            let ladders = ladders(&tree, &[], Some(&Links::default()), Some(&runs), None);
            standing(&ladders, &tree, "verifiedMass")
        };
        let failed = result(
            RunStatus::Completed,
            &[(Verdict::Passed, 1), (Verdict::Failed, 1)],
            None,
        );
        assert_eq!(standing_with(failed), Standing::FailingScenario);
        let unhandled = result(
            RunStatus::Stopped,
            &[(Verdict::NotRun, 1)],
            Some(StopReason::UnhandledMessage),
        );
        assert_eq!(standing_with(unhandled.clone()), Standing::FailingScenario);
        assert_eq!(
            unhandled.describe(),
            "Model execution: failed: the model's behaviour stopped it (unhandled-message) (current)"
        );
        // A limit, an evaluation error, a missing recording: nothing decided.
        for stop in [
            StopReason::EventLimit,
            StopReason::EvaluationError,
            StopReason::MissingRecording,
            StopReason::HarnessFailed,
            StopReason::BudgetExhausted,
        ] {
            let stopped = result(RunStatus::Stopped, &[(Verdict::NotRun, 1)], Some(stop));
            assert_eq!(
                standing_with(stopped.clone()),
                Standing::Inconclusive,
                "{stop:?}"
            );
            assert!(stopped.describe().contains("inconclusive"));
        }
        // A run with no checks, or with undecided checks, verifies nothing.
        let empty = result(RunStatus::Completed, &[], None);
        assert_eq!(standing_with(empty.clone()), Standing::Inconclusive);
        assert_eq!(
            empty.describe(),
            "Model execution: inconclusive: it has no checks (current)"
        );
        let undecided = result(
            RunStatus::Completed,
            &[(Verdict::Passed, 1), (Verdict::Inconclusive, 1)],
            None,
        );
        assert_eq!(standing_with(undecided), Standing::Inconclusive);
        // An inconclusive result never outranks a declaration.
        let runs = runs(&tree, result(RunStatus::Completed, &[], None));
        let ladders = ladders(&tree, &[], Some(&Links::default()), Some(&runs), None);
        assert_eq!(
            standing(&ladders, &tree, "unsatisfied"),
            Standing::Inconclusive
        );
    }

    #[test]
    fn the_words_keep_declared_and_calculated_apart() {
        let tree = tree();
        let ladders = ladders(&tree, &[], Some(&Links::default()), Some(&[]), None);
        let heavy = tree.find("D::heavyMass").unwrap();
        let text = describe(&tree, &ladders, Some(heavy));
        assert!(
            text.starts_with("D::heavyMass: violated by calculation."),
            "{text}"
        );
        assert!(
            text.contains("Declared (a claim, not evidence): satisfy by heavy."),
            "{text}"
        );
        assert!(
            text.contains("Calculated from the model (its constraints on the modelled configuration; not a test of a built system):"),
            "{text}"
        );
        assert!(
            text.contains(
                "required constraint `s.mass <= limit`: false (s.mass = 9500, limit = 2000)"
            ),
            "{text}"
        );
        assert!(text.contains("Scenarios: none verifies it."), "{text}");
        assert!(
            text.contains("Implementation: no tests are linked to it."),
            "{text}"
        );
        let words = tree.find("D::wordsOnly").unwrap();
        let text = describe(&tree, &ladders, Some(words));
        assert!(text.starts_with("D::wordsOnly: only declared."), "{text}");
        assert!(text.contains("not evaluable"), "{text}");
        let all = describe(&tree, &ladders, None);
        assert!(
            all.starts_with("Requirements: 2 violated by calculation"),
            "{all}"
        );
        let def = tree.find("D::MassLimit").unwrap();
        assert!(describe(&tree, &ladders, Some(def)).contains("a definition"));
        // Without the Studio's results and links, it says so.
        let headless = super::ladders(&tree, &[], None, None, None);
        let verified = tree.find("D::verifiedMass").unwrap();
        let text = describe(&tree, &headless, Some(verified));
        assert!(
            text.contains("Scenario D::FlyLight verifies it: its results are not available here"),
            "{text}"
        );
        assert!(
            text.contains("the implementation links are not available here"),
            "{text}"
        );
    }
}
