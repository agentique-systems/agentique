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
mod metrics;
use frontier::ProducerFrontier;
pub(crate) use metrics::FamilyTimer;
use metrics::ReopenReasons;
pub use metrics::{PublicationFamilyMetrics, PublicationReopenReason, PublicationRoundMetrics};
use std::time::Instant;

#[path = "publication_frontier.rs"]
mod checkpoint;
pub use checkpoint::{PublicationFrontierSession, PublicationFrontierStatistics};

/// Additional language producers participating in the same immutable frontiers
/// and positive/negative read index as KerML. Contributions must carry their
/// actual query evidence and stable rule/output identities.
pub trait PublicationProducerExtension {
    /// Complete immutable potential-effect registry. A legacy extension with
    /// no descriptors remains usable but cannot establish closure evidence.
    fn descriptors(&self) -> Vec<ProducerDescriptor> {
        vec![]
    }
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
    FeatureValuation,
}
impl ProducerFamily {
    pub const fn id(self) -> ProducerFamilyId {
        ProducerFamilyId::new(match self {
            Self::OwnedInstantiationResult => "KerML.OwnedInstantiationResult",
            Self::PositionalRedefinition => "KerML.PositionalRedefinition",
            Self::VariableFeaturing => "KerML.VariableFeaturing",
            Self::OwnedCrossing => "KerML.OwnedCrossing",
            Self::CrossDomain => "KerML.CrossDomain",
            Self::Invocation => "KerML.Invocation",
            Self::FeatureChainExpression => "KerML.FeatureChainExpression",
            Self::FeatureReferenceExpression => "KerML.FeatureReferenceExpression",
            Self::ExpressionResult => "KerML.ExpressionResult",
            Self::FeatureValue => "KerML.FeatureValue",
            Self::IndexSelectResult => "KerML.IndexSelectResult",
            Self::FeatureValuation => "KerML.FeatureValuation",
        })
    }
    pub fn descriptor(self, profile: BaselineProfile) -> ProducerDescriptor {
        use ProducerEffect as E;
        let (classes, effects): (Vec<_>, Vec<_>) = match self {
            Self::OwnedInstantiationResult => (
                vec![c::INSTANTIATION_EXPRESSION],
                vec![E::Membership, E::ResultStructure],
            ),
            Self::PositionalRedefinition => (vec![c::FEATURE], vec![E::Redefinition]),
            // Snapshot creation changes the owning Type's member population;
            // its new snapshot's redefinition cannot retype the owning Type.
            Self::VariableFeaturing => (
                vec![c::FEATURE],
                vec![E::Featuring, E::Membership, E::ResultStructure],
            ),
            Self::OwnedCrossing => (
                vec![c::FEATURE],
                vec![E::FeatureChain, E::Subsetting, E::ResultStructure],
            ),
            Self::CrossDomain => (
                vec![c::FEATURE],
                vec![
                    E::Typing,
                    E::Subsetting,
                    E::Featuring,
                    E::FeatureChain,
                    E::ResultStructure,
                ],
            ),
            Self::Invocation => (
                vec![c::INVOCATION_EXPRESSION],
                vec![
                    E::Specialization,
                    E::Subsetting,
                    E::Typing,
                    E::Membership,
                    E::ValueBinding,
                    E::ConnectorStructure,
                ],
            ),
            Self::FeatureChainExpression => (
                vec![c::FEATURE_CHAIN_EXPRESSION],
                vec![
                    E::Specialization,
                    E::Redefinition,
                    E::FeatureChain,
                    E::ResultStructure,
                ],
            ),
            Self::FeatureReferenceExpression => (
                vec![c::FEATURE_REFERENCE_EXPRESSION],
                vec![E::ValueBinding, E::ConnectorStructure],
            ),
            Self::ExpressionResult => (
                vec![c::EXPRESSION, c::FUNCTION],
                vec![
                    E::Membership,
                    E::ValueBinding,
                    E::ResultStructure,
                    E::ConnectorStructure,
                ],
            ),
            Self::FeatureValue => (
                vec![c::FEATURE],
                vec![
                    E::Membership,
                    E::ValueBinding,
                    E::ResultStructure,
                    E::ConnectorStructure,
                ],
            ),
            Self::FeatureValuation => (
                vec![c::FEATURE],
                vec![E::Subsetting, E::Membership, E::ResultStructure],
            ),
            Self::IndexSelectResult => (
                vec![c::INDEX_EXPRESSION, c::SELECT_EXPRESSION],
                vec![
                    E::Subsetting,
                    E::FeatureChain,
                    E::Membership,
                    E::ResultStructure,
                ],
            ),
        };
        let enabled = match self {
            Self::OwnedInstantiationResult
            | Self::PositionalRedefinition
            | Self::VariableFeaturing
            | Self::Invocation
            | Self::FeatureChainExpression => profile.supports_publication_producers(),
            Self::OwnedCrossing | Self::CrossDomain => profile.corrects_owned_cross_domain(),
            _ => true,
        };
        let mut descriptor = ProducerDescriptor::new(
            self.id(),
            effects,
            if enabled {
                ProducerApplicability::Subtypes(classes)
            } else {
                ProducerApplicability::Never
            },
        );
        descriptor.fresh_effects = match self {
            Self::OwnedInstantiationResult => {
                vec![E::Scalar(agq_kerml::properties::FEATURE_DIRECTION)]
            }
            Self::PositionalRedefinition => vec![],
            Self::VariableFeaturing => vec![
                E::Redefinition,
                E::Subsetting,
                E::Specialization,
                E::Membership,
                E::ResultStructure,
            ],
            Self::OwnedCrossing => vec![E::Typing, E::Featuring, E::FeatureChain, E::Membership],
            Self::CrossDomain => vec![
                E::Typing,
                E::Featuring,
                E::Subsetting,
                E::Specialization,
                E::Membership,
            ],
            Self::FeatureChainExpression => vec![E::FeatureChain, E::Redefinition, E::Membership],
            Self::FeatureValuation => vec![E::FeatureChain, E::Membership],
            Self::Invocation
            | Self::FeatureReferenceExpression
            | Self::ExpressionResult
            | Self::FeatureValue => vec![
                E::Subsetting,
                E::Featuring,
                E::Membership,
                E::FeatureChain,
                E::Scalar(agq_kerml::properties::FEATURE_IS_END),
            ],
            Self::IndexSelectResult => vec![E::FeatureChain, E::Membership],
        }
        .into_iter()
        .collect();
        if self == Self::VariableFeaturing {
            // TypeFeaturing is emitted only on the variable Feature. Snapshot
            // creation changes its owner's membership, not the owner's featuring.
            descriptor
                .effect_scopes
                .insert(E::Featuring, ProducerEffectScope::Subject);
            // Existing effects concern the variable Feature and its owning
            // Type, never intermediate membership carrier records.
            descriptor.effect_targets = Some(BTreeSet::from([c::TYPE]));
            descriptor.feature_populations = Some(BTreeSet::new());
            descriptor.relationship_classes = Some(
                [c::TYPE_FEATURING, c::FEATURE_MEMBERSHIP, c::REDEFINITION]
                    .into_iter()
                    .collect(),
            );
        }
        if self == Self::ExpressionResult {
            // The expression/function gains only contextual-result and binding
            // memberships. Chaining, featuring and subsetting concern newly
            // created helper features, never the existing producer subject.
            // In particular no FeatureValue carrier is introduced by this rule.
            // Its contextual Feature and binding Connector are undirected;
            // only the fresh binding ends enter a positional population.
            descriptor.feature_populations =
                Some(BTreeSet::from([crate::FeaturePopulationKind::End]));
            descriptor.relationship_classes = Some(
                [
                    c::OWNING_MEMBERSHIP,
                    c::FEATURE_MEMBERSHIP,
                    c::END_FEATURE_MEMBERSHIP,
                    c::FEATURE_CHAINING,
                    c::REFERENCE_SUBSETTING,
                    c::TYPE_FEATURING,
                    c::BINDING_CONNECTOR,
                ]
                .into_iter()
                .collect(),
            );
        }
        if matches!(self, Self::FeatureReferenceExpression | Self::FeatureValue) {
            let mut classes = BTreeSet::from([
                c::OWNING_MEMBERSHIP,
                c::END_FEATURE_MEMBERSHIP,
                c::REFERENCE_SUBSETTING,
                c::TYPE_FEATURING,
                c::BINDING_CONNECTOR,
            ]);
            if self == Self::FeatureValue {
                classes.insert(c::FEATURE_CHAINING);
            }
            descriptor.relationship_classes = Some(classes);
            descriptor.minimum_stratum = ResultStructureStratum::ContextualBindings;
        }
        if self == Self::FeatureValuation {
            descriptor.derivation_rules = Some(BTreeSet::from([crate::result_structure::rule_id(
                profile,
                "checkFeatureValuationSpecialization",
            )]));
            descriptor.relationship_classes = Some(BTreeSet::from([
                c::SUBSETTING,
                c::OWNING_MEMBERSHIP,
                c::FEATURE_CHAINING,
            ]));
        }
        if self == Self::FeatureValue {
            descriptor.derivation_rules = Some(BTreeSet::from([
                crate::ImpliedBindingRole::FeatureValue.rule_id(profile),
                crate::result_structure::rule_id(profile, "initial-feature-value-context/1"),
            ]));
        }
        if self == Self::FeatureChainExpression {
            descriptor.effects.insert(E::Subsetting);
            // A missing source-target Feature is created below the existing
            // first input Feature, so its membership is an existing-subject write.
            descriptor.effects.insert(E::Membership);
        }
        if self == Self::FeatureReferenceExpression {
            descriptor.effects.insert(E::Membership);
        }
        if self == Self::FeatureReferenceExpression {
            descriptor.minimum_stratum = ResultStructureStratum::ContextualBindings;
        }
        descriptor.scope = match self {
            Self::VariableFeaturing => ProducerEffectScope::SubjectAndOwningType,
            Self::Invocation => ProducerEffectScope::SubjectAndOwnedResults,
            Self::FeatureChainExpression => ProducerEffectScope::SubjectAndOwnedFeatures,
            Self::IndexSelectResult if profile.supports_publication_producers() => {
                // These profiles stage an owned instantiation result and require
                // its exact ownership before emitting Index/Select subsetting.
                ProducerEffectScope::SubjectAndOwnedResults
            }
            Self::IndexSelectResult => ProducerEffectScope::SubjectAndOwned,
            _ => ProducerEffectScope::Subject,
        };
        descriptor.scoped_fresh_ownership = true;
        descriptor
    }
    pub const ALL: [Self; 12] = [
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
        Self::FeatureValuation,
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
            Self::FeatureValue | Self::FeatureValuation => is(c::FEATURE),
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
    /// Optional durable, unaccepted scheduler frontiers. A restored session is
    /// authenticated by an independently retained journal digest and input pins.
    pub frontier_checkpoints: Option<std::sync::Arc<PublicationFrontierSession>>,
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
            frontier_checkpoints: None,
            initial_subjects: None,
            max_rounds: 32,
            batch_size: 32,
            order: Default::default(),
            strategy: Default::default(),
        }
    }
}
/// Work accounting and bounded diagnostic observations. Neither counts nor
/// elapsed times participate in acceptance or semantic identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PublicationCounters {
    /// Latest completed frontier, including its own certificate work.
    #[serde(skip)]
    pub round: PublicationRoundMetrics,
    pub negative_queries_certified: usize,
    pub families_registered: usize,
    pub applicable_subject_family_pairs: usize,
    pub closed_producer_pairs: usize,
    pub closed_producer_effects: usize,
    pub incomplete_producer_pairs: usize,
    pub certificate_bytes: usize,
    pub certificate_build_micros: u128,
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
    /// Exact final frontier, even when only some effect requirements closed.
    pub certificate: Option<std::sync::Arc<ProducerClosureCertificate>>,
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
    #[cfg(test)]
    fn dirty(
        &self,
        changed: &BTreeSet<ElementId>,
        counters: &mut PublicationCounters,
    ) -> BTreeSet<ElementId> {
        self.dirty_with_reasons(changed, counters, &mut BTreeMap::new())
    }
    fn dirty_with_reasons(
        &self,
        changed: &BTreeSet<ElementId>,
        counters: &mut PublicationCounters,
        reasons: &mut BTreeMap<ElementId, ReopenReasons>,
    ) -> BTreeSet<ElementId> {
        let mut result = changed.clone();
        for &subject in changed {
            reasons
                .entry(subject)
                .or_default()
                .insert(PublicationReopenReason::GraphFactChanged);
        }
        for key in std::iter::once(InvalidationKey::Global).chain(
            changed
                .iter()
                .flat_map(|&id| [InvalidationKey::Element(id), InvalidationKey::Incoming(id)]),
        ) {
            if let Some(readers) = self.readers.get(&key) {
                counters.dependency_edges_considered += readers.len();
                result.extend(readers);
                for &subject in readers {
                    let subject_reasons = reasons.entry(subject).or_default();
                    match key {
                        InvalidationKey::Element(_) => {
                            subject_reasons.insert(PublicationReopenReason::GraphFactChanged)
                        }
                        InvalidationKey::Incoming(_) => {
                            subject_reasons.insert(PublicationReopenReason::ProviderSearchChanged);
                            subject_reasons
                                .insert(PublicationReopenReason::NewRelationshipEndpoint);
                        }
                        InvalidationKey::Global => {
                            subject_reasons.insert(PublicationReopenReason::ProviderSearchChanged)
                        }
                    }
                }
            }
        }
        result
    }
}

