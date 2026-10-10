//! Delegation (C-54, ROADMAP §4.16 "Directives and delegation", Scenario
//! K4, requirement `ChildWorkBounded`): the lead's `delegate` tool, which
//! the Orchestrator validates (a budget within what is left, the lead's own
//! spend in its session counted, time left for it, at most two deep, one
//! child at a time, three in a turn of the lead and five in a cycle, never
//! more than the parent's permissions: an exploration child never pushes,
//! merges or adopts), records as a directive
//! (journaled, so a restart never makes a second child) and carries out by
//! running the child objective inside the parent's run, on the parent's
//! commands and events. The child's result (its findings, coverage and
//! spend) is recorded on the directive and in both threads, and waits for
//! the parent's lead as its next message: only that result, never a
//! transcript. Stopping the parent stops its children; a child interrupted
//! with its parent goes on with it, on the parent's base build, and what it
//! spent is counted in the parent's once (the directive keeps what was
//! counted); the Operator may stop a child alone.

use super::explore::Planner;
use super::{Driver, Poster, Session, With};
use crate::explore::Target;
use crate::findings::State as Found;
use crate::record::{
    Budgets, Cycle, DirectiveStatus, Exploring, MAX_DEPTH, Objective, Permissions, Recipient,
    RoleRef, Scope, State,
};
use crate::roles::{Planning, Role};
use crate::thread::{Author, Kind, ThreadEntry};
use crate::traceability::Elements;
use agq_assistant::policy::Policy;
use agq_assistant::turn::Toolset;
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

/// Children a lead may delegate in one of its turns, at most.
pub const DELEGATIONS: usize = 3;

/// Children an objective may delegate in one cycle, at most.
pub const PER_CYCLE: usize = 5;

/// The least time a child gets: with less left, none is delegated.
pub const CHILD_HOURS: f64 = 0.1;

/// Why the lead may not delegate now, if it may not: too deep, a child
/// running, nothing left of the spend or time budget, or as many children
/// as a turn or a cycle may have.
pub fn refusal(
    depth: u8,
    running: bool,
    usd_left: f64,
    hours_left: f64,
    in_turn: usize,
    in_cycle: usize,
) -> Option<String> {
    if depth >= MAX_DEPTH {
        Some(format!(
            "children nest at most {MAX_DEPTH} deep, and this objective is {depth} deep"
        ))
    } else if running {
        Some("one child runs at a time, and one is running".into())
    } else if usd_left <= 0.0 {
        Some("nothing is left of the spend budget".into())
    } else if hours_left < CHILD_HOURS {
        Some(format!(
            "less than {:.0} minutes of the time budget is left",
            CHILD_HOURS * 60.0
        ))
    } else if in_turn >= DELEGATIONS {
        Some(format!(
            "a lead delegates at most {DELEGATIONS} children in a turn, and this turn has"
        ))
    } else if in_cycle >= PER_CYCLE {
        Some(format!(
            "an objective delegates at most {PER_CYCLE} children in a cycle, and this cycle has"
        ))
    } else {
        None
    }
}

/// What the lead may delegate now, as its `delegate` tool checks it.
#[derive(Clone, Debug)]
pub(super) struct Bounds {
    /// Why it may not delegate at all, if it may not.
    pub refused: Option<String>,
    /// US dollars left of the objective's spend budget when the lead's
    /// session started (unbounded without one).
    pub usd: f64,
    /// The most steps a child's exploration may take.
    pub steps: u32,
}

