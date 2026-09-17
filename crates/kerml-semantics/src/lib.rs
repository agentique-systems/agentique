//! Pure, evidence-bearing queries over immutable normative KerML models.
//! See ADR 0005 for the precise rule boundary, completeness and invalidation contract.
#![forbid(unsafe_code)]

mod context;
mod contract;
mod inheritance;
mod queries;

pub use context::*;
pub use contract::*;
pub use queries::KerMlQueries;
