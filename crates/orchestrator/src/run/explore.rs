//! Explore and Reproduce before Propose, in a cycle of an objective that
//! explores (C-54, ROADMAP §4.16, Scenario K2–K3, K8): the lead plans what
//! to explore (it may delegate an area to a child objective), the explorer
//! operates a test instance of the base build toward that goal, after
//! replaying first the findings fixed since; the most severe new findings
//! are reproduced and reduced; the testing knowledge keeps all of it. Two
//! explorations in a row that reproduce nothing new end the objective as
//! "nothing new reproduced", an outcome, not a failure.
//!
//! The lead's plan names what the objective explores (the W13.7 repair):
//! the project, checked to hold a model at the base commit, with the goal
//! and where useful the elements it is about and where to start. A plan the
//! lead's turn accepts is recorded on the objective at once; a child takes
//! it, or a project the lead names among those it permits (before the first
//! plan, the lead must name one); later explorations (also after an
//! adoption), replays and the evaluation's exploration keep to its
//! projects. An exploration without a plan is asked for once more, and
//! without one the cycle ends: nothing alternates between projects. The
//! copy each test instance opens, for an exploration or a replay, is
//! checked against the project at the build's commit before the first
//! action; a mismatch acts on nothing.

use super::{Driver, Next, Watch};
use crate::builds::short;
use crate::control::{InstanceKey, Options};
use crate::decide::Way;
use crate::explore::{self, Changes, Plan, Provenance, Run, Target};
use crate::findings::{self, Disposition, DispositionKind, Finding, State as Found};
use crate::knowledge::Knowledge;
use crate::record::{Cost, Exploration, Exploring, Phase, Recipient, Scope};
use crate::roles::{self, Role};
use crate::thread::{Author, Kind, ThreadEntry};
use crate::traceability;
use agq_assistant::turn::Toolset;
use agq_providers::{Credential, ModelRef};
use serde_json::Value;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// The project an objective's evaluation explores when the objective
/// explores nothing of its own (it has no target): the URL shortener
/// sample. An objective that explores always has its lead's target.
pub const SAMPLE: &str = "models/url-shortener";

/// What a step of a run spent, and the role it counts for: a model asked
/// for the explorer, Jev alone for the decisions. A step the rules took
/// after a call failed, timed out or was stopped counts what was tried
/// (the review of the W13.7 repair: it was dropped), and a decision made
/// as the run stopped counts though nothing was acted on.
fn spent_on(taken: &explore::Taken) -> Option<(&'static str, Cost)> {
    use crate::decide::Source;
    let chosen = taken.chosen.as_ref()?;
    let role = match chosen.decision.source {
        Source::Rules if chosen.counted <= 0.0 => return None,
        Source::Model | Source::Escalated => "explorer",
        _ if taken.timing.calls > 0 => "explorer",
        _ => "decisions",
    };
    Some((
        role,
        Cost {
            usd: chosen.counted,
            tokens: taken.timing.tokens.map_or(0, |t| t.input + t.output),
            unknown: chosen.decision.usd.is_none(),
        },
    ))
}

/// What the lead's turn checks a plan of an exploration and a child's
/// project against (C-54, the W13.7 repair): the planning (the base
/// commit's projects, the commit, and what the objective explores so far,
/// which a plan it accepts becomes at once), and where a project's model is
/// read to resolve a plan's names (a checkout of the base in which nothing
/// ran, and a scratch folder).
pub(super) struct Planner {
    pub planning: RefCell<roles::Planning>,
    /// Whether the lead's turn had a plan accepted.
    pub accepted: std::cell::Cell<bool>,
    checkout: PathBuf,
    scratch: PathBuf,
}

impl Planner {
    /// The lead's `submit_exploration`, checked (its names resolved in the
    /// project's model at the base commit) and accepted: what the objective
    /// explores from now; or why not.
    pub fn plan(&self, input: &Value) -> Result<Target, String> {
        let planned = roles::read_exploration(input, &self.planning.borrow())?;
        let names: Vec<&String> = planned
            .scope
            .iter()
            .chain(planned.start.select.iter())
            .collect();
        if !names.is_empty() {
            let model =
                traceability::project_model(&self.checkout, &planned.project, &self.scratch)?;
            let unknown = traceability::unknown(&model, names);
            if !unknown.is_empty() {
                return Err(format!(
                    "{} {} not an element of the model of {} at {}",
                    unknown.join(", "),
                    if unknown.len() == 1 { "is" } else { "are" },
                    planned.project,
                    short(&planned.revision)
                ));
            }
        }
        self.accepted.set(true);
        Ok(self.planning.borrow_mut().accept(planned))
    }

    /// What the lead is told of the projects it may name.
    fn brief(&self) -> String {
        let planning = self.planning.borrow();
        let projects = if planning.projects.is_empty() {
            "none".to_string()
        } else {
            planning.projects.join(", ")
        };
        match &planning.target {
            Some(target) => format!(
                "This objective explores {}, as its first plan recorded: `project` names {}, and a child explores it unless you name another of them.\nProjects at this commit: {projects}.",
                target.line(),
                target.projects().join(" or ")
            ),
            None => format!(
                "Projects at this commit (`project` names the one the objective is about; `model` is Agentique's own model): {projects}. A child you delegate before planning needs its `project`."
            ),
        }
    }
}

/// The new findings a cycle reproduces at most, most severe first.
pub const REPRODUCED: usize = 3;

/// Cycles of one objective that may try to fix the same finding.
pub const TRIES: usize = 2;

/// Replays a finding's reproduction and reduction may take.
const REPLAYS: usize = 6;

/// How long one exploration may take at most.
const EXPLORE_SECONDS: u64 = 1800;

