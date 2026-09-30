//! The model's port defs, as the contracts parts use to talk to each other:
//! a part holds the other side of a connection, never the other part.

use crate::model::{LinkCandidate, LinkQuery, LinkRecord, ShortLink, StatusChange, Verdict};

/// UrlShortener::LinkStorePort, as its user sees it.
pub trait LinkStorePort {
    fn save(&mut self, link: ShortLink);
    fn query(&mut self, query: LinkQuery) -> LinkRecord;
    fn change(&mut self, change: StatusChange) -> LinkRecord;
}

/// UrlShortener::ScreeningPort, as its user sees it.
pub trait ScreeningPort {
    fn check(&mut self, candidate: &LinkCandidate) -> Verdict;
}
