use crate::dto;
use agq_kernel::ElementId;
use agq_modeling_repository::{BranchId, ProjectId, ProjectRevisionId, RepositoryError};
use agq_modeling_service::{ElementDto, ModelingService, RevisionSelector};
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

/// Protocol-neutral error categories interpreted by the HTTP adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// Malformed identifiers, unsupported option combinations or invalid input.
    BadRequest,
    /// The requested project, branch, revision or element does not exist.
    NotFound,
    /// Optimistic concurrency or identity conflict.
    Conflict,
    /// Explicitly outside the supported API profile.
    Unsupported,
    /// Bounded adapter capacity is exhausted; the caller may retry later.
    Busy,
    /// Repository or semantic construction failure.
    Internal,
}

/// An owned adapter error, without database, borrowed model or HTTP types.
#[derive(Clone, Debug, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    /// Failure category.
    pub kind: ErrorKind,
    /// Human-readable cause.
    pub message: String,
}

impl ApiError {
    /// Reject unsupported protocol semantics explicitly.
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Unsupported,
            message: message.into(),
        }
    }
    /// Reject malformed input without starting a durable operation.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::BadRequest,
            message: message.into(),
        }
    }
}

impl From<RepositoryError> for ApiError {
    fn from(error: RepositoryError) -> Self {
        let kind = match &error {
            RepositoryError::NotFound(_) => ErrorKind::NotFound,
            RepositoryError::AlreadyExists(_)
            | RepositoryError::Conflict { .. }
            | RepositoryError::OperationCollision(_) => ErrorKind::Conflict,
            _ => ErrorKind::Internal,
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

fn identity(value: impl std::fmt::Display) -> Uuid {
    Uuid::parse_str(&value.to_string()).expect("repository identities are canonical UUIDs")
}

/// Convert durable metadata to the exact pinned Project response shape.
pub fn project(value: &agq_modeling_repository::Project) -> dto::Project {
    dto::Project {
        id: identity(value.id),
        kind: "Project",
        alias: value.metadata.alias.clone(),
        created: value.metadata.created.clone(),
        default_branch: dto::Identified {
            id: identity(value.default_branch),
        },
        description: value.metadata.description.clone(),
        name: value.name.clone(),
    }
}

/// Convert a durable branch reference without copying the referenced graph.
pub fn branch(value: &agq_modeling_repository::Branch) -> dto::Branch {
    let head = dto::Identified {
        id: identity(value.head),
    };
    dto::Branch {
        id: identity(value.id),
        kind: "Branch",
        alias: value.metadata.alias.clone(),
        created: value.metadata.created.clone(),
        deleted: None,
        description: value.metadata.description.clone(),
        head: Some(head),
        name: value.name.clone(),
        owning_project: dto::Identified {
            id: identity(value.project_id),
        },
        referenced_commit: head,
    }
}

/// Convert an immutable manifest to the exact pinned Commit response shape.
pub fn commit(value: &agq_modeling_repository::RevisionManifest) -> dto::Commit {
    dto::Commit {
        id: identity(value.revision_id),
        kind: "Commit",
        alias: value.metadata.alias.clone(),
        created: value.metadata.created.clone(),
        description: value.metadata.description.clone(),
        name: value.metadata.name.clone(),
        owning_project: dto::Identified {
            id: identity(value.project_id),
        },
        previous_commit: value
            .parent_revision_id
            .into_iter()
            .map(|id| dto::Identified { id: identity(id) })
            .collect(),
    }
}

/// Standard-facing facade. Every semantic operation goes through ModelingService.
pub struct ModelingApi {
    service: Arc<ModelingService>,
}

impl ModelingApi {
    /// Bind the API to one application service and authenticated publications.
    pub fn new(service: Arc<ModelingService>) -> Self {
        Self { service }
    }

    /// Read repository projects in stable identity order.
    pub fn projects(&self) -> Result<Vec<dto::Project>, ApiError> {
        Ok(self
            .service
            .repository()
            .list_projects()?
            .iter()
            .map(project)
            .collect())
    }
    /// Read one project by stable identity.
    pub fn project(&self, id: ProjectId) -> Result<dto::Project, ApiError> {
        Ok(project(&self.service.repository().get_project(id)?))
    }
    /// Create a project through the application's source-backed initial revision.
    pub fn create_project(&self, request: dto::ProjectRequest) -> Result<dto::Project, ApiError> {
        if request.name.trim().is_empty() {
            return Err(ApiError::bad_request("project name must not be blank"));
        }
        if request
            .kind
            .as_deref()
            .is_some_and(|kind| kind != "Project")
        {
            return Err(ApiError::bad_request("@type must be Project"));
        }
        if !request.alias.is_empty() || request.default_branch.is_some() {
            return Err(ApiError::unsupported(
                "project aliases and explicit defaultBranch are not supported by create-project",
            ));
        }
        self.service
            .create_project(&request.name, request.description)
            .map(|value| project(&value))
            .map_err(service_error)
    }
    /// Read all named heads in a project.
    pub fn branches(&self, project_id: ProjectId) -> Result<Vec<dto::Branch>, ApiError> {
        Ok(self
            .service
            .repository()
            .list_branches(project_id)?
            .iter()
            .map(branch)
            .collect())
    }
    /// Read one named head atomically.
    pub fn branch(
        &self,
        project_id: ProjectId,
        branch_id: BranchId,
    ) -> Result<dto::Branch, ApiError> {
        Ok(branch(
            &self
                .service
                .repository()
                .get_branch(project_id, branch_id)?,
        ))
    }
    /// Create a branch at an existing revision through the application service.
    pub fn create_branch(
        &self,
        project_id: ProjectId,
        request: dto::BranchRequest,
    ) -> Result<dto::Branch, ApiError> {
        if request.name.trim().is_empty() {
            return Err(ApiError::bad_request("branch name must not be blank"));
        }
        if request.kind.as_deref().is_some_and(|kind| kind != "Branch") {
            return Err(ApiError::bad_request("@type must be Branch"));
        }
        if !request.alias.is_empty() || request.description.is_some() {
            return Err(ApiError::unsupported(
                "branch aliases and descriptions are not supported by create-branch",
            ));
        }
        let at = request
            .head
            .id
            .to_string()
            .parse()
            .map_err(|_| ApiError::bad_request("invalid head identity"))?;
        self.service
            .create_branch(project_id, &request.name, at)
            .map(|value| branch(&value))
            .map_err(service_error)
    }
    /// Read the complete immutable revision registry, including deleted-branch history.
    pub fn commits(&self, project_id: ProjectId) -> Result<Vec<dto::Commit>, ApiError> {
        Ok(self
            .service
            .repository()
            .list_revisions(project_id)?
            .iter()
            .map(commit)
            .collect())
    }
    /// Read one immutable revision's durable metadata.
    pub fn commit(
        &self,
        project_id: ProjectId,
        revision_id: ProjectRevisionId,
    ) -> Result<dto::Commit, ApiError> {
        Ok(commit(
            &self
                .service
                .repository()
                .load_revision(project_id, revision_id)?,
        ))
    }
    /// Project declared/current canonical elements from one immutable service request.
    pub fn elements(
        &self,
        project_id: ProjectId,
        revision_id: ProjectRevisionId,
    ) -> Result<Vec<Value>, ApiError> {
        let bound = self
            .service
            .resolve(project_id, RevisionSelector::Revision(revision_id))
            .map_err(service_error)?;
        bound
            .current_elements()
            .map_err(service_error)?
            .iter()
            .filter(|value| value.id != bound.revision().root())
            .map(element)
            .collect()
    }
    /// Project one current canonical element without changing its concrete metaclass.
    pub fn element(
        &self,
        project_id: ProjectId,
        revision_id: ProjectRevisionId,
        element_id: ElementId,
    ) -> Result<Value, ApiError> {
        let bound = self
            .service
            .resolve(project_id, RevisionSelector::Revision(revision_id))
            .map_err(service_error)?;
        element(&bound.current_element(element_id).map_err(service_error)?)
    }
    /// Project root elements according to the revision-bound service contract.
    pub fn roots(
        &self,
        project_id: ProjectId,
        revision_id: ProjectRevisionId,
    ) -> Result<Vec<Value>, ApiError> {
        let bound = self
            .service
            .resolve(project_id, RevisionSelector::Revision(revision_id))
            .map_err(service_error)?;
        bound
            .roots()
            .map_err(service_error)?
            .iter()
            .map(element)
            .collect()
    }
    /// Project relationships related to a canonical element in one immutable revision.
    pub fn relationships(
        &self,
        project_id: ProjectId,
        revision_id: ProjectRevisionId,
        element_id: ElementId,
    ) -> Result<Vec<Value>, ApiError> {
        let bound = self
            .service
            .resolve(project_id, RevisionSelector::Revision(revision_id))
            .map_err(service_error)?;
        bound
            .relationships(element_id)
            .map_err(service_error)?
            .iter()
            .map(element)
            .collect()
    }
    /// Project the service's canonical identity diff as normative DataDifference pairs.
    /// Source ranges, validation and association occurrences remain in the richer
    /// service diff; this adapter explicitly reports partial diff coverage.
    pub fn diff(
        &self,
        project_id: ProjectId,
        base: ProjectRevisionId,
        compare: ProjectRevisionId,
    ) -> Result<Vec<dto::DataDifference>, ApiError> {
        let changes = self
            .service
            .diff(project_id, base, compare)
            .map_err(service_error)?;
        let before = self
            .service
            .resolve(project_id, RevisionSelector::Revision(base))
            .map_err(service_error)?;
        let after = self
            .service
            .resolve(project_id, RevisionSelector::Revision(compare))
            .map_err(service_error)?;
        let ids = changes
            .declared
            .added
            .iter()
            .chain(&changes.declared.removed)
            .chain(&changes.declared.changed)
            .chain(&changes.derived.added)
            .chain(&changes.derived.removed)
            .chain(&changes.derived.changed)
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut differences = Vec::new();
        for id in ids {
            if id == before.revision().root() || id == after.revision().root() {
                continue;
            }
            let left = if before.revision().element(id).is_some() {
                Some(data_version(
                    &before.current_element(id).map_err(service_error)?,
                )?)
            } else {
                None
            };
            let right = if after.revision().element(id).is_some() {
                Some(data_version(
                    &after.current_element(id).map_err(service_error)?,
                )?)
            } else {
                None
            };
            differences.push(dto::DataDifference {
                kind: "DataDifference",
                base_data: left,
                compare_data: right,
            });
        }
        Ok(differences)
    }
}

fn service_error(error: agq_modeling_service::ServiceError) -> ApiError {
    match error {
        agq_modeling_service::ServiceError::Repository(error) => error.into(),
        _ => ApiError {
            kind: ErrorKind::Internal,
            message: error.to_string(),
        },
    }
}

fn data_version(value: &ElementDto) -> Result<dto::DataVersion, ApiError> {
    let payload = element(value)?;
    let bytes = serde_json::to_vec(&(value.id, value.metaclass, &value.properties, &payload))
        .map_err(|_| ApiError::bad_request("DataVersion projection failed"))?;
    let namespace = Uuid::from_u128(0x4f3193d7_0473_5278_ad36_6f0edaa8ba75);
    Ok(dto::DataVersion {
        id: Uuid::new_v5(&namespace, &bytes),
        kind: "DataVersion",
        alias: vec![],
        description: None,
        identity: dto::DataIdentity {
            id: identity(value.id),
            kind: "DataIdentity",
            alias: vec![],
            description: None,
            name: None,
        },
        name: None,
        payload: Some(payload),
    })
}

fn element(value: &ElementDto) -> Result<Value, ApiError> {
    let mut object = serde_json::Map::new();
    object.insert("@id".into(), json!(identity(value.id)));
    object.insert("@type".into(), json!(value.metaclass_name));
    object.insert("elementId".into(), json!(value.id.to_string()));
    if let Some(name) = &value.declared_qualified_name {
        object.insert("qualifiedName".into(), json!(name));
    }
    for property in &value.properties {
        object.insert(property.name.clone(), slot(&property.value)?);
    }
    Ok(Value::Object(object))
}

fn slot(value: &agq_modeling_service::SlotValueDto) -> Result<Value, ApiError> {
    use agq_modeling_service::SlotValueDto;
    match value {
        SlotValueDto::Scalar(value) => scalar(value),
        SlotValueDto::Ordered(values) | SlotValueDto::Set(values) | SlotValueDto::Bag(values) => {
            values
                .iter()
                .map(scalar)
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
    }
}

fn scalar(value: &agq_modeling_service::ScalarValueDto) -> Result<Value, ApiError> {
    use agq_modeling_service::ScalarValueDto;
    match value {
        ScalarValueDto::Boolean(value) => Ok(json!(value)),
        ScalarValueDto::String(value) => Ok(json!(value)),
        ScalarValueDto::Integer(value) | ScalarValueDto::Real(value) => value
            .parse::<serde_json::Number>()
            .map(Value::Number)
            .map_err(|_| {
                ApiError::unsupported("numeric lexical form cannot be represented as JSON")
            }),
        ScalarValueDto::Enumeration {
            name: Some(name), ..
        } => Ok(json!(name)),
        ScalarValueDto::Enumeration { name: None, .. } => Err(ApiError::unsupported(
            "enumeration wire name is unavailable",
        )),
        ScalarValueDto::Reference(id) => Ok(json!({"@id": identity(id)})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_exact_shape(name: &str, value: Value) {
        let authority: Value =
            serde_json::from_str(include_str!("../../../standards/artifacts/OpenAPI.json"))
                .unwrap();
        let schema = &authority["components"]["schemas"][name];
        let expected: std::collections::BTreeSet<_> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|key| key.as_str().unwrap())
            .collect();
        let actual = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(expected, actual);
        assert_eq!(value["@type"], schema["properties"]["@type"]["const"]);
    }

    #[test]
    fn repository_dtos_match_exact_pinned_required_fields() {
        use agq_modeling_repository::{Branch, Project, ResourceMetadata};
        let id = Uuid::from_u128(1).to_string().parse().unwrap();
        let branch_id = BranchId::from_uuid(Uuid::from_u128(2));
        let revision = Uuid::from_u128(3).to_string().parse().unwrap();
        let metadata = ResourceMetadata {
            created: "2026-09-24T12:00:00Z".into(),
            name: None,
            description: Some("test".into()),
            alias: vec![],
        };
        let project_value = Project {
            id,
            name: "Platform".into(),
            default_branch: branch_id,
            metadata: metadata.clone(),
        };
        assert_exact_shape(
            "Project",
            serde_json::to_value(project(&project_value)).unwrap(),
        );
        let branch_value = Branch {
            id: branch_id,
            project_id: id,
            name: "experiment".into(),
            head: revision,
            metadata,
        };
        let projected = serde_json::to_value(branch(&branch_value)).unwrap();
        assert_eq!(projected["head"], projected["referencedCommit"]);
        assert_exact_shape("Branch", projected);
    }

    #[test]
    fn numeric_projection_preserves_exact_kernel_carriers() {
        use agq_modeling_service::ScalarValueDto;
        let integer = "12345678901234567890123456789012345678901234567890";
        let real = "123456789012345678901234567890e-1000";
        assert_eq!(
            scalar(&ScalarValueDto::Integer(integer.into()))
                .unwrap()
                .to_string(),
            integer
        );
        assert_eq!(
            scalar(&ScalarValueDto::Real(real.into()))
                .unwrap()
                .to_string(),
            real
        );
    }

    #[test]
    fn data_version_identity_binds_payload_without_binding_unchanged_revision() {
        let mut value = ElementDto {
            id: ElementId::from_u128(1),
            revision_id: Uuid::from_u128(2).to_string().parse().unwrap(),
            metaclass: agq_kernel::MetaclassId::from_u128(3),
            metaclass_name: "Package".into(),
            declared_qualified_name: Some("Before::Nested".into()),
            properties: vec![],
            source: None,
            origin: agq_modeling_service::OriginDto::Declared(
                agq_kernel::provenance::DeclaredOrigin::Generated {
                    generator: agq_kernel::GeneratorId::from_u128(4),
                },
            ),
        };
        let original = data_version(&value).unwrap();
        value.revision_id = Uuid::from_u128(5).to_string().parse().unwrap();
        assert_eq!(data_version(&value).unwrap().id, original.id);
        value.declared_qualified_name = Some("After::Nested".into());
        let changed = data_version(&value).unwrap();
        assert_ne!(changed.id, original.id);
        assert_eq!(changed.identity.id, original.identity.id);
    }

    #[test]
    fn malformed_and_unknown_mutation_fields_are_rejected_before_service() {
        assert!(
            serde_json::from_str::<dto::BranchRequest>(r#"{"head":null,"name":"empty"}"#).is_err()
        );
        assert!(
            serde_json::from_str::<dto::ProjectRequest>(r#"{"name":"p","elements":[]}"#).is_err()
        );
        for body in [
            r#"{"name":"p","@type":null}"#,
            r#"{"name":"p","defaultBranch":null}"#,
        ] {
            assert!(serde_json::from_str::<dto::ProjectRequest>(body).is_err());
        }
        assert!(serde_json::from_str::<dto::BranchRequest>(
            r#"{"name":"b","@type":null,"head":{"@id":"00000000-0000-0000-0000-000000000001"}}"#,
        ).is_err());
        assert!(
            serde_json::from_str::<dto::ProjectRequest>(r#"{"name":"p","description":null}"#)
                .is_ok()
        );
    }
}
