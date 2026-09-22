use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{Completeness, KerMlQueries, SemanticContextId};
use agq_kernel::{
    ElementId, LibraryId, MetaclassId, ModelView,
    derived::PropertyState,
    provenance::{DeclaredOrigin, Origin},
    value::{SlotValue, Value},
};
use agq_sysml::classes as sc;
use std::collections::{BTreeMap, BTreeSet};

/// Distinct original artifact, source content set and canonical library IDs.
/// The source content identity is established by the source-loading boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemsLibraryIdentity {
    pub library: LibraryId,
    pub artifact_sha256: String,
    pub source_content_set: [u8; 32],
}
impl SystemsLibraryIdentity {
    pub const ARTIFACT_SHA256: &str =
        "df7d8b2c6e08232ca7ce123a63148949c383fcbeaeba8d89c27ceece43793a1f";
    pub const LIBRARY: LibraryId = LibraryId::from_u128(0x6c418b7477045af0bce228b92196cb15);
    /// Identity of a candidate made from the pinned final SysML 2.0 KPAR.
    /// This records input identity, never a claim of publication closure.
    pub fn pinned(source_content_set: [u8; 32]) -> Self {
        Self {
            library: Self::LIBRARY,
            artifact_sha256: Self::ARTIFACT_SHA256.into(),
            source_content_set,
        }
    }
    pub(crate) fn is_pinned(&self) -> bool {
        self.library == Self::LIBRARY && self.artifact_sha256 == Self::ARTIFACT_SHA256
    }
}

/// Only anchors needed by the first Item/Part semantic slice. KerML anchors
/// (including Attribute DataValue/dataValues) retain their accepted KerML IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardSysmlRole {
    Item,
    Items,
    Part,
    Parts,
}
impl StandardSysmlRole {
    pub const ALL: [Self; 4] = [Self::Item, Self::Items, Self::Part, Self::Parts];
    pub fn specification(self) -> (&'static [&'static str], MetaclassId) {
        match self {
            Self::Item => (&["Items", "Item"], sc::ITEM_DEFINITION),
            Self::Items => (&["Items", "items"], sc::ITEM_USAGE),
            Self::Part => (&["Parts", "Part"], sc::PART_DEFINITION),
            Self::Parts => (&["Parts", "parts"], sc::PART_USAGE),
        }
    }
}

/// Validated subset of semantic anchors. Absent roles stay absent; no fallback
/// creates declarations or silently aliases the unapproved subitem/subitems gap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandardSysmlBindings {
    identity: SystemsLibraryIdentity,
    targets: BTreeMap<StandardSysmlRole, ElementId>,
    model_digest: Option<[u8; 32]>,
    descriptor_digest: Option<[u8; 32]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SysmlBindingError {
    LibraryIdentity,
    QueryModelMismatch,
    Missing(StandardSysmlRole),
    Ambiguous(StandardSysmlRole),
    Incomplete(StandardSysmlRole),
    WrongMetaclass(StandardSysmlRole, ElementId),
    WrongLibrary(StandardSysmlRole, ElementId),
    Inaccessible(StandardSysmlRole, ElementId),
}
impl std::fmt::Display for SysmlBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SysmlBindingError {}

