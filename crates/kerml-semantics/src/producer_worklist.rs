//! Dependency-driven additive closure over immutable semantic frontiers.
//!
//! Producers in a frontier all observe the same graph. Their contributions are
//! merged before publication, so queue order and batch size cannot select a
//! semantic winner. Negative searches are indexed alongside positive reads.
use crate::*;
use agq_kerml::{BaselineProfile, classes as c};
use agq_kernel::{
    ElementId, MetaclassId, ModelView, Snapshot,
    derived::{DerivationBuilder, DerivedOverlay, StructuralSearch},
    provenance::{Dependency, FactKey},
};
use std::collections::{BTreeMap, BTreeSet};

/// Independent metaclass-gated structural producer families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProducerFamily {
    OwnedInstantiationResult,
    PositionalRedefinition,
    VariableFeaturing,
    OwnedCrossing,
    CrossDomain,
    Invocation,
    FeatureChainExpression,
    FeatureReferenceExpression,
    ExpressionResult,
    FeatureValue,
    IndexSelectResult,
}
impl ProducerFamily {
    pub const ALL: [Self; 11] = [
        Self::OwnedInstantiationResult,
        Self::PositionalRedefinition,
        Self::VariableFeaturing,
        Self::OwnedCrossing,
        Self::CrossDomain,
        Self::Invocation,
        Self::FeatureChainExpression,
        Self::FeatureReferenceExpression,
        Self::ExpressionResult,
        Self::FeatureValue,
        Self::IndexSelectResult,
    ];
    fn applies(self, model: &ModelView, class: MetaclassId, profile: BaselineProfile) -> bool {
        let is = |parent| model.registry().is_subtype(class, parent).unwrap_or(false);
        match self {
            Self::OwnedInstantiationResult => {
                profile == BaselineProfile::OPERATIONAL_V8 && is(c::INSTANTIATION_EXPRESSION)
            }
            Self::PositionalRedefinition | Self::VariableFeaturing => {
                profile == BaselineProfile::OPERATIONAL_V8 && is(c::FEATURE)
            }
            Self::OwnedCrossing | Self::CrossDomain => {
                profile.corrects_owned_cross_domain() && is(c::FEATURE)
            }
            Self::Invocation => {
                profile == BaselineProfile::OPERATIONAL_V8 && is(c::INVOCATION_EXPRESSION)
            }
            Self::FeatureChainExpression => {
                profile == BaselineProfile::OPERATIONAL_V8 && is(c::FEATURE_CHAIN_EXPRESSION)
            }
            Self::FeatureReferenceExpression => is(c::FEATURE_REFERENCE_EXPRESSION),
            Self::ExpressionResult => is(c::EXPRESSION) || is(c::FUNCTION),
            Self::FeatureValue => is(c::FEATURE),
            Self::IndexSelectResult => is(c::INDEX_EXPRESSION) || is(c::SELECT_EXPRESSION),
        }
    }
}

