//! Runtime distribution of already accepted publications.
//!
//! A bundle manifest identifies transport bytes. Only the language facades and
//! their compiled, checked-in receipts can authenticate semantic authority.
//! Installation never parses or republishes standards, and never downloads as
//! a build or normal startup action.
#![forbid(unsafe_code)]

use agq_kerml_text::{
    library::CanonicalKermlStandardLibraries, sysml::CanonicalSysmlSystemsLibrary,
};
use agq_standard_libraries::VerifiedLibrarySet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const FORMAT: &str = "agq-accepted-publication-bundle/1";
const MANIFEST: &str = "manifest.json";
const KERML: &str = "kerml.cache";
const SYSTEMS: &str = "systems.cache";
const MANIFEST_LIMIT: u64 = 64 * 1024;
const CACHE_LIMIT: u64 = 16 * 1024 * 1024 * 1024;
const KERML_RECEIPT: &str = include_str!("../../../standards/kerml-accepted-publication.json");
const SYSTEMS_RECEIPT: &str = include_str!("../../../standards/sysml-accepted-publication.json");

/// Errors never promote a partially copied or unauthenticated directory.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("runtime I/O: {0}")]
    Io(#[from] io::Error),
    #[error("bundle manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bundle archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("runtime authentication rejected: {0}")]
    Rejected(String),
    #[error(
        "accepted semantic runtime is not installed at {0}; install a local bundle with `cargo run -p agq-studio -- setup --bundle <path>`"
    )]
    Missing(PathBuf),
}

type Result<T> = std::result::Result<T, RuntimeError>;

/// Canonical JSON SHA-256 references identify existing authority without
/// importing receipt contents supplied by the package.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublicationContract {
    pub profile: String,
    pub publication_digest: String,
    pub receipt_path: String,
    pub receipt_json_sha256: String,
    pub binding_manifest_sha256: String,
    pub required_cache_formats: Vec<String>,
}

/// Exact transport identity, independently checked before facade restoration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BundleAsset {
    pub publication: PublicationContract,
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
}

/// Distribution metadata is not a second semantic trust catalogue.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AcceptedPublicationBundle {
    pub format: String,
    pub identity: String,
    pub kerml: BundleAsset,
    pub systems: BundleAsset,
}

/// Genuine observable phases; consumers must not invent percentage estimates.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePhase {
    LocatingPackage,
    Copying,
    AuthenticatingKerml,
    AuthenticatingSysml,
    Installing,
    Ready,
}

/// CLI/config values take precedence over environment compatibility overrides.
#[derive(Clone, Debug, Default)]
pub struct RuntimeConfig {
    pub runtime_dir: Option<PathBuf>,
    pub bundle: Option<PathBuf>,
    pub kerml_cache: Option<PathBuf>,
    pub systems_cache: Option<PathBuf>,
}

/// A located pair is not authenticated until [`load`] succeeds.
#[derive(Clone, Debug, Serialize)]
pub struct PublicationLocation {
    pub kerml_cache: PathBuf,
    pub systems_cache: PathBuf,
    pub bundle_dir: Option<PathBuf>,
    pub origin: String,
}

/// Timings deliberately separate transport digesting from semantic restoration.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RuntimeTimings {
    pub transport_verify_ms: u128,
    pub sources_verify_ms: u128,
    pub kerml_restore_ms: u128,
    pub systems_restore_ms: u128,
}

pub struct AuthenticatedRuntime {
    pub systems: Arc<CanonicalSysmlSystemsLibrary>,
    pub location: PublicationLocation,
    pub timings: RuntimeTimings,
}

#[derive(Serialize)]
pub struct InstalledBundle {
    pub directory: PathBuf,
    pub manifest: AcceptedPublicationBundle,
    pub timings: RuntimeTimings,
    /// Retained authenticated facades let the host open its repository without
    /// restoring the same multi-gigabyte graphs a second time after setup.
    #[serde(skip)]
    pub runtime: AuthenticatedRuntime,
}

