//! Receipt-authorized dependent overlay restoration. No producer replay occurs.
use super::*;
use agq_kerml_semantics::TrustedPublicationReceipt;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    io::{BufReader, Cursor, Read},
};
use zip::ZipArchive;

const ENTRIES: [&str; 3] = ["closure.json", "facade.json", "kernel.jsonl"];

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FacadeMetadata {
    format: String,
    source_content_set: String,
    roots: Vec<ElementId>,
    source_map: Vec<(FactKey, SourceOrigin)>,
    documents: Vec<DocumentMetadata>,
    mandatory_references: usize,
    complete_references: usize,
    checked: BTreeMap<String, usize>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentMetadata {
    path: String,
    document: DocumentId,
    sha256: String,
    profile: String,
    parsed: bool,
    byte_exact: bool,
    recovery_count: usize,
    production_count: usize,
    construction_gap: Option<String>,
}

// Candidate authority is created only from an existing accepted facade. JSON
// supplied by a caller never enters this type without exact live comparison.
enum ReceiptAuthority {
    Trusted(TrustedPublicationReceipt),
    Candidate {
        receipt: Value,
        bindings: Value,
        certificate: Arc<agq_kerml_semantics::ProducerClosureCertificate>,
    },
}
impl ReceiptAuthority {
    fn profile(
        &self,
    ) -> Result<(SysmlBaselineProfile, SysmlSyntaxProfile), SystemsPublicationCacheError> {
        match self.identity()["operational_profile"].as_str() {
            Some("agentique-sysml-2.0-operational/2") => Ok((
                SysmlBaselineProfile::OPERATIONAL_V2,
                SysmlSyntaxProfile::OperationalV2,
            )),
            Some("agentique-sysml-2.0-operational/3") => Ok((
                SysmlBaselineProfile::OPERATIONAL_V3,
                SysmlSyntaxProfile::OperationalV3,
            )),
            _ => Err(SystemsPublicationCacheError::Mismatch(
                "receipt SysML interpretation",
            )),
        }
    }
    fn identity(&self) -> &Value {
        match self {
            Self::Trusted(receipt) => receipt.identity(),
            Self::Candidate { receipt, .. } => &receipt["identity"],
        }
    }
    fn binding_manifest(&self) -> &Value {
        match self {
            Self::Trusted(receipt) => receipt.binding_manifest(),
            Self::Candidate { bindings, .. } => bindings,
        }
    }
    fn source_content_set(&self) -> &str {
        match self {
            Self::Trusted(receipt) => receipt.source_content_set(),
            Self::Candidate { receipt, .. } => receipt["source_content_set"]
                .as_str()
                .expect("live receipt"),
        }
    }
    fn publication_format(&self) -> &str {
        match self {
            Self::Trusted(receipt) => receipt.publication_format(),
            Self::Candidate { receipt, .. } => receipt["format"].as_str().expect("live receipt"),
        }
    }
    fn entry_names(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        match self {
            Self::Trusted(receipt) => Box::new(receipt.entry_names()),
            Self::Candidate { receipt, .. } => Box::new(
                receipt["entries"]
                    .as_object()
                    .expect("live receipt")
                    .keys()
                    .map(String::as_str),
            ),
        }
    }
    fn entry_bytes(&self, name: &str) -> Result<u64, SystemsPublicationCacheError> {
        match self {
            Self::Trusted(receipt) => Ok(receipt.entry_bytes(name)?),
            Self::Candidate { receipt, .. } => receipt["entries"][name]["bytes"]
                .as_u64()
                .filter(|n| *n < u64::MAX)
                .ok_or(SystemsPublicationCacheError::Mismatch(
                    "candidate archive entry size",
                )),
        }
    }
    fn verify_entry_digest(
        &self,
        name: &str,
        digest: [u8; 32],
    ) -> Result<(), SystemsPublicationCacheError> {
        match self {
            Self::Trusted(receipt) => Ok(receipt.verify_entry_digest(name, digest)?),
            Self::Candidate { receipt, .. }
                if receipt["entries"][name]["sha256"] == json!(digest) =>
            {
                Ok(())
            }
            Self::Candidate { .. } => Err(SystemsPublicationCacheError::Mismatch(
                "candidate archive entry digest",
            )),
        }
    }
    fn attach_closure<'m>(
        &self,
        context: SysmlSemanticContext<'m>,
        bytes: Vec<u8>,
    ) -> Result<SysmlSemanticContext<'m>, SystemsPublicationCacheError> {
        match self {
            Self::Trusted(receipt) => {
                Ok(context.with_trusted_producer_closure(receipt, Cursor::new(bytes))?)
            }
            // Bytes were authenticated against the live certificate serialization.
            // Reattach the actual scheduler certificate, validating the restored
            // context and independently reconstructed registry through its API.
            Self::Candidate { certificate, .. } => {
                Ok(context.with_producer_closure(certificate.clone())?)
            }
        }
    }
}

