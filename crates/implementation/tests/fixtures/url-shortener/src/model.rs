//! The model's item and enum defs, as data types.

/// UrlShortener::LinkStatus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStatus {
    Active,
    Held,
    Blocked,
}

/// UrlShortener::Decision
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Review,
    Block,
}

/// UrlShortener::ResolveOutcome
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveOutcome {
    Redirect,
    Held,
    Unknown,
}

/// UrlShortener::ShortenRequest
#[derive(Clone, Debug, PartialEq)]
pub struct ShortenRequest {
    pub long_url: String,
    pub host: String,
}

/// UrlShortener::ShortLink
#[derive(Clone, Debug, PartialEq)]
pub struct ShortLink {
    pub code: String,
    pub long_url: String,
    pub status: LinkStatus,
}

/// UrlShortener::ResolveRequest
#[derive(Clone, Debug, PartialEq)]
pub struct ResolveRequest {
    pub code: String,
}

/// UrlShortener::Resolution
#[derive(Clone, Debug, PartialEq)]
pub struct Resolution {
    pub outcome: ResolveOutcome,
    pub location: String,
}

/// UrlShortener::ReviewDecision
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewDecision {
    pub code: String,
    pub approve: bool,
}

/// UrlShortener::LinkQuery
#[derive(Clone, Debug, PartialEq)]
pub struct LinkQuery {
    pub code: String,
}

/// UrlShortener::StatusChange
#[derive(Clone, Debug, PartialEq)]
pub struct StatusChange {
    pub code: String,
    pub status: LinkStatus,
}

/// UrlShortener::LinkRecord
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRecord {
    pub code: String,
    pub link: Option<ShortLink>,
}

/// UrlShortener::LinkCandidate
#[derive(Clone, Debug, PartialEq)]
pub struct LinkCandidate {
    pub long_url: String,
    pub host: String,
}

/// UrlShortener::Verdict (an Agents::AgentOutput: the model's own confidence)
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    pub decision: Decision,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
}
