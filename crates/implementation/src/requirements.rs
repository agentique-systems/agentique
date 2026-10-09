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
//!   their newest result per mode and whether it is still current;
//! - implementation: the tests linked to it and their last outcome.
//!
//! Everything is computed from the model, the links and the kept results
//! when asked: the Requirements panel, the Inspector and the Assistant's
//! `check_requirements` read the same answer.

use crate::CheckReport;
use crate::checks::CheckKind;
use crate::links::{LinkKind, Links};
use agq_language::{ElementId, ElementKind, Role, Tree, printed_reference};
use agq_simulation::requirements::{Evaluation, Status, evaluate_all};
use agq_simulation::{Mode, RunStatus, Verdict};

/// The newest result of a scenario in one mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioResult {
    pub mode: Mode,
    pub status: RunStatus,
    pub all_passed: bool,
    /// Whether it still describes the model (and code) as they are now.
    pub current: bool,
}

impl ScenarioResult {
    /// It verified: it completed with every check passed, and is current.
    pub fn passes(&self) -> bool {
        self.current
            && self.status == RunStatus::Completed
            && self.all_passed
            && self.mode != Mode::Walkthrough
    }

    /// It found a failure and is current.
    pub fn fails(&self) -> bool {
        self.current
            && self.mode != Mode::Walkthrough
            && (self.status == RunStatus::Stopped
                || (self.status == RunStatus::Completed && !self.all_passed))
    }

    /// `Model execution: passed (current)`.
    pub fn describe(&self) -> String {
        let outcome = match self.status {
            RunStatus::Completed if self.all_passed => "passed",
            RunStatus::Completed => "failed",
            other => other.label(),
        };
        let currency = if self.current {
            "current"
        } else {
            "outdated: it no longer describes the model"
        };
        format!("{}: {outcome} ({currency})", self.mode.label())
    }
}

/// A scenario's newest results, one per mode (none: never run). The caller
/// says what it keeps; without kept results, every scenario is not run.
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
    /// For a subrequirement: the satisfied requirement it was evaluated in.
    pub within: Option<ElementId>,
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
    /// A subrequirement written in a requirement def: evaluated within
    /// each requirement that uses the def, and counted with them.
    pub in_definition: bool,
    pub declared: Vec<Declared>,
    pub calculated: Vec<Calculated>,
    pub scenarios: Vec<Scenario>,
    pub tests: Vec<Test>,
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
    /// The calculation from the model holds.
    Holds,
    /// Only `satisfy` says so: nothing calculated or verified concludes.
    OnlyDeclared,
    /// Nothing says anything about it yet.
    Nothing,
}

impl Standing {
    pub const ALL: [Standing; 6] = [
        Standing::Violated,
        Standing::FailingScenario,
        Standing::Verified,
        Standing::Holds,
        Standing::OnlyDeclared,
        Standing::Nothing,
    ];

    /// What a row says, strongest evidence first.
    pub fn label(self) -> &'static str {
        match self {
            Standing::Violated => "violated by calculation",
            Standing::FailingScenario => "fails a current scenario",
            Standing::Verified => "verified by a current passing scenario",
            Standing::Holds => "holds by calculation",
            Standing::OnlyDeclared => "only declared",
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
            (Standing::OnlyDeclared, _) => "only declared",
            (Standing::Nothing, _) => "with no evidence",
        };
        format!("{count} {words}")
    }
}

impl Ladder {
    /// The strongest thing that can be said, a violation or failure first.
    pub fn standing(&self) -> Standing {
        let statuses = || self.calculated.iter().map(|c| c.evaluation.status);
        let results = || self.scenarios.iter().flat_map(|s| &s.results);
        if statuses().any(|s| s == Status::Violated) {
            Standing::Violated
        } else if results().any(ScenarioResult::fails) {
            Standing::FailingScenario
        } else if results().any(ScenarioResult::passes) {
            Standing::Verified
        } else if statuses().any(|s| s == Status::Holds) {
            Standing::Holds
        } else if !self.declared.is_empty() {
            Standing::OnlyDeclared
        } else {
            Standing::Nothing
        }
    }

