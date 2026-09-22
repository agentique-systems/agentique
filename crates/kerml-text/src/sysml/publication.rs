//! Acceptance of the exact Systems Library, independently of SysML conformance.
use super::{SystemsDocumentStatus, SystemsLibraryCandidate};
use crate::library::{
    CanonicalKermlStandardLibraries, LibraryLoadError, LibrarySourceMap, PendingLibraryReference,
};
use agq_kerml_semantics::{
    Completeness, Diagnostic, KerMlQueries, PublicationClosureOptions, PublicationCounters,
    PublicationFamily, PublicationOverlayError, PublicationStage, SemanticContextId,
    close_result_structure_on_overlay_with_extension,
};
use agq_kerml_syntax::production::SysmlSyntaxProfile;
use agq_kernel::{
    DocumentId, ElementId, Snapshot, SourceRevisionId, SyntaxNodeId,
    derived::{DerivedOverlay, PropertyState},
    provenance::{ByteRange, DeclaredOrigin, FactKey, Origin, SourceOrigin},
    value::Value,
};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrary, VerifiedLibrarySet};
use agq_sysml::{classes as sc, properties as sp};
use agq_sysml_semantics::{
    SYSML_SEMANTIC_CONTEXT_DOMAIN, StandardSysmlBindings, StandardSysmlRole, SysmlBaselineProfile,
    SysmlBindingError, SysmlContextError, SysmlDependencyContract, SysmlProducerExtension,
    SysmlQueries, SysmlQueryResult, SysmlSemanticContext, SysmlSemanticContextId,
    SystemsLibraryIdentity, sysml_producer_rule_ids,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

#[path = "publication_cache.rs"]
mod cache;
pub use cache::SystemsPublicationCacheError;

/// Publication capabilities, not the complete set of SysML validation rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemsPublicationFamily {
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
}

/// An exact formal target absent from the original Systems corpus. These are
/// unresolved decisions; no spelling correction or source alias is implied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemsAuthorityConflict {
    pub rule: &'static str,
    pub formal_target: &'static str,
    pub original_declaration: &'static str,
    pub subject: ElementId,
}

/// Mutually exclusive outcome for a mandatory source assertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemsReferenceStatus {
    Complete,
    Unresolved,
    Incomplete,
    Ambiguous,
    Invalid,
    EndpointMismatch,
}

#[derive(Clone, Debug)]
pub struct SystemsReferenceFailure {
    pub relationship: ElementId,
    pub status: SystemsReferenceStatus,
    pub candidates: Vec<ElementId>,
    pub stored: Vec<ElementId>,
}