fn reject(message: impl Into<String>) -> RuntimeError {
    RuntimeError::Rejected(message.into())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest_json(value: &Value) -> Result<String> {
    Ok(hex(&Sha256::digest(serde_json::to_vec(value)?)))
}

fn receipt_digest(value: &Value) -> Result<String> {
    let bytes: [u8; 32] = serde_json::from_value(value.clone())?;
    Ok(hex(&bytes))
}

/// The two currently supported, separately accepted publications. All authority
/// identities are derived from compiled receipts, never network/package input.
pub fn accepted_contracts() -> Result<(PublicationContract, PublicationContract)> {
    let kerml: Value = serde_json::from_str(KERML_RECEIPT)?;
    let systems: Value = serde_json::from_str(SYSTEMS_RECEIPT)?;
    let kerml_identity = &kerml["complete_overlay"]["identity"];
    let systems_identity = &systems["identity"];
    let profile = |value: &Value| -> Result<String> {
        value["operational_profile"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| reject("compiled receipt has no operational profile"))
    };
    Ok((
        PublicationContract {
            profile: profile(kerml_identity)?,
            publication_digest: receipt_digest(&kerml_identity["semantic_digest"])?,
            receipt_path: "standards/kerml-accepted-publication.json".into(),
            receipt_json_sha256: digest_json(&kerml)?,
            binding_manifest_sha256: receipt_digest(&kerml["binding_manifest_sha256"])?,
            required_cache_formats: vec![
                "zip".into(),
                "agq-kerml-publication-facade/1".into(),
                "agq-kernel-graph-archive/1".into(),
            ],
        },
        PublicationContract {
            profile: profile(systems_identity)?,
            publication_digest: receipt_digest(&systems_identity["publication_digest"])?,
            receipt_path: "standards/sysml-accepted-publication.json".into(),
            receipt_json_sha256: digest_json(&systems)?,
            binding_manifest_sha256: receipt_digest(&systems["binding_manifest_sha256"])?,
            required_cache_formats: vec![
                "zip".into(),
                "agq-sysml-publication-facade/1".into(),
                "agq-kernel-dependent-evidence-archive/1".into(),
            ],
        },
    ))
}

/// Stable store key binds both accepted receipt identities. It is independent of
/// an upload URL, filesystem path, ZIP compression level, and developer worktree.
pub fn accepted_bundle_identity() -> Result<String> {
    let (kerml, systems) = accepted_contracts()?;
    let mut digest = Sha256::new();
    digest.update(FORMAT);
    digest.update(kerml.receipt_json_sha256);
    digest.update(systems.receipt_json_sha256);
    Ok(hex(&digest.finalize()))
}

impl AcceptedPublicationBundle {
    /// Checks the manifest against checked-in authority before reading assets.
    pub fn verify_contract(&self) -> Result<()> {
        let (kerml, systems) = accepted_contracts()?;
        if self.format != FORMAT
            || self.identity != accepted_bundle_identity()?
            || self.kerml.publication != kerml
            || self.systems.publication != systems
        {
            return Err(reject("bundle format or accepted publication identity"));
        }
        for (asset, expected) in [(&self.kerml, KERML), (&self.systems, SYSTEMS)] {
            if asset.file != expected
                || asset.bytes == 0
                || asset.bytes > CACHE_LIMIT
                || asset.sha256.len() != 64
                || !asset
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err(reject("bundle asset name, length or SHA-256"));
            }
        }
        Ok(())
    }
}

/// Normal per-user runtime store. An explicit directory or
/// `AGENTIQUE_RUNTIME_DIR` can isolate development/CI without changing authority.
pub fn runtime_directory(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    if let Some(path) = std::env::var_os("AGENTIQUE_RUNTIME_DIR").filter(|v| !v.is_empty()) {
        return Ok(path.into());
    }
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|value| !value.is_empty())
        .map(|path| PathBuf::from(path).join(".agentique"))
        .ok_or_else(|| reject("no user home; supply --runtime-dir"))
}

pub fn installed_directory(runtime_dir: &Path) -> Result<PathBuf> {
    Ok(runtime_dir
        .join("publications")
        .join(accepted_bundle_identity()?))
}

