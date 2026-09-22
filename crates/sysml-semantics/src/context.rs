use crate::{StandardSysmlBindings, StandardSysmlRole, SystemsLibraryIdentity};
use agq_kerml::BaselineProfile;
use agq_kerml_semantics::{
    AcceptedPublicationReceipt, CompletePublicationOverlay, ContextError, LibraryPin,
    LibrarySetIdentity, SemanticContext, SemanticContextId, SemanticOptions,
    StandardLibraryArtifact,
};
use agq_kernel::{ElementId, ModelView, Snapshot};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Identity of the implemented SysML query contract, independently of KerML rules.
pub const SYSML_RULE_SET_VERSION: &str = "agq-sysml-query/1";
/// Final SysML 2.0 formal descriptor authority, not a preliminary revision.
pub const SYSML_METAMODEL_VERSION: &str =
    "SysML/2.0;XMI:caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f";

/// Persist this contract with a project. Every field is compared on attachment;
/// changing the expected digest cannot authorize a different KerML publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SysmlDependencyContract {
    pub kerml_publication_digest: [u8; 32],
    pub kerml_profile: String,
    pub kerml_rule_set: String,
    pub kerml_libraries: LibrarySetIdentity,
    pub kerml_source_content_set: String,
    pub kerml_descriptor_digest: [u8; 32],
    pub combined_descriptor_digest: [u8; 32],
    pub sysml_rule_set: String,
    pub systems_library: SystemsLibraryIdentity,
    pub standard_bindings: BTreeMap<StandardSysmlRole, ElementId>,
}

/// Failures are explicit; none cause rebinding to the current default profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SysmlContextError {
    AcceptanceUnavailable,
    IdentityMismatch(&'static str),
    KerMl(ContextError),
}
impl std::fmt::Display for SysmlContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SysmlContextError {}
impl From<ContextError> for SysmlContextError {
    fn from(value: ContextError) -> Self {
        Self::KerMl(value)
    }
}

impl SysmlDependencyContract {
    /// Create an expected contract from compiled acceptance authority and the
    /// pinned combined descriptor graph. Systems Library construction is still
    /// a candidate; this method does not assert its semantic publication.
    pub fn checked_in(bindings: &StandardSysmlBindings) -> Result<Self, SysmlContextError> {
        if !bindings.identity().is_pinned() {
            return Err(SysmlContextError::IdentityMismatch("Systems Library pin"));
        }
        let receipt = AcceptedPublicationReceipt::checked_in()
            .map_err(|_| SysmlContextError::AcceptanceUnavailable)?;
        let manifest = receipt.binding_manifest();
        let digest = serde_json::from_value(manifest["semantic_publication_digest"].clone())
            .map_err(|_| SysmlContextError::AcceptanceUnavailable)?;
        let registry = agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9)
            .map_err(|_| SysmlContextError::IdentityMismatch("combined descriptor graph"))?;
        let empty = Snapshot::new(Arc::new(registry));
        let context = SemanticContext::for_snapshot(
            &empty,
            SemanticOptions {
                baseline_profile: BaselineProfile::OPERATIONAL_V9,
                ..Default::default()
            },
            BTreeSet::new(),
        )?;
        let kerml_empty = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V9)
                .map_err(|_| SysmlContextError::IdentityMismatch("KerML descriptor graph"))?,
        ));
        let kerml_context = SemanticContext::for_snapshot(
            &kerml_empty,
            SemanticOptions {
                baseline_profile: BaselineProfile::OPERATIONAL_V9,
                ..Default::default()
            },
            BTreeSet::new(),
        )?;
        let string = |key: &str| {
            manifest[key]
                .as_str()
                .map(str::to_owned)
                .ok_or(SysmlContextError::AcceptanceUnavailable)
        };
        Ok(Self {
            kerml_publication_digest: digest,
            kerml_profile: string("operational_profile")?,
            kerml_rule_set: string("rule_set")?,
            kerml_libraries: decode_libraries(&manifest["library_set_identity"])?,
            kerml_source_content_set: receipt.source_content_set().to_owned(),
            kerml_descriptor_digest: kerml_context.id().descriptor_digest,
            combined_descriptor_digest: context.id().descriptor_digest,
            sysml_rule_set: SYSML_RULE_SET_VERSION.to_owned(),
            systems_library: bindings.identity().clone(),
            standard_bindings: bindings.targets().clone(),
        })
    }
}

