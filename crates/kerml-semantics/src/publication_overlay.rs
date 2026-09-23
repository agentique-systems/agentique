//! Canonical publication closure, independent of conformance validation coverage.
use crate::read_dependencies::{
    InvalidationKey, publication_dependency_keys, query_publication_provider_keys, query_read_keys,
};
use crate::*;
use agq_kerml::{BaselineProfile, classes as c};
use agq_kernel::{
    ElementId, ModelView, Snapshot,
    derived::{ConstructionOverlay, DerivationError, DerivedOverlay},
    provenance::{DeclaredOrigin, Dependency, FactKey, Origin},
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "publication_restore.rs"]
mod restoration;
pub use restoration::{AcceptedPublicationReceipt, PublicationRestoreError};

/// Publication capabilities are separate from executable validator coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicationFamily {
    NamespaceImports,
    EffectiveMembership,
    Specialization,
    Typing,
    Featuring,
    FeatureChains,
    CrossFeatures,
    ConnectorsAssociations,
    ExpressionResults,
    FeatureValues,
    Multiplicity,
    StandardBindings,
    IdentityProvenance,
}
impl PublicationFamily {
    pub const ALL: [Self; 13] = [
        Self::NamespaceImports,
        Self::EffectiveMembership,
        Self::Specialization,
        Self::Typing,
        Self::Featuring,
        Self::FeatureChains,
        Self::CrossFeatures,
        Self::ConnectorsAssociations,
        Self::ExpressionResults,
        Self::FeatureValues,
        Self::Multiplicity,
        Self::StandardBindings,
        Self::IdentityProvenance,
    ];
}

/// Resource accounting for one additive producer stage; elapsed time is not a gate.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PublicationStage {
    pub stratum: ResultStructureStratum,
    /// Cumulative deterministic work, retained even when a later gate fails.
    pub counters: PublicationCounters,
    pub stage: usize,
    pub input_elements: usize,
    pub added_elements: usize,
    pub added_occurrences: usize,
    pub completeness: Completeness,
    pub diagnostics: BTreeSet<Diagnostic>,
}

