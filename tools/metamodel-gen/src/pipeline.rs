use crate::{
    Result,
    baseline::{self, Baseline},
    canonical_json, cross_check, graph,
    ir::*,
    sha256, xmi,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};

pub const LOCK_PATH: &str = baseline::KERML.input_lock;
pub const OUTPUT_PATH: &str = "standards/generated/kerml-1.0/metamodel.json";
#[derive(Debug, Deserialize)]
pub struct Lock {
    pub format: String,
    pub specification: String,
    pub version: String,
    pub metamodel_uri: String,
    pub artifacts: Vec<Artifact>,
}
#[derive(Debug, Deserialize)]
pub struct Artifact {
    pub specification: String,
    pub version: String,
    pub source: String,
    pub filename: String,
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
    pub role: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub metamodel: Metamodel,
    pub primitive_types: Metamodel,
    pub cross_check: cross_check::CrossCheck,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, Metamodel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub representation_differences: Vec<String>,
}
fn read_artifact(root: &Path, artifact: &Artifact) -> Result<String> {
    let path = Path::new(&artifact.path);
    if artifact.path.contains('\\')
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("unsafe artifact path {}", artifact.path));
    }
    if path.file_name().and_then(|s| s.to_str()) != Some(&artifact.filename) {
        return Err("artifact filename/path mismatch".into());
    }
    let bytes = fs::read(root.join(path)).map_err(|e| format!("{}: {e}", artifact.path))?;
    if bytes.len() != artifact.bytes || sha256(&bytes) != artifact.sha256 {
        return Err(format!(
            "pinned artifact integrity mismatch: {} (never overwritten)",
            artifact.path
        ));
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
fn source(artifact: &Artifact, metamodel_uri: &str) -> Source {
    Source {
        specification: artifact.specification.clone(),
        version: artifact.version.clone(),
        metamodel_uri: metamodel_uri.into(),
        artifact_uri: artifact.source.clone(),
        sha256: artifact.sha256.clone(),
    }
}
fn read_lock(root: &Path, profile: Baseline) -> Result<Lock> {
    let lock: Lock = serde_json::from_slice(
        &fs::read(root.join(profile.input_lock)).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let owner = if profile.id == baseline::PRIMITIVES.id {
        baseline::KERML
    } else {
        profile
    };
    if lock.format != "agentique-normative-metamodel-lock/1"
        || lock.specification != owner.specification
        || lock.version != owner.version
        || lock.metamodel_uri != owner.metamodel_uri
    {
        return Err("unsupported normative lock format/baseline/input set".into());
    }
    Ok(lock)
}
fn artifact<'a>(
    lock: &'a Lock,
    profile: Baseline,
    filename: &str,
    role: &str,
) -> Result<&'a Artifact> {
    let found: Vec<_> = lock
        .artifacts
        .iter()
        .filter(|a| a.filename == filename)
        .collect();
    if found.len() != 1 {
        return Err(format!("missing/duplicate pinned input {filename}"));
    }
    let a = found[0];
    let uri = if profile.id == baseline::PRIMITIVES.id {
        "https://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi".to_owned()
    } else {
        format!("{}/{filename}", profile.metamodel_uri)
    };
    if a.role != role
        || a.specification != profile.specification
        || a.version != profile.version
        || a.source != uri
    {
        return Err(format!(
            "incorrect normative role/version/source: {filename}"
        ));
    }
    if filename == profile.primary_xmi && a.sha256 != profile.primary_sha256 {
        return Err(format!("wrong authoritative artifact/version: {filename}"));
    }
    Ok(a)
}
fn import_baseline(
    root: &Path,
    profile: Baseline,
    models: &mut BTreeMap<String, Metamodel>,
) -> Result<()> {
    if models.contains_key(profile.id) {
        return Ok(());
    }
    for dependency in profile.dependencies {
        import_baseline(root, baseline::find(dependency)?, models)?;
    }
    let lock = read_lock(root, profile)?;
    let role = if profile.id == baseline::PRIMITIVES.id {
        "primary-dependency"
    } else {
        "primary"
    };
    let primary = artifact(&lock, profile, profile.primary_xmi, role)?;
    let mut external = xmi::ExternalTypes::new();
    for model in models.values() {
        external.extend(xmi::external_types(model));
    }
    let model = xmi::import_with_root_uri(
        &read_artifact(root, primary)?,
        source(primary, profile.metamodel_uri),
        &external,
        profile.serialized_root_uri,
    )?;
    models.insert(profile.id.into(), model);
    Ok(())
}
/// Reads only pinned local files. No network client or build script is involved.
pub fn generate_profile(root: &Path, profile: Baseline) -> Result<Bundle> {
    let mut models = BTreeMap::new();
    import_baseline(root, profile, &mut models)?;
    let metamodel = models.remove(profile.id).ok_or("missing primary model")?;
    let primitive_types = models
        .remove(baseline::PRIMITIVES.id)
        .ok_or("missing primitive dependency")?;
    let lock = read_lock(root, profile)?;
    let json = artifact(
        &lock,
        profile,
        profile.cross_check_json.ok_or("no cross-check schema")?,
        "cross-check",
    )?;
    let schema = serde_json::from_str(&read_artifact(root, json)?).map_err(|e| e.to_string())?;
    let graph = if models.is_empty() {
        metamodel.clone()
    } else {
        graph::combine(&metamodel, &models)?
    };
    let cross_check = cross_check::check(&graph, &schema, source(json, profile.metamodel_uri))?;
    let mut differences = Vec::new();
    if profile.serialized_root_uri != profile.metamodel_uri {
        differences.push(format!("Published XMI root URI is {}; publication namespace is {}. Original spelling is retained; external references are not rewritten.", profile.serialized_root_uri, profile.metamodel_uri));
    }
    if !models.is_empty() {
        differences.push("JSON projects dependency classes into the primary schema namespace. Descriptor keys retain their authoritative dependency source; schema URIs are not descriptor IDs.".into());
    }
    Ok(Bundle {
        format: "agentique-metamodel-bundle/1".into(),
        metamodel,
        primitive_types,
        cross_check,
        dependencies: models,
        representation_differences: differences,
    })
}
pub fn generate(root: &Path) -> Result<Bundle> {
    generate_profile(root, baseline::KERML)
}
pub fn bytes(root: &Path) -> Result<Vec<u8>> {
    canonical_json(&generate(root)?)
}
