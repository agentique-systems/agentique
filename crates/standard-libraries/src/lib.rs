//! Exact pinned KPAR verification. This loads source inputs, not semantic declarations.
//! No network, archive extraction, source rewriting or implicit library builtins.
#![forbid(unsafe_code)]
use agq_kernel::{
    DocumentId, ElementId, LibraryId, SourceRevisionId,
    provenance::{ByteRange, DeclaredOrigin},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
    path::Path,
    sync::Arc,
};

const MANIFEST: &str = include_str!("../../../standards/normative/sysml-2.0/library-set.json");
const ID_DOMAIN: uuid::Uuid = uuid::Uuid::from_u128(0x2abdcfe090714db6aa6bff4bbd55f188);

#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    format: String,
    id: String,
    root_resource: String,
    artifacts: Vec<Artifact>,
}
#[derive(Clone, Debug, Deserialize)]
struct Artifact {
    specification: String,
    version: String,
    source: String,
    path: String,
    sha256: String,
    bytes: u64,
    kpar: Kpar,
    entries: Vec<Entry>,
}
#[derive(Clone, Debug, Deserialize)]
struct Kpar {
    prefix: String,
    project: serde_json::Value,
    metamodel: String,
}
#[derive(Clone, Debug, Deserialize)]
struct Entry {
    path: String,
    sha256: String,
    bytes: u64,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub usage: Vec<ProjectUsage>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ProjectUsage {
    pub resource: String,
    #[serde(rename = "versionConstraint")]
    pub version_constraint: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ArchiveMetadata {
    pub index: BTreeMap<String, String>,
    pub created: String,
    pub metamodel: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryLanguage {
    KerMl,
    SysMl,
}

/// Exact verified source. Only immutable library bytes may use its identity allocator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryDocument {
    library: LibraryId,
    document: DocumentId,
    revision: SourceRevisionId,
    path: String,
    sha256: String,
    language: LibraryLanguage,
    source: Arc<str>,
}
/// Distinct semantic outputs at the same immutable source range remain distinct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryElementRole {
    Declaration,
    OwningMembership,
    Relationship(u32),
    Annotation,
    Expression,
}
impl LibraryDocument {
    pub fn library(&self) -> LibraryId {
        self.library
    }
    pub fn document(&self) -> DocumentId {
        self.document
    }
    pub fn revision(&self) -> SourceRevisionId {
        self.revision
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn language(&self) -> LibraryLanguage {
        self.language
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn origin(&self) -> DeclaredOrigin {
        DeclaredOrigin::StandardLibrary {
            library: self.library,
        }
    }
    /// Private Agentique immutable-library identity scheme, not an OMG formula.
    /// Byte locators apply only within this exact content-qualified document.
    /// This does not assert that a declaration exists or lower a canonical element.
    pub fn element_id(
        &self,
        range: ByteRange,
        role: LibraryElementRole,
    ) -> Result<ElementId, LibraryError> {
        let start = usize::try_from(range.start()).map_err(|_| LibraryError::InvalidLocator)?;
        let end = usize::try_from(range.end()).map_err(|_| LibraryError::InvalidLocator)?;
        if self.source.get(start..end).is_none() {
            return Err(LibraryError::InvalidLocator);
        }
        let (tag, ordinal) = match role {
            LibraryElementRole::Declaration => ("declaration", 0),
            LibraryElementRole::OwningMembership => ("owning-membership", 0),
            LibraryElementRole::Relationship(n) => ("relationship", n),
            LibraryElementRole::Annotation => ("annotation", 0),
            LibraryElementRole::Expression => ("expression", 0),
        };
        Ok(ElementId::from_u128(identity(&serde_json::json!([
            "agentique-library-element/1",
            self.document.to_string(),
            self.sha256,
            range.start(),
            range.end(),
            tag,
            ordinal
        ]))))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    ProjectMetadata,
    ArchiveMetadata,
    TextualSource,
    RetainedOther,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryEvidence {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub kind: EntryKind,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryDiagnostic {
    MissingIndexTarget {
        library: LibraryId,
        name: String,
        target: String,
    },
    UnindexedDocument {
        library: LibraryId,
        path: String,
    },
    RetainedOtherEntry {
        library: LibraryId,
        path: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedLibrary {
    pub id: LibraryId,
    pub resource: String,
    pub archive_sha256: String,
    pub project: ProjectMetadata,
    pub metadata: ArchiveMetadata,
    pub dependencies: BTreeSet<LibraryId>,
    pub entries: Vec<EntryEvidence>,
    pub documents: Vec<LibraryDocument>,
}
/// Source closure only. Canonical semantic loading is a separate frontend gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedLibrarySet {
    pub content_set_id: String,
    pub libraries: BTreeMap<LibraryId, VerifiedLibrary>,
    pub diagnostics: Vec<LibraryDiagnostic>,
}
impl VerifiedLibrarySet {
    /// Read only the exact paths from the compiled, reviewed manifest.
    pub fn load_from_directory(root: &Path) -> Result<Self, LibraryError> {
        Self::load_with(|path| std::fs::read(root.join(path)).ok())
    }
    /// An injectable offline byte provider. None means a missing pinned artifact.
    /// All usage edges are checked, including back edges in cyclic dependencies.
    pub fn load_with(mut read: impl FnMut(&str) -> Option<Vec<u8>>) -> Result<Self, LibraryError> {
        let manifest: Manifest = serde_json::from_str(MANIFEST)?;
        Self::load_manifest(manifest, &mut read)
    }
    fn load_manifest(
        manifest: Manifest,
        read: &mut impl FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<Self, LibraryError> {
        let expected_set = set_identity(&manifest);
        if manifest.format != "agentique-library-content-set/1" || manifest.id != expected_set {
            return Err(LibraryError::Manifest("content-set identity".into()));
        }
        let mut artifacts = BTreeMap::new();
        for artifact in &manifest.artifacts {
            if artifacts
                .insert(artifact.source.clone(), artifact)
                .is_some()
            {
                return Err(LibraryError::Manifest("duplicate resource".into()));
            }
        }
        let mut pending = vec![manifest.root_resource.clone()];
        let mut visited = BTreeSet::new();
        let mut libraries = BTreeMap::new();
        let mut diagnostics = vec![];
        while let Some(resource) = pending.pop() {
            if !visited.insert(resource.clone()) {
                continue;
            }
            let a = artifacts
                .get(&resource)
                .ok_or_else(|| LibraryError::MissingDependency(resource.clone()))?;
            let raw = read(&a.path).ok_or_else(|| LibraryError::MissingArchive(a.path.clone()))?;
            verify(&a.path, &raw, a.bytes, &a.sha256)?;
            let library = verify_archive(a, &raw, &mut diagnostics)?;
            for usage in &library.project.usage {
                let dependency = artifacts
                    .get(&usage.resource)
                    .ok_or_else(|| LibraryError::MissingDependency(usage.resource.clone()))?;
                let expected: ProjectMetadata =
                    serde_json::from_value(dependency.kpar.project.clone())?;
                if usage.version_constraint != expected.version {
                    return Err(LibraryError::DependencyVersion {
                        resource: usage.resource.clone(),
                        requested: usage.version_constraint.clone(),
                        pinned: expected.version,
                    });
                }
                pending.push(usage.resource.clone());
            }
            libraries.insert(library.id, library);
        }
        if visited.len() != artifacts.len() {
            return Err(LibraryError::Manifest(
                "unreachable artifact in exact closure".into(),
            ));
        }
        let ids: BTreeMap<_, _> = libraries
            .values()
            .map(|l| (l.resource.clone(), l.id))
            .collect();
        for library in libraries.values_mut() {
            library.dependencies = library
                .project
                .usage
                .iter()
                .map(|u| ids[&u.resource])
                .collect();
        }
        Ok(Self {
            content_set_id: manifest.id,
            libraries,
            diagnostics,
        })
    }
    pub fn documents(&self) -> impl Iterator<Item = &LibraryDocument> {
        self.libraries.values().flat_map(|l| &l.documents)
    }
}

fn verify_archive(
    a: &Artifact,
    raw: &[u8],
    diagnostics: &mut Vec<LibraryDiagnostic>,
) -> Result<VerifiedLibrary, LibraryError> {
    let id = LibraryId::from_u128(identity(&serde_json::json!([
        "agentique-pinned-library/1",
        a.specification,
        a.version,
        a.source,
        a.sha256,
        a.kpar.metamodel
    ])));
    let mut zip = zip::ZipArchive::new(Cursor::new(raw))?;
    let mut expected = BTreeMap::new();
    for e in &a.entries {
        if expected.insert(e.path.as_str(), e).is_some() || !safe_path(&e.path) {
            return Err(LibraryError::Manifest("duplicate or unsafe entry".into()));
        }
    }
    if zip.len() != expected.len() {
        return Err(LibraryError::EntryInventory(a.source.clone()));
    }
    let mut contents = BTreeMap::new();
    for n in 0..zip.len() {
        let mut entry = zip.by_index(n)?;
        let name = entry.name().to_owned();
        let pin = expected
            .get(name.as_str())
            .ok_or_else(|| LibraryError::EntryInventory(name.clone()))?;
        if entry.size() != pin.bytes || contents.contains_key(&name) {
            return Err(LibraryError::EntryInventory(name));
        }
        let mut data = vec![];
        (&mut entry)
            .take(pin.bytes.saturating_add(1))
            .read_to_end(&mut data)?;
        verify(&name, &data, pin.bytes, &pin.sha256)?;
        contents.insert(name, data);
    }
    let project_path = format!("{}.project.json", a.kpar.prefix);
    let metadata_path = format!("{}.meta.json", a.kpar.prefix);
    let project_value: serde_json::Value = serde_json::from_slice(
        contents
            .get(&project_path)
            .ok_or_else(|| LibraryError::Metadata(project_path.clone()))?,
    )?;
    if project_value != a.kpar.project {
        return Err(LibraryError::Metadata(project_path));
    }
    let project: ProjectMetadata = serde_json::from_value(project_value)?;
    let metadata: ArchiveMetadata = serde_json::from_slice(
        contents
            .get(&metadata_path)
            .ok_or_else(|| LibraryError::Metadata(metadata_path.clone()))?,
    )?;
    if metadata.metamodel != a.kpar.metamodel {
        return Err(LibraryError::Metadata(metadata_path));
    }
    let mut documents = vec![];
    let mut entries = vec![];
    for (name, data) in &contents {
        let pin = expected[name.as_str()];
        let language = if name.ends_with(".kerml") {
            Some(LibraryLanguage::KerMl)
        } else if name.ends_with(".sysml") {
            Some(LibraryLanguage::SysMl)
        } else {
            None
        };
        let kind = if let Some(language) = language {
            if !name.starts_with(&a.kpar.prefix) {
                return Err(LibraryError::EntryInventory(name.clone()));
            }
            let source = std::str::from_utf8(data).map_err(|_| LibraryError::Utf8(name.clone()))?;
            let document = DocumentId::from_u128(identity(&serde_json::json!([
                "agentique-library-document/1",
                id.to_string(),
                name,
                pin.sha256
            ])));
            let revision = SourceRevisionId::from_u128(identity(&serde_json::json!([
                "agentique-library-source-revision/1",
                document.to_string(),
                pin.sha256
            ])));
            documents.push(LibraryDocument {
                library: id,
                document,
                revision,
                path: name.clone(),
                sha256: pin.sha256.clone(),
                language,
                source: source.into(),
            });
            EntryKind::TextualSource
        } else if name == &project_path {
            EntryKind::ProjectMetadata
        } else if name == &metadata_path {
            EntryKind::ArchiveMetadata
        } else if name.ends_with('/') {
            EntryKind::Directory
        } else {
            diagnostics.push(LibraryDiagnostic::RetainedOtherEntry {
                library: id,
                path: name.clone(),
            });
            EntryKind::RetainedOther
        };
        entries.push(EntryEvidence {
            path: name.clone(),
            sha256: pin.sha256.clone(),
            bytes: pin.bytes,
            kind,
        });
    }
    let mut indexed = BTreeSet::new();
    for (name, target) in &metadata.index {
        if !safe_path(target) {
            return Err(LibraryError::Metadata(target.clone()));
        }
        let path = format!("{}{target}", a.kpar.prefix);
        if !documents.iter().any(|d| d.path == path) {
            diagnostics.push(LibraryDiagnostic::MissingIndexTarget {
                library: id,
                name: name.clone(),
                target: path,
            });
        } else {
            indexed.insert(path);
        }
    }
    for d in &documents {
        if !indexed.contains(&d.path) {
            diagnostics.push(LibraryDiagnostic::UnindexedDocument {
                library: id,
                path: d.path.clone(),
            });
        }
    }
    Ok(VerifiedLibrary {
        id,
        resource: a.source.clone(),
        archive_sha256: a.sha256.clone(),
        project,
        metadata,
        dependencies: BTreeSet::new(),
        entries,
        documents,
    })
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains(['\\', ':', '\0'])
        && path
            .trim_end_matches('/')
            .split('/')
            .all(|s| !matches!(s, "" | "." | ".."))
}
fn identity(value: &serde_json::Value) -> u128 {
    uuid::Uuid::new_v5(
        &ID_DOMAIN,
        &serde_json::to_vec(value).expect("identity tuple"),
    )
    .as_u128()
}
fn hash(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn verify(path: &str, data: &[u8], bytes: u64, sha256: &str) -> Result<(), LibraryError> {
    if data.len() as u64 != bytes || hash(data) != sha256 {
        Err(LibraryError::ContentMismatch(path.into()))
    } else {
        Ok(())
    }
}
fn set_identity(manifest: &Manifest) -> String {
    let mut entries: Vec<_> = manifest
        .artifacts
        .iter()
        .map(|a| [&a.source, &a.sha256])
        .collect();
    entries.sort();
    format!(
        "sha256:{}",
        hash(&serde_json::to_vec(&(manifest.format.as_str(), entries)).expect("identity tuple"))
    )
}

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("invalid pinned manifest: {0}")]
    Manifest(String),
    #[error("missing exact pinned archive: {0}")]
    MissingArchive(String),
    #[error("missing pinned dependency: {0}")]
    MissingDependency(String),
    #[error("dependency {resource} requests {requested}, pinned version is {pinned}")]
    DependencyVersion {
        resource: String,
        requested: String,
        pinned: String,
    },
    #[error("archive/entry size or SHA-256 mismatch: {0}")]
    ContentMismatch(String),
    #[error("archive entry inventory differs: {0}")]
    EntryInventory(String),
    #[error("invalid archive metadata: {0}")]
    Metadata(String),
    #[error("source entry is not UTF-8: {0}")]
    Utf8(String),
    #[error("immutable source locator is outside the verified document or splits UTF-8")]
    InvalidLocator,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest() -> Manifest {
        serde_json::from_str(MANIFEST).unwrap()
    }
    fn bytes(path: &str) -> Option<Vec<u8>> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .ok()
    }
    #[test]
    fn entry_hash_is_checked_independently_of_outer_hash() {
        let m = manifest();
        let mut a = m.artifacts[0].clone();
        let entry = a
            .entries
            .iter_mut()
            .find(|e| e.path.ends_with("Base.kerml"))
            .unwrap();
        entry.sha256 = "0".repeat(64);
        assert!(
            matches!(verify_archive(&a,&bytes(&a.path).unwrap(),&mut vec![]),Err(LibraryError::ContentMismatch(path)) if path.ends_with("Base.kerml"))
        );
    }
    #[test]
    fn source_and_metadata_pins_cannot_disagree() {
        let m = manifest();
        let mut a = m.artifacts[0].clone();
        a.kpar.project["version"] = "9.9.9".into();
        assert!(matches!(
            verify_archive(&a, &bytes(&a.path).unwrap(), &mut vec![]),
            Err(LibraryError::Metadata(_))
        ));
        a = m.artifacts[0].clone();
        a.kpar.metamodel = "wrong-version".into();
        assert!(matches!(
            verify_archive(&a, &bytes(&a.path).unwrap(), &mut vec![]),
            Err(LibraryError::Metadata(_))
        ));
    }
    #[test]
    fn dependency_must_exist_in_exact_content_set() {
        let mut m = manifest();
        m.artifacts
            .retain(|a| !a.source.ends_with("/Data-Type-Library.kpar"));
        m.id = set_identity(&m);
        assert!(matches!(
            VerifiedLibrarySet::load_manifest(m, &mut bytes),
            Err(LibraryError::MissingDependency(_))
        ));
    }
    #[test]
    fn immutable_content_change_changes_private_identity() {
        let m = manifest();
        let a = &m.artifacts[0];
        let key = serde_json::json!([
            "agentique-pinned-library/1",
            a.specification,
            a.version,
            a.source,
            a.sha256,
            a.kpar.metamodel
        ]);
        let mut changed = key.clone();
        changed[4] = "0".repeat(64).into();
        assert_ne!(identity(&key), identity(&changed));
        assert_ne!(
            identity(&key),
            identity(&serde_json::json!(["another-domain", key]))
        );
    }
    #[test]
    fn unsafe_archive_locators_are_rejected_without_extraction() {
        for path in [
            "",
            "/absolute",
            "../escape",
            "a/../b",
            "a\\b",
            "C:/drive",
            "a//b",
            "a/./b",
            "a\0b",
        ] {
            assert!(!safe_path(path), "{path:?}");
        }
        assert!(safe_path("Kernel Semantic Library/"));
        assert!(safe_path("Kernel Semantic Library/Base.kerml"));
    }
}
