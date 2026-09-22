//! Restore acceptance only from the repository's separately pinned receipt.
//!
//! The kernel archive carries structure and evidence, never language acceptance.
//! The receipt pins that exact archive independently of its semantic digest.
use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

const RECEIPT: &str = include_str!("../../../standards/kerml-accepted-publication.json");
const BINDINGS: &str = include_str!("../../../standards/kerml-standard-bindings.json");

#[cfg(test)]
#[path = "publication_restore_tests.rs"]
mod tests;

/// A restoration failure cannot produce a complete publication overlay.
#[derive(Debug)]
pub enum PublicationRestoreError {
    NotAccepted,
    Mismatch(&'static str),
    Json(serde_json::Error),
    Archive(agq_kernel::archive::ArchiveError),
    Publication(PublicationOverlayError),
}
impl std::fmt::Display for PublicationRestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PublicationRestoreError {}
impl From<serde_json::Error> for PublicationRestoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
impl From<agq_kernel::archive::ArchiveError> for PublicationRestoreError {
    fn from(value: agq_kernel::archive::ArchiveError) -> Self {
        Self::Archive(value)
    }
}

/// Compiled, checked-in acceptance authority. Callers cannot construct this from
/// their cache, a success flag, a semantic digest, or arbitrary JSON.
pub struct AcceptedPublicationReceipt {
    receipt: Value,
    bindings: Value,
}
impl AcceptedPublicationReceipt {
    /// Read the repository's trusted receipt and cross-check its accepted binding
    /// manifest. An absent/stale acceptance keeps restoration disabled.
    pub fn checked_in() -> Result<Self, PublicationRestoreError> {
        Self::from_trusted_documents(RECEIPT, BINDINGS)
    }
    fn from_trusted_documents(
        receipt: &str,
        bindings: &str,
    ) -> Result<Self, PublicationRestoreError> {
        let receipt: Value = serde_json::from_str(receipt)?;
        let bindings: Value = serde_json::from_str(bindings)?;
        if receipt["format"] != "agq-kerml-accepted-publication/1"
            || receipt["status"] != "accepted"
            || bindings["format"] != "agq-kerml-accepted-bindings/2"
            || bindings["operational_profile"] != BaselineProfile::OPERATIONAL_V9.id()
        {
            return Err(PublicationRestoreError::NotAccepted);
        }
        if receipt["binding_manifest_sha256"] != json!(json_digest(&bindings)?) {
            return Err(PublicationRestoreError::Mismatch(
                "accepted binding manifest",
            ));
        }
        let identity = &receipt["complete_overlay"]["identity"];
        for (context_field, binding_field) in [
            ("operational_profile", "operational_profile"),
            ("rule_set", "rule_set"),
            ("semantic_digest", "semantic_publication_digest"),
            ("library_set", "library_set_identity"),
            ("binding_contract", "binding_contract"),
        ] {
            if identity[context_field].is_null()
                || identity[context_field] != bindings[binding_field]
            {
                return Err(PublicationRestoreError::Mismatch(
                    "receipt and binding identity",
                ));
            }
        }
        if receipt["source_content_set"] != bindings["source_content_set"]
            || receipt["source_content_set"].as_str().is_none()
            || receipt["facade_metadata_sha256"]
                .as_array()
                .is_none_or(|v| v.len() != 32)
            || receipt["complete_overlay"]["graph_sha256"]
                .as_array()
                .is_none_or(|v| v.len() != 32)
        {
            return Err(PublicationRestoreError::Mismatch(
                "receipt source or artifact identity",
            ));
        }
        Ok(Self { receipt, bindings })
    }
    /// Exact accepted source content set, independent of the model/archive hashes.
    pub fn source_content_set(&self) -> &str {
        self.receipt["source_content_set"]
            .as_str()
            .expect("checked receipt")
    }
    /// Verify the facade's complete source map, roots and mandatory reference count.
    pub fn verify_facade_metadata(&self, metadata: &Value) -> Result<(), PublicationRestoreError> {
        self.verify_facade_digest(json_digest(metadata)?)
    }
    /// Verify exact streamed facade bytes before JSON allocation.
    pub fn verify_facade_digest(&self, digest: [u8; 32]) -> Result<(), PublicationRestoreError> {
        if self.receipt["facade_metadata_sha256"] != json!(digest) {
            return Err(PublicationRestoreError::Mismatch("facade metadata"));
        }
        Ok(())
    }
    /// Exact uncompressed facade size, for bounded metadata ingestion.
    pub fn facade_metadata_bytes(&self) -> Result<u64, PublicationRestoreError> {
        self.receipt["facade_metadata_bytes"]
            .as_u64()
            .ok_or(PublicationRestoreError::Mismatch("facade byte count"))
    }
    /// Exact trusted accepted anchors, used for a final facade stale check.
    pub fn binding_manifest(&self) -> &Value {
        &self.bindings
    }
    /// Expected uncompressed graph size, for bounded archive ingestion.
    pub fn graph_bytes(&self) -> Result<u64, PublicationRestoreError> {
        self.receipt["complete_overlay"]["graph_bytes"]
            .as_u64()
            .ok_or(PublicationRestoreError::Mismatch("graph byte count"))
    }
    /// Check the streamed archive before allocating the decoded graph. This does
    /// not confer acceptance; restoration independently hashes the actual graph.
    pub fn verify_graph_digest(&self, digest: [u8; 32]) -> Result<(), PublicationRestoreError> {
        if self.receipt["complete_overlay"]["graph_sha256"] != json!(digest) {
            return Err(PublicationRestoreError::Mismatch("kernel archive"));
        }
        Ok(())
    }
}

impl CompletePublicationOverlay {
    /// Stream the neutral graph archive and return receipt material. Only an
    /// already sealed publication can produce this metadata. Checking it into
    /// the repository is separate from archive creation.
    pub fn write_accepted_archive(
        &self,
        writer: impl Write,
    ) -> Result<Value, PublicationRestoreError> {
        let mut writer = DigestWriter {
            inner: writer,
            digest: Sha256::new(),
            bytes: 0,
        };
        agq_kernel::archive::write_overlay(&self.overlay, &mut writer)?;
        let graph_sha256: [u8; 32] = writer.digest.finalize().into();
        let mut receipt = json!({
            "format": "agq-kerml-sealed-graph/1",
            "graph_sha256": graph_sha256,
            "graph_bytes": writer.bytes,
            "identity": context_identity(&self.context),
            "checked": self.checked.iter().map(|(family, count)| json!({
                "family":format!("{family:?}"), "count":count,
            })).collect::<Vec<_>>(),
        });
        if let Some(certificate) = &self.certificate {
            receipt["producer_closure"] = certificate.receipt_value();
        }
        Ok(receipt)
    }

