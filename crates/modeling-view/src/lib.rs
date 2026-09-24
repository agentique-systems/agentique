//! Read-only, revision-bound semantic lenses for Agentique Studio.
//!
//! Definitions select canonical elements; they never own model content. Returned
//! DTOs are disposable projections, not an editable graph or validation authority.
//! Pan, layout, selection and semantic zoom need no reconstruction or validation.
#![forbid(unsafe_code)]

mod explain;
mod inspector;
mod projection;

pub use explain::*;
pub use inspector::*;
pub use projection::*;

use agq_kernel::{ElementId, RuleId};
use agq_modeling_workspace::ProjectRevisionId;
use serde::{Deserialize, Serialize};

/// A missing model, missing identity or unsupported saved-view version.
#[derive(Debug, thiserror::Error)]
pub enum ViewError {
    /// This Working revision has no semantic construction available yet.
    #[error("semantic graph unavailable for revision {0}")]
    Unavailable(ProjectRevisionId),
    /// The requested identity does not occur in this exact revision.
    #[error("element {0} is absent from the selected revision")]
    MissingElement(ElementId),
    /// Saved view versions must be understood before they can be applied.
    #[error("unsupported view definition version {0}")]
    UnsupportedVersion(u32),
    /// Effective semantic context is unavailable; no empty successful answer is invented.
    #[error("semantic query unavailable: {0}")]
    Query(String),
}

/// Product lens, separate from modeled SysML View/Viewpoint elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewKind {
    Architecture,
    SemanticGraph,
    Requirements,
}

/// Canonical relationship families which a human may independently display.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RelationshipFamily {
    Ownership,
    Typing,
    Specialization,
    Subsetting,
    Redefinition,
    Connection,
    Requirement,
    Verification,
    Reference,
}
impl RelationshipFamily {
    /// All supported relation families, in presentation order.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Ownership,
            Self::Typing,
            Self::Specialization,
            Self::Subsetting,
            Self::Redefinition,
            Self::Connection,
            Self::Requirement,
            Self::Verification,
            Self::Reference,
        ]
    }
}

/// Versioned project-side metadata. Hiding an element never deletes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDefinition {
    pub version: u32,
    pub name: String,
    pub kind: ViewKind,
    /// Optional center for a bounded semantic neighborhood.
    pub focus: Option<ElementId>,
    /// Maximum relationship hops from focus, capped at eight by the engine.
    pub depth: u8,
    pub relationship_families: Vec<RelationshipFamily>,
    /// Expand only standards adjacent to selected local records, never the corpus.
    pub include_standard_library: bool,
    /// Presentation-only omissions; semantic content is untouched.
    pub hidden_elements: Vec<ElementId>,
}
impl ViewDefinition {
    pub const VERSION: u32 = 1;
    /// A quiet authored architecture lens, with no frozen node copies.
    pub fn architecture() -> Self {
        Self {
            version: Self::VERSION,
            name: "Agentique Architecture".into(),
            kind: ViewKind::Architecture,
            focus: None,
            depth: 2,
            relationship_families: vec![
                RelationshipFamily::Ownership,
                RelationshipFamily::Typing,
                RelationshipFamily::Connection,
            ],
            include_standard_library: false,
            hidden_elements: vec![],
        }
    }
    /// An authored graph; standards require deliberate expansion.
    pub fn semantic_graph() -> Self {
        Self {
            name: "Semantic Graph".into(),
            kind: ViewKind::SemanticGraph,
            relationship_families: RelationshipFamily::all(),
            ..Self::architecture()
        }
    }
    /// Requirement context is selected by metaclass, not label text.
    pub fn requirements() -> Self {
        Self {
            name: "Requirements".into(),
            kind: ViewKind::Requirements,
            relationship_families: RelationshipFamily::all(),
            ..Self::architecture()
        }
    }
}
impl Default for ViewDefinition {
    fn default() -> Self {
        Self::architecture()
    }
}

/// Provenance category; the exact source or producer is available through inspection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewOrigin {
    Authored,
    Derived,
    Standard,
    Generated,
}

/// Small canonical feature reference; identity is never inherited by copying.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSummary {
    pub id: ElementId,
    pub name: String,
    pub semantic_kind: String,
}

/// Counts of canonical directly owned features, not effective/inherited totals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureCounts {
    pub parts: usize,
    pub ports: usize,
    pub requirements: usize,
}

/// One disposable display projection of an original canonical element.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewNode {
    pub id: ElementId,
    pub revision_id: ProjectRevisionId,
    pub semantic_kind: String,
    pub name: String,
    pub qualified_name: Option<String>,
    pub owner: Option<ElementId>,
    pub origin: ViewOrigin,
    pub source_available: bool,
    pub features: Vec<FeatureSummary>,
    pub counts: FeatureCounts,
    pub badges: Vec<String>,
}

/// A semantic edge, with a stable presentation identity separate from its record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewEdge {
    pub id: String,
    pub relationship_id: Option<ElementId>,
    pub revision_id: ProjectRevisionId,
    pub family: RelationshipFamily,
    pub semantic_kind: String,
    pub source: ElementId,
    pub target: ElementId,
    pub origin: ViewOrigin,
    pub rule_id: Option<RuleId>,
    pub label: String,
    /// Connection endpoint order does not by itself establish flow direction.
    pub directed: bool,
    /// Endpoint order read from the canonical relation, never layout order.
    pub order: usize,
}

/// Ownership grouping hint; groups do not contain alternate model records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewGroup {
    pub element_id: ElementId,
    pub children: Vec<ElementId>,
}

/// Explicit current-graph scope and closure status, without claiming validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewMetadata {
    /// Structurally selected system root; no project-specific name matching.
    pub suggested_focus: Option<ElementId>,
    pub scope: String,
    pub producer_completeness: String,
    pub local_element_count: usize,
    pub omitted_standard_endpoints: usize,
    pub warnings: Vec<String>,
}

/// Every node, edge, inspector and explanation binds this exact immutable revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewProjection {
    pub revision_id: ProjectRevisionId,
    pub view: ViewDefinition,
    pub nodes: Vec<ViewNode>,
    pub edges: Vec<ViewEdge>,
    pub groups: Vec<ViewGroup>,
    pub metadata: ViewMetadata,
}

/// Semantic detail is a presentation decision, independent of CSS and model mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetailLevel {
    Far,
    Medium,
    Near,
}

/// Stable defaults suitable for a 2D viewport camera's scale.
pub fn detail_level(scale: f64) -> DetailLevel {
    if scale < 0.65 {
        DetailLevel::Far
    } else if scale < 1.35 {
        DetailLevel::Medium
    } else {
        DetailLevel::Near
    }
}