impl Bounds {
    /// The child the lead asked for, checked: its instruction, focus,
    /// budget (within what is left after `spent_now`, the lead's own spend
    /// in its session so far), steps and target (the W13.7 repair: its
    /// project is one `planning` permits, the objective's target's when the
    /// lead names none), or why it is refused.
    pub(super) fn check(
        &self,
        input: &Value,
        spent_now: f64,
        planning: &Planning,
    ) -> Result<Asked, String> {
        if let Some(why) = &self.refused {
            return Err(why.clone());
        }
        let left = (self.usd - spent_now).max(0.0);
        let instruction = input["instruction"].as_str().unwrap_or_default().trim();
        if instruction.is_empty() {
            return Err("a child needs an instruction".into());
        }
        let usd = input["usd"]
            .as_f64()
            .ok_or("a child needs a budget, `usd`")?;
        if !(usd > 0.0 && usd <= left) {
            return Err(if left.is_finite() {
                format!(
                    "a child's budget is more than nothing and at most what is left of this objective's: ${left:.2}"
                )
            } else {
                "a child's budget is more than nothing".into()
            });
        }
        let steps = input["steps"].as_u64().ok_or("a child needs `steps`")? as u32;
        if steps == 0 || steps > self.steps {
            return Err(format!(
                "a child's exploration takes 1 to {} steps, this objective's own",
                self.steps
            ));
        }
        let target = planning
            .child(input["project"].as_str(), instruction)
            .map_err(|problem| format!("`project`: {problem}"))?;
        Ok(Asked {
            instruction: instruction.to_string(),
            focus: input["focus"]
                .as_str()
                .map(str::trim)
                .filter(|f| !f.is_empty())
                .map(str::to_string),
            usd,
            steps,
            target,
        })
    }
}

/// What the lead's turn hands over is checked against: a proposal against
/// the base commit's model (C-55), a plan of an exploration and a child's
/// project against the planner (the W13.7 repair).
pub(super) struct Against<'a> {
    pub model: Option<&'a Result<Elements, String>>,
    /// None where the lead may not delegate and does not plan.
    pub planner: Option<&'a Planner>,
}

/// A child the lead asked for, checked.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Asked {
    pub instruction: String,
    pub focus: Option<String>,
    pub usd: f64,
    pub steps: u32,
    /// What it explores (the W13.7 repair).
    pub target: Target,
}

impl Driver {
    /// Whether this objective's lead may delegate at all: it is less than
    /// two deep.
    pub(super) fn may_delegate(&self) -> bool {
        self.objective.depth < MAX_DEPTH
    }

    /// What the lead may delegate now.
    pub(super) fn bounds(&self) -> Bounds {
        let left = self.objective.budgets.usd_left(self.objective.spent.usd);
        let running = self.objective.directives.iter().any(|d| {
            matches!(&d.recipient, Recipient::Child(c) if !c.is_empty())
                && d.status == DirectiveStatus::Running
        });
        let in_cycle = self.objective.cycle().map_or(0, |cycle| {
            let delegation = format!("cycle-{}/delegation", cycle.n);
            self.objective
                .directives
                .iter()
                .filter(|d| d.refers_to.as_deref() == Some(delegation.as_str()))
                .count()
        });
        Bounds {
            refused: refusal(
                self.objective.depth,
                running,
                left,
                self.hours_left(),
                self.delegated,
                in_cycle,
            ),
            usd: left,
            steps: self.objective.budgets.steps,
        }
    }

    /// Hours left of the time budget; unbounded without one.
    fn hours_left(&self) -> f64 {
        self.objective
            .budgets
            .hours_left(self.objective.spent.seconds)
    }

