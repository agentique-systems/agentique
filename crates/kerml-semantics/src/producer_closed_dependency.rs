//! Authenticated mounting of a producer-closed graph. This is not publication
//! acceptance: language-specific facades retain their additional acceptance gates.
use crate::{
    ClosureSource, ContextError, ProducerClosureCertificate, ProducerRegistry, SemanticContext,
    SemanticContextId, SemanticNamingExtension,
};
use agq_kernel::{
    ConstructionView, ElementId, Snapshot,
    derived::{ConstructionOverlay, DerivedOverlay},
};
use std::{collections::BTreeSet, sync::Arc};

const DEPENDENCY_DOMAIN: &str = "agq-producer-closed-dependency/1";

/// An exact immutable graph with a complete certificate from the independently
/// expected producer registry. Fields are private; construction does not accept
/// caller-supplied completeness or standard-publication labels.
pub struct ProducerClosedDependency {
    overlay: Arc<DerivedOverlay>,
    context: SemanticContextId,
    certificate: Arc<ProducerClosureCertificate>,
    naming_extension: Option<(&'static str, Arc<dyn SemanticNamingExtension>)>,
    accepted_ancestor: Option<Arc<DerivedOverlay>>,
}
impl std::fmt::Debug for ProducerClosedDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProducerClosedDependency")
            .field("model_digest", &self.context.model_digest)
            .field("certificate_digest", &self.certificate.digest())
            .finish_non_exhaustive()
    }
}
impl ProducerClosedDependency {
    /// Validate exact graph, semantic context and every closed requirement. The
    /// registry is constructed independently by the language consumer, never
    /// reconstructed from the certificate's own claimed digest.
    pub fn new(
        overlay: Arc<DerivedOverlay>,
        context: &SemanticContext<'_>,
        expected: &ProducerRegistry,
    ) -> Result<Arc<Self>, ContextError> {
        let certificate = context
            .producer_closure()
            .ok_or(ContextError::ProducerClosureMismatch)?;
        if !std::ptr::eq(overlay.model(), context.model)
            || context.id().producer_registry_digest != Some(expected.digest())
            || certificate.registry_digest() != expected.digest()
            || !certificate.compatible_context(context.id())
            || !context.id().construction_obligations.is_empty()
            || !certificate.is_fully_closed(overlay.model())
        {
            return Err(ContextError::ProducerClosureMismatch);
        }
        let mut candidate = overlay.declared().immutable_dependency().cloned();
        let mut accepted_ancestor = None;
        while let Some(ancestor) = candidate {
            if context
                .accepted_dependency
                .as_ref()
                .is_some_and(|model| std::ptr::eq(model.model(), ancestor.model()))
            {
                accepted_ancestor = Some(ancestor);
                break;
            }
            candidate = ancestor.declared().immutable_dependency().cloned();
        }
        if context.accepted_dependency.is_some() && accepted_ancestor.is_none() {
            return Err(ContextError::PublicationDependencyMismatch);
        }
        Ok(Arc::new(Self {
            overlay,
            context: context.id().clone(),
            certificate: certificate.clone(),
            naming_extension: context.naming_extension.clone(),
            accepted_ancestor,
        }))
    }
    pub fn overlay(&self) -> &Arc<DerivedOverlay> {
        &self.overlay
    }
    pub fn context(&self) -> &SemanticContextId {
        &self.context
    }
    pub fn certificate(&self) -> &Arc<ProducerClosureCertificate> {
        &self.certificate
    }
    pub fn project_snapshot(&self) -> Snapshot {
        Snapshot::with_immutable_dependency(self.overlay.clone())
    }