impl CanonicalSysmlSystemsLibrary {
    /// Restore only the exact separately accepted receipt and original sources.
    /// Uses the current independently accepted Systems interpretation.
    /// The accepted KerML allocation is shared; local producer counters are zero.
    pub fn restore_cache(
        reader: impl Read + Seek,
        sources: &VerifiedLibrarySet,
        accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
    ) -> Result<Self, SystemsPublicationCacheError> {
        Self::restore_cache_for_profile(
            reader,
            sources,
            accepted_kerml,
            SysmlBaselineProfile::OPERATIONAL,
        )
    }

    /// Restore a specific separately accepted interpretation. Selecting a
    /// profile cannot create catalogue authority or substitute another receipt.
    pub fn restore_cache_for_profile(
        reader: impl Read + Seek,
        sources: &VerifiedLibrarySet,
        accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
        profile: SysmlBaselineProfile,
    ) -> Result<Self, SystemsPublicationCacheError> {
        let id = match profile {
            SysmlBaselineProfile::OperationalV2 => "sysml-systems-operational-v2",
            SysmlBaselineProfile::OperationalV3 => "sysml-systems-operational-v3",
            _ => {
                return Err(SystemsPublicationCacheError::Mismatch(
                    "receipt SysML interpretation",
                ));
            }
        };
        let receipt = ReceiptAuthority::Trusted(TrustedPublicationReceipt::checked_in(id)?);
        if receipt.profile()?.0 != profile {
            return Err(SystemsPublicationCacheError::Mismatch(
                "receipt SysML interpretation",
            ));
        }
        Self::restore_with_authority(reader, sources, accepted_kerml, receipt)
    }

    /// Cross-validate candidate artifacts against this already accepted facade,
    /// then restore their exact graph with no producer replay. Caller JSON is
    /// compared with independently regenerated live entry/identity digests.
    /// Consuming the original releases its graph before decoding the replacement.
    /// This does not add a receipt to the separately compiled trust catalogue.
    pub fn verify_candidate_cache(
        self,
        reader: impl Read + Seek,
        sources: &VerifiedLibrarySet,
        candidate_receipt: &Value,
        candidate_bindings: &Value,
    ) -> Result<Self, SystemsPublicationCacheError> {
        self.check_binding_manifest(sources, candidate_bindings)?;
        let bindings = self.binding_manifest(sources)?;
        let mut entries = BTreeMap::new();
        for (name, value) in [
            ("facade.json", self.facade_metadata(sources)),
            ("closure.json", self.producer_closure.receipt_value()),
        ] {
            let mut digest = DigestWriter::new(io::sink());
            serde_json::to_writer(&mut digest, &value)?;
            entries.insert(name, digest.identity());
        }
        let mut digest = DigestWriter::new(io::sink());
        agq_kernel::archive::write_dependent_overlay_with_evidence(&self.overlay, &mut digest)?;
        entries.insert("kernel.jsonl", digest.identity());
        let receipt = self.cache_receipt(sources, &bindings, entries)?;
        if *candidate_receipt != receipt {
            return Err(SystemsPublicationCacheError::Mismatch(
                "candidate receipt differs from accepted facade",
            ));
        }
        let accepted_kerml = self.accepted_kerml.clone();
        let authority = ReceiptAuthority::Candidate {
            receipt,
            bindings,
            certificate: self.producer_closure.clone(),
        };
        drop(self);
        Self::restore_with_authority(reader, sources, accepted_kerml, authority)
    }