/// Findings are computed internally; callers cannot provide acceptance labels.
#[derive(Clone, Debug)]
pub enum SystemsPublicationFinding {
    Identity(&'static str),
    Document {
        path: String,
        reason: String,
    },
    KernelObligations(usize),
    Producers {
        completeness: Completeness,
        converged: bool,
    },
    /// Outstanding final-frontier evidence retained when authority rejects the
    /// input before repeating strict producer work.
    ProducerDiagnostic(Diagnostic),
    Capability {
        family: SystemsPublicationFamily,
        diagnostic: Diagnostic,
    },
    Authority(SystemsAuthorityConflict),
    Reference(SystemsReferenceFailure),
    Binding(SysmlBindingError),
    Provenance(FactKey),
}

/// A report records scoped successes and all failed gates without conferring
/// publication or conformance on an unpublished construction overlay.
#[derive(Clone, Debug, Default)]
pub struct SystemsPublicationAudit {
    pub checked: BTreeMap<SystemsPublicationFamily, usize>,
    pub mandatory_references: usize,
    pub complete_references: usize,
    pub findings: Vec<SystemsPublicationFinding>,
}
impl SystemsPublicationAudit {
    fn checked(&mut self, family: SystemsPublicationFamily, count: usize) {
        *self.checked.entry(family).or_default() += count;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SystemsPublicationError {
    #[error(transparent)]
    Source(#[from] LibraryLoadError),
    #[error(transparent)]
    Context(#[from] SysmlContextError),
    #[error(transparent)]
    Overlay(#[from] PublicationOverlayError),
    #[error("Systems Library publication rejected: {} findings", .0.findings.len())]
    Rejected(Box<SystemsPublicationAudit>),
}

/// Immutable identity of both source interpretation and its resulting graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemsPublicationIdentity {
    pub dependencies: SysmlDependencyContract,
    pub semantic_digest: [u8; 32],
    pub publication_digest: [u8; 32],
}

/// Accepted Systems records retain the exact sealed KerML dependency by Arc.
/// Only `publish` constructs this facade, after every applicable gate passes.
pub struct CanonicalSysmlSystemsLibrary {
    overlay: Arc<DerivedOverlay>,
    producer_closure: Arc<agq_kerml_semantics::ProducerClosureCertificate>,
    accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
    bindings: StandardSysmlBindings,
    identity: SystemsPublicationIdentity,
    context: SysmlSemanticContextId,
    roots: Vec<ElementId>,
    source_map: LibrarySourceMap,
    documents: Vec<SystemsDocumentStatus>,
    audit: SystemsPublicationAudit,
    counters: PublicationCounters,
}

/// Source metadata survives acceptance preparation; no construction graph or
/// construction derivation is retained while the strict scheduler runs.
struct PublicationInputs {
    accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
    roots: Vec<ElementId>,
    source_map: LibrarySourceMap,
    documents: Vec<SystemsDocumentStatus>,
    references: Vec<PendingLibraryReference>,
}
impl PublicationInputs {
    fn consume(candidate: SystemsLibraryCandidate) -> Self {
        Self {
            accepted_kerml: candidate.accepted_kerml().clone(),
            roots: candidate.draft().roots().to_vec(),
            source_map: candidate.draft().source_map().clone(),
            documents: candidate.documents().to_vec(),
            references: candidate.draft().references().to_vec(),
        }
    }
}
impl CanonicalSysmlSystemsLibrary {
    /// Revalidate strict storage and rerun the combined scheduler on all local
    /// records, including existing generated outputs. Neither an earlier
    /// construction result nor a restricted population establishes acceptance.
    pub fn publish(
        mut candidate: SystemsLibraryCandidate,
        sources: &VerifiedLibrarySet,
        options: PublicationClosureOptions,
        batch_progress: impl FnMut(usize, usize, usize, usize),
        stage_progress: impl FnMut(&PublicationStage),
    ) -> Result<Self, SystemsPublicationError> {
        let mut audit = SystemsPublicationAudit::default();
        let profile = match candidate.syntax_profile() {
            SysmlSyntaxProfile::Published => SysmlBaselineProfile::Published,
            SysmlSyntaxProfile::OperationalV1 => SysmlBaselineProfile::OperationalV1,
            SysmlSyntaxProfile::OperationalV2 => SysmlBaselineProfile::OperationalV2,
        };
        let identity = SystemsLibraryIdentity::pinned(SystemsLibraryIdentity::SOURCE_CONTENT_SET);
        let empty_bindings = StandardSysmlBindings::unbound(identity.clone());
        let mut contract =
            SysmlDependencyContract::checked_in_for_profile(&empty_bindings, profile)?;
        audit_inputs(&candidate, sources, &contract, &mut audit);
        if options.initial_subjects.is_some() {
            audit.findings.push(SystemsPublicationFinding::Identity(
                "restricted publication population",
            ));
        }
        let obligations = candidate.draft().candidate().obligations().len();
        if obligations != 0 {
            audit
                .findings
                .push(SystemsPublicationFinding::KernelObligations(obligations));
        }
        audit_prior_authority(candidate.production(), &mut audit);
        if !audit.findings.is_empty() {
            return Err(SystemsPublicationError::Rejected(Box::new(audit)));
        }
        let declared = candidate.draft().strict_snapshot()?;
        let library = sources
            .libraries()
            .get(&identity.library)
            .expect("checked exact source identity");
        audit_source_provenance(
            &declared,
            candidate.draft().source_map(),
            library,
            candidate.syntax_profile(),
            &mut audit,
        );
        if !audit.findings.is_empty() {
            return Err(SystemsPublicationError::Rejected(Box::new(audit)));
        }
        let candidate_queries = candidate.queries()?;
        let producer_bindings = StandardSysmlBindings::validate(
            candidate_queries.model(),
            &candidate_queries,
            identity.clone(),
            candidate.draft().roots(),
            StandardSysmlRole::ALL,
        )
        .and_then(|bindings| {
            bindings.with_verified_sources(library, candidate.draft().source_map())
        });
        let producer_bindings = match producer_bindings {
            Ok(bindings) => bindings,
            Err(error) => {
                audit
                    .findings
                    .push(SystemsPublicationFinding::Binding(error));
                return Err(SystemsPublicationError::Rejected(Box::new(audit)));
            }
        };
        drop(candidate_queries);
        // Strictly revalidate the exact existing graph, retaining derived
        // provenance and the protected dependency. The fresh strict scheduler
        // still evaluates every family under its final bindings and contract.
        let initial_overlay = match candidate.draft.take_semantic_candidate() {
            Some(overlay) => overlay
                .revalidate(declared.clone())
                .map_err(PublicationOverlayError::from)?,
            None => agq_kernel::derived::DerivationBuilder::new(declared.clone())
                .build()
                .map_err(PublicationOverlayError::from)?,
        };
        let inputs = PublicationInputs::consume(candidate);
        contract.standard_bindings = producer_bindings.targets().clone();
        let contract_digest = contract.context_identity_digest();
        let roots: Vec<_> = inputs
            .roots
            .iter()
            .chain(inputs.accepted_kerml.roots())
            .copied()
            .collect();
        let extension =
            SysmlProducerExtension::new(profile, producer_bindings.clone(), roots.clone());
        let closure = close_result_structure_on_overlay_with_extension(
            initial_overlay,
            options,
            |overlay| {
                inputs
                    .accepted_kerml
                    .complete_overlay()
                    .project_overlay_context(overlay, &inputs.roots)
                    .and_then(|context| {
                        context.with_naming_extension(
                            SYSML_SEMANTIC_CONTEXT_DOMAIN,
                            contract_digest,
                            Arc::new(agq_sysml_semantics::SysmlNamingExtension),
                        )
                    })
                    .map_err(PublicationOverlayError::Context)
            },
            &extension,
            batch_progress,
            stage_progress,
        )?;
        if closure.completeness != Completeness::Complete || !closure.converged {
            audit.findings.push(SystemsPublicationFinding::Producers {
                completeness: closure.completeness,
                converged: closure.converged,
            });
        }
        let Some(certificate) = closure.certificate.clone() else {
            audit.findings.push(SystemsPublicationFinding::Identity(
                "scheduler closure certificate missing",
            ));
            return Err(SystemsPublicationError::Rejected(Box::new(audit)));
        };
        let q = KerMlQueries::new(
            inputs
                .accepted_kerml
                .complete_overlay()
                .project_overlay_context(&closure.overlay, &inputs.roots)
                .and_then(|context| {
                    context.with_naming_extension(
                        SYSML_SEMANTIC_CONTEXT_DOMAIN,
                        contract_digest,
                        Arc::new(agq_sysml_semantics::SysmlNamingExtension),
                    )
                })
                .and_then(|context| {
                    context.with_producer_registry_digest(certificate.registry_digest())
                })
                .and_then(|context| context.with_producer_closure(certificate.clone()))
                .map_err(PublicationOverlayError::Context)?,
        );
        if q.context().descriptor_digest != contract.combined_descriptor_digest {
            audit.findings.push(SystemsPublicationFinding::Identity(
                "combined descriptor graph",
            ));
        }
        let local: Vec<_> = q
            .model()
            .elements()
            .filter(|record| !declared.is_dependency_element(record.id()))
            .map(|record| record.id())
            .collect();
        let kerml = q.audit_publication_capabilities_with_rules(
            local.iter().copied(),
            sysml_producer_rule_ids(profile),
        );
        for (family, count) in kerml.checked_items {
            audit.checked(map_family(family), count);
        }
        for (family, diagnostics) in kerml.failures {
            audit
                .findings
                .extend(diagnostics.into_iter().map(|diagnostic| {
                    SystemsPublicationFinding::Capability {
                        family: map_family(family),
                        diagnostic,
                    }
                }));
        }
        audit.findings.extend(
            final_authority_conflicts(&closure.stages)
                .into_iter()
                .map(SystemsPublicationFinding::Authority),
        );
        audit_references(&q, &inputs.references, &mut audit);
        let bindings = StandardSysmlBindings::validate(
            q.model(),
            &q,
            identity,
            &inputs.roots,
            StandardSysmlRole::ALL,
        )
        .and_then(|bindings| bindings.with_verified_sources(library, &inputs.source_map));
        let bindings = match bindings {
            Ok(bindings) => {
                if bindings.targets() != producer_bindings.targets() {
                    audit.findings.push(SystemsPublicationFinding::Identity(
                        "producer and accepted Systems binding targets",
                    ));
                }
                audit.checked(
                    SystemsPublicationFamily::StandardBindings,
                    bindings.targets().len(),
                );
                Some(bindings)
            }
            Err(error) => {
                audit
                    .findings
                    .push(SystemsPublicationFinding::Binding(error));
                None
            }
        };
        if let Some(bindings) = &bindings {
            // Typed current-graph queries validate narrowed SysML domains. The
            // separate producer gate is responsible for the closure claim.
            let context = SysmlSemanticContext::for_overlay(
                &closure.overlay,
                inputs.accepted_kerml.complete_overlay(),
                &inputs.roots,
                &contract,
                bindings.clone(),
            )?
            .with_producer_closure(certificate.clone())?;
            for batch in local.chunks(32) {
                audit_sysml_population(&SysmlQueries::new(context.fork()), batch, &mut audit);
            }
        }
        if !audit.findings.is_empty() {
            return Err(SystemsPublicationError::Rejected(Box::new(audit)));
        }
        let bindings = bindings.expect("binding failure is an acceptance finding");
        let semantic_context = q.context().clone();
        let publication_identity = publication_identity(contract.clone(), &semantic_context);
        let context = SysmlSemanticContextId {
            kerml: semantic_context,
            dependencies: contract,
            metamodel_version: agq_sysml_semantics::SYSML_METAMODEL_VERSION,
        };
        drop(q);
        Ok(Self {
            overlay: Arc::new(closure.overlay),
            producer_closure: certificate,
            accepted_kerml: inputs.accepted_kerml,
            bindings,
            identity: publication_identity,
            context,
            roots: inputs.roots,
            source_map: inputs.source_map,
            documents: inputs.documents,
            audit,
            counters: closure.counters,
        })
    }
    pub fn declared(&self) -> &Snapshot {
        self.overlay.declared()
    }
    pub fn overlay(&self) -> &DerivedOverlay {
        &self.overlay
    }
    /// Immutable scheduler evidence for this exact accepted graph and registry.
    pub fn producer_closure(&self) -> &Arc<agq_kerml_semantics::ProducerClosureCertificate> {
        &self.producer_closure
    }
    pub fn accepted_kerml(&self) -> &Arc<CanonicalKermlStandardLibraries> {
        &self.accepted_kerml
    }
    pub fn bindings(&self) -> &StandardSysmlBindings {
        &self.bindings
    }
    pub fn identity(&self) -> &SystemsPublicationIdentity {
        &self.identity
    }
    pub fn context(&self) -> &SysmlSemanticContextId {
        &self.context
    }
    pub fn roots(&self) -> &[ElementId] {
        &self.roots
    }
    pub fn source_map(&self) -> &LibrarySourceMap {
        &self.source_map
    }
    pub fn documents(&self) -> &[SystemsDocumentStatus] {
        &self.documents
    }
    pub fn audit(&self) -> &SystemsPublicationAudit {
        &self.audit
    }
    pub fn counters(&self) -> &PublicationCounters {
        &self.counters
    }
    pub fn semantic_digest(&self) -> [u8; 32] {
        self.identity.semantic_digest
    }
    pub fn publication_digest(&self) -> [u8; 32] {
        self.identity.publication_digest
    }
    /// Independent authored history sharing both immutable library layers.
    pub fn project_snapshot(&self) -> Snapshot {
        Snapshot::with_immutable_dependency(self.overlay.clone())
    }
    pub fn queries(&self) -> KerMlQueries<'_> {
        KerMlQueries::new(
            self.accepted_kerml
                .complete_overlay()
                .project_overlay_context(&self.overlay, &self.roots)
                .expect("accepted immutable Systems dependency")
                .with_naming_extension(
                    SYSML_SEMANTIC_CONTEXT_DOMAIN,
                    self.identity.dependencies.context_identity_digest(),
                    Arc::new(agq_sysml_semantics::SysmlNamingExtension),
                )
                .and_then(|context| {
                    context.with_producer_registry_digest(self.producer_closure.registry_digest())
                })
                .and_then(|context| context.with_producer_closure(self.producer_closure.clone()))
                .expect("accepted SysML context identity and producer closure"),
        )
    }
}
impl std::fmt::Debug for CanonicalSysmlSystemsLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanonicalSysmlSystemsLibrary")
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

fn audit_inputs(
    candidate: &SystemsLibraryCandidate,
    sources: &VerifiedLibrarySet,
    contract: &SysmlDependencyContract,
    audit: &mut SystemsPublicationAudit,
) {
    audit_reference_population(candidate.draft().references(), audit);
    let accepted = candidate.accepted_kerml();
    for (valid, field) in [
        (
            candidate.dependency_contract() == contract,
            "candidate dependency contract",
        ),
        (
            candidate.syntax_profile() == SysmlSyntaxProfile::OperationalV2,
            "SysML syntax profile",
        ),
        (
            accepted.profile() == agq_kerml::BaselineProfile::OPERATIONAL_V9,
            "KerML operational profile",
        ),
        (
            accepted.semantic_digest() == contract.kerml_publication_digest,
            "accepted KerML publication digest",
        ),
        (
            accepted.context().descriptor_digest == contract.kerml_descriptor_digest,
            "KerML descriptor graph",
        ),
        (
            accepted.context().rule_set_version == contract.kerml_rule_set,
            "KerML rule set",
        ),
        (
            accepted.library_set() == &contract.kerml_libraries,
            "KerML library set",
        ),
        (
            sources.content_set_id() == contract.kerml_source_content_set
                && candidate.source_content_set() == sources.content_set_id()
                && accepted.source_content_set() == sources.content_set_id(),
            "source content set",
        ),
        (
            sources
                .libraries()
                .get(&SystemsLibraryIdentity::LIBRARY)
                .is_some_and(|library| {
                    library.archive_sha256() == SystemsLibraryIdentity::ARTIFACT_SHA256
                }),
            "Systems KPAR identity",
        ),
    ] {
        if !valid {
            audit
                .findings
                .push(SystemsPublicationFinding::Identity(field));
        }
    }
    let original: BTreeMap<_, _> = sources
        .documents()
        .filter(|source| source.language() == LibraryLanguage::SysMl)
        .map(|source| (source.document(), source))
        .collect();
    let actual: std::collections::BTreeSet<_> = candidate
        .documents()
        .iter()
        .map(|doc| doc.document)
        .collect();
    if candidate.documents().len() != 21
        || original.len() != 21
        || actual != original.keys().copied().collect()
    {
        audit.findings.push(SystemsPublicationFinding::Identity(
            "exact 21 Systems documents",
        ));
    }
    for status in candidate.documents() {
        let valid = status.profile == SysmlSyntaxProfile::OperationalV2
            && status.parsed
            && status.byte_exact
            && status.recovery_count == 0
            && status.construction_gap.is_none()
            && original.get(&status.document).is_some_and(|source| {
                source.path() == status.path && source.sha256() == status.source_sha256
            });
        if valid {
            audit.checked(SystemsPublicationFamily::Syntax, 1);
            audit.checked(SystemsPublicationFamily::CanonicalLowering, 1);
        } else {
            audit.findings.push(SystemsPublicationFinding::Document {
                path: status.path.clone(),
                reason: status.construction_gap.clone().unwrap_or_else(|| {
                    "strict operational parse or original byte identity failed".into()
                }),
            });
        }
    }
}

/// Corpus acceptance baseline for the exact pinned 21-document source set.
/// This is not a limit on authored SysML projects or a language conformance rule.
const SYSTEMS_MANDATORY_REFERENCE_ASSERTIONS: usize = 1_327;

fn audit_reference_population(
    references: &[PendingLibraryReference],
    audit: &mut SystemsPublicationAudit,
) {
    if references.len() != SYSTEMS_MANDATORY_REFERENCE_ASSERTIONS {
        audit.findings.push(SystemsPublicationFinding::Identity(
            "exact 1,327 Systems mandatory reference assertions",
        ));
    }
    // One relationship/property pair can carry distinct source assertions.
    // Preserve that distinction; only a duplicate complete request, including
    // exact source identity/range, could falsely pad the audited population.
    let distinct: std::collections::BTreeSet<_> = references
        .iter()
        .map(|reference| {
            (
                reference.relationship,
                reference.property,
                reference.expected,
                reference.name.absolute,
                &reference.name.segments,
                reference.membership_target,
                reference.executable_expression,
                reference.origin.document,
                reference.origin.revision,
                reference.origin.range.start(),
                reference.origin.range.end(),
                reference.origin.syntax_node,
            )
        })
        .collect();
    if distinct.len() != references.len() {
        audit.findings.push(SystemsPublicationFinding::Identity(
            "distinct Systems mandatory source assertions",
        ));
    }
}

fn audit_source_provenance(
    declared: &Snapshot,
    source_map: &LibrarySourceMap,
    library: &VerifiedLibrary,
    profile: SysmlSyntaxProfile,
    audit: &mut SystemsPublicationAudit,
) {
    let expected = Origin::Declared(DeclaredOrigin::StandardLibrary {
        library: library.id(),
    });
    let mut source_nodes = SourceNodes::new();
    for source in library.documents() {
        let syntax = agq_kerml_syntax::production::parse_sysml_with_profile(
            profile,
            source.document(),
            source.revision(),
            source.source(),
            Default::default(),
        );
        match syntax {
            Ok(syntax) if syntax.is_complete() => {
                source_nodes.insert(
                    source.document(),
                    (
                        source.revision(),
                        syntax
                            .nodes()
                            .map(|node| (node.id(), node.range()))
                            .collect(),
                    ),
                );
            }
            _ => audit.findings.push(SystemsPublicationFinding::Document {
                path: source.path().into(),
                reason: "Source provenance requires the exact complete operational syntax arena"
                    .into(),
            }),
        }
    }
    for record in declared
        .model()
        .elements()
        .filter(|record| !declared.is_dependency_element(record.id()))
    {
        audit_source_fact(
            FactKey::Element(record.id()),
            record.origin(),
            &expected,
            source_map,
            &source_nodes,
            audit,
        );
        for (property, slot) in record.slots() {
            audit_source_fact(
                FactKey::Property {
                    element: record.id(),
                    property,
                },
                slot.origin(),
                &expected,
                source_map,
                &source_nodes,
                audit,
            );
        }
    }
    for occurrence in declared
        .model()
        .association_occurrences()
        .filter(|occurrence| {
            declared.immutable_dependency().is_none_or(|dependency| {
                dependency
                    .model()
                    .association_occurrence(occurrence.id())
                    .is_none()
            })
        })
    {
        audit_source_fact(
            FactKey::AssociationOccurrence(occurrence.id()),
            occurrence.origin(),
            &expected,
            source_map,
            &source_nodes,
            audit,
        );
    }
}

type SourceNodes = BTreeMap<DocumentId, (SourceRevisionId, BTreeMap<SyntaxNodeId, ByteRange>)>;

fn valid_source_origin(origin: &SourceOrigin, source_nodes: &SourceNodes) -> bool {
    source_nodes
        .get(&origin.document)
        .is_some_and(|(revision, nodes)| {
            *revision == origin.revision
                && origin
                    .syntax_node
                    .is_some_and(|node| nodes.get(&node) == Some(&origin.range))
        })
}

fn audit_source_fact(
    fact: FactKey,
    origin: &Origin,
    expected: &Origin,
    source_map: &LibrarySourceMap,
    source_nodes: &SourceNodes,
    audit: &mut SystemsPublicationAudit,
) {
    if origin != expected
        || !source_map
            .get(&fact)
            .is_some_and(|s| valid_source_origin(s, source_nodes))
    {
        audit
            .findings
            .push(SystemsPublicationFinding::Provenance(fact));
    } else {
        audit.checked(SystemsPublicationFamily::IdentityProvenance, 1);
    }
}

fn audit_sysml_population<'m>(
    q: &SysmlQueries<'m>,
    subjects: &[ElementId],
    audit: &mut SystemsPublicationAudit,
) {
    let profile = q.context().dependencies.sysml_profile;
    for &subject in subjects {
        let class = q
            .model()
            .element(subject)
            .expect("audited local subject")
            .metaclass();
        let is = |base| {
            q.model()
                .registry()
                .is_subtype(class, base)
                .unwrap_or(false)
        };
        let mut families = Vec::new();
        for (family, classes) in [
            (
                SystemsPublicationFamily::DefinitionUsage,
                &[sc::DEFINITION, sc::USAGE][..],
            ),
            (
                SystemsPublicationFamily::AttributeItemPart,
                &[
                    sc::ATTRIBUTE_DEFINITION,
                    sc::ATTRIBUTE_USAGE,
                    sc::ITEM_DEFINITION,
                    sc::ITEM_USAGE,
                    sc::PART_DEFINITION,
                    sc::PART_USAGE,
                ][..],
            ),
            (
                SystemsPublicationFamily::OccurrenceActionState,
                &[
                    sc::OCCURRENCE_DEFINITION,
                    sc::OCCURRENCE_USAGE,
                    sc::ACTION_DEFINITION,
                    sc::ACTION_USAGE,
                    sc::STATE_DEFINITION,
                    sc::STATE_USAGE,
                ][..],
            ),
            (
                SystemsPublicationFamily::CalculationConstraintRequirementCase,
                &[
                    sc::CALCULATION_DEFINITION,
                    sc::CALCULATION_USAGE,
                    sc::CONSTRAINT_DEFINITION,
                    sc::CONSTRAINT_USAGE,
                    sc::REQUIREMENT_DEFINITION,
                    sc::REQUIREMENT_USAGE,
                    sc::CASE_DEFINITION,
                    sc::CASE_USAGE,
                ][..],
            ),
            (
                SystemsPublicationFamily::PortConnectionInterfaceFlow,
                &[
                    sc::PORT_DEFINITION,
                    sc::PORT_USAGE,
                    sc::CONNECTION_DEFINITION,
                    sc::CONNECTION_USAGE,
                    sc::INTERFACE_DEFINITION,
                    sc::INTERFACE_USAGE,
                    sc::FLOW_DEFINITION,
                    sc::FLOW_USAGE,
                ][..],
            ),
            (
                SystemsPublicationFamily::ViewMetadata,
                &[
                    sc::VIEW_DEFINITION,
                    sc::VIEW_USAGE,
                    sc::VIEWPOINT_DEFINITION,
                    sc::VIEWPOINT_USAGE,
                    sc::METADATA_DEFINITION,
                    sc::METADATA_USAGE,
                    sc::ENUMERATION_DEFINITION,
                    sc::ENUMERATION_USAGE,
                    sc::RENDERING_DEFINITION,
                    sc::RENDERING_USAGE,
                ][..],
            ),
        ] {
            if classes.iter().any(|&class| is(class)) {
                families.push(family);
            }
        }
        if is(sc::DEFINITION) || is(sc::USAGE) {
            audit_typed_answer(
                audit,
                &families,
                subject,
                "effective names",
                q.effective_names(subject),
            );
            audit_typed_answer(
                audit,
                &families,
                subject,
                "current effective usages",
                q.current_effective_usages(subject),
            );
        }
        if is(sc::DEFINITION) {
            audit_typed_answer(
                audit,
                &families,
                subject,
                "direct specializations",
                q.direct_specializations(subject),
            );
        }
        if is(sc::USAGE) {
            audit_typed_answer(
                audit,
                &families,
                subject,
                "current usage types",
                q.current_usage_types(subject),
            );
            let actual = q.model().property_state(subject, sp::USAGE_MAY_TIME_VARY);
            let valid = matches!(&actual, Ok(PropertyState::Computed(slot))
                if matches!(slot.value(), agq_kernel::value::SlotValue::Scalar(Value::Boolean(_)))
                && matches!(slot.origin(), Origin::Derived(proof)
                    if proof.rule == profile.rule_id("deriveUsageMayTimeVary")));
            audit.checked(SystemsPublicationFamily::MayTimeVary, 1);
            if !valid {
                audit.findings.push(SystemsPublicationFinding::Capability {
                    family: SystemsPublicationFamily::MayTimeVary,
                    diagnostic: Diagnostic {
                        code: "SQ_PUBLICATION_MAY_TIME_VARY",
                        subject,
                        message:
                            "Complete mayTimeVary producer evidence is absent from canonical storage"
                                .into(),
                    },
                });
            }
        }
        for (applies, name, query) in [
            (
                sc::ATTRIBUTE_USAGE,
                "current attribute definitions",
                SysmlQueries::current_attribute_definitions
                    as fn(&SysmlQueries<'m>, ElementId) -> _,
            ),
            (
                sc::ITEM_USAGE,
                "current item definitions",
                SysmlQueries::current_item_definitions,
            ),
            (
                sc::PART_USAGE,
                "current part definitions",
                SysmlQueries::current_part_definitions,
            ),
            (
                sc::PORT_USAGE,
                "current port definitions",
                SysmlQueries::current_port_definitions,
            ),
            (
                sc::CONNECTOR_AS_USAGE,
                "current connection related features",
                SysmlQueries::current_connection_related_features,
            ),
        ] {
            if is(applies) {
                audit_typed_answer(audit, &families, subject, name, query(q, subject));
            }
        }
    }
}

fn audit_typed_answer<T>(
    audit: &mut SystemsPublicationAudit,
    families: &[SystemsPublicationFamily],
    subject: ElementId,
    operation: &'static str,
    answer: SysmlQueryResult<T>,
) {
    let completeness = answer.completeness();
    if completeness == Completeness::Complete {
        for &family in families {
            audit.checked(family, 1);
        }
        return;
    }
    let mut diagnostics = answer.diagnostics;
    diagnostics.extend(answer.kerml.diagnostics);
    for query in answer.supporting_queries {
        diagnostics.extend(query.diagnostics);
    }
    for query in answer.supporting_names {
        diagnostics.extend(query.diagnostics);
    }
    for query in answer.observations.into_values() {
        diagnostics.extend(query.diagnostics);
    }
    diagnostics.insert(Diagnostic {
        code: "SQ_PUBLICATION_TYPED_QUERY",
        subject,
        message: format!(
            "{operation} is {completeness:?}; pending implications: {:?}",
            answer.pending
        ),
    });
    for &family in families {
        audit.findings.extend(
            diagnostics
                .iter()
                .cloned()
                .map(|diagnostic| SystemsPublicationFinding::Capability { family, diagnostic }),
        );
    }
}

fn authority_conflict(
    rule: &'static str,
    subject: ElementId,
    diagnostic: &Diagnostic,
) -> Option<SystemsAuthorityConflict> {
    let (formal_target, original_declaration) = match rule {
        "checkViewpointDefinitionSpecialization" => ("Views::Viewpoint", "Views::ViewpointCheck"),
        "checkViewpointUsageSpecialization" => ("Views::viewpoints", "Views::viewpointChecks"),
        "checkConnectionDefinitionBinarySpecialization" => (
            "Connections::BinaryConnections",
            "Connections::BinaryConnection",
        ),
        _ => return None,
    };
    (diagnostic.code == "SQ_TARGET_MISSING"
        && diagnostic.message == format!("Exact formal target {formal_target} is unavailable"))
    .then_some(SystemsAuthorityConflict {
        rule,
        formal_target,
        original_declaration,
        subject,
    })
}

pub(super) fn final_authority_conflicts(
    stages: &[PublicationStage],
) -> Vec<SystemsAuthorityConflict> {
    stages
        .last()
        .into_iter()
        .flat_map(|last| &last.diagnostics)
        .flat_map(|diagnostic| {
            [
                "checkViewpointDefinitionSpecialization",
                "checkViewpointUsageSpecialization",
                "checkConnectionDefinitionBinarySpecialization",
            ]
            .into_iter()
            .filter_map(|rule| authority_conflict(rule, diagnostic.subject, diagnostic))
        })
        .collect()
}

fn audit_prior_authority(
    production: Option<&super::SystemsConstructionProduction>,
    audit: &mut SystemsPublicationAudit,
) {
    let Some(production) = production else { return };
    let Some(last) = production.stages.last() else {
        return;
    };
    let conflicts = final_authority_conflicts(&production.stages);
    if conflicts.is_empty() {
        return;
    }
    audit.findings.extend(
        conflicts
            .into_iter()
            .map(SystemsPublicationFinding::Authority),
    );
    audit.findings.push(SystemsPublicationFinding::Producers {
        completeness: production.completeness,
        converged: production.converged,
    });
    // A known standards conflict does not relabel ordinary implementation gaps.
    // Retain every outstanding producer diagnostic in its original form.
    audit.findings.extend(
        last.diagnostics
            .iter()
            .cloned()
            .map(SystemsPublicationFinding::ProducerDiagnostic),
    );
}

fn audit_references(
    q: &KerMlQueries<'_>,
    references: &[PendingLibraryReference],
    audit: &mut SystemsPublicationAudit,
) {
    for batch in references.chunks(32) {
        let status_queries = q.status_queries();
        for reference in batch {
            let answer = status_queries.lookup_relationship_target(
                reference.relationship,
                reference.property,
                &reference.name,
            );
            let targets: Vec<_> = answer
                .value
                .iter()
                .map(|target| {
                    if reference.membership_target {
                        target.membership
                    } else {
                        target.element
                    }
                })
                .collect();
            let stored: Vec<_> = q
                .model()
                .navigation_slot(reference.relationship, reference.property)
                .into_iter()
                .flat_map(|slot| slot.value().values())
                .filter_map(|value| {
                    if let Value::Reference(id) = value {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();
            let kind_valid = targets.first().is_some_and(|id| {
                q.model().element(*id).is_some_and(|record| {
                    q.model()
                        .registry()
                        .is_subtype(record.metaclass(), reference.expected)
                        .unwrap_or(false)
                })
            });
            let status = reference_status(answer.completeness, &targets, &stored, kind_valid);
            audit.mandatory_references += 1;
            if status == SystemsReferenceStatus::Complete {
                audit.complete_references += 1;
            } else {
                audit.findings.push(SystemsPublicationFinding::Reference(
                    SystemsReferenceFailure {
                        relationship: reference.relationship,
                        status,
                        candidates: targets,
                        stored,
                    },
                ));
            }
        }
    }
}

fn reference_status(
    completeness: Completeness,
    candidates: &[ElementId],
    stored: &[ElementId],
    kind_valid: bool,
) -> SystemsReferenceStatus {
    match completeness {
        Completeness::Incomplete => SystemsReferenceStatus::Incomplete,
        Completeness::Invalid => SystemsReferenceStatus::Invalid,
        Completeness::Complete => match candidates.len() {
            0 => SystemsReferenceStatus::Unresolved,
            1 if !kind_valid => SystemsReferenceStatus::Invalid,
            1 if candidates != stored => SystemsReferenceStatus::EndpointMismatch,
            1 => SystemsReferenceStatus::Complete,
            _ => SystemsReferenceStatus::Ambiguous,
        },
    }
}

fn map_family(family: PublicationFamily) -> SystemsPublicationFamily {
    match family {
        PublicationFamily::NamespaceImports | PublicationFamily::EffectiveMembership => {
            SystemsPublicationFamily::NamespacesImports
        }
        PublicationFamily::StandardBindings => SystemsPublicationFamily::StandardBindings,
        PublicationFamily::IdentityProvenance => SystemsPublicationFamily::IdentityProvenance,
        PublicationFamily::ConnectorsAssociations => {
            SystemsPublicationFamily::PortConnectionInterfaceFlow
        }
        PublicationFamily::Specialization
        | PublicationFamily::Typing
        | PublicationFamily::Featuring
        | PublicationFamily::FeatureChains
        | PublicationFamily::CrossFeatures
        | PublicationFamily::ExpressionResults
        | PublicationFamily::FeatureValues
        | PublicationFamily::Multiplicity => {
            SystemsPublicationFamily::TypingSpecializationSubsettingRedefinition
        }
    }
}

fn publication_identity(
    dependencies: SysmlDependencyContract,
    context: &SemanticContextId,
) -> SystemsPublicationIdentity {
    let bindings: Vec<_> = dependencies
        .standard_bindings
        .iter()
        .map(|(role, id)| serde_json::json!({"path":role.specification().0,"element":id}))
        .collect();
    let identity = serde_json::json!({
        "format":"agentique-sysml-systems-publication/1",
        "dependency_context_digest":dependencies.context_identity_digest(),
        "kerml_publication_digest":dependencies.kerml_publication_digest,
        "kerml_profile":dependencies.kerml_profile,
        "kerml_rule_set":dependencies.kerml_rule_set,
        "kerml_descriptor_graph":dependencies.kerml_descriptor_digest,
        "source_content_set":dependencies.kerml_source_content_set,
        "sysml_profile":dependencies.sysml_profile.id(),
        "sysml_rule_set":dependencies.sysml_rule_set,
        "sysml_descriptor_graph":dependencies.combined_descriptor_digest,
        "grammar_compatibility_manifest":dependencies.grammar_compatibility_manifest_digest,
        "semantic_correction_manifest":dependencies.semantic_correction_manifest_digest,
        "systems_library":dependencies.systems_library.library,
        "systems_kpar":dependencies.systems_library.artifact_sha256,
        "systems_source_content_set":dependencies.systems_library.source_content_set,
        "bindings":bindings,
        "semantic_digest":context.model_digest,
        "producer_registry_digest":context.producer_registry_digest,
        "producer_closure_digest":context.producer_closure_digest,
    });
    SystemsPublicationIdentity {
        dependencies,
        semantic_digest: context.model_digest,
        publication_digest: Sha256::digest(
            serde_json::to_vec(&identity).expect("canonical identity JSON"),
        )
        .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_reference_population_rejects_omissions_and_duplicate_padding() {
        let references: Vec<_> = (0..SYSTEMS_MANDATORY_REFERENCE_ASSERTIONS)
            .map(|index| PendingLibraryReference {
                relationship: ElementId::from_u128(index as u128 + 1),
                property: agq_kerml::properties::SPECIALIZATION_GENERAL,
                expected: agq_kerml::classes::TYPE,
                name: agq_kerml_semantics::QualifiedName {
                    absolute: false,
                    segments: vec!["Target".into()],
                },
                membership_target: false,
                executable_expression: false,
                origin: SourceOrigin {
                    document: DocumentId::from_u128(1),
                    revision: SourceRevisionId::from_u128(2),
                    range: ByteRange::new(index as u64, index as u64 + 1).unwrap(),
                    syntax_node: Some(SyntaxNodeId::from_u128(index as u128 + 1)),
                },
            })
            .collect();
        let audit = |population: &[PendingLibraryReference]| {
            let mut audit = SystemsPublicationAudit::default();
            audit_reference_population(population, &mut audit);
            audit.findings
        };
        assert!(audit(&references).is_empty());
        assert!(matches!(
            audit(&references[..references.len() - 1]).as_slice(),
            [SystemsPublicationFinding::Identity(
                "exact 1,327 Systems mandatory reference assertions"
            )]
        ));
        let mut duplicated = references.clone();
        duplicated[1] = duplicated[0].clone();
        assert!(matches!(
            audit(&duplicated).as_slice(),
            [SystemsPublicationFinding::Identity(
                "distinct Systems mandatory source assertions"
            )]
        ));
        let mut distinct_source = references.clone();
        distinct_source[1].relationship = distinct_source[0].relationship;
        assert!(
            audit(&distinct_source).is_empty(),
            "different source assertions on one property remain distinct"
        );
    }
    #[test]
    fn authority_preflight_retains_ordinary_failures_and_ignores_superseded_frontiers() {
        let missing = Diagnostic {
            code: "SQ_TARGET_MISSING",
            subject: ElementId::from_u128(1),
            message: "Exact formal target Views::Viewpoint is unavailable".into(),
        };
        let ordinary = Diagnostic {
            code: "KQ_PENDING_SUPERTYPES",
            subject: ElementId::from_u128(2),
            message: "Pending ordinary producer".into(),
        };
        let mut production = super::super::SystemsConstructionProduction {
            final_predicates: true,
            completeness: Completeness::Incomplete,
            converged: true,
            counters: Default::default(),
            stages: vec![PublicationStage {
                stratum: agq_kerml_semantics::ResultStructureStratum::Structural,
                counters: Default::default(),
                stage: 0,
                input_elements: 0,
                added_elements: 0,
                added_occurrences: 0,
                completeness: Completeness::Incomplete,
                diagnostics: [missing, ordinary.clone()].into_iter().collect(),
            }],
        };
        let mut audit = SystemsPublicationAudit::default();
        audit_prior_authority(Some(&production), &mut audit);
        assert_eq!(final_authority_conflicts(&production.stages).len(), 1);
        assert_eq!(
            audit
                .findings
                .iter()
                .filter(|f| matches!(f, SystemsPublicationFinding::Authority(_)))
                .count(),
            1
        );
        assert!(audit.findings.iter().any(
            |f| matches!(f, SystemsPublicationFinding::ProducerDiagnostic(d) if d == &ordinary)
        ));
        let mut last = production.stages[0].clone();
        last.stage = 1;
        last.diagnostics = [ordinary].into_iter().collect();
        production.stages.push(last);
        assert!(
            final_authority_conflicts(&production.stages).is_empty(),
            "strict final acceptance must ignore a superseded authority diagnostic"
        );
        assert!(final_authority_conflicts(&[]).is_empty());
        let mut audit = SystemsPublicationAudit::default();
        audit_prior_authority(Some(&production), &mut audit);
        assert!(
            audit.findings.is_empty(),
            "a transient failure is not a final authority witness"
        );
    }
    #[test]
    fn source_gate_requires_exact_node_revision_range_and_origin_for_every_fact_kind() {
        let document = DocumentId::from_u128(1);
        let revision = SourceRevisionId::from_u128(2);
        let node = SyntaxNodeId::from_u128(3);
        let range = ByteRange::new(4, 8).unwrap();
        let source = SourceOrigin {
            document,
            revision,
            range,
            syntax_node: Some(node),
        };
        let nodes = SourceNodes::from([(document, (revision, BTreeMap::from([(node, range)])))]);
        let expected = Origin::Declared(DeclaredOrigin::StandardLibrary {
            library: SystemsLibraryIdentity::LIBRARY,
        });
        for fact in [
            FactKey::Element(ElementId::from_u128(4)),
            FactKey::Property {
                element: ElementId::from_u128(4),
                property: agq_kerml::properties::ELEMENT_DECLARED_NAME,
            },
            FactKey::AssociationOccurrence(agq_kernel::AssociationOccurrenceId::from_u128(5)),
        ] {
            let mut audit = SystemsPublicationAudit::default();
            let mut source_map = LibrarySourceMap::from([(fact, source.clone())]);
            audit_source_fact(fact, &expected, &expected, &source_map, &nodes, &mut audit);
            assert!(audit.findings.is_empty());
            for invalid in [
                SourceOrigin {
                    document: DocumentId::from_u128(99),
                    ..source.clone()
                },
                SourceOrigin {
                    revision: SourceRevisionId::from_u128(99),
                    ..source.clone()
                },
                SourceOrigin {
                    syntax_node: None,
                    ..source.clone()
                },
                SourceOrigin {
                    syntax_node: Some(SyntaxNodeId::from_u128(99)),
                    ..source.clone()
                },
                SourceOrigin {
                    range: ByteRange::new(4, 7).unwrap(),
                    ..source.clone()
                },
            ] {
                source_map.insert(fact, invalid);
                audit_source_fact(fact, &expected, &expected, &source_map, &nodes, &mut audit);
            }
            source_map.clear();
            audit_source_fact(fact, &expected, &expected, &source_map, &nodes, &mut audit);
            source_map.insert(fact, source.clone());
            audit_source_fact(
                fact,
                &Origin::Declared(DeclaredOrigin::Authored { source: None }),
                &expected,
                &source_map,
                &nodes,
                &mut audit,
            );
            assert_eq!(audit.findings.len(), 7);
            assert!(audit.findings.iter().all(|finding| matches!(finding, SystemsPublicationFinding::Provenance(key) if *key == fact)));
        }
    }

    #[test]
    fn all_systems_declared_fact_sources_match_exact_operational_syntax_nodes() {
        use crate::library::construction::{self, SourceInput};
        use agq_kerml_syntax::production;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
        let base = Snapshot::new(Arc::new(
            agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        ));
        let expected = Origin::Declared(DeclaredOrigin::StandardLibrary {
            library: SystemsLibraryIdentity::LIBRARY,
        });
        let mut audit = SystemsPublicationAudit::default();
        let mut references = Vec::new();
        for source in sources
            .documents()
            .filter(|d| d.language() == LibraryLanguage::SysMl)
        {
            let syntax = production::parse_sysml_with_profile(
                SysmlSyntaxProfile::OperationalV2,
                source.document(),
                source.revision(),
                source.source(),
                Default::default(),
            )
            .unwrap();
            assert!(syntax.is_complete());
            let nodes = SourceNodes::from([(
                source.document(),
                (
                    source.revision(),
                    syntax.nodes().map(|n| (n.id(), n.range())).collect(),
                ),
            )]);
            let draft = construction::construct_on(
                &[SourceInput {
                    syntax: &syntax,
                    library: Some(source),
                    sysml: true,
                }],
                &Default::default(),
                agq_kerml::BaselineProfile::OPERATIONAL_V9,
                base.clone(),
                None,
            )
            .unwrap();
            references.extend(draft.references().iter().cloned());
            for record in draft.candidate().model().elements() {
                audit_source_fact(
                    FactKey::Element(record.id()),
                    record.origin(),
                    &expected,
                    draft.source_map(),
                    &nodes,
                    &mut audit,
                );
                for (property, slot) in record.slots() {
                    audit_source_fact(
                        FactKey::Property {
                            element: record.id(),
                            property,
                        },
                        slot.origin(),
                        &expected,
                        draft.source_map(),
                        &nodes,
                        &mut audit,
                    );
                }
            }
            for occurrence in draft.candidate().model().association_occurrences() {
                audit_source_fact(
                    FactKey::AssociationOccurrence(occurrence.id()),
                    occurrence.origin(),
                    &expected,
                    draft.source_map(),
                    &nodes,
                    &mut audit,
                );
            }
        }
        audit_reference_population(&references, &mut audit);
        assert!(audit.findings.is_empty(), "{:?}", audit.findings);
        assert!(audit.checked[&SystemsPublicationFamily::IdentityProvenance] > 7591);
    }

    #[test]
    fn reference_gate_requires_complete_unique_correct_kind_matching_endpoint() {
        let a = ElementId::from_u128(1);
        let b = ElementId::from_u128(2);
        let cases = [
            (
                Completeness::Complete,
                vec![a],
                vec![a],
                true,
                SystemsReferenceStatus::Complete,
            ),
            (
                Completeness::Incomplete,
                vec![a],
                vec![a],
                true,
                SystemsReferenceStatus::Incomplete,
            ),
            (
                Completeness::Invalid,
                vec![a],
                vec![a],
                true,
                SystemsReferenceStatus::Invalid,
            ),
            (
                Completeness::Complete,
                vec![],
                vec![],
                true,
                SystemsReferenceStatus::Unresolved,
            ),
            (
                Completeness::Complete,
                vec![a, b],
                vec![a],
                true,
                SystemsReferenceStatus::Ambiguous,
            ),
            (
                Completeness::Complete,
                vec![a],
                vec![a],
                false,
                SystemsReferenceStatus::Invalid,
            ),
            (
                Completeness::Complete,
                vec![a],
                vec![b],
                true,
                SystemsReferenceStatus::EndpointMismatch,
            ),
            (
                Completeness::Complete,
                vec![a],
                vec![a, a],
                true,
                SystemsReferenceStatus::EndpointMismatch,
            ),
        ];
        for (completeness, candidates, stored, kind, expected) in cases {
            assert_eq!(
                reference_status(completeness, &candidates, &stored, kind),
                expected
            );
        }
    }
    #[test]
    fn authority_findings_require_the_exact_failed_target_read() {
        let subject = ElementId::from_u128(1);
        let rule = "checkViewpointDefinitionSpecialization";
        let mut diagnostic = Diagnostic {
            code: "SQ_TARGET_MISSING",
            subject,
            message: "Exact formal target Views::Viewpoint is unavailable".into(),
        };
        assert_eq!(
            authority_conflict(rule, subject, &diagnostic)
                .unwrap()
                .original_declaration,
            "Views::ViewpointCheck"
        );
        diagnostic.code = "SQ_TARGET_NAMESPACE_PENDING";
        assert!(authority_conflict(rule, subject, &diagnostic).is_none());
        diagnostic.code = "SQ_TARGET_MISSING";
        diagnostic.message = "Exact formal target Actions::Action is unavailable".into();
        assert!(authority_conflict(rule, subject, &diagnostic).is_none());
    }
}
