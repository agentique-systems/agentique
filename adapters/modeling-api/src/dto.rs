//! Explicit protocol values from the pinned Systems Modeling API 1.0 schemas.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn supplied_non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Identity-only reference, matching the normative `Identified` shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identified {
    /// The stable identifier of the referenced object.
    #[serde(rename = "@id")]
    pub id: Uuid,
}

/// Complete repository project metadata wire projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Repository project identity.
    #[serde(rename = "@id")]
    pub id: Uuid,
    /// Fixed wire discriminator, `Project`.
    #[serde(rename = "@type")]
    pub kind: &'static str,
    /// Alternate project names.
    pub alias: Vec<String>,
    /// Durable creation time in ISO 8601 format.
    pub created: String,
    /// The project's default branch.
    pub default_branch: Identified,
    /// Optional human-readable description.
    pub description: Option<String>,
    /// Human-readable name; not the identity.
    pub name: String,
}

/// Complete branch metadata. `head` and `referencedCommit` identify one commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    /// Stable branch identity.
    #[serde(rename = "@id")]
    pub id: Uuid,
    /// Fixed wire discriminator, `Branch`.
    #[serde(rename = "@type")]
    pub kind: &'static str,
    /// Alternate branch names.
    pub alias: Vec<String>,
    /// Durable creation time in ISO 8601 format.
    pub created: String,
    /// Deletion timestamp; active branches have none.
    pub deleted: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Current immutable project revision.
    pub head: Option<Identified>,
    /// Display name; not the identity.
    pub name: String,
    /// Project that owns this branch.
    pub owning_project: Identified,
    /// The commit reference redefined by `head` (PDF 7.1.2).
    pub referenced_commit: Identified,
}

/// Immutable project revision projected as a normative Commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    /// Stable project revision identity, distinct from a kernel revision.
    #[serde(rename = "@id")]
    pub id: Uuid,
    /// Fixed wire discriminator, `Commit`.
    #[serde(rename = "@type")]
    pub kind: &'static str,
    /// Alternate names.
    pub alias: Vec<String>,
    /// Durable creation time in ISO 8601 format.
    pub created: String,
    /// Optional description.
    pub description: Option<String>,
    /// Optional display name.
    pub name: Option<String>,
    /// Owning project. The pinned schema comment saying Branch is inconsistent
    /// with its property name and PDF 7.1.2; the reference is a Project.
    pub owning_project: Identified,
    /// Zero or one parent; merge commits are not supported.
    pub previous_commit: Vec<Identified>,
}

/// Normative create-project request. Unsupported supplied options are rejected.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectRequest {
    /// Optional discriminator; if supplied it must be `Project`.
    #[serde(rename = "@type", default, deserialize_with = "supplied_non_null")]
    pub kind: Option<String>,
    /// Optional alternate names.
    #[serde(default)]
    pub alias: Vec<String>,
    /// Optional explicit default branch; creation cannot reference a foreign branch.
    #[serde(default, deserialize_with = "supplied_non_null")]
    pub default_branch: Option<Identified>,
    /// Optional description.
    pub description: Option<String>,
    /// Required human-readable name.
    pub name: String,
}

/// Normative create-branch request, directly mapped to create-at-revision.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BranchRequest {
    /// Optional discriminator; if supplied it must be `Branch`.
    #[serde(rename = "@type", default, deserialize_with = "supplied_non_null")]
    pub kind: Option<String>,
    /// Optional alternate names.
    #[serde(default)]
    pub alias: Vec<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Required existing commit; null is rejected by this repository profile.
    pub head: Identified,
    /// Required human-readable name.
    pub name: String,
}

/// Honest partial Element projection: omitted properties are not fabricated.
///
/// Concrete metaclass schemas require additional inherited properties. Therefore
/// endpoints returning this projection explicitly retain partial coverage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementProjection {
    /// Canonical kernel ElementId, represented as a UUID.
    #[serde(rename = "@id")]
    pub id: Uuid,
    /// Actual canonical metaclass name; never widened to `Element` to hide data.
    #[serde(rename = "@type")]
    pub kind: String,
    /// Canonical identity string.
    pub element_id: String,
    /// Current declared name, when present.
    pub declared_name: Option<String>,
    /// Qualified name answered by the revision-bound language query.
    pub qualified_name: Option<String>,
}

/// Normative version-independent data identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DataIdentity {
    /// Canonical element identity.
    #[serde(rename = "@id")]
    pub id: Uuid,
    /// Fixed `DataIdentity` discriminator.
    #[serde(rename = "@type")]
    pub kind: &'static str,
    /// Alternate names.
    pub alias: Vec<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Optional name.
    pub name: Option<String>,
}

/// DataVersion with an element payload; payload completeness is explicit in coverage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DataVersion {
    /// Deterministic version identity, independent of branch names.
    #[serde(rename = "@id")]
    pub id: Uuid,
    /// Fixed `DataVersion` discriminator.
    #[serde(rename = "@type")]
    pub kind: &'static str,
    /// Alternate names.
    pub alias: Vec<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Version-independent identity.
    pub identity: DataIdentity,
    /// Optional name.
    pub name: Option<String>,
    /// Projected state of the canonical element.
    pub payload: Option<serde_json::Value>,
}

/// Normative diff pair; additions/removals have one absent side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDifference {
    /// Fixed `DataDifference` discriminator.
    #[serde(rename = "@type")]
    pub kind: &'static str,
    /// State in the base commit.
    pub base_data: Option<DataVersion>,
    /// State in the compared commit.
    pub compare_data: Option<DataVersion>,
}
