//! Published metadata and reviewed operational interpretations are separate graphs.
use agq_kernel::{AssociationId, PropertyId, metamodel::*};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Exact published artifact identity, unchanged by operational interpretation.
pub const PUBLISHED_ARTIFACT_URI: &str = "https://www.omg.org/spec/KerML/20250201/KerML.xmi";
/// SHA-256 of the pinned, untouched KerML 1.0 XMI bytes.
pub const PUBLISHED_ARTIFACT_SHA256: &str =
    "45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466";
/// Reviewed manifest; provenance is shipped with the runtime, not fetched at build time.
pub const OPERATIONAL_ERRATA_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-errata.json");
/// Semantic errata v2 extends, and does not replace, the frozen v1 manifest.
pub const OPERATIONAL_ERRATA_V2_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-errata-v2.json");
/// Reviewed library-content corrections; original library artifacts remain unchanged.
pub const OPERATIONAL_LIBRARY_ERRATA_V3_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-library-errata-v3.json");
/// Validation-only correction; inherits the frozen v3 canonical correction facts.
pub const OPERATIONAL_VALIDATION_ERRATA_V4_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-validation-errata-v4.json");
/// Six exact formal-target corrections; no descriptor or library changes.
pub const OPERATIONAL_FORMAL_TARGET_ERRATA_V5_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-formal-target-errata-v5.json");
/// Frozen v6 profile and its two independently reviewed corrections.
pub const OPERATIONAL_PROFILE_V6_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-profile-v6.json");
/// The five KERML11-145 contextual-result corrections.
pub const OPERATIONAL_RESULT_DOMAIN_V6_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-result-domain-errata-v6.json");
/// Only the KERML11-8 implied reference-result role.
pub const OPERATIONAL_REFERENCE_BINDING_V6_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-reference-binding-errata-v6.json");
/// Reviewed KERML11-1 selector; inherited manifests remain immutable.
pub const OPERATIONAL_OWNED_CROSS_FEATURE_V7_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-owned-cross-feature-errata-v7.json");
/// Reviewed v8 publication interpretation, independently pinned.
pub const OPERATIONAL_PROFILE_V8_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-profile-v8.json");
const REVIEWED_PROFILE_V8_SHA256: &str =
    "56dc7bf0710069fbe7b6081138ee917d786d63a55d080193f7c9eb8cdce58794";
/// Reviewed v8 publication interpretation, independently pinned.
pub const OPERATIONAL_CROSS_DOMAIN_V8_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-cross-domain-errata-v8.json");
const REVIEWED_CROSS_DOMAIN_V8_SHA256: &str =
    "e5758ce87ac56d5920565d2fb2652dad2606f2cb15091489de50ebf76d263c6a";
/// Reviewed v8 publication interpretation, independently pinned.
pub const OPERATIONAL_IMPORT_COLLISION_V8_MANIFEST: &str =
    include_str!("../../../standards/kerml-1.0-operational-import-collision-errata-v8.json");
const REVIEWED_IMPORT_COLLISION_V8_SHA256: &str =
    "9f3243f46d9baad4e642dcc035c2851097e8872d19b471f03f5da37d70edca77";
const REVIEWED_V7_SHA256: &str = "57676586b516124929fad739ff06498980ef9bbeefbfcc2f4e1ce8bb4a0eb3fe";
const REVIEWED_V6_SHA256: &str = "4e9e06f8c22ac187d7a53fb2a7bdef9abba36ef374f5402adaa504b7407cf859";
const REVIEWED_RESULT_V6_SHA256: &str =
    "e901987dce4e2b05de3ecd09a2644128343d47d2e84a147f022b6d326898d5fc";
const REVIEWED_REFERENCE_V6_SHA256: &str =
    "5cdf9c17b4a3efdb84ec9c19d371a669a768fd8f00adb171379b2d73ea7bee20";