fn decode_libraries(value: &Value) -> Result<LibrarySetIdentity, SysmlContextError> {
    let error = || SysmlContextError::AcceptanceUnavailable;
    let pins = value["pins"]
        .as_array()
        .ok_or_else(error)?
        .iter()
        .map(|pin| {
            Ok(LibraryPin {
                name: pin["resource"].as_str().ok_or_else(error)?.into(),
                sha256: serde_json::from_value(pin["sha256"].clone()).map_err(|_| error())?,
            })
        })
        .collect::<Result<BTreeSet<_>, SysmlContextError>>()?;
    let artifacts = value["artifacts"]
        .as_array()
        .ok_or_else(error)?
        .iter()
        .map(|artifact| {
            let kind = StandardLibraryArtifact::ALL
                .into_iter()
                .find(|kind| json!(kind.resource()) == artifact["artifact"])
                .ok_or_else(error)?;
            Ok((
                kind,
                serde_json::from_value(artifact["library"].clone()).map_err(|_| error())?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, SysmlContextError>>()?;
    if artifacts.len() != StandardLibraryArtifact::ALL.len() {
        return Err(error());
    }
    Ok(LibrarySetIdentity { artifacts, pins })
}

/// Authored revision and all semantic dependencies of a SysML answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SysmlSemanticContextId {
    pub kerml: SemanticContextId,
    pub dependencies: SysmlDependencyContract,
    pub metamodel_version: &'static str,
}

/// Composes an accepted KerML context; there is no parallel SysML model store.
pub struct SysmlSemanticContext<'m> {
    pub(crate) model: &'m ModelView,
    pub(crate) kerml: SemanticContext<'m>,
    pub(crate) id: SysmlSemanticContextId,
    pub(crate) bindings: StandardSysmlBindings,
}

impl<'m> SysmlSemanticContext<'m> {
    /// Attach a project to its exact accepted immutable KerML dependency.
    /// The checked-in receipt is authoritative, independently of caller input.
    pub fn for_project(
        snapshot: &'m Snapshot,
        accepted: &CompletePublicationOverlay,
        project_root: ElementId,
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
        expected: &SysmlDependencyContract,
        bindings: StandardSysmlBindings,
    ) -> Result<Self, SysmlContextError> {
        let trusted = SysmlDependencyContract::checked_in(&bindings)?;
        validate_contract(expected, &trusted)?;
        validate_accepted(accepted.context(), &trusted)?;
        let kerml = accepted.project_context(
            snapshot,
            project_root,
            pending_specializations,
            pending_namespaces,
        )?;
        Self::attach(snapshot.model(), kerml, trusted, bindings)
    }
    /// Bind an unpublished Systems Library or authored construction view with
    /// exact accepted KerML dependency and honest missing-endpoint obligations.
    /// Local roots see one another; accepted roots never gain authored visibility.
    pub fn for_construction(
        candidate: &'m agq_kernel::ConstructionView,
        accepted: &CompletePublicationOverlay,
        local_roots: &[ElementId],
        pending_specializations: BTreeSet<ElementId>,
        pending_namespaces: BTreeSet<ElementId>,
        expected: &SysmlDependencyContract,
        bindings: StandardSysmlBindings,
    ) -> Result<Self, SysmlContextError> {
        let trusted = SysmlDependencyContract::checked_in(&bindings)?;
        validate_contract(expected, &trusted)?;
        validate_accepted(accepted.context(), &trusted)?;
        let kerml = accepted.project_construction_context(
            candidate,
            local_roots,
            pending_specializations,
            pending_namespaces,
        )?;
        Self::attach(candidate.model(), kerml, trusted, bindings)
    }
    fn attach(
        model: &'m ModelView,
        kerml: SemanticContext<'m>,
        trusted: SysmlDependencyContract,
        bindings: StandardSysmlBindings,
    ) -> Result<Self, SysmlContextError> {
        if kerml.id().descriptor_digest != trusted.combined_descriptor_digest {
            return Err(SysmlContextError::IdentityMismatch(
                "combined descriptor graph",
            ));
        }
        if !bindings.valid_for(kerml.id()) {
            return Err(SysmlContextError::IdentityMismatch(
                "standard SysML bindings graph",
            ));
        }
        let id = SysmlSemanticContextId {
            kerml: kerml.id().clone(),
            dependencies: trusted,
            metamodel_version: SYSML_METAMODEL_VERSION,
        };
        Ok(Self {
            model,
            kerml,
            id,
            bindings,
        })
    }
    pub fn id(&self) -> &SysmlSemanticContextId {
        &self.id
    }
    /// Borrowed canonical model, shared with the composed KerML evaluator.
    pub fn model(&self) -> &'m ModelView {
        self.model
    }
}

fn validate_contract(
    expected: &SysmlDependencyContract,
    actual: &SysmlDependencyContract,
) -> Result<(), SysmlContextError> {
    for (same, field) in [
        (
            expected.kerml_publication_digest == actual.kerml_publication_digest,
            "KerML publication digest",
        ),
        (
            expected.kerml_profile == actual.kerml_profile,
            "KerML operational profile",
        ),
        (
            expected.kerml_rule_set == actual.kerml_rule_set,
            "KerML rule set",
        ),
        (
            expected.kerml_libraries == actual.kerml_libraries,
            "KerML library set",
        ),
        (
            expected.kerml_source_content_set == actual.kerml_source_content_set,
            "KerML source identity",
        ),
        (
            expected.kerml_descriptor_digest == actual.kerml_descriptor_digest,
            "KerML descriptor graph",
        ),
        (
            expected.combined_descriptor_digest == actual.combined_descriptor_digest,
            "combined descriptor graph",
        ),
        (
            expected.sysml_rule_set == actual.sysml_rule_set,
            "SysML rule set",
        ),
        (
            expected.systems_library == actual.systems_library,
            "Systems Library identity",
        ),
        (
            expected.standard_bindings == actual.standard_bindings,
            "standard SysML bindings",
        ),
    ] {
        if !same {
            return Err(SysmlContextError::IdentityMismatch(field));
        }
    }
    Ok(())
}

fn validate_accepted(
    context: &SemanticContextId,
    trusted: &SysmlDependencyContract,
) -> Result<(), SysmlContextError> {
    if context.model_digest != trusted.kerml_publication_digest {
        return Err(SysmlContextError::IdentityMismatch(
            "accepted KerML publication digest",
        ));
    }
    if context.baseline_profile_id != trusted.kerml_profile {
        return Err(SysmlContextError::IdentityMismatch(
            "accepted KerML operational profile",
        ));
    }
    if context.rule_set_version != trusted.kerml_rule_set {
        return Err(SysmlContextError::IdentityMismatch(
            "accepted KerML rule set",
        ));
    }
    if context.descriptor_digest != trusted.kerml_descriptor_digest {
        return Err(SysmlContextError::IdentityMismatch(
            "accepted KerML descriptor graph",
        ));
    }
    if context.pinned_libraries != trusted.kerml_libraries.pins
        || context
            .standard_bindings
            .as_ref()
            .is_none_or(|b| b.library_set() != &trusted.kerml_libraries)
    {
        return Err(SysmlContextError::IdentityMismatch(
            "accepted KerML library set",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn fixture_context<'m>(
    snapshot: &'m Snapshot,
    pending: BTreeSet<ElementId>,
) -> SysmlSemanticContext<'m> {
    let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32]));
    let contract = SysmlDependencyContract::checked_in(&bindings).unwrap();
    let kerml = SemanticContext::for_project_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: BaselineProfile::OPERATIONAL_V9,
            exclude_implied: true,
        },
        BTreeSet::new(),
        pending,
        BTreeSet::new(),
    )
    .unwrap();
    let id = SysmlSemanticContextId {
        kerml: kerml.id().clone(),
        dependencies: contract,
        metamodel_version: SYSML_METAMODEL_VERSION,
    };
    SysmlSemanticContext {
        model: snapshot.model(),
        kerml,
        id,
        bindings,
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    #[test]
    fn every_expected_dependency_is_checked_without_silent_rebinding() {
        let b = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([3; 32]));
        let actual = SysmlDependencyContract::checked_in(&b).unwrap();
        assert!(validate_contract(&actual, &actual).is_ok());
        for index in 0..10 {
            let mut wrong = actual.clone();
            match index {
                0 => wrong.kerml_publication_digest[0] ^= 1,
                1 => wrong.kerml_profile = BaselineProfile::OPERATIONAL_V8.id().into(),
                2 => wrong.kerml_rule_set.push('x'),
                3 => wrong.kerml_libraries.artifacts.clear(),
                4 => wrong.kerml_source_content_set.push('x'),
                5 => wrong.combined_descriptor_digest[0] ^= 1,
                6 => wrong.sysml_rule_set.push('x'),
                7 => wrong.systems_library.source_content_set[0] ^= 1,
                8 => wrong.kerml_descriptor_digest[0] ^= 1,
                _ => {
                    wrong
                        .standard_bindings
                        .insert(StandardSysmlRole::Part, ElementId::new());
                }
            }
            assert!(validate_contract(&wrong, &actual).is_err(), "{index}");
        }
    }
}
