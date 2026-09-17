use std::fmt;

macro_rules! identity {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u128);
        impl $name {
            /// Construct an externally allocated identity; not an arena index.
            pub const fn from_u128(value: u128) -> Self {
                Self(value)
            }
            /// Obtain the identity's portable 128-bit representation.
            pub const fn as_u128(self) -> u128 {
                self.0
            }
            /// Allocate a fresh UUID-v4 identity.
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4().as_u128())
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                uuid::Uuid::from_u128(self.0).fmt(f)
            }
        }
    };
}
identity!(
    ElementId,
    "Durable semantic identity, independent of names and storage location."
);
identity!(RevisionId, "Identity of one immutable declared snapshot.");
identity!(
    MetamodelId,
    "Identity of one exact metamodel release/artifact."
);
identity!(
    MetaclassId,
    "Identity of a class descriptor within a registry."
);
identity!(
    PropertyId,
    "Identity of a property descriptor within a registry."
);
identity!(
    DocumentId,
    "Source document identity; never semantic element identity."
);
identity!(
    SyntaxNodeId,
    "Optional source node identity, independent of semantic identity."
);
identity!(
    RuleId,
    "Identity of a semantic rule, including its revision."
);
identity!(LibraryId, "Identity of a pinned standard-library artifact.");
identity!(TransformationId, "Identity of a transformation invocation.");
identity!(
    GeneratorId,
    "Identity of a non-semantic internal generator."
);
identity!(
    OutputKey,
    "Rule-local stable identity of one derived output role."
);

/// A private-kernel derivation key, not an OMG-mandated identity formula.
///
/// Rules producing several results for a subject must allocate distinct output
/// keys. Dependencies are evidence, not part of the stable output identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DerivationKey {
    /// Versioned semantic rule.
    pub rule: RuleId,
    /// Semantic subject of the derivation.
    pub subject: ElementId,
    /// Stable output role within this rule and subject.
    pub output: OutputKey,
}
impl DerivationKey {
    /// Deterministically identify an implied element in kernel ID scheme v1.
    pub fn element_id(self) -> ElementId {
        const DOMAIN: uuid::Uuid = uuid::Uuid::from_u128(0xbc17c2dfe3e44b85a4d791eef738aa91);
        let mut bytes = [0; 48];
        bytes[..16].copy_from_slice(&self.rule.as_u128().to_be_bytes());
        bytes[16..32].copy_from_slice(&self.subject.as_u128().to_be_bytes());
        bytes[32..].copy_from_slice(&self.output.as_u128().to_be_bytes());
        ElementId::from_u128(uuid::Uuid::new_v5(&DOMAIN, &bytes).as_u128())
    }
}
