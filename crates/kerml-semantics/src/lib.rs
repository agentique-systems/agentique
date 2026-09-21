//! Pure, evidence-bearing queries over immutable normative KerML models.
//! See ADR 0005 for the precise rule boundary, completeness and invalidation contract.
#![forbid(unsafe_code)]

mod bindings;
mod connector_structure;
mod context;
mod contract;
mod cross_features;
mod expression_structure;
mod featuring;
mod formal_targets;
mod implicit;
mod inheritance;
mod namespaces;
mod naming;
mod ordering;
mod producer_worklist;
mod publication;
mod publication_overlay;
mod queries;
mod resolution;
mod result_structure;
mod structural;
mod typing;
mod validation;

#[cfg(test)]
#[path = "../tests/unit/variable_publication.rs"]
mod variable_publication;

#[cfg(test)]
#[path = "../tests/unit/expression_chains.rs"]
mod expression_chains;

pub use bindings::*;
pub use connector_structure::*;
pub use context::*;
pub use contract::*;
pub use cross_features::{CrossDomainFactor, OwnedCrossDomain};
pub use featuring::ConnectorFeaturing;
pub use formal_targets::*;
pub use namespaces::{MemberAccess, MemberMatch};
pub use naming::EffectiveNames;
pub use producer_worklist::*;
pub use publication::*;
pub use publication_overlay::*;
pub use queries::KerMlQueries;
pub use resolution::*;
pub use result_structure::*;
pub use structural::MultiplicityBounds;
pub use validation::RedefinitionEndConformance;

#[cfg(test)]
#[path = "../tests/unit/producer_worklist.rs"]
mod producer_worklist_tests;
