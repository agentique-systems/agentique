//! SysML 2.0 semantics on the existing canonical kernel graph.
//!
//! Production contexts require the checked-in accepted KerML publication and its
//! exact protected dependency. Queries retain KerML evidence and original IDs;
//! inherited usages are never copied. Current-graph queries do not certify SysML
//! producer closure. Effective queries expose the remaining implications.
#![forbid(unsafe_code)]

mod bindings;
mod context;
mod context_identity;
mod producer_extension;
mod producers;
mod profile;
mod queries;

pub use bindings::*;
pub use context::*;
pub use context_identity::*;
pub use producer_extension::*;
pub use producers::*;
pub use profile::*;
pub use queries::*;

#[cfg(test)]
mod tests;
