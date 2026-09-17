//! Offline abstract-syntax extraction, not a language implementation or runtime store.
#![forbid(unsafe_code)]

pub mod baseline;
pub mod cross_check;
pub mod descriptors;
pub mod graph;
pub mod ir;
pub mod pipeline;
mod profile;
pub mod typed_views;
pub mod xmi;

pub type Result<T> = std::result::Result<T, String>;

pub fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// Agentique's deterministic JSON format: struct field order, sorted maps, LF,
/// no clock, machine path, hash-map traversal order or network-derived values.
pub fn canonical_json(value: &impl serde::Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}
