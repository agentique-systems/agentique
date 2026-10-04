//! Explore and Reproduce before Propose, in a cycle of an objective that
//! explores (C-54, ROADMAP §4.16, Scenario K2–K3, K8): the lead plans what
//! to explore (it may delegate an area to a child objective), the explorer
//! operates a test instance of the base build toward that goal, after
//! replaying first the findings fixed since; the most severe new findings
//! are reproduced and reduced; the testing knowledge keeps all of it. Two
//! explorations in a row that reproduce nothing new end the objective as
//! "nothing new reproduced", an outcome, not a failure.

use super::{Driver, Next, Watch};
use crate::control::{InstanceKey, Options};
use crate::decide::Way;
use crate::explore::{self, Changes, Plan, Run};
use crate::findings::{self, Finding, State as Found};
use crate::knowledge::Knowledge;
use crate::record::{Cost, Exploration, Exploring, Phase, Recipient, Scope};
use crate::roles::{self, Role};
use crate::thread::{Author, Kind, ThreadEntry};
use agq_assistant::turn::Toolset;
use agq_providers::{Credential, ModelRef};
use std::sync::Arc;
use std::time::Duration;

/// The start states explorations alternate between (C-54): a copy of the
/// URL shortener sample, and of Agentique's own model as a project.
pub const STARTS: [&str; 2] = ["models/url-shortener", "model"];

/// The new findings a cycle reproduces at most, most severe first.
pub const REPRODUCED: usize = 3;

/// Replays a finding's reproduction and reduction may take.
const REPLAYS: usize = 6;

/// The ways of deciding successive explorations take: Jev escalating when
/// unsure, then the explorer's model, then the rules.
const WAYS: [Way; 3] = [Way::Escalating, Way::Model, Way::Rules];

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