    fn restore_with_authority(
        reader: impl Read + Seek,
        sources: &VerifiedLibrarySet,
        accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
        receipt: ReceiptAuthority,
    ) -> Result<Self, SystemsPublicationCacheError> {
        let mut trace = crate::runtime_restore_trace::RuntimeRestoreTrace::new("sysml");
        let identity = SystemsLibraryIdentity::pinned(SystemsLibraryIdentity::SOURCE_CONTENT_SET);
        let (profile, syntax_profile) = receipt.profile()?;
        let empty_bindings = StandardSysmlBindings::unbound(identity.clone());
        let initial_contract =
            SysmlDependencyContract::checked_in_for_profile(&empty_bindings, profile)?;
        check_interpretation(&receipt, sources, &accepted_kerml, &initial_contract)?;
        let mut archive = ZipArchive::new(reader)?;
        check_entries(&mut archive)?;
        trace.phase("receipt_interpretation_and_archive_open");
        let metadata = authenticated_bytes(&mut archive, &receipt, "facade.json")?;
        let metadata: FacadeMetadata = serde_json::from_slice(&metadata)?;
        metadata.validate(sources, syntax_profile)?;
        trace.phase("facade_authentication_decode_validation");
        // Hash the compressed entry before allocating decoded graph records.
        let graph_bytes = receipt.entry_bytes("kernel.jsonl")?;
        let digest = entry_digest(&mut archive, "kernel.jsonl", graph_bytes)?;
        receipt.verify_entry_digest("kernel.jsonl", digest)?;
        trace.phase("graph_input_authentication");
        let registry = agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9)
            .map_err(|_| SystemsPublicationCacheError::Mismatch("combined descriptor registry"))?;
        let dependency = accepted_kerml
            .project_snapshot()
            .immutable_dependency()
            .expect("accepted KerML dependency")
            .clone();
        trace.phase("descriptor_registry_and_dependency_mount");
        let overlay = agq_kernel::archive::read_dependent_overlay_with_evidence(
            BufReader::new(archive.by_name("kernel.jsonl")?.take(graph_bytes)),
            Arc::new(registry),
            dependency,
        )?;
        trace.phase("graph_decode_kernel_validation_and_dependency_authentication");
        // A changing seekable input cannot replace the graph after its first
        // hash. Authenticate what was actually decoded, retaining derived facts.
        let mut actual_graph = DigestWriter::new(io::sink());
        agq_kernel::archive::write_dependent_overlay_with_evidence(&overlay, &mut actual_graph)?;
        if actual_graph.bytes != graph_bytes {
            return Err(SystemsPublicationCacheError::Mismatch(
                "decoded graph byte count",
            ));
        }
        receipt.verify_entry_digest("kernel.jsonl", actual_graph.digest.finalize().into())?;
        trace.phase("graph_recanonicalization_and_dependency_authentication");
        metadata.validate_graph(&overlay, identity.library)?;
        let source_map: LibrarySourceMap = metadata.source_map.iter().cloned().collect();
        let library = sources.libraries().get(&identity.library).ok_or(
            SystemsPublicationCacheError::Mismatch("Systems source library"),
        )?;
        trace.phase("source_and_graph_validation");
        // Bind role validation to the same producer-aware model identity used
        // by the restored certificate and subsequent mounted dependency. The
        // explicit constructor derives the full registry independently; an
        // aggregate current-graph digest is not interchangeable with this one.
        let current = SysmlSemanticContext::for_producer_overlay(
            &overlay,
            accepted_kerml.complete_overlay(),
            &metadata.roots,
            &initial_contract,
            empty_bindings,
        )?;
        trace.phase("initial_producer_context");
        let queries = KerMlQueries::new(current.kerml_context().fork());
        let bindings = StandardSysmlBindings::validate(
            overlay.model(),
            &queries,
            identity,
            &metadata.roots,
            StandardSysmlRole::ALL,
        )?
        .with_verified_sources(library, &source_map)?;
        trace.phase("standard_binding_validation");
        drop(queries);
        drop(current);
        let contract = SysmlDependencyContract::checked_in_for_profile(&bindings, profile)?;
        if receipt.identity()["dependency_contract_digest"]
            != json!(contract.context_identity_digest())
        {
            return Err(SystemsPublicationCacheError::Mismatch(
                "dependency contract digest",
            ));
        }
        trace.phase("bound_dependency_contract");
        let closure_bytes = authenticated_bytes(&mut archive, &receipt, "closure.json")?;
        trace.phase("closure_input_authentication");
        let context = SysmlSemanticContext::for_producer_overlay(
            &overlay,
            accepted_kerml.complete_overlay(),
            &metadata.roots,
            &contract,
            bindings.clone(),
        )?;
        trace.phase("bound_producer_context");
        let context = receipt.attach_closure(context, closure_bytes)?;
        let producer_closure = context
            .kerml_context()
            .producer_closure()
            .ok_or(SystemsPublicationCacheError::Mismatch(
                "restored producer certificate",
            ))?
            .clone();
        if !producer_closure.is_fully_closed(overlay.model()) {
            return Err(SystemsPublicationCacheError::Mismatch(
                "restored producer requirement coverage",
            ));
        }
        trace.phase("producer_closure_authentication_and_coverage");
        let context_id = context.id().clone();
        let publication_identity = publication_identity(contract, &context_id.kerml);
        if receipt.identity()["publication_digest"]
            != json!(publication_identity.publication_digest)
            || receipt.identity()["semantic_digest"] != json!(publication_identity.semantic_digest)
        {
            return Err(SystemsPublicationCacheError::Mismatch(
                "restored publication identity",
            ));
        }
        trace.phase("accepted_publication_identity");
        drop(context);
        let checked = metadata.checked_families()?;
        let publication = Self {
            overlay: Arc::new(overlay),
            producer_closure,
            accepted_kerml,
            bindings,
            identity: publication_identity,
            context: context_id,
            roots: metadata.roots,
            source_map,
            documents: metadata
                .documents
                .into_iter()
                .map(|doc| SystemsDocumentStatus {
                    path: doc.path,
                    document: doc.document,
                    source_sha256: doc.sha256,
                    profile: syntax_profile,
                    parsed: doc.parsed,
                    byte_exact: doc.byte_exact,
                    recovery_count: doc.recovery_count,
                    production_count: doc.production_count,
                    construction_gap: doc.construction_gap,
                })
                .collect(),
            audit: SystemsPublicationAudit {
                checked,
                mandatory_references: metadata.mandatory_references,
                complete_references: metadata.complete_references,
                findings: vec![],
            },
            counters: PublicationCounters::default(),
        };
        publication.check_binding_manifest(sources, receipt.binding_manifest())?;
        trace.phase("facade_and_binding_manifest");
        Ok(publication)
    }
}

