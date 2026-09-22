//! Dependency-driven additive closure over immutable semantic frontiers.
//!
//! Producers in a frontier all observe the same graph. Their contributions are
//! merged before publication, so queue order and batch size cannot select a
//! semantic winner. Negative searches are indexed alongside positive reads.
use crate::read_dependencies::{InvalidationKey, query_read_keys};
use crate::*;
use agq_kerml::{BaselineProfile, classes as c};
use agq_kernel::{ElementId, MetaclassId, ModelView, Snapshot, derived::DerivedOverlay};
use std::collections::{BTreeMap, BTreeSet};

mod frontier;
use frontier::ProducerFrontier;

/// Additional language producers participating in the same immutable frontiers
/// and positive/negative read index as KerML. Contributions must carry their
/// actual query evidence and stable rule/output identities.
pub trait PublicationProducerExtension {
    fn applies(&self, model: &ModelView, class: MetaclassId) -> bool;
    /// Whether to establish structurally dependent scalar predicates before
    /// context-sensitive bindings. Later dirty reads still reevaluate them;
    /// contradictory additive values fail instead of selecting a winner.
    fn has_stable_properties(&self) -> bool {
        false
    }
    fn contribute<'m>(
        &self,
        queries: &KerMlQueries<'m>,
        subject: ElementId,
        stratum: ResultStructureStratum,
        plan: &mut ResultStructurePlan<'m>,
    ) -> Result<(), agq_kernel::derived::DerivationError>;
}

impl PublicationProducerExtension for () {
    fn applies(&self, _: &ModelView, _: MetaclassId) -> bool {
        false
    }
    fn contribute<'m>(
        &self,
        _: &KerMlQueries<'m>,
        _: ElementId,
        _: ResultStructureStratum,
        _: &mut ResultStructurePlan<'m>,
    ) -> Result<(), agq_kernel::derived::DerivationError> {
        Ok(())
    }
}

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
                profile.supports_publication_producers() && is(c::INSTANTIATION_EXPRESSION)
            }
            Self::PositionalRedefinition | Self::VariableFeaturing => {
                profile.supports_publication_producers() && is(c::FEATURE)
            }
            Self::OwnedCrossing | Self::CrossDomain => {
                profile.corrects_owned_cross_domain() && is(c::FEATURE)
            }
            Self::Invocation => {
                profile.supports_publication_producers() && is(c::INVOCATION_EXPRESSION)
            }
            Self::FeatureChainExpression => {
                profile.supports_publication_producers() && is(c::FEATURE_CHAIN_EXPRESSION)
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
    /// Restricts a scoped audit to these subjects plus their generated outputs.
    /// The caller supplies any required dependency closure; reading a dependency
    /// does not automatically schedule producers outside this population.
    /// Canonical publication always uses None and covers the entire input.
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
    /// Validated worklist frontiers that retained the previous immutable overlay.
    pub empty_frontiers_reused: usize,
    pub fixed_point_rounds: usize,
    pub active_dependency_subjects: usize,
    pub active_dependency_keys: usize,
    /// Logical subject/read-key pairs; the bidirectional index stores each twice.
    pub active_dependency_edges: usize,
    pub maximum_dependency_keys: usize,
    pub maximum_dependency_edges: usize,
    /// Current published search metadata before sharing, counted per fact.
    pub logical_search_sets: usize,
    pub logical_search_entries: usize,
    /// Current retained search pool, including distinct superseded union inputs.
    pub retained_search_sets: usize,
    pub retained_search_entries: usize,
    /// Cumulative interning work from actual overlay materializations only.
    pub search_sets_interned: usize,
    pub search_sets_reused: usize,
}
/// A scoped closure is useful for authored projects and real-corpus slices, but
/// cannot assert whole-library capabilities or become a complete publication.
pub struct PublicationClosure<Overlay = DerivedOverlay> {
    pub overlay: Overlay,
    pub stages: Vec<PublicationStage>,
    pub counters: PublicationCounters,
    pub completeness: Completeness,
    pub converged: bool,
    /// Bounded model population read by the latest evaluation of each producer.
    /// Scoped callers can audit their boundary without rerunning producers.
    pub producer_reads: QueryInvalidationSet,
}

