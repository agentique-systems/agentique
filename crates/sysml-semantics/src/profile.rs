use agq_kernel::{OutputKey, RuleId};
use sha2::{Digest, Sha256};

/// Explicit final SysML 2.0 interpretation. Published remains reproducible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SysmlBaselineProfile {
    #[default]
    Published,
    OperationalV1,
    OperationalV2,
    OperationalV3,
}

impl SysmlBaselineProfile {
    pub const PUBLISHED: Self = Self::Published;
    pub const OPERATIONAL_V1: Self = Self::OperationalV1;
    pub const OPERATIONAL_V2: Self = Self::OperationalV2;
    pub const OPERATIONAL_V3: Self = Self::OperationalV3;

    pub const fn id(self) -> &'static str {
        match self {
            Self::Published => "omg-sysml-2.0-published/1",
            Self::OperationalV1 => "agentique-sysml-2.0-operational/1",
            Self::OperationalV2 => "agentique-sysml-2.0-operational/2",
            Self::OperationalV3 => "agentique-sysml-2.0-operational/3",
        }
    }

    /// Exact formal target; operational v1 changes only its final segment.
    pub const fn composite_item_target(self) -> &'static [&'static str] {
        match self {
            Self::Published => &["Items", "Item", "subitem"],
            Self::OperationalV1 => &["Items", "Item", "subitems"],
            Self::OperationalV2 | Self::OperationalV3 => &["Items", "Item", "subitems"],
        }
    }

    pub fn semantic_correction_manifest_digest(self) -> Option<[u8; 32]> {
        match self {
            Self::Published => None,
            Self::OperationalV1 => Some(manifest_digest(include_str!(
                "../../../standards/sysml-2.0-operational-semantic-v1.json"
            ))),
            Self::OperationalV2 => Some(manifest_digest(include_str!(
                "../../../standards/sysml-2.0-operational-semantic-v2.json"
            ))),
            Self::OperationalV3 => Some(manifest_digest(include_str!(
                "../../../standards/sysml-2.0-operational-semantic-v3.json"
            ))),
        }
    }

    pub fn grammar_compatibility_manifest_digest(self) -> Option<[u8; 32]> {
        matches!(
            self,
            Self::OperationalV1 | Self::OperationalV2 | Self::OperationalV3
        )
        .then(|| {
            manifest_digest(include_str!(
                "../../../standards/grammar/sysml-2.0-operational-v1.json"
            ))
        })
    }

    /// AGQ-SYSML20-005 permits plain Association classifiers on ConnectionUsage
    /// while retaining the narrowed domains of its typed definition projections.
    /// Published and earlier operational interpretations remain unchanged.
    pub const fn permits_connection_association_types(self) -> bool {
        matches!(self, Self::OperationalV3)
    }

    /// Versioned semantic rule identity, distinct from source and syntax IDs.
    pub fn rule_id(self, rule: &str) -> RuleId {
        RuleId::from_u128(identity(&[self.id().as_bytes(), rule.as_bytes()]))
    }
}

pub(crate) fn manifest_digest(bytes: &str) -> [u8; 32] {
    Sha256::digest(bytes.replace("\r\n", "\n").as_bytes()).into()
}

pub(crate) fn output_key(role: &str, input: u128) -> OutputKey {
    OutputKey::from_u128(identity(&[role.as_bytes(), &input.to_be_bytes()]))
}

fn identity(parts: &[&[u8]]) -> u128 {
    let mut hash = Sha256::new();
    hash.update(b"agentique-sysml-semantic-id/1");
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    u128::from_be_bytes(hash.finalize()[..16].try_into().expect("SHA-256 prefix"))
}