fn finish_stage(
    mut stage: PublicationStage,
    mut metrics: PublicationRoundMetrics,
    counters: &mut PublicationCounters,
    pending_reasons: &BTreeMap<ElementId, ReopenReasons>,
    next_worklist: &BTreeSet<ElementId>,
    stages: &mut Vec<PublicationStage>,
    progress: &mut impl FnMut(&PublicationStage),
) {
    metrics.next_dirty_subjects = next_worklist.len();
    for subject in next_worklist {
        for reason in pending_reasons
            .get(subject)
            .copied()
            .unwrap_or_default()
            .iter()
        {
            *metrics.next_dirty_by_reason.entry(reason).or_default() += 1;
        }
    }
    counters.round = metrics;
    stage.counters = counters.clone();
    progress(&stage);
    stages.push(stage);
}

fn record_family_reopens(plan: &mut ResultStructurePlan<'_>, reasons: ReopenReasons) {
    for metrics in plan.family_metrics.values_mut() {
        for reason in reasons.iter() {
            *metrics.reopened_by_reason.entry(reason).or_default() += 1;
        }
    }
}

fn contribute_with_metrics<'m>(
    extension: &impl PublicationProducerExtension,
    queries: &KerMlQueries<'m>,
    subject: ElementId,
    stratum: ResultStructureStratum,
    plan: &mut ResultStructurePlan<'m>,
) -> Result<(), agq_kernel::derived::DerivationError> {
    let offset = plan.producer_evaluations.len();
    let started = Instant::now();
    extension.contribute(queries, subject, stratum, plan)?;
    let elapsed = started.elapsed().as_micros();
    // Extension callbacks may implement several registered families together.
    // Attribute their shared work to actual recorded evaluations only.
    for (_, family, _) in &plan.producer_evaluations[offset..] {
        let metrics = plan.family_metrics.entry(*family).or_default();
        metrics.attempts += 1;
        metrics.shared_planning_micros += elapsed;
    }
    Ok(())
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
        None,
        options,
        context,
        extension,
        batch_progress,
        progress,
    )
}

