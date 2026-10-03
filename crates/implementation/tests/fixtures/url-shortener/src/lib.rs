//! The URL shortener with AI screening (Scenario I), implemented from its
//! model `UrlShortener`: each module implements one part def and says so
//! in `model/links.json`; the shared data types and port contracts are the
//! model's item, enum and port defs. `jev` is the screening agent's client
//! for a typed decision model (`TypedLinkScreening`).

pub mod api;
pub mod jev;
pub mod json;
pub mod model;
pub mod ports;
pub mod screening;
pub mod service;
pub mod store;
