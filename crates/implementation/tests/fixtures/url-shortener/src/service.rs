//! UrlShortener::UrlShortenerService: the api, the screening agent (with
//! its fallback) and the store, connected as the model connects them.

use crate::api::LinkApi;
use crate::model::{Resolution, ResolveRequest, ReviewDecision, ShortLink, ShortenRequest};
use crate::screening::{AgentClient, AgentPolicy, BlocklistScreening, LinkScreening};
use crate::store::LinkStore;

pub struct UrlShortenerService<C: AgentClient> {
    pub api: LinkApi,
    pub screening: LinkScreening<C>,
    pub store: LinkStore,
}

impl<C: AgentClient> UrlShortenerService<C> {
    pub fn new(client: C, policy: AgentPolicy) -> Self {
        UrlShortenerService {
            api: LinkApi::default(),
            screening: LinkScreening::new(client, policy, BlocklistScreening::default()),
            store: LinkStore::default(),
        }
    }

    pub fn shorten(&mut self, request: &ShortenRequest) -> ShortLink {
        self.api.shorten(request, &mut self.screening, &mut self.store)
    }

    pub fn resolve(&mut self, request: &ResolveRequest) -> Resolution {
        self.api.resolve(request, &mut self.store)
    }

    pub fn review(&mut self, decision: &ReviewDecision) -> ShortLink {
        self.api.review(decision, &mut self.store)
    }
}
