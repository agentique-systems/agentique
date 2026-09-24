//! Authenticated read-only finalization of a scheduler-issued strict frontier.
use super::*;
use crate::library::construction::{self, SourceInput};
use agq_kerml_semantics::PublicationFrontierSession;
use agq_kerml_syntax::production;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    time::Instant,
};

/// Resource policy for read-only effective API acceptance. Both modes execute
/// the identical queries and merge findings in the original subject order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SystemsFinalizationAuditMode {
    #[default]
    Serial,
    ParallelTwo,
}
impl SystemsFinalizationAuditMode {
    fn workers(self) -> usize {
        match self {
            Self::Serial => 1,
            Self::ParallelTwo => 2,
        }
    }
}

/// Observational journal only. Each complete line is flushed and synchronized;
/// an interrupted stage never acquires an end record or publication authority.
pub(super) struct AuditLog {
    file: Option<File>,
    started: Instant,
    effective_mode: SystemsFinalizationAuditMode,
}
impl AuditLog {
    pub(super) fn disabled() -> Self {
        Self::disabled_with_mode(SystemsFinalizationAuditMode::Serial)
    }
    pub(super) fn disabled_with_mode(effective_mode: SystemsFinalizationAuditMode) -> Self {
        Self {
            file: None,
            started: Instant::now(),
            effective_mode,
        }
    }
    pub(super) fn effective_workers(&self) -> usize {
        self.effective_mode.workers()
    }
    pub(super) fn create(
        directory: &Path,
        effective_mode: SystemsFinalizationAuditMode,
    ) -> std::io::Result<Self> {
        std::fs::create_dir_all(directory)?;
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(directory.join("audit-events.jsonl"))?;
        Ok(Self {
            file: Some(file),
            started: Instant::now(),
            effective_mode,
        })
    }
    pub(super) fn begin(&mut self, stage: &str) -> std::io::Result<Instant> {
        let started = Instant::now();
        self.event(stage, "begin", 0, 0)?;
        Ok(started)
    }
    pub(super) fn end(
        &mut self,
        stage: &str,
        started: Instant,
        findings: usize,
    ) -> std::io::Result<()> {
        self.event(stage, "end", started.elapsed().as_micros(), findings)
    }
    pub(super) fn effective_batch(
        &mut self,
        batch_index: usize,
        subjects: &[ElementId],
        total_subjects: usize,
        elapsed_micros: Option<u128>,
        findings: usize,
    ) -> std::io::Result<()> {
        if let Some(file) = &mut self.file {
            let event = if elapsed_micros.is_some() {
                "end"
            } else {
                "begin"
            };
            let first_subject = subjects.first().expect("nonempty effective audit batch");
            let last_subject = subjects.last().expect("nonempty effective audit batch");
            serde_json::to_writer(
                &mut *file,
                &serde_json::json!({
                    "format":"agq-systems-finalization-audit/1", "stage":"effective_sysml_population_batch",
                    "event":event, "batch_index":batch_index,
                    "first_subject":first_subject, "last_subject":last_subject,
                    "subjects":subjects.len(), "total_subjects":total_subjects,
                    "workers":self.effective_mode.workers(),
                    "elapsed_micros":elapsed_micros, "findings":findings,
                    "run_elapsed_micros":self.started.elapsed().as_micros(), "publication_authority":false,
                }),
            )?;
            file.write_all(b"\n")?;
            file.flush()?;
            file.sync_all()?;
            eprintln!(
                "Systems effective audit batch {batch_index} {event}: {} subjects ({first_subject} .. {last_subject}), {elapsed_micros:?} us, {findings} findings",
                subjects.len()
            );
        }
        Ok(())
    }
    fn event(
        &mut self,
        stage: &str,
        event: &str,
        elapsed_micros: u128,
        findings: usize,
    ) -> std::io::Result<()> {
        if let Some(file) = &mut self.file {
            let value = serde_json::json!({
                "format":"agq-systems-finalization-audit/1", "stage":stage, "event":event,
                "elapsed_micros":elapsed_micros, "run_elapsed_micros":self.started.elapsed().as_micros(),
                "findings":findings, "publication_authority":false,
            });
            serde_json::to_writer(&mut *file, &value)?;
            file.write_all(b"\n")?;
            file.flush()?;
            file.sync_all()?;
            eprintln!(
                "Systems finalization {event}: {stage} ({elapsed_micros} us, {findings} findings)"
            );
        }
        Ok(())
    }
}