/// Reference mode is intentionally retained for equivalence fixtures. It scans
/// every element at every frontier, including inapplicable metaclasses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PublicationClosureStrategy {
    #[default]
    Worklist,
    ReferenceFullScan,
}
/// Scheduling choices affect resource use only, never visibility within a frontier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PublicationWorklistOrder {
    #[default]
    Fifo,
    Lifo,
    ReversedInitial,
    Partitioned,
}
#[derive(Clone, Debug)]
pub struct PublicationClosureOptions {
    /// Restricts a scoped audit; canonical publication always uses None.
    pub initial_subjects: Option<BTreeSet<ElementId>>,
    pub max_rounds: usize,
    pub batch_size: usize,
    pub order: PublicationWorklistOrder,
    pub strategy: PublicationClosureStrategy,
}
impl Default for PublicationClosureOptions {
    fn default() -> Self {
        Self {
            initial_subjects: None,
            max_rounds: 32,
            batch_size: 32,
            order: Default::default(),
            strategy: Default::default(),
        }
    }
}
/// Deterministic work accounting. Timing and allocator observations belong to
/// the external watchdog; none of these counters participates in acceptance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicationCounters {
    pub declared_subjects: usize,
    pub subjects_considered: usize,
    pub subjects_evaluated: usize,
    pub subjects_skipped_by_applicability: usize,
    pub producer_families_attempted: usize,
    pub new_elements_proposed: usize,
    pub new_association_occurrences_proposed: usize,
    pub existing_derived_facts_reused: usize,
    pub new_proof_sets_interned: usize,
    pub existing_proof_sets_reused: usize,
    pub dependency_edges_considered: usize,
    pub dirty_subjects_enqueued: usize,
    pub dirty_reevaluations: usize,
    pub maximum_worklist_size: usize,
    pub overlay_materializations: usize,
    pub fixed_point_rounds: usize,
}
/// A scoped closure is useful for authored projects and real-corpus slices, but
/// cannot assert whole-library capabilities or become a complete publication.
pub struct PublicationClosure {
    pub overlay: DerivedOverlay,
    pub stages: Vec<PublicationStage>,
    pub counters: PublicationCounters,
    pub completeness: Completeness,
    pub converged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum InvalidationKey {
    Element(ElementId),
    Incoming(ElementId),
    Global,
}

fn search_keys(search: &SearchDependency) -> Vec<InvalidationKey> {
    use InvalidationKey as K;
    use SearchDependency as S;
    match search {
        S::Element(id) => vec![K::Element(*id)],
        S::PropertySet { element, .. } => vec![K::Element(*element)],
        S::Incoming { target } => vec![K::Incoming(*target)],
        S::NamespaceMembers { namespace } | S::ImportSet { namespace } => {
            vec![K::Element(*namespace)]
        }
        S::ImportedNamespace { import, namespace } => {
            vec![K::Element(*import), K::Element(*namespace)]
        }
        S::RedefinitionScope {
            relationship,
            namespace,
            ..
        } => vec![K::Element(*relationship), K::Element(*namespace)],
        S::Kernel(StructuralSearch::Element(id)) => vec![K::Element(*id)],
        S::Kernel(
            StructuralSearch::Property { element, .. }
            | StructuralSearch::Association { element, .. },
        ) => vec![K::Element(*element)],
        S::Kernel(StructuralSearch::Incoming(id)) => vec![K::Incoming(*id)],
        S::Kernel(StructuralSearch::Model) | S::Instances { .. } => vec![K::Global],
        // These identities are immutable during this additive session. Binding
        // role reads are local provenance reads, not global role-population scans.
        S::Kernel(StructuralSearch::DescriptorGraph)
        | S::StandardLibraries
        | S::ProjectRoots { .. }
        | S::FormalConstraintTarget(_)
        | S::ValidationRule(_)
        | S::ImpliedBindingRole(_) => vec![],
    }
}
/// Translate language-level reads into persistent kernel computation searches.
/// Context identities (profile, bindings, available roots) are fixed for closure.
pub(crate) fn structural_searches<T>(answer: &QueryResult<T>) -> BTreeSet<StructuralSearch> {
    let mut result = BTreeSet::new();
    for search in &answer.search_dependencies {
        if let SearchDependency::Kernel(search) = search {
            result.insert(search.clone());
        } else if let SearchDependency::PropertySet { element, property } = search {
            result.insert(StructuralSearch::Property {
                element: *element,
                property: *property,
            });
        } else {
            result.extend(search_keys(search).into_iter().map(|key| match key {
                InvalidationKey::Element(id) => StructuralSearch::Element(id),
                InvalidationKey::Incoming(id) => StructuralSearch::Incoming(id),
                InvalidationKey::Global => StructuralSearch::Model,
            }));
        }
    }
    result
}
#[derive(Default)]
struct DependencyIndex {
    readers: BTreeMap<InvalidationKey, BTreeSet<ElementId>>,
    subjects: BTreeMap<ElementId, BTreeSet<InvalidationKey>>,
}
impl DependencyIndex {
    fn replace<T>(
        &mut self,
        subject: ElementId,
        answer: &QueryResult<T>,
        model: &ModelView,
        counters: &mut PublicationCounters,
    ) {
        let mut keys: BTreeSet<_> = answer
            .search_dependencies
            .iter()
            .flat_map(search_keys)
            .collect();
        // Immediate canonical dependencies suffice: kernel computation searches
        // propagate the bounded negative reads of any derived facts queried.
        for dependency in &answer.canonical_dependencies {
            let (Dependency::Declared(fact) | Dependency::Derived(fact)) = dependency;
            match fact {
                FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                    keys.insert(InvalidationKey::Element(*id));
                }
                FactKey::AssociationOccurrence(id) => {
                    if let Some(occurrence) = model.association_occurrence(*id) {
                        keys.extend(
                            occurrence
                                .ends()
                                .values()
                                .copied()
                                .map(InvalidationKey::Element),
                        );
                    }
                }
            }
        }
        keys.insert(InvalidationKey::Element(subject));
        counters.dependency_edges_considered += keys.len();
        if let Some(previous) = self.subjects.insert(subject, keys.clone()) {
            for key in previous {
                if let Some(readers) = self.readers.get_mut(&key) {
                    readers.remove(&subject);
                }
            }
        }
        for key in keys {
            self.readers.entry(key).or_default().insert(subject);
        }
    }
    fn dirty(
        &self,
        changed: &BTreeSet<ElementId>,
        counters: &mut PublicationCounters,
    ) -> BTreeSet<ElementId> {
        let mut result = changed.clone();
        for key in std::iter::once(InvalidationKey::Global).chain(
            changed
                .iter()
                .flat_map(|&id| [InvalidationKey::Element(id), InvalidationKey::Incoming(id)]),
        ) {
            if let Some(readers) = self.readers.get(&key) {
                counters.dependency_edges_considered += readers.len();
                result.extend(readers);
            }
        }
        result
    }
}
fn changed_elements(
    before: &ModelView,
    after: &ModelView,
    candidates: &BTreeSet<ElementId>,
) -> BTreeSet<ElementId> {
    candidates
        .iter()
        .copied()
        .filter(|&id| {
            before.element(id) != after.element(id)
                || !before.incoming(id).eq(after.incoming(id))
                || !before
                    .incident_associations(id)
                    .eq(after.incident_associations(id))
        })
        .collect()
}