const REVIEWED_V5_SHA256: &str = "f9f95648d6f300dc60126f48196eadd295f7c12351804d18c9d718c1c1b08a97";
const REVIEWED_V4_SHA256: &str = "525bae8868e6f1ce2705eb707560ef5597dc50ca3a810070e08717b368605953";
const REVIEWED_V3_SHA256: &str = "c762275c6a7c4cb310c4ba3084683d62125a9319228d8653f39956c61794ae30";
const REVIEWED_V2_SHA256: [u8; 32] = [
    0x67, 0x44, 0xa4, 0xe5, 0x89, 0xad, 0x38, 0x78, 0xc2, 0xb7, 0x4c, 0x87, 0x60, 0xd7, 0x71, 0x45,
    0x00, 0xa1, 0x2f, 0x35, 0x84, 0xf2, 0x5c, 0xfe, 0x11, 0x75, 0xcd, 0xd5, 0x8f, 0x02, 0xa2, 0x7d,
];
/// Frozen review content. Changing a review requires an explicitly versioned implementation.
pub const REVIEWED_MANIFEST_SHA256: [u8; 32] = [
    0x76, 0x14, 0xfe, 0xc2, 0xb7, 0x5e, 0x14, 0xe7, 0x18, 0x1d, 0x7e, 0x8a, 0x0d, 0x47, 0x4c, 0xbd,
    0x30, 0x00, 0xf5, 0x3c, 0xa3, 0x47, 0x0d, 0x8d, 0x98, 0x14, 0xf4, 0xb4, 0x0f, 0xd8, 0x29, 0x94,
];

/// Closed set of implemented, reviewed transforms. A new revision requires review
/// and explicit implementation; a version label cannot enable an arbitrary patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperationalErrataProfile {
    /// KERML11-81, qualified by the exact KerML 1.0 artifact and six descriptor IDs.
    ReviewedV1,
    /// V1 plus AGQ-KERML10-002 / KERML11-140 semantic algorithm version 1.
    ReviewedV2,
    /// V2 plus reviewed KERML11-76 canonical library corrections.
    ReviewedV3,
    /// V3 plus reviewed KERML11-68 validation interpretation.
    ReviewedV4,
    /// V4 plus exactly KERML11-205/206/207 formal library targets.
    ReviewedV5,
    /// V5 plus KERML11-145 and the narrow KERML11-8 reference-binding role.
    ReviewedV6,
    /// V6 plus exactly the reviewed KERML11-1 owned-cross-feature selector.
    ReviewedV7,
    /// V7 plus owned-cross domains and KERML11-75 imported membership exclusion.
    ReviewedV8,
}

/// Language interpretation authority. This is part of semantic context identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BaselineProfile {
    /// Exact source metadata, including published anomalies.
    #[default]
    PublishedKerMl10,
    /// Agentique interpretation, never advertised as untouched published metadata.
    OperationalKerMl10 {
        errata_profile: OperationalErrataProfile,
    },
}

impl BaselineProfile {
    /// Explicit publication interpretation; availability does not establish acceptance.
    pub const OPERATIONAL_V8: Self = Self::OperationalKerMl10 {
        errata_profile: OperationalErrataProfile::ReviewedV8,
    };
    pub const fn corrects_owned_cross_domain(self) -> bool {
        matches!(self, Self::OPERATIONAL_V8)
    }
    pub const fn corrects_import_collisions(self) -> bool {
        matches!(self, Self::OPERATIONAL_V8)
    }
    pub fn owned_cross_domain_manifest_sha256(self) -> Option<[u8; 32]> {
        self.corrects_owned_cross_domain()
            .then(|| Sha256::digest(OPERATIONAL_CROSS_DOMAIN_V8_MANIFEST).into())
    }
    pub fn import_collision_manifest_sha256(self) -> Option<[u8; 32]> {
        self.corrects_import_collisions()
            .then(|| Sha256::digest(OPERATIONAL_IMPORT_COLLISION_V8_MANIFEST).into())
    }

    /// Default authority for usable generation-2 KerML model construction.
    pub const OPERATIONAL: Self = Self::OPERATIONAL_V2;

    /// Explicit v7 selector; availability does not establish library acceptance.
    pub const OPERATIONAL_V7: Self = Self::OperationalKerMl10 {
        errata_profile: OperationalErrataProfile::ReviewedV7,
    };

    /// Whether the reviewed KERML11-1 selection rules apply.
    pub const fn corrects_owned_cross_feature(self) -> bool {
        matches!(self, Self::OPERATIONAL_V7 | Self::OPERATIONAL_V8)
    }

    /// Independent correction identity for the owned-cross-feature operation.
    pub fn owned_cross_feature_manifest_sha256(self) -> Option<[u8; 32]> {
        self.corrects_owned_cross_feature()
            .then(|| Sha256::digest(OPERATIONAL_OWNED_CROSS_FEATURE_V7_MANIFEST).into())
    }

