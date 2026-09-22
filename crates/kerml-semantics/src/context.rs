use agq_kernel::{
    ElementId, ModelView, RevisionId, Snapshot,
    derived::{ConstructionOverlay, DerivedOverlay},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Change whenever rules, proof construction, dependency semantics or digest encoding change.
pub const RULE_SET_VERSION: &str = "agq-kerml-query/26";
pub const METAMODEL_VERSION: &str =
    "KerML/1.0;XMI:45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466";

/// Exact library pin supplied by the caller, not a textual import or a library version label.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LibraryPin {
    pub name: String,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticOptions {
    /// Explicit interpretation authority; legacy callers default to published metadata.
    pub baseline_profile: agq_kerml::BaselineProfile,
    /// KerML Type::supertypes(excludeImplied). Applies to specialization traversal.
    pub exclude_implied: bool,
}

/// Full identity, including overlay content/provenance. A revision alone is insufficient.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticContextId {
    pub revision: RevisionId,
    pub model_digest: [u8; 32],
    pub baseline_profile_id: &'static str,
    pub metamodel_version: &'static str,
    /// Reviewed errata content identity; None is the exact published profile.
    pub errata_manifest_digest: Option<[u8; 32]>,
    /// Independent reviewed KERML11-145 manifest identity.
    pub result_domain_manifest_digest: Option<[u8; 32]>,
    /// Independent reviewed KERML11-8 manifest identity.
    pub reference_binding_manifest_digest: Option<[u8; 32]>,
    /// Independent reviewed KERML11-1 manifest identity.
    pub owned_cross_feature_manifest_digest: Option<[u8; 32]>,
    pub owned_cross_domain_manifest_digest: Option<[u8; 32]>,
    pub import_collision_manifest_digest: Option<[u8; 32]>,
    /// Independent KERML11-4 and KERML11-3 operational authority identities.
    pub multiplicity_context_manifest_digest: Option<[u8; 32]>,
    pub cross_multiplicity_context_manifest_digest: Option<[u8; 32]>,
    pub descriptor_digest: [u8; 32],
    pub rule_set_version: &'static str,
    pub pinned_libraries: BTreeSet<LibraryPin>,
    pub options: SemanticOptions,
    /// Reference-bearing working input: Types with unlowered specialization
    /// assertions. These can affect name lookup even without a canonical edge.
    pub pending_specialization_scopes: BTreeSet<ElementId>,
    /// Namespaces whose declaration populations are not fully available yet.
    pub pending_namespace_scopes: BTreeSet<ElementId>,
    /// Unpublished canonical values whose required lower bound is unsatisfied.
    pub construction_obligations: Arc<BTreeSet<(ElementId, agq_kernel::PropertyId)>>,
    /// Explicit project/dependency availability; absent entries see only their own root.
    pub available_roots: Arc<BTreeMap<ElementId, BTreeSet<ElementId>>>,
    /// Validated target IDs, independently versioned from semantic rules.
    pub standard_bindings: Option<Arc<crate::StandardKermlBindings>>,
    /// Rule-qualified, exact formal library targets under this profile and model.
    pub formal_constraint_targets: Option<Arc<crate::FormalConstraintTargets>>,
    pub binding_version: &'static str,
    /// Exact canonical StandardLibrary records/occurrences, independent of authored
    /// revision changes. None means no validated canonical binding input was attached.
    pub library_graph_digest: Option<[u8; 32]>,
    /// Accepted immutable dependency, including all publication derivations.
    /// Authored revisions retain this identity without claiming their own closure.
    pub publication_dependency_digest: Option<[u8; 32]>,
    /// Phase is part of query identity; a partial overlay cannot claim closure.
    pub derivation_phase: crate::DerivationPhase,
}

/// Validated identity bound to an immutable input, never to a caller-provided revision label.
pub struct SemanticContext<'m> {
    pub(crate) model: &'m ModelView,
    pub(crate) id: SemanticContextId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextError {
    UnsupportedMetamodel,
    InvalidPendingScope(ElementId),
    /// A reviewed library fact belongs to a different explicit authority profile.
    CorrectionProfileMismatch(agq_kernel::provenance::FactKey),
    PublicationDependencyMismatch,
}