fn check_interpretation(
    receipt: &ReceiptAuthority,
    sources: &VerifiedLibrarySet,
    accepted_kerml: &CanonicalKermlStandardLibraries,
    contract: &SysmlDependencyContract,
) -> Result<(), SystemsPublicationCacheError> {
    if receipt.entry_names().collect::<BTreeSet<_>>() != ENTRIES.into_iter().collect() {
        return Err(SystemsPublicationCacheError::Mismatch(
            "receipt archive entries",
        ));
    }
    check_receipt_documents(
        receipt.publication_format(),
        receipt.source_content_set(),
        receipt.identity(),
        receipt.binding_manifest(),
    )?;
    if receipt.source_content_set() != sources.content_set_id()
        || accepted_kerml.source_content_set() != sources.content_set_id()
        || accepted_kerml.semantic_digest() != contract.kerml_publication_digest
        || sources
            .libraries()
            .get(&contract.systems_library.library)
            .is_none_or(|library| {
                library.archive_sha256() != contract.systems_library.artifact_sha256
            })
    {
        return Err(SystemsPublicationCacheError::Mismatch(
            "verified source or dependency identity",
        ));
    }
    for (field, expected) in [
        (
            "accepted_kerml_digest",
            json!(accepted_kerml.semantic_digest()),
        ),
        (
            "systems_kpar",
            json!(contract.systems_library.artifact_sha256),
        ),
        (
            "systems_source_content_set",
            json!(contract.systems_library.source_content_set),
        ),
        ("operational_profile", json!(contract.sysml_profile.id())),
        ("rule_set", json!(contract.sysml_rule_set)),
        (
            "grammar_compatibility_manifest",
            json!(contract.grammar_compatibility_manifest_digest),
        ),
        (
            "semantic_correction_manifest",
            json!(contract.semantic_correction_manifest_digest),
        ),
        (
            "combined_descriptor_graph",
            json!(contract.combined_descriptor_digest),
        ),
    ] {
        if receipt.identity()[field] != expected {
            return Err(SystemsPublicationCacheError::Mismatch(field));
        }
    }
    Ok(())
}

