use agq_kernel::{ElementId, ModelView, RevisionId, Snapshot, derived::DerivedOverlay};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Change whenever rules, proof construction, dependency semantics or digest encoding change.
pub const RULE_SET_VERSION: &str = "agq-kerml-query/14";
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
    pub binding_version: &'static str,
    /// Exact canonical StandardLibrary records/occurrences, independent of authored
    /// revision changes. None means no validated canonical binding input was attached.
    pub library_graph_digest: Option<[u8; 32]>,
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
}

impl<'m> SemanticContext<'m> {
    /// Attach bindings only after validating them against this exact canonical view.
    pub fn with_standard_bindings(
        mut self,
        roots: &[ElementId],
        library: agq_kernel::LibraryId,
    ) -> Result<Self, crate::BindingError> {
        let queries = crate::KerMlQueries::new(self);
        let bindings = crate::StandardKermlBindings::validate(&queries, roots, library)?;
        self = queries.context;
        self.id.standard_bindings = Some(Arc::new(bindings));
        let mut digest = Sha256::new();
        digest.update(b"agq-canonical-library-graph/1");
        for encoded in self
            .model
            .elements()
            .filter(|r| {
                matches!(
                    r.origin(),
                    agq_kernel::provenance::Origin::Declared(
                        agq_kernel::provenance::DeclaredOrigin::StandardLibrary { .. }
                            | agq_kernel::provenance::DeclaredOrigin::ReviewedCorrection { .. }
                    )
                )
            })
            .map(|r| format!("{r:?}"))
            .chain(
                self.model
                    .association_occurrences()
                    .filter(|r| {
                        matches!(
                            r.origin(),
                            agq_kernel::provenance::DeclaredOrigin::StandardLibrary { .. }
                                | agq_kernel::provenance::DeclaredOrigin::ReviewedCorrection { .. }
                        )
                    })
                    .map(|r| format!("{r:?}")),
            )
        {
            digest.update((encoded.len() as u64).to_be_bytes());
            digest.update(encoded.as_bytes());
        }
        self.id.library_graph_digest = Some(digest.finalize().into());
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
        Self::bind(overlay.model(), overlay.base_revision(), options, libraries)
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
            if mismatch(&Origin::Declared(link.origin().clone())) {
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
        let mut digest = Sha256::new();
        for record in model.elements() {
            let encoded = format!("{record:?}");
            digest.update((encoded.len() as u64).to_be_bytes());
            digest.update(encoded.as_bytes());
        }
        for encoded in model
            .association_occurrences()
            .map(|v| format!("{v:?}"))
            .chain(model.derived_navigation_results().map(|v| format!("{v:?}")))
            .chain(model.computation_failures().map(|v| format!("{v:?}")))
            .chain(model.computation_searches().map(|v| format!("{v:?}")))
        {
            digest.update((encoded.len() as u64).to_be_bytes());
            digest.update(encoded.as_bytes());
        }
        Ok(Self {
            model,
            id: SemanticContextId {
                revision,
                model_digest: digest.finalize().into(),
                baseline_profile_id: profile.id(),
                metamodel_version: METAMODEL_VERSION,
                errata_manifest_digest: profile.errata_manifest_sha256(),
                descriptor_digest: Sha256::digest(descriptors.as_bytes()).into(),
                rule_set_version: RULE_SET_VERSION,
                pinned_libraries,
                options,
                pending_specialization_scopes: BTreeSet::new(),
                pending_namespace_scopes: BTreeSet::new(),
                construction_obligations: Arc::default(),
                available_roots: Arc::default(),
                standard_bindings: None,
                binding_version: crate::BINDING_VERSION,
                library_graph_digest: None,
            },
        })
    }
    pub fn id(&self) -> &SemanticContextId {
        &self.id
    }
}
