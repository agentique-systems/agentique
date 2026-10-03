//! UrlShortener::LinkApi: shortens, resolves and reviews links. A new link
//! is screened first and its status follows the verdict: allow activates
//! it, review holds it, block stores nothing. Only an active link
//! redirects; a reviewer's approval activates a held link.

use crate::model::{
    Decision, LinkCandidate, LinkQuery, LinkStatus, Resolution, ResolveOutcome, ResolveRequest,
    ReviewDecision, ShortLink, ShortenRequest, StatusChange,
};
use crate::ports::{LinkStorePort, ScreeningPort};

pub struct LinkApi {
    pub next_code: String,
}

impl Default for LinkApi {
    fn default() -> Self {
        LinkApi {
            next_code: "abc123".into(),
        }
    }
}

impl LinkApi {
    /// The shorten port: screen, then store as the verdict says.
    pub fn shorten(
        &mut self,
        request: &ShortenRequest,
        screening: &mut dyn ScreeningPort,
        storage: &mut dyn LinkStorePort,
    ) -> ShortLink {
        let verdict = screening.check(&LinkCandidate {
            long_url: request.long_url.clone(),
            host: request.host.clone(),
        });
        let status = match verdict.decision {
            Decision::Allow => LinkStatus::Active,
            Decision::Review => LinkStatus::Held,
            Decision::Block => {
                return ShortLink {
                    code: String::new(),
                    long_url: request.long_url.clone(),
                    status: LinkStatus::Blocked,
                };
            }
        };
        let link = ShortLink {
            code: self.next_code.clone(),
            long_url: request.long_url.clone(),
            status,
        };
        storage.save(link.clone());
        link
    }

    /// The resolve port: only an active link redirects.
    pub fn resolve(
        &mut self,
        request: &ResolveRequest,
        storage: &mut dyn LinkStorePort,
    ) -> Resolution {
        let record = storage.query(LinkQuery {
            code: request.code.clone(),
        });
        match record.link {
            Some(link) if link.status == LinkStatus::Active => Resolution {
                outcome: ResolveOutcome::Redirect,
                location: link.long_url,
            },
            Some(_) => Resolution {
                outcome: ResolveOutcome::Held,
                location: String::new(),
            },
            None => Resolution {
                outcome: ResolveOutcome::Unknown,
                location: String::new(),
            },
        }
    }

    /// The review port: approval activates, rejection blocks.
    pub fn review(
        &mut self,
        decision: &ReviewDecision,
        storage: &mut dyn LinkStorePort,
    ) -> ShortLink {
        let record = storage.change(StatusChange {
            code: decision.code.clone(),
            status: if decision.approve {
                LinkStatus::Active
            } else {
                LinkStatus::Blocked
            },
        });
        record.link.unwrap_or(ShortLink {
            code: record.code,
            long_url: String::new(),
            status: LinkStatus::Blocked,
        })
    }
}