// The lower semantic layer authenticates an opaque catalogue envelope. SysML
// owns the schema and cross-document interpretation of this accepted facade.
fn check_receipt_documents(
    format: &str,
    source_content_set: &str,
    identity: &Value,
    bindings: &Value,
) -> Result<(), SystemsPublicationCacheError> {
    if format != "agq-sysml-accepted-publication/1"
        || bindings["format"] != "agq-sysml-accepted-bindings/1"
        || bindings["source_content_set"] != source_content_set
    {
        return Err(SystemsPublicationCacheError::Mismatch(
            "receipt document schema",
        ));
    }
    for (receipt_field, binding_field) in [
        ("publication_digest", "accepted_systems_digest"),
        ("semantic_digest", "semantic_digest"),
        ("accepted_kerml_digest", "accepted_kerml_digest"),
        ("systems_kpar", "systems_kpar"),
        ("systems_source_content_set", "systems_source_content_set"),
        ("operational_profile", "operational_profile"),
        ("rule_set", "rule_set"),
        ("producer_registry_digest", "producer_registry_digest"),
        ("producer_closure_digest", "producer_closure_digest"),
    ] {
        if identity[receipt_field].is_null() || identity[receipt_field] != bindings[binding_field] {
            return Err(SystemsPublicationCacheError::Mismatch(
                "receipt binding identity",
            ));
        }
    }
    Ok(())
}

impl FacadeMetadata {
    fn validate(
        &self,
        sources: &VerifiedLibrarySet,
        profile: SysmlSyntaxProfile,
    ) -> Result<(), SystemsPublicationCacheError> {
        let documents: BTreeMap<_, _> = sources
            .documents()
            .filter(|source| source.language() == LibraryLanguage::SysMl)
            .map(|source| (source.document(), source))
            .collect();
        let actual: BTreeSet<_> = self.documents.iter().map(|doc| doc.document).collect();
        if self.format != "agq-sysml-publication-facade/1"
            || self.source_content_set != sources.content_set_id()
            || self.mandatory_references != SYSTEMS_MANDATORY_REFERENCE_ASSERTIONS
            || self.complete_references != SYSTEMS_MANDATORY_REFERENCE_ASSERTIONS
            || self.documents.len() != 21
            || documents.len() != 21
            || actual != documents.keys().copied().collect()
            || self.roots.is_empty()
            || self.roots.iter().copied().collect::<BTreeSet<_>>().len() != self.roots.len()
            || self
                .source_map
                .windows(2)
                .any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err(SystemsPublicationCacheError::Mismatch(
                "facade populations or source map order",
            ));
        }
        for doc in &self.documents {
            if doc.profile != profile.id()
                || !doc.parsed
                || !doc.byte_exact
                || doc.recovery_count != 0
                || doc.construction_gap.is_some()
                || documents
                    .get(&doc.document)
                    .is_none_or(|source| source.path() != doc.path || source.sha256() != doc.sha256)
            {
                return Err(SystemsPublicationCacheError::Mismatch(
                    "exact document identity or status",
                ));
            }
        }
        for (_, origin) in &self.source_map {
            if documents.get(&origin.document).is_none_or(|source| {
                source.revision() != origin.revision
                    || origin.syntax_node.is_none()
                    || source
                        .source()
                        .get(origin.range.start() as usize..origin.range.end() as usize)
                        .is_none()
            }) {
                return Err(SystemsPublicationCacheError::Mismatch("source provenance"));
            }
        }
        self.checked_families()?;
        Ok(())
    }

    fn validate_graph(
        &self,
        overlay: &DerivedOverlay,
        library: agq_kernel::LibraryId,
    ) -> Result<(), SystemsPublicationCacheError> {
        let expected = DeclaredOrigin::StandardLibrary { library };
        let source_map: BTreeMap<_, _> = self.source_map.iter().cloned().collect();
        if self.roots.iter().any(|id| {
            overlay.declared().is_dependency_element(*id) || overlay.model().element(*id).is_none()
        }) || source_map
            .keys()
            .any(|fact| overlay.declared().model().declared_fact_origin(*fact) != Some(&expected))
        {
            return Err(SystemsPublicationCacheError::Mismatch(
                "source map graph provenance",
            ));
        }
        let mut local_facts = BTreeSet::new();
        for record in overlay
            .declared()
            .model()
            .elements()
            .filter(|record| !overlay.declared().is_dependency_element(record.id()))
        {
            local_facts.insert(FactKey::Element(record.id()));
            local_facts.extend(record.slots().map(|(property, _)| FactKey::Property {
                element: record.id(),
                property,
            }));
        }
        for occurrence in
            overlay
                .declared()
                .model()
                .association_occurrences()
                .filter(|occurrence| {
                    overlay
                        .declared()
                        .immutable_dependency()
                        .is_none_or(|dependency| {
                            dependency
                                .model()
                                .association_occurrence(occurrence.id())
                                .is_none()
                        })
                })
        {
            local_facts.insert(FactKey::AssociationOccurrence(occurrence.id()));
        }
        if local_facts != source_map.keys().copied().collect() {
            return Err(SystemsPublicationCacheError::Mismatch(
                "complete declared source map",
            ));
        }
        Ok(())
    }

    fn checked_families(
        &self,
    ) -> Result<BTreeMap<SystemsPublicationFamily, usize>, SystemsPublicationCacheError> {
        use SystemsPublicationFamily::*;
        let families = [
            Syntax,
            CanonicalLowering,
            NamespacesImports,
            DefinitionUsage,
            AttributeItemPart,
            OccurrenceActionState,
            CalculationConstraintRequirementCase,
            PortConnectionInterfaceFlow,
            ViewMetadata,
            TypingSpecializationSubsettingRedefinition,
            MayTimeVary,
            StandardBindings,
            IdentityProvenance,
        ];
        if self.checked.len() != families.len() {
            return Err(SystemsPublicationCacheError::Mismatch(
                "checked family population",
            ));
        }
        families
            .into_iter()
            .map(|family| {
                self.checked
                    .get(&format!("{family:?}"))
                    .copied()
                    .filter(|count| *count != 0)
                    .map(|count| (family, count))
                    .ok_or(SystemsPublicationCacheError::Mismatch(
                        "checked family population",
                    ))
            })
            .collect()
    }
}