    /// The ladder in words, each rung on its own lines, for the Assistant
    /// (and anyone reading text).
    pub fn describe(&self, tree: &Tree) -> String {
        let mut lines = Vec::new();
        if self.definition {
            lines.push(format!(
                "{} (requirement def): a definition; its usages are satisfied, calculated and verified.",
                self.name
            ));
            return lines.join("\n");
        }
        lines.push(format!("{}: {}.", self.name, self.standing().label()));
        if self.in_definition {
            lines.push(
                "  It is part of a requirement def: it is evaluated within each requirement that uses the def, and counted with them."
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
                "  Calculated from the model: nothing (no satisfy to bind its subject).".into(),
            );
        } else {
            lines.push(
                "  Calculated from the model (its constraints on the modelled configuration; not a test of a built system):"
                    .into(),
            );
            for calculated in &self.calculated {
                if let Some(within) = calculated.within {
                    lines.push(format!(
                        "    within {} (a subrequirement):",
                        tree.qualified_name(within)
                    ));
                }
                for line in calculated.evaluation.describe(tree).lines() {
                    lines.push(format!("    {line}"));
                }
            }
        }
        if self.scenarios.is_empty() {
            lines.push("  Scenarios: none verifies it.".into());
        } else {
            for scenario in &self.scenarios {
                let results = if scenario.results.is_empty() {
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
        if self.tests.is_empty() {
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

/// The headline over requirement usages: how many stand where, the
/// strongest first, leaving out what no requirement is.
pub fn headline(ladders: &[Ladder]) -> String {
    let usages: Vec<&Ladder> = ladders
        .iter()
        .filter(|l| !l.definition && !l.in_definition)
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
/// order. `runs` are the kept scenario results the caller knows;
/// `report` the newest implementation checks and whether they are current.
pub fn ladders(
    tree: &Tree,
    links: &Links,
    runs: &[ScenarioRuns],
    report: Option<(&CheckReport, bool)>,
) -> Vec<Ladder> {
    let evaluations = evaluate_all(tree);
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
                        within: (depth > 0).then_some(evaluation.requirement),
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
                    .iter()
                    .find(|r| r.scenario == scenario)
                    .map(|r| r.results.clone())
                    .unwrap_or_default(),
            })
            .collect();
        let tests = links
            .for_element(id)
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
        let mut in_definition = false;
        while let Some(o) = owner {
            in_definition |= tree[o].kind == ElementKind::RequirementDef;
            owner = tree[o].owner();
        }
        out.push(Ladder {
            requirement: id,
            name: tree.qualified_name(id),
            definition: kind == ElementKind::RequirementDef,
            in_definition: kind == ElementKind::Requirement && in_definition,
            declared,
            calculated,
            scenarios,
            tests,
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
        lines.push(ladder.describe(tree));
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
    requirement lightMass : MassLimit { attribute :>> limit = 2000; }
    requirement heavyMass : MassLimit { attribute :>> limit = 2000; }
    requirement verifiedMass : MassLimit { attribute :>> limit = 2000; }
    requirement wordsOnly { doc /* It shall be pleasant to fly. */ }
    requirement unsatisfied : MassLimit;
    satisfy lightMass by light;
    satisfy heavyMass by heavy;
    satisfy wordsOnly by light;
    verification def FlyLight {
        subject drone : Drone;
        objective { verify verifiedMass; }
    }
}";

    fn tree() -> Tree {
        let tree = parse(&[Source::new("d.sysml", MODEL)]);
        assert_eq!(agq_language::validate(&tree), []);
        tree
    }

    fn standing(ladders: &[Ladder], tree: &Tree, name: &str) -> Standing {
        let id = tree.find(&format!("D::{name}")).unwrap();
        ladders
            .iter()
            .find(|l| l.requirement == id)
            .unwrap()
            .standing()
    }

    #[test]
    fn each_requirement_stands_on_its_strongest_evidence_and_no_further() {
        let tree = tree();
        let scenario = tree.find("D::FlyLight").unwrap();
        let passed = ScenarioResult {
            mode: Mode::Model,
            status: RunStatus::Completed,
            all_passed: true,
            current: true,
        };
        let runs = [ScenarioRuns {
            scenario,
            results: vec![passed.clone()],
        }];
        let ladders = ladders(&tree, &Links::default(), &runs, None);
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
        assert_eq!(standing(&ladders, &tree, "unsatisfied"), Standing::Nothing);
        assert_eq!(
            headline(&ladders),
            "1 violated by calculation · 1 verified by a current passing scenario · 1 holds by calculation · 1 only declared · 1 with no evidence (of 5)"
        );
        // An outdated pass verifies nothing.
        let outdated = [ScenarioRuns {
            scenario,
            results: vec![ScenarioResult {
                current: false,
                ..passed
            }],
        }];
        let again = super::ladders(&tree, &Links::default(), &outdated, None);
        assert_eq!(standing(&again, &tree, "verifiedMass"), Standing::Nothing);
    }

    #[test]
    fn the_words_keep_declared_and_calculated_apart() {
        let tree = tree();
        let ladders = ladders(&tree, &Links::default(), &[], None);
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
            all.starts_with("Requirements: 1 violated by calculation"),
            "{all}"
        );
        let def = tree.find("D::MassLimit").unwrap();
        assert!(describe(&tree, &ladders, Some(def)).contains("a definition"));
    }
}