    /// The lead's turn: its session, and for each child it delegates, the
    /// child's run and the lead's session again with the child's result.
    /// What waits for the lead (the Operator's messages, children's results)
    /// opens each of its sessions; each is offered the reproduced findings
    /// it may choose among, and what it hands over is checked `against` the
    /// base commit's model (a proposal, C-55) and the planner (a plan, a
    /// child's project). What it judged findings to be is recorded on the
    /// cycle after each session, and a plan it accepted becomes the
    /// objective's target at once, before any child starts (the W13.7
    /// repair).
    pub(super) fn lead(
        &mut self,
        cwd: &Path,
        policy: Policy,
        brief: String,
        kit: Toolset,
        resume: bool,
        against: Against,
    ) -> Result<Session, String> {
        let Against { model, planner } = against;
        let proposing = kit
            .definitions
            .as_array()
            .into_iter()
            .flatten()
            .any(|d| d["name"] == crate::roles::SUBMIT_PROPOSAL);
        // A child left running (the parent was interrupted with it) goes
        // on first; its result then waits for the lead.
        let running: Vec<(String, String)> = self
            .objective
            .directives
            .iter()
            .filter(|d| d.status == DirectiveStatus::Running)
            .filter_map(|d| match &d.recipient {
                Recipient::Child(child) => Some((d.id.clone(), child.clone())),
                Recipient::Role(_) => None,
            })
            .collect();
        for (directive, child) in running {
            if self.setup.store.load(&child).is_err() {
                // Recorded, but its objective was never made (Agentique
                // ended in between): it did not start.
                self.objective.settle(
                    &directive,
                    DirectiveStatus::Failed,
                    Some("its child objective was never started".into()),
                );
                self.save();
                continue;
            }
            self.run_child(&child)?;
        }
        let mut brief = brief;
        let mut resume = resume;
        // A new turn of the lead: its delegations are counted from none.
        self.delegated = 0;
        loop {
            // The reproduced findings it may choose among (a child's join
            // them when it ends).
            let offered = if proposing {
                self.offered_findings().0
            } else {
                Vec::new()
            };
            let waiting = self.for_the_lead();
            let opening = if waiting.is_empty() {
                brief.clone()
            } else {
                format!("{brief}\n\nWaiting for you:\n{waiting}")
            };
            let session = self.session(
                Role::Lead,
                cwd,
                policy.clone(),
                opening,
                resume,
                With {
                    test: None,
                    kit: Some(kit.clone()),
                    offered,
                    model,
                    planner,
                },
            )?;
            self.record_refused(&session);
            self.record_adjudicated(&session.adjudicated);
            // A plan it accepted is what the objective explores from now.
            let accepted = planner.and_then(|p| p.planning.borrow().target.clone());
            if accepted.is_some() && accepted != self.objective.target {
                self.objective.target = accepted;
                self.save();
            }
            let Some(asked) = session.delegated.clone() else {
                return Ok(session);
            };
            self.delegated += 1;
            self.delegate(asked)?;
            // A budget the child used up, or a stop, ends the turn here.
            if let Some((_, reason)) = self.over() {
                return Err(if self.controls.stopped() {
                    "stopped".into()
                } else {
                    reason
                });
            }
            brief = if self.delegated >= DELEGATIONS {
                format!(
                    "The child you delegated has ended (its result is below). You have delegated {DELEGATIONS} children in this turn, the most: go on without delegating again."
                )
            } else {
                "The child you delegated has ended; its result is below. Go on.".into()
            };
            if proposing {
                brief = format!("{brief}\n\n{}", self.findings_brief());
            }
            resume = true;
        }
    }