/// Closure evidence is established by the canonical publication builder, or
/// restored from the exact graph pinned by a checked-in acceptance receipt.
/// Private fields prevent callers from promoting an arbitrary partial overlay.
pub struct CompletePublicationOverlay {
    overlay: DerivedOverlay,
    context: SemanticContextId,
    checked: BTreeMap<PublicationFamily, usize>,
    stages: Vec<PublicationStage>,
    counters: PublicationCounters,
    restored_from_receipt: bool,
    certificate: Option<std::sync::Arc<ProducerClosureCertificate>>,
}
impl CompletePublicationOverlay {
    /// Bind an authored snapshot only when the kernel retains this exact immutable
    /// publication as its protected dependency. Library roots never see authored roots.
    pub fn project_context<'m>(
        &self,
        snapshot: &'m Snapshot,
        project_root: ElementId,
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<SemanticContext<'m>, ContextError> {
        if !snapshot
            .immutable_dependency()
            .is_some_and(|dependency| std::ptr::eq(dependency.model(), self.overlay.model()))
        {
            return Err(ContextError::PublicationDependencyMismatch);
        }
        let context = SemanticContext::for_project_snapshot(
            snapshot,
            self.context.options.clone(),
            self.context.pinned_libraries.clone(),
            pending_specializations,
            pending_namespaces,
        )?;
        self.attach_project_context(context, &[project_root])
    }
    /// Bind a partial source/Systems candidate that retains this exact protected
    /// dependency. Local roots see one another and the accepted roots; accepted
    /// library roots keep their original availability. Missing endpoints and
    /// pending scopes remain explicit construction/query incompleteness.
    pub fn project_construction_context<'m>(
        &self,
        candidate: &'m agq_kernel::ConstructionView,
        local_roots: &[ElementId],
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<SemanticContext<'m>, ContextError> {
        if !candidate
            .immutable_dependency()
            .is_some_and(|dependency| std::ptr::eq(dependency.model(), self.overlay.model()))
        {
            return Err(ContextError::PublicationDependencyMismatch);
        }
        let context = SemanticContext::for_project_construction(
            candidate,
            self.context.options.clone(),
            self.context.pinned_libraries.clone(),
            pending_specializations,
            pending_namespaces,
        )?;
        self.attach_project_context(context, local_roots)
    }
    /// Bind a producer frontier over new local declarations. The exact accepted
    /// dependency remains sealed; its namespace availability is unchanged.
    pub fn project_overlay_context<'m>(
        &self,
        overlay: &'m DerivedOverlay,
        local_roots: &[ElementId],
    ) -> Result<SemanticContext<'m>, ContextError> {
        if !overlay
            .declared()
            .immutable_dependency()
            .is_some_and(|dependency| std::ptr::eq(dependency.model(), self.overlay.model()))
        {
            return Err(ContextError::PublicationDependencyMismatch);
        }
        let context = SemanticContext::for_overlay(
            overlay,
            self.context.options.clone(),
            self.context.pinned_libraries.clone(),
        )?;
        self.attach_project_context(context, local_roots)
    }
    /// Bind an unpublished producer frontier over this exact protected
    /// dependency. Missing values and pending source scopes remain explicit;
    /// accepted library roots retain their original namespace availability.
    pub fn project_construction_overlay_context<'m>(
        &self,
        overlay: &'m ConstructionOverlay,
        local_roots: &[ElementId],
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<SemanticContext<'m>, ContextError> {
        if !overlay
            .declared()
            .immutable_dependency()
            .is_some_and(|dependency| std::ptr::eq(dependency.model(), self.overlay.model()))
        {
            return Err(ContextError::PublicationDependencyMismatch);
        }
        let context = SemanticContext::for_project_construction_overlay(
            overlay,
            self.context.options.clone(),
            self.context.pinned_libraries.clone(),
            pending_specializations,
            pending_namespaces,
        )?;
        self.attach_project_context(context, local_roots)
    }
    fn attach_project_context<'m>(
        &self,
        context: SemanticContext<'m>,
        local_roots: &[ElementId],
    ) -> Result<SemanticContext<'m>, ContextError> {
        let mut availability = (*self.context.available_roots).clone();
        let visible: BTreeSet<_> = availability.keys().chain(local_roots).copied().collect();
        for &root in local_roots {
            // Existing accepted roots must never gain visibility of authored roots.
            if availability.contains_key(&root) {
                continue;
            }
            availability.insert(root, visible.clone());
        }
        let mut context = context.with_available_roots(availability)?;
        context.id.standard_bindings = self.context.standard_bindings.clone();
        context.id.formal_constraint_targets = self.context.formal_constraint_targets.clone();
        context.id.library_graph_digest = self.context.library_graph_digest;
        context.id.publication_dependency_digest = Some(self.context.model_digest);
        context.accepted_dependency = Some(std::sync::Arc::new(self.overlay.clone()));
        Ok(context)
    }
    pub fn overlay(&self) -> &DerivedOverlay {
        &self.overlay
    }
    pub fn context(&self) -> &SemanticContextId {
        &self.context
    }
    /// Successful query answers per family; bindings count roles and provenance
    /// counts derived facts. These are not counts of distinct subjects.
    pub fn checked_items(&self) -> &BTreeMap<PublicationFamily, usize> {
        &self.checked
    }
    /// Deterministic resource accounting from the dependency-driven closure.
    pub fn counters(&self) -> &PublicationCounters {
        &self.counters
    }
    pub fn stages(&self) -> &[PublicationStage] {
        &self.stages
    }
    /// Restored publications retain semantic evidence, but do not replay resource
    /// counters or stages from the original publication process.
    pub fn restored_from_receipt(&self) -> bool {
        self.restored_from_receipt
    }
    pub fn queries(&self) -> KerMlQueries<'_> {
        KerMlQueries::new(SemanticContext {
            model: self.overlay.model(),
            id: self.context.clone(),
            naming_extension: None,
            producer_closure: self.certificate.clone(),
            immutable_dependency: self
                .overlay
                .declared()
                .immutable_dependency()
                .map(|dependency| dependency.model()),
            accepted_dependency: None,
            closed_dependency: None,
        })
    }
}

