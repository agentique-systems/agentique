//! Systems Modeling API 1.0 wire projections and immutable-revision pagination.
//!
//! The pinned OpenAPI and JSON Schema artifacts govern wire names. These values
//! are projections, never an independently mutable model or semantic authority.
#![forbid(unsafe_code)]

pub mod dto;
pub mod paging;
mod service;
pub use service::*;