/// A finding as the lead reads it: its check, steps, reduced steps,
/// evidence and build (bounded).
fn finding_text(id: &str, finding: &Finding) -> String {
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
        "{id}: {}\n  check: {}; build {}; start {}\n  steps:\n{}{}\n  evidence: {evidence}",
        finding_line(finding),
        finding.check.name(),
        finding.build,
        finding.start,
        steps(&finding.steps),
        match &finding.reduced {
            Some(reduced) => format!("\n  reduced to:\n{}", steps(reduced)),
            None => String::new(),
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
            stand_in: false,
            key: self.instance_key(),
            condition: None,
        }
    }

    /// Explore: the lead's plan, the fixed findings replayed first, then
    /// the explorer's run in a test instance of the base build.
    pub(super) fn explore(&mut self) -> Next {
        let base = self.cycle_base()?;
        let goal = self.plan_exploration()?;
        let built = self.base_build()?;
        let checkout = self.checkout("base", &base)?;
        let repository = self.objective.repository.clone();
        let file = Knowledge::file(&self.setup.store, &repository);
        let project = Knowledge::key(&repository);
        let knowledge = Knowledge::load(&file, &project)?;
        let options = self.explore_options();
        // First, the findings fixed since: one that fails again is a
        // regression, reproduced with the new findings.
        let mut regressions = Vec::new();
        for fixed in knowledge.to_replay(&built.build) {
            if self.controls.stopped() {
                return Err("stopped".into());
            }
            let start = checkout.join(&fixed.start);
            let mut instance = self.setup.studios.instance(
                &built.exe,
                &start,
                &self.folder("replay"),
                options.clone(),
            );
            let controls = self.controls.clone();
            let replay = findings::replay(instance.as_mut(), fixed, &mut || controls.stopped());
            drop(instance);
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
        let made: u32 = self
            .objective
            .cycles
            .iter()
            .map(|c| c.explorations.len() as u32)
            .sum();
        let start = STARTS[((self.cycle().n + n) % 2) as usize];
        let changes = self.changes_since(knowledge.last_commit(), &base);
        let left_usd = (self.objective.budgets.usd - self.objective.spent.usd).max(0.0);
        let left_seconds =
            (self.objective.budgets.hours * 3600.0 - self.objective.spent.seconds).max(60.0);
        // Without models for exploring (an objective recorded without them),
        // the rules decide: they ask no model.
        let modelled = crate::models::with_deciding(&self.objective.models, |_| ()).is_ok();
        let plan = Plan {
            goal: goal.clone(),
            way: if modelled {
                WAYS[made as usize % WAYS.len()]
            } else {
                Way::Rules
            },
            seed: seed(&self.objective.id, self.cycle().n, n),
            steps: self.objective.budgets.steps,
            seconds: (left_seconds as u64).min(EXPLORE_SECONDS),
            usd: left_usd.min(self.objective.budgets.usd / 4.0),
            changes,
            start: start.to_string(),
            conversation: options.key.is_some(),
            turn_ms: findings::TURN_BUDGET_MS,
            stop_ms: findings::STOP_BUDGET_MS,
        };
        self.post(
            ThreadEntry::new(
                Kind::Event,
                Author::agent("explorer", self.model_of("explorer")),
                format!(
                    "Explores {} from a copy of {start}: {} ({} steps, {:?}{})",
                    built.build,
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
                "Recent changes: {}",
                if plan.changes.paths.is_empty() {
                    "none known".to_string()
                } else {
                    plan.changes.subjects.join("; ")
                }
            )),
        );
        let mut instance = self.setup.studios.instance(
            &built.exe,
            &checkout.join(start),
            &self.folder("explore"),
            options,
        );
        let mut watch = Watch {
            controls: self.controls.clone(),
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
        // The build and commit explored, as the Orchestrator chose them (a
        // debug build may not know its own commit).
        let mut run = run;
        run.build = built.build.clone();
        run.commit = built.commit.clone();
        Knowledge::change(&file, &project, |k| k.add_run(&run))?;
        let new: Vec<Finding> = run
            .findings
            .iter()
            .filter(|f| knowledge.findings.iter().all(|k| k.identity != f.identity))
            .cloned()
            .collect();
        let found: Vec<String> = new.iter().map(|f| f.identity.clone()).collect();
        let regressed: Vec<String> = regressions.iter().map(|f| f.identity.clone()).collect();
        self.post(
            ThreadEntry::new(
                Kind::Result,
                Author::agent("explorer", self.model_of("explorer")),
                format!(
                    "Explored {} steps: {} new coverage, {} new finding(s), {} regression(s); ${:.3}; {}",
                    run.actions,
                    run.new_coverage.len(),
                    new.len(),
                    regressions.len(),
                    run.usd,
                    run.ended
                ),
            )
            .with_details(
                new.iter()
                    .chain(&regressions)
                    .map(finding_line)
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

    /// The model the objective recorded for `role`.
    pub(super) fn model_of(&self, role: &str) -> Option<ModelRef> {
        self.objective
            .models
            .iter()
            .find(|m| m.role == role)
            .map(|m| m.model.clone())
    }

    /// An exploration's spend, by the role whose model decided: Jev's under
    /// `decisions`, the explorer's model under `explorer`, an escalation
    /// under `escalation`, and the instance's Assistant (on the explorer's
    /// key) under `explorer`.
    fn count_exploration(&mut self, run: &Run) {
        for taken in &run.steps {
            let Some(chosen) = &taken.chosen else {
                continue;
            };
            let role = match chosen.decision.source {
                crate::decide::Source::Rules => continue,
                crate::decide::Source::Jev => "decisions",
                crate::decide::Source::Model => "explorer",
                crate::decide::Source::Escalated => "escalation",
            };
            let Some(model) = self.model_of(role) else {
                continue;
            };
            self.objective.spent.add(
                role,
                &model,
                Cost {
                    usd: chosen.counted,
                    tokens: 0,
                    unknown: chosen.decision.usd.is_none(),
                },
            );
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
            let mut instance = self.setup.studios.instance(
                &built.exe,
                &checkout.join(&finding.start),
                &self.folder("reproduce"),
                options.clone(),
            );
            let controls = self.controls.clone();
            findings::reproduce(instance.as_mut(), &mut finding, REPLAYS, &mut || {
                controls.stopped()
            });
            drop(instance);
            if self.controls.stopped() {
                return Err("stopped".into());
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

    /// The lead plans the exploration: what the explorer looks at (its
    /// directive to the explorer), after any child it delegates. Without a
    /// plan, the explorer's goal is the objective's intent. An objective two
    /// deep has no plan of its lead's: its goal is its intent.
    fn plan_exploration(&mut self) -> Result<String, String> {
        let n = self.cycle().n;
        let explored = self.cycle().explorations.len() + 1;
        let handed = format!("cycle-{n}/exploration-{explored}");
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
        let policy = self.policy(&lead, false, false);
        let context = format!(
            "Plan this cycle's exploration (exploration {explored} of cycle {n}).\n\n{}",
            self.testing_summary()
        );
        let brief = roles::brief(Role::Lead, &self.objective, &context);
        let kit = Toolset {
            system: roles::planning_instructions(),
            definitions: roles::lead_tools(true, self.may_delegate()),
        };
        let session = self.lead(&lead, policy, brief, kit, false)?;
        let goal = session
            .submitted
            .as_ref()
            .and_then(|v| v["goal"].as_str())
            .map(str::trim)
            .filter(|g| !g.is_empty())
            .map(str::to_string);
        let (goal, text) = match goal {
            Some(goal) => (goal.clone(), format!("Explore: {goal}")),
            None => (
                intent.clone(),
                "Explore toward the objective (the lead gave no other goal)".to_string(),
            ),
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
            String::new(),
        );
        Ok(goal)
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
            .map(|f| format!("- {} ({:?})", finding_line(f), f.state).to_lowercase())
            .collect();
        let runs: Vec<String> = knowledge
            .runs
            .iter()
            .rev()
            .take(5)
            .map(|r| {
                format!(
                    "- run {} in {} ({:?}): {} steps, {} new coverage, {} finding(s): {}",
                    r.n,
                    r.build,
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

    /// The cycle's reproduced findings, as the lead chooses among them:
    /// ids `f1`, `f2`, … with their identities, and the text of each.
    pub(super) fn offered_findings(&self) -> (Vec<(String, String)>, String) {
        let mut offered = Vec::new();
        let mut text = Vec::new();
        for (i, finding) in self
            .cycle()
            .findings
            .iter()
            .filter(|f| f.state == Found::Reproduced)
            .enumerate()
        {
            let id = format!("f{}", i + 1);
            text.push(finding_text(&id, finding));
            offered.push((id, finding.identity.clone()));
        }
        (offered, text.join("\n\n"))
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