    fn check_dependency(
        &self,
        dependency: Option<&Arc<DerivedOverlay>>,
    ) -> Result<(), ContextError> {
        if !dependency.is_some_and(|dependency| Arc::ptr_eq(dependency, &self.overlay)) {
            return Err(ContextError::PublicationDependencyMismatch);
        }
        Ok(())
    }
    /// Mount into an authored snapshot while retaining prior namespace scopes.
    pub fn project_context<'m>(
        self: &Arc<Self>,
        snapshot: &'m Snapshot,
        roots: &[ElementId],
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<SemanticContext<'m>, ContextError> {
        self.check_dependency(snapshot.immutable_dependency())?;
        self.attach(
            SemanticContext::for_project_snapshot(
                snapshot,
                self.context.options.clone(),
                self.context.pinned_libraries.clone(),
                pending_specializations,
                pending_namespaces,
            )?,
            roots,
        )
    }
    /// Partial declared construction keeps missing endpoints explicit.
    pub fn project_construction_context<'m>(
        self: &Arc<Self>,
        candidate: &'m ConstructionView,
        roots: &[ElementId],
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<SemanticContext<'m>, ContextError> {
        self.check_dependency(candidate.immutable_dependency())?;
        self.attach(
            SemanticContext::for_project_construction(
                candidate,
                self.context.options.clone(),
                self.context.pinned_libraries.clone(),
                pending_specializations,
                pending_namespaces,
            )?,
            roots,
        )
    }
    /// A local producer frontier shares the immutable dependency and its proof.
    pub fn project_overlay_context<'m>(
        self: &Arc<Self>,
        overlay: &'m DerivedOverlay,
        roots: &[ElementId],
    ) -> Result<SemanticContext<'m>, ContextError> {
        self.check_dependency(overlay.declared().immutable_dependency())?;
        self.attach(
            SemanticContext::for_overlay(
                overlay,
                self.context.options.clone(),
                self.context.pinned_libraries.clone(),
            )?,
            roots,
        )
    }
    /// Partial producer frontiers retain construction obligations and scoped visibility.
    pub fn project_construction_overlay_context<'m>(
        self: &Arc<Self>,
        overlay: &'m ConstructionOverlay,
        roots: &[ElementId],
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<SemanticContext<'m>, ContextError> {
        self.check_dependency(overlay.declared().immutable_dependency())?;
        self.attach(
            SemanticContext::for_project_construction_overlay(
                overlay,
                self.context.options.clone(),
                self.context.pinned_libraries.clone(),
                pending_specializations,
                pending_namespaces,
            )?,
            roots,
        )
    }
    fn attach<'m>(
        self: &Arc<Self>,
        mut context: SemanticContext<'m>,
        roots: &[ElementId],
    ) -> Result<SemanticContext<'m>, ContextError> {
        if context.id.descriptor_digest != self.context.descriptor_digest {
            return Err(ContextError::UnsupportedMetamodel);
        }
        let mut available = (*self.context.available_roots).clone();
        let visible: BTreeSet<_> = available.keys().chain(roots).copied().collect();
        for &root in roots {
            available.entry(root).or_insert_with(|| visible.clone());
        }
        context = context.with_available_roots(available)?;
        context.id.standard_bindings = self.context.standard_bindings.clone();
        context.id.formal_constraint_targets = self.context.formal_constraint_targets.clone();
        context.id.library_graph_digest = self.context.library_graph_digest;
        context.id.publication_dependency_digest = self.context.publication_dependency_digest;
        context.id.semantic_extensions = self.context.semantic_extensions.clone();
        context
            .id
            .semantic_extensions
            .insert(DEPENDENCY_DOMAIN, self.certificate.digest());
        context = context.with_producer_registry_digest(self.certificate.registry_digest())?;
        context.naming_extension = self.naming_extension.clone();
        context.accepted_dependency = self.accepted_ancestor.clone();
        context.closed_dependency = Some(self.clone());
        Ok(context)
    }
}

impl SemanticContext<'_> {
    /// Exact canonical support from an authenticated immutable interpretation.
    /// Subject membership alone does not seal a navigation projection: local
    /// inverse carriers may change its value or occurrence provenance.
    pub(crate) fn sealed_dependency_fact(&self, fact: agq_kernel::provenance::FactKey) -> bool {
        use agq_kernel::provenance::FactKey;
        fn same<T: PartialEq>(current: Option<&T>, dependency: Option<&T>) -> bool {
            matches!((current, dependency), (Some(current), Some(dependency))
                if std::ptr::eq(current, dependency) || current == dependency)
        }
        let closed = self.closed_dependency.as_ref().filter(|dependency| {
            self.id.producer_registry_digest == Some(dependency.certificate.registry_digest())
        });
        [
            self.accepted_dependency.as_deref(),
            closed.map(|dependency| dependency.overlay.as_ref()),
        ]
        .into_iter()
        .flatten()
        .any(|dependency| {
            let dependency = dependency.model();
            match fact {
                FactKey::Element(element) => {
                    same(self.model.element(element), dependency.element(element))
                }
                FactKey::Property { element, property } => same(
                    self.model.navigation_slot(element, property),
                    dependency.navigation_slot(element, property),
                ),
                FactKey::AssociationOccurrence(occurrence) => same(
                    self.model.association_occurrence(occurrence),
                    dependency.association_occurrence(occurrence),
                ),
            }
        })
    }

    pub(crate) fn dependency_closure_source(&self, subject: ElementId) -> Option<ClosureSource> {
        if self
            .accepted_dependency
            .as_ref()
            .is_some_and(|model| model.model().element(subject).is_some())
        {
            return Some(ClosureSource::AcceptedDependency);
        }
        self.closed_dependency
            .as_ref()
            .filter(|dependency| {
                self.id.producer_registry_digest == Some(dependency.certificate.registry_digest())
                    && dependency.overlay.model().element(subject).is_some()
            })
            .map(|_| ClosureSource::LocalProducerClosure)
    }
}
