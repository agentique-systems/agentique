//! Pure, evidence-bearing queries over immutable normative KerML models.
//! See ADR 0005 for the precise rule boundary, completeness and invalidation contract.
#![forbid(unsafe_code)]

mod bindings;
mod context;
mod contract;
mod cross_features;
mod featuring;
mod formal_targets;
mod implicit;
mod inheritance;
mod namespaces;
mod naming;
mod ordering;
mod queries;
mod resolution;
mod result_structure;
mod structural;
mod typing;
mod validation;

pub use bindings::*;
pub use context::*;
pub use contract::*;
pub use featuring::ConnectorFeaturing;
pub use formal_targets::*;
pub use namespaces::{MemberAccess, MemberMatch};
pub use naming::EffectiveNames;
pub use queries::KerMlQueries;
pub use resolution::*;
pub use result_structure::*;
pub use validation::RedefinitionEndConformance;
