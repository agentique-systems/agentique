//! Unpublished rule-evidenced construction; strict publication is a separate API.
use super::*;
use crate::{ConstructionObligation, ConstructionView};

/// Immutable derivation over an unpublished declared input. Missing lower bounds
/// remain obligations, even when other semantic facts are available. There is no
/// unchecked conversion to a `Snapshot` or a strict `DerivedOverlay`.
///
/// ```compile_fail
/// use agq_kernel::derived::{ConstructionOverlay, DerivedOverlay};
/// fn promote(candidate: ConstructionOverlay) -> DerivedOverlay { candidate }
/// ```
#[derive(Clone, Debug)]
pub struct ConstructionOverlay {
    pub(super) inner: Arc<OverlayData>,
}
impl ConstructionOverlay {
    /// Exact original declared construction, without any inferred facts.
    pub fn declared(&self) -> &ConstructionView {
        self.inner.declared.construction()
    }
    /// Shared input for preparing the next additive frontier without copying its indexes.
    pub fn declared_shared(&self) -> &Arc<ConstructionView> {
        self.inner.declared.construction()
    }
    /// Revision of the original unpublished declared input.
    pub fn base_revision(&self) -> RevisionId {
        self.inner.declared.revision()
    }
    /// Declared and derived facts together; this view remains unpublished.
    pub fn model(&self) -> &ModelView {
        &self.inner.model
    }
    /// All still-missing required values after this derivation stage.
    pub fn obligations(&self) -> &[ConstructionObligation] {
        &self.inner.obligations
    }
    /// Immediate producer rule and evidence for a derived fact.
    pub fn explain(&self, fact: FactKey) -> Option<&Explanation> {
        self.inner.explanations.get(&fact).map(Arc::as_ref)
    }
    /// Derived assertion keys and evidence in deterministic identity order.
    pub fn facts(&self) -> impl Iterator<Item = (FactKey, &Explanation)> {
        self.inner
            .explanations
            .iter()
            .map(|(key, proof)| (*key, proof.as_ref()))
    }
    /// Diagnostic work counters, never a publication or completeness certificate.
    pub fn build_metrics(&self) -> &DerivationBuildMetrics {
        &self.inner.build_metrics
    }
    /// Revalidate the complete graph against an independently committed strict
    /// snapshot of the exact original declarations and reserved identity history.
    /// A fresh revision label is permitted only after that equivalence check.
    ///
    /// Every structural bound and protected ownership constraint is checked
    /// again, as are every proof dependency, computation search and proof cycle.
    /// Derived facts keep their provenance and never enter the declared snapshot.
    /// The dependency Arc, canonical records and immutable evidence remain shared;
    /// consuming the last overlay handle also transfers its owned index maps.
    ///
    /// This performs no language-rule evaluation and confers no semantic
    /// completeness or publication acceptance. Outstanding structural obligations
    /// are rejected by strict validation. Different declarations, registry,
    /// reserved identities or dependency identity cannot be substituted.
    pub fn revalidate(self, declared: Snapshot) -> Result<DerivedOverlay, DerivationError> {
        if !self.declared().matches_strict_snapshot(&declared) {
            return Err(DerivationError::InputContextMismatch);
        }
        // The exact-input comparison is the sole bridge between construction
        // and strict builder inputs. The ordinary builder then rechecks the full
        // model using strict multiplicities, never the construction deficit mode.
        let mut builder = DerivationBuilder::new(declared);
        builder.previous = Some(self.inner);
        let mut strict = builder.build()?;
        let inner = Arc::get_mut(&mut strict.inner).expect("new unshared strict overlay");
        let mut checked_proofs = HashSet::new();
        for (&fact, explanation) in &inner.explanations {
            let unsuccessful = matches!(fact, FactKey::Property { element, property }
                if inner.model.statuses.contains_key(&(element, property)));
            if !checked_proofs.insert((Arc::as_ptr(explanation) as usize, unsuccessful)) {
                continue;
            }
            for &dependency in &explanation.dependencies {
                inner.build_metrics.dependency_edges_considered += 1;
                let exists = match dependency {
                    Dependency::Declared(key) => inner.declared.has_declared_fact(key),
                    Dependency::Derived(key) => {
                        if !unsuccessful
                            && matches!(key, FactKey::Property { element, property }
                                if inner.model.statuses.contains_key(&(element, property)))
                        {
                            return Err(DerivationError::IncompleteDependency(key));
                        }
                        inner.explanations.contains_key(&key)
                    }
                };
                if !exists {
                    return Err(DerivationError::MissingDependency { fact, dependency });
                }
            }
        }
        for fact in inner.model.searches.keys() {
            if !inner.explanations.contains_key(fact) {
                return Err(DerivationError::MissingSearchSubject(*fact));
            }
        }
        let all_facts = inner.explanations.keys().copied().collect();
        let cycle =
            cyclic_explanations(&inner.explanations, &inner.evidence_pool, &all_facts, true);
        if !cycle.is_empty() {
            return Err(DerivationError::DependencyCycle(cycle));
        }
        Ok(strict)
    }
}

/// The shared derivation queue with a construction-only output type. All queued
/// writes pass the same endpoint, upper-bound, ownership, dependency and proof
/// validation as strict derivation; only missing lower bounds remain obligations.
///
/// ```compile_fail
/// use agq_kernel::derived::{ConstructionDerivationBuilder, DerivedOverlay};
/// fn promote(builder: ConstructionDerivationBuilder) -> DerivedOverlay {
///     builder.build().unwrap()
/// }
/// ```
pub type ConstructionDerivationBuilder = DerivationBuilder<Arc<ConstructionView>>;
impl DerivationBuilder<Arc<ConstructionView>> {
    /// Begin an unpublished derivation over this exact immutable construction.
    pub fn for_construction(declared: Arc<ConstructionView>) -> Self {
        Self::with_input(DerivationInput::Construction(declared))
    }
    /// Add a frontier to the same construction input, retaining all earlier facts.
    pub fn from_construction_overlay(previous: ConstructionOverlay) -> Self {
        let mut builder = Self::with_input(previous.inner.declared.clone());
        builder.previous = Some(previous.inner);
        builder
    }
    /// Finish a prepared frontier after releasing readers of the prior overlay.
    /// Unshared storage is transferred; mismatched input identities are rejected.
    pub fn build_on_construction_overlay(
        mut self,
        previous: ConstructionOverlay,
    ) -> Result<ConstructionOverlay, DerivationError> {
        self.attach_previous(previous.inner)?;
        self.build()
    }
    /// Validate the candidate atomically and retain every lower-bound obligation.
    /// Even an obligation-free result remains an unpublished construction overlay.
    pub fn build(self) -> Result<ConstructionOverlay, DerivationError> {
        self.build_inner()
            .map(|inner| ConstructionOverlay { inner })
    }
}

#[cfg(any(test, feature = "verification"))]
impl ConstructionOverlay {
    pub(crate) fn storage_observation(&self) -> crate::storage_observer::DependencyStorage {
        self.inner.storage_observation()
    }
}