    /// Reviewed v6 structural semantics; availability is not publication acceptance.
    pub const OPERATIONAL_V6: Self = Self::OperationalKerMl10 {
        errata_profile: OperationalErrataProfile::ReviewedV6,
    };

    /// Whether the five reviewed contextual-result rules apply.
    pub const fn corrects_result_domains(self) -> bool {
        matches!(
            self,
            Self::OPERATIONAL_V6 | Self::OPERATIONAL_V7 | Self::OPERATIONAL_V8
        )
    }
    /// Whether the reviewed reference-result connector role has its narrow exception.
    pub const fn corrects_reference_binding(self) -> bool {
        matches!(
            self,
            Self::OPERATIONAL_V6 | Self::OPERATIONAL_V7 | Self::OPERATIONAL_V8
        )
    }
    /// Independent identity of the result-domain correction family.
    pub fn result_domain_manifest_sha256(self) -> Option<[u8; 32]> {
        self.corrects_result_domains()
            .then(|| Sha256::digest(OPERATIONAL_RESULT_DOMAIN_V6_MANIFEST).into())
    }
    /// Independent identity of the reference-binding correction.
    pub fn reference_binding_manifest_sha256(self) -> Option<[u8; 32]> {
        self.corrects_reference_binding()
            .then(|| Sha256::digest(OPERATIONAL_REFERENCE_BINDING_V6_MANIFEST).into())
    }

    /// Reviewed formal-target corrections, independent of publication acceptance.
    pub const OPERATIONAL_V5: Self = Self::OperationalKerMl10 {
        errata_profile: OperationalErrataProfile::ReviewedV5,
    };

    /// Whether the six reviewed formal-target replacements apply.
    pub const fn corrects_formal_constraint_targets(self) -> bool {
        matches!(
            self,
            Self::OPERATIONAL_V5
                | Self::OPERATIONAL_V6
                | Self::OPERATIONAL_V7
                | Self::OPERATIONAL_V8
        )
    }

    /// Explicit validation semantic erratum; does not itself establish publication.
    pub const OPERATIONAL_V4: Self = Self::OperationalKerMl10 {
        errata_profile: OperationalErrataProfile::ReviewedV4,
    };

    /// Whether the reviewed KERML11-68 validation rule applies.
    pub const fn corrects_redefinition_end_conformance(self) -> bool {
        matches!(
            self,
            Self::OPERATIONAL_V4
                | Self::OPERATIONAL_V5
                | Self::OPERATIONAL_V6
                | Self::OPERATIONAL_V7
                | Self::OPERATIONAL_V8
        )
    }

    /// Whether this profile includes the frozen KERML11-76 library transform.
    pub const fn corrects_library_content(self) -> bool {
        matches!(
            self,
            Self::OPERATIONAL_V3
                | Self::OPERATIONAL_V4
                | Self::OPERATIONAL_V5
                | Self::OPERATIONAL_V6
                | Self::OPERATIONAL_V7
                | Self::OPERATIONAL_V8
        )
    }

    /// A validation-only successor retains the original correction facts and IDs.
    /// This explicit lineage does not admit facts from unrelated or future profiles.
    pub fn accepts_correction_profile(self, correction: &str) -> bool {
        correction == self.id()
            || (matches!(
                self,
                Self::OPERATIONAL_V4
                    | Self::OPERATIONAL_V5
                    | Self::OPERATIONAL_V6
                    | Self::OPERATIONAL_V7
                    | Self::OPERATIONAL_V8
            ) && correction == Self::OPERATIONAL_V3.id())
    }

    /// Explicit library-content correction profile, independent of publication acceptance.
    pub const OPERATIONAL_V3: Self = Self::OperationalKerMl10 {
        errata_profile: OperationalErrataProfile::ReviewedV3,
    };

    /// Whether the reviewed KERML11-140 resolution algorithm applies.
    pub const fn corrects_redefinition_resolution(self) -> bool {
        matches!(
            self,
            Self::OPERATIONAL_V2
                | Self::OPERATIONAL_V3
                | Self::OPERATIONAL_V4
                | Self::OPERATIONAL_V5
                | Self::OPERATIONAL_V6
                | Self::OPERATIONAL_V7
                | Self::OPERATIONAL_V8
        )
    }