#[derive(Default)]
struct DependencyIndex {
    readers: BTreeMap<InvalidationKey, BTreeSet<ElementId>>,
    // A subject's reads are replaced atomically, never mutated in place. Retain
    // their sorted unique keys densely instead of a second tree per subject.
    subjects: BTreeMap<ElementId, Box<[InvalidationKey]>>,
    edges: usize,
}
impl DependencyIndex {
    fn replace<T>(
        &mut self,
        subject: ElementId,
        answer: &QueryResult<T>,
        model: &ModelView,
        counters: &mut PublicationCounters,
    ) {
        let mut keys = query_read_keys(answer, model);
        keys.insert(InvalidationKey::Element(subject));
        counters.dependency_edges_considered += keys.len();
        self.replace_keys(subject, keys, counters);
    }
    fn replace_keys(
        &mut self,
        subject: ElementId,
        keys: BTreeSet<InvalidationKey>,
        counters: &mut PublicationCounters,
    ) {
        let keys: Box<[_]> = keys.into_iter().collect();
        self.edges += keys.len();
        if let Some(previous) = self.subjects.remove(&subject) {
            self.edges -= previous.len();
            for key in previous {
                if let Some(readers) = self.readers.get_mut(&key) {
                    readers.remove(&subject);
                    if readers.is_empty() {
                        self.readers.remove(&key);
                    }
                }
            }
        }
        for &key in &keys {
            self.readers.entry(key).or_default().insert(subject);
        }
        self.subjects.insert(subject, keys);
        counters.active_dependency_subjects = self.subjects.len();
        counters.active_dependency_keys = self.readers.len();
        counters.active_dependency_edges = self.edges;
        counters.maximum_dependency_keys = counters.maximum_dependency_keys.max(self.readers.len());
        counters.maximum_dependency_edges = counters.maximum_dependency_edges.max(self.edges);
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

/// Close structural producers using immutable frontiers and query dependencies.
/// The context factory must retain identical profile, binding and availability
/// identities between frontiers; only the supplied overlay may change.
/// The observer arguments match `CanonicalPublicationBuilder::build_with_progress`.
pub fn close_result_structure(
    snapshot: &Snapshot,
    options: PublicationClosureOptions,
    context: impl for<'m> FnMut(
        &'m DerivedOverlay,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError>,
    batch_progress: impl FnMut(usize, usize, usize, usize),
    progress: impl FnMut(&PublicationStage),
) -> Result<PublicationClosure, PublicationOverlayError> {
    close_result_structure_with_extension(snapshot, options, context, &(), batch_progress, progress)
}

/// Close KerML and extension producers together. Accepted immutable dependency
/// elements are never scheduled, including when explicitly listed by a caller.
pub fn close_result_structure_with_extension(
    snapshot: &Snapshot,
    options: PublicationClosureOptions,
    context: impl for<'m> FnMut(
        &'m DerivedOverlay,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError>,
    extension: &impl PublicationProducerExtension,
    batch_progress: impl FnMut(usize, usize, usize, usize),
    progress: impl FnMut(&PublicationStage),
) -> Result<PublicationClosure, PublicationOverlayError> {
    close_frontiers::<DerivedOverlay>(
        snapshot,
        options,
        context,
        extension,
        batch_progress,
        progress,
    )
}

/// Run the identical scheduler over unpublished construction frontiers. Missing
/// required values remain query obligations; this result can never certify a
/// strict publication. Accepted dependency subjects remain excluded.
pub fn close_construction_structure_with_extension(
    candidate: &std::sync::Arc<agq_kernel::ConstructionView>,
    options: PublicationClosureOptions,
    context: impl for<'m> FnMut(
        &'m agq_kernel::derived::ConstructionOverlay,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError>,
    extension: &impl PublicationProducerExtension,
    batch_progress: impl FnMut(usize, usize, usize, usize),
    progress: impl FnMut(&PublicationStage),
) -> Result<PublicationClosure<agq_kernel::derived::ConstructionOverlay>, PublicationOverlayError> {
    close_frontiers::<agq_kernel::derived::ConstructionOverlay>(
        candidate,
        options,
        context,
        extension,
        batch_progress,
        progress,
    )
}

fn close_frontiers<Overlay: ProducerFrontier>(
    input: &Overlay::Input,
    options: PublicationClosureOptions,
    mut context: impl for<'m> FnMut(&'m Overlay) -> Result<SemanticContext<'m>, PublicationOverlayError>,
    extension: &impl PublicationProducerExtension,
    mut batch_progress: impl FnMut(usize, usize, usize, usize),
    mut progress: impl FnMut(&PublicationStage),
) -> Result<PublicationClosure<Overlay>, PublicationOverlayError> {
    let mut overlay = Overlay::empty(input)?;
    let declared_model = Overlay::input_model(input);
    let mut counters = PublicationCounters {
        declared_subjects: declared_model
            .elements()
            .filter(|r| !Overlay::is_dependency_element(input, r.id()))
            .count(),
        overlay_materializations: 1, // The empty derived input is a materialization too.
        logical_search_sets: overlay.build_metrics().logical_search_sets,
        logical_search_entries: overlay.build_metrics().logical_search_entries,
        retained_search_sets: overlay.build_metrics().retained_search_sets,
        retained_search_entries: overlay.build_metrics().retained_search_entries,
        search_sets_interned: overlay.build_metrics().search_sets_interned,
        search_sets_reused: overlay.build_metrics().search_sets_reused,
        ..Default::default()
    };
    let mut stages = vec![];
    let mut index = DependencyIndex::default();
    let mut status = BTreeMap::new();
    let mut population: BTreeSet<_> = options
        .initial_subjects
        .clone()
        .unwrap_or_else(|| declared_model.elements().map(|r| r.id()).collect());
    population.retain(|id| !Overlay::is_dependency_element(input, *id));
    let mut worklist = population.clone();
    let mut seen = BTreeSet::new();
    let mut applicability = BTreeMap::<MetaclassId, Vec<ProducerFamily>>::new();
    let mut identity: Option<SemanticContextId> = None;
    let mut converged = false;
    let mut stratum = ResultStructureStratum::Structural;
    let mut deferred_bindings = BTreeSet::new();
    for round in 0..options.max_rounds {
        let context = context(&overlay)?;
        if let Some(previous) = &identity {
            // Compare all immutable inputs; only graph digest and derivation
            // phase naturally change as the additive overlay grows.
            let mut current = context.id().clone();
            current.model_digest = previous.model_digest;
            current.derivation_phase = previous.derivation_phase;
            current.library_graph_digest = previous.library_graph_digest;
            // Missing lower bounds are mutable construction frontier state,
            // not a change of semantic authority. Their participants join the
            // dirty population after each materialization below.
            current.construction_obligations = previous.construction_obligations.clone();
            if let (Some(current_targets), Some(previous_targets)) = (
                &current.formal_constraint_targets,
                &previous.formal_constraint_targets,
            ) && current_targets.same_binding_contract(previous_targets)
            {
                current.formal_constraint_targets = previous.formal_constraint_targets.clone();
            }
            if &current != previous {
                return Err(PublicationOverlayError::Derivation(
                    agq_kernel::derived::DerivationError::InputContextMismatch,
                ));
            }
        } else {
            identity = Some(context.id().clone());
        }
        let profile = context.id().options.baseline_profile;
        // Only v8 establishes the required isolation: these binding roles use
        // OwningMembership, and BindingConnector is excluded from owned-cross
        // production. Historical profiles retain their existing closure policy.
        if !profile.supports_publication_producers() {
            stratum = ResultStructureStratum::ContextualBindings;
        }
        counters.maximum_worklist_size = counters.maximum_worklist_size.max(worklist.len());
        let mut subjects = vec![];
        for subject in worklist {
            counters.subjects_considered += 1;
            let Some(record) = overlay.model().element(subject) else {
                status.insert(
                    subject,
                    (
                        Completeness::Invalid,
                        BTreeSet::from([Diagnostic {
                            code: "KQ_PRODUCER_SUBJECT",
                            subject,
                            message: "Missing producer subject".into(),
                        }]),
                    ),
                );
                continue;
            };
            let families = applicability.entry(record.metaclass()).or_insert_with(|| {
                ProducerFamily::ALL
                    .into_iter()
                    .filter(|f| f.applies(overlay.model(), record.metaclass(), profile))
                    .collect()
            });
            if families.is_empty()
                && !extension.applies(overlay.model(), record.metaclass())
                && options.strategy == PublicationClosureStrategy::Worklist
            {
                counters.subjects_skipped_by_applicability += 1;
            } else {
                subjects.push(subject);
            }
        }
        match options.order {
            PublicationWorklistOrder::Lifo => subjects.reverse(),
            PublicationWorklistOrder::ReversedInitial if round == 0 => subjects.reverse(),
            PublicationWorklistOrder::Partitioned => {
                subjects.sort_by_key(|id| (id.as_u128() % 4, *id))
            }
            _ => {}
        }
        let kerml_stratum = if stratum == ResultStructureStratum::StableProperties {
            ResultStructureStratum::Structural
        } else {
            stratum
        };
        let mut plan =
            KerMlQueries::new(context.fork()).plan_result_structure_in_stratum([], kerml_stratum);
        for (batch_index, batch) in subjects.chunks(options.batch_size.max(1)).enumerate() {
            let q = match options.strategy {
                PublicationClosureStrategy::Worklist => {
                    KerMlQueries::for_production(context.fork())
                }
                PublicationClosureStrategy::ReferenceFullScan => KerMlQueries::new(context.fork()),
            };
            if options.strategy == PublicationClosureStrategy::ReferenceFullScan {
                let mut part =
                    q.plan_result_structure_reference(batch.iter().copied(), kerml_stratum);
                for &subject in batch {
                    if extension.applies(
                        overlay.model(),
                        overlay
                            .model()
                            .element(subject)
                            .expect("scheduled subject")
                            .metaclass(),
                    ) {
                        extension.contribute(&q, subject, stratum, &mut part)?;
                    }
                }
                counters.subjects_evaluated += batch.len();
                counters.producer_families_attempted += part.producer_families_attempted;
                for &subject in batch {
                    // The reference batch shares a graph/proof accumulator;
                    // retain its conservative reads for every batch member.
                    index.replace(subject, &part.production, overlay.model(), &mut counters);
                    counters.dirty_reevaluations += usize::from(!seen.insert(subject));
                    status.insert(
                        subject,
                        (
                            part.production.completeness,
                            part.production.diagnostics.clone(),
                        ),
                    );
                }
                part.discard_aggregate_proof();
                plan.merge(part)?;
            } else {
                for &subject in batch {
                    counters.subjects_evaluated += 1;
                    counters.dirty_reevaluations += usize::from(!seen.insert(subject));
                    let mut part = q.plan_result_structure_in_stratum([subject], kerml_stratum);
                    if extension.applies(
                        overlay.model(),
                        overlay
                            .model()
                            .element(subject)
                            .expect("scheduled subject")
                            .metaclass(),
                    ) {
                        extension.contribute(&q, subject, stratum, &mut part)?;
                    }
                    counters.producer_families_attempted += part.producer_families_attempted;
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
            }
            batch_progress(
                round,
                ((batch_index + 1) * options.batch_size.max(1)).min(subjects.len()),
                subjects.len(),
                plan.planned_elements().count(),
            );
        }
        deferred_bindings.extend(plan.deferred_bindings.iter().copied());
        let mut changed = plan.changed_population();
        let prior_obligations = overlay.obligation_keys();
        let new_subjects: BTreeSet<_> = changed
            .iter()
            .copied()
            .filter(|id| overlay.model().element(*id).is_none())
            .collect();
        counters.existing_derived_facts_reused += plan.existing_elements_reused();
        let completeness = status
            .values()
            .map(|(s, _)| *s)
            .max()
            .unwrap_or(Completeness::Complete);
        let diagnostics = status
            .values()
            .flat_map(|(_, ds)| ds.iter().cloned())
            .collect();
        let input_elements = overlay.model().len();
        let input_occurrences = overlay.model().association_occurrences().count();
        // The slow reference retains and compares the complete old graph. It
        // deliberately does not trust the optimized additive change boundary.
        let reference_input = (options.strategy == PublicationClosureStrategy::ReferenceFullScan)
            .then(|| overlay.clone());
        let prepared = Overlay::prepare(plan, &overlay)?;
        drop(context);
        // Preparation validates every reused record and proposed slot even when
        // no writes remain. Only then may an empty optimized frontier reuse its
        // input. The reference still builds and compares an independent graph.
        let materialized = prepared.has_changes() || reference_input.is_some();
        let next = if materialized {
            Overlay::build(prepared, overlay)?
        } else {
            overlay
        };
        changed.extend(
            prior_obligations
                .symmetric_difference(&next.obligation_keys())
                .map(|(subject, _, _)| *subject),
        );
        let stable = reference_input.as_ref().map_or_else(
            || changed.is_empty(),
            |before| {
                before.facts_equal(&next)
                    && before.model().elements().eq(next.model().elements())
                    && before
                        .model()
                        .association_occurrences()
                        .eq(next.model().association_occurrences())
                    && before
                        .model()
                        .computation_searches()
                        .eq(next.model().computation_searches())
            },
        );
        if materialized {
            let metrics = next.build_metrics();
            counters.existing_derived_facts_reused += metrics.existing_facts_reused;
            counters.new_proof_sets_interned += metrics.proof_sets_interned;
            counters.existing_proof_sets_reused += metrics.proof_sets_reused;
            counters.dependency_edges_considered += metrics.dependency_edges_considered;
            counters.overlay_materializations += 1;
            counters.logical_search_sets = metrics.logical_search_sets;
            counters.logical_search_entries = metrics.logical_search_entries;
            counters.retained_search_sets = metrics.retained_search_sets;
            counters.retained_search_entries = metrics.retained_search_entries;
            counters.search_sets_interned += metrics.search_sets_interned;
            counters.search_sets_reused += metrics.search_sets_reused;
        } else {
            // build_metrics describes the previous actual build and must not be
            // counted again when the previous overlay is retained unchanged.
            counters.empty_frontiers_reused += 1;
        }
        counters.fixed_point_rounds += 1;
        counters.new_elements_proposed += next.model().len() - input_elements;
        counters.new_association_occurrences_proposed +=
            next.model().association_occurrences().count() - input_occurrences;
        let record = PublicationStage {
            stratum,
            counters: counters.clone(),
            stage: round,
            input_elements,
            added_elements: next.model().len() - input_elements,
            added_occurrences: next.model().association_occurrences().count() - input_occurrences,
            completeness,
            diagnostics,
        };
        progress(&record);
        stages.push(record);
        if stable {
            if stratum == ResultStructureStratum::Structural && extension.has_stable_properties() {
                stratum = ResultStructureStratum::StableProperties;
                worklist = population.clone();
                counters.dirty_subjects_enqueued += worklist.len();
                overlay = next;
                continue;
            }
            if stratum != ResultStructureStratum::ContextualBindings
                && (!deferred_bindings.is_empty()
                    || options.strategy == PublicationClosureStrategy::ReferenceFullScan)
            {
                // Structural fixed point is not publication completion. Query
                // failures remain in `status`; every deferred subject is now
                // reevaluated against the closed structural frontier.
                stratum = ResultStructureStratum::ContextualBindings;
                worklist = match options.strategy {
                    PublicationClosureStrategy::Worklist => std::mem::take(&mut deferred_bindings),
                    // The oracle scans all subjects at this transition too; it
                    // does not trust optimized deferred-subject discovery.
                    PublicationClosureStrategy::ReferenceFullScan => population.clone(),
                };
                counters.dirty_subjects_enqueued += worklist.len();
                overlay = next;
                continue;
            }
            converged = true;
            overlay = next;
            break;
        }
        population.extend(new_subjects);
        worklist = match options.strategy {
            PublicationClosureStrategy::Worklist => index
                .dirty(&changed, &mut counters)
                .intersection(&population)
                .copied()
                .collect(),
            PublicationClosureStrategy::ReferenceFullScan => {
                // Independent full scan also discovers every fresh subject.
                population.extend(
                    next.model()
                        .elements()
                        .map(|r| r.id())
                        .filter(|id| declared_model.element(*id).is_none()),
                );
                population.clone()
            }
        };
        counters.dirty_subjects_enqueued += worklist.len();
        overlay = next;
    }
    Ok(PublicationClosure {
        overlay,
        stages,
        counters,
        converged,
        producer_reads: QueryInvalidationSet::from_keys(
            index.subjects.into_values().flatten().collect(),
        ),
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

#[cfg(test)]
mod dependency_index_tests {
    use super::*;

    #[test]
    fn replaced_reads_release_empty_reverse_buckets_and_track_live_edges() {
        let first = ElementId::from_u128(1);
        let second = ElementId::from_u128(2);
        let endpoint = ElementId::from_u128(3);
        let own = InvalidationKey::Element(first);
        let incoming = InvalidationKey::Incoming(endpoint);
        let mut index = DependencyIndex::default();
        let mut counters = PublicationCounters::default();
        index.replace_keys(first, BTreeSet::from([own, incoming]), &mut counters);
        index.replace_keys(second, BTreeSet::from([incoming]), &mut counters);
        assert_eq!(counters.active_dependency_subjects, 2);
        assert_eq!(counters.active_dependency_keys, 2);
        assert_eq!(counters.active_dependency_edges, 3);

        index.replace_keys(first, BTreeSet::from([own]), &mut counters);
        assert_eq!(counters.active_dependency_keys, 2);
        assert_eq!(counters.active_dependency_edges, 2);
        assert_eq!(
            index.dirty(&BTreeSet::from([endpoint]), &mut counters),
            BTreeSet::from([endpoint, second])
        );
        index.replace_keys(second, BTreeSet::from([own]), &mut counters);
        assert!(!index.readers.contains_key(&incoming));
        assert_eq!(counters.active_dependency_keys, 1);
        assert_eq!(counters.active_dependency_edges, 2);
        assert_eq!(counters.maximum_dependency_keys, 2);
        assert_eq!(counters.maximum_dependency_edges, 3);
        assert_eq!(
            index.readers.values().map(BTreeSet::len).sum::<usize>(),
            counters.active_dependency_edges
        );
        assert_eq!(
            index
                .subjects
                .values()
                .map(|keys| keys.len())
                .sum::<usize>(),
            counters.active_dependency_edges
        );
        assert_eq!(
            index.dirty(&BTreeSet::from([endpoint]), &mut counters),
            BTreeSet::from([endpoint])
        );
    }
}
