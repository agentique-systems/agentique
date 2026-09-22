//! Frozen SysML interpretation inputs carried by the shared query context.
use crate::{SYSML_METAMODEL_VERSION, SysmlDependencyContract};
use serde_json::json;
use sha2::{Digest, Sha256};

/// Versioned key in the language-neutral semantic-context extension map.
pub const SYSML_SEMANTIC_CONTEXT_DOMAIN: &str = "agentique-sysml-dependency-context/1";

impl SysmlDependencyContract {
    /// Content identity for the frozen SysML inputs of a combined producer run.
    ///
    /// This identity includes every dependency field, including the accepted
    /// KerML publication, both profile authorities, descriptors, manifests,
    /// source content, bindings and rule sets. Current graph content, pending
    /// inputs and derivation phase remain in the shared `SemanticContextId`.
    ///
    /// Computing a digest is not authentication or publication acceptance. Use
    /// `checked_in_for_profile` and the context attachment checks before binding
    /// it with `SemanticContext::with_semantic_extension_identity`.
    pub fn context_identity_digest(&self) -> [u8; 32] {
        let identity = json!({
            "format": SYSML_SEMANTIC_CONTEXT_DOMAIN,
            "sysml_metamodel": SYSML_METAMODEL_VERSION,
            "kerml_publication_digest": self.kerml_publication_digest,
            "kerml_profile": self.kerml_profile,
            "kerml_rule_set": self.kerml_rule_set,
            "kerml_libraries": {
                "artifacts": self.kerml_libraries.artifacts.iter().map(|(artifact, id)| json!({
                    "resource": artifact.resource(), "library_id": id.to_string(),
                })).collect::<Vec<_>>(),
                "pins": self.kerml_libraries.pins.iter().map(|pin| json!({
                    "resource": pin.name, "sha256": pin.sha256,
                })).collect::<Vec<_>>(),
            },
            "kerml_source_content_set": self.kerml_source_content_set,
            "kerml_descriptor_digest": self.kerml_descriptor_digest,
            "combined_descriptor_digest": self.combined_descriptor_digest,
            "sysml_rule_set": self.sysml_rule_set,
            "sysml_profile": self.sysml_profile.id(),
            "grammar_compatibility_manifest_digest": self.grammar_compatibility_manifest_digest,
            "semantic_correction_manifest_digest": self.semantic_correction_manifest_digest,
            "systems_library": {
                "library_id": self.systems_library.library.to_string(),
                "artifact_sha256": self.systems_library.artifact_sha256,
                "source_content_set": self.systems_library.source_content_set,
            },
            "standard_bindings": self.standard_bindings.iter().map(|(role, element)| {
                let (path, metaclass) = role.specification();
                json!({"path": path, "metaclass": metaclass.to_string(), "element": element.to_string()})
            }).collect::<Vec<_>>(),
        });
        // Ordered maps and explicit scalar identities avoid Debug encodings,
        // pointer addresses and source traversal/insertion order.
        Sha256::digest(serde_json::to_vec(&identity).expect("identity consists of JSON values"))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        StandardSysmlBindings, StandardSysmlRole, SysmlBaselineProfile, SystemsLibraryIdentity,
    };
    use agq_kernel::ElementId;

    fn contract() -> SysmlDependencyContract {
        SysmlDependencyContract::checked_in_for_profile(
            &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned(
                SystemsLibraryIdentity::SOURCE_CONTENT_SET,
            )),
            SysmlBaselineProfile::OPERATIONAL_V1,
        )
        .unwrap()
    }

    #[test]
    fn every_frozen_dependency_changes_the_shared_sysml_identity() {
        let actual = contract();
        let digest = actual.context_identity_digest();
        for index in 0..17 {
            let mut changed = actual.clone();
            match index {
                0 => changed.kerml_publication_digest[0] ^= 1,
                1 => changed.kerml_profile.push('x'),
                2 => changed.kerml_rule_set.push('x'),
                3 => changed.kerml_libraries.artifacts.clear(),
                4 => changed.kerml_libraries.pins.clear(),
                5 => changed.kerml_source_content_set.push('x'),
                6 => changed.kerml_descriptor_digest[0] ^= 1,
                7 => changed.combined_descriptor_digest[0] ^= 1,
                8 => changed.sysml_rule_set.push('x'),
                9 => changed.sysml_profile = SysmlBaselineProfile::PUBLISHED,
                10 => {
                    changed
                        .grammar_compatibility_manifest_digest
                        .as_mut()
                        .unwrap()[0] ^= 1
                }
                11 => {
                    changed
                        .semantic_correction_manifest_digest
                        .as_mut()
                        .unwrap()[0] ^= 1
                }
                12 => changed.systems_library.library = agq_kernel::LibraryId::from_u128(1),
                13 => changed.systems_library.artifact_sha256.push('x'),
                14 => changed.systems_library.source_content_set[0] ^= 1,
                15 => {
                    changed
                        .standard_bindings
                        .insert(StandardSysmlRole::Part, ElementId::from_u128(1));
                }
                _ => changed.grammar_compatibility_manifest_digest = None,
            }
            assert_ne!(
                changed.context_identity_digest(),
                digest,
                "dependency {index}"
            );
        }
    }

    #[test]
    fn binding_role_endpoint_and_profile_are_deterministic_and_distinct() {
        let mut first = contract();
        first
            .standard_bindings
            .insert(StandardSysmlRole::Part, ElementId::from_u128(1));
        first
            .standard_bindings
            .insert(StandardSysmlRole::Item, ElementId::from_u128(2));
        let mut reordered = contract();
        reordered
            .standard_bindings
            .insert(StandardSysmlRole::Item, ElementId::from_u128(2));
        reordered
            .standard_bindings
            .insert(StandardSysmlRole::Part, ElementId::from_u128(1));
        assert_eq!(
            first.context_identity_digest(),
            reordered.context_identity_digest()
        );
        reordered
            .standard_bindings
            .insert(StandardSysmlRole::Part, ElementId::from_u128(3));
        assert_ne!(
            first.context_identity_digest(),
            reordered.context_identity_digest()
        );
        let mut other_role = contract();
        other_role
            .standard_bindings
            .insert(StandardSysmlRole::Items, ElementId::from_u128(2));
        other_role
            .standard_bindings
            .insert(StandardSysmlRole::Part, ElementId::from_u128(1));
        assert_ne!(
            first.context_identity_digest(),
            other_role.context_identity_digest()
        );
        let published = SysmlDependencyContract::checked_in(&StandardSysmlBindings::unbound(
            SystemsLibraryIdentity::pinned(SystemsLibraryIdentity::SOURCE_CONTENT_SET),
        ))
        .unwrap();
        assert_ne!(
            published.context_identity_digest(),
            contract().context_identity_digest()
        );
    }

    #[test]
    fn composed_sysml_context_propagates_its_contract_into_ker_ml_query_answers() {
        let snapshot = agq_kernel::Snapshot::new(std::sync::Arc::new(
            agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        ));
        let context = crate::context::fixture_context(&snapshot, Default::default());
        let expected = context.id().dependencies.context_identity_digest();
        assert_eq!(
            context.id().kerml.semantic_extensions[SYSML_SEMANTIC_CONTEXT_DOMAIN],
            expected
        );
        let answer = agq_kerml_semantics::KerMlQueries::new(context.kerml.fork())
            .owned_relationships(ElementId::from_u128(1));
        assert_eq!(
            answer.context.semantic_extensions[SYSML_SEMANTIC_CONTEXT_DOMAIN],
            expected
        );
        assert_eq!(&answer.context, &context.id().kerml);
    }
}
