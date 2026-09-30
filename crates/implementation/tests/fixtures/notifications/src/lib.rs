//! A notification service: a dispatcher that sends each notification to an
//! SMS gateway and retries a failed send with a growing backoff.

pub mod dispatcher;
pub mod gateway;
pub mod json;
pub mod model;