    /// Reviewed semantic errata v2; this identifier never follows a future default.
    pub const OPERATIONAL_V2: Self = Self::OperationalKerMl10 {
        errata_profile: OperationalErrataProfile::ReviewedV2,
    };

    /// Frozen descriptor-only operational interpretation, for reproducibility.
    pub const OPERATIONAL_V1: Self = Self::OperationalKerMl10 {
        errata_profile: OperationalErrataProfile::ReviewedV1,
    };

    /// Stable profile identity. Any future change requires a new identity.
    pub const fn id(self) -> &'static str {
        match self {
            Self::PublishedKerMl10 => "omg-kerml-1.0-published/1",
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV1,
            } => "agentique-kerml-1.0-operational/1",
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV2,
            } => "agentique-kerml-1.0-operational/2",
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV3,
            } => "agentique-kerml-1.0-operational/3",
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV4,
            } => "agentique-kerml-1.0-operational/4",
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV5,
            } => "agentique-kerml-1.0-operational/5",
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV6,
            } => "agentique-kerml-1.0-operational/6",
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV7,
            } => "agentique-kerml-1.0-operational/7",
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV8,
            } => "agentique-kerml-1.0-operational/8",
        }
    }

    /// Content identity of the reviewed interpretation, separate from source identity.
    pub fn errata_manifest_sha256(self) -> Option<[u8; 32]> {
        match self {
            Self::PublishedKerMl10 => None,
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV1,
            } => Some(Sha256::digest(OPERATIONAL_ERRATA_MANIFEST).into()),
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV2,
            } => Some(Sha256::digest(OPERATIONAL_ERRATA_V2_MANIFEST).into()),
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV3,
            } => Some(Sha256::digest(OPERATIONAL_LIBRARY_ERRATA_V3_MANIFEST).into()),
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV4,
            } => Some(Sha256::digest(OPERATIONAL_VALIDATION_ERRATA_V4_MANIFEST).into()),
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV5,
            } => Some(Sha256::digest(OPERATIONAL_FORMAL_TARGET_ERRATA_V5_MANIFEST).into()),
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV6,
            } => Some(Sha256::digest(OPERATIONAL_PROFILE_V6_MANIFEST).into()),
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV7,
            } => Some(Sha256::digest(OPERATIONAL_OWNED_CROSS_FEATURE_V7_MANIFEST).into()),
            Self::OperationalKerMl10 {
                errata_profile: OperationalErrataProfile::ReviewedV8,
            } => Some(Sha256::digest(OPERATIONAL_PROFILE_V8_MANIFEST).into()),
        }
    }
}

/// Fail-closed transform/registration errors; no partial operational set is returned.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("errata manifest content differs from the implemented review")]
    UnreviewedManifest,
    #[error("descriptor graph differs from the exact reviewed KerML 1.0 artifact")]
    UnreviewedGraph,
    #[error("reviewed deletion identity or exclusive ownership closure does not match")]
    InvalidDeletionClosure,
    #[error(transparent)]
    Metamodel(#[from] MetamodelError),
}

/// Exact published descriptor graph. Legacy `descriptors()` has this same meaning.
pub fn published_descriptors() -> DescriptorSet {
    crate::descriptors()
}

/// Explicitly choose source fidelity or a reviewed operational interpretation.
pub fn descriptors_for_profile(profile: BaselineProfile) -> Result<DescriptorSet, ProfileError> {
    match profile {
        BaselineProfile::PublishedKerMl10 => Ok(published_descriptors()),
        BaselineProfile::OperationalKerMl10 { errata_profile } => {
            operational_descriptors(errata_profile)
        }
    }
}

/// Build a separate operational graph; the raw descriptors remain independently available.
pub fn operational_descriptors(
    profile: OperationalErrataProfile,
) -> Result<DescriptorSet, ProfileError> {
    apply_operational_errata(&published_descriptors(), profile)
}