impl CanonicalSysmlSystemsLibrary {
    /// Finish strict acceptance from the exact authenticated converged frontier.
    /// The journal pin must be retained independently of the checkpoint files.
    /// No producer scheduler or source-reference refinement runs on this path.
    /// All publication audits run again against the restored immutable graph;
    /// durable observational events require a fresh ignored output directory.
    pub fn finalize_from_converged_frontier(
        sources: &VerifiedLibrarySet,
        accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
        journal: impl AsRef<Path>,
        expected_journal_sha256: [u8; 32],
        audit_directory: impl AsRef<Path>,
    ) -> Result<Self, SystemsPublicationError> {
        Self::finalize_from_converged_frontier_with_audit_mode(
            sources,
            accepted_kerml,
            journal,
            expected_journal_sha256,
            audit_directory,
            SystemsFinalizationAuditMode::Serial,
        )
    }

    /// The same authenticated finalization with an explicit bounded read-only
    /// audit resource policy. Parallel execution creates at most two eight-subject
    /// query contexts over the identical immutable graph, never parallel producers.
    pub fn finalize_from_converged_frontier_with_audit_mode(
        sources: &VerifiedLibrarySet,
        accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
        journal: impl AsRef<Path>,
        expected_journal_sha256: [u8; 32],
        audit_directory: impl AsRef<Path>,
        audit_mode: SystemsFinalizationAuditMode,
    ) -> Result<Self, SystemsPublicationError> {
        Self::finalize_from_converged_frontier_with_profile(
            sources,
            accepted_kerml,
            journal,
            expected_journal_sha256,
            audit_directory,
            audit_mode,
            SysmlSyntaxProfile::OperationalV2,
        )
    }

    /// Authenticate the requested interpretation independently of checkpoint
    /// contents. Every strict finalization gate is identical for v2 and v3.
    pub fn finalize_from_converged_frontier_with_profile(
        sources: &VerifiedLibrarySet,
        accepted_kerml: Arc<CanonicalKermlStandardLibraries>,
        journal: impl AsRef<Path>,
        expected_journal_sha256: [u8; 32],
        audit_directory: impl AsRef<Path>,
        audit_mode: SystemsFinalizationAuditMode,
        profile: SysmlSyntaxProfile,
    ) -> Result<Self, SystemsPublicationError> {
        let mut observer = AuditLog::create(audit_directory.as_ref(), audit_mode)?;
        let started = observer.begin("frontier_authentication")?;
        let source_identity =
            super::super::systems_frontier_source_identity(sources, &accepted_kerml, profile, None);
        let session = PublicationFrontierSession::resume(
            journal,
            expected_journal_sha256,
            source_identity,
            0,
        )?;
        observer.end("frontier_authentication", started, 0)?;
        let base = super::super::base(&accepted_kerml)?;
        let started = observer.begin("graph_restore")?;
        let frontier = session.restore_converged_frontier(
            Arc::new(base.model().registry().clone()),
            base.immutable_dependency().cloned(),
        )?;
        observer.end("graph_restore", started, 0)?;
        let started = observer.begin("source_context_reconstruction")?;
        let candidate =
            reconstruct_source_metadata(sources, accepted_kerml, frontier.overlay(), profile)?;
        let mut contract = candidate.dependency_contract().clone();
        let mut audit = SystemsPublicationAudit::default();
        audit_inputs(&candidate, sources, &contract, &mut audit);
        let library = sources
            .libraries()
            .get(&contract.systems_library.library)
            .ok_or(SysmlContextError::IdentityMismatch(
                "Systems source library",
            ))?;
        audit_source_provenance(
            frontier.overlay().declared(),
            candidate.draft().source_map(),
            library,
            profile,
            &mut audit,
        );
        if !audit.findings.is_empty() {
            observer.end(
                "source_context_reconstruction",
                started,
                audit.findings.len(),
            )?;
            return Err(SystemsPublicationError::Rejected(Box::new(audit)));
        }
        let inputs = PublicationInputs::consume(candidate);
        observer.end(
            "source_context_reconstruction",
            started,
            audit.findings.len(),
        )?;
        let started = observer.begin("candidate_binding_context")?;
        let empty = StandardSysmlBindings::unbound(contract.systems_library.clone());
        let current = SysmlSemanticContext::for_producer_overlay(
            frontier.overlay(),
            inputs.accepted_kerml.complete_overlay(),
            &inputs.roots,
            &contract,
            empty,
        )?;
        let queries = KerMlQueries::new(current.kerml_context().fork());
        let bindings = StandardSysmlBindings::validate(
            frontier.overlay().model(),
            &queries,
            contract.systems_library.clone(),
            &inputs.roots,
            StandardSysmlRole::ALL,
        )
        .and_then(|bindings| bindings.with_verified_sources(library, &inputs.source_map))
        .map_err(|error| {
            SystemsPublicationError::Rejected(Box::new(SystemsPublicationAudit {
                findings: vec![SystemsPublicationFinding::Binding(error)],
                ..audit.clone()
            }))
        })?;
        drop(queries);
        drop(current);
        contract.standard_bindings = bindings.targets().clone();
        observer.end("candidate_binding_context", started, 0)?;
        let started = observer.begin("frontier_certificate_authentication")?;
        let registry = super::super::systems_producer_registry(inputs.accepted_kerml.profile());
        let closure = frontier.authenticate(
            |overlay| {
                inputs
                    .accepted_kerml
                    .complete_overlay()
                    .project_overlay_context(overlay, &inputs.roots)
                    .and_then(|context| {
                        context.with_naming_extension(
                            SYSML_SEMANTIC_CONTEXT_DOMAIN,
                            contract.context_identity_digest(),
                            Arc::new(agq_sysml_semantics::SysmlNamingExtension),
                        )
                    })
                    .and_then(|context| context.with_producer_registry_digest(registry.digest()))
                    .map_err(PublicationOverlayError::Context)
            },
            &registry,
        )?;
        observer.end("frontier_certificate_authentication", started, 0)?;
        Self::accept_closed(
            closure,
            inputs,
            sources,
            contract,
            bindings,
            audit,
            &mut observer,
        )
    }
}