    /// Restore the exact previously accepted graph without rerunning producers.
    /// The only authority is a compiled checked-in receipt, never cache metadata.
    /// Original run telemetry remains in verification evidence: this instance has
    /// no producer stages and zero local work counters.
    pub fn restore_accepted(
        overlay: DerivedOverlay,
        roots: &[ElementId],
        libraries: &LibrarySetIdentity,
        receipt: &AcceptedPublicationReceipt,
    ) -> Result<Self, PublicationRestoreError> {
        let expected = &receipt.receipt["complete_overlay"];
        if expected["format"] != "agq-kerml-sealed-graph/1" {
            return Err(PublicationRestoreError::Mismatch("sealed graph format"));
        }
        // Re-encode from the actual immutable graph. An upstream caller cannot
        // substitute a claimed digest for this validation.
        let mut writer = DigestWriter {
            inner: io::sink(),
            digest: Sha256::new(),
            bytes: 0,
        };
        agq_kernel::archive::write_overlay(&overlay, &mut writer)?;
        let digest: [u8; 32] = writer.digest.finalize().into();
        receipt.verify_graph_digest(digest)?;
        let builder = CanonicalPublicationBuilder::new(overlay.declared(), roots, libraries);
        let mut context = builder
            .context(&overlay)
            .map_err(PublicationRestoreError::Publication)?
            .id()
            .clone();
        context.derivation_phase = DerivationPhase::CompletePublicationOverlay;
        let certificate = if let Some(proof) = expected.get("producer_closure") {
            let registry = ProducerRegistry::new(
                ProducerFamily::ALL
                    .into_iter()
                    .map(|family| family.descriptor(context.options.baseline_profile)),
            )
            .map_err(|_| PublicationRestoreError::Mismatch("producer registry"))?;
            context.model_digest =
                crate::context_digest::producer_model_digest(overlay.model(), context.model_digest);
            context.producer_registry_digest = Some(registry.digest());
            let certificate = ProducerClosureCertificate::from_trusted_receipt(
                proof,
                &context,
                &registry,
                overlay.model(),
            )
            .ok_or(PublicationRestoreError::Mismatch(
                "producer closure certificate",
            ))?;
            context.producer_closure_digest = Some(certificate.digest());
            Some(std::sync::Arc::new(certificate))
        } else {
            None
        };
        if expected["identity"] != context_identity(&context) {
            return Err(PublicationRestoreError::Mismatch("semantic context"));
        }
        let entries = expected["checked"]
            .as_array()
            .ok_or(PublicationRestoreError::Mismatch("capability evidence"))?;
        let checked = entries
            .iter()
            .map(|entry| {
                let family = PublicationFamily::ALL
                    .into_iter()
                    .find(|family| entry["family"] == format!("{family:?}"))
                    .ok_or(PublicationRestoreError::Mismatch("capability family"))?;
                let count = entry["count"]
                    .as_u64()
                    .and_then(|count| usize::try_from(count).ok())
                    .ok_or(PublicationRestoreError::Mismatch("capability count"))?;
                Ok((family, count))
            })
            .collect::<Result<BTreeMap<_, _>, PublicationRestoreError>>()?;
        if entries.len() != PublicationFamily::ALL.len()
            || checked.len() != PublicationFamily::ALL.len()
            || checked.get(&PublicationFamily::StandardBindings) != Some(&StandardRole::ALL.len())
        {
            return Err(PublicationRestoreError::Mismatch(
                "complete capability population",
            ));
        }
        Ok(Self {
            overlay,
            context,
            checked,
            stages: vec![],
            counters: PublicationCounters::default(),
            restored_from_receipt: true,
            certificate,
        })
    }
}

fn context_identity(context: &SemanticContextId) -> Value {
    let bindings = context
        .standard_bindings
        .as_ref()
        .expect("complete publication bindings");
    let libraries = bindings.library_set();
    let mut identity = json!({
        "operational_profile":context.baseline_profile_id,
        "rule_set":context.rule_set_version,
        "binding_contract":context.binding_version,
        "metamodel":context.metamodel_version,
        "descriptor_sha256":context.descriptor_digest,
        "semantic_digest":context.model_digest,
        "revision":context.revision.to_string(),
        "library_graph_digest":context.library_graph_digest,
        "library_set":{
            "artifacts":libraries.artifacts.iter().map(|(a,id)|json!({"artifact":a.resource(),"library":id.to_string()})).collect::<Vec<_>>(),
            "pins":libraries.pins.iter().map(|p|json!({"resource":p.name,"sha256":p.sha256})).collect::<Vec<_>>(),
        },
        "available_roots":context.available_roots.iter().map(|(root, available)| json!({
            "root":root.to_string(), "available":available.iter().map(ToString::to_string).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "authority":{
            "errata":context.errata_manifest_digest,
            "result_domain":context.result_domain_manifest_digest,
            "reference_binding":context.reference_binding_manifest_digest,
            "owned_cross_feature":context.owned_cross_feature_manifest_digest,
            "owned_cross_domain":context.owned_cross_domain_manifest_digest,
            "import_collision":context.import_collision_manifest_digest,
            "multiplicity_context":context.multiplicity_context_manifest_digest,
            "cross_multiplicity_context":context.cross_multiplicity_context_manifest_digest,
        },
    });
    // Preserve the exact historical receipt encoding for KerML-only inputs.
    // A composed producer contract must never disappear from a new receipt.
    if !context.semantic_extensions.is_empty() {
        identity["semantic_extensions"] = json!(context.semantic_extensions);
    }
    identity
}

fn json_digest(value: &Value) -> Result<[u8; 32], serde_json::Error> {
    let mut writer = DigestWriter {
        inner: io::sink(),
        digest: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut writer, value)?;
    Ok(writer.digest.finalize().into())
}
struct DigestWriter<W> {
    inner: W,
    digest: Sha256,
    bytes: u64,
}
impl<W: Write> Write for DigestWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.digest.update(&bytes[..written]);
        self.bytes += written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
