//! UrlShortener::LinkScreening (an agent) and UrlShortener::BlocklistScreening
//! (its deterministic fallback). The agent's answer is used only when it
//! keeps the agent's contract: in time, a valid verdict, a confidence in
//! [0, 1] and at least `min_confidence`. Anything else is a failure, and the
//! fallback decides.

use crate::model::{Decision, LinkCandidate, Verdict};
use crate::ports::ScreeningPort;

/// What the agent's model gave back.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentAnswer {
    Answer { verdict: Verdict, latency_ms: u64 },
    Timeout,
    InvalidOutput,
    Refusal,
    ToolUnavailable,
}

/// The call to the agent's model (a provider client in production; the
/// harness stands in for it in scenarios).
pub trait AgentClient {
    fn ask(&mut self, instructions: &str, candidate: &LinkCandidate) -> AgentAnswer;
}

/// The agent's limits, as the model sets them (`Agents::Agent`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentPolicy {
    pub min_confidence: f64,
    pub max_latency_ms: u64,
}

impl Default for AgentPolicy {
    /// UrlShortener::LinkScreening's settings.
    fn default() -> Self {
        AgentPolicy {
            min_confidence: 0.8,
            max_latency_ms: 500,
        }
    }
}

/// The instructions: LinkScreening's documentation in the model.
pub const INSTRUCTIONS: &str = "Decides whether a new short link may go live. Answer allow for an ordinary link, review when unsure or when the site looks risky, and block for known abuse (malware, phishing). Give a confidence between 0 and 1 and a short reason.";

/// UrlShortener::BlocklistScreening
#[derive(Clone, Debug, PartialEq)]
pub struct BlocklistScreening {
    pub blocked_host: String,
}

impl Default for BlocklistScreening {
    fn default() -> Self {
        BlocklistScreening {
            blocked_host: "malware.example".into(),
        }
    }
}

impl ScreeningPort for BlocklistScreening {
    fn check(&mut self, candidate: &LinkCandidate) -> Verdict {
        if candidate.host == self.blocked_host {
            Verdict {
                decision: Decision::Block,
                confidence: Some(1.0),
                reason: Some("the host is on the blocklist".into()),
            }
        } else {
            Verdict {
                decision: Decision::Review,
                confidence: Some(1.0),
                reason: Some("screening was not available".into()),
            }
        }
    }
}

/// UrlShortener::LinkScreening: the agent with its fallback.
pub struct LinkScreening<C: AgentClient> {
    pub client: C,
    pub policy: AgentPolicy,
    pub fallback: BlocklistScreening,
}

impl<C: AgentClient> LinkScreening<C> {
    pub fn new(client: C, policy: AgentPolicy, fallback: BlocklistScreening) -> Self {
        LinkScreening {
            client,
            policy,
            fallback,
        }
    }

    /// The agent's verdict if it keeps its contract.
    fn accepted(&self, answer: AgentAnswer) -> Option<Verdict> {
        let AgentAnswer::Answer { verdict, latency_ms } = answer else {
            return None;
        };
        if latency_ms > self.policy.max_latency_ms {
            return None;
        }
        let confidence = verdict.confidence?;
        if !(0.0..=1.0).contains(&confidence) || confidence < self.policy.min_confidence {
            return None;
        }
        Some(verdict)
    }
}

impl<C: AgentClient> ScreeningPort for LinkScreening<C> {
    fn check(&mut self, candidate: &LinkCandidate) -> Verdict {
        let answer = self.client.ask(INSTRUCTIONS, candidate);
        match self.accepted(answer) {
            Some(verdict) => verdict,
            None => self.fallback.check(candidate),
        }
    }
}