/// A stable seed for an exploration: successive ones take other paths.
fn seed(objective: &str, cycle: u32, n: u32) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in format!("{objective}/{cycle}/{n}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// A finding in a line, for a brief or the thread.
pub fn finding_line(finding: &Finding) -> String {
    format!(
        "{} on {}: {}",
        finding.check.name(),
        if finding.control.is_empty() {
            "the instance"
        } else {
            finding.control.as_str()
        },
        finding.message
    )
}

/// A disposition in a few words: `judged a wrong expectation (why)`.
pub fn disposition_line(disposition: &Disposition) -> String {
    format!(
        "judged {}{} ({})",
        disposition.kind.name(),
        disposition
            .requirement
            .as_ref()
            .map(|r| format!(" against {r}"))
            .unwrap_or_default(),
        disposition.reason
    )
}

/// The lead's disposition of finding `id` as the thread shows it (C-55): an
/// ambiguous requirement as a question for the Operator.
pub(super) fn adjudication_entry(
    lead: Author,
    id: &str,
    finding: &Finding,
    disposition: &Disposition,
) -> ThreadEntry {
    let line = finding_line(finding);
    let text = match disposition.kind {
        DispositionKind::Defect => format!("Judges finding {id} a defect, to be fixed: {line}"),
        DispositionKind::WrongExpectation => format!(
            "Judges finding {id} a wrong expectation, not a defect; it is not offered again: {line}"
        ),
        DispositionKind::UnreliableReproduction => format!(
            "Judges finding {id} an unreliable reproduction; it is not offered again: {line}"
        ),
        DispositionKind::AmbiguousRequirement => format!(
            "A question for you: is finding {id} a defect? The requirement{} is ambiguous, so it is not proposed: {line}",
            disposition
                .requirement
                .as_ref()
                .map(|r| format!(" {r}"))
                .unwrap_or_default()
        ),
    };
    ThreadEntry::new(Kind::Result, lead, text).with_details(format!(
        "Reason: {}{}",
        disposition.reason,
        disposition
            .requirement
            .as_ref()
            .map(|r| format!("\nRequirement: {r}"))
            .unwrap_or_default()
    ))
}

/// A finding as the lead reads it: its check, steps, reduced steps,
/// evidence, build (bounded) and how it was judged, if it was.
fn finding_text(id: &str, finding: &Finding, disposition: Option<&Disposition>) -> String {
    let steps = |steps: &[explore::Step]| {
        steps
            .iter()
            .enumerate()
            .map(|(i, s)| {
                format!(
                    "  {}. on {}: {} {}",
                    i + 1,
                    s.screen,
                    s.action["kind"].as_str().unwrap_or("act"),
                    if s.label.is_empty() {
                        s.target().to_string()
                    } else {
                        format!("“{}”", s.label)
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut evidence = serde_json::to_string(&finding.evidence).unwrap_or_default();
    if evidence.len() > 1200 {
        evidence = crate::thread::capped(&evidence, 1200);
    }
    format!(
        "{id}: {}\n  check: {}; build {}; start {}\n  steps:\n{}{}\n  evidence: {evidence}{}",
        finding_line(finding),
        finding.check.name(),
        finding.build,
        finding.start,
        steps(&finding.steps),
        match &finding.reduced {
            Some(reduced) => format!("\n  reduced to:\n{}", steps(reduced)),
            None => String::new(),
        },
        match disposition {
            Some(d) => format!("\n  {}", disposition_line(d)),
            None if finding.check == findings::Check::Expectation => {
                "\n  not adjudicated: the explorer's own expectation".to_string()
            }
            None => "\n  not adjudicated".to_string(),
        }
    )
}

impl Driver {
    /// The commit the cycle starts from: the default branch here, as it is
    /// when the cycle starts (the adopted build's commit after an adoption).
    pub(super) fn cycle_base(&mut self) -> Result<String, String> {
        if let Some(base) = self.cycle().base.clone() {
            return Ok(base);
        }
        let repository = self.objective.repository.clone();
        let head = agq_execution::git::head(&repository).map_err(|e| e.to_string())?;
        if head.branch.as_deref() != Some(self.objective.base_branch.as_str()) {
            return Err(format!(
                "paused: the repository is on {}, not {}",
                head.branch.as_deref().unwrap_or("a detached commit"),
                self.objective.base_branch
            ));
        }
        self.cycle_mut().base = Some(head.commit.clone());
        self.save();
        Ok(head.commit)
    }

    /// The explorer's provider key for a test instance of a merged build
    /// (C-54, decided in session): its Assistant then answers for real,
    /// within the objective's spend. None when the explorer's model is on a
    /// Claude subscription (usable only in the Claude Agent runtime) or its
    /// key is not there.
    fn instance_key(&self) -> Option<InstanceKey> {
        let explorer = crate::models::for_role(&self.objective.models, "explorer").ok()?;
        if explorer.access != crate::record::Access::Key {
            return None;
        }
        let secret = (self.setup.credential)(Credential::Key(explorer.model.provider))?;
        Some(InstanceKey {
            model: explorer.model.clone(),
            secret: Arc::new(secret),
        })
    }

    /// How an exploration's test instance starts: at the Operator's speed,
    /// so they can watch, with the explorer's key when there is one (the
    /// explored build is the base, which is merged).
    fn explore_options(&self) -> Options {
        Options {
            speed: Some(self.setup.speed.clone()),
            key: self.instance_key(),
            ..Options::default()
        }
    }

    /// Explore: the lead's plan, the fixed findings replayed first, then
    /// the explorer's run in a test instance of the base build.
    pub(super) fn explore(&mut self) -> Next {
        let base = self.cycle_base()?;
        // Built before the lead plans: a child it delegates explores the
        // same build.
        let built = self.base_build()?;
        let goal = self.plan_exploration()?;
        let checkout = self.checkout("base", &base)?;
        let repository = self.objective.repository.clone();
        let file = Knowledge::file(&self.setup.store, &repository);
        let project = Knowledge::key(&repository);
        let knowledge = Knowledge::load(&file, &project)?;
        let options = self.explore_options();
        // First, the findings fixed since (of the projects it explores): one
        // that fails again is a regression, reproduced with the new
        // findings.
        let mut regressions = Vec::new();
        let to_replay: Vec<&Finding> = knowledge
            .to_replay(&built.build)
            .into_iter()
            .filter(|f| self.explores(&f.start))
            .collect();
        for fixed in to_replay {
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            // Its project gone from this base: it cannot be replayed here.
            let source = match self.copy_of(&checkout, &fixed.start, &built.commit) {
                Ok(source) => source,
                Err(why) => {
                    self.event(format!(
                        "A fixed finding could not be replayed in {}: {why}",
                        built.build
                    ));
                    continue;
                }
            };
            let mut instance = self.setup.studios.instance(
                &built.exe,
                &explore::within(&checkout, &fixed.start),
                &self.folder("replay"),
                options.clone(),
            );
            let controls = self.controls.clone();
            let replay = findings::replay(instance.as_mut(), fixed, Some(&source), &mut || {
                controls.stopped()
            });
            drop(instance);
            let replay = match replay {
                Ok(replay) => replay,
                Err(why) => {
                    return Err(self.not_replayed(
                        &format!("a fixed finding ({})", finding_line(fixed)),
                        &why,
                    ));
                }
            };
            Knowledge::change(&file, &project, |k| {
                k.replayed(&fixed.identity, &built.build, &replay)
            })?;
            match &replay {
                findings::Replay::Failed { .. } => {
                    self.post(ThreadEntry::event(format!(
                        "A fixed finding fails again in {}: {} (a regression)",
                        built.build,
                        finding_line(fixed)
                    )));
                    let mut again = fixed.clone();
                    again.build = built.build.clone();
                    again.commit = built.commit.clone();
                    again.set_state(
                        Found::FailingAgain,
                        &format!("fails again in build {}", built.build),
                    );
                    again.replays.clear();
                    regressions.push(again);
                }
                findings::Replay::Passed => self.event(format!(
                    "A fixed finding stays fixed in {}: {}",
                    built.build,
                    finding_line(fixed)
                )),
                findings::Replay::Diverged { reason, .. } => self.event(format!(
                    "A fixed finding could not be replayed in {}: {reason}",
                    built.build
                )),
            }
        }
        let n = self.cycle().explorations.len() as u32 + 1;
        // What the objective explores, at the build's commit; nothing
        // alternates (the W13.7 repair). A scope and start planned at
        // another commit are not taken over unchecked.
        let mut target = self
            .objective
            .target
            .clone()
            .ok_or("no project was planned for this exploration")?;
        let mut dropped = String::new();
        if target.revision != built.commit && !(target.scope.is_empty() && target.start.is_empty())
        {
            dropped = format!(
                "\nIts scope and start were planned at {}, not this commit: left out.",
                short(&target.revision)
            );
            target.scope.clear();
            target.start = Default::default();
        }
        target.revision = built.commit.clone();
        let start = target.project.clone();
        let source = self.copy_of(&checkout, &start, &built.commit)?;
        let changes = self.changes_since(knowledge.last_commit(), &base);
        let budgets = &self.objective.budgets;
        let left_usd = budgets.usd_left(self.objective.spent.usd);
        let left_seconds = (budgets.hours_left(self.objective.spent.seconds) * 3600.0).max(60.0);
        // A quarter of the spend budget at most; without one, the run's
        // steps bound it (`f64::MAX` keeps its record a number).
        let run_usd = left_usd.min(budgets.usd.map_or(f64::MAX, |usd| usd / 4.0));
        // Jev first, escalating only when unsure (the W13.7 repair: no
        // longer the explorer's model or the rules in turn); without models
        // for exploring (an objective recorded without them), the rules
        // decide: they ask no model.
        let modelled = crate::models::with_deciding(&self.objective.models, |_| ()).is_ok();
        let plan = Plan {
            goal: goal.clone(),
            way: if modelled {
                Way::Escalating
            } else {
                Way::Rules
            },
            seed: seed(&self.objective.id, self.cycle().n, n),
            steps: self.objective.budgets.steps,
            seconds: (left_seconds as u64).min(EXPLORE_SECONDS),
            usd: run_usd,
            changes,
            start: start.clone(),
            begin: target.start.clone(),
            source: Some(source.clone()),
            conversation: options.key.is_some(),
            turn_ms: findings::TURN_BUDGET_MS,
            stop_ms: findings::STOP_BUDGET_MS,
        };
        let explores = self.post(
            ThreadEntry::new(
                Kind::Event,
                Author::agent("explorer", self.model_of("explorer")),
                format!(
                    "Explores {} from a copy of {start} at {}: {} ({} steps, {:?}{})",
                    built.build,
                    short(&built.commit),
                    goal,
                    plan.steps,
                    plan.way,
                    if plan.conversation {
                        ", its Assistant on the explorer's key"
                    } else {
                        ""
                    }
                ),
            )
            .with_details(format!(
                "Explores {}{dropped}\nCopied from {} (the project's tree {}; its model files' digest {}).\nRecent changes: {}",
                target.line(),
                source.folder.display(),
                source.tree.as_deref().unwrap_or("not known"),
                source.digest,
                if plan.changes.paths.is_empty() {
                    "none known".to_string()
                } else {
                    plan.changes.subjects.join("; ")
                }
            )),
        );
        let mut instance = self.setup.studios.instance(
            &built.exe,
            &explore::within(&checkout, &start),
            &self.folder("explore"),
            options,
        );
        // Its progress folds under that entry (the W13.7 repair).
        let mut watch = Watch {
            controls: self.controls.clone(),
            report: Some(super::Report {
                poster: self.poster.clone(),
                author: Author::agent("explorer", self.model_of("explorer")),
                under: (explores.seq > 0).then_some(explores.seq),
                directive: self.objective.running_for("explorer").map(|d| d.id.clone()),
            }),
        };
        let run = if modelled {
            crate::models::with_deciding(&self.objective.models, |deciding| {
                explore::explore(instance.as_mut(), &plan, deciding, &knowledge, &mut watch)
            })?
        } else {
            let answers = crate::decide::Decider::default();
            explore::explore(
                instance.as_mut(),
                &plan,
                &super::by_rules(&answers),
                &knowledge,
                &mut watch,
            )
        };
        drop(instance);
        self.count_exploration(&run);
        if run.ended == "stopped" || self.controls.stopped() {
            self.save();
            return Err("stopped".into());
        }
        // Not the planned project: nothing was explored, and nothing else
        // takes its place (C-54, the W13.7 repair).
        if let Some(mismatch) = &run.mismatch {
            let why = mismatch.reason();
            self.post(
                ThreadEntry::new(
                    Kind::Result,
                    Author::agent("explorer", self.model_of("explorer")),
                    format!(
                        "Did not explore: the test instance did not open {start} at {} ({why})",
                        short(&built.commit)
                    ),
                )
                .with_details(format!(
                    "Planned: {start} at {} (digest {}), copied from {}.\nOpened: {}",
                    source.revision,
                    source.digest,
                    source.folder.display(),
                    match &run.opened {
                        Some(opened) => format!(
                            "{} from {} (digest {})",
                            opened.folder.display(),
                            opened.from.display(),
                            opened.digest
                        ),
                        None => "nothing the instance reported".to_string(),
                    }
                )),
            );
            return Err(format!(
                "the exploration did not open the planned project {start}: {why}"
            ));
        }
        // The build and commit explored, as the Orchestrator chose them (a
        // debug build may not know its own commit).
        let mut run = run;
        run.build = built.build.clone();
        run.commit = built.commit.clone();
        Knowledge::change(&file, &project, |k| k.add_run(&run))?;
        // Found again after it was adjudicated: not new (C-55).
        let adjudicated: Vec<String> = knowledge
            .already_adjudicated(&run.findings, &built.build)
            .into_iter()
            .map(|(f, d)| {
                format!(
                    "found again, already adjudicated: {} ({})",
                    finding_line(f),
                    disposition_line(d)
                )
            })
            .collect();
        let here: Vec<String> = self
            .cycle()
            .findings
            .iter()
            .map(|f| f.identity.clone())
            .collect();
        let new: Vec<Finding> = knowledge
            .new_findings(&run.findings, &built.build)
            .into_iter()
            .filter(|f| !here.contains(&f.identity))
            .map(|mut f| {
                // The build it was found in, as the Orchestrator chose it:
                // its reproduction is reused on the base only if that is it.
                f.build = built.build.clone();
                f.commit = built.commit.clone();
                f
            })
            .collect();
        // Known but never replayed: offered again, said apart from the new.
        let resurfaced = new
            .iter()
            .filter(|f| knowledge.findings.iter().any(|k| k.identity == f.identity))
            .count();
        let found: Vec<String> = new.iter().map(|f| f.identity.clone()).collect();
        let regressed: Vec<String> = regressions.iter().map(|f| f.identity.clone()).collect();
        self.post(
            ThreadEntry::new(
                Kind::Result,
                Author::agent("explorer", self.model_of("explorer")),
                format!(
                    "Explored {} steps: {} new coverage, {} new finding(s){}, {} regression(s); ${:.3}; {}",
                    run.actions,
                    run.new_coverage.len(),
                    new.len() - resurfaced,
                    if resurfaced > 0 {
                        format!(" and {resurfaced} found before but never replayed")
                    } else {
                        String::new()
                    },
                    regressions.len(),
                    run.usd,
                    run.ended
                ),
            )
            .with_details(
                std::iter::once(format!("Where the time went: {}", run.time()))
                    .chain(new.iter().chain(&regressions).map(finding_line))
                    .chain(adjudicated)
                    .chain(run.recoveries.iter().map(|r| format!("recovered: {} ({})", r.kind, r.detail)))
                    .chain(run.conditions.iter().map(|c| format!("condition: {c}")))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        );
        let cycle = self.cycle_mut();
        cycle.explorations.push(Exploration {
            n,
            build: built.build.clone(),
            start: start.to_string(),
            way: plan.way,
            seed: plan.seed,
            steps: run.actions,
            new_coverage: run.new_coverage.len() as u32,
            found,
            regressions: regressed,
            reproduced: 0,
            usd: run.usd,
            ended: run.ended.clone(),
        });
        cycle.findings.extend(new);
        cycle.findings.extend(regressions);
        cycle.exploring = Some(Exploring::Reproduce);
        Ok(Phase::Propose)
    }

    /// A replay whose test instance did not open the planned project (the
    /// W13.7 repair): said in the thread, the finding left as it was (it
    /// says nothing about it), and why the cycle ends.
    fn not_replayed(&self, what: &str, why: &explore::Mismatch) -> String {
        self.event(format!(
            "Did not replay {what}: {why}. The finding stays as it was."
        ));
        format!("{what} was not replayed: {why}")
    }

    /// The model the objective recorded for `role`.
    pub(super) fn model_of(&self, role: &str) -> Option<ModelRef> {
        self.objective
            .models
            .iter()
            .find(|m| m.role == role)
            .map(|m| m.model.clone())
    }

    /// An exploration's spend, by the role whose model decided: Jev's under
    /// `decisions`, the explorer's model under `explorer` (also when Jev
    /// escalated to it), and the instance's Assistant (on the explorer's
    /// key) under `explorer`.
    fn count_exploration(&mut self, run: &Run) {
        for (role, cost) in run.steps.iter().filter_map(spent_on) {
            let Some(model) = self.model_of(role) else {
                continue;
            };
            self.objective.spent.add(role, &model, cost);
        }
        if run.assistant_usd > 0.0
            && let Some(model) = self.model_of("explorer")
        {
            self.objective.spent.add(
                "explorer",
                &model,
                Cost {
                    usd: run.assistant_usd,
                    tokens: 0,
                    unknown: false,
                },
            );
        }
        self.save();
    }

    /// What changed on the base since `since` (the last explored commit),
    /// or in its latest commits when none is known: paths and subjects,
    /// bounded.
    fn changes_since(&self, since: Option<&str>, base: &str) -> Changes {
        let repository = &self.objective.repository;
        let range = match since {
            Some(since) if since != base => format!("{since}..{base}"),
            Some(_) => return Changes::default(),
            None => base.to_string(),
        };
        let mut words = vec!["git", "log", "--format=%x1e%s", "--name-only"];
        if since.is_none() {
            words.push("-10");
        }
        words.push(&range);
        let Ok(log) = crate::forge::run(repository, &words, Duration::from_secs(60)) else {
            return Changes::default();
        };
        let mut changes = Changes::default();
        for commit in log.stdout.split('\u{1e}').filter(|c| !c.trim().is_empty()) {
            let mut lines = commit.lines();
            if let Some(subject) = lines.next()
                && changes.subjects.len() < 30
            {
                changes.subjects.push(subject.trim().to_string());
            }
            for path in lines.map(str::trim).filter(|l| !l.is_empty()) {
                if changes.paths.len() < 200 && !changes.paths.iter().any(|p| p == path) {
                    changes.paths.push(path.to_string());
                }
            }
        }
        changes
    }

    /// Reproduce: the most severe new findings (and regressions), each
    /// replayed twice from a fresh start of the base build and reduced; the
    /// testing knowledge keeps what became of them.
    pub(super) fn reproduce(&mut self) -> Next {
        let base = self.cycle_base()?;
        let built = self.base_build()?;
        let checkout = self.checkout("base", &base)?;
        let repository = self.objective.repository.clone();
        let file = Knowledge::file(&self.setup.store, &repository);
        let project = Knowledge::key(&repository);
        let mut todo: Vec<usize> = (0..self.cycle().findings.len())
            .filter(|&i| {
                matches!(
                    self.cycle().findings[i].state,
                    Found::Open | Found::FailingAgain
                )
            })
            .collect();
        todo.sort_by_key(|&i| self.cycle().findings[i].check.severity());
        let done = self
            .cycle()
            .findings
            .iter()
            .filter(|f| matches!(f.state, Found::Reproduced | Found::NotReproduced))
            .count();
        todo.truncate(REPRODUCED.saturating_sub(done));
        let options = self.explore_options();
        for i in todo {
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            let mut finding = self.cycle().findings[i].clone();
            let source = self.copy_of(&checkout, &finding.start, &built.commit)?;
            let mut instance = self.setup.studios.instance(
                &built.exe,
                &explore::within(&checkout, &finding.start),
                &self.folder("reproduce"),
                options.clone(),
            );
            let controls = self.controls.clone();
            let reproduced = findings::reproduce(
                instance.as_mut(),
                &mut finding,
                REPLAYS,
                Some(&source),
                &mut || controls.stopped(),
            );
            drop(instance);
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            if let Err(why) = reproduced {
                return Err(self.not_replayed(
                    &format!("the reproduction of {}", finding_line(&finding)),
                    &why,
                ));
            }
            Knowledge::change(&file, &project, |k| k.update(&finding))?;
            let reproduced = finding.state == Found::Reproduced;
            self.post(
                ThreadEntry::event(format!(
                    "{}: {}",
                    if reproduced {
                        "Reproduced"
                    } else {
                        "Not reproduced"
                    },
                    finding_line(&finding)
                ))
                .with_details(format!(
                    "{} step(s){}; {}",
                    finding.steps.len(),
                    finding
                        .reduced
                        .as_ref()
                        .map(|r| format!(", reduced to {}", r.len()))
                        .unwrap_or_default(),
                    finding.note
                )),
            );
            self.cycle_mut().findings[i] = finding;
            self.save();
        }
        let reproduced = self
            .cycle()
            .findings
            .iter()
            .filter(|f| f.state == Found::Reproduced)
            .count() as u32;
        if let Some(last) = self.cycle_mut().explorations.last_mut() {
            last.reproduced = reproduced;
        }
        self.cycle_mut().exploring = None;
        if self.objective.parent.is_some() {
            // A child explores and reproduces; its result goes to its
            // parent's lead.
            return Ok(Phase::Done);
        }
        if reproduced > 0 {
            return Ok(Phase::Propose);
        }
        // Nothing new: a reproduced finding that no cycle has fixed, and
        // that this objective tried fewer than TRIES times, is reproduced
        // again on this base, and offered only if it still fails here (a
        // stale one costs no cycle; the knowledge learns it no longer
        // reproduces).
        let mut still = Vec::new();
        for mut known in self.untried().into_iter().take(REPRODUCED) {
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            known.build = built.build.clone();
            known.commit = built.commit.clone();
            known.state = Found::Open;
            known.replays.clear();
            known.reduced = None;
            let source = match self.copy_of(&checkout, &known.start, &built.commit) {
                Ok(source) => source,
                Err(why) => {
                    self.event(format!(
                        "A known finding could not be reproduced on this base: {why}"
                    ));
                    continue;
                }
            };
            let mut instance = self.setup.studios.instance(
                &built.exe,
                &explore::within(&checkout, &known.start),
                &self.folder("reproduce"),
                self.explore_options(),
            );
            let controls = self.controls.clone();
            let reproduced = findings::reproduce(
                instance.as_mut(),
                &mut known,
                REPLAYS,
                Some(&source),
                &mut || controls.stopped(),
            );
            drop(instance);
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            if let Err(why) = reproduced {
                return Err(self.not_replayed(
                    &format!("the reproduction of {}", finding_line(&known)),
                    &why,
                ));
            }
            Knowledge::change(&file, &project, |k| k.update(&known))?;
            if known.state == Found::Reproduced {
                still.push(known);
            } else {
                self.event(format!(
                    "A known finding no longer reproduces on this base: {}",
                    finding_line(&known)
                ));
            }
        }
        if !still.is_empty() {
            self.event(format!(
                "Nothing new reproduced; {} known finding(s) not yet fixed still fail here and are offered again",
                still.len()
            ));
            self.cycle_mut().findings.extend(still);
            self.save();
            return Ok(Phase::Propose);
        }
        if self.empty_explorations() >= 2 {
            // The loop ends the objective: nothing new reproduced.
            return Ok(Phase::Done);
        }
        if (self.cycle().explorations.len() as u32) < self.objective.budgets.attempts {
            self.event("Nothing reproduced: exploring again, from another start and way");
            self.cycle_mut().exploring = Some(Exploring::Explore);
            return Ok(Phase::Propose);
        }
        self.event("Nothing reproduced within the cycle's attempts");
        Ok(Phase::Done)
    }

    /// Reproduced findings the testing knowledge keeps that no cycle has
    /// fixed and this objective tried to fix fewer than [`TRIES`] times,
    /// not in the cycle yet, of the projects it explores: they stay
    /// eligible after a cycle that failed, once reproduced again on the
    /// cycle's base.
    pub(super) fn untried(&self) -> Vec<Finding> {
        let repository = &self.objective.repository;
        let Ok(knowledge) = Knowledge::load(
            &Knowledge::file(&self.setup.store, repository),
            &Knowledge::key(repository),
        ) else {
            return Vec::new();
        };
        let tried = |identity: &str| {
            self.objective
                .cycles
                .iter()
                .filter(|c| c.replay.as_ref().is_some_and(|r| r.identity == identity))
                .count()
        };
        let here: Vec<&str> = self
            .objective
            .cycle()
            .map(|c| c.findings.iter().map(|f| f.identity.as_str()).collect())
            .unwrap_or_default();
        knowledge
            .findings
            .iter()
            .filter(|f| matches!(f.state, Found::Reproduced | Found::FailingAgain))
            .filter(|f| !here.contains(&f.identity.as_str()) && tried(&f.identity) < TRIES)
            // Of the projects the objective explores (its target's).
            .filter(|f| self.explores(&f.start))
            // Judged a wrong expectation or an unreliable reproduction: not
            // offered again; an ambiguous requirement: not until the
            // Operator answers it (C-55).
            .filter(|f| {
                !f.disposition.as_ref().is_some_and(|d| {
                    d.sets_aside() || d.kind == DispositionKind::AmbiguousRequirement
                })
            })
            .cloned()
            .collect()
    }

    /// Explorations in a row, the latest last, that reproduced no new
    /// problem.
    pub(super) fn empty_explorations(&self) -> usize {
        self.objective
            .cycles
            .iter()
            .flat_map(|c| c.explorations.iter())
            .rev()
            .take_while(|e| e.reproduced == 0)
            .count()
    }

    /// The lead plans the exploration: what the objective explores (the
    /// project, the goal, where useful the elements it is about and where
    /// to start), checked when submitted and recorded on the objective as
    /// soon as it is accepted, and its directive to the explorer, after any
    /// child it delegates. A lead that plans nothing is asked once more
    /// (with why its plan was refused, if it was); without a plan, an
    /// objective that already explores a project goes on toward its intent,
    /// and one that does not ends the cycle (the W13.7 repair: no project
    /// is chosen for it). An objective two deep has no plan of its lead's:
    /// its goal is its intent, on the project it was given.
    fn plan_exploration(&mut self) -> Result<String, String> {
        let n = self.cycle().n;
        let explored = self.cycle().explorations.len() + 1;
        let handed = format!("cycle-{n}/exploration-{explored}");
        // Planned and handed over before a restart: not asked again (the
        // objective's target is that plan's).
        if let Some(directive) = self
            .objective
            .directives
            .iter()
            .find(|d| d.refers_to.as_deref() == Some(handed.as_str()))
        {
            return Ok(directive.scope.instruction.clone());
        }
        let intent = self.objective.intent.clone();
        if self.objective.depth >= crate::record::MAX_DEPTH {
            return Ok(intent);
        }
        let base = self.cycle_base()?;
        let lead = self.checkout("lead", &base)?;
        let planner = self.planner(&lead, &base)?;
        let policy = self.policy(&lead, false, false);
        let kit = Toolset {
            system: roles::planning_instructions(),
            definitions: roles::lead_tools(true, self.may_delegate()),
        };
        let mut brief = roles::brief(
            Role::Lead,
            &self.objective,
            &format!(
                "Plan this cycle's exploration (exploration {explored} of cycle {n}).\n\n{}\n\n{}",
                planner.brief(),
                self.testing_summary()
            ),
        );
        // Asked once more when it planned nothing, or only had its plans
        // refused; a budget used up or a stop ends the planning with why.
        let mut refused = None;
        for attempt in 0..2 {
            let session = self.lead(
                &lead,
                policy.clone(),
                brief,
                kit.clone(),
                attempt > 0,
                super::children::Against {
                    model: None,
                    planner: Some(&planner),
                },
            )?;
            if planner.accepted.get() {
                break;
            }
            if let Some((_, reason)) = self.over() {
                return Err(if self.controls.stopped() {
                    "stopped".into()
                } else {
                    reason
                });
            }
            refused = session.refusal;
            if refused.is_none() && self.objective.target.is_some() {
                break;
            }
            brief = format!(
                "You ended without a plan that was accepted{}. Plan this exploration with submit_exploration: an objective explores only the project its lead names.\n\n{}",
                refused
                    .as_ref()
                    .map(|why| format!(" (the last was refused: {why})"))
                    .unwrap_or_default(),
                planner.brief()
            );
        }
        let Some(target) = self.objective.target.clone() else {
            return Err(format!(
                "the lead planned no exploration{}: an objective explores only a project its lead names",
                refused
                    .map(|why| format!(" (its plan was refused: {why})"))
                    .unwrap_or_default()
            ));
        };
        let (goal, text) = if planner.accepted.get() {
            (target.goal.clone(), format!("Explore: {}", target.goal))
        } else {
            (
                intent,
                match refused {
                    Some(why) => format!(
                        "Explore toward the objective (the lead's plans were refused: {why})"
                    ),
                    None => {
                        "Explore toward the objective (the lead gave no other goal)".to_string()
                    }
                },
            )
        };
        self.direct(
            "lead",
            Recipient::Role("explorer".into()),
            Scope {
                instruction: goal.clone(),
                focus: None,
                budgets: None,
                permissions: None,
            },
            Some(handed),
            text,
            format!("Explores {}", target.line()),
        );
        Ok(goal)
    }

    /// What the lead's turn checks plans and children against: the base
    /// commit's projects, read by its tree, and what the objective explores
    /// so far; a project's model is read in `lead`, a checkout of `base`.
    pub(super) fn planner(&self, lead: &Path, base: &str) -> Result<Planner, String> {
        Ok(Planner {
            planning: RefCell::new(roles::Planning {
                projects: traceability::projects(&self.objective.repository, base)?,
                revision: base.to_string(),
                target: self.objective.target.clone(),
                given: Vec::new(),
            }),
            accepted: Default::default(),
            checkout: lead.to_path_buf(),
            scratch: self.folder("project-model"),
        })
    }

    /// Where a copy of `project` of `checkout` (a checkout of `revision`)
    /// comes from: its folder, its files' digest, and the project's tree in
    /// the repository when git knows it.
    pub(super) fn copy_of(
        &self,
        checkout: &Path,
        project: &str,
        revision: &str,
    ) -> Result<Provenance, String> {
        let mut source = Provenance::of(checkout, project, revision)?;
        source.tree = traceability::project_tree(&self.objective.repository, revision, project);
        Ok(source)
    }

    /// Whether the objective explores `project`: one of its target's, or
    /// any while it has none.
    pub(super) fn explores(&self, project: &str) -> bool {
        self.objective
            .target
            .as_ref()
            .is_none_or(|target| target.projects().contains(&project))
    }

    /// What the testing knowledge says, for the lead: coverage, the
    /// findings and their state, the latest runs.
    pub(super) fn testing_summary(&self) -> String {
        let repository = &self.objective.repository;
        let file = Knowledge::file(&self.setup.store, repository);
        let Ok(knowledge) = Knowledge::load(&file, &Knowledge::key(repository)) else {
            return "The testing knowledge cannot be read.".into();
        };
        let mut areas: std::collections::BTreeMap<String, u64> = Default::default();
        for (key, covered) in &knowledge.coverage {
            let area = key.split('|').nth(1).unwrap_or("other").to_string();
            *areas.entry(area).or_default() += covered.count;
        }
        let findings: Vec<String> = knowledge
            .findings
            .iter()
            .rev()
            .take(12)
            .map(|f| {
                format!(
                    "- {} ({}{})",
                    finding_line(f),
                    format!("{:?}", f.state).to_lowercase(),
                    f.disposition
                        .as_ref()
                        .map(|d| format!("; {}", disposition_line(d)))
                        .unwrap_or_default()
                )
            })
            .collect();
        let runs: Vec<String> = knowledge
            .runs
            .iter()
            .rev()
            .take(5)
            .map(|r| {
                format!(
                    "- run {} in {}{} ({:?}): {} steps, {} new coverage, {} finding(s): {}",
                    r.n,
                    r.build,
                    if r.start.is_empty() {
                        String::new()
                    } else {
                        format!(" on {}", r.start)
                    },
                    r.way,
                    r.steps,
                    r.new_coverage,
                    r.findings.len(),
                    r.goal
                )
            })
            .collect();
        format!(
            "Testing knowledge: {} coverage keys over {} runs; by area: {}.\nFindings, latest first:\n{}\nLatest runs:\n{}",
            knowledge.coverage.len(),
            knowledge.runs_made,
            if areas.is_empty() {
                "none yet".to_string()
            } else {
                areas
                    .iter()
                    .map(|(a, n)| format!("{a} {n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            if findings.is_empty() {
                "none".to_string()
            } else {
                findings.join("\n")
            },
            if runs.is_empty() {
                "none".to_string()
            } else {
                runs.join("\n")
            }
        )
    }

    /// The testing knowledge of the objective's project, or none yet.
    pub(super) fn knowledge(&self) -> Knowledge {
        let repository = &self.objective.repository;
        Knowledge::load(
            &Knowledge::file(&self.setup.store, repository),
            &Knowledge::key(repository),
        )
        .unwrap_or_else(|_| Knowledge::new(&Knowledge::key(repository)))
    }

    /// What each finding was judged to be (C-55), by identity: the testing
    /// knowledge's, and this cycle's own.
    pub(super) fn dispositions(&self) -> std::collections::BTreeMap<String, Disposition> {
        let mut dispositions = self.knowledge().dispositions();
        for finding in self.objective.cycle().map_or(&[][..], |c| &c.findings[..]) {
            if let Some(d) = &finding.disposition {
                dispositions.insert(finding.identity.clone(), d.clone());
            }
        }
        dispositions
    }

    /// The cycle's reproduced findings, as the lead chooses among them:
    /// ids `f1`, `f2`, … with their identities, and the text of each with
    /// how it was judged; none judged a wrong expectation or an unreliable
    /// reproduction (C-55).
    pub(super) fn offered_findings(&self) -> (Vec<(String, String)>, String) {
        let knowledge = self.knowledge();
        let dispositions = self.dispositions();
        let mut offered = Vec::new();
        let mut text = Vec::new();
        for (i, finding) in knowledge.offered(&self.cycle().findings) {
            // Its place among the cycle's findings: the same id in every
            // session of the cycle (C-55).
            let id = crate::knowledge::finding_id(i);
            text.push(finding_text(
                &id,
                finding,
                dispositions.get(&finding.identity),
            ));
            offered.push((id, finding.identity.clone()));
        }
        (offered, text.join("\n\n"))
    }

    /// Whether the cycle's reproduced findings were all judged other than a
    /// defect (C-55): there is nothing to fix.
    pub(super) fn nothing_to_fix(&self) -> bool {
        let dispositions = self.dispositions();
        let reproduced: Vec<&Finding> = self
            .cycle()
            .findings
            .iter()
            .filter(|f| f.state == Found::Reproduced)
            .collect();
        !reproduced.is_empty()
            && reproduced.iter().all(|f| {
                dispositions
                    .get(&f.identity)
                    .is_some_and(|d| d.kind != DispositionKind::Defect)
            })
    }

    /// Records on the cycle's findings what the lead judged them to be in
    /// its session (the testing knowledge has it already).
    pub(super) fn record_adjudicated(&mut self, adjudicated: &[(String, Disposition)]) {
        if adjudicated.is_empty() {
            return;
        }
        if let Some(cycle) = self.objective.cycle_mut() {
            for (identity, disposition) in adjudicated {
                for finding in cycle
                    .findings
                    .iter_mut()
                    .filter(|f| &f.identity == identity)
                {
                    finding.disposition = Some(disposition.clone());
                }
            }
        }
        self.save();
    }

    /// The Operator's messages to the lead and its children's results it
    /// has not been given yet (thread entries `to` the lead after the last
    /// delivered), now given: recorded as delivered, and said in the thread.
    pub(super) fn for_the_lead(&mut self) -> String {
        let id = self.id();
        let waiting: Vec<ThreadEntry> = self
            .setup
            .store
            .thread(&id, self.objective.delivered)
            .into_iter()
            .filter(|e| e.to.as_deref() == Some("lead"))
            .collect();
        let Some(last) = waiting.iter().map(|e| e.seq).max() else {
            return String::new();
        };
        self.objective.delivered = last;
        self.save();
        let messages: Vec<String> = waiting
            .iter()
            .map(|e| match e.kind {
                Kind::Human => format!("The Operator wrote: {}", e.text),
                _ => format!(
                    "{}{}",
                    e.text,
                    e.details
                        .as_ref()
                        .map(|d| format!("\n{d}"))
                        .unwrap_or_default()
                ),
            })
            .collect();
        let numbers: Vec<String> = waiting.iter().map(|e| format!("#{}", e.seq)).collect();
        self.event(format!(
            "Gave the lead {} ({})",
            if waiting.len() == 1 {
                "what was waiting for it"
            } else {
                "what was waiting for it, in order"
            },
            numbers.join(", ")
        ));
        messages.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decide::{Decision, Source};
    use crate::explore::{Chosen, Step, Taken, Timing, Tokens};

    fn taken(source: Source, counted: f64, usd: Option<f64>, calls: u32) -> Taken {
        Taken {
            step: Step {
                action: serde_json::json!({ "kind": "click", "control": "x" }),
                key: "k".into(),
                screen: "surface".into(),
                label: "X".into(),
                expect: None,
                by: "explorer".into(),
            },
            chosen: Some(Chosen {
                input: None,
                expect: None,
                why: String::new(),
                decision: Decision {
                    choice: String::new(),
                    source,
                    confidence: None,
                    millis: 0,
                    usd,
                    note: String::new(),
                },
                counted,
            }),
            outcome: "ok".into(),
            detail: String::new(),
            took_ms: None,
            timing: Timing {
                calls,
                tokens: (calls > 0).then_some(Tokens {
                    input: 900,
                    output: 100,
                    reasoning: 80,
                }),
                ..Timing::default()
            },
        }
    }

    /// The review of the W13.7 repair: what a step spent counts, also when
    /// the rules took it after a call failed or was stopped, with the
    /// tokens the provider reported; a rule that tried nothing counts
    /// nothing.
    #[test]
    fn what_a_step_spent_counts_for_the_role_that_spent_it() {
        let cost = |usd, tokens, unknown| Cost {
            usd,
            tokens,
            unknown,
        };
        assert_eq!(spent_on(&taken(Source::Rules, 0.0, Some(0.0), 0)), None);
        assert_eq!(
            spent_on(&taken(Source::Jev, 0.001, Some(0.001), 0)),
            Some(("decisions", cost(0.001, 0, false)))
        );
        assert_eq!(
            spent_on(&taken(Source::Escalated, 0.004, Some(0.004), 1)),
            Some(("explorer", cost(0.004, 1000, false)))
        );
        // A call that failed or was stopped: the rules took the step, and
        // the call counts at the most it could have cost.
        assert_eq!(
            spent_on(&taken(Source::Rules, 0.03, None, 1)),
            Some(("explorer", cost(0.03, 1000, true)))
        );
        // Jev failed, no model was asked.
        assert_eq!(
            spent_on(&taken(Source::Rules, 0.002, None, 0)),
            Some(("decisions", cost(0.002, 0, true)))
        );
    }
}
