//! Durable SQLite adapter for source-backed Generation-2 project history.
//!
//! Revision registration, branch compare-and-set and idempotency receipts share
//! one FULL-synchronous transaction. Semantic acceptance remains a workspace and
//! service responsibility; source manifests never become canonical graphs here.
#![forbid(unsafe_code)]

use agq_modeling_repository::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Mutex,
};

mod database;
mod faults;
use faults::{Faults, Point};

/// Thread-safe durable repository. Independent handles and processes may compete
/// for writes; an explicit expected head determines which candidate can commit.
pub struct SqliteRepository {
    connection: Mutex<Connection>,
    repository_id: RepositoryId,
    faults: Faults,
}

fn storage(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn key(value: &impl Serialize) -> Result<String, RepositoryError> {
    Ok(serde_json::to_string(value)?)
}

fn encoded(value: &impl Serialize) -> Result<(Vec<u8>, String), RepositoryError> {
    let bytes = serde_json::to_vec(value)?;
    let digest = ContentDigest::of(&bytes).hex();
    Ok((bytes, digest))
}

fn checked<T: DeserializeOwned>(bytes: Vec<u8>, digest: String) -> Result<T, RepositoryError> {
    if ContentDigest::of(&bytes).hex() != digest {
        return Err(RepositoryError::Integrity(
            "record checksum mismatch".into(),
        ));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

type StoredRecord = (Vec<u8>, String);

impl SqliteRepository {
    /// Open or initialize a distinct Gen2 repository; unsupported schemas are refused.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RepositoryError> {
        let connection =
            database::open(path.as_ref(), &RepositoryId::new().to_string()).map_err(storage)?;
        let (identity, format): (String, String) = connection
            .query_row(
                "SELECT repository_id, format FROM repository_metadata WHERE singleton=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(storage)?;
        if format != "agentique-modeling-sqlite/1" {
            return Err(RepositoryError::Integrity(
                "repository format mismatch".into(),
            ));
        }
        Ok(Self {
            connection: Mutex::new(connection),
            repository_id: identity.parse().map_err(storage)?,
            faults: Faults::default(),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, RepositoryError> {
        self.connection.lock().map_err(storage)
    }

    fn acknowledge(&self, receipt: CommitReceipt) -> Result<CommitReceipt, RepositoryError> {
        if self.faults.check(Point::AfterDurableCommit).is_err() {
            return Err(RepositoryError::OutcomeUnknown(receipt.operation_id));
        }
        Ok(receipt)
    }
}

fn project(connection: &Connection, id: ProjectId) -> Result<Project, RepositoryError> {
    let row: Option<StoredRecord> = connection
        .query_row(
            "SELECT data,digest FROM projects WHERE id=?",
            [key(&id)?],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(storage)?;
    let (data, digest) = row.ok_or_else(|| RepositoryError::NotFound(format!("project {id:?}")))?;
    let value: Project = checked(data, digest)?;
    if value.id != id {
        return Err(RepositoryError::Integrity(
            "project identity mismatch".into(),
        ));
    }
    Ok(value)
}

fn branch(
    connection: &Connection,
    project: ProjectId,
    id: BranchId,
) -> Result<Branch, RepositoryError> {
    let row: Option<(Vec<u8>, String, String, String)> = connection
        .query_row(
            "SELECT data,digest,name,head_id FROM branches WHERE project_id=? AND id=?",
            params![key(&project)?, key(&id)?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(storage)?;
    let (data, digest, name, head) =
        row.ok_or_else(|| RepositoryError::NotFound(format!("branch {id}")))?;
    let value: Branch = checked(data, digest)?;
    if value.id != id
        || value.project_id != project
        || value.name != name
        || key(&value.head)? != head
    {
        return Err(RepositoryError::Integrity(
            "branch indexed data mismatch".into(),
        ));
    }
    Ok(value)
}

fn revision(
    connection: &Connection,
    project: ProjectId,
    id: ProjectRevisionId,
) -> Result<RevisionManifest, RepositoryError> {
    let row: Option<(Vec<u8>, String, Option<String>)> = connection
        .query_row(
            "SELECT manifest,digest,parent_id FROM revisions WHERE project_id=? AND id=?",
            params![key(&project)?, key(&id)?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(storage)?;
    let (data, digest, parent) =
        row.ok_or_else(|| RepositoryError::NotFound(format!("revision {id:?}")))?;
    let manifest: RevisionManifest = checked(data, digest)?;
    manifest.verify()?;
    if manifest.project_id != project
        || manifest.revision_id != id
        || manifest.parent_revision_id.as_ref().map(key).transpose()? != parent
    {
        return Err(RepositoryError::Integrity(
            "revision indexed data mismatch".into(),
        ));
    }
    Ok(manifest)
}

fn blob(connection: &Connection, digest: ContentDigest) -> Result<Vec<u8>, RepositoryError> {
    let bytes: Option<Vec<u8>> = connection
        .query_row(
            "SELECT bytes FROM blobs WHERE digest=?",
            [digest.hex()],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)?;
    let bytes = bytes.ok_or_else(|| RepositoryError::NotFound(format!("blob {digest}")))?;
    if ContentDigest::of(&bytes) != digest {
        return Err(RepositoryError::Integrity(format!(
            "blob checksum mismatch {digest}"
        )));
    }
    Ok(bytes)
}

fn replay(
    connection: &Connection,
    operation: OperationId,
    request_digest: &str,
) -> Result<Option<CommitReceipt>, RepositoryError> {
    let row: Option<(String, Vec<u8>, String, String)> = connection
        .query_row(
            "SELECT request_digest,data,digest,revision_id FROM operation_receipts WHERE operation_id=?",
            [key(&operation)?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(storage)?;
    row.map(|(stored_request, data, digest, revision)| {
        if stored_request != request_digest {
            return Err(RepositoryError::OperationCollision(operation));
        }
        let mut receipt: CommitReceipt = checked(data, digest)?;
        if receipt.operation_id != operation || key(&receipt.revision_id)? != revision {
            return Err(RepositoryError::Integrity(
                "operation receipt identity mismatch".into(),
            ));
        }
        receipt.replayed = true;
        Ok(receipt)
    })
    .transpose()
}

fn store_receipt(
    connection: &Connection,
    fingerprint: &str,
    receipt: &CommitReceipt,
) -> Result<(), RepositoryError> {
    let (data, digest) = encoded(receipt)?;
    connection.execute(
        "INSERT INTO operation_receipts(operation_id,request_digest,revision_id,data,digest) VALUES(?,?,?,?,?)",
        params![key(&receipt.operation_id)?, fingerprint, key(&receipt.revision_id)?, data, digest],
    ).map_err(storage)?;
    Ok(())
}

fn store_branch(connection: &Connection, branch: &Branch) -> Result<(), RepositoryError> {
    if branch.name.trim().is_empty() {
        return Err(RepositoryError::Integrity("empty branch name".into()));
    }
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM branches WHERE id=? OR (project_id=? AND name=?))",
            params![key(&branch.id)?, key(&branch.project_id)?, branch.name],
            |row| row.get(0),
        )
        .map_err(storage)?;
    if exists {
        return Err(RepositoryError::AlreadyExists(format!(
            "branch {}",
            branch.name
        )));
    }
    let (data, digest) = encoded(branch)?;
    connection
        .execute(
            "INSERT INTO branches(id,project_id,name,head_id,data,digest) VALUES(?,?,?,?,?,?)",
            params![
                key(&branch.id)?,
                key(&branch.project_id)?,
                branch.name,
                key(&branch.head)?,
                data,
                digest
            ],
        )
        .map_err(storage)?;
    Ok(())
}

fn store_candidate(
    connection: &Connection,
    candidate: &CandidateRevision,
    faults: &Faults,
) -> Result<(), RepositoryError> {
    let manifest = &candidate.manifest;
    for (digest, bytes) in &candidate.blobs {
        let previous: Option<Vec<u8>> = connection
            .query_row(
                "SELECT bytes FROM blobs WHERE digest=?",
                [digest.hex()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        match previous {
            Some(previous) if previous != *bytes => {
                return Err(RepositoryError::Integrity(format!(
                    "existing blob checksum/collision {digest}"
                )));
            }
            Some(_) => {}
            None => {
                connection
                    .execute(
                        "INSERT INTO blobs(digest,bytes) VALUES(?,?)",
                        params![digest.hex(), bytes],
                    )
                    .map_err(storage)?;
            }
        }
    }
    faults.check(Point::AfterSourceBlobWrite).map_err(storage)?;
    let (data, digest) = encoded(manifest)?;
    let previous: Option<StoredRecord> = connection
        .query_row(
            "SELECT manifest,digest FROM revisions WHERE id=?",
            [key(&manifest.revision_id)?],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(storage)?;
    if let Some((previous, previous_digest)) = previous {
        if previous != data || previous_digest != digest {
            return Err(RepositoryError::Integrity(
                "revision identity collision".into(),
            ));
        }
        return Ok(());
    }
    connection
        .execute(
            "INSERT INTO revisions(id,project_id,parent_id,manifest,digest) VALUES(?,?,?,?,?)",
            params![
                key(&manifest.revision_id)?,
                key(&manifest.project_id)?,
                manifest.parent_revision_id.as_ref().map(key).transpose()?,
                data,
                digest
            ],
        )
        .map_err(storage)?;
    faults
        .check(Point::AfterRevisionManifestWrite)
        .map_err(storage)?;
    for document in &manifest.documents {
        let source = key(&document.source_revision_id)?;
        let document_id = key(&document.document_id)?;
        let language = key(&document.language)?;
        let digest = document.content_digest.hex();
        let previous: Option<(String, String, String)> = connection.query_row(
            "SELECT document_id,language,blob_digest FROM source_revisions WHERE source_revision_id=?", [&source],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional().map_err(storage)?;
        if let Some(previous) = previous {
            if previous != (document_id.clone(), language.clone(), digest.clone()) {
                return Err(RepositoryError::Integrity(
                    "source revision identity collision".into(),
                ));
            }
        } else {
            connection.execute(
                "INSERT INTO source_revisions(source_revision_id,document_id,language,blob_digest) VALUES(?,?,?,?)",
                params![source, document_id, language, digest],
            ).map_err(storage)?;
        }
        connection.execute(
            "INSERT INTO revision_documents(revision_id,document_id,path,source_revision_id,language,blob_digest) VALUES(?,?,?,?,?,?)",
            params![key(&manifest.revision_id)?, document_id, document.path, source, language, digest],
        ).map_err(storage)?;
    }
    if let Some(cache) = &manifest.semantic_cache {
        connection
            .execute(
                "INSERT INTO semantic_caches(revision_id,blob_digest) VALUES(?,?)",
                params![key(&manifest.revision_id)?, cache.content_digest.hex()],
            )
            .map_err(storage)?;
    }
    Ok(())
}

impl ModelingRepository for SqliteRepository {
    fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    fn create_project(&self, request: &CreateProject) -> Result<CommitReceipt, RepositoryError> {
        request.initial.verify()?;
        if request.project.id != request.initial.manifest.project_id
            || request.project.id != request.branch.project_id
            || request.project.default_branch != request.branch.id
            || request.branch.head != request.initial.manifest.revision_id
            || request.initial.manifest.parent_revision_id.is_some()
            || request.project.name.trim().is_empty()
        {
            return Err(RepositoryError::Integrity(
                "inconsistent initial project binding".into(),
            ));
        }
        let (_, fingerprint) = encoded(&(
            "create_project/1",
            &request.project,
            &request.branch,
            &request.initial.manifest,
            request.initial.blobs.keys().collect::<Vec<_>>(),
        ))?;
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if let Some(receipt) = replay(&transaction, request.operation_id, &fingerprint)? {
            return Ok(receipt);
        }
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?)",
                [key(&request.project.id)?],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if exists {
            return Err(RepositoryError::AlreadyExists(format!(
                "project {:?}",
                request.project.id
            )));
        }
        let (data, digest) = encoded(&request.project)?;
        transaction
            .execute(
                "INSERT INTO projects(id,data,digest) VALUES(?,?,?)",
                params![key(&request.project.id)?, data, digest],
            )
            .map_err(storage)?;
        store_candidate(&transaction, &request.initial, &self.faults)?;
        self.faults
            .check(Point::BeforeHeadUpdate)
            .map_err(storage)?;
        store_branch(&transaction, &request.branch)?;
        self.faults
            .check(Point::AfterHeadUpdateStatement)
            .map_err(storage)?;
        let receipt = CommitReceipt {
            operation_id: request.operation_id,
            branch_id: request.branch.id,
            revision_id: request.initial.manifest.revision_id,
            replayed: false,
        };
        store_receipt(&transaction, &fingerprint, &receipt)?;
        self.faults
            .check(Point::BeforeTransactionCommit)
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;
        self.acknowledge(receipt)
    }

    fn get_project(&self, id: ProjectId) -> Result<Project, RepositoryError> {
        project(&*self.lock()?, id)
    }

    fn list_projects(&self) -> Result<Vec<Project>, RepositoryError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare("SELECT id FROM projects ORDER BY id")
            .map_err(storage)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage)?;
        ids.map(|id| project(&connection, serde_json::from_str(&id.map_err(storage)?)?))
            .collect()
    }

    fn create_branch(&self, value: &Branch) -> Result<(), RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        project(&transaction, value.project_id)?;
        revision(&transaction, value.project_id, value.head)?;
        store_branch(&transaction, value)?;
        transaction.commit().map_err(storage)
    }

    fn get_branch(&self, project: ProjectId, id: BranchId) -> Result<Branch, RepositoryError> {
        branch(&*self.lock()?, project, id)
    }

    fn list_branches(&self, id: ProjectId) -> Result<Vec<Branch>, RepositoryError> {
        let connection = self.lock()?;
        project(&connection, id)?;
        let mut statement = connection
            .prepare("SELECT id FROM branches WHERE project_id=? ORDER BY id")
            .map_err(storage)?;
        let ids = statement
            .query_map([key(&id)?], |row| row.get::<_, String>(0))
            .map_err(storage)?;
        ids.map(|branch_id| {
            branch(
                &connection,
                id,
                serde_json::from_str(&branch_id.map_err(storage)?)?,
            )
        })
        .collect()
    }

    fn delete_branch(&self, id: ProjectId, branch_id: BranchId) -> Result<(), RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if project(&transaction, id)?.default_branch == branch_id {
            return Err(RepositoryError::Integrity(
                "cannot delete the default branch".into(),
            ));
        }
        branch(&transaction, id, branch_id)?;
        transaction
            .execute(
                "DELETE FROM branches WHERE project_id=? AND id=?",
                params![key(&id)?, key(&branch_id)?],
            )
            .map_err(storage)?;
        transaction.commit().map_err(storage)
    }

    fn load_revision(
        &self,
        project: ProjectId,
        id: ProjectRevisionId,
    ) -> Result<RevisionManifest, RepositoryError> {
        revision(&*self.lock()?, project, id)
    }

    fn list_revisions(&self, project: ProjectId) -> Result<Vec<RevisionManifest>, RepositoryError> {
        let connection = self.lock()?;
        self::project(&connection, project)?;
        let mut statement = connection
            .prepare("SELECT id FROM revisions WHERE project_id=? ORDER BY id")
            .map_err(storage)?;
        let ids = statement
            .query_map([key(&project)?], |row| row.get::<_, String>(0))
            .map_err(storage)?;
        ids.map(|id| {
            revision(
                &connection,
                project,
                serde_json::from_str(&id.map_err(storage)?)?,
            )
        })
        .collect()
    }

    fn read_blob(&self, digest: ContentDigest) -> Result<Vec<u8>, RepositoryError> {
        blob(&*self.lock()?, digest)
    }

    fn list_revision_history(
        &self,
        project: ProjectId,
        start: ProjectRevisionId,
    ) -> Result<Vec<RevisionManifest>, RepositoryError> {
        let connection = self.lock()?;
        let mut history = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = Some(start);
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(RepositoryError::Integrity("parent cycle".into()));
            }
            let manifest = revision(&connection, project, id)?;
            current = manifest.parent_revision_id;
            history.push(manifest);
        }
        Ok(history)
    }

    fn commit_revision(&self, request: &CommitRevision) -> Result<CommitReceipt, RepositoryError> {
        request.candidate.verify()?;
        if request.candidate.manifest.project_id != request.project_id
            || request.candidate.manifest.parent_revision_id != Some(request.expected_head)
        {
            return Err(RepositoryError::Integrity(
                "candidate parent/project does not match request".into(),
            ));
        }
        let (_, fingerprint) = encoded(&(
            "commit_revision/1",
            request.project_id,
            request.branch_id,
            request.expected_head,
            &request.candidate.manifest,
            request.candidate.blobs.keys().collect::<Vec<_>>(),
        ))?;
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if let Some(receipt) = replay(&transaction, request.operation_id, &fingerprint)? {
            return Ok(receipt);
        }
        let mut current = branch(&transaction, request.project_id, request.branch_id)?;
        if current.head != request.expected_head {
            return Err(RepositoryError::Conflict {
                expected: request.expected_head,
                actual: current.head,
            });
        }
        revision(&transaction, request.project_id, request.expected_head)?;
        store_candidate(&transaction, &request.candidate, &self.faults)?;
        self.faults
            .check(Point::BeforeHeadUpdate)
            .map_err(storage)?;
        current.head = request.candidate.manifest.revision_id;
        let (data, digest) = encoded(&current)?;
        let changed = transaction.execute(
            "UPDATE branches SET head_id=?,data=?,digest=? WHERE project_id=? AND id=? AND head_id=?",
            params![key(&current.head)?, data, digest, key(&request.project_id)?, key(&request.branch_id)?, key(&request.expected_head)?],
        ).map_err(storage)?;
        if changed != 1 {
            return Err(RepositoryError::Integrity(
                "CAS transaction lost branch".into(),
            ));
        }
        self.faults
            .check(Point::AfterHeadUpdateStatement)
            .map_err(storage)?;
        let receipt = CommitReceipt {
            operation_id: request.operation_id,
            branch_id: request.branch_id,
            revision_id: current.head,
            replayed: false,
        };
        store_receipt(&transaction, &fingerprint, &receipt)?;
        self.faults
            .check(Point::BeforeTransactionCommit)
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;
        self.acknowledge(receipt)
    }

    fn check_integrity(&self) -> Result<IntegrityReport, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction().map_err(storage)?;
        let report = integrity(&transaction)?;
        transaction.commit().map_err(storage)?;
        Ok(report)
    }
}

fn integrity(connection: &Connection) -> Result<IntegrityReport, RepositoryError> {
    let mut report = IntegrityReport::default();
    let mut statement = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(storage)?;
    let violations = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
        })
        .map_err(storage)?;
    for violation in violations {
        report.errors.push(format!(
            "foreign key violation: {:?}",
            violation.map_err(storage)?
        ));
    }

    let mut statement = connection
        .prepare("SELECT project_id,id FROM revisions ORDER BY id")
        .map_err(storage)?;
    let identities = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage)?;
    let mut mandatory = BTreeSet::new();
    let mut caches = Vec::new();
    let mut parents = BTreeMap::new();
    for identity in identities {
        report.revisions_checked += 1;
        let (project, id) = identity.map_err(storage)?;
        let result = (|| -> Result<(), RepositoryError> {
            let project_id = serde_json::from_str(&project)?;
            let revision_id = serde_json::from_str(&id)?;
            let manifest = revision(connection, project_id, revision_id)?;
            parents.insert(manifest.revision_id, manifest.parent_revision_id);
            if let Some(parent) = manifest.parent_revision_id {
                revision(connection, project_id, parent)?;
            }
            for digest in manifest.required_blobs() {
                mandatory.insert(digest.hex());
                blob(connection, digest)?;
            }
            let mut documents = connection.prepare("SELECT document_id,path,source_revision_id,language,blob_digest FROM revision_documents WHERE revision_id=? ORDER BY document_id").map_err(storage)?;
            let rows = documents
                .query_map([&id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(storage)?;
            let stored = rows.collect::<Result<BTreeSet<_>, _>>().map_err(storage)?;
            let expected = manifest
                .documents
                .iter()
                .map(|document| {
                    Ok((
                        key(&document.document_id)?,
                        document.path.clone(),
                        key(&document.source_revision_id)?,
                        key(&document.language)?,
                        document.content_digest.hex(),
                    ))
                })
                .collect::<Result<BTreeSet<_>, RepositoryError>>()?;
            if stored != expected {
                return Err(RepositoryError::Integrity(
                    "document rows differ from manifest".into(),
                ));
            }
            for document in &manifest.documents {
                let binding: Option<(String,String,String)> = connection.query_row("SELECT document_id,language,blob_digest FROM source_revisions WHERE source_revision_id=?", [key(&document.source_revision_id)?], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(storage)?;
                if binding
                    != Some((
                        key(&document.document_id)?,
                        key(&document.language)?,
                        document.content_digest.hex(),
                    ))
                {
                    return Err(RepositoryError::Integrity(
                        "source revision binding mismatch".into(),
                    ));
                }
            }
            let cache_row: Option<String> = connection
                .query_row(
                    "SELECT blob_digest FROM semantic_caches WHERE revision_id=?",
                    [&id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            if cache_row
                != manifest
                    .semantic_cache
                    .as_ref()
                    .map(|cache| cache.content_digest.hex())
            {
                report.discardable_caches.push(manifest.revision_id);
            }
            if let Some(cache) = &manifest.semantic_cache {
                if cache.source_binding != manifest.source_binding()? {
                    report.discardable_caches.push(manifest.revision_id);
                }
                caches.push((manifest.revision_id, cache.content_digest));
            }
            Ok(())
        })();
        if let Err(error) = result {
            report.errors.push(format!("revision {id}: {error}"));
        }
    }
    for (revision, digest) in &caches {
        if blob(connection, *digest).is_err() {
            report.discardable_caches.push(*revision);
        }
    }
    let mut walked = BTreeSet::new();
    for id in parents.keys() {
        let mut current = Some(*id);
        let mut path = BTreeSet::new();
        while let Some(id) = current {
            if walked.contains(&id) {
                break;
            }
            if !path.insert(id) {
                report
                    .errors
                    .push(format!("revision parent cycle at {id:?}"));
                break;
            }
            current = parents.get(&id).copied().flatten();
        }
        walked.extend(path);
    }
    let mut statement = connection
        .prepare("SELECT digest,bytes FROM blobs ORDER BY digest")
        .map_err(storage)?;
    let blobs = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(storage)?;
    for entry in blobs {
        report.blobs_checked += 1;
        let (digest, bytes) = entry.map_err(storage)?;
        if ContentDigest::of(&bytes).hex() != digest
            && (mandatory.contains(&digest)
                || !caches.iter().any(|(_, cache)| cache.hex() == digest))
        {
            report
                .errors
                .push(format!("blob checksum mismatch {digest}"));
        }
    }
    let mut statement = connection
        .prepare("SELECT project_id,id FROM branches ORDER BY id")
        .map_err(storage)?;
    let branches = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage)?;
    for entry in branches {
        let (project, id) = entry.map_err(storage)?;
        let result = (|| -> Result<(), RepositoryError> {
            let project = serde_json::from_str(&project)?;
            let value = branch(connection, project, serde_json::from_str(&id)?)?;
            revision(connection, project, value.head)?;
            Ok(())
        })();
        if let Err(error) = result {
            report.errors.push(format!("branch {id}: {error}"));
        }
    }
    let mut statement = connection
        .prepare("SELECT id FROM projects ORDER BY id")
        .map_err(storage)?;
    for id in statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(storage)?
    {
        let id = id.map_err(storage)?;
        let result = (|| -> Result<(), RepositoryError> {
            let value = project(connection, serde_json::from_str(&id)?)?;
            branch(connection, value.id, value.default_branch)?;
            Ok(())
        })();
        if let Err(error) = result {
            report.errors.push(format!("project {id}: {error}"));
        }
    }
    let mut statement = connection.prepare("SELECT operation_id,request_digest,revision_id,data,digest FROM operation_receipts ORDER BY operation_id").map_err(storage)?;
    let receipts = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(storage)?;
    for row in receipts {
        let (id, request_digest, revision, data, digest) = row.map_err(storage)?;
        let result = (|| -> Result<(), RepositoryError> {
            let receipt: CommitReceipt = checked(data, digest)?;
            if key(&receipt.operation_id)? != id
                || key(&receipt.revision_id)? != revision
                || request_digest.len() != 64
                || !request_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(RepositoryError::Integrity(
                    "operation receipt binding mismatch".into(),
                ));
            }
            Ok(())
        })();
        if let Err(error) = result {
            report.errors.push(format!("operation {id}: {error}"));
        }
    }
    report.discardable_caches.sort();
    report.discardable_caches.dedup();
    Ok(report)
}

#[cfg(test)]
mod tests;
