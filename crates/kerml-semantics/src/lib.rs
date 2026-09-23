//! Pure, evidence-bearing queries over immutable normative KerML models.
//! See ADR 0005 for the precise rule boundary, completeness and invalidation contract.
#![forbid(unsafe_code)]

mod bindings;
mod connector_structure;
mod context;
mod context_digest;
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
#[cfg(test)]
#[path = "../tests/unit/producer_chain_closure.rs"]
mod producer_chain_closure_tests;
mod producer_closed_dependency;
#[cfg(test)]
#[path = "../tests/unit/producer_closed_dependency.rs"]
mod producer_closed_dependency_tests;
mod producer_closure;
#[cfg(test)]
#[path = "../tests/unit/producer_closure.rs"]
mod producer_closure_tests;
#[cfg(test)]
#[path = "../tests/unit/producer_parameter_scope.rs"]
mod producer_parameter_scope_tests;
#[cfg(test)]
#[path = "../tests/unit/producer_rule_ownership.rs"]
mod producer_rule_ownership_tests;
#[cfg(test)]
#[path = "../tests/unit/producer_value_strata.rs"]
mod producer_value_strata_tests;
mod producer_worklist;
mod publication;
mod publication_overlay;
mod queries;
mod read_dependencies;
mod relationship_sources;
mod resolution;
mod result_structure;
#[cfg(test)]
#[path = "../tests/unit/selected_contribution_merge.rs"]
mod selected_contribution_merge_tests;
#[cfg(test)]
#[path = "../tests/unit/selected_contribution.rs"]
mod selected_contribution_tests;
mod specialization_witness;
mod status_queries;
mod structural;
mod trusted_publication;
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
pub use producer_closed_dependency::*;
pub use producer_closure::*;
pub use producer_worklist::*;
pub use publication::*;
pub use publication_overlay::*;
pub use queries::KerMlQueries;
pub use read_dependencies::{PublicationProviderReads, QueryInvalidationSet, QueryReadSet};
pub use resolution::*;
pub use result_structure::*;
pub use status_queries::*;
pub use structural::MultiplicityBounds;
pub use trusted_publication::{TrustedPublicationError, TrustedPublicationReceipt};
pub use validation::RedefinitionEndConformance;

#[cfg(test)]
#[path = "../tests/unit/producer_worklist.rs"]
mod producer_worklist_tests;
