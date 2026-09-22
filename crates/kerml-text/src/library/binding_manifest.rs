//! Accepted binding records can only be generated from a sealed publication.
use super::{CanonicalKermlStandardLibraries, LibraryLoadError};
use agq_kernel::provenance::FactKey;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::{Value, json};

impl CanonicalKermlStandardLibraries {
    /// Exact accepted anchors and their independent source, artifact and semantic
    /// identities. Construction bindings cannot call this publication-only API.
    pub fn binding_manifest(
        &self,
        sources: &VerifiedLibrarySet,
    ) -> Result<Value, LibraryLoadError> {
        if sources.content_set_id() != self.source_content_set() {
            return Err(LibraryLoadError::Interpretation(
                "Binding sources do not identify this accepted publication".into(),
            ));
        }
        let library_set_identity = json!({
            "artifacts": self.library_set().artifacts.iter().map(|(artifact,id)|json!({
                "artifact":artifact.resource(), "library":id.to_string(),
            })).collect::<Vec<_>>(),
            "pins": self.library_set().pins.iter().map(|pin|json!({
                "resource":pin.name, "sha256":pin.sha256,
            })).collect::<Vec<_>>(),
        });
        let mut entries = Vec::new();
        for (role, target) in self.bindings().iter() {
            let (path, metaclass) = role.specification();
            let source = self
                .source_map()
                .get(&FactKey::Element(target))
                .ok_or_else(|| {
                    LibraryLoadError::Interpretation(format!(
                        "Missing accepted source for {role:?}"
                    ))
                })?;
            let document = sources
                .documents()
                .find(|d| d.document() == source.document)
                .ok_or_else(|| {
                    LibraryLoadError::Interpretation(format!(
                        "Missing source document for {role:?}"
                    ))
                })?;
            if document.revision() != source.revision
                || document.library() != self.bindings().bound(role).library
            {
                return Err(LibraryLoadError::Interpretation(format!(
                    "Accepted source identity mismatch for {role:?}"
                )));
            }
            let actual_metaclass = self
                .overlay()
                .model()
                .element(target)
                .expect("validated bound element")
                .metaclass();
            entries.push(json!({
                "semantic_role": format!("{role:?}"),
                "operational_profile": self.profile().id(),
                "canonical_publication_digest": self.semantic_digest(),
                "library_set_identity": library_set_identity,
                "library": self.bindings().bound(role).library.to_string(),
                "artifact": role.library_artifact().resource(),
                "qualified_path": path,
                "metaclass": actual_metaclass.to_string(),
                "metaclass_name": self.overlay().model().registry().class(actual_metaclass)
                    .map_err(agq_kernel::ModelError::from)?.name,
                "expected_metaclass": metaclass.to_string(),
                "expected_metaclass_name": self.overlay().model().registry().class(metaclass)
                    .map_err(agq_kernel::ModelError::from)?.name,
                "element_id": target.to_string(),
                "source": {
                    "document_id": source.document.to_string(),
                    "source_revision_id": source.revision.to_string(),
                    "path": document.path(), "sha256": document.sha256(),
                    "byte_range": [source.range.start(), source.range.end()],
                    "syntax_node_id": source.syntax_node.map(|n| n.to_string()),
                },
            }));
        }
        Ok(json!({
            "format": "agq-kerml-accepted-bindings/2",
            "binding_contract": agq_kerml_semantics::BINDING_VERSION,
            "identity_authority": "Agentique content- and role-qualified IDs; not OMG-assigned semantic IDs",
            "scope": "Accepted canonical KerML publication",
            "operational_profile": self.profile().id(),
            "rule_set": self.context().rule_set_version,
            "source_content_set": self.source_content_set(),
            "semantic_publication_digest": self.semantic_digest(),
            "library_set_identity": library_set_identity,
            "library_set": self.library_set().artifacts.iter().map(|(artifact, id)| json!({
                "artifact": artifact.resource(), "library": id.to_string(),
            })).collect::<Vec<_>>(),
            "artifact_pins": self.library_set().pins.iter().map(|pin| json!({
                "resource": pin.name, "sha256": pin.sha256,
            })).collect::<Vec<_>>(),
            "entries": entries,
        }))
    }

    /// Compare an accepted manifest structurally; field order and line endings
    /// are not semantic. All role, source, profile and digest fields must match.
    pub fn check_binding_manifest(
        &self,
        sources: &VerifiedLibrarySet,
        candidate: &Value,
    ) -> Result<(), LibraryLoadError> {
        if &self.binding_manifest(sources)? != candidate {
            return Err(LibraryLoadError::Interpretation(
                "Accepted KerML binding manifest is stale".into(),
            ));
        }
        Ok(())
    }
}