/// A failed publication gate never returns a complete overlay. Validator-only
/// findings are collected separately by `KerMlConformanceReport`.
/// Exact rejected output and its declared producer contract. Constructed only
/// on an audit failure; successful publication does not expand this detail.
#[derive(Debug)]
pub struct ProducerEffectAuditFailure {
    pub operation: &'static str,
    pub fact: FactKey,
    pub origin_rule: agq_kernel::RuleId,
    pub producer_subject: Option<ElementId>,
    pub semantic_target: ElementId,
    pub detail: String,
}

#[derive(Debug)]
pub enum PublicationOverlayError {
    FrontierCheckpoint(String),
    UnsupportedProfile(BaselineProfile),
    Context(ContextError),
    Bindings(BindingError),
    Derivation(DerivationError),
    ProducerEffectViolation(Box<ProducerEffectAuditFailure>),
    ProducerEvaluationMismatch {
        subject: ElementId,
        family: ProducerFamilyId,
        reason: &'static str,
        state: Option<ProducerEvaluationState>,
        descriptor: Option<Box<ProducerDescriptor>>,
    },
    SchedulerContextMismatch {
        changed_fields: Vec<&'static str>,
    },
    ForeignDeclaredFact(FactKey),
    IncompleteProducers(Vec<PublicationStage>),
    IncompleteCapabilities(BTreeMap<PublicationFamily, BTreeSet<Diagnostic>>),
}
impl std::fmt::Display for PublicationOverlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PublicationOverlayError {}
impl From<DerivationError> for PublicationOverlayError {
    fn from(value: DerivationError) -> Self {
        Self::Derivation(value)
    }
}

