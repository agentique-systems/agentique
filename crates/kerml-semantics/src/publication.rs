//! Publication relevance is independent of conformance implementation coverage.
use crate::{Diagnostic, SemanticContextId};
use std::collections::{BTreeMap, BTreeSet};

/// How a formal obligation affects the semantic dependency contract (ADR 0022).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicationRelevance {
    PublicationCritical,
    ValidatorOnly,
    ExecutionDependent,
    AuthorityBlocked,
    NotApplicableToCorpus,
}

/// Whether an exact authority disagreement changes canonical interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityImpact {
    PublicationBlockingAuthorityConflict,
    ValidationOnlyAuthorityConflict,
}

/// Completeness of the validator, never a synonym for graph validity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationCoverage {
    Complete,
    Incomplete,
}

/// Auditable constraint inventory for a particular report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstraintCoverage {
    pub inventory: BTreeSet<String>,
    pub checked: BTreeSet<String>,
    /// Constraints deliberately not evaluated on a partial derivation overlay.
    pub deferred_by_phase: BTreeSet<String>,
}
impl ConstraintCoverage {
    pub fn status(&self) -> ValidationCoverage {
        if self.inventory == self.checked && self.deferred_by_phase.is_empty() {
            ValidationCoverage::Complete
        } else {
            ValidationCoverage::Incomplete
        }
    }
}

/// Separate conformance output, bound to the same exact semantic context.
/// This report confers no publication acceptance and can also describe a draft.
#[derive(Clone, Debug)]
pub struct KerMlConformanceReport {
    pub context: SemanticContextId,
    pub diagnostics: BTreeSet<Diagnostic>,
    pub coverage: ConstraintCoverage,
    pub authority_conflicts: BTreeMap<String, AuthorityImpact>,
}

/// Input phase for checks that assert that all implied relationships are included.
/// Contexts acquire the complete phase only through `CanonicalPublicationBuilder`;
/// supplying a phase label cannot promote a partial overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum DerivationPhase {
    #[default]
    Declared,
    PartialDerivationOverlay,
    /// Assigned only after the canonical builder has checked producer closure.
    CompletePublicationOverlay,
}