/// Strictly apply an implemented review to an exact published graph. Changes in
/// descriptors, provenance, hashes, duplicates or unrelated extensions fail closed.
/// Composed language registries must apply this transform before adding extensions.
pub fn apply_operational_errata(
    source: &DescriptorSet,
    profile: OperationalErrataProfile,
) -> Result<DescriptorSet, ProfileError> {
    match profile {
        OperationalErrataProfile::ReviewedV1 => {}
        OperationalErrataProfile::ReviewedV2
        | OperationalErrataProfile::ReviewedV3
        | OperationalErrataProfile::ReviewedV4
        | OperationalErrataProfile::ReviewedV5
        | OperationalErrataProfile::ReviewedV6
        | OperationalErrataProfile::ReviewedV7
        | OperationalErrataProfile::ReviewedV8 => {
            let digest: [u8; 32] = Sha256::digest(OPERATIONAL_ERRATA_V2_MANIFEST).into();
            if digest != REVIEWED_V2_SHA256 {
                return Err(ProfileError::UnreviewedManifest);
            }
        }
    }
    if matches!(
        profile,
        OperationalErrataProfile::ReviewedV3
            | OperationalErrataProfile::ReviewedV4
            | OperationalErrataProfile::ReviewedV5
            | OperationalErrataProfile::ReviewedV6
            | OperationalErrataProfile::ReviewedV7
            | OperationalErrataProfile::ReviewedV8
    ) && format!(
        "{:x}",
        Sha256::digest(OPERATIONAL_LIBRARY_ERRATA_V3_MANIFEST)
    ) != REVIEWED_V3_SHA256
    {
        return Err(ProfileError::UnreviewedManifest);
    }
    if matches!(
        profile,
        OperationalErrataProfile::ReviewedV4
            | OperationalErrataProfile::ReviewedV5
            | OperationalErrataProfile::ReviewedV6
            | OperationalErrataProfile::ReviewedV7
            | OperationalErrataProfile::ReviewedV8
    ) && format!(
        "{:x}",
        Sha256::digest(OPERATIONAL_VALIDATION_ERRATA_V4_MANIFEST)
    ) != REVIEWED_V4_SHA256
    {
        return Err(ProfileError::UnreviewedManifest);
    }
    if matches!(
        profile,
        OperationalErrataProfile::ReviewedV5
            | OperationalErrataProfile::ReviewedV6
            | OperationalErrataProfile::ReviewedV7
            | OperationalErrataProfile::ReviewedV8
    ) && format!(
        "{:x}",
        Sha256::digest(OPERATIONAL_FORMAL_TARGET_ERRATA_V5_MANIFEST)
    ) != REVIEWED_V5_SHA256
    {
        return Err(ProfileError::UnreviewedManifest);
    }
    if matches!(
        profile,
        OperationalErrataProfile::ReviewedV6
            | OperationalErrataProfile::ReviewedV7
            | OperationalErrataProfile::ReviewedV8
    ) {
        for (manifest, expected) in [
            (OPERATIONAL_PROFILE_V6_MANIFEST, REVIEWED_V6_SHA256),
            (
                OPERATIONAL_RESULT_DOMAIN_V6_MANIFEST,
                REVIEWED_RESULT_V6_SHA256,
            ),
            (
                OPERATIONAL_REFERENCE_BINDING_V6_MANIFEST,
                REVIEWED_REFERENCE_V6_SHA256,
            ),
        ] {
            if format!("{:x}", Sha256::digest(manifest)) != expected {
                return Err(ProfileError::UnreviewedManifest);
            }
        }
    }
    if matches!(
        profile,
        OperationalErrataProfile::ReviewedV7 | OperationalErrataProfile::ReviewedV8
    ) && format!(
        "{:x}",
        Sha256::digest(OPERATIONAL_OWNED_CROSS_FEATURE_V7_MANIFEST)
    ) != REVIEWED_V7_SHA256
    {
        return Err(ProfileError::UnreviewedManifest);
    }
    if profile == OperationalErrataProfile::ReviewedV8 {
        for (manifest, expected) in [
            (OPERATIONAL_PROFILE_V8_MANIFEST, REVIEWED_PROFILE_V8_SHA256),
            (
                OPERATIONAL_CROSS_DOMAIN_V8_MANIFEST,
                REVIEWED_CROSS_DOMAIN_V8_SHA256,
            ),
            (
                OPERATIONAL_IMPORT_COLLISION_V8_MANIFEST,
                REVIEWED_IMPORT_COLLISION_V8_SHA256,
            ),
        ] {
            if format!("{:x}", Sha256::digest(manifest)) != expected {
                return Err(ProfileError::UnreviewedManifest);
            }
        }
    }
    let review_digest: [u8; 32] = Sha256::digest(OPERATIONAL_ERRATA_MANIFEST).into();
    if review_digest != REVIEWED_MANIFEST_SHA256 {
        return Err(ProfileError::UnreviewedManifest);
    }
    let published = published_descriptors();
    if source.sources != published.sources
        || source.models != published.models
        || source.classes != published.classes
        || source.properties != published.properties
        || source.associations != published.associations
        || source.enumerations != published.enumerations
        || source.primitives != published.primitives
        || source.reviews != published.reviews
    {
        return Err(ProfileError::UnreviewedGraph);
    }
    let removed: BTreeSet<_> = DELETIONS.iter().map(|(id, _, _)| *id).collect();
    for (id, external, range) in DELETIONS {
        let Some(provenance) = source.sources.get(id) else {
            return Err(ProfileError::InvalidDeletionClosure);
        };
        if provenance.specification != "KerML"
            || provenance.version != "1.0"
            || provenance.artifact_uri != PUBLISHED_ARTIFACT_URI
            || provenance.sha256 != PUBLISHED_ARTIFACT_SHA256
            || provenance.external_id != *external
            || provenance.byte_range != *range
            || source
                .sources
                .values()
                .filter(|s| s.artifact_uri == PUBLISHED_ARTIFACT_URI && s.external_id == *external)
                .count()
                != 1
        {
            return Err(ProfileError::InvalidDeletionClosure);
        }
    }
    for association in &source.associations {
        if removed.contains(&DescriptorId::Association(association.id)) {
            let owned: BTreeSet<_> = source
                .properties
                .iter()
                .filter(|p| p.owner == PropertyOwner::Association(association.id))
                .map(|p| p.id)
                .collect();
            if owned != association.member_ends.iter().copied().collect()
                || owned
                    .iter()
                    .any(|id| !removed.contains(&DescriptorId::Property(*id)))
            {
                return Err(ProfileError::InvalidDeletionClosure);
            }
        }
    }
    let mut effective = source.clone();
    effective
        .associations
        .retain(|a| !removed.contains(&DescriptorId::Association(a.id)));
    effective
        .properties
        .retain(|p| !removed.contains(&DescriptorId::Property(p.id)));
    effective.sources.retain(|id, _| !removed.contains(id));
    // Reviews are immutable historical conformance evidence, not runtime edges.
    // All surviving structural references must still register without any repair.
    MetamodelRegistry::from_descriptors(effective.clone())?;
    Ok(effective)
}