/// The sole way to establish new publication acceptance. It covers the
/// entire strict input and every produced subject under Operational v9. Callers
/// cannot supply capability statuses or restrict the mandatory subject population.
pub struct CanonicalPublicationBuilder<'a> {
    snapshot: &'a Snapshot,
    roots: &'a [ElementId],
    library_set: &'a LibrarySetIdentity,
    profile: BaselineProfile,
}
impl<'a> CanonicalPublicationBuilder<'a> {
    pub fn new(
        snapshot: &'a Snapshot,
        roots: &'a [ElementId],
        library_set: &'a LibrarySetIdentity,
    ) -> Self {
        Self {
            snapshot,
            roots,
            library_set,
            profile: BaselineProfile::OPERATIONAL_V9,
        }
    }
    /// Reproduce a historical publication-capable profile explicitly. The
    /// operational construction alias remains independently frozen until acceptance.
    pub fn with_profile(
        mut self,
        profile: BaselineProfile,
    ) -> Result<Self, PublicationOverlayError> {
        if !profile.supports_publication_producers() {
            return Err(PublicationOverlayError::UnsupportedProfile(profile));
        }
        self.profile = profile;
        Ok(self)
    }
    fn context<'m>(
        &self,
        overlay: &'m DerivedOverlay,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError> {
        let roots: BTreeSet<_> = self.roots.iter().copied().collect();
        let context = SemanticContext::for_overlay(
            overlay,
            SemanticOptions {
                baseline_profile: self.profile,
                exclude_implied: false,
            },
            self.library_set.pins.clone(),
        )
        .and_then(|context| {
            context.with_available_roots(roots.iter().map(|&root| (root, roots.clone())).collect())
        })
        .map_err(PublicationOverlayError::Context)?
        .with_standard_bindings(self.roots, self.library_set)
        .map_err(PublicationOverlayError::Bindings)?;
        Ok(context.with_formal_constraint_targets(
            self.roots,
            self.library_set.artifacts[&StandardLibraryArtifact::Semantic],
        ))
    }
    /// Run bounded subject batches until all producers complete without adding
    /// facts. The stage limit is a resource limit: exhausting it returns an
    /// incomplete result and never changes the publication acceptance contract.
    pub fn build(
        self,
        max_stages: usize,
        progress: impl FnMut(&PublicationStage),
    ) -> Result<CompletePublicationOverlay, PublicationOverlayError> {
        self.build_with_progress(max_stages, |_, _, _, _| {}, progress, |_, _, _| {})
    }
    /// As `build`, with per-batch resource observations: stage, completed
    /// subjects, total subjects and proposed Elements. The capability observer
    /// receives completed subjects, total subjects and finding count. Observers
    /// cannot alter producer input, merge order or acceptance criteria.
    pub fn build_with_progress(
        self,
        max_stages: usize,
        mut batch_progress: impl FnMut(usize, usize, usize, usize),
        mut progress: impl FnMut(&PublicationStage),
        mut capability_progress: impl FnMut(usize, usize, usize),
    ) -> Result<CompletePublicationOverlay, PublicationOverlayError> {
        let libraries: BTreeSet<_> = self.library_set.artifacts.values().copied().collect();
        let accepted = |origin: &Origin| {
            matches!(origin, Origin::Declared(DeclaredOrigin::StandardLibrary {library}
            | DeclaredOrigin::ReviewedCorrection { library, .. }) if libraries.contains(library))
        };
        for record in self.snapshot.model().elements() {
            if !accepted(record.origin()) {
                return Err(PublicationOverlayError::ForeignDeclaredFact(
                    FactKey::Element(record.id()),
                ));
            }
            for (property, slot) in record.slots() {
                if !accepted(slot.origin()) {
                    return Err(PublicationOverlayError::ForeignDeclaredFact(
                        FactKey::Property {
                            element: record.id(),
                            property,
                        },
                    ));
                }
            }
        }
        for occurrence in self.snapshot.model().association_occurrences() {
            if !accepted(occurrence.origin()) {
                return Err(PublicationOverlayError::ForeignDeclaredFact(
                    FactKey::AssociationOccurrence(occurrence.id()),
                ));
            }
        }
        let closure = close_result_structure(
            self.snapshot,
            PublicationClosureOptions {
                max_rounds: max_stages,
                ..Default::default()
            },
            |overlay| self.context(overlay),
            &mut batch_progress,
            &mut progress,
        )?;
        if !closure.converged || closure.completeness != Completeness::Complete {
            return Err(PublicationOverlayError::IncompleteProducers(closure.stages));
        }
        let PublicationClosure {
            overlay,
            stages,
            counters,
            certificate,
            ..
        } = closure;
        let mut context = self.context(&overlay)?;
        if let Some(witness) = &certificate {
            context = context
                .with_producer_registry_digest(witness.registry_digest())
                .and_then(|context| context.with_producer_closure(witness.clone()))
                .map_err(PublicationOverlayError::Context)?;
        }
        let mut checks = PublicationChecks::new(overlay.model(), false);
        let subjects: Vec<_> = overlay.model().elements().map(|r| r.id()).collect();
        for (index, batch) in subjects.chunks(32).enumerate() {
            let q = KerMlQueries::for_production(context.fork());
            for &subject in batch {
                checks.subject(&q, subject);
            }
            capability_progress(
                ((index + 1) * 32).min(subjects.len()),
                subjects.len(),
                checks.failures.values().map(BTreeSet::len).sum(),
            );
        }
        checks.standard_bindings(&KerMlQueries::for_production(context.fork()));
        if !checks.failures.is_empty() {
            return Err(PublicationOverlayError::IncompleteCapabilities(
                checks.failures,
            ));
        }
        let mut identity = context.id().clone();
        identity.derivation_phase = DerivationPhase::CompletePublicationOverlay;
        let checked = checks.counts;
        Ok(CompletePublicationOverlay {
            overlay,
            context: identity,
            checked,
            stages,
            counters,
            restored_from_receipt: false,
            certificate,
        })
    }
}

