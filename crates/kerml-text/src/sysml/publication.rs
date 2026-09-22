//! Acceptance of the exact Systems Library, independently of SysML conformance.
use super::{SystemsDocumentStatus, SystemsLibraryCandidate};
use crate::library::{CanonicalKermlStandardLibraries, LibraryLoadError, LibrarySourceMap};
use agq_kerml_semantics::{
    Completeness, Diagnostic, KerMlQueries, PublicationClosureOptions, PublicationCounters,
    PublicationFamily, PublicationOverlayError, PublicationStage, SemanticContextId,
    close_result_structure_with_extension,
};
use agq_kerml_syntax::production::SysmlSyntaxProfile;
use agq_kernel::{
    ElementId, Snapshot,
    derived::{DerivedOverlay, PropertyState},
    provenance::{DeclaredOrigin, FactKey, Origin},
    value::Value,
};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrary, VerifiedLibrarySet};
use agq_sysml::{classes as sc, properties as sp};
use agq_sysml_semantics::{
    SYSML_SEMANTIC_CONTEXT_DOMAIN, StandardSysmlBindings, StandardSysmlRole, SysmlBaselineProfile,
    SysmlBindingError, SysmlContextError, SysmlDependencyContract, SysmlProducerExtension,
    SysmlSemanticContextId, SystemsLibraryIdentity, sysml_producer_rule_ids,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

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
impl CanonicalSysmlSystemsLibrary {
    /// Revalidate declared storage and rerun the combined scheduler on new local
    /// records only. Neither an earlier construction closure nor a caller's
    /// restricted subject list can establish publication acceptance.
    pub fn publish(
        candidate: SystemsLibraryCandidate,
        sources: &VerifiedLibrarySet,
        options: PublicationClosureOptions,
        batch_progress: impl FnMut(usize, usize, usize, usize),
        stage_progress: impl FnMut(&PublicationStage),
    ) -> Result<Self, SystemsPublicationError> {
        let mut audit = SystemsPublicationAudit::default();
        let profile = SysmlBaselineProfile::OperationalV1;
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
        contract.standard_bindings = producer_bindings.targets().clone();
        let contract_digest = contract.context_identity_digest();
        let roots: Vec<_> = candidate
            .draft()
            .roots()
            .iter()
            .chain(candidate.accepted_kerml().roots())
            .copied()
            .collect();
        let extension =
            SysmlProducerExtension::new(profile, producer_bindings.clone(), roots.clone());
        let closure = close_result_structure_with_extension(
            &declared,
            options,
            |overlay| {
                candidate
                    .accepted_kerml()
                    .complete_overlay()
                    .project_overlay_context(overlay, candidate.draft().roots())
                    .and_then(|context| {
                        context.with_semantic_extension_identity(
                            SYSML_SEMANTIC_CONTEXT_DOMAIN,
                            contract_digest,
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
        let q = KerMlQueries::new(
            candidate
                .accepted_kerml()
                .complete_overlay()
                .project_overlay_context(&closure.overlay, candidate.draft().roots())
                .and_then(|context| {
                    context.with_semantic_extension_identity(
                        SYSML_SEMANTIC_CONTEXT_DOMAIN,
                        contract_digest,
                    )
                })
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
        for diagnostic in closure
            .stages
            .iter()
            .flat_map(|stage| &stage.diagnostics)
            .collect::<std::collections::BTreeSet<_>>()
        {
            for rule in [
                "checkViewpointDefinitionSpecialization",
                "checkViewpointUsageSpecialization",
                "checkConnectionDefinitionBinarySpecialization",
            ] {
                if let Some(conflict) = authority_conflict(rule, diagnostic.subject, diagnostic) {
                    audit
                        .findings
                        .push(SystemsPublicationFinding::Authority(conflict));
                }
            }
        }
        audit_sysml_population(&q, &local, &mut audit);
        audit_references(&q, &candidate, &mut audit);
        let bindings = StandardSysmlBindings::validate(
            q.model(),
            &q,
            identity,
            candidate.draft().roots(),
            StandardSysmlRole::ALL,
        )
        .and_then(|bindings| {
            bindings.with_verified_sources(library, candidate.draft().source_map())
        });
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
            accepted_kerml: candidate.accepted_kerml().clone(),
            bindings,
            identity: publication_identity,
            context,
            roots: candidate.draft().roots().to_vec(),
            source_map: candidate.draft().source_map().clone(),
            documents: candidate.documents().to_vec(),
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
                .with_semantic_extension_identity(
                    SYSML_SEMANTIC_CONTEXT_DOMAIN,
                    self.identity.dependencies.context_identity_digest(),
                )
                .expect("accepted SysML context identity"),
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
    let accepted = candidate.accepted_kerml();
    for (valid, field) in [
        (
            candidate.dependency_contract() == contract,
            "candidate dependency contract",
        ),
        (
            candidate.syntax_profile() == SysmlSyntaxProfile::OperationalV1,
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
        let valid = status.profile == SysmlSyntaxProfile::OperationalV1
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

fn audit_source_provenance(
    declared: &Snapshot,
    source_map: &LibrarySourceMap,
    library: &VerifiedLibrary,
    audit: &mut SystemsPublicationAudit,
) {
    let expected = Origin::Declared(DeclaredOrigin::StandardLibrary {
        library: library.id(),
    });
    for record in declared
        .model()
        .elements()
        .filter(|record| !declared.is_dependency_element(record.id()))
    {
        let fact = FactKey::Element(record.id());
        let source_valid = source_map.get(&fact).is_some_and(|origin| {
            library
                .documents()
                .iter()
                .find(|source| source.document() == origin.document)
                .is_some_and(|source| {
                    source.revision() == origin.revision
                        && origin.syntax_node.is_some()
                        && origin.range.start() < origin.range.end()
                        && source
                            .source()
                            .get(origin.range.start() as usize..origin.range.end() as usize)
                            .is_some()
                })
        });
        if record.origin() != &expected || !source_valid {
            audit
                .findings
                .push(SystemsPublicationFinding::Provenance(fact));
        }
        for (property, slot) in record.slots() {
            if slot.origin() != &expected {
                audit
                    .findings
                    .push(SystemsPublicationFinding::Provenance(FactKey::Property {
                        element: record.id(),
                        property,
                    }));
            }
        }
        audit.checked(SystemsPublicationFamily::IdentityProvenance, 1);
    }
}

fn audit_sysml_population(
    q: &KerMlQueries<'_>,
    subjects: &[ElementId],
    audit: &mut SystemsPublicationAudit,
) {
    let profile = SysmlBaselineProfile::OperationalV1;
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
                ][..],
            ),
        ] {
            if classes.iter().any(|&class| is(class)) {
                audit.checked(family, 1);
            }
        }
        if is(sc::USAGE) {
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

fn audit_references(
    q: &KerMlQueries<'_>,
    candidate: &SystemsLibraryCandidate,
    audit: &mut SystemsPublicationAudit,
) {
    for batch in candidate.draft().references().chunks(32) {
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
