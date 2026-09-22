use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{Completeness, KerMlQueries, SemanticContextId};
use agq_kernel::{
    ElementId, LibraryId, MetaclassId, ModelView,
    derived::PropertyState,
    provenance::{DeclaredOrigin, FactKey, Origin, SourceOrigin},
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
    pub const SOURCE_CONTENT_SET: [u8; 32] = [
        0x6c, 0xce, 0xb5, 0x02, 0x86, 0xd6, 0xed, 0xd4, 0x11, 0xf3, 0x27, 0x20, 0x1b, 0x60, 0x16,
        0xb5, 0x65, 0x17, 0x44, 0xd4, 0xa3, 0x01, 0x75, 0xc7, 0x8e, 0x01, 0x98, 0x10, 0xd4, 0x82,
        0xe9, 0x28,
    ];
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

/// Canonical anchors required by SysML algorithms. KerML anchors retain their
/// accepted KerML identities and are never rebound here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardSysmlRole {
    Item,
    Items,
    Subitems,
    Subparts,
    Part,
    Parts,
    Action,
    Actions,
    Subactions,
    OwnedActions,
    Port,
    Ports,
    Subports,
    OwnedPorts,
    Connection,
    Connections,
    BinaryConnection,
    BinaryConnections,
    Interface,
    Interfaces,
    MessageAction,
    Messages,
    Flows,
    SuccessionFlows,
    StateAction,
    StateActions,
    Calculation,
    Calculations,
    ConstraintCheck,
    ConstraintChecks,
    CheckedConstraints,
    RequirementCheck,
    RequirementChecks,
    ConcernCheck,
    ConcernChecks,
    Case,
    Cases,
    AnalysisCase,
    AnalysisCases,
    VerificationCase,
    VerificationCases,
    UseCase,
    UseCases,
    Allocation,
    Allocations,
    MetadataItem,
    MetadataItems,
    View,
    Views,
    ViewpointCheck,
    ViewpointChecks,
    Rendering,
    Renderings,
}
impl StandardSysmlRole {
    pub const ALL: [Self; 53] = [
        Self::Item,
        Self::Items,
        Self::Subitems,
        Self::Subparts,
        Self::Part,
        Self::Parts,
        Self::Action,
        Self::Actions,
        Self::Subactions,
        Self::OwnedActions,
        Self::Port,
        Self::Ports,
        Self::Subports,
        Self::OwnedPorts,
        Self::Connection,
        Self::Connections,
        Self::BinaryConnection,
        Self::BinaryConnections,
        Self::Interface,
        Self::Interfaces,
        Self::MessageAction,
        Self::Messages,
        Self::Flows,
        Self::SuccessionFlows,
        Self::StateAction,
        Self::StateActions,
        Self::Calculation,
        Self::Calculations,
        Self::ConstraintCheck,
        Self::ConstraintChecks,
        Self::CheckedConstraints,
        Self::RequirementCheck,
        Self::RequirementChecks,
        Self::ConcernCheck,
        Self::ConcernChecks,
        Self::Case,
        Self::Cases,
        Self::AnalysisCase,
        Self::AnalysisCases,
        Self::VerificationCase,
        Self::VerificationCases,
        Self::UseCase,
        Self::UseCases,
        Self::Allocation,
        Self::Allocations,
        Self::MetadataItem,
        Self::MetadataItems,
        Self::View,
        Self::Views,
        Self::ViewpointCheck,
        Self::ViewpointChecks,
        Self::Rendering,
        Self::Renderings,
    ];
    pub fn specification(self) -> (&'static [&'static str], MetaclassId) {
        match self {
            Self::Item => (&["Items", "Item"], sc::ITEM_DEFINITION),
            Self::Items => (&["Items", "items"], sc::ITEM_USAGE),
            Self::Subitems => (&["Items", "Item", "subitems"], sc::ITEM_USAGE),
            Self::Subparts => (&["Items", "Item", "subparts"], sc::PART_USAGE),
            Self::Part => (&["Parts", "Part"], sc::PART_DEFINITION),
            Self::Parts => (&["Parts", "parts"], sc::PART_USAGE),
            Self::Action => (&["Actions", "Action"], sc::ACTION_DEFINITION),
            Self::Actions => (&["Actions", "actions"], sc::ACTION_USAGE),
            Self::Subactions => (&["Actions", "Action", "subactions"], sc::ACTION_USAGE),
            Self::OwnedActions => (&["Parts", "Part", "ownedActions"], sc::ACTION_USAGE),
            Self::Port => (&["Ports", "Port"], sc::PORT_DEFINITION),
            Self::Ports => (&["Ports", "ports"], sc::PORT_USAGE),
            Self::Subports => (&["Ports", "Port", "subports"], sc::PORT_USAGE),
            Self::OwnedPorts => (&["Parts", "Part", "ownedPorts"], sc::PORT_USAGE),
            Self::Connection => (&["Connections", "Connection"], sc::CONNECTION_DEFINITION),
            Self::Connections => (&["Connections", "connections"], sc::CONNECTION_USAGE),
            Self::BinaryConnection => (
                &["Connections", "BinaryConnection"],
                sc::CONNECTION_DEFINITION,
            ),
            Self::BinaryConnections => {
                (&["Connections", "binaryConnections"], sc::CONNECTION_USAGE)
            }
            Self::Interface => (&["Interfaces", "Interface"], sc::INTERFACE_DEFINITION),
            Self::Interfaces => (&["Interfaces", "interfaces"], sc::INTERFACE_USAGE),
            Self::MessageAction => (&["Flows", "MessageAction"], sc::FLOW_DEFINITION),
            Self::Messages => (&["Flows", "messages"], sc::FLOW_USAGE),
            Self::Flows => (&["Flows", "flows"], sc::FLOW_USAGE),
            Self::SuccessionFlows => (&["Flows", "successionFlows"], sc::FLOW_USAGE),
            Self::StateAction => (&["States", "StateAction"], sc::STATE_DEFINITION),
            Self::StateActions => (&["States", "stateActions"], sc::STATE_USAGE),
            Self::Calculation => (&["Calculations", "Calculation"], sc::CALCULATION_DEFINITION),
            Self::Calculations => (&["Calculations", "calculations"], sc::CALCULATION_USAGE),
            Self::ConstraintCheck => (
                &["Constraints", "ConstraintCheck"],
                sc::CONSTRAINT_DEFINITION,
            ),
            Self::ConstraintChecks => (&["Constraints", "constraintChecks"], sc::CONSTRAINT_USAGE),
            Self::CheckedConstraints => (
                &["Items", "Item", "checkedConstraints"],
                sc::CONSTRAINT_USAGE,
            ),
            Self::RequirementCheck => (
                &["Requirements", "RequirementCheck"],
                sc::REQUIREMENT_DEFINITION,
            ),
            Self::RequirementChecks => (
                &["Requirements", "requirementChecks"],
                sc::REQUIREMENT_USAGE,
            ),
            Self::ConcernCheck => (&["Requirements", "ConcernCheck"], sc::CONCERN_DEFINITION),
            Self::ConcernChecks => (&["Requirements", "concernChecks"], sc::CONCERN_USAGE),
            Self::Case => (&["Cases", "Case"], sc::CASE_DEFINITION),
            Self::Cases => (&["Cases", "cases"], sc::CASE_USAGE),
            Self::AnalysisCase => (
                &["AnalysisCases", "AnalysisCase"],
                sc::ANALYSIS_CASE_DEFINITION,
            ),
            Self::AnalysisCases => (&["AnalysisCases", "analysisCases"], sc::ANALYSIS_CASE_USAGE),
            Self::VerificationCase => (
                &["VerificationCases", "VerificationCase"],
                sc::VERIFICATION_CASE_DEFINITION,
            ),
            Self::VerificationCases => (
                &["VerificationCases", "verificationCases"],
                sc::VERIFICATION_CASE_USAGE,
            ),
            Self::UseCase => (&["UseCases", "UseCase"], sc::USE_CASE_DEFINITION),
            Self::UseCases => (&["UseCases", "useCases"], sc::USE_CASE_USAGE),
            Self::Allocation => (&["Allocations", "Allocation"], sc::ALLOCATION_DEFINITION),
            Self::Allocations => (&["Allocations", "allocations"], sc::ALLOCATION_USAGE),
            Self::MetadataItem => (&["Metadata", "MetadataItem"], sc::METADATA_DEFINITION),
            Self::MetadataItems => (&["Metadata", "metadataItems"], sc::ITEM_USAGE),
            Self::View => (&["Views", "View"], sc::VIEW_DEFINITION),
            Self::Views => (&["Views", "views"], sc::VIEW_USAGE),
            Self::ViewpointCheck => (&["Views", "ViewpointCheck"], sc::VIEWPOINT_DEFINITION),
            Self::ViewpointChecks => (&["Views", "viewpointChecks"], sc::VIEWPOINT_USAGE),
            Self::Rendering => (&["Views", "Rendering"], sc::RENDERING_DEFINITION),
            Self::Renderings => (&["Views", "renderings"], sc::RENDERING_USAGE),
        }
    }
    pub(crate) fn prefix_class(self, index: usize) -> MetaclassId {
        if index == 1 {
            match self {
                Self::Subitems | Self::Subparts | Self::CheckedConstraints => sc::ITEM_DEFINITION,
                Self::Subactions => sc::ACTION_DEFINITION,
                Self::Subports => sc::PORT_DEFINITION,
                Self::OwnedActions | Self::OwnedPorts => sc::PART_DEFINITION,
                _ => kc::LIBRARY_PACKAGE,
            }
        } else {
            kc::LIBRARY_PACKAGE
        }
    }
}