/// Reevaluate the complete producer registry over an existing strictly
/// validated overlay. Existing generated subjects are included. This preserves
/// immutable graph sharing while issuing fresh evidence under the caller's
/// exact current contract; no previous evaluation status is trusted.
pub fn close_result_structure_on_overlay_with_extension(
    overlay: DerivedOverlay,
    options: PublicationClosureOptions,
    context: impl for<'m> FnMut(
        &'m DerivedOverlay,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError>,
    extension: &impl PublicationProducerExtension,
    batch_progress: impl FnMut(usize, usize, usize, usize),
    progress: impl FnMut(&PublicationStage),
) -> Result<PublicationClosure, PublicationOverlayError> {
    let snapshot = overlay.declared().clone();
    close_frontiers::<DerivedOverlay>(
        &snapshot,
        Some(overlay),
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
        None,
        options,
        context,
        extension,
        batch_progress,
        progress,
    )
}

fn close_frontiers<Overlay: ProducerFrontier>(
    input: &Overlay::Input,
    initial_overlay: Option<Overlay>,
    options: PublicationClosureOptions,
    mut context_factory: impl for<'m> FnMut(
        &'m Overlay,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError>,
    extension: &impl PublicationProducerExtension,
    mut batch_progress: impl FnMut(usize, usize, usize, usize),
    mut progress: impl FnMut(&PublicationStage),
) -> Result<PublicationClosure<Overlay>, PublicationOverlayError> {
    let mut overlay = if let Some(overlay) = initial_overlay {
        overlay
    } else {
        Overlay::empty(input)?
    };
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
        .unwrap_or_else(|| overlay.model().elements().map(|r| r.id()).collect());
    population.retain(|id| !Overlay::is_dependency_element(input, *id));
    let mut worklist = population.clone();
    let mut seen = BTreeSet::new();
    let mut applicability = BTreeMap::<MetaclassId, Vec<ProducerFamily>>::new();
    let mut identity: Option<SemanticContextId> = None;
    let mut converged = false;
    let mut registry: Option<ProducerRegistry> = None;
    let mut evaluations = crate::producer_closure::ProducerEvaluationTable::default();
    let mut certificate: Option<std::sync::Arc<ProducerClosureCertificate>> = None;
    let mut certificate_builder = crate::producer_closure::ClosureCertificateBuilder::default();
    let extension_descriptors = extension.descriptors();
    let mut unregistered_extension = extension_descriptors.is_empty()
        && population.iter().any(|&subject| {
            overlay
                .model()
                .element(subject)
                .is_some_and(|record| extension.applies(overlay.model(), record.metaclass()))
        });
    let mut stratum = ResultStructureStratum::Structural;
    let mut deferred_bindings = BTreeSet::new();
    let mut pending_reasons = BTreeMap::<ElementId, ReopenReasons>::new();
    let mut first_round = 0;
    let invocation = if let Some(session) = &options.frontier_checkpoints {
        let initial = context_factory(&overlay)?;
        let combined = ProducerRegistry::new(
            ProducerFamily::ALL
                .into_iter()
                .map(|family| family.descriptor(initial.id().options.baseline_profile))
                .chain(extension_descriptors.iter().cloned()),
        )
        .expect("producer family identities must be unique");
        let initial = initial
            .with_producer_registry_digest(combined.digest())
            .map_err(PublicationOverlayError::Context)?;
        if let Some(witness) = initial.producer_closure() {
            evaluations = witness.evaluation_table();
            certificate = Some(witness.clone());
        }
        let invocation =
            session.begin(initial.id(), &options, extension.has_stable_properties())?;
        drop(initial);
        if let Some((restored, state)) = invocation.restore::<Overlay>(input)? {
            overlay = restored;
            let restored_context = context_factory(&overlay)?
                .with_producer_registry_digest(combined.digest())
                .map_err(PublicationOverlayError::Context)?;
            if restored_context.id().model_digest != state.model_digest
                || restored_context.id().closure_contract_digest() != state.context_contract
                || state.round > options.max_rounds
            {
                return Err(PublicationOverlayError::FrontierCheckpoint(
                    "restored graph/context/round mismatch".into(),
                ));
            }
            let witness = checkpoint::restore_certificate(
                state.certificate,
                restored_context.id(),
                &combined,
                overlay.model(),
            )?;
            evaluations = witness.evaluation_table();
            evaluations
                .restore_frontier_rows(state.evaluation_rows, &combined, overlay.model())
                .ok_or_else(|| {
                    PublicationOverlayError::FrontierCheckpoint(
                        "restored evaluation state mismatch".into(),
                    )
                })?;
            certificate = Some(witness);
            identity = Some(restored_context.id().clone());
            first_round = state.round;
            converged = state.converged;
            stratum = state.stratum;
            stages = state.stages;
            status = state.status;
            population = state.population;
            worklist = state.worklist;
            seen = state.seen;
            deferred_bindings = state.deferred_bindings;
            for (subject, keys) in state.dependencies {
                index.replace_keys(subject, keys.into_iter().collect(), &mut counters);
            }
            counters = state.counters;
        }
        registry = Some(combined);
        Some(invocation)
    } else {
        None
    };
    for round in first_round..if converged {
        first_round
    } else {
        options.max_rounds
    } {
        let round_started = Instant::now();
        let evaluated_before = counters.subjects_evaluated;
        let skipped_before = counters.subjects_skipped_by_applicability;
        let reopened_before = counters.dirty_reevaluations;
        let certificate_before = counters.certificate_build_micros;
        let mut round_metrics = PublicationRoundMetrics::default();
        let mut current_context = context_factory(&overlay)?;
        let registry = registry.get_or_insert_with(|| {
            let descriptors = ProducerFamily::ALL
                .into_iter()
                .map(|f| f.descriptor(current_context.id().options.baseline_profile))
                .chain(extension_descriptors.iter().cloned());
            ProducerRegistry::new(descriptors).expect("producer family identities must be unique")
        });
        counters.families_registered = registry.descriptors().len();
        current_context = current_context
            .with_producer_registry_digest(registry.digest())
            .map_err(PublicationOverlayError::Context)?;
        if round == 0
            && let Some(witness) = current_context.producer_closure()
        {
            evaluations = witness.evaluation_table();
            certificate = Some(witness.clone());
        }
        if certificate.is_none() && !unregistered_extension {
            let started = Instant::now();
            certificate = Some(std::sync::Arc::new(
                ProducerClosureCertificate::initial(&current_context, registry)
                    .map_err(PublicationOverlayError::Context)?,
            ));
            counters.certificate_build_micros += started.elapsed().as_micros();
        }
        if let Some(witness) = &certificate {
            if witness.compatible_context(current_context.id()) {
                current_context = current_context
                    .with_producer_closure(witness.clone())
                    .map_err(PublicationOverlayError::Context)?;
            } else {
                certificate = None;
            }
        }
        let context = current_context;
        if let Some(invocation) = &invocation
            && round != first_round
            && invocation.should_capture(
                round,
                stratum,
                stages.last().map(|stage| stage.stratum),
                false,
            )
        {
            capture_frontier(
                invocation,
                &overlay,
                &context,
                round,
                false,
                stratum,
                &stages,
                &counters,
                &status,
                &population,
                &worklist,
                &seen,
                &deferred_bindings,
                &index,
                &evaluations,
                certificate.as_deref(),
            )?;
        }
        if let Some(previous) = &identity {
            // Compare all immutable inputs; only graph digest and derivation
            // phase naturally change as the additive overlay grows.
            let mut current = context.id().clone();
            current.model_digest = previous.model_digest;
            current.derivation_phase = previous.derivation_phase;
            current.library_graph_digest = previous.library_graph_digest;
            current.producer_closure_digest = previous.producer_closure_digest;
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
                return Err(PublicationOverlayError::SchedulerContextMismatch {
                    changed_fields: current.differing_fields(previous),
                });
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
            evaluations.pending(subject, overlay.model(), registry);
            unregistered_extension |= extension_descriptors.is_empty()
                && extension.applies(overlay.model(), record.metaclass());
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
        let planning_started = Instant::now();
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
                        contribute_with_metrics(extension, &q, subject, stratum, &mut part)?;
                    }
                }
                counters.subjects_evaluated += batch.len();
                counters.producer_families_attempted += part.producer_families_attempted;
                for &subject in batch {
                    // The reference batch shares a graph/proof accumulator;
                    // retain its conservative reads for every batch member.
                    let index_started = Instant::now();
                    index.replace(subject, &part.production, overlay.model(), &mut counters);
                    round_metrics.dependency_index_micros += index_started.elapsed().as_micros();
                    counters.dirty_reevaluations += usize::from(!seen.insert(subject));
                    status.insert(
                        subject,
                        (
                            part.production.completeness,
                            part.production.diagnostics.clone(),
                        ),
                    );
                }
                if !unregistered_extension {
                    part.validate_declared_effects(batch, registry)?;
                }
                evaluations.record(&part.producer_evaluations, registry)?;
                evaluations.record_reads(&part.producer_reads, registry);
                part.producer_reads.clear();
                part.producer_evaluations.clear();
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
                        contribute_with_metrics(extension, &q, subject, stratum, &mut part)?;
                    }
                    counters.producer_families_attempted += part.producer_families_attempted;
                    let index_started = Instant::now();
                    index.replace(subject, &part.production, overlay.model(), &mut counters);
                    round_metrics.dependency_index_micros += index_started.elapsed().as_micros();
                    status.insert(
                        subject,
                        (
                            part.production.completeness,
                            part.production.diagnostics.clone(),
                        ),
                    );
                    if !unregistered_extension {
                        part.validate_declared_effects(&[subject], registry)?;
                    }
                    evaluations.record(&part.producer_evaluations, registry)?;
                    evaluations.record_reads(&part.producer_reads, registry);
                    part.producer_reads.clear();
                    part.producer_evaluations.clear();
                    part.discard_aggregate_proof();
                    record_family_reopens(
                        &mut part,
                        pending_reasons.get(&subject).copied().unwrap_or_default(),
                    );
                    plan.merge(part)?;
                }
            }
            counters.negative_queries_certified += q.negative_queries_certified();
            batch_progress(
                round,
                ((batch_index + 1) * options.batch_size.max(1)).min(subjects.len()),
                subjects.len(),
                plan.planned_elements().count(),
            );
        }
        round_metrics.planning_micros = planning_started
            .elapsed()
            .as_micros()
            .saturating_sub(round_metrics.dependency_index_micros);
        round_metrics.subjects_evaluated = counters.subjects_evaluated - evaluated_before;
        round_metrics.subjects_skipped =
            counters.subjects_skipped_by_applicability - skipped_before;
        round_metrics.subjects_reopened = counters.dirty_reevaluations - reopened_before;
        round_metrics.planned_elements = plan.planned_elements().count();
        round_metrics.families = std::mem::take(&mut plan.family_metrics);
        pending_reasons.clear();
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
        let materialization_started = Instant::now();
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
        round_metrics.model_materialization_micros = materialization_started.elapsed().as_micros();
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
        round_metrics.accepted_elements = next.model().len() - input_elements;
        round_metrics.accepted_occurrences =
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
        if stable {
            if !unregistered_extension {
                let started = std::time::Instant::now();
                let next_context = context_factory(&next)?
                    .with_producer_registry_digest(registry.digest())
                    .map_err(PublicationOverlayError::Context)?;
                let issued = std::sync::Arc::new(
                    if options.strategy == PublicationClosureStrategy::ReferenceFullScan {
                        ProducerClosureCertificate::issue(
                            next.model(),
                            next_context.id(),
                            registry,
                            &evaluations,
                            |id| next_context.dependency_closure_source(id),
                        )
                    } else {
                        certificate_builder.issue(
                            next.model(),
                            next_context.id(),
                            registry,
                            &evaluations,
                            &changed,
                            |id| next_context.dependency_closure_source(id),
                        )
                    },
                );
                counters.applicable_subject_family_pairs = issued.applicable_pairs();
                counters.closed_producer_pairs = issued.closed_pairs();
                counters.closed_producer_effects = issued.closed_effects();
                counters.incomplete_producer_pairs = issued.incomplete_pairs();
                counters.certificate_bytes = issued.storage_bytes();
                counters.certificate_build_micros += started.elapsed().as_micros();
                let changed_witness = certificate
                    .as_ref()
                    .is_none_or(|old| old.digest() != issued.digest());
                certificate = Some(issued);
                evaluations.prune_unused_reads();
                if changed_witness
                    && status
                        .values()
                        .any(|(state, _)| *state != Completeness::Complete)
                {
                    worklist = status
                        .iter()
                        .filter_map(|(&id, (state, _))| {
                            (*state != Completeness::Complete).then_some(id)
                        })
                        .collect();
                    counters.dirty_subjects_enqueued += worklist.len();
                    for &subject in &worklist {
                        pending_reasons
                            .entry(subject)
                            .or_default()
                            .insert(PublicationReopenReason::CertificateTransportChanged);
                    }
                    round_metrics.certificate_build_micros =
                        counters.certificate_build_micros - certificate_before;
                    round_metrics.elapsed_micros = round_started.elapsed().as_micros();
                    finish_stage(
                        record,
                        round_metrics,
                        &mut counters,
                        &pending_reasons,
                        &worklist,
                        &mut stages,
                        &mut progress,
                    );
                    overlay = next;
                    continue;
                }
            }
            if stratum == ResultStructureStratum::Structural && extension.has_stable_properties() {
                stratum = ResultStructureStratum::StableProperties;
                worklist = population.clone();
                counters.dirty_subjects_enqueued += worklist.len();
                for &subject in &worklist {
                    pending_reasons
                        .entry(subject)
                        .or_default()
                        .insert(PublicationReopenReason::NewProducerOpportunity);
                }
                round_metrics.certificate_build_micros =
                    counters.certificate_build_micros - certificate_before;
                round_metrics.elapsed_micros = round_started.elapsed().as_micros();
                finish_stage(
                    record,
                    round_metrics,
                    &mut counters,
                    &pending_reasons,
                    &worklist,
                    &mut stages,
                    &mut progress,
                );
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
                for &subject in &worklist {
                    pending_reasons
                        .entry(subject)
                        .or_default()
                        .insert(PublicationReopenReason::ContextualBindingDependency);
                }
                round_metrics.certificate_build_micros =
                    counters.certificate_build_micros - certificate_before;
                round_metrics.elapsed_micros = round_started.elapsed().as_micros();
                finish_stage(
                    record,
                    round_metrics,
                    &mut counters,
                    &pending_reasons,
                    &worklist,
                    &mut stages,
                    &mut progress,
                );
                overlay = next;
                continue;
            }
            round_metrics.certificate_build_micros =
                counters.certificate_build_micros - certificate_before;
            round_metrics.elapsed_micros = round_started.elapsed().as_micros();
            finish_stage(
                record,
                round_metrics,
                &mut counters,
                &pending_reasons,
                &BTreeSet::new(),
                &mut stages,
                &mut progress,
            );
            converged = true;
            overlay = next;
            break;
        }
        // Revalidate evaluations before rebinding evidence to the changed graph.
        // Unaffected read sets survive; new families/subjects and changed reads
        // reopen, and issue recomputes upstream causal blockers and closed masks.
        let revalidation_started = Instant::now();
        evaluations.invalidate(&changed);
        round_metrics.certificate_revalidation_micros += revalidation_started.elapsed().as_micros();
        certificate = if unregistered_extension {
            None
        } else {
            let started = std::time::Instant::now();
            let next_context = context_factory(&next)?
                .with_producer_registry_digest(registry.digest())
                .map_err(PublicationOverlayError::Context)?;
            let issued = std::sync::Arc::new(
                if options.strategy == PublicationClosureStrategy::ReferenceFullScan {
                    ProducerClosureCertificate::issue(
                        next.model(),
                        next_context.id(),
                        registry,
                        &evaluations,
                        |id| next_context.dependency_closure_source(id),
                    )
                } else {
                    certificate_builder.issue(
                        next.model(),
                        next_context.id(),
                        registry,
                        &evaluations,
                        &changed,
                        |id| next_context.dependency_closure_source(id),
                    )
                },
            );
            counters.certificate_build_micros += started.elapsed().as_micros();
            counters.applicable_subject_family_pairs = issued.applicable_pairs();
            counters.closed_producer_pairs = issued.closed_pairs();
            counters.closed_producer_effects = issued.closed_effects();
            counters.incomplete_producer_pairs = issued.incomplete_pairs();
            counters.certificate_bytes = issued.storage_bytes();
            Some(issued)
        };
        evaluations.prune_unused_reads();
        for &subject in &new_subjects {
            let reasons = pending_reasons.entry(subject).or_default();
            reasons.insert(PublicationReopenReason::NewHelperCreated);
            reasons.insert(PublicationReopenReason::NewProducerOpportunity);
        }
        population.extend(new_subjects);
        worklist = match options.strategy {
            PublicationClosureStrategy::Worklist => index
                .dirty_with_reasons(&changed, &mut counters, &mut pending_reasons)
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
        round_metrics.certificate_build_micros =
            counters.certificate_build_micros - certificate_before;
        round_metrics.elapsed_micros = round_started.elapsed().as_micros();
        finish_stage(
            record,
            round_metrics,
            &mut counters,
            &pending_reasons,
            &worklist,
            &mut stages,
            &mut progress,
        );
        overlay = next;
    }
    if let Some(invocation) = &invocation
        && converged
        && counters.fixed_point_rounds > first_round
    {
        let final_context = context_factory(&overlay)?
            .with_producer_registry_digest(
                registry
                    .as_ref()
                    .expect("registered checkpoint invocation")
                    .digest(),
            )
            .map_err(PublicationOverlayError::Context)?;
        capture_frontier(
            invocation,
            &overlay,
            &final_context,
            counters.fixed_point_rounds,
            true,
            stratum,
            &stages,
            &counters,
            &status,
            &population,
            &BTreeSet::new(),
            &seen,
            &deferred_bindings,
            &index,
            &evaluations,
            certificate.as_deref(),
        )?;
    }
    Ok(PublicationClosure {
        overlay,
        stages,
        counters,
        converged,
        certificate,
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

#[allow(clippy::too_many_arguments)]
fn capture_frontier<Overlay: ProducerFrontier>(
    invocation: &checkpoint::Invocation<'_>,
    overlay: &Overlay,
    context: &SemanticContext<'_>,
    round: usize,
    converged: bool,
    stratum: ResultStructureStratum,
    stages: &[PublicationStage],
    counters: &PublicationCounters,
    status: &BTreeMap<ElementId, (Completeness, BTreeSet<Diagnostic>)>,
    population: &BTreeSet<ElementId>,
    worklist: &BTreeSet<ElementId>,
    seen: &BTreeSet<ElementId>,
    deferred_bindings: &BTreeSet<ElementId>,
    index: &DependencyIndex,
    evaluations: &crate::producer_closure::ProducerEvaluationTable,
    certificate: Option<&ProducerClosureCertificate>,
) -> Result<(), PublicationOverlayError> {
    let certificate = certificate.ok_or_else(|| {
        PublicationOverlayError::FrontierCheckpoint(
            "unregistered producers cannot be checkpointed".into(),
        )
    })?;
    if !certificate.compatible_context(context.id()) {
        return Err(PublicationOverlayError::FrontierCheckpoint(
            "checkpoint certificate context mismatch".into(),
        ));
    }
    invocation.capture(
        overlay,
        &checkpoint::State {
            round,
            converged,
            stratum,
            model_digest: context.id().model_digest,
            context_contract: context.id().closure_contract_digest(),
            stages: stages.to_vec(),
            counters: counters.clone(),
            status: status.clone(),
            population: population.clone(),
            worklist: worklist.clone(),
            seen: seen.clone(),
            deferred_bindings: deferred_bindings.clone(),
            dependencies: index.subjects.clone(),
            certificate: certificate.frontier_state(),
            evaluation_rows: evaluations.frontier_rows(),
        },
    )
}

#[cfg(test)]
mod dependency_index_tests {
    use super::*;

    #[test]
    fn changed_interpretation_at_later_stratum_reports_exact_context_field() {
        let snapshot = Snapshot::new(std::sync::Arc::new(
            agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
        ));
        let mut context_calls = 0;
        let result = close_result_structure(
            &snapshot,
            PublicationClosureOptions {
                strategy: PublicationClosureStrategy::ReferenceFullScan,
                ..Default::default()
            },
            |overlay| {
                // First frontier and its issued witness have the same contract.
                // The contextual-binding frontier deliberately changes it.
                let contract = if context_calls < 2 { [1; 32] } else { [2; 32] };
                context_calls += 1;
                SemanticContext::for_overlay(
                    overlay,
                    SemanticOptions {
                        baseline_profile: BaselineProfile::OPERATIONAL_V9,
                        ..Default::default()
                    },
                    BTreeSet::new(),
                )
                .and_then(|context| {
                    context.with_semantic_extension_identity("fixture-scheduler/1", contract)
                })
                .map_err(PublicationOverlayError::Context)
            },
            |_, _, _, _| {},
            |_| {},
        );
        assert!(
            matches!(
                &result,
                Err(PublicationOverlayError::SchedulerContextMismatch { changed_fields })
                    if changed_fields == &["semantic_extensions"]
            ),
            "unexpected context rejection: {:?}; context calls: {context_calls}",
            result.as_ref().err(),
        );
        assert_eq!(context_calls, 3);
    }

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
