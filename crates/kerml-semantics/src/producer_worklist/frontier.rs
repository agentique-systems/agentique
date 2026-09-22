//! One worklist algorithm, with strict and unpublished typed materializers.
use crate::ResultStructurePlan;
use agq_kernel::{
    ConstructionView, ElementId, ModelView, Snapshot,
    derived::{
        ConstructionDerivationBuilder, ConstructionOverlay, DerivationBuildMetrics,
        DerivationBuilder, DerivationError, DerivedOverlay,
    },
};
use std::sync::Arc;

pub(super) trait ProducerFrontier: Clone {
    type Input;
    fn empty(input: &Self::Input) -> Result<Self, DerivationError>;
    fn input_model(input: &Self::Input) -> &ModelView;
    fn is_dependency_element(input: &Self::Input, subject: ElementId) -> bool;
    fn model(&self) -> &ModelView;
    fn obligation_keys(
        &self,
    ) -> std::collections::BTreeSet<(ElementId, agq_kernel::PropertyId, usize)>;
    fn build_metrics(&self) -> &DerivationBuildMetrics;
    fn facts_equal(&self, other: &Self) -> bool;
    fn prepare(
        plan: ResultStructurePlan<'_>,
        input: &Self,
    ) -> Result<DerivationBuilder<Self::Input>, DerivationError>;
    fn build(builder: DerivationBuilder<Self::Input>, input: Self)
    -> Result<Self, DerivationError>;
}
impl ProducerFrontier for DerivedOverlay {
    type Input = Snapshot;
    fn empty(input: &Snapshot) -> Result<Self, DerivationError> {
        DerivationBuilder::new(input.clone()).build()
    }
    fn input_model(input: &Snapshot) -> &ModelView {
        input.model()
    }
    fn is_dependency_element(input: &Snapshot, subject: ElementId) -> bool {
        input.is_dependency_element(subject)
    }
    fn model(&self) -> &ModelView {
        self.model()
    }
    fn obligation_keys(
        &self,
    ) -> std::collections::BTreeSet<(ElementId, agq_kernel::PropertyId, usize)> {
        Default::default()
    }
    fn build_metrics(&self) -> &DerivationBuildMetrics {
        self.build_metrics()
    }
    fn facts_equal(&self, other: &Self) -> bool {
        self.facts().eq(other.facts())
    }
    fn prepare(
        plan: ResultStructurePlan<'_>,
        input: &Self,
    ) -> Result<DerivationBuilder, DerivationError> {
        plan.prepare_on_overlay(input)
    }
    fn build(builder: DerivationBuilder, input: Self) -> Result<Self, DerivationError> {
        builder.build_on_overlay(input)
    }
}
impl ProducerFrontier for ConstructionOverlay {
    type Input = Arc<ConstructionView>;
    fn empty(input: &Self::Input) -> Result<Self, DerivationError> {
        ConstructionDerivationBuilder::for_construction(input.clone()).build()
    }
    fn input_model(input: &Self::Input) -> &ModelView {
        input.model()
    }
    fn is_dependency_element(input: &Self::Input, subject: ElementId) -> bool {
        input
            .immutable_dependency()
            .is_some_and(|dependency| dependency.model().element(subject).is_some())
    }
    fn model(&self) -> &ModelView {
        self.model()
    }
    fn obligation_keys(
        &self,
    ) -> std::collections::BTreeSet<(ElementId, agq_kernel::PropertyId, usize)> {
        self.obligations()
            .iter()
            .map(|obligation| (obligation.element, obligation.property, obligation.actual))
            .collect()
    }
    fn build_metrics(&self) -> &DerivationBuildMetrics {
        self.build_metrics()
    }
    fn facts_equal(&self, other: &Self) -> bool {
        self.facts().eq(other.facts())
    }
    fn prepare(
        plan: ResultStructurePlan<'_>,
        input: &Self,
    ) -> Result<ConstructionDerivationBuilder, DerivationError> {
        plan.prepare_on_construction_overlay(input)
    }
    fn build(builder: ConstructionDerivationBuilder, input: Self) -> Result<Self, DerivationError> {
        builder.build_on_construction_overlay(input)
    }
}
