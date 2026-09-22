//! Exact producer evidence restoration under independently compiled authority.
//! Language facades interpret their own authenticated receipt and binding fields.
use crate::{ProducerClosureCertificate, ProducerRegistry, SemanticContext};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read},
    sync::Arc,
};

struct CatalogueEntry<'a> {
    id: &'static str,
    receipt_format: &'static str,
    receipt: &'a str,
    bindings: &'a str,
}

// Add only independently accepted receipt/binding documents. Cache files and
// callers cannot supply entries. Existing KerML /26 restoration is separate.
const CATALOGUE: &[CatalogueEntry<'static>] = &[];

/// Failure to authenticate previously accepted producer evidence.
#[derive(Debug)]
pub enum TrustedPublicationError {
    Unavailable,
    Mismatch(&'static str),
    Io(io::Error),
    Json(serde_json::Error),
}
impl std::fmt::Display for TrustedPublicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TrustedPublicationError {}
impl From<io::Error> for TrustedPublicationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<serde_json::Error> for TrustedPublicationError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// Opaque authority selected from a finite compiled catalogue. It has no public
/// data constructor, deserializer, mutable document access or authority trait.
pub struct TrustedPublicationReceipt {
    id: &'static str,
    receipt: Value,
    bindings: Value,
}

impl TrustedPublicationReceipt {
    /// Select existing independent authority. An unknown or not-yet-accepted
    /// identifier cannot enable restoration, regardless of cache contents.
    pub fn checked_in(id: &str) -> Result<Self, TrustedPublicationError> {
        let entry = CATALOGUE
            .iter()
            .find(|entry| entry.id == id)
            .ok_or(TrustedPublicationError::Unavailable)?;
        Self::from_catalogue(entry)
    }

    fn from_catalogue(entry: &CatalogueEntry<'_>) -> Result<Self, TrustedPublicationError> {
        let receipt: Value = serde_json::from_str(entry.receipt)?;
        let bindings: Value = serde_json::from_str(entry.bindings)?;
        if receipt["format"] != entry.receipt_format
            || receipt["status"] != "accepted"
            || receipt["binding_manifest_sha256"] != json!(digest_json(&bindings)?)
            || receipt["source_content_set"].as_str().is_none()
        {
            return Err(TrustedPublicationError::Mismatch("catalogue documents"));
        }
        let result = Self {
            id: entry.id,
            receipt,
            bindings,
        };
        let entries =
            result.receipt["entries"]
                .as_object()
                .ok_or(TrustedPublicationError::Mismatch(
                    "catalogue archive entries",
                ))?;
        if entries.is_empty() {
            return Err(TrustedPublicationError::Mismatch(
                "catalogue archive entries",
            ));
        }
        for name in entries.keys() {
            result.entry_bytes(name)?;
            let _: [u8; 32] =
                serde_json::from_value(result.receipt["entries"][name]["sha256"].clone())?;
        }
        Ok(result)
    }

    /// Catalogue selection, independent of caller-supplied cache labels.
    pub fn id(&self) -> &'static str {
        self.id
    }
    /// Authenticated format label. Only the consuming language facade interprets it.
    pub fn publication_format(&self) -> &str {
        self.receipt["format"].as_str().expect("checked catalogue")
    }
    /// Authenticated archive population, without prescribing a language container.
    pub fn entry_names(&self) -> impl Iterator<Item = &str> {
        self.receipt["entries"]
            .as_object()
            .expect("checked catalogue")
            .keys()
            .map(String::as_str)
    }
    /// Authenticated, immutable publication identity fields.
    pub fn identity(&self) -> &Value {
        &self.receipt["identity"]
    }
    /// Complete authenticated anchor manifest, for independent reconstruction.
    pub fn binding_manifest(&self) -> &Value {
        &self.bindings
    }
    /// Verified source set required by this accepted publication.
    pub fn source_content_set(&self) -> &str {
        self.receipt["source_content_set"]
            .as_str()
            .expect("checked catalogue")
    }
    /// Exact uncompressed size used before decoding archive data.
    pub fn entry_bytes(&self, name: &str) -> Result<u64, TrustedPublicationError> {
        self.receipt["entries"][name]["bytes"]
            .as_u64()
            .filter(|bytes| *bytes < u64::MAX)
            .ok_or(TrustedPublicationError::Mismatch(
                "archive entry byte count",
            ))
    }
    /// Compare actual bytes with independent authority; this alone grants no
    /// model acceptance and does not bypass decoded graph revalidation.
    pub fn verify_entry_digest(
        &self,
        name: &str,
        digest: [u8; 32],
    ) -> Result<(), TrustedPublicationError> {
        if self.receipt["entries"][name]["sha256"] != json!(digest) {
            return Err(TrustedPublicationError::Mismatch("archive entry digest"));
        }
        Ok(())
    }

    /// Restore exact closure evidence without replay. The context owns an actual
    /// model binding; callers cannot substitute an asserted context identifier.
    pub fn restore_producer_closure(
        &self,
        reader: impl Read,
        context: &SemanticContext<'_>,
        registry: &ProducerRegistry,
    ) -> Result<Arc<ProducerClosureCertificate>, TrustedPublicationError> {
        let expected = self.entry_bytes("closure.json")?;
        let mut bytes = Vec::new();
        reader.take(expected + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != expected {
            return Err(TrustedPublicationError::Mismatch("closure byte count"));
        }
        self.verify_entry_digest("closure.json", Sha256::digest(&bytes).into())?;
        if context.id().producer_registry_digest != Some(registry.digest())
            || self.identity()["semantic_digest"] != json!(context.id().model_digest)
            || self.identity()["producer_registry_digest"] != json!(registry.digest())
            || self.identity()["producer_context_contract_digest"]
                != json!(context.id().closure_contract_digest())
        {
            return Err(TrustedPublicationError::Mismatch(
                "closure context identity",
            ));
        }
        let value: Value = serde_json::from_slice(&bytes)?;
        let certificate = ProducerClosureCertificate::from_trusted_receipt(
            &value,
            context.id(),
            registry,
            context.model,
        )
        .ok_or(TrustedPublicationError::Mismatch("closure certificate"))?;
        if self.identity()["producer_closure_digest"] != json!(certificate.digest()) {
            return Err(TrustedPublicationError::Mismatch(
                "closure certificate digest",
            ));
        }
        Ok(Arc::new(certificate))
    }
}

fn digest_json(value: &Value) -> Result<[u8; 32], serde_json::Error> {
    Ok(Sha256::digest(serde_json::to_vec(value)?).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ProducerApplicability, ProducerDescriptor, ProducerEffect, ProducerFamilyId,
        SemanticOptions,
    };
    use agq_kernel::Snapshot;
    use std::collections::BTreeSet;

    fn fixture_receipt(
        context: &SemanticContext<'_>,
        registry: &ProducerRegistry,
    ) -> (TrustedPublicationReceipt, Vec<u8>) {
        let certificate = ProducerClosureCertificate::initial(context, registry).unwrap();
        let bytes = serde_json::to_vec(&certificate.receipt_value()).unwrap();
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let identity = json!({
            "semantic_digest":context.id().model_digest,
            "producer_registry_digest":registry.digest(),
            "producer_closure_digest":certificate.digest(),
            "producer_context_contract_digest":context.id().closure_contract_digest(),
        });
        let bindings = json!({
            "fixture_anchors":[{"label":"independent language payload"}],
        });
        let receipt = json!({
            "format":"fixture-publication/1", "status":"accepted",
            "source_content_set":"fixture", "binding_manifest_sha256":digest_json(&bindings).unwrap(),
            "identity":identity, "entries":{
                "closure.json":{"bytes":bytes.len(),"sha256":digest},
            },
        });
        let receipt_text = receipt.to_string();
        let bindings_text = bindings.to_string();
        let trusted = TrustedPublicationReceipt::from_catalogue(&CatalogueEntry {
            id: "private-unit-fixture",
            receipt_format: "fixture-publication/1",
            receipt: &receipt_text,
            bindings: &bindings_text,
        })
        .unwrap();
        (trusted, bytes)
    }

    #[test]
    fn catalogue_authenticates_generic_envelope_and_opaque_binding_payload() {
        let snapshot = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
        let registry = ProducerRegistry::new([]).unwrap();
        let context =
            SemanticContext::for_snapshot(&snapshot, SemanticOptions::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(registry.digest())
                .unwrap();
        let (receipt, _) = fixture_receipt(&context, &registry);
        assert_eq!(receipt.publication_format(), "fixture-publication/1");
        assert!(receipt.binding_manifest()["format"].is_null());
        assert_eq!(
            receipt.entry_names().collect::<Vec<_>>(),
            vec!["closure.json"]
        );
        let receipt_text = receipt.receipt.to_string();
        let bindings_text = receipt.bindings.to_string();
        assert!(
            TrustedPublicationReceipt::from_catalogue(&CatalogueEntry {
                id: "private-unit-fixture",
                receipt_format: "different-publication/1",
                receipt: &receipt_text,
                bindings: &bindings_text,
            })
            .is_err()
        );
    }

    #[test]
    fn caller_identifier_cannot_enable_unaccepted_systems_restoration() {
        assert!(matches!(
            TrustedPublicationReceipt::checked_in("sysml-systems-operational-v2"),
            Err(TrustedPublicationError::Unavailable)
        ));
        assert!(matches!(
            TrustedPublicationReceipt::checked_in("{\"status\":\"accepted\"}"),
            Err(TrustedPublicationError::Unavailable)
        ));
    }

    #[test]
    fn exact_closure_restores_but_changed_bytes_bounds_and_context_do_not() {
        let snapshot = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
        let registry = ProducerRegistry::new([]).unwrap();
        let context =
            SemanticContext::for_snapshot(&snapshot, SemanticOptions::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(registry.digest())
                .unwrap();
        let (receipt, bytes) = fixture_receipt(&context, &registry);
        let restored = receipt
            .restore_producer_closure(bytes.as_slice(), &context, &registry)
            .unwrap();
        assert_eq!(
            restored.receipt_value(),
            ProducerClosureCertificate::initial(&context, &registry)
                .unwrap()
                .receipt_value()
        );
        let mut changed = bytes.clone();
        changed[0] = b'[';
        assert!(matches!(
            receipt.restore_producer_closure(changed.as_slice(), &context, &registry),
            Err(TrustedPublicationError::Mismatch("archive entry digest"))
        ));
        for altered in [
            &bytes[..bytes.len() - 1],
            [bytes.as_slice(), b" "].concat().as_slice(),
        ] {
            assert!(matches!(
                receipt.restore_producer_closure(altered, &context, &registry),
                Err(TrustedPublicationError::Mismatch("closure byte count"))
            ));
        }
        let changed = SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                exclude_implied: true,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
        assert!(matches!(
            receipt.restore_producer_closure(bytes.as_slice(), &changed, &registry),
            Err(TrustedPublicationError::Mismatch(
                "closure context identity"
            ))
        ));
    }

    #[test]
    fn weaker_registry_and_recomputed_cache_certificate_cannot_replace_authority() {
        let snapshot = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
        let registry = ProducerRegistry::new([ProducerDescriptor::new(
            ProducerFamilyId::new("fixture.typing"),
            [ProducerEffect::Typing],
            ProducerApplicability::Never,
        )])
        .unwrap();
        let context =
            SemanticContext::for_snapshot(&snapshot, SemanticOptions::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(registry.digest())
                .unwrap();
        let (receipt, bytes) = fixture_receipt(&context, &registry);
        let weaker = ProducerRegistry::new([]).unwrap();
        let weaker_context =
            SemanticContext::for_snapshot(&snapshot, SemanticOptions::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(weaker.digest())
                .unwrap();
        assert!(matches!(
            receipt.restore_producer_closure(bytes.as_slice(), &weaker_context, &weaker),
            Err(TrustedPublicationError::Mismatch(
                "closure context identity"
            ))
        ));
        let forged = ProducerClosureCertificate::initial(&weaker_context, &weaker).unwrap();
        let forged_bytes = serde_json::to_vec(&forged.receipt_value()).unwrap();
        assert!(
            receipt
                .restore_producer_closure(forged_bytes.as_slice(), &weaker_context, &weaker)
                .is_err()
        );
        let mut bad_bindings = receipt.bindings.clone();
        bad_bindings["semantic_digest"] = json!(vec![7u8; 32]);
        let receipt_text = receipt.receipt.to_string();
        let binding_text = bad_bindings.to_string();
        assert!(
            TrustedPublicationReceipt::from_catalogue(&CatalogueEntry {
                id: "private-unit-fixture",
                receipt_format: "fixture-publication/1",
                receipt: &receipt_text,
                bindings: &binding_text,
            })
            .is_err()
        );
    }
}