    /// The delegations the Orchestrator refused in the lead's session,
    /// recorded as refused directives (the thread already shows each).
    fn record_refused(&mut self, session: &Session) {
        for (input, reason) in &session.refused {
            let id = self.objective.direct(
                "lead",
                Recipient::Child(String::new()),
                Scope {
                    instruction: input["instruction"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    focus: input["focus"].as_str().map(str::to_string),
                    budgets: None,
                    permissions: None,
                },
                None,
            );
            self.objective.settle(
                &id,
                DirectiveStatus::Refused {
                    reason: reason.clone(),
                },
                None,
            );
        }
        if !session.refused.is_empty() {
            self.save();
        }
    }

    /// Carries out a delegation: the directive and the child objective are
    /// recorded once (journaled), then the child runs to its end.
    fn delegate(&mut self, asked: Asked) -> Result<(), String> {
        let id = self.id();
        let k = self
            .objective
            .directives
            .iter()
            .filter(|d| matches!(&d.recipient, Recipient::Child(c) if !c.is_empty()))
            .count()
            + 1;
        let child = format!("{id}-c{k}");
        // Within the time left (`bounds` refused one with too little), an
        // hour at most.
        let budgets = Budgets {
            usd: Some(asked.usd),
            cycles: 1,
            attempts: 2,
            hours: Some(self.hours_left().min(1.0)),
            steps: asked.steps,
            calls: self.objective.budgets.calls.clone(),
        };
        // Never more than the parent may: an exploration child pushes,
        // merges and adopts nothing.
        let permissions = Permissions::default();
        let recorded = self
            .objective
            .directives
            .iter()
            .any(|d| d.recipient == Recipient::Child(child.clone()));
        if !recorded {
            self.direct(
                "lead",
                Recipient::Child(child.clone()),
                Scope {
                    instruction: asked.instruction.clone(),
                    focus: asked.focus.clone(),
                    budgets: Some(budgets.clone()),
                    permissions: Some(permissions.clone()),
                },
                self.objective
                    .cycle()
                    .map(|c| format!("cycle-{}/delegation", c.n)),
                format!(
                    "Delegates a child objective ({child}): {}",
                    asked.instruction
                ),
                format!(
                    "Explores {}; budget ${:.2}, {} steps; permissions: explore only (no push, merge or adopt){}",
                    asked.target.line(),
                    asked.usd,
                    asked.steps,
                    asked
                        .focus
                        .as_ref()
                        .map(|f| format!("; focus: {f}"))
                        .unwrap_or_default()
                ),
            );
        }
        let parent = self.objective.clone();
        self.setup.store.once(&id, &format!("child-{child}"), || {
            if self.setup.store.load(&child).is_ok() {
                return Ok(child.clone());
            }
            let created = child_objective(&parent, &child, &asked, budgets, permissions);
            self.setup.store.save(&created)?;
            Ok(child.clone())
        })?;
        self.run_child(&child)
    }

    /// Runs child objective `child` inside this run until it ends (or is
    /// interrupted with its parent), then records its result on its
    /// directive and in both threads, for the lead's next turn, and counts
    /// its spend in this objective's.
    pub(super) fn run_child(&mut self, child: &str) -> Result<(), String> {
        let objective = self.setup.store.load(child)?;
        let directive = self
            .objective
            .directives
            .iter()
            .find(|d| d.recipient == Recipient::Child(child.to_string()))
            .map(|d| d.id.clone());
        // What of its spend is counted in this objective's already: after a
        // restart, only the rest is added.
        let spent_before = directive
            .as_ref()
            .and_then(|id| self.objective.directive(id))
            .map(|d| d.counted.clone())
            .unwrap_or_default();
        let controls = self.controls.child(child);
        let mut driver = Driver {
            setup: self.setup.clone(),
            objective,
            events: self.events.clone(),
            poster: Poster {
                objective: child.to_string(),
                ..self.poster.clone()
            },
            controls,
            last: Instant::now(),
            delegated: 0,
        };
        if driver.objective.active() {
            driver.run();
        }
        self.controls.forget(child);
        let ended = driver.objective;
        // Its spend is its parent's too (what it spent in this run).
        for (role, models) in &ended.spent.roles {
            for (model, cost) in models {
                let before = spent_before
                    .roles
                    .get(role)
                    .and_then(|m| m.get(model))
                    .copied()
                    .unwrap_or_default();
                let added = crate::record::Cost {
                    usd: cost.usd - before.usd,
                    tokens: cost.tokens.saturating_sub(before.tokens),
                    unknown: cost.unknown,
                };
                if let Some(model) = agq_providers::ModelRef::parse(model)
                    && (added.usd > 0.0 || added.tokens > 0)
                {
                    self.objective.spent.add(role, &model, added);
                }
            }
        }
        if let Some(id) = &directive
            && let Some(record) = self.objective.directives.iter_mut().find(|d| &d.id == id)
        {
            record.counted = ended.spent.clone();
        }
        self.save();
        if ended.active() {
            // Interrupted with its parent: it goes on when the parent does.
            return Err("stopped".into());
        }
        let status = match ended.state {
            State::Done => DirectiveStatus::Done,
            State::Stopped => DirectiveStatus::Stopped,
            _ => DirectiveStatus::Failed,
        };
        let result = match status {
            DirectiveStatus::Stopped => {
                format!("stopped by the Operator. {}", child_result(&ended))
            }
            _ => child_result(&ended),
        };
        if let Some(id) = &directive {
            self.objective
                .settle(id, status.clone(), Some(result.clone()));
        }
        // Its reproduced findings are this cycle's to choose among.
        if let Some(cycle) = ended.cycle() {
            let found: Vec<_> = cycle
                .findings
                .iter()
                .filter(|f| f.state == Found::Reproduced)
                .cloned()
                .collect();
            if let Some(mine) = self.objective.cycle_mut() {
                for finding in found {
                    if mine.findings.iter().all(|f| f.identity != finding.identity) {
                        mine.findings.push(finding);
                    }
                }
            }
        }
        self.save();
        let text = format!(
            "The child objective {child} {}",
            match status {
                DirectiveStatus::Done => "ended",
                DirectiveStatus::Stopped => "was stopped by the Operator",
                _ => "did not finish",
            }
        );
        // In the child's thread, and in this one for the lead's next turn.
        let mut theirs = ThreadEntry::new(Kind::Result, Author::Agentique, text.clone())
            .with_details(result.clone())
            .for_directive(directive.as_deref());
        let _ = self
            .setup
            .store
            .append_thread(child, theirs.clone())
            .map(|added| {
                let _ = self.events.send(super::Event::Thread(added));
            });
        theirs.to = Some("lead".into());
        self.post(theirs);
        Ok(())
    }
}

/// The child objective a delegation starts: explore only, within the
/// parent's budget and permissions, one deeper, on the parent's models,
/// with the target the delegation was checked to have (the W13.7 repair).
pub(super) fn child_objective(
    parent: &Objective,
    id: &str,
    asked: &Asked,
    budgets: Budgets,
    permissions: Permissions,
) -> Objective {
    let mut child = parent.clone();
    child.id = id.to_string();
    child.intent = match &asked.focus {
        Some(focus) => format!("{} (focus: {focus})", asked.instruction),
        None => asked.instruction.clone(),
    };
    child.created = agq_launcher::now();
    child.state = State::Running;
    child.budgets = budgets;
    child.permissions = permissions;
    // It explores the parent's base build, so what it reproduces is
    // reproduced there.
    let mut cycle = Cycle::new(1);
    cycle.exploring = Some(Exploring::Explore);
    if let Some(theirs) = parent.cycle() {
        cycle.base = theirs.base.clone();
        cycle.base_build = theirs.base_build.clone();
    }
    child.target = Some(asked.target.clone());
    child.cycles = vec![cycle];
    child.spent = Default::default();
    child.continuation = None;
    child.note = None;
    child.explore = true;
    child.parent = Some(parent.id.clone());
    child.depth = parent.depth + 1;
    child.requested_by = Some(RoleRef {
        role: "lead".into(),
        objective: parent.id.clone(),
    });
    child.directives = Vec::new();
    child.interrupted = false;
    child.resumes = 0;
    child.delivered = 0;
    child
}

/// A child's result as its parent's lead receives it: its reproduced
/// findings, its coverage and its spend; never its transcript.
pub(super) fn child_result(child: &Objective) -> String {
    let explorations: Vec<_> = child
        .cycles
        .iter()
        .flat_map(|c| c.explorations.iter())
        .collect();
    let findings: Vec<String> = child
        .cycles
        .iter()
        .flat_map(|c| c.findings.iter())
        .map(|f| {
            format!(
                "- {} ({})",
                super::explore::finding_line(f),
                match f.state {
                    Found::Reproduced => "reproduced",
                    Found::NotReproduced => "not reproduced",
                    _ => "not replayed",
                }
            )
        })
        .collect();
    format!(
        "{} exploration(s), {} steps, {} new coverage keys; findings:\n{}\nSpent ${:.3}.{}",
        explorations.len(),
        explorations.iter().map(|e| e.steps).sum::<u32>(),
        explorations.iter().map(|e| e.new_coverage).sum::<u32>(),
        if findings.is_empty() {
            "none".to_string()
        } else {
            findings.join("\n")
        },
        child.spent.usd,
        child
            .note
            .as_ref()
            .map(|n| format!(" {n}"))
            .unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ChildWorkBounded`: two deep at most, one child at a time, a budget
    /// and time left, three in a turn, five in a cycle; and a child's
    /// budget within what is left once the lead's own spend is counted.
    #[test]
    fn delegation_is_refused_beyond_its_bounds() {
        assert_eq!(refusal(0, false, 1.0, 1.0, 0, 0), None);
        assert_eq!(
            refusal(1, false, 1.0, 1.0, 0, 0),
            None,
            "a child may delegate"
        );
        let deep = refusal(2, false, 1.0, 1.0, 0, 0).unwrap();
        assert!(deep.contains("at most 2 deep"), "{deep}");
        assert!(
            refusal(0, true, 1.0, 1.0, 0, 0)
                .unwrap()
                .contains("one child")
        );
        assert!(refusal(0, false, 0.0, 1.0, 0, 0).unwrap().contains("spend"));
        assert!(refusal(0, false, 1.0, 0.05, 0, 0).unwrap().contains("time"));
        assert!(
            refusal(0, false, 1.0, 1.0, DELEGATIONS, 0)
                .unwrap()
                .contains("in a turn")
        );
        assert!(
            refusal(0, false, 1.0, 1.0, 0, PER_CYCLE)
                .unwrap()
                .contains("in a cycle")
        );
        let bounds = Bounds {
            refused: None,
            usd: 1.0,
            steps: 20,
        };
        let planning = Planning {
            projects: vec!["model".into(), "models/shop".into()],
            revision: "abc".into(),
            target: None,
            given: Vec::new(),
        };
        let asked = serde_json::json!({ "instruction": "Look", "usd": 0.6, "steps": 10, "project": "model" });
        assert!(bounds.check(&asked, 0.0, &planning).is_ok());
        let after = bounds.check(&asked, 0.5, &planning).unwrap_err();
        assert!(
            after.contains("$0.50"),
            "the lead's own spend counts: {after}"
        );
        let too_long = serde_json::json!({ "instruction": "Look", "usd": 0.1, "steps": 30 });
        assert!(bounds.check(&too_long, 0.0, &planning).is_err());
        // A child's project (the W13.7 repair): named before the objective
        // has a target, one of the base commit's; after, one of its own,
        // its own when none is named.
        let on = |project: Option<&str>| serde_json::json!({ "instruction": "Look", "usd": 0.1, "steps": 5, "project": project });
        let unbound = bounds.check(&on(None), 0.0, &planning).unwrap_err();
        assert!(unbound.contains("submit_exploration first"), "{unbound}");
        let named = bounds.check(&on(Some(r"models\shop/")), 0.0, &planning);
        assert_eq!(named.unwrap().target.project, "models/shop");
        let none = bounds.check(&on(Some("models/none")), 0.0, &planning);
        assert!(none.unwrap_err().contains("holds no model"));
        let mut targeted = planning.clone();
        targeted.accept(Target {
            project: "model".into(),
            revision: "abc".into(),
            goal: "g".into(),
            scope: Vec::new(),
            start: Default::default(),
            vary: Vec::new(),
        });
        let inherited = bounds.check(&on(None), 0.0, &targeted).unwrap();
        assert_eq!(inherited.target.project, "model");
        let other = bounds.check(&on(Some("models/shop")), 0.0, &targeted);
        let other = other.unwrap_err();
        assert!(other.contains("this objective explores model"), "{other}");
    }
}