/// Validated subset of semantic anchors. Absent roles stay absent; no fallback
/// creates declarations or aliases a formal target absent from pinned source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandardSysmlBindings {
    identity: SystemsLibraryIdentity,
    targets: BTreeMap<StandardSysmlRole, ElementId>,
    model_digest: Option<[u8; 32]>,
    descriptor_digest: Option<[u8; 32]>,
    // Every searched population can affect path uniqueness, including roots
    // that currently contribute no matching declaration.
    path_scopes: BTreeSet<ElementId>,
    declaration_sources: BTreeMap<ElementId, SourceOrigin>,
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
    MissingSource(StandardSysmlRole, ElementId),
    WrongSource(StandardSysmlRole, ElementId),
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
            path_scopes: BTreeSet::new(),
            declaration_sources: BTreeMap::new(),
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
                    out.path_scopes.insert(scope);
                    if queries.context().pending_namespace_scopes.contains(&scope) {
                        return Err(SysmlBindingError::Incomplete(role));
                    }
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
                            role.prefix_class(index)
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
    /// Complete source acceptance against already hash-verified original KPAR
    /// documents. Candidate path binding alone is not publication acceptance.
    pub fn with_verified_sources(
        mut self,
        library: &agq_standard_libraries::VerifiedLibrary,
        source_map: &BTreeMap<FactKey, SourceOrigin>,
    ) -> Result<Self, SysmlBindingError> {
        if library.id() != self.identity.library
            || library.archive_sha256() != self.identity.artifact_sha256
            || self.identity.source_content_set != SystemsLibraryIdentity::SOURCE_CONTENT_SET
        {
            return Err(SysmlBindingError::LibraryIdentity);
        }
        for (&role, &element) in &self.targets {
            let source = source_map
                .get(&FactKey::Element(element))
                .ok_or(SysmlBindingError::MissingSource(role, element))?;
            let Some(document) = library
                .documents()
                .iter()
                .find(|document| document.document() == source.document)
            else {
                return Err(SysmlBindingError::WrongSource(role, element));
            };
            let path = role.specification().0;
            let expected_path = format!("Systems Library/{}.sysml", path[0]);
            if document.path() != expected_path
                || document.revision() != source.revision
                || source.syntax_node.is_none()
                || source.range.start() == source.range.end()
                || document
                    .source()
                    .get(source.range.start() as usize..source.range.end() as usize)
                    .is_none()
            {
                return Err(SysmlBindingError::WrongSource(role, element));
            }
            self.declaration_sources.insert(element, source.clone());
        }
        Ok(self)
    }
    /// True only for a nonempty canonical binding set with original document,
    /// revision, syntax-node and byte-range evidence checked for every role.
    pub fn sources_verified(&self) -> bool {
        !self.targets.is_empty()
            && self
                .targets
                .values()
                .all(|id| self.declaration_sources.contains_key(id))
    }
    pub fn declaration_sources(&self) -> &BTreeMap<ElementId, SourceOrigin> {
        &self.declaration_sources
    }
    pub(crate) fn valid_for(&self, context: &SemanticContextId) -> bool {
        self.targets.is_empty()
            || self.model_digest == Some(context.model_digest)
                && self.descriptor_digest == Some(context.descriptor_digest)
                && self
                    .path_scopes
                    .is_disjoint(&context.pending_namespace_scopes)
    }
}
