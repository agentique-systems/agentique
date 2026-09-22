//! Unpublished rule-evidenced construction; strict publication is a separate API.
use super::*;
use crate::{ConstructionObligation, ConstructionView};

/// Immutable derivation over an unpublished declared input. Missing lower bounds
/// remain obligations, even when other semantic facts are available. There is no
/// conversion to a `Snapshot` or a strict `DerivedOverlay`.
///
/// ```compile_fail
/// use agq_kernel::derived::{ConstructionOverlay, DerivedOverlay};
/// fn promote(candidate: ConstructionOverlay) -> DerivedOverlay { candidate }
/// ```
#[derive(Clone, Debug)]
pub struct ConstructionOverlay {
    inner: Arc<OverlayData>,
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