fn directory_location(directory: &Path, origin: &str) -> Result<PublicationLocation> {
    if !directory.join(MANIFEST).is_file() {
        return Err(RuntimeError::Missing(directory.to_path_buf()));
    }
    Ok(PublicationLocation {
        kerml_cache: directory.join(KERML),
        systems_cache: directory.join(SYSTEMS),
        bundle_dir: Some(directory.to_path_buf()),
        origin: origin.into(),
    })
}

fn cache_pair(
    kerml: Option<PathBuf>,
    systems: Option<PathBuf>,
    origin: &str,
) -> Result<Option<PublicationLocation>> {
    match (kerml, systems) {
        (Some(kerml_cache), Some(systems_cache)) => Ok(Some(PublicationLocation {
            kerml_cache,
            systems_cache,
            bundle_dir: None,
            origin: origin.into(),
        })),
        (None, None) => Ok(None),
        _ => Err(reject(
            "KerML and Systems cache overrides must be supplied together",
        )),
    }
}

/// Deterministic discovery: explicit bundle/cache pair, installed runtime store,
/// then paired legacy environment overrides. Explicit failures do not fall back.
/// Archived packages must be installed first; discovery does not extract them.
pub fn discover(config: &RuntimeConfig) -> Result<PublicationLocation> {
    if let Some(bundle) = &config.bundle {
        if config.kerml_cache.is_some() || config.systems_cache.is_some() {
            return Err(reject("choose a bundle or a cache pair, not both"));
        }
        return directory_location(bundle, "explicit_bundle");
    }
    if let Some(pair) = cache_pair(
        config.kerml_cache.clone(),
        config.systems_cache.clone(),
        "explicit_caches",
    )? {
        return Ok(pair);
    }
    let directory = installed_directory(&runtime_directory(config.runtime_dir.as_deref())?)?;
    if directory.exists() {
        return directory_location(&directory, "installed_bundle");
    }
    if let Some(pair) = cache_pair(
        std::env::var_os("AGENTIQUE_KERML_CACHE").map(PathBuf::from),
        std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").map(PathBuf::from),
        "development_environment",
    )? {
        return Ok(pair);
    }
    Err(RuntimeError::Missing(directory))
}

