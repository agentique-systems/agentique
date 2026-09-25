//! First-run source import is resumable without rewriting an authored repository.
use agq_kerml_text::{ProjectChange, SourceLanguage};
use agq_modeling_repository::{ContentDigest, RevisionManifest};
use agq_modeling_repository::{OperationId, ProjectId, ProjectRevisionId};
use agq_modeling_service::ServiceError;
use agq_modeling_service::{ApplyDocumentChanges, ModelingService};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const DESCRIPTION: &str = "Agentique's own durable system architecture";

#[derive(Serialize, Deserialize)]
struct SeedPlan {
    initial: ProjectRevisionId,
    baseline: Option<ProjectRevisionId>,
    documents: BTreeMap<String, String>,
    fabric: String,
}
fn invalid(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Invalid(error.to_string())
}

fn untouched_initial_seed(
    empty: bool,
    has_parent: bool,
    revisions: usize,
    branches: usize,
) -> bool {
    empty && !has_parent && revisions == 1 && branches == 1
}
fn save(
    db: &rusqlite::Connection,
    project: ProjectId,
    plan: &SeedPlan,
) -> Result<(), ServiceError> {
    db.execute("INSERT INTO studio_seed(project,plan) VALUES(?1,?2) ON CONFLICT(project) DO UPDATE SET plan=excluded.plan", rusqlite::params![project.to_string(), serde_json::to_string(plan).map_err(invalid)?]).map_err(invalid)?;
    Ok(())
}
fn matches(manifest: &RevisionManifest, plan: &SeedPlan, with_fabric: bool) -> bool {
    let mut expected: BTreeMap<_, _> = plan
        .documents
        .iter()
        .map(|(name, source)| (name.as_str(), ContentDigest::of(source.as_bytes())))
        .collect();
    if with_fabric {
        expected.insert(
            "AgentFabric.sysml",
            ContentDigest::of(plan.fabric.as_bytes()),
        );
    }
    manifest.documents.len() == expected.len()
        && manifest.documents.iter().all(|doc| {
            doc.language == SourceLanguage::SysMl
                && expected.get(doc.path.as_str()) == Some(&doc.content_digest)
        })
}

/// Resumable first-run import shared by native and web hosts. Existing authored projects are preserved.
pub fn seed_agentique(
    service: &ModelingService,
    root: &std::path::Path,
    db: &rusqlite::Connection,
    progress: &mut impl FnMut(&str),
) -> Result<(), ServiceError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS studio_seed (project TEXT PRIMARY KEY NOT NULL, plan TEXT NOT NULL);").map_err(invalid)?;
    let project = match service
        .repository()
        .list_projects()?
        .into_iter()
        .find(|project| project.name == "Agentique")
    {
        Some(project) => project,
        None => service.create_project("Agentique", Some(DESCRIPTION.into()))?,
    };
    let current = service
        .repository()
        .get_branch(project.id, project.default_branch)?
        .head;
    let current_manifest = service.repository().load_revision(project.id, current)?;
    let stored: Option<String> = db
        .query_row(
            "SELECT plan FROM studio_seed WHERE project=?1",
            [project.id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(invalid)?;
    let mut plan: SeedPlan = if let Some(stored) = stored {
        serde_json::from_str(&stored).map_err(invalid)?
    } else {
        // No pending bootstrap journal means an existing authored project is final.
        // An interrupted create_project is recognizable by this exact empty seed.
        if !untouched_initial_seed(
            current_manifest.documents.is_empty(),
            current_manifest.parent_revision_id.is_some(),
            service.repository().list_revisions(project.id)?.len(),
            service.repository().list_branches(project.id)?.len(),
        ) || project.metadata.description.as_deref() != Some(DESCRIPTION)
        {
            return Ok(());
        }
        let names = [
            "Contracts.sysml",
            "LanguageEngine.sysml",
            "ModelingPlatform.sysml",
            "ExecutionRuntime.sysml",
            "Agentique.sysml",
        ];
        let documents = names
            .into_iter()
            .map(|name| {
                Ok((
                    name.to_owned(),
                    std::fs::read_to_string(root.join("models/agentique").join(name))
                        .map_err(invalid)?,
                ))
            })
            .collect::<Result<_, ServiceError>>()?;
        let plan = SeedPlan {
            initial: current,
            baseline: None,
            documents,
            fabric: std::fs::read_to_string(root.join("models/agentique/AgentFabric.sysml"))
                .map_err(invalid)?,
        };
        // Exact source intent is durable before the first semantic commit. It is only
        // bootstrap bookkeeping, never a source of semantic validation authority.
        save(db, project.id, &plan)?;
        plan
    };
    let baseline = if let Some(baseline) = plan.baseline {
        baseline
    } else {
        let baseline = if current == plan.initial {
            progress("validating_architecture");
            service
                .apply_document_changes(ApplyDocumentChanges {
                    operation_id: OperationId::new(),
                    project: project.id,
                    branch: project.default_branch,
                    expected_head: current,
                    changes: plan
                        .documents
                        .iter()
                        .map(|(path, source)| ProjectChange::Add {
                            path: path.clone(),
                            source: source.clone(),
                            language: SourceLanguage::SysMl,
                        })
                        .collect(),
                    validate: true,
                })?
                .revision_id
        } else if current_manifest.parent_revision_id == Some(plan.initial)
            && matches(&current_manifest, &plan, false)
        {
            // The semantic transaction committed, but the process stopped before
            // recording its acknowledgement in the independent bootstrap journal.
            current
        } else {
            return Err(invalid(
                "The Agentique branch changed during bootstrap; existing authored revisions have been preserved.",
            ));
        };
        plan.baseline = Some(baseline);
        save(db, project.id, &plan)?;
        baseline
    };
    if let Some(branch) = service
        .repository()
        .list_branches(project.id)?
        .into_iter()
        .find(|branch| branch.name == "architecture-baseline")
    {
        if branch.head != baseline {
            return Err(invalid(
                "Existing architecture-baseline differs from the pending bootstrap; it has been preserved.",
            ));
        }
    } else {
        service.create_branch(project.id, "architecture-baseline", baseline)?;
    }
    let current = service
        .repository()
        .get_branch(project.id, project.default_branch)?
        .head;
    if current == baseline {
        progress("validating_agent_fabric");
        service.apply_document_changes(ApplyDocumentChanges {
            operation_id: OperationId::new(),
            project: project.id,
            branch: project.default_branch,
            expected_head: baseline,
            changes: vec![ProjectChange::Add {
                path: "AgentFabric.sysml".into(),
                language: SourceLanguage::SysMl,
                source: plan.fabric.clone(),
            }],
            validate: true,
        })?;
    } else {
        let manifest = service.repository().load_revision(project.id, current)?;
        if manifest.parent_revision_id != Some(baseline) || !matches(&manifest, &plan, true) {
            return Err(invalid(
                "The Agentique branch changed during bootstrap; existing authored revisions have been preserved.",
            ));
        }
    }
    db.execute(
        "DELETE FROM studio_seed WHERE project=?1",
        [project.id.to_string()],
    )
    .map_err(invalid)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::untouched_initial_seed;

    #[test]
    fn bootstrap_never_adopts_an_authored_empty_revision_or_retained_history() {
        assert!(untouched_initial_seed(true, false, 1, 1));
        assert!(!untouched_initial_seed(true, true, 2, 1));
        assert!(!untouched_initial_seed(true, false, 3, 1));
        assert!(!untouched_initial_seed(true, false, 1, 2));
        assert!(!untouched_initial_seed(false, false, 1, 1));
    }
}
