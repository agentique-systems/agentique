use crate::{Result, canonical_json, cross_check, ir::*, sha256, xmi};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path},
};

pub const LOCK_PATH: &str = "standards/normative/kerml-1.0/lock.json";
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

/// Reads only pinned local files. Every input is hash-checked before parsing.
pub fn generate(root: &Path) -> Result<Bundle> {
    let lock: Lock =
        serde_json::from_slice(&fs::read(root.join(LOCK_PATH)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if lock.format != "agentique-normative-metamodel-lock/1"
        || lock.specification != "KerML"
        || lock.version != "1.0"
        || lock.artifacts.len() != 3
    {
        return Err("unsupported normative lock format/baseline/input set".into());
    }
    let find = |filename: &str, role: &str, spec: &str, version: &str| -> Result<&Artifact> {
        let matches: Vec<_> = lock
            .artifacts
            .iter()
            .filter(|a| a.filename == filename)
            .collect();
        if matches.len() != 1 {
            return Err(format!("missing/duplicate pinned input {filename}"));
        }
        let a = matches[0];
        if a.role != role || a.specification != spec || a.version != version {
            return Err(format!("incorrect normative role/version: {filename}"));
        }
        Ok(a)
    };
    let primary = find("KerML.xmi", "primary", "KerML", "1.0")?;
    let json = find("KerML.json", "cross-check", "KerML", "1.0")?;
    let primitives = find("PrimitiveTypes.xmi", "primary-dependency", "UML", "2.5.1")?;
    let primitive_types = xmi::import(
        &read_artifact(root, primitives)?,
        source(
            primitives,
            "http://www.omg.org/spec/PrimitiveTypes/20161101",
        ),
        &Default::default(),
    )?;
    let metamodel = xmi::import(
        &read_artifact(root, primary)?,
        source(primary, &lock.metamodel_uri),
        &xmi::external_types(&primitive_types),
    )?;
    let schema = serde_json::from_str(&read_artifact(root, json)?).map_err(|e| e.to_string())?;
    let cross_check = cross_check::check(&metamodel, &schema, source(json, &lock.metamodel_uri))?;
    Ok(Bundle {
        format: "agentique-metamodel-bundle/1".into(),
        metamodel,
        primitive_types,
        cross_check,
    })
}

pub fn bytes(root: &Path) -> Result<Vec<u8>> {
    canonical_json(&generate(root)?)
}
