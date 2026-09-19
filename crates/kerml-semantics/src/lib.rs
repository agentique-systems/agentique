//! Pure, evidence-bearing queries over immutable normative KerML models.
//! See ADR 0005 for the precise rule boundary, completeness and invalidation contract.
#![forbid(unsafe_code)]

mod bindings;
mod context;
mod contract;
mod implicit;
mod inheritance;
mod namespaces;
mod queries;
mod resolution;
mod validation;

pub use bindings::*;
pub use context::*;
pub use contract::*;
pub use namespaces::{MemberAccess, MemberMatch};
pub use queries::KerMlQueries;
pub use resolution::*;