/// Close structural producers using immutable frontiers and query dependencies.
/// The context factory must retain identical profile, binding and availability
/// identities between frontiers; only the supplied overlay may change.
/// The observer arguments match `CanonicalPublicationBuilder::build_with_progress`.
pub fn close_result_structure(
    snapshot: &Snapshot,
    options: PublicationClosureOptions,
    mut context: impl for<'m> FnMut(
        &'m DerivedOverlay,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError>,
    mut batch_progress: impl FnMut(usize, usize, usize, usize),
    mut progress: impl FnMut(&PublicationStage),
) -> Result<PublicationClosure, PublicationOverlayError> {
    let mut overlay = DerivationBuilder::new(snapshot.clone()).build()?;
    let mut counters = PublicationCounters {
        declared_subjects: snapshot.model().len(),
        ..Default::default()
    };
    let mut stages = vec![];
    let mut index = DependencyIndex::default();
    let mut status = BTreeMap::new();
    let mut population: BTreeSet<_> = options
        .initial_subjects
        .clone()
        .unwrap_or_else(|| snapshot.model().elements().map(|r| r.id()).collect());
    let mut worklist = population.clone();
    let mut seen = BTreeSet::new();
    let mut applicability = BTreeMap::<MetaclassId, Vec<ProducerFamily>>::new();
    let mut identity: Option<SemanticContextId> = None;
    let mut converged = false;
    for round in 0..options.max_rounds {
        let context = context(&overlay)?;
        if let Some(previous) = &identity {
            // Compare all immutable inputs; only graph digest and derivation
            // phase naturally change as the additive overlay grows.
            let mut current = context.id().clone();
            current.model_digest = previous.model_digest;
            current.derivation_phase = previous.derivation_phase;
            current.library_graph_digest = previous.library_graph_digest;
            if &current != previous {
                return Err(PublicationOverlayError::Derivation(
                    agq_kernel::derived::DerivationError::InputContextMismatch,
                ));
            }
        } else {
            identity = Some(context.id().clone());
        }
        let profile = context.id().options.baseline_profile;
        let mut subjects = vec![];
        for subject in worklist {
            counters.subjects_considered += 1;
            let Some(record) = overlay.model().element(subject) else {
                continue;
            };
            let families = applicability.entry(record.metaclass()).or_insert_with(|| {
                ProducerFamily::ALL
                    .into_iter()
                    .filter(|f| f.applies(overlay.model(), record.metaclass(), profile))
                    .collect()
            });
            if families.is_empty() && options.strategy == PublicationClosureStrategy::Worklist {
                counters.subjects_skipped_by_applicability += 1;
            } else {
                subjects.push(subject);
            }
        }
        counters.maximum_worklist_size = counters.maximum_worklist_size.max(subjects.len());
        match options.order {
            PublicationWorklistOrder::Lifo => subjects.reverse(),
            PublicationWorklistOrder::ReversedInitial if round == 0 => subjects.reverse(),
            PublicationWorklistOrder::Partitioned => {
                subjects.sort_by_key(|id| (id.as_u128() % 4, *id))
            }
            _ => {}
        }
        let mut plan = KerMlQueries::new(context.fork()).plan_result_structure([]);
        for (batch_index, batch) in subjects.chunks(options.batch_size.max(1)).enumerate() {
            let q = match options.strategy {
                PublicationClosureStrategy::Worklist => {
                    KerMlQueries::for_production(context.fork())
                }
                PublicationClosureStrategy::ReferenceFullScan => KerMlQueries::new(context.fork()),
            };
            for &subject in batch {
                counters.subjects_evaluated += 1;
                counters.dirty_reevaluations += usize::from(!seen.insert(subject));
                counters.producer_families_attempted += applicability[&overlay
                    .model()
                    .element(subject)
                    .expect("worklist subject")
                    .metaclass()]
                    .len();
                let mut part = q.plan_result_structure([subject]);
                index.replace(subject, &part.production, overlay.model(), &mut counters);
                status.insert(
                    subject,
                    (
                        part.production.completeness,
                        part.production.diagnostics.clone(),
                    ),
                );
                part.discard_aggregate_proof();
                plan.merge(part)?;
            }
            batch_progress(
                round,
                ((batch_index + 1) * options.batch_size.max(1)).min(subjects.len()),
                subjects.len(),
                plan.planned_elements().count(),
            );
        }
        let candidates = plan.affected_elements();
        let completeness = status
            .values()
            .map(|(s, _)| *s)
            .max()
            .unwrap_or(Completeness::Complete);
        let diagnostics = status
            .values()
            .flat_map(|(_, ds)| ds.iter().cloned())
            .collect();
        let next = plan.materialize_on_overlay(&overlay)?.overlay;
        let metrics = next.build_metrics();
        counters.existing_derived_facts_reused += metrics.existing_facts_reused;
        counters.new_proof_sets_interned += metrics.proof_sets_interned;
        counters.existing_proof_sets_reused += metrics.proof_sets_reused;
        counters.dependency_edges_considered += metrics.dependency_edges_considered;
        counters.overlay_materializations += 1;
        counters.fixed_point_rounds += 1;
        let record = PublicationStage {
            stage: round,
            input_elements: overlay.model().len(),
            added_elements: next.model().len() - overlay.model().len(),
            added_occurrences: next.model().association_occurrences().count()
                - overlay.model().association_occurrences().count(),
            completeness,
            diagnostics,
        };
        counters.new_elements_proposed += record.added_elements;
        counters.new_association_occurrences_proposed += record.added_occurrences;
        let changed = changed_elements(overlay.model(), next.model(), &candidates);
        progress(&record);
        stages.push(record);
        if changed.is_empty() {
            converged = true;
            overlay = next;
            break;
        }
        population.extend(candidates.iter().copied().filter(|id| {
            overlay.model().element(*id).is_none() && next.model().element(*id).is_some()
        }));
        worklist = match options.strategy {
            PublicationClosureStrategy::Worklist => index
                .dirty(&changed, &mut counters)
                .intersection(&population)
                .copied()
                .collect(),
            PublicationClosureStrategy::ReferenceFullScan => population.clone(),
        };
        counters.dirty_subjects_enqueued += worklist.len();
        overlay = next;
    }
    Ok(PublicationClosure {
        overlay,
        stages,
        counters,
        converged,
        completeness: if converged {
            status
                .values()
                .map(|(s, _)| *s)
                .max()
                .unwrap_or(Completeness::Complete)
        } else {
            Completeness::Incomplete
        },
    })
}