impl StandardSysmlBindings {
    /// Explicitly no Systems Library anchor declarations are available yet.
    pub fn unbound(identity: SystemsLibraryIdentity) -> Self {
        Self {
            identity,
            targets: BTreeMap::new(),
            model_digest: None,
            descriptor_digest: None,
        }
    }
    /// Validate only requested anchors: exact owned path, public visibility,
    /// metaclass, unique declaration, and pinned canonical library provenance.
    pub fn validate(
        model: &ModelView,
        queries: &KerMlQueries<'_>,
        identity: SystemsLibraryIdentity,
        roots: &[ElementId],
        required: impl IntoIterator<Item = StandardSysmlRole>,
    ) -> Result<Self, SysmlBindingError> {
        if !identity.is_pinned() {
            return Err(SysmlBindingError::LibraryIdentity);
        }
        if !std::ptr::eq(model, queries.model()) {
            return Err(SysmlBindingError::QueryModelMismatch);
        }
        let mut out = Self::unbound(identity);
        for role in required {
            let (path, expected) = role.specification();
            let mut scopes = roots.to_vec();
            for (index, segment) in path.iter().enumerate() {
                let mut matches = BTreeSet::new();
                for scope in scopes {
                    let owned = queries.owned_relationships(scope);
                    if owned.completeness != Completeness::Complete {
                        return Err(SysmlBindingError::Incomplete(role));
                    }
                    for membership in owned.value {
                        let Some(record) = model.element(membership) else {
                            return Err(SysmlBindingError::QueryModelMismatch);
                        };
                        if !model
                            .registry()
                            .is_subtype(record.metaclass(), kc::OWNING_MEMBERSHIP)
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        let member = queries.member(membership);
                        if member.completeness != Completeness::Complete {
                            return Err(SysmlBindingError::Incomplete(role));
                        }
                        let Some(element) = member.value else {
                            return Err(SysmlBindingError::Incomplete(role));
                        };
                        let Some(target) = model.element(element) else {
                            return Err(SysmlBindingError::QueryModelMismatch);
                        };
                        let Ok(PropertyState::Computed(name)) =
                            model.property_state(element, kp::ELEMENT_DECLARED_NAME)
                        else {
                            continue;
                        };
                        if name.value() != &SlotValue::Scalar(Value::String((*segment).into())) {
                            continue;
                        }
                        let Ok(PropertyState::Computed(visibility)) =
                            model.property_state(membership, kp::MEMBERSHIP_VISIBILITY)
                        else {
                            return Err(SysmlBindingError::Incomplete(role));
                        };
                        let SlotValue::Scalar(Value::Enumeration(literal)) = visibility.value()
                        else {
                            return Err(SysmlBindingError::Inaccessible(role, membership));
                        };
                        let visibility_property = model
                            .registry()
                            .property(kp::MEMBERSHIP_VISIBILITY)
                            .expect("pinned visibility descriptor");
                        let public = match visibility_property.value_kind {
                            agq_kernel::metamodel::ValueKind::Enumeration(domain) => {
                                model.registry().enumeration(domain).is_ok_and(|domain| {
                                    domain
                                        .literals
                                        .get(literal)
                                        .is_some_and(|name| name == "public")
                                })
                            }
                            _ => false,
                        };
                        if !public {
                            return Err(SysmlBindingError::Inaccessible(role, membership));
                        }
                        let origin = Origin::Declared(DeclaredOrigin::StandardLibrary {
                            library: out.identity.library,
                        });
                        if [
                            record.origin(),
                            target.origin(),
                            name.origin(),
                            visibility.origin(),
                        ]
                        .into_iter()
                        .any(|o| o != &origin)
                        {
                            return Err(SysmlBindingError::WrongLibrary(role, element));
                        }
                        let class = if index + 1 == path.len() {
                            expected
                        } else {
                            kc::LIBRARY_PACKAGE
                        };
                        if target.metaclass() != class {
                            return Err(SysmlBindingError::WrongMetaclass(role, element));
                        }
                        matches.insert(element);
                    }
                }
                let element = match matches.len() {
                    0 => return Err(SysmlBindingError::Missing(role)),
                    1 => *matches.first().expect("one match"),
                    _ => return Err(SysmlBindingError::Ambiguous(role)),
                };
                scopes = vec![element];
            }
            out.targets.insert(role, scopes[0]);
        }
        out.model_digest = Some(queries.context().model_digest);
        out.descriptor_digest = Some(queries.context().descriptor_digest);
        Ok(out)
    }
    pub fn identity(&self) -> &SystemsLibraryIdentity {
        &self.identity
    }
    pub fn targets(&self) -> &BTreeMap<StandardSysmlRole, ElementId> {
        &self.targets
    }
    pub fn get(&self, role: StandardSysmlRole) -> Option<ElementId> {
        self.targets.get(&role).copied()
    }
    pub(crate) fn valid_for(&self, context: &SemanticContextId) -> bool {
        self.targets.is_empty()
            || self.model_digest == Some(context.model_digest)
                && self.descriptor_digest == Some(context.descriptor_digest)
    }
}