/// Register the explicitly selected profile using ordinary kernel validation.
pub fn registry_for_profile(profile: BaselineProfile) -> Result<MetamodelRegistry, ProfileError> {
    Ok(MetamodelRegistry::from_descriptors(
        descriptors_for_profile(profile)?,
    )?)
}
// Exact reviewed source identities; independently checked against the XMI and manifest.
const DELETIONS: &[(DescriptorId, &str, [usize; 2])] = &[
    (
        DescriptorId::Property(PropertyId::from_u128(0x6e7567725da957648b99ac075c73d974)),
        "Kernel-Interactions-A_participantFeature_Interaction-",
        [230032, 230315],
    ),
    (
        DescriptorId::Property(PropertyId::from_u128(0x7e19278d53e55cb0ad3b91a380383ff9)),
        "Kernel-Interactions-A_participantFeature_Interaction-participantFeature",
        [230326, 231197],
    ),
    (
        DescriptorId::Property(PropertyId::from_u128(0xb26d54f02b81520bab0ffc1ca39f068e)),
        "Kernel-Connectors-A_participantFeature_Association-participantFeature",
        [303187, 303938],
    ),
    (
        DescriptorId::Property(PropertyId::from_u128(0xed11f4a307955d6ab39102252ee0e096)),
        "Kernel-Connectors-A_participantFeature_Association-",
        [302897, 303176],
    ),
    (
        DescriptorId::Association(AssociationId::from_u128(0x6b53ef0633525d90a0e337567f9b284f)),
        "Kernel-Connectors-A_participantFeature_Association",
        [302437, 303965],
    ),
    (
        DescriptorId::Association(AssociationId::from_u128(0x91f4b58ffd94516a8aa9e47ac23bbd62)),
        "Kernel-Interactions-A_participantFeature_Interaction",
        [229264, 231224],
    ),
];
