//! Canonical publication closure, independent of conformance validation coverage.
use crate::read_dependencies::{InvalidationKey, query_read_keys};
use crate::*;
use agq_kerml::{BaselineProfile, classes as c};
use agq_kernel::{
    ElementId, ModelView, Snapshot,
    derived::{DerivationError, DerivedOverlay},
    provenance::{DeclaredOrigin, Dependency, FactKey, Origin},
};
use std::collections::{BTreeMap, BTreeSet};

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
#[derive(Clone, Debug)]
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

/// Closure evidence can only be issued by the canonical publication builder.
/// Private fields prevent callers from promoting an arbitrary partial overlay.
pub struct CompletePublicationOverlay {
    overlay: DerivedOverlay,
    context: SemanticContextId,
    checked: BTreeMap<PublicationFamily, usize>,
    stages: Vec<PublicationStage>,
    counters: PublicationCounters,
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
        let mut availability = (*self.context.available_roots).clone();
        availability.insert(
            project_root,
            availability.keys().copied().chain([project_root]).collect(),
        );
        let mut context = SemanticContext::for_project_snapshot(
            snapshot,
            self.context.options.clone(),
            self.context.pinned_libraries.clone(),
            pending_specializations,
            pending_namespaces,
        )?
        .with_available_roots(availability)?;
        context.id.standard_bindings = self.context.standard_bindings.clone();
        context.id.formal_constraint_targets = self.context.formal_constraint_targets.clone();
        context.id.library_graph_digest = self.context.library_graph_digest;
        context.id.publication_dependency_digest = Some(self.context.model_digest);
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
    pub fn queries(&self) -> KerMlQueries<'_> {
        KerMlQueries::new(SemanticContext {
            model: self.overlay.model(),
            id: self.context.clone(),
        })
    }
}

/// A failed publication gate never returns a complete overlay. Validator-only
/// findings are collected separately by `KerMlConformanceReport`.
#[derive(Debug)]
pub enum PublicationOverlayError {
    UnsupportedProfile(BaselineProfile),
    Context(ContextError),
    Bindings(BindingError),
    Derivation(DerivationError),
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

/// The sole constructor for a complete publication overlay. It covers the
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
            ..
        } = closure;
        let context = self.context(&overlay)?;
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
        }
    }
    /// Audit a selected population for focused publication regressions.
    pub fn audit_publication_capabilities(
        &self,
        subjects: impl IntoIterator<Item = ElementId>,
    ) -> PublicationCapabilityReport {
        let q = KerMlQueries::for_production(self.context.fork());
        let mut checks = PublicationChecks::new(self.model(), true);
        checks.standard_bindings(&q);
        for subject in subjects {
            checks.subject(&q, subject);
        }
        PublicationCapabilityReport {
            context: self.context().clone(),
            checked_items: checks.counts,
            failures: checks.failures,
            read_dependencies: QueryInvalidationSet::from_keys(checks.read_keys.unwrap()),
        }
    }
}

struct PublicationChecks<'m> {
    model: &'m ModelView,
    provenance_checked: BTreeSet<FactKey>,
    // Borrowed immutable proof allocations stay alive with the model. Addresses
    // only avoid rereading a shared premise set; they never enter published data.
    proof_reads_checked: BTreeSet<usize>,
    read_keys: Option<BTreeSet<InvalidationKey>>,
    counts: BTreeMap<PublicationFamily, usize>,
    failures: BTreeMap<PublicationFamily, BTreeSet<Diagnostic>>,
}
impl<'m> PublicationChecks<'m> {
    fn new(model: &'m ModelView, capture_reads: bool) -> Self {
        Self {
            model,
            provenance_checked: BTreeSet::new(),
            proof_reads_checked: BTreeSet::new(),
            read_keys: capture_reads.then(BTreeSet::new),
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
            self.answer(
                F::ConnectorsAssociations,
                subject,
                q.connector_structure(subject),
            );
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