fn reconstruct_source_metadata(
    sources: &VerifiedLibrarySet,
    publication: Arc<CanonicalKermlStandardLibraries>,
    overlay: &DerivedOverlay,
    profile: SysmlSyntaxProfile,
) -> Result<SystemsLibraryCandidate, SystemsPublicationError> {
    let mut parsed = Vec::new();
    let mut documents = Vec::new();
    for source in sources
        .documents()
        .filter(|source| source.language() == LibraryLanguage::SysMl)
    {
        let syntax = production::parse_sysml_with_profile(
            profile,
            source.document(),
            source.revision(),
            source.source(),
            Default::default(),
        )
        .map_err(LibraryLoadError::from)?;
        let construction_gap = construction::check_supported_sysml(&syntax)
            .err()
            .map(|error| error.to_string());
        documents.push(SystemsDocumentStatus {
            path: source.path().into(),
            document: source.document(),
            source_sha256: source.sha256().into(),
            profile,
            parsed: syntax.is_complete(),
            byte_exact: syntax
                .tokens()
                .iter()
                .map(|token| syntax.token_text(token))
                .collect::<String>()
                == source.source(),
            recovery_count: syntax.recovery().len(),
            production_count: syntax.nodes().count(),
            construction_gap,
        });
        parsed.push((source, syntax));
    }
    let source_inputs: Vec<_> = parsed
        .iter()
        .map(|(source, syntax)| SourceInput {
            syntax,
            library: Some(source),
            sysml: true,
        })
        .collect();
    let base = super::super::base(&publication)?;
    let draft = construction::construct_on(
        &source_inputs,
        &Default::default(),
        publication.profile(),
        base.clone(),
        None,
    )?;
    // Endpoints authenticate source construction only. Independent final semantic
    // lookup still checks every mandatory assertion against this restored graph.
    let endpoints = draft
        .references()
        .iter()
        .filter_map(|reference| {
            overlay
                .declared()
                .model()
                .navigation_slot(reference.relationship, reference.property)
                .and_then(|slot| {
                    slot.value().values().find_map(|value| match value {
                        Value::Reference(target) => {
                            Some(((reference.relationship, reference.property), *target))
                        }
                        _ => None,
                    })
                })
        })
        .collect();
    drop(draft);
    let draft = construction::construct_on(
        &source_inputs,
        &endpoints,
        publication.profile(),
        base,
        None,
    )?;
    if !draft.candidate().obligations().is_empty()
        || !draft
            .candidate()
            .model()
            .elements()
            .eq(overlay.declared().model().elements())
        || !draft
            .candidate()
            .model()
            .association_occurrences()
            .eq(overlay.declared().model().association_occurrences())
    {
        return Err(SystemsPublicationError::Overlay(
            PublicationOverlayError::FrontierCheckpoint(
                "restored declarations differ from exact Systems source construction".into(),
            ),
        ));
    }
    let dependency_contract = SysmlDependencyContract::checked_in_for_profile(
        &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned(
            SystemsLibraryIdentity::SOURCE_CONTENT_SET,
        )),
        match profile {
            SysmlSyntaxProfile::Published => SysmlBaselineProfile::PUBLISHED,
            SysmlSyntaxProfile::OperationalV1 => SysmlBaselineProfile::OPERATIONAL_V1,
            SysmlSyntaxProfile::OperationalV2 => SysmlBaselineProfile::OPERATIONAL_V2,
            SysmlSyntaxProfile::OperationalV3 => SysmlBaselineProfile::OPERATIONAL_V3,
        },
    )?;
    Ok(SystemsLibraryCandidate {
        draft,
        documents,
        publication,
        source_content_set: sources.content_set_id().into(),
        syntax_profile: profile,
        production: None,
        dependency_contract,
    })
}