/// Scoped capability evidence. This report cannot promote a partial overlay.
pub struct PublicationCapabilityReport {
    pub context: SemanticContextId,
    /// Number of applicable query answers checked, not distinct subjects.
    pub checked_items: BTreeMap<PublicationFamily, usize>,
    pub failures: BTreeMap<PublicationFamily, BTreeSet<Diagnostic>>,
    /// Inputs consulted by capability queries and canonical provenance. Scoped
    /// callers must check their producer population against these dependencies.
    pub read_dependencies: QueryInvalidationSet,
    /// Producer obligations under additive closure, preserving mutable reads
    /// without scheduling a declared record solely for its identity or name.
    pub provider_reads: PublicationProviderReads,
}
impl KerMlQueries<'_> {
    #[cfg(test)]
    pub(crate) fn audit_expanded_capabilities(
        &self,
        subjects: impl IntoIterator<Item = ElementId>,
    ) -> PublicationCapabilityReport {
        let mut checks = PublicationChecks::new(self.model(), true);
        checks.standard_bindings(self);
        for subject in subjects {
            checks.subject(self, subject);
        }
        PublicationCapabilityReport {
            context: self.context().clone(),
            checked_items: checks.counts,
            failures: checks.failures,
            read_dependencies: QueryInvalidationSet::from_keys(checks.read_keys.unwrap()),
            provider_reads: PublicationProviderReads::from_keys(checks.provider_keys.unwrap()),
        }
    }
    /// Audit a selected population for focused publication regressions.
    pub fn audit_publication_capabilities(
        &self,
        subjects: impl IntoIterator<Item = ElementId>,
    ) -> PublicationCapabilityReport {
        self.audit_publication_capabilities_with_rules(subjects, [])
    }
    /// Audit a combined language population with an explicit finite registry
    /// of extension producer rules. This only recognizes their provenance;
    /// all ordinary capability, dependency and completeness checks still run.
    /// The report cannot promote an overlay to an accepted publication.
    pub fn audit_publication_capabilities_with_rules(
        &self,
        subjects: impl IntoIterator<Item = ElementId>,
        extension_rules: impl IntoIterator<Item = agq_kernel::RuleId>,
    ) -> PublicationCapabilityReport {
        let mut q = KerMlQueries::for_production(self.context.fork());
        let mut checks = PublicationChecks::new(self.model(), true);
        checks.extension_rules.extend(extension_rules);
        checks.standard_bindings(&q);
        for (index, subject) in subjects.into_iter().enumerate() {
            if index > 0 && index.is_multiple_of(32) {
                // Keep the aggregate audit/proof deduplication, while bounding
                // per-query memo retention over a combined language corpus.
                q = KerMlQueries::for_production(self.context.fork());
            }
            checks.subject(&q, subject);
        }
        PublicationCapabilityReport {
            context: self.context().clone(),
            checked_items: checks.counts,
            failures: checks.failures,
            read_dependencies: QueryInvalidationSet::from_keys(checks.read_keys.unwrap()),
            provider_reads: PublicationProviderReads::from_keys(checks.provider_keys.unwrap()),
        }
    }
}

