use agq_kernel::{ElementId, ModelView, RevisionId, Snapshot, derived::DerivedOverlay};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Change whenever rules, proof construction, dependency semantics or digest encoding change.
pub const RULE_SET_VERSION: &str = "agq-kerml-query/3";
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
    /// KerML Type::supertypes(excludeImplied). Applies to specialization traversal.
    pub exclude_implied: bool,
}

/// Full identity, including overlay content/provenance. A revision alone is insufficient.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticContextId {
    pub revision: RevisionId,
    pub model_digest: [u8; 32],
    pub metamodel_version: &'static str,
    pub descriptor_digest: [u8; 32],
    pub rule_set_version: &'static str,
    pub pinned_libraries: BTreeSet<LibraryPin>,
    pub options: SemanticOptions,
    /// Reference-bearing working input: Types with unlowered specialization
    /// assertions. These can affect name lookup even without a canonical edge.
    pub pending_specialization_scopes: BTreeSet<ElementId>,
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
}

impl<'m> SemanticContext<'m> {
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
        // Require the exact generated slice. Debug encoding is private, deterministic
        // (ordered maps), and rule-versioned; it is not a persistence format.
        let descriptors = format!("{:?}", model.registry());
        if descriptors
            != format!(
                "{:?}",
                agq_kerml::registry().expect("generated descriptors")
            )
        {
            return Err(ContextError::UnsupportedMetamodel);
        }
        let mut digest = Sha256::new();
        for record in model.elements() {
            let encoded = format!("{record:?}");
            digest.update((encoded.len() as u64).to_be_bytes());
            digest.update(encoded.as_bytes());
        }
        Ok(Self {
            model,
            id: SemanticContextId {
                revision,
                model_digest: digest.finalize().into(),
                metamodel_version: METAMODEL_VERSION,
                descriptor_digest: Sha256::digest(descriptors.as_bytes()).into(),
                rule_set_version: RULE_SET_VERSION,
                pinned_libraries,
                options,
                pending_specialization_scopes: BTreeSet::new(),
            },
        })
    }
    pub fn id(&self) -> &SemanticContextId {
        &self.id
    }
}
