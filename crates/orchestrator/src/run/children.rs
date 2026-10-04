//! Delegation (C-54, ROADMAP §4.16 "Directives and delegation", Scenario
//! K4, requirement `ChildWorkBounded`): the lead's `delegate` tool, which
//! the Orchestrator validates (a budget within what is left, at most two
//! deep, one child at a time, never more than the parent's permissions: an
//! exploration child never pushes, merges or adopts), records as a directive
//! (journaled, so a restart never makes a second child) and carries out by
//! running the child objective inside the parent's run, on the parent's
//! commands and events. The child's result (its findings, coverage and
//! spend) is recorded on the directive and in both threads, and waits for
//! the parent's lead as its next message: only that result, never a
//! transcript. Stopping the parent stops its children; a child interrupted
//! with its parent goes on with it; the Operator may stop a child alone.

use super::{Driver, Poster, Session, With};
use crate::findings::State as Found;
use crate::record::{
    Budgets, DirectiveStatus, MAX_DEPTH, Objective, Permissions, Recipient, RoleRef, Scope, State,
};
use crate::roles::Role;
use crate::thread::{Author, Kind, ThreadEntry};
use agq_assistant::policy::Policy;
use agq_assistant::turn::Toolset;
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

/// Children a lead may delegate in one of its turns, at most.
const DELEGATIONS: usize = 3;

/// What the lead may delegate now, as its `delegate` tool checks it.
#[derive(Clone, Debug)]
pub(super) struct Bounds {
    /// Why it may not delegate at all, if it may not.
    pub refused: Option<String>,
    /// US dollars left of the objective's spend budget.
    pub usd: f64,
    /// The most steps a child's exploration may take.
    pub steps: u32,
}

impl Bounds {
    /// The child the lead asked for, checked: its instruction, focus,
    /// budget and steps, or why it is refused.
    pub(super) fn check(&self, input: &Value) -> Result<Asked, String> {
        if let Some(why) = &self.refused {
            return Err(why.clone());
        }
        let instruction = input["instruction"].as_str().unwrap_or_default().trim();
        if instruction.is_empty() {
            return Err("a child needs an instruction".into());
        }
        let usd = input["usd"]
            .as_f64()
            .ok_or("a child needs a budget, `usd`")?;
        if !(usd > 0.0 && usd <= self.usd) {
            return Err(format!(
                "a child's budget is more than nothing and at most what is left of this objective's: ${:.2}",
                self.usd
            ));
        }
        let steps = input["steps"].as_u64().ok_or("a child needs `steps`")? as u32;
        if steps == 0 || steps > self.steps {
            return Err(format!(
                "a child's exploration takes 1 to {} steps, this objective's own",
                self.steps
            ));
        }
        Ok(Asked {
            instruction: instruction.to_string(),
            focus: input["focus"]
                .as_str()
                .map(str::trim)
                .filter(|f| !f.is_empty())
                .map(str::to_string),
            usd,
            steps,
        })
    }
}

/// A child the lead asked for, checked.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Asked {
    pub instruction: String,
    pub focus: Option<String>,
    pub usd: f64,
    pub steps: u32,
}

impl Driver {
    /// Whether this objective's lead may delegate at all: it is less than
    /// two deep.
    pub(super) fn may_delegate(&self) -> bool {
        self.objective.depth < MAX_DEPTH
    }

    /// What the lead may delegate now.
    pub(super) fn bounds(&self) -> Bounds {
        let left = (self.objective.budgets.usd - self.objective.spent.usd).max(0.0);
        let running = self.objective.directives.iter().any(|d| {
            matches!(d.recipient, Recipient::Child(_)) && d.status == DirectiveStatus::Running
        });
        let refused = if !self.may_delegate() {
            Some(format!(
                "children nest at most {MAX_DEPTH} deep, and this objective is {} deep",
                self.objective.depth
            ))
        } else if running {
            Some("one child runs at a time, and one is running".into())
        } else if left <= 0.0 {
            Some("nothing is left of the spend budget".into())
        } else {
            None
        };
        Bounds {
            refused,
            usd: left,
            steps: self.objective.budgets.steps,
        }
    }

    /// The lead's turn: its session, and for each child it delegates, the
    /// child's run and the lead's session again with the child's result.
    /// What waits for the lead (the Operator's messages, children's results)
    /// opens each of its sessions. `offered` are the reproduced findings it
    /// may choose among.
    pub(super) fn lead(
        &mut self,
        cwd: &Path,
        policy: Policy,
        brief: String,
        kit: Toolset,
        resume: bool,
    ) -> Result<Session, String> {
        let proposing = kit
            .definitions
            .as_array()
            .into_iter()
            .flatten()
            .any(|d| d["name"] == crate::roles::SUBMIT_PROPOSAL);
        // A child left running (the parent was interrupted with it) goes
        // on first; its result then waits for the lead.
        let running: Vec<String> = self
            .objective
            .directives
            .iter()
            .filter(|d| d.status == DirectiveStatus::Running)
            .filter_map(|d| match &d.recipient {
                Recipient::Child(id) => Some(id.clone()),
                Recipient::Role(_) => None,
            })
            .collect();
        for child in running {
            self.run_child(&child)?;
        }
        let mut brief = brief;
        let mut resume = resume;
        let mut delegated = 0;
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
                },
            )?;
            self.record_refused(&session);
            let Some(asked) = session.delegated.clone() else {
                return Ok(session);
            };
            delegated += 1;
            self.delegate(asked)?;
            brief = if delegated >= DELEGATIONS {
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
        let budgets = Budgets {
            usd: asked.usd,
            cycles: 1,
            attempts: 2,
            hours: (self.objective.budgets.hours - self.objective.spent.seconds / 3600.0)
                .clamp(0.1, 1.0),
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
                None,
                format!(
                    "Delegates a child objective ({child}): {}",
                    asked.instruction
                ),
                format!(
                    "Budget ${:.2}, {} steps; permissions: explore only (no push, merge or adopt){}",
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
        let spent_before = objective.spent.clone();
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
        let result = child_result(&ended);
        let directive = self
            .objective
            .directives
            .iter()
            .find(|d| d.recipient == Recipient::Child(child.to_string()))
            .map(|d| d.id.clone());
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
/// parent's budget and permissions, one deeper, on the parent's models.
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
    child.cycles = Vec::new();
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
