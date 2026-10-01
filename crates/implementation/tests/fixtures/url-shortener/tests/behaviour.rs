//! The service's own tests: linked to the model's requirements and parts.

use url_shortener::model::{
    Decision, LinkCandidate, LinkStatus, ResolveOutcome, ResolveRequest, ReviewDecision,
    ShortenRequest, Verdict,
};
use url_shortener::screening::{AgentAnswer, AgentClient, AgentPolicy};
use url_shortener::service::UrlShortenerService;

struct Answers(Vec<AgentAnswer>);

impl AgentClient for Answers {
    fn ask(&mut self, _: &str, _: &LinkCandidate) -> AgentAnswer {
        if self.0.is_empty() {
            AgentAnswer::ToolUnavailable
        } else {
            self.0.remove(0)
        }
    }
}

fn verdict(decision: Decision, confidence: f64) -> AgentAnswer {
    AgentAnswer::Answer {
        verdict: Verdict {
            decision,
            confidence: Some(confidence),
            reason: None,
        },
        latency_ms: 100,
    }
}

fn request(host: &str) -> ShortenRequest {
    ShortenRequest {
        long_url: format!("https://{host}/x"),
        host: host.into(),
    }
}

#[test]
fn a_confident_allow_redirects() {
    let mut service = UrlShortenerService::new(Answers(vec![verdict(Decision::Allow, 0.95)]), AgentPolicy::default());
    let link = service.shorten(&request("news.example"));
    assert_eq!(link.status, LinkStatus::Active);
    let resolved = service.resolve(&ResolveRequest { code: link.code });
    assert_eq!(resolved.outcome, ResolveOutcome::Redirect);
}

#[test]
fn a_failed_or_unsure_screening_holds_the_link_until_approved() {
    for answer in [
        verdict(Decision::Allow, 0.5),
        verdict(Decision::Allow, 1.7),
        AgentAnswer::Timeout,
        AgentAnswer::Answer {
            verdict: Verdict {
                decision: Decision::Allow,
                confidence: Some(0.99),
                reason: None,
            },
            latency_ms: 900,
        },
    ] {
        let mut service = UrlShortenerService::new(Answers(vec![answer]), AgentPolicy::default());
        let link = service.shorten(&request("new.example"));
        assert_eq!(link.status, LinkStatus::Held);
        let before = service.resolve(&ResolveRequest { code: link.code.clone() });
        assert_eq!(before.outcome, ResolveOutcome::Held);
        let approved = service.review(&ReviewDecision {
            code: link.code.clone(),
            approve: true,
        });
        assert_eq!(approved.status, LinkStatus::Active);
    }
}

#[test]
fn a_block_stores_nothing_and_the_blocklist_blocks_when_the_agent_refuses() {
    let mut service = UrlShortenerService::new(Answers(vec![verdict(Decision::Block, 0.97)]), AgentPolicy::default());
    let link = service.shorten(&request("phish.example"));
    assert_eq!(link.status, LinkStatus::Blocked);
    assert_eq!(service.resolve(&ResolveRequest { code: link.code }).outcome, ResolveOutcome::Unknown);
    let mut service = UrlShortenerService::new(Answers(vec![AgentAnswer::Refusal]), AgentPolicy::default());
    assert_eq!(service.shorten(&request("malware.example")).status, LinkStatus::Blocked);
}