fn check_entries<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<(), SystemsPublicationCacheError> {
    let mut names = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() || !names.insert(entry.name().to_owned()) {
            return Err(SystemsPublicationCacheError::Mismatch(
                "archive duplicate or directory",
            ));
        }
    }
    if names != ENTRIES.into_iter().map(str::to_owned).collect() {
        return Err(SystemsPublicationCacheError::Mismatch("archive entries"));
    }
    Ok(())
}

fn authenticated_bytes<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    receipt: &ReceiptAuthority,
    name: &str,
) -> Result<Vec<u8>, SystemsPublicationCacheError> {
    let expected = receipt.entry_bytes(name)?;
    let entry = archive.by_name(name)?;
    if entry.size() != expected {
        return Err(SystemsPublicationCacheError::Mismatch(
            "archive entry byte count",
        ));
    }
    let mut bytes = Vec::new();
    entry.take(expected + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != expected {
        return Err(SystemsPublicationCacheError::Mismatch(
            "actual archive entry byte count",
        ));
    }
    receipt.verify_entry_digest(name, Sha256::digest(&bytes).into())?;
    Ok(bytes)
}

fn entry_digest<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    expected: u64,
) -> Result<[u8; 32], SystemsPublicationCacheError> {
    let entry = archive.by_name(name)?;
    if entry.size() != expected {
        return Err(SystemsPublicationCacheError::Mismatch(
            "archive entry byte count",
        ));
    }
    let mut reader = entry.take(expected + 1);
    let mut writer = DigestWriter::new(io::sink());
    io::copy(&mut reader, &mut writer)?;
    if writer.bytes != expected {
        return Err(SystemsPublicationCacheError::Mismatch(
            "actual archive entry byte count",
        ));
    }
    Ok(writer.digest.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt_documents() -> (Value, Value) {
        let identity = json!({
            "publication_digest":[1], "semantic_digest":[2],
            "accepted_kerml_digest":[3], "systems_kpar":"fixture-kpar",
            "systems_source_content_set":[4], "operational_profile":"fixture-profile",
            "rule_set":"fixture-rules", "producer_registry_digest":[5],
            "producer_closure_digest":[6],
        });
        let mut bindings = identity.clone();
        let bindings_map = bindings.as_object_mut().unwrap();
        bindings_map.remove("publication_digest");
        bindings_map.insert(
            "accepted_systems_digest".into(),
            identity["publication_digest"].clone(),
        );
        bindings_map.insert("format".into(), json!("agq-sysml-accepted-bindings/1"));
        bindings_map.insert("source_content_set".into(), json!("fixture-sources"));
        (identity, bindings)
    }

    #[test]
    fn systems_receipt_schema_rejects_missing_or_mismatched_fields() {
        const FORMAT: &str = "agq-sysml-accepted-publication/1";
        let (identity, bindings) = receipt_documents();
        check_receipt_documents(FORMAT, "fixture-sources", &identity, &bindings).unwrap();
        assert!(
            check_receipt_documents("other-format", "fixture-sources", &identity, &bindings)
                .is_err()
        );
        assert!(check_receipt_documents(FORMAT, "other-sources", &identity, &bindings).is_err());
        for field in bindings.as_object().unwrap().keys() {
            let mut missing = bindings.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                check_receipt_documents(FORMAT, "fixture-sources", &identity, &missing).is_err(),
                "missing binding field {field}"
            );
            let mut changed = bindings.clone();
            changed[field] = json!("changed");
            assert!(
                check_receipt_documents(FORMAT, "fixture-sources", &identity, &changed).is_err(),
                "changed binding field {field}"
            );
        }
        for field in identity.as_object().unwrap().keys() {
            let mut missing = identity.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                check_receipt_documents(FORMAT, "fixture-sources", &missing, &bindings).is_err(),
                "missing receipt field {field}"
            );
            let mut changed = identity.clone();
            changed[field] = json!("changed");
            assert!(
                check_receipt_documents(FORMAT, "fixture-sources", &changed, &bindings).is_err(),
                "changed receipt field {field}"
            );
            let mut absent_both = bindings.clone();
            absent_both
                .as_object_mut()
                .unwrap()
                .remove(if field == "publication_digest" {
                    "accepted_systems_digest"
                } else {
                    field
                });
            assert!(
                check_receipt_documents(FORMAT, "fixture-sources", &missing, &absent_both).is_err(),
                "matching omissions {field} must not count as identity agreement"
            );
        }
    }

    fn archive(names: &[&str]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(vec![]));
        for name in names {
            writer
                .start_file(
                    *name,
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(b"{}").unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn systems_archive_rejects_missing_extra_alias_and_duplicate_entries() {
        assert!(
            check_entries(&mut ZipArchive::new(Cursor::new(archive(&ENTRIES))).unwrap()).is_ok()
        );
        for names in [
            vec!["closure.json", "facade.json"],
            vec![
                "closure.json",
                "facade.json",
                "kernel.jsonl",
                "receipt.json",
            ],
            vec!["closure.json", "facade.json", "nested/kernel.jsonl"],
            vec!["closure.json", "facade.json", "kernel.jsonl/"],
        ] {
            assert!(
                check_entries(&mut ZipArchive::new(Cursor::new(archive(&names))).unwrap()).is_err()
            );
        }
        // ZIP writers normally reject duplicate names. Change both the local
        // and central names of the equal-length kernel entry to a duplicate.
        let mut duplicate = archive(&ENTRIES);
        let name = b"kernel.jsonl";
        for index in 0..=duplicate.len() - name.len() {
            if &duplicate[index..index + name.len()] == name {
                duplicate[index..index + name.len()].copy_from_slice(b"closure.json");
            }
        }
        // Earlier decoder rejection also preserves the boundary.
        if let Ok(mut archive) = ZipArchive::new(Cursor::new(duplicate)) {
            assert!(check_entries(&mut archive).is_err());
        }
    }

    #[test]
    fn systems_archive_checks_declared_and_actual_entry_bounds() {
        let bytes = archive(&ENTRIES);
        let mut input = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(
            entry_digest(&mut input, "kernel.jsonl", 2).unwrap(),
            <[u8; 32]>::from(Sha256::digest(b"{}"))
        );
        assert!(entry_digest(&mut input, "kernel.jsonl", 1).is_err());
        assert!(entry_digest(&mut input, "kernel.jsonl", 3).is_err());
        assert!(ZipArchive::new(Cursor::new(&bytes[..bytes.len() / 2])).is_err());
    }

    fn metadata(sources: &VerifiedLibrarySet) -> FacadeMetadata {
        use SystemsPublicationFamily::*;
        FacadeMetadata {
            format: "agq-sysml-publication-facade/1".into(),
            source_content_set: sources.content_set_id().into(),
            roots: vec![ElementId::from_u128(1)],
            source_map: vec![],
            documents: sources
                .documents()
                .filter(|source| source.language() == LibraryLanguage::SysMl)
                .map(|source| DocumentMetadata {
                    path: source.path().into(),
                    document: source.document(),
                    sha256: source.sha256().into(),
                    profile: SysmlSyntaxProfile::OperationalV2.id().into(),
                    parsed: true,
                    byte_exact: true,
                    recovery_count: 0,
                    production_count: 1,
                    construction_gap: None,
                })
                .collect(),
            mandatory_references: SYSTEMS_MANDATORY_REFERENCE_ASSERTIONS,
            complete_references: SYSTEMS_MANDATORY_REFERENCE_ASSERTIONS,
            checked: [
                Syntax,
                CanonicalLowering,
                NamespacesImports,
                DefinitionUsage,
                AttributeItemPart,
                OccurrenceActionState,
                CalculationConstraintRequirementCase,
                PortConnectionInterfaceFlow,
                ViewMetadata,
                TypingSpecializationSubsettingRedefinition,
                MayTimeVary,
                StandardBindings,
                IdentityProvenance,
            ]
            .into_iter()
            .map(|family| (format!("{family:?}"), 1))
            .collect(),
        }
    }

    #[test]
    fn systems_metadata_rejects_incomplete_populations_and_stale_sources() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
        let good = metadata(&sources);
        good.validate(&sources, SysmlSyntaxProfile::OperationalV2)
            .unwrap();
        assert!(
            good.validate(&sources, SysmlSyntaxProfile::OperationalV3)
                .is_err()
        );
        let mut v3 = good.clone();
        for doc in &mut v3.documents {
            doc.profile = SysmlSyntaxProfile::OperationalV3.id().into();
        }
        v3.validate(&sources, SysmlSyntaxProfile::OperationalV3)
            .unwrap();
        assert!(
            v3.validate(&sources, SysmlSyntaxProfile::OperationalV2)
                .is_err()
        );
        v3.documents[0].profile = SysmlSyntaxProfile::OperationalV2.id().into();
        assert!(
            v3.validate(&sources, SysmlSyntaxProfile::OperationalV3)
                .is_err()
        );
        let mut omitted = good.clone();
        omitted.documents.pop();
        assert!(
            omitted
                .validate(&sources, SysmlSyntaxProfile::OperationalV2)
                .is_err()
        );
        let mut repeated = good.clone();
        repeated.documents[1] = repeated.documents[0].clone();
        assert!(
            repeated
                .validate(&sources, SysmlSyntaxProfile::OperationalV2)
                .is_err()
        );
        let mut stale = good.clone();
        stale.documents[0].sha256 = "0".repeat(64);
        assert!(
            stale
                .validate(&sources, SysmlSyntaxProfile::OperationalV2)
                .is_err()
        );
        let mut reference = good.clone();
        reference.complete_references -= 1;
        assert!(
            reference
                .validate(&sources, SysmlSyntaxProfile::OperationalV2)
                .is_err()
        );
        let mut family = good.clone();
        family.checked.remove("IdentityProvenance");
        assert!(
            family
                .validate(&sources, SysmlSyntaxProfile::OperationalV2)
                .is_err()
        );
        let mut duplicate_root = good;
        duplicate_root.roots.push(duplicate_root.roots[0]);
        assert!(
            duplicate_root
                .validate(&sources, SysmlSyntaxProfile::OperationalV2)
                .is_err()
        );
    }
}