impl<'m> SemanticContext<'m> {
    /// Share the same immutable input and complete identity with a fresh query
    /// evaluator. This does not rebind, revalidate or change any evidence scope.
    pub fn fork(&self) -> Self {
        Self {
            model: self.model,
            id: self.id.clone(),
        }
    }
    /// Attach bindings only after validating them against this exact canonical view.
    pub fn with_standard_bindings(
        mut self,
        roots: &[ElementId],
        library_set: &crate::LibrarySetIdentity,
    ) -> Result<Self, crate::BindingError> {
        let queries = crate::KerMlQueries::new(self);
        let bindings = crate::StandardKermlBindings::validate(&queries, roots, library_set)?;
        self = queries.context;
        self.id.standard_bindings = Some(Arc::new(bindings));
        self.id.library_graph_digest =
            Some(crate::context_digest::library_graph_digest(self.model));
        Ok(self)
    }
    /// Bind an unpublished candidate without claiming structural publication.
    /// A query reading an outstanding obligation must report incompleteness.
    pub fn for_construction(
        candidate: &'m agq_kernel::ConstructionView,
        options: SemanticOptions,
        libraries: BTreeSet<LibraryPin>,
    ) -> Result<Self, ContextError> {
        let mut context = Self::bind(candidate.model(), candidate.revision(), options, libraries)?;
        context.id.construction_obligations = candidate
            .obligations()
            .iter()
            .map(|o| (o.element, o.property))
            .collect::<BTreeSet<_>>()
            .into();
        Ok(context)
    }
    /// Bind an unpublished project candidate, including pending source scopes
    /// and structural obligations. This never promotes a candidate to a Snapshot.
    pub fn for_project_construction(
        candidate: &'m agq_kernel::ConstructionView,
        options: SemanticOptions,
        libraries: BTreeSet<LibraryPin>,
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<Self, ContextError> {
        Self::for_construction(candidate, options, libraries)?
            .with_pending_scopes(pending_specializations, pending_namespaces)
    }
    /// Bind a producer frontier over an unpublished project. The current
    /// frontier's obligations and pending source scopes remain incomplete.
    pub fn for_project_construction_overlay(
        overlay: &'m ConstructionOverlay,
        options: SemanticOptions,
        libraries: BTreeSet<LibraryPin>,
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<Self, ContextError> {
        Self::for_construction_overlay(overlay, options, libraries)?
            .with_pending_scopes(pending_specializations, pending_namespaces)
    }
    fn with_pending_scopes(
        mut self,
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
    ) -> Result<Self, ContextError> {
        for (&class, scopes) in [
            (&agq_kerml::classes::TYPE, &pending_specializations),
            (&agq_kerml::classes::NAMESPACE, &pending_namespaces),
        ] {
            for &id in scopes {
                if !self.model.element(id).is_some_and(|record| {
                    self.model
                        .registry()
                        .is_subtype(record.metaclass(), class)
                        .unwrap_or(false)
                }) {
                    return Err(ContextError::InvalidPendingScope(id));
                }
            }
        }
        self.id.pending_specialization_scopes = pending_specializations;
        self.id.pending_namespace_scopes = pending_namespaces;
        Ok(self)
    }
    /// Install exact root availability. Library roots can exclude authored roots
    /// while authored projects explicitly depend on the library roots.
    pub fn with_available_roots(
        mut self,
        mut roots: BTreeMap<ElementId, BTreeSet<ElementId>>,
    ) -> Result<Self, ContextError> {
        for id in roots.keys().chain(roots.values().flatten()) {
            if !self.model.element(*id).is_some_and(|record| {
                self.model
                    .registry()
                    .is_subtype(record.metaclass(), agq_kerml::classes::NAMESPACE)
                    .unwrap_or(false)
            }) || self
                .model
                .incoming(*id)
                .any(|r| r.property == agq_kerml::properties::RELATIONSHIP_OWNED_RELATED_ELEMENT)
            {
                return Err(ContextError::InvalidPendingScope(*id));
            }
        }
        for (root, available) in &mut roots {
            available.insert(*root);
        }
        self.id.available_roots = Arc::new(roots);
        Ok(self)
    }
    /// Bind a project with unavailable declaration evidence. A namespace barrier
    /// prevents a lookup miss (or a candidate) from being reported as definitive.
    pub fn for_project_snapshot(
        snapshot: &'m Snapshot,
        options: SemanticOptions,
        libraries: BTreeSet<LibraryPin>,
        pending_specialization_scopes: BTreeSet<ElementId>,
        pending_namespace_scopes: BTreeSet<ElementId>,
    ) -> Result<Self, ContextError> {
        let mut context = Self::for_working_snapshot(
            snapshot,
            options,
            libraries,
            pending_specialization_scopes,
        )?;
        for &id in &pending_namespace_scopes {
            if !snapshot.model().element(id).is_some_and(|record| {
                snapshot
                    .model()
                    .registry()
                    .is_subtype(record.metaclass(), agq_kerml::classes::NAMESPACE)
                    .unwrap_or(false)
            }) {
                return Err(ContextError::InvalidPendingScope(id));
            }
        }
        context.id.pending_namespace_scopes = pending_namespace_scopes;
        Ok(context)
    }
    /// Bind unresolved working assertions by their semantic owning Types, without
    /// importing source syntax. The set participates in the full context identity.
    pub fn for_working_snapshot(
        snapshot: &'m Snapshot,
        options: SemanticOptions,
        libraries: BTreeSet<LibraryPin>,
        pending_specialization_scopes: BTreeSet<ElementId>,
    ) -> Result<Self, ContextError> {
        let mut context = Self::for_snapshot(snapshot, options, libraries)?;
        for &id in &pending_specialization_scopes {
            if !snapshot.model().element(id).is_some_and(|record| {
                snapshot
                    .model()
                    .registry()
                    .is_subtype(record.metaclass(), agq_kerml::classes::TYPE)
                    .unwrap_or(false)
            }) {
                return Err(ContextError::InvalidPendingScope(id));
            }
        }
        context.id.pending_specialization_scopes = pending_specialization_scopes;
        Ok(context)
    }
    pub fn for_snapshot(
        snapshot: &'m Snapshot,
        options: SemanticOptions,
        libraries: BTreeSet<LibraryPin>,
    ) -> Result<Self, ContextError> {
        Self::bind(snapshot.model(), snapshot.revision(), options, libraries)
    }
    pub fn for_overlay(
        overlay: &'m DerivedOverlay,
        options: SemanticOptions,
        libraries: BTreeSet<LibraryPin>,
    ) -> Result<Self, ContextError> {
        let mut context = Self::bind(overlay.model(), overlay.base_revision(), options, libraries)?;
        context.id.derivation_phase = crate::DerivationPhase::PartialDerivationOverlay;
        Ok(context)
    }
    /// Bind an unpublished derivation without claiming producer closure or
    /// structural publication. Obligations describe this frontier, including
    /// missing values on newly derived records.
    pub fn for_construction_overlay(
        overlay: &'m ConstructionOverlay,
        options: SemanticOptions,
        libraries: BTreeSet<LibraryPin>,
    ) -> Result<Self, ContextError> {
        let mut context = Self::bind(overlay.model(), overlay.base_revision(), options, libraries)?;
        context.id.derivation_phase = crate::DerivationPhase::PartialDerivationOverlay;
        context.id.construction_obligations = overlay
            .obligations()
            .iter()
            .map(|o| (o.element, o.property))
            .collect::<BTreeSet<_>>()
            .into();
        Ok(context)
    }
    fn bind(
        model: &'m ModelView,
        revision: RevisionId,
        options: SemanticOptions,
        pinned_libraries: BTreeSet<LibraryPin>,
    ) -> Result<Self, ContextError> {
        // Require the exact normative KerML contracts while admitting independent
        // metaclasses and subclasses from a dependency extension such as SysML.
        let profile = options.baseline_profile;
        use agq_kernel::provenance::{DeclaredOrigin, FactKey, Origin};
        let mismatch = |origin: &Origin| {
            matches!(origin,
            Origin::Declared(DeclaredOrigin::ReviewedCorrection { profile: correction, .. })
                if !profile.accepts_correction_profile(correction))
                || matches!(origin, Origin::Derived(explanation)
                    if crate::result_structure::structural_rule_profile(explanation.rule)
                        .is_some_and(|producer| producer != profile))
        };
        for record in model.elements() {
            if mismatch(record.origin()) {
                return Err(ContextError::CorrectionProfileMismatch(FactKey::Element(
                    record.id(),
                )));
            }
            for (property, slot) in record.slots() {
                if mismatch(slot.origin()) {
                    return Err(ContextError::CorrectionProfileMismatch(FactKey::Property {
                        element: record.id(),
                        property,
                    }));
                }
            }
        }
        for link in model.association_occurrences() {
            if mismatch(link.origin()) {
                return Err(ContextError::CorrectionProfileMismatch(
                    FactKey::AssociationOccurrence(link.id()),
                ));
            }
        }
        let expected = agq_kerml::descriptors_for_profile(profile)
            .map_err(|_| ContextError::UnsupportedMetamodel)?;
        let registry = model.registry();
        let base = agq_kerml::registry_for_profile(profile)
            .map_err(|_| ContextError::UnsupportedMetamodel)?;
        // A profile cannot admit excluded raw descriptors through an extension.
        if agq_kerml::published_descriptors().sources.keys().any(|id| {
            !expected.sources.contains_key(id)
                && (registry.source(*id).is_some()
                    || match id {
                        agq_kernel::metamodel::DescriptorId::Property(id) => {
                            registry.property(*id).is_ok()
                        }
                        agq_kernel::metamodel::DescriptorId::Association(id) => {
                            registry.association(*id).is_ok()
                        }
                        _ => false,
                    })
        }) {
            return Err(ContextError::UnsupportedMetamodel);
        }
        if expected
            .models
            .iter()
            .any(|d| registry.metamodel(d.id) != Ok(d))
            || expected.classes.iter().any(|d| {
                registry.class(d.id) != Ok(d)
                    || registry
                        .effective_properties(d.id)
                        .map(|p| p.map(|p| p.id).collect::<BTreeSet<_>>())
                        != base
                            .effective_properties(d.id)
                            .map(|p| p.map(|p| p.id).collect())
            })
            || expected
                .properties
                .iter()
                .any(|d| registry.property(d.id) != Ok(d))
            || expected
                .associations
                .iter()
                .any(|d| registry.association(d.id) != Ok(d))
            || expected
                .enumerations
                .iter()
                .any(|d| registry.enumeration(d.id) != Ok(d))
            || expected
                .primitives
                .iter()
                .any(|d| registry.primitive(d.id) != Ok(d))
            || expected
                .sources
                .iter()
                .any(|(id, source)| registry.source(*id) != Some(source))
        {
            return Err(ContextError::UnsupportedMetamodel);
        }
        // Private, deterministic and rule-versioned; not a persistence format.
        let descriptors = format!("{registry:?}");
        let model_digest = crate::context_digest::model_digest(model);
        Ok(Self {
            model,
            id: SemanticContextId {
                revision,
                model_digest,
                baseline_profile_id: profile.id(),
                metamodel_version: METAMODEL_VERSION,
                errata_manifest_digest: profile.errata_manifest_sha256(),
                result_domain_manifest_digest: profile.result_domain_manifest_sha256(),
                reference_binding_manifest_digest: profile.reference_binding_manifest_sha256(),
                owned_cross_feature_manifest_digest: profile.owned_cross_feature_manifest_sha256(),
                owned_cross_domain_manifest_digest: profile.owned_cross_domain_manifest_sha256(),
                import_collision_manifest_digest: profile.import_collision_manifest_sha256(),
                multiplicity_context_manifest_digest: profile
                    .multiplicity_context_manifest_sha256(),
                cross_multiplicity_context_manifest_digest: profile
                    .cross_multiplicity_context_manifest_sha256(),
                descriptor_digest: Sha256::digest(descriptors.as_bytes()).into(),
                rule_set_version: RULE_SET_VERSION,
                pinned_libraries,
                options,
                pending_specialization_scopes: BTreeSet::new(),
                pending_namespace_scopes: BTreeSet::new(),
                construction_obligations: Arc::default(),
                available_roots: Arc::default(),
                standard_bindings: None,
                formal_constraint_targets: None,
                binding_version: crate::BINDING_VERSION,
                library_graph_digest: None,
                derivation_phase: crate::DerivationPhase::Declared,
                publication_dependency_digest: None,
            },
        })
    }
    pub fn id(&self) -> &SemanticContextId {
        &self.id
    }
}