fn read_manifest(reader: impl Read) -> Result<AcceptedPublicationBundle> {
    let mut bytes = Vec::new();
    reader.take(MANIFEST_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MANIFEST_LIMIT {
        return Err(reject("manifest size limit"));
    }
    let manifest: AcceptedPublicationBundle = serde_json::from_slice(&bytes)?;
    manifest.verify_contract()?;
    Ok(manifest)
}

fn stream_digest(
    mut reader: impl Read,
    mut writer: impl Write,
    limit: u64,
) -> Result<(u64, String)> {
    let mut buffer = [0u8; 128 * 1024];
    let mut digest = Sha256::new();
    let mut count = 0;
    loop {
        let bytes = reader.read(&mut buffer)?;
        if bytes == 0 {
            break;
        }
        count += bytes as u64;
        if count > limit {
            return Err(reject("asset exceeds declared byte length"));
        }
        digest.update(&buffer[..bytes]);
        writer.write_all(&buffer[..bytes])?;
    }
    writer.flush()?;
    Ok((count, hex(&digest.finalize())))
}

fn check_asset(reader: impl Read, writer: impl Write, asset: &BundleAsset) -> Result<()> {
    let (bytes, digest) = stream_digest(reader, writer, asset.bytes)?;
    if bytes != asset.bytes || digest != asset.sha256 {
        return Err(reject(format!(
            "{} byte length or SHA-256 mismatch",
            asset.file
        )));
    }
    Ok(())
}

fn verify_transport(directory: &Path) -> Result<AcceptedPublicationBundle> {
    let manifest = read_manifest(File::open(directory.join(MANIFEST))?)?;
    for asset in [&manifest.kerml, &manifest.systems] {
        check_asset(File::open(directory.join(&asset.file))?, io::sink(), asset)?;
    }
    Ok(manifest)
}

fn restore(
    location: PublicationLocation,
    source_root: &Path,
    mut progress: impl FnMut(RuntimePhase),
    transport_verify_ms: u128,
) -> Result<AuthenticatedRuntime> {
    let start = Instant::now();
    let sources = VerifiedLibrarySet::load_from_directory(source_root)
        .map_err(|error| reject(format!("pinned library source bytes: {error}")))?;
    let sources_verify_ms = start.elapsed().as_millis();
    progress(RuntimePhase::AuthenticatingKerml);
    let start = Instant::now();
    let kerml = Arc::new(
        CanonicalKermlStandardLibraries::restore_cache(
            File::open(&location.kerml_cache)?,
            &sources,
        )
        .map_err(|error| reject(format!("KerML Operational v9: {error}")))?,
    );
    let kerml_restore_ms = start.elapsed().as_millis();
    progress(RuntimePhase::AuthenticatingSysml);
    let start = Instant::now();
    let systems = Arc::new(
        CanonicalSysmlSystemsLibrary::restore_cache(
            File::open(&location.systems_cache)?,
            &sources,
            kerml,
        )
        .map_err(|error| reject(format!("SysML Operational v3: {error}")))?,
    );
    let systems_restore_ms = start.elapsed().as_millis();
    Ok(AuthenticatedRuntime {
        systems,
        location,
        timings: RuntimeTimings {
            transport_verify_ms,
            sources_verify_ms,
            kerml_restore_ms,
            systems_restore_ms,
        },
    })
}

/// Authenticates each cache against its existing facade contract with zero
/// producer replay. Installed labels, manifests and directory names confer no
/// authority, and a damaged installation fails closed on every process launch.
pub fn load(
    config: &RuntimeConfig,
    source_root: &Path,
    mut progress: impl FnMut(RuntimePhase),
) -> Result<AuthenticatedRuntime> {
    progress(RuntimePhase::LocatingPackage);
    let location = discover(config)?;
    let start = Instant::now();
    if let Some(directory) = &location.bundle_dir {
        verify_transport(directory)?;
    }
    let result = restore(
        location,
        source_root,
        &mut progress,
        start.elapsed().as_millis(),
    )?;
    progress(RuntimePhase::Ready);
    Ok(result)
}

fn write_manifest(directory: &Path, manifest: &AcceptedPublicationBundle) -> Result<()> {
    let mut file = File::create(directory.join(MANIFEST))?;
    serde_json::to_writer_pretty(&mut file, manifest)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

fn stage_bundle(source: &Path, directory: &Path) -> Result<AcceptedPublicationBundle> {
    let manifest = if source.is_dir() {
        let manifest = read_manifest(File::open(source.join(MANIFEST))?)?;
        for asset in [&manifest.kerml, &manifest.systems] {
            let mut output = File::create(directory.join(&asset.file))?;
            check_asset(File::open(source.join(&asset.file))?, &mut output, asset)?;
            output.sync_all()?;
        }
        manifest
    } else {
        let mut archive = ZipArchive::new(File::open(source)?)?;
        if archive.len() != 3 {
            return Err(reject(
                "bundle archive must contain exactly manifest.json, kerml.cache, systems.cache",
            ));
        }
        let manifest = read_manifest(archive.by_name(MANIFEST)?)?;
        for asset in [&manifest.kerml, &manifest.systems] {
            let mut output = File::create(directory.join(&asset.file))?;
            check_asset(archive.by_name(&asset.file)?, &mut output, asset)?;
            output.sync_all()?;
        }
        manifest
    };
    write_manifest(directory, &manifest)?;
    Ok(manifest)
}

fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Installs a directory or `.agq-runtime` ZIP from local disk with identical
/// authentication. Files stream into a sibling temporary directory. Publication
/// restoration must succeed for both assets before the atomic rename.
/// An existing installation is reauthenticated and never overwritten.
pub fn install_bundle(
    source: &Path,
    runtime_dir: &Path,
    source_root: &Path,
    mut progress: impl FnMut(RuntimePhase),
) -> Result<InstalledBundle> {
    progress(RuntimePhase::LocatingPackage);
    let destination = installed_directory(runtime_dir)?;
    if destination.exists() {
        let runtime = load(
            &RuntimeConfig {
                bundle: Some(destination.clone()),
                ..Default::default()
            },
            source_root,
            &mut progress,
        )?;
        let manifest = read_manifest(File::open(destination.join(MANIFEST))?)?;
        return Ok(InstalledBundle {
            directory: destination,
            manifest,
            timings: runtime.timings.clone(),
            runtime,
        });
    }
    let parent = destination
        .parent()
        .expect("publication store has a parent");
    fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".install-")
        .tempdir_in(parent)?;
    progress(RuntimePhase::Copying);
    let start = Instant::now();
    let manifest = stage_bundle(source, temporary.path())?;
    let transport_verify_ms = start.elapsed().as_millis();
    let mut runtime = restore(
        directory_location(temporary.path(), "install_candidate")?,
        source_root,
        &mut progress,
        transport_verify_ms,
    )?;
    let timings = runtime.timings.clone();
    progress(RuntimePhase::Installing);
    sync_directory(temporary.path())?;
    fs::rename(temporary.path(), &destination)?;
    sync_directory(parent)?;
    runtime.location = directory_location(&destination, "installed_bundle")?;
    progress(RuntimePhase::Ready);
    Ok(InstalledBundle {
        directory: destination,
        manifest,
        timings,
        runtime,
    })
}

/// Verifies a directory or package without installing it or changing a runtime
/// store. Archive members are copied only by fixed names, never extracted paths.
pub fn verify_bundle(
    source: &Path,
    source_root: &Path,
    mut progress: impl FnMut(RuntimePhase),
) -> Result<RuntimeTimings> {
    if source.is_dir() {
        return Ok(load(
            &RuntimeConfig {
                bundle: Some(source.into()),
                ..Default::default()
            },
            source_root,
            progress,
        )?
        .timings);
    }
    progress(RuntimePhase::LocatingPackage);
    let temporary = tempfile::Builder::new()
        .prefix("agentique-verify-")
        .tempdir()?;
    progress(RuntimePhase::Copying);
    let start = Instant::now();
    stage_bundle(source, temporary.path())?;
    let runtime = restore(
        directory_location(temporary.path(), "verify_candidate")?,
        source_root,
        &mut progress,
        start.elapsed().as_millis(),
    )?;
    progress(RuntimePhase::Ready);
    Ok(runtime.timings)
}

/// Packages existing accepted bytes. This operation authenticates, copies and
/// hashes; it never publishes/regenerates semantics. A `.agq-runtime` or `.zip`
/// output creates one portable ZIP; otherwise the output is a bundle directory.
pub fn pack_bundle(
    kerml: &Path,
    systems: &Path,
    output: &Path,
    source_root: &Path,
    mut progress: impl FnMut(RuntimePhase),
) -> Result<AcceptedPublicationBundle> {
    if output.exists() {
        return Err(reject(
            "package output already exists; choose a new output path",
        ));
    }
    progress(RuntimePhase::LocatingPackage);
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".package-")
        .tempdir_in(parent)?;
    let (kerml_contract, systems_contract) = accepted_contracts()?;
    progress(RuntimePhase::Copying);
    let copy =
        |input: &Path, file: &str, publication: PublicationContract| -> Result<BundleAsset> {
            let mut output = File::create(temporary.path().join(file))?;
            let (bytes, sha256) = stream_digest(File::open(input)?, &mut output, CACHE_LIMIT)?;
            output.sync_all()?;
            Ok(BundleAsset {
                publication,
                file: file.into(),
                bytes,
                sha256,
            })
        };
    let manifest = AcceptedPublicationBundle {
        format: FORMAT.into(),
        identity: accepted_bundle_identity()?,
        kerml: copy(kerml, KERML, kerml_contract)?,
        systems: copy(systems, SYSTEMS, systems_contract)?,
    };
    manifest.verify_contract()?;
    write_manifest(temporary.path(), &manifest)?;
    let authenticated = restore(
        directory_location(temporary.path(), "package_candidate")?,
        source_root,
        &mut progress,
        0,
    )?;
    drop(authenticated);
    if matches!(
        output.extension().and_then(|ext| ext.to_str()),
        Some("agq-runtime" | "zip")
    ) {
        let mut archive_file = tempfile::NamedTempFile::new_in(parent)?;
        {
            let mut archive = ZipWriter::new(archive_file.as_file_mut());
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
                .large_file(true);
            for name in [MANIFEST, KERML, SYSTEMS] {
                archive.start_file(name, options)?;
                io::copy(&mut File::open(temporary.path().join(name))?, &mut archive)?;
            }
            archive.finish()?.sync_all()?;
        }
        archive_file
            .persist_noclobber(output)
            .map_err(|error| RuntimeError::Io(error.error))?;
    } else {
        sync_directory(temporary.path())?;
        fs::rename(temporary.path(), output)?;
    }
    sync_directory(parent)?;
    progress(RuntimePhase::Ready);
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    // Deliberately invalid publication bytes. These test transport rejection and
    // trust boundaries; they must never stand in for accepted runtime evidence.
    fn malformed_bundle(directory: &Path) -> AcceptedPublicationBundle {
        let (kerml, systems) = accepted_contracts().unwrap();
        let asset = |file: &str, publication: PublicationContract| {
            let bytes = b"not an accepted publication";
            fs::write(directory.join(file), bytes).unwrap();
            BundleAsset {
                publication,
                file: file.into(),
                bytes: bytes.len() as u64,
                sha256: hex(&Sha256::digest(bytes)),
            }
        };
        let manifest = AcceptedPublicationBundle {
            format: FORMAT.into(),
            identity: accepted_bundle_identity().unwrap(),
            kerml: asset(KERML, kerml),
            systems: asset(SYSTEMS, systems),
        };
        write_manifest(directory, &manifest).unwrap();
        manifest
    }

    #[test]
    fn receipt_contracts_bind_existing_authority_and_formats() {
        let (kerml, systems) = accepted_contracts().unwrap();
        assert_eq!(kerml.profile, "agentique-kerml-1.0-operational/9");
        assert_eq!(systems.profile, "agentique-sysml-2.0-operational/3");
        assert_eq!(
            kerml.publication_digest,
            "815573353973607bc62a25195ed4461645182027407f8476be521ffc99174f12"
        );
        assert_eq!(
            systems.publication_digest,
            "25aeddb099be16462b553d6debcad97f43bf22e89ce8a9bbb53cf72eb7d193fa"
        );
        for (contract, path) in [
            (kerml, "standards/kerml-standard-bindings.json"),
            (systems, "standards/sysml-standard-bindings.json"),
        ] {
            let bindings: Value =
                serde_json::from_reader(File::open(source_root().join(path)).unwrap()).unwrap();
            assert_eq!(
                contract.binding_manifest_sha256,
                digest_json(&bindings).unwrap()
            );
        }
    }

    #[test]
    fn manifest_cannot_replace_receipts_bindings_profiles_or_paths() {
        let temp = tempfile::tempdir().unwrap();
        let original = malformed_bundle(temp.path());
        original.verify_contract().unwrap();
        for change in 0..7 {
            let mut manifest = original.clone();
            match change {
                0 => manifest.kerml.publication.receipt_json_sha256 = "0".repeat(64),
                1 => manifest.systems.publication.publication_digest = "0".repeat(64),
                2 => manifest.kerml.publication.binding_manifest_sha256 = "0".repeat(64),
                3 => manifest.systems.publication.profile = "accepted-by-package".into(),
                4 => manifest.kerml.file = "../outside".into(),
                5 => manifest.systems.publication.required_cache_formats = vec!["json".into()],
                _ => manifest.identity = "0".repeat(64),
            }
            assert!(manifest.verify_contract().is_err(), "mutation {change}");
        }
    }

    #[test]
    fn manifest_size_and_unknown_fields_are_rejected() {
        assert!(read_manifest(&vec![b' '; MANIFEST_LIMIT as usize + 1][..]).is_err());
        let directory = tempfile::tempdir().unwrap();
        let manifest = malformed_bundle(directory.path());
        let mut value = serde_json::to_value(manifest).unwrap();
        value["accepted"] = Value::Bool(true);
        assert!(read_manifest(&serde_json::to_vec(&value).unwrap()[..]).is_err());
    }

    #[test]
    fn explicit_discovery_precedes_store_and_partial_overrides_fail() {
        let root = tempfile::tempdir().unwrap();
        let store = installed_directory(root.path()).unwrap();
        fs::create_dir_all(&store).unwrap();
        malformed_bundle(&store);
        let bundle = tempfile::tempdir().unwrap();
        malformed_bundle(bundle.path());
        let config = RuntimeConfig {
            runtime_dir: Some(root.path().into()),
            bundle: Some(bundle.path().into()),
            ..Default::default()
        };
        assert_eq!(
            discover(&config).unwrap().bundle_dir.as_deref(),
            Some(bundle.path())
        );
        assert_eq!(
            discover(&RuntimeConfig {
                bundle: None,
                ..config.clone()
            })
            .unwrap()
            .bundle_dir,
            Some(store)
        );
        assert!(
            discover(&RuntimeConfig {
                kerml_cache: Some("missing".into()),
                bundle: None,
                ..config.clone()
            })
            .is_err()
        );
        assert!(
            discover(&RuntimeConfig {
                bundle: Some(root.path().join("missing")),
                ..config
            })
            .is_err()
        );
    }

    #[test]
    fn both_transport_hashes_are_checked_before_semantic_allocation() {
        let source = tempfile::tempdir().unwrap();
        malformed_bundle(source.path());
        fs::write(source.path().join(SYSTEMS), b"changed systems bytes").unwrap();
        let mut phases = Vec::new();
        let error = load(
            &RuntimeConfig {
                bundle: Some(source.path().into()),
                ..Default::default()
            },
            &source_root(),
            |phase| phases.push(phase),
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains(SYSTEMS));
        assert_eq!(phases, vec![RuntimePhase::LocatingPackage]);
    }

    #[test]
    fn self_consistent_transport_hashes_cannot_fabricate_acceptance_or_install() {
        let source = tempfile::tempdir().unwrap();
        let runtime = tempfile::tempdir().unwrap();
        malformed_bundle(source.path());
        let mut phases = Vec::new();
        let error = install_bundle(source.path(), runtime.path(), &source_root(), |phase| {
            phases.push(phase)
        })
        .err()
        .unwrap();
        assert!(
            error.to_string().contains("KerML Operational v9"),
            "{error}"
        );
        assert!(phases.contains(&RuntimePhase::AuthenticatingKerml));
        assert!(!phases.contains(&RuntimePhase::Installing));
        assert!(!installed_directory(runtime.path()).unwrap().exists());
        assert_eq!(
            fs::read_dir(runtime.path().join("publications"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn truncated_overlong_and_corrupt_assets_never_install() {
        for replacement in [
            b"short".as_slice(),
            &[1u8; 100],
            b"not an accepted publicatiom",
        ] {
            let source = tempfile::tempdir().unwrap();
            let runtime = tempfile::tempdir().unwrap();
            malformed_bundle(source.path());
            fs::write(source.path().join(KERML), replacement).unwrap();
            assert!(install_bundle(source.path(), runtime.path(), &source_root(), |_| {}).is_err());
            assert!(!installed_directory(runtime.path()).unwrap().exists());
            assert_eq!(
                fs::read_dir(runtime.path().join("publications"))
                    .unwrap()
                    .count(),
                0
            );
        }
    }

    #[test]
    fn interrupted_stream_is_an_error_even_with_a_valid_prefix() {
        struct Interrupted {
            prefix: io::Cursor<Vec<u8>>,
        }
        impl Read for Interrupted {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                let bytes = self.prefix.read(buffer)?;
                if bytes == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionReset,
                        "copy interrupted",
                    ));
                }
                Ok(bytes)
            }
        }
        let mut output = Vec::new();
        let error = stream_digest(
            Interrupted {
                prefix: io::Cursor::new(b"prefix".to_vec()),
            },
            &mut output,
            CACHE_LIMIT,
        )
        .unwrap_err();
        assert!(matches!(error, RuntimeError::Io(_)));
        assert_eq!(output, b"prefix");
    }

    #[test]
    fn existing_store_is_not_overwritten_by_a_new_bundle() {
        let source = tempfile::tempdir().unwrap();
        let runtime = tempfile::tempdir().unwrap();
        malformed_bundle(source.path());
        let installed = installed_directory(runtime.path()).unwrap();
        fs::create_dir_all(&installed).unwrap();
        fs::write(installed.join(MANIFEST), b"damaged existing manifest").unwrap();
        fs::write(installed.join("retain-me"), b"existing bytes").unwrap();
        assert!(install_bundle(source.path(), runtime.path(), &source_root(), |_| {}).is_err());
        assert_eq!(
            fs::read(installed.join(MANIFEST)).unwrap(),
            b"damaged existing manifest"
        );
        assert_eq!(
            fs::read(installed.join("retain-me")).unwrap(),
            b"existing bytes"
        );
    }

    fn archive(path: &Path, manifest: &AcceptedPublicationBundle, names: &[&str]) {
        let mut archive = ZipWriter::new(File::create(path).unwrap());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        archive.start_file(MANIFEST, options).unwrap();
        serde_json::to_writer(&mut archive, manifest).unwrap();
        for name in names {
            archive.start_file(*name, options).unwrap();
            archive.write_all(b"not an accepted publication").unwrap();
        }
        archive.finish().unwrap();
    }

    #[test]
    fn archive_extra_missing_duplicate_and_truncated_entries_are_rejected() {
        let source = tempfile::tempdir().unwrap();
        let manifest = malformed_bundle(source.path());
        for names in [
            vec![KERML],
            vec![KERML, SYSTEMS, "unexpected"],
            vec![KERML, "fake1.cache"],
        ] {
            let archive_path = source.path().join("invalid.agq-runtime");
            archive(&archive_path, &manifest, &names);
            if names.contains(&"fake1.cache") {
                // Change both local and central-directory names to duplicate an
                // existing entry (ZipWriter itself refuses duplicate names).
                let mut bytes = fs::read(&archive_path).unwrap();
                for start in 0..bytes.len().saturating_sub(10) {
                    if &bytes[start..start + 11] == b"fake1.cache" {
                        bytes[start..start + 11].copy_from_slice(b"kerml.cache");
                    }
                }
                fs::write(&archive_path, bytes).unwrap();
            }
            let runtime = tempfile::tempdir().unwrap();
            assert!(install_bundle(&archive_path, runtime.path(), &source_root(), |_| {}).is_err());
            assert!(!installed_directory(runtime.path()).unwrap().exists());
        }
        let archive_path = source.path().join("truncated.agq-runtime");
        archive(&archive_path, &manifest, &[KERML, SYSTEMS]);
        let bytes = fs::read(&archive_path).unwrap();
        fs::write(&archive_path, &bytes[..bytes.len() / 2]).unwrap();
        assert!(verify_bundle(&archive_path, &source_root(), |_| {}).is_err());
    }

    #[test]
    fn package_cannot_emit_a_manifest_for_unauthenticated_bytes() {
        let source = tempfile::tempdir().unwrap();
        malformed_bundle(source.path());
        let output = source.path().join("must-not-exist.agq-runtime");
        assert!(
            pack_bundle(
                &source.path().join(KERML),
                &source.path().join(SYSTEMS),
                &output,
                &source_root(),
                |_| {}
            )
            .is_err()
        );
        assert!(!output.exists());
        assert!(!fs::read_dir(source.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".package-")
        }));
    }
}
