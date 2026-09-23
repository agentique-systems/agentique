//! Bounded observations of scheduler work. These never participate in evidence.
use crate::ProducerFamilyId;
use std::collections::BTreeMap;
use std::time::Instant;

/// Why a subject was put back on the worklist. Categories can overlap.
/// Graph/provider categories describe the existing conservative invalidation
/// keys, not a claim that each corresponding semantic answer changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicationReopenReason {
    GraphFactChanged,
    ProviderSearchChanged,
    CertificateTransportChanged,
    NewHelperCreated,
    NewRelationshipEndpoint,
    NewProducerOpportunity,
    ContextualBindingDependency,
}

impl PublicationReopenReason {
    const ALL: [Self; 7] = [
        Self::GraphFactChanged,
        Self::ProviderSearchChanged,
        Self::CertificateTransportChanged,
        Self::NewHelperCreated,
        Self::NewRelationshipEndpoint,
        Self::NewProducerOpportunity,
        Self::ContextualBindingDependency,
    ];
}

#[derive(Clone, Copy, Default)]
pub(super) struct ReopenReasons(u8);

impl ReopenReasons {
    pub(super) fn insert(&mut self, reason: PublicationReopenReason) {
        self.0 |= 1 << reason as u8;
    }

    pub(super) fn iter(self) -> impl Iterator<Item = PublicationReopenReason> {
        PublicationReopenReason::ALL
            .into_iter()
            .filter(move |reason| self.0 & (1 << *reason as u8) != 0)
    }
}

/// Attempts and inclusive planning time for one producer family.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicationFamilyMetrics {
    pub attempts: usize,
    pub planning_micros: u128,
    /// Shared implementations (valuation/binding and language extensions) cannot
    /// attribute their common query work to one family. This is inclusive time
    /// attributed to each participant, and must not be summed across families.
    pub shared_planning_micros: u128,
    pub reopened_by_reason: BTreeMap<PublicationReopenReason, usize>,
}

impl PublicationFamilyMetrics {
    pub(crate) fn merge(&mut self, other: Self) {
        self.attempts += other.attempts;
        self.planning_micros += other.planning_micros;
        self.shared_planning_micros += other.shared_planning_micros;
        for (reason, count) in other.reopened_by_reason {
            *self.reopened_by_reason.entry(reason).or_default() += count;
        }
    }
}

/// Observations of one fully completed immutable frontier. Timing is diagnostic
/// only. Planning includes queries and their in-memory caches; materialization,
/// read indexing, certificate invalidation and certificate construction are
/// measured separately. No per-subject or per-query trace is retained.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicationRoundMetrics {
    pub subjects_evaluated: usize,
    pub subjects_skipped: usize,
    pub subjects_reopened: usize,
    pub planned_elements: usize,
    pub accepted_elements: usize,
    pub accepted_occurrences: usize,
    pub next_dirty_subjects: usize,
    pub next_dirty_by_reason: BTreeMap<PublicationReopenReason, usize>,
    pub families: BTreeMap<ProducerFamilyId, PublicationFamilyMetrics>,
    pub planning_micros: u128,
    pub dependency_index_micros: u128,
    pub model_materialization_micros: u128,
    pub certificate_revalidation_micros: u128,
    pub certificate_build_micros: u128,
    pub elapsed_micros: u128,
}

/// Measures the existing family block without changing branch/early-return
/// behavior. The tiny family array is inline and no trace allocations occur.
pub(crate) struct FamilyTimer<'a, const N: usize> {
    metrics: &'a mut BTreeMap<ProducerFamilyId, PublicationFamilyMetrics>,
    families: [ProducerFamilyId; N],
    started: Instant,
}

impl<'a, const N: usize> FamilyTimer<'a, N> {
    pub(crate) fn new(
        metrics: &'a mut BTreeMap<ProducerFamilyId, PublicationFamilyMetrics>,
        families: [ProducerFamilyId; N],
    ) -> Self {
        Self {
            metrics,
            families,
            started: Instant::now(),
        }
    }
}

impl<const N: usize> Drop for FamilyTimer<'_, N> {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed().as_micros();
        for &family in &self.families {
            let metrics = self.metrics.entry(family).or_default();
            metrics.attempts += 1;
            if self.families.len() == 1 {
                metrics.planning_micros += elapsed;
            } else {
                metrics.shared_planning_micros += elapsed;
            }
        }
    }
}