struct PublicationChecks<'m> {
    model: &'m ModelView,
    extension_rules: BTreeSet<agq_kernel::RuleId>,
    provenance_checked: BTreeSet<FactKey>,
    // Borrowed immutable proof allocations stay alive with the model. Addresses
    // only avoid rereading a shared premise set; they never enter published data.
    proof_reads_checked: BTreeSet<usize>,
    read_keys: Option<BTreeSet<InvalidationKey>>,
    provider_keys: Option<BTreeSet<InvalidationKey>>,
    counts: BTreeMap<PublicationFamily, usize>,
    failures: BTreeMap<PublicationFamily, BTreeSet<Diagnostic>>,
}
impl<'m> PublicationChecks<'m> {
    fn new(model: &'m ModelView, capture_reads: bool) -> Self {
        Self {
            model,
            extension_rules: BTreeSet::new(),
            provenance_checked: BTreeSet::new(),
            proof_reads_checked: BTreeSet::new(),
            read_keys: capture_reads.then(BTreeSet::new),
            provider_keys: capture_reads.then(BTreeSet::new),
            counts: PublicationFamily::ALL.into_iter().map(|f| (f, 0)).collect(),
            failures: BTreeMap::new(),
        }
    }
}
impl PublicationChecks<'_> {
    fn standard_bindings(&mut self, q: &KerMlQueries<'_>) {
        // A scoped audit without a library dependency makes no binding claim.
        // Attached bindings were validated against this exact semantic context.
        if q.context().standard_bindings.is_some() {
            for role in StandardRole::ALL {
                let answer = q.standard_role(role);
                self.answer(
                    PublicationFamily::StandardBindings,
                    answer.value.unwrap_or(ElementId::from_u128(0)),
                    answer,
                );
            }
        }
    }
    fn provenance(
        &mut self,
        q: &KerMlQueries<'_>,
        fact: FactKey,
        subject: ElementId,
        origin: &Origin,
    ) {
        let Origin::Derived(explanation) = origin else {
            return;
        };
        if !self.provenance_checked.insert(fact) {
            return;
        }
        if let Some(keys) = &mut self.read_keys
            && self
                .proof_reads_checked
                .insert(std::sync::Arc::as_ptr(explanation) as usize)
        {
            for dependency in &explanation.dependencies {
                if let Some(providers) = &mut self.provider_keys {
                    providers.extend(publication_dependency_keys(dependency, self.model));
                }
                let (Dependency::Declared(fact) | Dependency::Derived(fact)) = dependency;
                match fact {
                    FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                        keys.insert(InvalidationKey::Element(*id));
                    }
                    FactKey::AssociationOccurrence(id) => {
                        if let Some(link) = self.model.association_occurrence(*id) {
                            keys.extend(
                                link.ends().values().copied().map(InvalidationKey::Element),
                            );
                        }
                    }
                }
            }
        }
        *self
            .counts
            .entry(PublicationFamily::IdentityProvenance)
            .or_default() += 1;
        if crate::result_structure::structural_rule_profile(explanation.rule)
            != Some(q.context().options.baseline_profile)
            && !self.extension_rules.contains(&explanation.rule)
        {
            self.problem(
                PublicationFamily::IdentityProvenance,
                subject,
                "KQ_PUBLICATION_RULE",
                "Unknown publication producer profile",
            );
        }
    }
    fn answer<T>(&mut self, family: PublicationFamily, subject: ElementId, answer: QueryResult<T>) {
        if let Some(keys) = &mut self.read_keys {
            keys.extend(query_read_keys(&answer, self.model));
        }
        if let Some(keys) = &mut self.provider_keys {
            keys.extend(query_publication_provider_keys(&answer, self.model));
        }
        *self.counts.entry(family).or_default() += 1;
        if answer.completeness != Completeness::Complete {
            self.problem(
                family,
                subject,
                "KQ_PUBLICATION_QUERY",
                "Mandatory publication query is not Complete",
            );
            self.failures
                .entry(family)
                .or_default()
                .extend(answer.diagnostics);
        }
    }
    fn problem(
        &mut self,
        family: PublicationFamily,
        subject: ElementId,
        code: &'static str,
        message: &str,
    ) {
        self.failures.entry(family).or_default().insert(Diagnostic {
            code,
            subject,
            message: message.into(),
        });
    }
    fn subject(&mut self, q: &KerMlQueries<'_>, subject: ElementId) {
        use PublicationFamily as F;
        if let Some(record) = q.model().element(subject) {
            self.provenance(q, FactKey::Element(subject), subject, record.origin());
            for (property, slot) in record.slots() {
                self.provenance(
                    q,
                    FactKey::Property {
                        element: subject,
                        property,
                    },
                    subject,
                    slot.origin(),
                );
            }
            for occurrence in q.model().incident_associations(subject) {
                self.provenance(
                    q,
                    FactKey::AssociationOccurrence(occurrence.id()),
                    *occurrence
                        .ends()
                        .values()
                        .next()
                        .expect("canonical occurrence"),
                    occurrence.origin(),
                );
            }
        }
        if q.is(subject, c::NAMESPACE) {
            self.answer(
                F::NamespaceImports,
                subject,
                q.namespace_members(subject, MemberAccess::All),
            );
        }
        if q.is(subject, c::TYPE) {
            self.answer(
                F::EffectiveMembership,
                subject,
                q.effective_features(subject),
            );
            self.answer(
                F::Specialization,
                subject,
                q.direct_specializations(subject),
            );
        }
        if q.is(subject, c::FEATURE) {
            self.answer(F::Typing, subject, q.feature_types(subject));
            self.answer(F::Featuring, subject, q.featuring_types(subject));
            self.answer(F::FeatureChains, subject, q.feature_target(subject));
            self.answer(F::FeatureChains, subject, q.chaining_features(subject));
            let selected = q.owned_cross_feature(subject);
            if let Some(expected) = selected.value {
                let crossing = q.cross_feature(subject);
                let owner = q.owning_type(subject);
                if let Some(owner) = owner.value {
                    let ends = q.structural_end_features(owner);
                    if ends.value.len() > 1 && crossing.value != Some(expected) {
                        self.problem(
                            F::CrossFeatures,
                            subject,
                            "KQ_PUBLICATION_CROSSING",
                            "Missing canonical crossing identity",
                        );
                    }
                    self.answer(F::CrossFeatures, subject, ends);
                }
                self.answer(F::CrossFeatures, subject, owner);
                self.answer(F::CrossFeatures, subject, crossing);
                self.answer(
                    F::CrossFeatures,
                    expected,
                    q.owned_cross_feature_domain(expected),
                );
            }
            self.answer(F::CrossFeatures, subject, selected);
        }
        if q.is(subject, c::CONNECTOR) {
            // V9 publication uses the exact published relatedFeature projection.
            // Keep historical strict endpoint queries and audits reproducible.
            let structure =
                if q.context().options.baseline_profile == BaselineProfile::OPERATIONAL_V9 {
                    q.connector_related_structure(subject)
                } else {
                    q.connector_structure(subject)
                };
            self.answer(F::ConnectorsAssociations, subject, structure);
        }
        if q.is(subject, c::ASSOCIATION) {
            self.answer(
                F::ConnectorsAssociations,
                subject,
                q.association_structure(subject),
            );
        }
        if q.is(subject, c::EXPRESSION) || q.is(subject, c::FUNCTION) {
            self.answer(F::ExpressionResults, subject, q.structural_result(subject));
        }
        if q.is(subject, c::FEATURE_VALUE) {
            self.answer(F::FeatureValues, subject, q.feature_with_value(subject));
        }
        if q.is(subject, c::MULTIPLICITY_RANGE) {
            self.answer(F::Multiplicity, subject, q.multiplicity_bounds(subject));
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/publication_reads.rs"]
mod tests;

#[cfg(test)]
mod construction_context_tests {
    use crate as agq_kerml_semantics;
    include!("../tests/common/namespace_fixture.rs");
    use super::CompletePublicationOverlay;
    use agq_kernel::derived::{ConstructionDerivationBuilder, DerivationBuilder};

    fn publication() -> CompletePublicationOverlay {
        let mut fixture = Fixture::new();
        fixture.create(1, c::PACKAGE);
        let overlay = DerivationBuilder::new(fixture.finish()).build().unwrap();
        let context =
            SemanticContext::for_overlay(&overlay, Default::default(), Default::default())
                .unwrap()
                .with_available_roots(BTreeMap::from([(id(1), BTreeSet::from([id(1)]))]))
                .unwrap()
                .id()
                .clone();
        CompletePublicationOverlay {
            overlay,
            context,
            checked: BTreeMap::new(),
            stages: Vec::new(),
            counters: Default::default(),
            restored_from_receipt: false,
            certificate: None,
        }
    }

    fn frontier(publication: &CompletePublicationOverlay) -> super::ConstructionOverlay {
        let base = Snapshot::with_immutable_dependency(Arc::new(publication.overlay.clone()));
        let changes = base.change_set();
        let mut fixture = Fixture {
            base,
            changes,
            owned: BTreeMap::new(),
        };
        fixture.create(100, c::PACKAGE);
        fixture.create(101, c::CLASS);
        fixture.create(200, c::SPECIALIZATION);
        fixture.value(200, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(101)));
        let declared = Arc::new(fixture.construction());
        assert_eq!(declared.obligations().len(), 1);
        let mut builder = ConstructionDerivationBuilder::for_construction(declared);
        builder.element(
            DerivationKey {
                rule: RuleId::from_u128(900),
                subject: id(101),
                output: OutputKey::from_u128(901),
            },
            c::SPECIALIZATION,
            [
                (
                    p::RELATIONSHIP_IS_IMPLIED,
                    SlotValue::Scalar(Value::Boolean(true)),
                ),
                (
                    p::SPECIALIZATION_SPECIFIC,
                    SlotValue::Scalar(Value::Reference(id(101))),
                ),
            ],
            BTreeSet::from([Dependency::Declared(FactKey::Element(id(101)))]),
        );
        builder.build().unwrap()
    }

    #[test]
    fn construction_overlay_context_retains_current_obligations_and_protected_root_scopes() {
        let publication = publication();
        let overlay = frontier(&publication);
        assert!(overlay.obligations().len() > overlay.declared().obligations().len());
        assert!(overlay.obligations().iter().any(|obligation| {
            obligation.element != id(200) && obligation.property == p::SPECIALIZATION_GENERAL
        }));
        let context = publication
            .project_construction_overlay_context(
                &overlay,
                &[id(100)],
                BTreeSet::from([id(101)]),
                BTreeSet::from([id(100)]),
            )
            .unwrap();
        assert_eq!(
            context.id().derivation_phase,
            DerivationPhase::PartialDerivationOverlay
        );
        assert_eq!(context.id().revision, overlay.base_revision());
        assert_eq!(
            *context.id().construction_obligations,
            overlay
                .obligations()
                .iter()
                .map(|o| (o.element, o.property))
                .collect()
        );
        assert_eq!(
            context.id().pending_specialization_scopes,
            BTreeSet::from([id(101)])
        );
        assert_eq!(
            context.id().pending_namespace_scopes,
            BTreeSet::from([id(100)])
        );
        assert_eq!(
            context.id().available_roots[&id(1)],
            BTreeSet::from([id(1)])
        );
        assert_eq!(
            context.id().available_roots[&id(100)],
            BTreeSet::from([id(1), id(100)])
        );
        assert_eq!(
            context.id().publication_dependency_digest,
            Some(publication.context.model_digest)
        );
        assert_eq!(context.id().options, publication.context.options);
        assert_eq!(
            context.id().pinned_libraries,
            publication.context.pinned_libraries
        );
        let without_pending = publication
            .project_construction_overlay_context(
                &overlay,
                &[id(100)],
                BTreeSet::new(),
                BTreeSet::new(),
            )
            .unwrap();
        assert_ne!(context.id(), without_pending.id());
        assert!(std::ptr::eq(context.model, overlay.model()));
    }

    #[test]
    fn construction_overlay_context_rejects_foreign_dependency_and_invalid_scopes() {
        let accepted = publication();
        let overlay = frontier(&accepted);
        let foreign = publication();
        assert!(matches!(
            foreign.project_construction_overlay_context(
                &overlay,
                &[id(100)],
                BTreeSet::new(),
                BTreeSet::new()
            ),
            Err(ContextError::PublicationDependencyMismatch)
        ));
        for (roots, specializations, namespaces) in [
            (vec![id(100)], BTreeSet::from([id(200)]), BTreeSet::new()),
            (vec![id(100)], BTreeSet::new(), BTreeSet::from([id(200)])),
            (vec![id(200)], BTreeSet::new(), BTreeSet::new()),
        ] {
            assert!(matches!(accepted.project_construction_overlay_context(
                &overlay, &roots, specializations, namespaces
            ), Err(ContextError::InvalidPendingScope(bad)) if bad == id(200)));
        }
    }
}
